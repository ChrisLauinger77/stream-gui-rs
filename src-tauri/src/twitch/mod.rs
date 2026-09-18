pub(crate) mod api;
#[cfg(test)]
mod tests;

use crate::{
    credentials::{CredentialStore, CredentialVault, Credentials},
    domain::{AppError, ErrorCode, Result},
    twitch_http::error,
};
pub use api::HttpTwitchApi;
use api::{PollResult, TwitchApi, Validation};
use serde::{Deserialize, Serialize};
use std::{
    ops::{Deref, DerefMut},
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{
    sync::{Mutex, MutexGuard, watch},
    time::Instant,
};
use tokio_util::sync::CancellationToken;
use ts_rs::TS;
use zeroize::Zeroizing;

pub const REQUIRED_SCOPES: &[&str] = &["user:read:follows"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AuthPhase {
    NotConfigured,
    Restoring,
    SignedOut,
    Authorizing,
    Authenticated,
    Cancelled,
    Expired,
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
    store: CredentialVault,
    credentials: Option<Credentials>,
    expected_user: Option<String>,
    restored: bool,
    expires: Option<Instant>,
    next_validation: Instant,
    next_refresh: Instant,
    generation: u64,
    session_id: u64,
    session_cancel: CancellationToken,
    last_validation: Result<()>,
    last_refresh: Result<()>,
}
#[derive(Clone)]
struct Snapshot {
    status: AuthStatus,
    expires: Option<Instant>,
    grant_expires: Option<Instant>,
}
impl Snapshot {
    fn from_state(state: &State) -> Self {
        Self {
            status: state.status.clone(),
            expires: state.expires,
            grant_expires: state.pending.as_ref().map(|p| p.expires),
        }
    }
}
// Publishing on every exit path (including errors) keeps the read-only snapshot
// independent of the serialized owner of credentials and OAuth requests.
struct StateGuard<'a> {
    state: MutexGuard<'a, State>,
    snapshot: &'a watch::Sender<Snapshot>,
}
impl Deref for StateGuard<'_> {
    type Target = State;
    fn deref(&self) -> &State {
        &self.state
    }
}
impl DerefMut for StateGuard<'_> {
    fn deref_mut(&mut self) -> &mut State {
        &mut self.state
    }
}
impl Drop for StateGuard<'_> {
    fn drop(&mut self) {
        self.snapshot
            .send_replace(Snapshot::from_state(&self.state));
    }
}

/// Rust-internal authorization lease. Never Serialize, Debug or TS.
pub(crate) struct AccessLease {
    pub token: Zeroizing<String>,
    pub client_id: String,
    pub user_id: String,
    pub scopes: Vec<String>,
    pub generation: u64,
    pub session_id: u64,
    pub cancel: CancellationToken,
}

pub struct AuthService<A: TwitchApi = HttpTwitchApi> {
    api: Arc<A>,
    client_id: Option<String>,
    state: Arc<Mutex<State>>,
    snapshot: watch::Sender<Snapshot>,
    grant_cancel: Arc<StdMutex<CancellationToken>>,
    validation_revision: Arc<AtomicU64>,
    refresh_revision: Arc<AtomicU64>,
    closing: Arc<AtomicBool>,
}

