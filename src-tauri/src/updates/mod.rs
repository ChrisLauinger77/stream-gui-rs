//! Manual, fixed-project release awareness. Independent of Twitch HTTP/credentials.
use crate::{
    domain::{AppError, ErrorCode, Result},
    time::Clock,
};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use ts_rs::TS;

const ENDPOINT: &str = "https://api.github.com/repos/ChrisLauinger77/stream-gui-rs/releases/latest";
const RELEASE_PREFIX: &str = "https://github.com/ChrisLauinger77/stream-gui-rs/releases/tag/";
const MAX_BODY: usize = 256 * 1024;
const TTL: Duration = Duration::from_secs(24 * 60 * 60);
const REFRESH_INTERVAL: Duration = Duration::from_secs(60);
const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum UpdatePhase {
    #[default]
    NotChecked,
    Checking,
    Current,
    Available,
    Development,
    NoStableRelease,
    Unavailable,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub phase: UpdatePhase,
    pub latest_version: Option<String>,
}
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    draft: bool,
    prerelease: bool,
}
fn release_url(tag: &str) -> Result<String> {
    let version = tag
        .strip_prefix('v')
        .and_then(|v| Version::parse(v).ok())
        .filter(|v| v.pre.is_empty() && v.build.is_empty());
    if version.is_none() || tag.len() > 64 {
        return Err(unavailable());
    }
    Ok(format!("{RELEASE_PREFIX}{tag}"))
}
fn parse_release(body: &[u8], current: &str) -> Result<UpdateStatus> {
    let release: Release = serde_json::from_slice(body).map_err(|_| unavailable())?;
    let remote = release
        .tag_name
        .strip_prefix('v')
        .filter(|v| v.len() <= 63)
        .and_then(|v| Version::parse(v).ok())
        .ok_or_else(unavailable)?;
    if release.draft || release.prerelease || !remote.pre.is_empty() {
        return Ok(UpdateStatus {
            phase: UpdatePhase::NoStableRelease,
            latest_version: None,
        });
    }
    if release.html_url != release_url(&release.tag_name)? {
        return Err(unavailable());
    }
    let current = Version::parse(current).map_err(|_| unavailable())?;
    use std::cmp::Ordering;
    let phase = match current.cmp_precedence(&remote) {
        Ordering::Less => UpdatePhase::Available,
        Ordering::Equal => UpdatePhase::Current,
        Ordering::Greater => UpdatePhase::Development,
    };
    Ok(UpdateStatus {
        phase,
        latest_version: Some(remote.to_string()),
    })
}
fn unavailable() -> AppError {
    AppError::new(ErrorCode::Network, "Unable to check for updates.")
}
#[derive(Default)]
struct Cache {
    status: UpdateStatus,
    completed: Option<Duration>,
}
/// Construction performs no HTTP or fallible client initialization; startup never waits.
pub struct Updates {
    cache: Mutex<Cache>,
    operation: Arc<tokio::sync::Mutex<()>>,
    clock: Clock,
    cancel: tokio_util::sync::CancellationToken,
    #[cfg(test)]
    endpoint: String,
    #[cfg(test)]
    timeout: Duration,
}
impl Default for Updates {
    fn default() -> Self {
        Self {
            cache: Mutex::new(Cache::default()),
            operation: Arc::default(),
            clock: Clock::default(),
            cancel: Default::default(),
            #[cfg(test)]
            endpoint: ENDPOINT.into(),
            #[cfg(test)]
            timeout: TIMEOUT,
        }
    }
}
impl Updates {
    pub fn status(&self) -> UpdateStatus {
        self.cache
            .lock()
            .expect("update cache poisoned")
            .status
            .clone()
    }
    pub fn release_destination(&self) -> Result<String> {
        let cache = self.cache.lock().expect("update cache poisoned");
        let version = cache
            .status
            .latest_version
            .as_ref()
            .ok_or_else(unavailable)?;
        // Reconstruct from the accepted structured version, never a remote URL or UI argument.
        release_url(&format!("v{version}"))
    }
    pub fn shutdown(&self) {
        self.cancel.cancel();
    }
    pub async fn check(self: &Arc<Self>, refresh: bool) -> UpdateStatus {
        let Ok(guard) = self.operation.clone().try_lock_owned() else {
            return self.status();
        };
        {
            let mut cache = self.cache.lock().expect("update cache poisoned");
            if self.cancel.is_cancelled()
                || cache.completed.is_some_and(|when| {
                    self.clock.now().saturating_sub(when)
                        < if refresh { REFRESH_INTERVAL } else { TTL }
                })
            {
                return cache.status.clone();
            }
            cache.status = UpdateStatus {
                phase: UpdatePhase::Checking,
                latest_version: None,
            };
        }
        let service = self.clone();
        // An IPC caller disappearing cannot clear the check/backoff or duplicate its HTTP.
        let _ = tokio::spawn(async move {
            let _guard = guard;
            let result = tokio::select! {
                _ = service.cancel.cancelled() => Err(unavailable()),
                result = service.fetch() => result,
            };
            let mut cache = service.cache.lock().expect("update cache poisoned");
            cache.status = result.unwrap_or(UpdateStatus {
                phase: UpdatePhase::Unavailable,
                latest_version: None,
            });
            cache.completed = Some(service.clock.now());
        })
        .await;
        self.status()
    }
    async fn fetch(&self) -> Result<UpdateStatus> {
        #[cfg(not(test))]
        let (endpoint, timeout) = (ENDPOINT, TIMEOUT);
        #[cfg(test)]
        let (endpoint, timeout) = (self.endpoint.as_str(), self.timeout);
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .timeout(timeout)
            .connect_timeout(timeout)
            .user_agent("Stream-GUI-RS-update-awareness")
            .build()
            .map_err(|_| unavailable())?;
        // No Authorization, client ID, cookies, environment/account data or Twitch client.
        let mut response = client
            .get(endpoint)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|_| unavailable())?;
        if response.status() != reqwest::StatusCode::OK
            || response
                .content_length()
                .is_some_and(|n| n > MAX_BODY as u64)
        {
            return Err(unavailable());
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| unavailable())? {
            if body.len().saturating_add(chunk.len()) > MAX_BODY {
                return Err(unavailable());
            }
            body.extend_from_slice(&chunk);
        }
        parse_release(&body, crate::build_info::VERSION)
    }
}
#[cfg(test)]
mod tests;
