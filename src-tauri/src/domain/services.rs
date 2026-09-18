use super::{AppError, ErrorCode, LaunchRequest, Result};
use crate::{
    config::SettingsStore,
    credentials::MemoryCredentialStore,
    diagnostics::BackendDiagnostics,
    streamlink::{self, ProbeResult, SessionSnapshot, Supervisor},
    twitch::{AuthService, HttpTwitchApi},
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
    streamlink_operation: Mutex<()>,
    closing: AtomicBool,
    auth_configured: bool,
}

impl Services {
    pub fn new(settings_directory: &Path, client_id: Option<String>) -> Result<Self> {
        let auth_configured = client_id.as_ref().is_some_and(|id| !id.trim().is_empty());
        Ok(Self {
            settings: SettingsStore::open(settings_directory)?,
            sessions: Supervisor::default(),
            auth: Arc::new(AuthService::new(
                HttpTwitchApi::new()?,
                client_id,
                Box::<MemoryCredentialStore>::default(),
            )),
            streamlink_operation: Mutex::new(()),
            closing: AtomicBool::new(false),
            auth_configured,
        })
    }

    pub fn diagnostics(&self) -> BackendDiagnostics {
        BackendDiagnostics {
            name: "Twitch GUI RS".into(),
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
        self.settings
            .set_streamlink_path(custom_path.map(|_| result.executable.clone()))?;
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
        let (_, sessions) =
            tokio::join!(self.streamlink_operation.lock(), self.sessions.shutdown());
        sessions
    }
}