impl<A: TwitchApi> Clone for AuthService<A> {
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
            client_id: self.client_id.clone(),
            state: self.state.clone(),
            snapshot: self.snapshot.clone(),
            grant_cancel: self.grant_cancel.clone(),
            validation_revision: self.validation_revision.clone(),
            refresh_revision: self.refresh_revision.clone(),
            closing: self.closing.clone(),
        }
    }
}
impl<A: TwitchApi + 'static> AuthService<A> {
    pub fn new(api: A, client_id: Option<String>, store: Box<dyn CredentialStore>) -> Self {
        let client_id = client_id.filter(|id| !id.trim().is_empty());
        let state = State {
            status: AuthStatus {
                phase: if client_id.is_some() {
                    AuthPhase::Restoring
                } else {
                    AuthPhase::NotConfigured
                },
                user: None,
                authorization: None,
                error: None,
                credential_storage: store.name().into(),
            },
            pending: None,
            store: CredentialVault::new(store),
            credentials: None,
            expected_user: None,
            restored: false,
            expires: None,
            next_validation: Instant::now(),
            next_refresh: Instant::now(),
            generation: 0,
            session_id: 0,
            session_cancel: CancellationToken::new(),
            last_validation: Ok(()),
            last_refresh: Ok(()),
        };
        let (snapshot, _) = watch::channel(Snapshot::from_state(&state));
        Self {
            api: Arc::new(api),
            client_id,
            state: Arc::new(Mutex::new(state)),
            snapshot,
            grant_cancel: Arc::new(StdMutex::new(CancellationToken::new())),
            validation_revision: Arc::new(AtomicU64::new(0)),
            refresh_revision: Arc::new(AtomicU64::new(0)),
            closing: Arc::new(AtomicBool::new(false)),
        }
    }
    // The service owns each operation independently of its IPC/HTTP caller.
    // Dropping a caller cannot interrupt a one-use refresh or reorder a pending
    // blocking keychain write behind logout. Device cancellation is explicit.
    async fn owned<
        T: Send + 'static,
        F: std::future::Future<Output = Result<T>> + Send + 'static,
    >(
        &self,
        operation: impl FnOnce(Self) -> F + Send + 'static,
    ) -> Result<T> {
        if self.closing.load(Ordering::SeqCst) {
            return Err(error(ErrorCode::Cancelled));
        }
        let service = self.clone();
        tokio::spawn(async move { operation(service).await })
            .await
            .map_err(|_| error(ErrorCode::Internal))?
    }
    pub async fn login(&self) -> Result<AuthStatus> {
        self.owned(|service| async move { service.login_owned().await })
            .await
    }
    pub async fn cancel(&self) -> Result<AuthStatus> {
        self.owned(|service| async move { service.cancel_owned().await })
            .await
    }
    pub async fn validate(&self) -> Result<AuthStatus> {
        self.owned(|service| async move { service.validate_owned().await })
            .await
    }
    pub async fn refresh(&self) -> Result<AuthStatus> {
        self.owned(|service| async move { service.refresh_owned().await })
            .await
    }
    pub async fn logout(&self) -> Result<AuthStatus> {
        self.owned(|service| async move { service.logout_owned().await })
            .await
    }
    pub async fn tick(&self) -> Result<()> {
        self.owned(|service| async move { service.tick_owned().await })
            .await
    }
    pub(crate) async fn lease(&self) -> Result<AccessLease> {
        self.owned(|service| async move { service.lease_owned(None).await })
            .await
    }
    pub(crate) async fn lease_for_session(&self, session_id: u64) -> Result<AccessLease> {
        self.owned(move |service| async move { service.lease_owned(Some(session_id)).await })
            .await
    }
    pub(crate) async fn refresh_rejected(&self, session_id: u64, generation: u64) -> Result<()> {
        self.owned(move |service| async move {
            service.refresh_rejected_owned(session_id, generation).await
        })
        .await
    }
    pub(crate) async fn reject(&self, session_id: u64, generation: u64) -> Result<()> {
        self.owned(move |service| async move { service.reject_owned(session_id, generation).await })
            .await
    }
    async fn lock(&self) -> Result<StateGuard<'_>> {
        let state = self.state.lock().await;
        if self.closing.load(Ordering::SeqCst) {
            return Err(error(ErrorCode::Cancelled));
        }
        Ok(StateGuard {
            state,
            snapshot: &self.snapshot,
        })
    }
    /// Finish an already-dispatched rotation/keychain operation before normal
    /// desktop exit. Closing must never abandon a successfully rotated pair.
    pub async fn shutdown(&self) {
        self.closing.store(true, Ordering::SeqCst);
        self.grant_cancel().cancel();
        let state = self.state.lock().await;
        state.session_cancel.cancel();
    }
    fn publish(&self, state: &State) {
        self.snapshot.send_replace(Snapshot::from_state(state));
    }
    pub async fn status(&self) -> AuthStatus {
        let snapshot = self.snapshot.borrow().clone();
        let mut status = snapshot.status;
        if let (Some(auth), Some(expires)) = (&mut status.authorization, snapshot.grant_expires) {
            auth.expires_in = expires.saturating_duration_since(Instant::now()).as_secs() as u32;
        }
        if let (Some(user), Some(expires)) = (&mut status.user, snapshot.expires) {
            user.expires_in = expires.saturating_duration_since(Instant::now()).as_secs() as u32;
        }
        status
    }
    fn grant_cancel(&self) -> CancellationToken {
        self.grant_cancel
            .lock()
            .expect("grant mutex poisoned")
            .clone()
    }
    fn client_id(&self) -> Result<&str> {
        self.client_id.as_deref().ok_or_else(|| AppError::new(ErrorCode::NotConfigured,
            "Register a public Twitch application and set TWITCH_CLIENT_ID in the backend environment. No client secret is used."))
    }
    async fn restore_locked(&self, state: &mut State) -> Result<()> {
        if state.restored {
            return Ok(());
        }
        state.next_validation = Instant::now() + Duration::from_secs(60);
        match state.store.load().await {
            Ok(credentials) => state.credentials = credentials,
            Err(error) => {
                state.status.phase = AuthPhase::Error;
                state.status.error = Some(error.clone());
                return Err(error);
            }
        }
        state.restored = true;
        if state.credentials.is_none() {
            state.status.phase = AuthPhase::SignedOut;
            state.status.error = None;
            return Ok(());
        }
        state.session_id += 1;
        state.generation += 1;
        // Restoration and later validation retries share the same bounded
        // recovery path, including reconnect after an offline startup.
        self.validate_locked(state).await
    }

    async fn login_owned(&self) -> Result<AuthStatus> {
        let client = self.client_id()?;
        let mut state = self.lock().await?;
        self.restore_locked(&mut state).await?;
        if state.pending.is_some() {
            return Ok(state.status.clone());
        }
        if state.credentials.is_some() {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Log out before starting a new authorization.",
            ));
        }
        let cancel = CancellationToken::new();
        *self.grant_cancel.lock().expect("grant mutex poisoned") = cancel.clone();
        state.status.phase = AuthPhase::Authorizing;
        state.status.error = None;
        self.publish(&state);
        let grant = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(error(ErrorCode::Cancelled)),
            grant = self.api.begin(client) => grant,
        };
        let grant = match grant {
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
            let error = error(ErrorCode::InvalidResponse);
            state.status.phase = AuthPhase::Error;
            state.status.error = Some(error.clone());
            return Err(error);
        }
        let now = Instant::now();
        let interval = Duration::from_secs(grant.interval.max(5).into());
        state.pending = Some(Pending {
            device_code: Zeroizing::new(grant.device_code),
            expires: now + Duration::from_secs(grant.expires_in.into()),
            next_poll: now + interval,
            interval,
        });
        state.status.authorization = Some(DeviceAuthorization {
            user_code: grant.user_code,
            verification_uri: grant.verification_uri,
            expires_in: grant.expires_in,
        });
        Ok(state.status.clone())
    }
    async fn cancel_owned(&self) -> Result<AuthStatus> {
        let authorizing = self.snapshot.borrow().status.phase == AuthPhase::Authorizing;
        self.grant_cancel().cancel();
        let mut state = self.lock().await?;
        if authorizing || state.status.phase == AuthPhase::Authorizing {
            state.session_cancel.cancel();
            state.credentials = None;
            state.expected_user = None;
            state.expires = None;
            state.status.user = None;
            state.pending = None;
            state.status.authorization = None;
            if let Err(error) = state.store.clear().await {
                state.status.phase = AuthPhase::Error;
                state.status.error = Some(error.clone());
                return Err(error);
            }
            state.status.phase = AuthPhase::Cancelled;
            state.status.error = None;
        }
        Ok(state.status.clone())
    }
    pub async fn verification_uri(&self) -> Result<String> {
        let snapshot = self.snapshot.borrow().clone();
        if snapshot
            .grant_expires
            .is_some_and(|expiry| expiry > Instant::now())
        {
            if let Some(auth) = snapshot.status.authorization {
                return Ok(auth.verification_uri);
            }
        }
        Err(AppError::new(
            ErrorCode::AuthExpired,
            "Start login to obtain a current authorization link.",
        ))
    }
    async fn validate_owned(&self) -> Result<AuthStatus> {
        self.client_id()?;
        let revision = self.validation_revision.load(Ordering::SeqCst);
        let mut state = self.lock().await?;
        self.restore_locked(&mut state).await?;
        if revision != self.validation_revision.load(Ordering::SeqCst) {
            state.last_validation.clone()?;
        } else {
            self.validate_locked(&mut state).await?;
        }
        Ok(state.status.clone())
    }
    async fn refresh_owned(&self) -> Result<AuthStatus> {
        self.client_id()?;
        let revision = self.refresh_revision.load(Ordering::SeqCst);
        let mut state = self.lock().await?;
        self.restore_locked(&mut state).await?;
        if revision != self.refresh_revision.load(Ordering::SeqCst) {
            state.last_refresh.clone()?;
        } else {
            self.refresh_locked(&mut state).await?;
        }
        Ok(state.status.clone())
    }
    async fn logout_owned(&self) -> Result<AuthStatus> {
        self.grant_cancel().cancel();
        let mut state = self.lock().await?;
        state.session_cancel.cancel();
        let credentials = state.credentials.take();
        state.expected_user = None;
        state.restored = true;
        state.pending = None;
        state.expires = None;
        state.status.user = None;
        state.status.authorization = None;
        state.status.error = None;
        if let Err(error) = state.store.clear().await {
            state.status.phase = AuthPhase::Error;
            state.status.error = Some(error.clone());
            return Err(error);
        }
        state.status.phase = if self.client_id.is_some() {
            AuthPhase::SignedOut
        } else {
            AuthPhase::NotConfigured
        };
        self.publish(&state);
        if let (Some(credentials), Some(client)) = (credentials, &self.client_id) {
            if self
                .api
                .revoke(client, &credentials.access_token)
                .await
                .is_err()
            {
                state.status.error = Some(AppError::new(
                    ErrorCode::Network,
                    "Local credentials were deleted. Remote revocation could not be confirmed; disconnect the app in Twitch settings if needed.",
                ));
            }
        }
        Ok(state.status.clone())
    }
    async fn tick_owned(&self) -> Result<()> {
        let Some(client) = &self.client_id else {
            return Ok(());
        };
        let mut state = self.lock().await?;
        let now = Instant::now();
        if !state.restored {
            if now < state.next_validation {
                return Ok(());
            }
            return self.restore_locked(&mut state).await;
        }
        if let Some(pending) = &mut state.pending {
            if now >= pending.expires {
                state.pending = None;
                state.status.authorization = None;
                state.status.phase = AuthPhase::Expired;
                let error = AppError::new(
                    ErrorCode::AuthExpired,
                    "The authorization code expired. Start login again.",
                );
                state.status.error = Some(error.clone());
                return Err(error);
            }
            if now < pending.next_poll {
                return Ok(());
            }
            let cancel = self.grant_cancel();
            let response = tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(error(ErrorCode::Cancelled)),
                response = self.api.poll(client, &pending.device_code) => response,
            };
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
                    state.session_cancel.cancel();
                    state.session_cancel = CancellationToken::new();
                    state.session_id += 1;
                    self.accept_credentials(&mut state, credentials).await?;
                }
                Err(error)
                    if matches!(
                        error.code,
                        ErrorCode::Network | ErrorCode::Timeout | ErrorCode::TwitchServer
                    ) =>
                {
                    let pending = state.pending.as_mut().expect("pending grant");
                    pending.interval = (pending.interval * 2).min(Duration::from_secs(300));
                    pending.next_poll = Instant::now() + pending.interval;
                    state.status.error = Some(error);
                }
                Err(error) => return self.invalidate(&mut state, error).await,
            }
        } else if state.credentials.is_some() {
            if state.expires.is_some_and(|expiry| expiry <= now) && now >= state.next_refresh {
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
    async fn validate_locked(&self, state: &mut State) -> Result<()> {
        let result = self.check_token_locked(state).await;
        // Identity/client/scope mismatches already invalidate the credentials.
        // Only a provider rejection of an existing access token is recoverable.
        let result = match result {
            Err(error) if error.code == ErrorCode::AuthInvalid && state.credentials.is_some() => {
                self.refresh_locked(state).await
            }
            result => result,
        };
        state.last_validation = result.clone();
        self.validation_revision.fetch_add(1, Ordering::SeqCst);
        result
    }
    async fn check_token_locked(&self, state: &mut State) -> Result<()> {
        let token = state
            .credentials
            .as_ref()
            .ok_or_else(|| error(ErrorCode::Unauthenticated))?
            .access_token
            .clone();
        state.next_validation = Instant::now() + Duration::from_secs(60);
        match self.api.validate(&token).await {
            Ok(validation) => self.apply_validation(state, validation).await,
            Err(error) if error.code == ErrorCode::AuthInvalid => Err(error),
            Err(error) => {
                state.status.phase = AuthPhase::Error;
                state.status.user = None;
                state.status.error = Some(error.clone());
                Err(error)
            }
        }
    }
    async fn apply_validation(&self, state: &mut State, validation: Validation) -> Result<()> {
        if Some(&validation.client_id) != self.client_id.as_ref()
            || validation.user_id.is_empty()
            || validation.login.is_empty()
            || validation.expires_in == 0
            || state
                .expected_user
                .as_ref()
                .is_some_and(|id| id != &validation.user_id)
        {
            return self
                .invalidate(
                    state,
                    AppError::new(
                        ErrorCode::AuthInvalid,
                        "Token identity, expiry, or client ID did not validate. Sign in again.",
                    ),
                )
                .await;
        }
        if !REQUIRED_SCOPES
            .iter()
            .all(|scope| validation.scopes.iter().any(|granted| granted == scope))
        {
            return self.invalidate(state, error(ErrorCode::Unauthorized)).await;
        }
        state.status.phase = AuthPhase::Authenticated;
        state.status.error = None;
        state.expected_user = Some(validation.user_id.clone());
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
        if let Err(error) = state.store.save(credentials.clone()).await {
            return self.invalidate(state, error).await;
        }
        state.credentials = Some(credentials);
        state.generation += 1;
        // A newly issued pair gets validation only, never another refresh.
        let result = match self.check_token_locked(state).await {
            Err(error) if error.code == ErrorCode::AuthInvalid && state.credentials.is_some() => {
                self.invalidate(state, error).await
            }
            result => result,
        };
        state.last_validation = result.clone();
        self.validation_revision.fetch_add(1, Ordering::SeqCst);
        result
    }
    async fn refresh_locked(&self, state: &mut State) -> Result<()> {
        let client = self.client_id()?;
        let refresh = state
            .credentials
            .as_ref()
            .ok_or_else(|| error(ErrorCode::Unauthenticated))?
            .refresh_token
            .clone();
        state.next_refresh = Instant::now() + Duration::from_secs(60);
        // Clear the old one-use token BEFORE dispatch. A crash or ambiguous
        // response must never restore and reuse a possibly consumed token.
        if let Err(error) = state.store.clear().await {
            state.status.error = Some(error.clone());
            state.last_refresh = Err(error.clone());
            self.refresh_revision.fetch_add(1, Ordering::SeqCst);
            return Err(error);
        }
        state.credentials = None;
        state.status.phase = AuthPhase::Error;
        state.status.user = None;
        state.status.error = Some(AppError::new(
            ErrorCode::AuthInvalid,
            "The refresh was interrupted. Sign in again.",
        ));
        let result = match self.api.refresh(client, &refresh).await {
            Ok(next) => { state.expires = None; self.accept_credentials(state, next).await }
            Err(error) => self.invalidate(state, AppError::new(error.code, "Refresh failed or its outcome is uncertain. Local credentials were deleted; sign in again.")).await,
        };
        state.last_refresh = result.clone();
        self.refresh_revision.fetch_add(1, Ordering::SeqCst);
        result
    }
    async fn invalidate(&self, state: &mut State, error: AppError) -> Result<()> {
        state.session_cancel.cancel();
        state.credentials = None;
        state.pending = None;
        state.expected_user = None;
        state.expires = None;
        state.status.phase = if error.code == ErrorCode::AuthExpired {
            AuthPhase::Expired
        } else {
            AuthPhase::Error
        };
        state.status.user = None;
        state.status.authorization = None;
        state.status.error = Some(error.clone());
        if let Err(storage) = state.store.clear().await {
            state.status.error = Some(storage.clone());
            return Err(storage);
        }
        Err(error)
    }
    async fn lease_owned(&self, session_id: Option<u64>) -> Result<AccessLease> {
        let client_id = self.client_id()?.to_owned();
        let mut state = self.lock().await?;
        // Check before any validation/rotation: an old caller must not do work
        // on behalf of the replacement account while waiting for this lock.
        if session_id
            .is_some_and(|id| id != state.session_id || state.session_cancel.is_cancelled())
        {
            return Err(error(ErrorCode::Unauthenticated));
        }
        self.restore_locked(&mut state).await?;
        if state.credentials.is_none() {
            return Err(error(ErrorCode::Unauthenticated));
        }
        if state.expires.is_some_and(|expiry| expiry <= Instant::now()) {
            self.refresh_locked(&mut state).await?;
        } else if Instant::now() >= state.next_validation {
            self.validate_locked(&mut state).await?;
        }
        let user = state
            .status
            .user
            .as_ref()
            .filter(|_| state.status.phase == AuthPhase::Authenticated)
            .ok_or_else(|| error(ErrorCode::Unauthenticated))?;
        Ok(AccessLease {
            token: state
                .credentials
                .as_ref()
                .expect("validated credentials")
                .access_token
                .clone(),
            client_id,
            user_id: user.id.clone(),
            scopes: user.scopes.clone(),
            generation: state.generation,
            session_id: state.session_id,
            cancel: state.session_cancel.clone(),
        })
    }
    async fn refresh_rejected_owned(&self, session_id: u64, generation: u64) -> Result<()> {
        let mut state = self.lock().await?;
        if state.session_id != session_id
            || state.session_cancel.is_cancelled()
            || state.credentials.is_none()
        {
            return Err(error(ErrorCode::Unauthenticated));
        }
        if state.generation != generation {
            return Ok(());
        }
        self.refresh_locked(&mut state).await
    }
    async fn reject_owned(&self, session_id: u64, generation: u64) -> Result<()> {
        let mut state = self.lock().await?;
        if state.session_id == session_id && state.generation == generation {
            return self
                .invalidate(&mut state, error(ErrorCode::Unauthenticated))
                .await;
        }
        Ok(())
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
