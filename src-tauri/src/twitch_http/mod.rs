//! Shared connection pool and conservative Helix request policy. OAuth token
//! exchanges use this same pool but are never automatically retried.
pub mod rate;
use crate::domain::{AppError, ErrorCode, Result};
use rate::RateLimiter;
use reqwest::{
    Client, Response, StatusCode,
    header::{AUTHORIZATION, HeaderValue},
};
use serde::de::DeserializeOwned;
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;

#[derive(Clone)]
pub struct TwitchHttp {
    pub(crate) client: Client,
    pub(crate) oauth_base_url: String,
    helix_base: url::Url,
    pub rate: Arc<RateLimiter>,
}
impl TwitchHttp {
    pub fn new() -> Result<Self> {
        Self::build(
            "https://id.twitch.tv/oauth2",
            "https://api.twitch.tv/helix/",
            Duration::from_secs(15),
        )
    }
    fn build(oauth_base_url: &str, helix: &str, timeout: Duration) -> Result<Self> {
        Ok(Self {
            client: Client::builder()
                .timeout(timeout)
                .connect_timeout(Duration::from_secs(5))
                .redirect(reqwest::redirect::Policy::none())
                .user_agent(concat!("stream-gui-rs/", env!("CARGO_PKG_VERSION")))
                .build()
                .map_err(|_| error(ErrorCode::Internal))?,
            oauth_base_url: oauth_base_url.into(),
            helix_base: url::Url::parse(helix).map_err(|_| error(ErrorCode::Internal))?,
            rate: Arc::default(),
        })
    }
    #[cfg(test)]
    pub(crate) fn for_test(base: &str, timeout: Duration) -> Self {
        Self::build(base, &format!("{base}/helix/"), timeout).unwrap()
    }

    #[cfg(test)]
    pub(crate) async fn get(
        &self,
        endpoint: &str,
        query: &[(String, String)],
        client_id: &str,
        token: &str,
        cancel: &CancellationToken,
    ) -> Result<Vec<u8>> {
        self.get_with_priority(endpoint, query, client_id, token, cancel, false)
            .await
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn get_with_priority(
        &self,
        endpoint: &str,
        query: &[(String, String)],
        client_id: &str,
        token: &str,
        cancel: &CancellationToken,
        background: bool,
    ) -> Result<Vec<u8>> {
        // Endpoint is selected by Rust endpoint methods, never by IPC input.
        let mut url = self
            .helix_base
            .join(endpoint)
            .map_err(|_| error(ErrorCode::InvalidInput))?;
        url.query_pairs_mut().extend_pairs(query);
        let mut authorization = HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|_| error(ErrorCode::Unauthenticated))?;
        authorization.set_sensitive(true);
        for attempt in 0..2 {
            let reservation = self.rate.reserve_with_priority(cancel, background).await?;
            let response = tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(error(ErrorCode::Cancelled)),
                response = self.client.get(url.clone()).header(AUTHORIZATION, authorization.clone()).header("Client-Id", client_id).send() => response,
            };
            let mut retryable = true;
            let result = match response {
                Ok(response) => {
                    let status = response.status();
                    self.rate
                        .observe(response.headers(), status == StatusCode::TOO_MANY_REQUESTS);
                    if status.is_success() {
                        tokio::select! {
                            biased;
                            _ = cancel.cancelled() => Err(error(ErrorCode::Cancelled)),
                            body = read_body(response, 4 * 1024 * 1024) => body.map(|body| body.to_vec()),
                        }
                    } else {
                        retryable = matches!(status.as_u16(), 500 | 502 | 503 | 504);
                        Err(error(match status.as_u16() {
                            401 => ErrorCode::Unauthenticated,
                            403 => ErrorCode::Unauthorized,
                            429 => ErrorCode::RateLimited,
                            500..=599 => ErrorCode::TwitchServer,
                            400 | 404 => ErrorCode::InvalidInput,
                            _ => ErrorCode::InvalidResponse,
                        }))
                    }
                }
                Err(error) => Err(network_error(&error)),
            };
            drop(reservation);
            match result {
                Err(ref error)
                    if attempt == 0
                        && retryable
                        && matches!(
                            error.code,
                            ErrorCode::Network | ErrorCode::Timeout | ErrorCode::TwitchServer
                        ) =>
                {
                    tokio::select! {
                        biased;
                        _ = cancel.cancelled() => return Err(self::error(ErrorCode::Cancelled)),
                        _ = tokio::time::sleep(Duration::from_millis(250)) => {},
                    }
                }
                result => return result,
            }
        }
        unreachable!("bounded retry returns on its final attempt")
    }
}

pub(crate) async fn read_body(mut response: Response, limit: usize) -> Result<Zeroizing<Vec<u8>>> {
    let mut bytes = Zeroizing::new(Vec::new());
    while let Some(chunk) = response.chunk().await.map_err(|e| network_error(&e))? {
        if bytes.len() + chunk.len() > limit {
            return Err(error(ErrorCode::InvalidResponse));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
pub(crate) async fn decode<T: DeserializeOwned>(response: Response, limit: usize) -> Result<T> {
    serde_json::from_slice(&read_body(response, limit).await?)
        .map_err(|_| error(ErrorCode::InvalidResponse))
}
pub(crate) fn network_error(error: &reqwest::Error) -> AppError {
    self::error(if error.is_timeout() {
        ErrorCode::Timeout
    } else {
        ErrorCode::Network
    })
}
pub(crate) fn error(code: ErrorCode) -> AppError {
    let message = match code {
        ErrorCode::Unauthenticated => "Sign in to Twitch to continue.",
        ErrorCode::Unauthorized => {
            "Twitch permission is missing or insufficient. Sign in again to grant the required scope."
        }
        ErrorCode::RateLimited => {
            "Twitch's request budget is exhausted. Wait for the shared rate limit to reset."
        }
        ErrorCode::Network => {
            "Twitch could not be reached. Check the network connection and retry."
        }
        ErrorCode::Timeout => "The Twitch request timed out.",
        ErrorCode::InvalidResponse => "Twitch returned an invalid or oversized response.",
        ErrorCode::TwitchServer => "Twitch returned a temporary server error.",
        ErrorCode::Cancelled => "The Twitch request was cancelled.",
        ErrorCode::InvalidInput => {
            "The Twitch request parameters are invalid or the resource does not exist."
        }
        _ => "The Twitch operation could not complete.",
    };
    AppError::new(code, message)
}

#[cfg(test)]
mod tests;
