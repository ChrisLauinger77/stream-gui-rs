use super::{AppError, ErrorCode, LaunchRequest, Result};
use crate::{
    config::SettingsStore,
    credentials::{CredentialStore, UnavailableCredentialStore},
    diagnostics::BackendDiagnostics,
    helix::HelixClient,
    streamlink::{self, ProbeResult, SessionSnapshot, Supervisor},
    twitch::{AuthService, HttpTwitchApi},
    twitch_http::TwitchHttp,
};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::Mutex;

pub struct Services {
    pub settings: SettingsStore,
    pub sessions: Supervisor,
    pub auth: Arc<AuthService>,
    pub helix: HelixClient,
    streamlink_operation: Mutex<()>,
    closing: AtomicBool,
    auth_configured: bool,
    browse_slots: Arc<tokio::sync::Semaphore>,
}

impl Services {
    pub fn new(settings_directory: &Path, client_id: Option<String>) -> Result<Self> {
        let auth_configured = client_id.as_ref().is_some_and(|id| !id.trim().is_empty());
        let http = TwitchHttp::new()?;
        let store: Box<dyn CredentialStore> = match client_id
            .as_deref()
            .filter(|id| !id.trim().is_empty())
        {
            Some(id) => {
                #[cfg(any(target_os = "macos", target_os = "linux", windows))]
                {
                    match crate::credentials::PlatformCredentialStore::open(settings_directory, id)
                    {
                        Ok(store) => Box::new(store),
                        Err(error) => Box::new(UnavailableCredentialStore(error)),
                    }
                }
                #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
                {
                    let _ = id;
                    Box::new(UnavailableCredentialStore(AppError::new(
                        ErrorCode::CredentialStore,
                        "Secure storage is not supported on this platform.",
                    )))
                }
            }
            None => Box::new(UnavailableCredentialStore(AppError::new(
                ErrorCode::NotConfigured,
                "Configure a public Twitch client ID first.",
            ))),
        };
        let auth = Arc::new(AuthService::new(
            HttpTwitchApi::with_http(&http),
            client_id,
            store,
        ));
        Ok(Self {
            settings: SettingsStore::open(settings_directory)?,
            sessions: Supervisor::default(),
            helix: HelixClient::new(http, auth.clone()),
            auth,
            streamlink_operation: Mutex::new(()),
            closing: AtomicBool::new(false),
            auth_configured,
            browse_slots: Arc::new(tokio::sync::Semaphore::new(8)),
        })
    }

    pub fn browse_permit(&self) -> Result<tokio::sync::OwnedSemaphorePermit> {
        self.ensure_open()?;
        self.browse_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| AppError::new(ErrorCode::Capacity, "Browsing is busy. Try again shortly."))
    }

    pub fn diagnostics(&self) -> BackendDiagnostics {
        BackendDiagnostics {
            name: "Stream GUI RS".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            platform: format!("{} / {}", std::env::consts::OS, std::env::consts::ARCH),
            settings_path: self.settings.path().to_string_lossy().into_owned(),
            settings: self.settings.snapshot(),
            auth_configured: self.auth_configured,
        }
    }

    pub async fn probe(&self, custom_path: Option<String>) -> Result<ProbeResult> {
        self.ensure_open()?;
        let _operation = self.streamlink_operation.lock().await;
        self.ensure_open()?;
        let custom_path = custom_path.filter(|p| !p.trim().is_empty());
        let result = streamlink::probe(custom_path.as_deref(), Duration::from_secs(5)).await?;
        self.settings.set_streamlink_path(custom_path)?;
        Ok(result)
    }

    pub async fn launch(&self, request: LaunchRequest) -> Result<SessionSnapshot> {
        // Validate arguments before any process is started, then re-probe the
        // configured executable instead of accepting a frontend executable/argv.
        streamlink::build_arguments(&request)?;
        self.ensure_open()?;
        let _operation = self.streamlink_operation.lock().await;
        self.ensure_open()?;
        let path = self.settings.snapshot().streamlink_path;
        let result = streamlink::probe(path.as_deref(), Duration::from_secs(5)).await?;
        self.ensure_open()?;
        self.sessions
            .launch(Path::new(&result.executable), request)
            .await
    }

    fn ensure_open(&self) -> Result<()> {
        if self.closing.load(Ordering::SeqCst) {
            return Err(AppError::new(
                ErrorCode::ProcessFailed,
                "The application is shutting down.",
            ));
        }
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.closing.store(true, Ordering::SeqCst);
        // Probe owns its child until it exits or its five-second timeout kills
        // and reaps it. Wait for that ownership to end before Tauri exits, while
        // stopping playback immediately. Queued operations recheck closing.
        let (_, sessions, ()) = tokio::join!(
            self.streamlink_operation.lock(),
            self.sessions.shutdown(),
            self.auth.shutdown(),
        );
        sessions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn browsing_capacity_is_bounded_released_and_closed_on_shutdown() {
        let directory = tempfile::tempdir().unwrap();
        let services = Services::new(directory.path(), None).unwrap();
        let mut permits: Vec<_> = (0..8).map(|_| services.browse_permit().unwrap()).collect();
        assert_eq!(
            services.browse_permit().unwrap_err().code,
            ErrorCode::Capacity
        );
        permits.pop();
        let replacement = services.browse_permit().unwrap();
        services.shutdown().await.unwrap();
        drop(replacement);
        assert_eq!(
            services.browse_permit().unwrap_err().code,
            ErrorCode::ProcessFailed
        );
    }
}
