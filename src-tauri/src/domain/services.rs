#[cfg(any(test, feature = "test-support"))]
use super::LaunchRequest;
use super::{AppError, ErrorCode, Result};
use crate::streamlink::{
    discovery::{PlayerDiscovery, SearchLocations, discover_players, resolve_player},
    playback::{
        LaunchSpec, PlaybackRequest, PlaybackStream, QualityPolicy, RestartRequest, check_version,
    },
};
use crate::{
    config::{Settings, SettingsStore},
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
    pub settings: Arc<SettingsStore>,
    pub sessions: Supervisor,
    pub auth: Arc<AuthService>,
    pub helix: HelixClient,
    chat: super::chat::BrowserChat,
    streamlink_operation: Mutex<()>,
    closing: AtomicBool,
    auth_configured: bool,
    browse_slots: Arc<tokio::sync::Semaphore>,
    playback_slots: Arc<tokio::sync::Semaphore>,
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
            settings: Arc::new(SettingsStore::open(settings_directory)?),
            sessions: Supervisor::default(),
            helix: HelixClient::new(http, auth.clone()),
            auth,
            chat: super::chat::BrowserChat::default(),
            streamlink_operation: Mutex::new(()),
            closing: AtomicBool::new(false),
            auth_configured,
            browse_slots: Arc::new(tokio::sync::Semaphore::new(8)),
            playback_slots: Arc::new(tokio::sync::Semaphore::new(8)),
        })
    }

    pub fn with_chat_opener(mut self, opener: Arc<dyn super::chat::ChatOpener>) -> Self {
        self.chat = super::chat::BrowserChat::new(opener);
        self
    }

    pub async fn open_chat(&self, request: super::chat::ChatRequest) -> Result<()> {
        let _permit = self.browse_permit()?;
        let login = self
            .helix
            .chat_login(
                request.auth_session_id.clone(),
                request.broadcaster_id,
                &tokio_util::sync::CancellationToken::new(),
            )
            .await?;
        let session = request.auth_session_id.parse().map_err(|_| {
            AppError::new(ErrorCode::InvalidInput, "Invalid authentication session.")
        })?;
        let lease = self.auth.lease_for_session(session).await?;
        self.ensure_open()?;
        self.chat
            .open(super::chat::ChatTarget::for_login(&login)?, lease.cancel)
            .await
    }

    async fn automatic_chat(
        &self,
        snapshot: SessionSnapshot,
        cancel: tokio_util::sync::CancellationToken,
    ) -> SessionSnapshot {
        if !snapshot
            .effective_settings
            .as_ref()
            .is_some_and(|settings| settings.automatic_chat)
        {
            return snapshot;
        }
        let Some(stream) = &snapshot.stream else {
            return snapshot;
        };
        let result = match super::chat::ChatTarget::for_login(&stream.login) {
            Ok(target) => self.chat.open(target, cancel).await,
            Err(error) => Err(error),
        };
        self.sessions
            .record_chat_result(
                &snapshot.id,
                snapshot.generation,
                result.err().map(|error| error.code),
            )
            .await
            .unwrap_or(snapshot)
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
        check_version(&result.version)?;
        let settings = self.settings.clone();
        tokio::task::spawn_blocking(move || settings.set_streamlink_path(custom_path))
            .await
            .map_err(|_| AppError::new(ErrorCode::Settings, "Settings operation failed."))??;
        Ok(result)
    }

    pub fn players(&self) -> PlayerDiscovery {
        discover_players(&SearchLocations::system())
    }

    pub async fn save_settings(&self, settings: Settings) -> Result<Settings> {
        settings.validate()?;
        self.ensure_open()?;
        let _operation = self.streamlink_operation.lock().await;
        self.ensure_open()?;
        if let Some(path) = &settings.streamlink_path {
            let probe = streamlink::probe(Some(path), Duration::from_secs(5)).await?;
            check_version(&probe.version)?;
        }
        resolve_player(&settings.player, &SearchLocations::system())?;
        self.ensure_open()?;
        let store = self.settings.clone();
        tokio::task::spawn_blocking(move || store.update(settings))
            .await
            .map_err(|_| AppError::new(ErrorCode::Settings, "Settings operation failed."))?
    }

    pub async fn save_channel_settings(
        &self,
        request: crate::config::SaveChannelSettingsRequest,
    ) -> Result<crate::config::ChannelSettings> {
        self.ensure_open()?;
        let store = self.settings.clone();
        tokio::task::spawn_blocking(move || store.set_channel(request))
            .await
            .map_err(|_| AppError::new(ErrorCode::Settings, "Settings operation failed."))?
    }

    async fn prepare_playback(
        &self,
        stream: PlaybackStream,
        quality: Option<QualityPolicy>,
    ) -> Result<LaunchSpec> {
        self.ensure_open()?;
        let settings = self.settings.effective(&stream.broadcaster_id, quality)?;
        let player = resolve_player(&settings.player, &SearchLocations::system())?;
        let probe =
            streamlink::probe(settings.streamlink_path.as_deref(), Duration::from_secs(5)).await?;
        check_version(&probe.version)?;
        self.ensure_open()?;
        Ok(LaunchSpec {
            executable: probe.executable.into(),
            player,
            settings,
            stream,
        })
    }

    fn playback_permit(&self) -> Result<tokio::sync::OwnedSemaphorePermit> {
        self.ensure_open()?;
        self.playback_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| AppError::new(ErrorCode::Capacity, "Playback is busy. Try again shortly."))
    }

    pub async fn play(self: &Arc<Self>, request: PlaybackRequest) -> Result<SessionSnapshot> {
        let permit = self.playback_permit()?;
        let services = self.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let stream = services
                .helix
                .playback_stream(
                    request.auth_session_id.clone(),
                    request.broadcaster_id,
                    &tokio_util::sync::CancellationToken::new(),
                )
                .await?;
            let _operation = services.streamlink_operation.lock().await;
            let spec = services
                .prepare_playback(stream.into(), request.quality)
                .await?;
            let session = request.auth_session_id.parse().map_err(|_| {
                AppError::new(ErrorCode::InvalidInput, "Invalid authentication session.")
            })?;
            // A logout while discovery/probing was pending must not launch a stale request.
            let lease = services.auth.lease_for_session(session).await?;
            services.ensure_open()?;
            let snapshot = services
                .sessions
                .launch_authenticated(spec, &lease.cancel)
                .await?;
            drop(_operation);
            Ok(services.automatic_chat(snapshot, lease.cancel).await)
        })
        .await
        .map_err(|_| {
            AppError::new(
                ErrorCode::Internal,
                "Playback operation ended unexpectedly.",
            )
        })?
    }

    pub async fn restart_playback(
        self: &Arc<Self>,
        request: RestartRequest,
    ) -> Result<SessionSnapshot> {
        let permit = self.playback_permit()?;
        let services = self.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let snapshot = services
                .sessions
                .restart(&request.session_id, request.generation, |stream| async {
                    let _operation = services.streamlink_operation.lock().await;
                    services.prepare_playback(stream, request.quality).await
                })
                .await?;
            Ok(services
                .automatic_chat(snapshot, tokio_util::sync::CancellationToken::new())
                .await)
        })
        .await
        .map_err(|_| {
            AppError::new(
                ErrorCode::RestartFailed,
                "Restart operation ended unexpectedly.",
            )
        })?
    }

    /// Phase 0 contract adapter, retained for native process tests only.
    #[cfg(any(test, feature = "test-support"))]
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
        let (_, sessions, (), ()) = tokio::join!(
            self.streamlink_operation.lock(),
            self.sessions.shutdown(),
            self.auth.shutdown(),
            self.chat.shutdown(),
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
