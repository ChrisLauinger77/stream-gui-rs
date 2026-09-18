use crate::{
    credentials::Credentials,
    domain::{AppError, ErrorCode, Result},
};
use reqwest::{Client, Response, StatusCode};
use serde::{Deserialize, de::DeserializeOwned};
use std::{future::Future, time::Duration};
use zeroize::Zeroizing;

// No privileged scopes are needed to prove identity via /validate.
const SCOPES: &str = "";
const DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";

#[derive(Deserialize)]
pub struct DeviceGrant {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u32,
    pub interval: u32,
}

#[derive(Deserialize)]
pub struct TokenResponse {
    access_token: String,
    refresh_token: String,
    token_type: String,
}

impl TokenResponse {
    fn into_credentials(self) -> Result<Credentials> {
        if !self.token_type.eq_ignore_ascii_case("bearer")
            || self.access_token.is_empty()
            || self.refresh_token.is_empty()
        {
            return Err(provider_error());
        }
        Ok(Credentials {
            access_token: Zeroizing::new(self.access_token),
            refresh_token: Zeroizing::new(self.refresh_token),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Validation {
    pub client_id: String,
    pub user_id: String,
    pub login: String,
    pub scopes: Vec<String>,
    pub expires_in: u32,
}

pub enum PollResult {
    Pending,
    SlowDown,
    Authorized(Credentials),
}

pub trait TwitchApi: Send + Sync {
    fn begin(&self, client_id: &str) -> impl Future<Output = Result<DeviceGrant>> + Send;
    fn poll(
        &self,
        client_id: &str,
        device_code: &str,
    ) -> impl Future<Output = Result<PollResult>> + Send;
    fn validate(&self, token: &str) -> impl Future<Output = Result<Validation>> + Send;
    fn refresh(
        &self,
        client_id: &str,
        token: &str,
    ) -> impl Future<Output = Result<Credentials>> + Send;
    fn revoke(&self, client_id: &str, token: &str) -> impl Future<Output = Result<()>> + Send;
}

pub struct HttpTwitchApi {
    client: Client,
    base: String,
}

impl HttpTwitchApi {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("twitch-gui-rs/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| provider_error())?;
        Ok(Self {
            client,
            base: "https://id.twitch.tv/oauth2".into(),
        })
    }
}

impl TwitchApi for HttpTwitchApi {
    async fn begin(&self, client_id: &str) -> Result<DeviceGrant> {
        let response = self
            .client
            .post(format!("{}/device", self.base))
            .form(&[("client_id", client_id), ("scopes", SCOPES)])
            .send()
            .await
            .map_err(|_| network_error())?;
        if !response.status().is_success() {
            return Err(provider_error());
        }
        decode(response).await
    }

    async fn poll(&self, client_id: &str, device_code: &str) -> Result<PollResult> {
        let response = self
            .client
            .post(format!("{}/token", self.base))
            .form(&[
                ("client_id", client_id),
                ("scopes", SCOPES),
                ("device_code", device_code),
                ("grant_type", DEVICE_GRANT),
            ])
            .send()
            .await
            .map_err(|_| network_error())?;
        let status = response.status();
        if status.is_success() {
            return Ok(PollResult::Authorized(
                decode::<TokenResponse>(response)
                    .await?
                    .into_credentials()?,
            ));
        }
        if status == StatusCode::TOO_MANY_REQUESTS {
            return Ok(PollResult::SlowDown);
        }
        if status.is_server_error() {
            return Err(network_error());
        }
        let error: OAuthError = decode(response).await?;
        match error.error.as_deref().or(error.message.as_deref()) {
            Some("authorization_pending") => Ok(PollResult::Pending),
            Some("slow_down") => Ok(PollResult::SlowDown),
            Some("access_denied") => Err(AppError::new(
                ErrorCode::AuthDenied,
                "Twitch authorization was declined.",
            )),
            Some("expired_token" | "invalid device code") => Err(AppError::new(
                ErrorCode::AuthExpired,
                "The device authorization expired or was already used. Start login again.",
            )),
            _ => Err(provider_error()),
        }
    }

    async fn validate(&self, token: &str) -> Result<Validation> {
        let mut authorization = reqwest::header::HeaderValue::from_str(&format!("OAuth {token}"))
            .map_err(|_| provider_error())?;
        authorization.set_sensitive(true);
        let response = self
            .client
            .get(format!("{}/validate", self.base))
            .header(reqwest::header::AUTHORIZATION, authorization)
            .send()
            .await
            .map_err(|_| network_error())?;
        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(AppError::new(
                ErrorCode::AuthInvalid,
                "Twitch rejected the token. Sign in again.",
            ));
        }
        if !response.status().is_success() {
            return Err(network_error());
        }
        decode(response).await
    }

    async fn refresh(&self, client_id: &str, token: &str) -> Result<Credentials> {
        let response = self
            .client
            .post(format!("{}/token", self.base))
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", client_id),
                ("refresh_token", token),
            ])
            .send()
            .await
            .map_err(|_| network_error())?;
        if matches!(
            response.status(),
            StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED
        ) {
            return Err(AppError::new(
                ErrorCode::AuthInvalid,
                "Twitch refresh is no longer valid. Sign in again.",
            ));
        }
        if !response.status().is_success() {
            return Err(network_error());
        }
        decode::<TokenResponse>(response).await?.into_credentials()
    }

    async fn revoke(&self, client_id: &str, token: &str) -> Result<()> {
        let response = self
            .client
            .post(format!("{}/revoke", self.base))
            .form(&[("client_id", client_id), ("token", token)])
            .send()
            .await
            .map_err(|_| network_error())?;
        if response.status().is_success() || response.status() == StatusCode::BAD_REQUEST {
            Ok(())
        } else {
            Err(provider_error())
        }
    }
}

#[derive(Deserialize)]
struct OAuthError {
    error: Option<String>,
    message: Option<String>,
}

async fn decode<T: DeserializeOwned>(mut response: Response) -> Result<T> {
    // Never log or return provider bodies: success bodies contain credentials,
    // and failure bodies are untrusted. Bound buffering before JSON decoding.
    let mut body = Zeroizing::new(Vec::new());
    while let Some(chunk) = response.chunk().await.map_err(|_| network_error())? {
        if body.len() + chunk.len() > 64 * 1024 {
            return Err(provider_error());
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|_| provider_error())
}

fn network_error() -> AppError {
    AppError::new(
        ErrorCode::Network,
        "Twitch could not be reached or returned a temporary failure. Retry shortly.",
    )
}
fn provider_error() -> AppError {
    AppError::new(
        ErrorCode::AuthProvider,
        "Twitch returned an unexpected response. Check that TWITCH_CLIENT_ID belongs to a public application.",
    )
}

#[cfg(test)]
mod tests;
