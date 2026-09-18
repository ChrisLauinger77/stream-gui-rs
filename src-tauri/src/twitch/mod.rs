mod api;
#[cfg(test)]
mod tests;

use crate::{
    credentials::{CredentialStore, Credentials},
    domain::{AppError, ErrorCode, Result},
};
pub use api::HttpTwitchApi;
use api::{PollResult, TwitchApi, Validation};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{Mutex, watch},
    time::Instant,
};
use ts_rs::TS;
use zeroize::Zeroizing;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AuthPhase {
    NotConfigured,
    SignedOut,
    Authorizing,
    Authenticated,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DeviceAuthorization {
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AuthUser {
    pub id: String,
    pub login: String,
    pub scopes: Vec<String>,
    pub expires_in: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub phase: AuthPhase,
    pub user: Option<AuthUser>,
    pub authorization: Option<DeviceAuthorization>,
    pub error: Option<AppError>,
    pub credential_storage: String,
}

struct Pending {
    device_code: Zeroizing<String>,
    expires: Instant,
    next_poll: Instant,
    interval: Duration,
}

struct State {
    status: AuthStatus,
    pending: Option<Pending>,
    store: Box<dyn CredentialStore>,
    expires: Option<Instant>,
    next_validation: Instant,
    next_refresh: Instant,
}

/// One serialized owner for grant polling, token rotation, validation and logout.
/// Holding the async mutex across bounded HTTP requests prevents late poll/refresh
/// responses from restoring credentials after logout, and double-use of refresh.
pub struct AuthService<A: TwitchApi = HttpTwitchApi> {
    api: A,
    client_id: Option<String>,
    state: Mutex<State>,
}

impl<A: TwitchApi + 'static> AuthService<A> {
    pub fn new(api: A, client_id: Option<String>, store: Box<dyn CredentialStore>) -> Self {
        let client_id = client_id.filter(|s| !s.trim().is_empty());
        let phase = if client_id.is_some() {
            AuthPhase::SignedOut
        } else {
            AuthPhase::NotConfigured
        };
        Self {
            api,
            client_id,
            state: Mutex::new(State {
                status: AuthStatus {
                    phase,
                    user: None,
                    authorization: None,
                    error: None,
                    credential_storage: store.name().into(),
                },
                store,
                pending: None,
                expires: None,
                next_validation: Instant::now(),
                next_refresh: Instant::now(),
            }),
        }
    }

    pub async fn status(&self) -> AuthStatus {
        let state = self.state.lock().await;
        let mut status = state.status.clone();
        if let (Some(auth), Some(pending)) = (&mut status.authorization, &state.pending) {
            auth.expires_in = pending
                .expires
                .saturating_duration_since(Instant::now())
                .as_secs() as u32;
        }
        if let (Some(user), Some(expires)) = (&mut status.user, state.expires) {
            user.expires_in = expires.saturating_duration_since(Instant::now()).as_secs() as u32;
        }
        status
    }

    pub async fn login(&self) -> Result<AuthStatus> {
        let client = self.client_id()?;
        let mut state = self.state.lock().await;
        if state.pending.is_some() {
            return Ok(state.status.clone());
        }
        if state.store.load()?.is_some() {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Log out before starting a new authorization.",
            ));
        }
        let grant = match self.api.begin(client).await {
            Ok(grant) => grant,
            Err(error) => {
                state.status.phase = AuthPhase::Error;
                state.status.error = Some(error.clone());
                return Err(error);
            }
        };
        if !valid_verification_uri(&grant.verification_uri)
            || grant.device_code.is_empty()
            || grant.user_code.is_empty()
            || grant.user_code.len() > 64
            || grant.expires_in == 0
            || grant.expires_in > 86400
            || grant.interval > grant.expires_in
        {
            return Err(AppError::new(
                ErrorCode::AuthProvider,
                "Twitch returned an invalid device authorization.",
            ));
        }
        let now = Instant::now();
        let interval = Duration::from_secs(grant.interval.max(5).into());
        state.pending = Some(Pending {
            device_code: Zeroizing::new(grant.device_code),
            expires: now + Duration::from_secs(grant.expires_in.into()),
            next_poll: now + interval,
            interval,
        });
        state.status.phase = AuthPhase::Authorizing;
        state.status.error = None;
        state.status.authorization = Some(DeviceAuthorization {
            user_code: grant.user_code,
            verification_uri: grant.verification_uri,
            expires_in: grant.expires_in,
        });
        Ok(state.status.clone())
    }

    pub async fn verification_uri(&self) -> Result<String> {
        let state = self.state.lock().await;
        if state
            .pending
            .as_ref()
            .is_some_and(|pending| pending.expires > Instant::now())
        {
            if let Some(auth) = &state.status.authorization {
                return Ok(auth.verification_uri.clone());
            }
        }
        Err(AppError::new(
            ErrorCode::AuthExpired,
            "Start login to obtain a current authorization link.",
        ))
    }

    pub async fn validate(&self) -> Result<AuthStatus> {
        self.client_id()?;
        let mut state = self.state.lock().await;
        self.validate_locked(&mut state).await?;
        Ok(state.status.clone())
    }

    pub async fn refresh(&self) -> Result<AuthStatus> {
        self.client_id()?;
        let mut state = self.state.lock().await;
        self.refresh_locked(&mut state).await?;
        Ok(state.status.clone())
    }

    pub async fn logout(&self) -> Result<AuthStatus> {
        let mut state = self.state.lock().await;
        let credentials = state.store.load()?;
        state.store.clear()?;
        state.pending = None;
        state.expires = None;
        state.status.user = None;
        state.status.authorization = None;
        state.status.error = None;
        state.status.phase = if self.client_id.is_some() {
            AuthPhase::SignedOut
        } else {
            AuthPhase::NotConfigured
        };
        if let (Some(credentials), Some(client)) = (credentials, &self.client_id) {
            if self
                .api
                .revoke(client, &credentials.access_token)
                .await
                .is_err()
            {
                state.status.error = Some(AppError::new(
                    ErrorCode::Network,
                    "Local credentials were cleared. Remote token revocation could not be confirmed; disconnect the app in Twitch settings if needed.",
                ));
            }
        }
        Ok(state.status.clone())
    }

    pub async fn tick(&self) -> Result<()> {
        let Some(client) = &self.client_id else {
            return Ok(());
        };
        let mut state = self.state.lock().await;
        let now = Instant::now();
        if let Some(pending) = &mut state.pending {
            if now >= pending.expires {
                let error = AppError::new(
                    ErrorCode::AuthExpired,
                    "The authorization code expired. Start login again.",
                );
                return Self::invalidate(&mut state, error);
            }
            if now < pending.next_poll {
                return Ok(());
            }
            let response = self.api.poll(client, &pending.device_code).await;
            match response {
                Ok(PollResult::Pending) => {
                    let pending = state.pending.as_mut().expect("pending grant");
                    pending.next_poll = Instant::now() + pending.interval;
                    state.status.error = None;
                }
                Ok(PollResult::SlowDown) => {
                    let pending = state.pending.as_mut().expect("pending grant");
                    pending.interval =
                        (pending.interval + Duration::from_secs(5)).min(Duration::from_secs(300));
                    pending.next_poll = Instant::now() + pending.interval;
                }
                Ok(PollResult::Authorized(credentials)) => {
                    state.pending = None;
                    state.status.authorization = None;
                    self.accept_credentials(&mut state, credentials).await?;
                }
                Err(error) if error.code == ErrorCode::Network => {
                    let pending = state.pending.as_mut().expect("pending grant");
                    pending.interval = (pending.interval * 2).min(Duration::from_secs(300));
                    pending.next_poll = Instant::now() + pending.interval;
                    state.status.error = Some(error);
                }
                Err(error) => return Self::invalidate(&mut state, error),
            }
        } else if state.store.load()?.is_some() {
            if state
                .expires
                .is_some_and(|expiry| expiry <= now + Duration::from_secs(300))
                && now >= state.next_refresh
            {
                self.refresh_locked(&mut state).await?;
            } else if now >= state.next_validation {
                self.validate_locked(&mut state).await?;
            }
        }
        Ok(())
    }

    pub fn start(self: &Arc<Self>) -> watch::Sender<bool> {
        let (stop, mut receiver) = watch::channel(false);
        let service = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    result = receiver.changed() => if result.is_err() || *receiver.borrow() { break; },
                    _ = interval.tick() => { let _ = service.tick().await; }
                }
            }
        });
        stop
    }

    fn client_id(&self) -> Result<&str> {
        self.client_id.as_deref().ok_or_else(|| AppError::new(ErrorCode::NotConfigured, "Register a new public Twitch application, set TWITCH_CLIENT_ID in the backend environment, and restart. No client secret is used."))
    }

    async fn validate_locked(&self, state: &mut State) -> Result<()> {
        let credentials = state.store.load()?.ok_or_else(|| {
            AppError::new(
                ErrorCode::AuthInvalid,
                "No stored credentials. Sign in first.",
            )
        })?;
        // Retry temporary failures in one minute, not on every tick.
        state.next_validation = Instant::now() + Duration::from_secs(60);
        match self.api.validate(&credentials.access_token).await {
            Ok(validation) => self.apply_validation(state, validation),
            Err(error) if error.code == ErrorCode::AuthInvalid => Self::invalidate(state, error),
            Err(error) => {
                // An identity is only shown after a successful validation.
                state.status.phase = AuthPhase::Error;
                state.status.user = None;
                state.status.error = Some(error.clone());
                Err(error)
            }
        }
    }

    fn apply_validation(&self, state: &mut State, validation: Validation) -> Result<()> {
        if Some(&validation.client_id) != self.client_id.as_ref()
            || validation.user_id.is_empty()
            || validation.login.is_empty()
            || validation.expires_in == 0
        {
            return Self::invalidate(
                state,
                AppError::new(
                    ErrorCode::AuthInvalid,
                    "Token identity, expiry, or client ID did not validate. Sign in again.",
                ),
            );
        }
        state.status.phase = AuthPhase::Authenticated;
        state.status.error = None;
        state.status.user = Some(AuthUser {
            id: validation.user_id,
            login: validation.login,
            scopes: validation.scopes,
            expires_in: validation.expires_in,
        });
        state.expires = Some(Instant::now() + Duration::from_secs(validation.expires_in.into()));
        state.next_validation = Instant::now() + Duration::from_secs(3600);
        Ok(())
    }

    async fn accept_credentials(&self, state: &mut State, credentials: Credentials) -> Result<()> {
        // Save a rotated refresh token before validation, so a temporary validate
        // failure cannot cause the already-consumed old refresh token to be used.
        if let Err(error) = state.store.save(credentials) {
            return Self::invalidate(state, error);
        }
        self.validate_locked(state).await
    }

    async fn refresh_locked(&self, state: &mut State) -> Result<()> {
        let client = self.client_id()?;
        let credentials = state.store.load()?.ok_or_else(|| {
            AppError::new(
                ErrorCode::AuthInvalid,
                "No stored credentials. Sign in first.",
            )
        })?;
        state.next_refresh = Instant::now() + Duration::from_secs(60);
        match self.api.refresh(client, &credentials.refresh_token).await {
            Ok(next) => {
                state.expires = None;
                self.accept_credentials(state, next).await
            }
            // Device-flow refresh tokens are single use. An ambiguous network
            // outcome cannot safely be retried with the same refresh token.
            Err(error) => Self::invalidate(
                state,
                AppError::new(
                    error.code,
                    "Refresh failed or its outcome is uncertain. Local credentials were cleared; sign in again.",
                ),
            ),
        }
    }

    fn invalidate(state: &mut State, error: AppError) -> Result<()> {
        state.pending = None;
        state.expires = None;
        state.status.phase = AuthPhase::Error;
        state.status.user = None;
        state.status.authorization = None;
        state.status.error = Some(error.clone());
        state.store.clear()?;
        Err(error)
    }
}

fn valid_verification_uri(uri: &str) -> bool {
    url::Url::parse(uri).is_ok_and(|url| {
        url.scheme() == "https"
            && matches!(url.host_str(), Some("www.twitch.tv" | "twitch.tv"))
            && url.path() == "/activate"
            && url.username().is_empty()
            && url.password().is_none()
            && url.port().is_none()
            && url.fragment().is_none()
    })
}
