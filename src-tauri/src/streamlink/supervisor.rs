use super::playback::{CommandSpec, LaunchSpec, PlaybackStream, QualityPolicy, build_command};
use crate::{
    domain::{AppError, ErrorCode, Result},
    platform::{ProcessTree, configure_process},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    process::Stdio,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::AsyncReadExt,
    process::Command,
    sync::{Mutex as AsyncMutex, watch},
    task::JoinHandle,
    time::timeout,
};
use ts_rs::TS;

#[cfg(any(test, feature = "test-support"))]
use {crate::domain::LaunchRequest, std::path::Path};

pub const MAX_LOG_ENTRIES: usize = 200;
pub const MAX_LINE_BYTES: usize = 2048;
const MAX_SESSIONS: usize = 16;
const MAX_ACTIVE: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SessionPhase {
    Starting,
    Running,
    Stopping,
    Exited,
    Failed,
}

impl SessionPhase {
    fn terminal(self) -> bool {
        matches!(self, Self::Exited | Self::Failed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum LogSource {
    Stdout,
    Stderr,
    Supervisor,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub sequence: u32,
    pub source: LogSource,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
    pub id: String,
    pub generation: u32,
    pub restarting: bool,
    pub stream: Option<PlaybackStream>,
    pub quality_policy: Option<QualityPolicy>,
    #[ts(type = "number")]
    pub started_at: u64,
    #[ts(type = "number | null")]
    pub ended_at: Option<u64>,
    pub failure: Option<ErrorCode>,
    pub phase: SessionPhase,
    pub pid: u32,
    pub url: String,
    pub quality: String,
    pub exit_code: Option<i32>,
    pub stop_requested: bool,
    #[ts(as = "Vec<LogEntry>")]
    pub logs: VecDeque<LogEntry>,
    pub dropped_log_entries: u32,
}

impl SessionSnapshot {
    fn log(&mut self, source: LogSource, text: String) {
        let sequence = self
            .logs
            .back()
            .map_or(1, |entry| entry.sequence.saturating_add(1));
        if self.logs.len() == MAX_LOG_ENTRIES {
            self.logs.pop_front();
            self.dropped_log_entries = self.dropped_log_entries.saturating_add(1);
        }
        self.logs.push_back(LogEntry {
            sequence,
            source,
            text,
        });
    }
}

struct Run {
    stop: watch::Sender<bool>,
    done: watch::Sender<bool>,
}

struct Session {
    state: Mutex<SessionSnapshot>,
    run: Mutex<Arc<Run>>,
    restart_operation: AsyncMutex<()>,
    restart_done: watch::Sender<bool>,
    stop_revision: AtomicU64,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// A dropped caller cannot leave the reservation stuck. Production operations
// are additionally owned by a service task, independent of the IPC future.
struct RestartGuard(Arc<Session>);
impl Drop for RestartGuard {
    fn drop(&mut self) {
        let mut state = self.0.state.lock().expect("session mutex poisoned");
        state.restarting = false;
        if matches!(state.phase, SessionPhase::Starting | SessionPhase::Stopping)
            && *self.0.run.lock().expect("run mutex poisoned").done.borrow()
        {
            state.phase = SessionPhase::Exited;
            state.failure = Some(ErrorCode::Cancelled);
            state.ended_at = Some(now());
        }
        self.0.restart_done.send_replace(true);
    }
}
impl Session {
    fn snapshot(&self) -> SessionSnapshot {
        self.state.lock().expect("session mutex poisoned").clone()
    }
    fn log(&self, source: LogSource, text: String) {
        self.state
            .lock()
            .expect("session mutex poisoned")
            .log(source, text);
    }
    fn request_stop(&self) {
        self.stop_revision.fetch_add(1, Ordering::SeqCst);
        self.stop_run();
    }
    fn stop_run(&self) {
        let mut state = self.state.lock().expect("session mutex poisoned");
        if !state.phase.terminal() {
            state.stop_requested = true;
            state.phase = SessionPhase::Stopping;
            self.run
                .lock()
                .expect("run mutex poisoned")
                .stop
                .send_replace(true);
        }
    }
    async fn wait(&self) -> Result<SessionSnapshot> {
        let run = self.run.lock().expect("run mutex poisoned").clone();
        Self::wait_run(&run).await?;
        let mut restarting = self.restart_done.subscribe();
        timeout(Duration::from_secs(6), restarting.wait_for(|done| *done))
            .await
            .map_err(|_| {
                AppError::new(ErrorCode::Timeout, "Restart cleanup is still in progress.")
            })?
            .map_err(|_| {
                AppError::new(
                    ErrorCode::ProcessFailed,
                    "Restart supervision ended unexpectedly.",
                )
            })?;
        Ok(self.snapshot())
    }
    async fn wait_run(run: &Run) -> Result<()> {
        let mut done = run.done.subscribe();
        timeout(Duration::from_secs(5), done.wait_for(|done| *done))
            .await
            .map_err(|_| {
                AppError::new(
                    ErrorCode::Timeout,
                    "Session cleanup is still in progress; check its status.",
                )
            })?
            .map_err(|_| {
                AppError::new(
                    ErrorCode::ProcessFailed,
                    "Session supervisor stopped unexpectedly.",
                )
            })?;
        Ok(())
    }
}

#[derive(Default)]
struct Registry {
    sessions: HashMap<String, Arc<Session>>,
    order: VecDeque<String>,
    closing: bool,
}

#[derive(Default)]
pub struct Supervisor {
    registry: AsyncMutex<Registry>,
}

impl Supervisor {
    /// The caller supplies only a path already validated by the probing service;
    /// neither executable paths nor arbitrary arguments come from launch IPC.
    #[cfg(any(test, feature = "test-support"))]
    pub async fn launch(
        &self,
        executable: &Path,
        request: LaunchRequest,
    ) -> Result<SessionSnapshot> {
        let arguments = build_arguments(&request)?;
        self.launch_command(CommandSpec {
            executable: executable.to_path_buf(),
            arguments,
            url: request.url,
            quality: request.quality,
            stream: None,
            policy: None,
        })
        .await
    }

    pub async fn launch_spec(&self, spec: LaunchSpec) -> Result<SessionSnapshot> {
        self.launch_command(build_command(spec)?).await
    }
    pub async fn launch_authenticated(
        &self,
        spec: LaunchSpec,
        auth_cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<SessionSnapshot> {
        let command = build_command(spec)?;
        let mut registry = self.registry.lock().await;
        // Check after waiting for the process registry, immediately before the
        // synchronous spawn. A cancelled account cannot authorize queued work.
        if auth_cancel.is_cancelled() {
            return Err(AppError::new(
                ErrorCode::Unauthenticated,
                "The Twitch session has ended.",
            ));
        }
        Self::spawn_session(&mut registry, command, None).await
    }
    async fn launch_command(&self, spec: CommandSpec) -> Result<SessionSnapshot> {
        let mut registry = self.registry.lock().await;
        Self::spawn_session(&mut registry, spec, None).await
    }
    async fn spawn_session(
        registry: &mut Registry,
        spec: CommandSpec,
        existing: Option<Arc<Session>>,
    ) -> Result<SessionSnapshot> {
        if registry.closing {
            return Err(AppError::new(
                ErrorCode::ProcessFailed,
                "The application is shutting down.",
            ));
        }
        if existing.is_none()
            && registry
                .sessions
                .values()
                .filter(|s| {
                    let state = s.snapshot();
                    !state.phase.terminal() || state.restarting
                })
                .count()
                >= MAX_ACTIVE
        {
            return Err(AppError::new(
                ErrorCode::Capacity,
                "Up to eight playback sessions can be active at once.",
            ));
        }
        let mut command = Command::new(&spec.executable);
        command
            .args(spec.arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        configure_process(&mut command);
        let mut child = command.spawn().map_err(|e| {
            AppError::new(
                ErrorCode::SpawnFailed,
                format!("Could not spawn Streamlink: {e}"),
            )
        })?;
        let tree = match ProcessTree::attach(&child) {
            Ok(tree) => tree,
            Err(e) => {
                let _ = child.kill().await;
                return Err(AppError::new(
                    ErrorCode::ProcessFailed,
                    format!("Could not supervise Streamlink: {e}"),
                ));
            }
        };
        let started = Instant::now();
        let id = existing.as_ref().map_or_else(
            || uuid::Uuid::new_v4().to_string(),
            |session| session.snapshot().id,
        );
        let (stop, mut stop_rx) = watch::channel(false);
        let (done, _) = watch::channel(false);
        let run = Arc::new(Run { stop, done });
        let initial = SessionSnapshot {
            id: id.clone(),
            generation: 1,
            restarting: false,
            stream: spec.stream,
            quality_policy: spec.policy,
            started_at: now(),
            ended_at: None,
            failure: None,
            phase: SessionPhase::Running,
            pid: child.id().expect("spawned PID"),
            url: spec.url,
            quality: spec.quality,
            exit_code: None,
            stop_requested: false,
            logs: VecDeque::new(),
            dropped_log_entries: 0,
        };
        let session = if let Some(session) = existing {
            {
                let mut state = session.state.lock().expect("session mutex poisoned");
                let mut next = initial;
                next.generation = state.generation;
                next.restarting = true;
                next.logs = std::mem::take(&mut state.logs);
                next.dropped_log_entries = state.dropped_log_entries;
                *state = next;
            }
            *session.run.lock().expect("run mutex poisoned") = run.clone();
            session
        } else {
            let (restart_done, _) = watch::channel(true);
            Arc::new(Session {
                state: Mutex::new(initial),
                run: Mutex::new(run.clone()),
                restart_operation: AsyncMutex::new(()),
                restart_done,
                stop_revision: AtomicU64::new(0),
            })
        };
        let initial = session.snapshot();
        while !registry.sessions.contains_key(&id) && registry.sessions.len() >= MAX_SESSIONS {
            let index = registry
                .order
                .iter()
                .position(|id| {
                    let state = registry.sessions[id].snapshot();
                    state.phase.terminal() && !state.restarting
                })
                .expect("terminal history available");
            let old = registry.order.remove(index).expect("history ID");
            registry.sessions.remove(&old);
        }
        if !registry.sessions.contains_key(&id) {
            registry.order.push_back(id.clone());
        }
        registry.sessions.insert(id, session.clone());
        tokio::spawn(async move {
            let stdout = tokio::spawn(read_output(
                child.stdout.take().expect("piped stdout"),
                session.clone(),
                LogSource::Stdout,
            ));
            let stderr = tokio::spawn(read_output(
                child.stderr.take().expect("piped stderr"),
                session.clone(),
                LogSource::Stderr,
            ));
            let status = tokio::select! {
                status = child.wait() => status,
                _ = async { let _ = stop_rx.wait_for(|stop| *stop).await; } => {
                    if tree.terminate().is_err() {
                        session.log(LogSource::Supervisor, "Could not terminate the process tree; terminating the direct child.".into());
                    }
                    // start_kill handles races with a natural exit; always wait.
                    let _ = child.start_kill();
                    child.wait().await
                }
            };
            // Also close inherited pipes held by non-detached descendants.
            drop(tree);
            tokio::join!(
                finish_reader(stdout, &session),
                finish_reader(stderr, &session)
            );
            {
                let mut state = session.state.lock().expect("session mutex poisoned");
                state.ended_at = Some(now());
                match status {
                    Ok(status) => {
                        state.exit_code = status.code();
                        state.phase = if status.success() || state.stop_requested {
                            SessionPhase::Exited
                        } else {
                            state.failure = Some(if started.elapsed() < Duration::from_secs(3) {
                                ErrorCode::StartupFailed
                            } else {
                                ErrorCode::StreamlinkExited
                            });
                            SessionPhase::Failed
                        };
                    }
                    Err(_) => {
                        state.phase = SessionPhase::Failed;
                        state.failure = Some(ErrorCode::ProcessFailed);
                        state.log(
                            LogSource::Supervisor,
                            "Failed to wait for the child process.".into(),
                        );
                    }
                }
            }
            run.done.send_replace(true);
        });
        Ok(initial)
    }

    pub async fn sessions(&self) -> Vec<SessionSnapshot> {
        let registry = self.registry.lock().await;
        registry
            .order
            .iter()
            .map(|id| registry.sessions[id].snapshot())
            .collect()
    }

    pub async fn stop(&self, id: &str) -> Result<SessionSnapshot> {
        let session = {
            let registry = self.registry.lock().await;
            let session = registry.sessions.get(id).cloned().ok_or_else(|| {
                AppError::new(ErrorCode::NotFound, "Unknown or evicted session ID.")
            })?;
            // Serialize the signal with spawning/replacing a run, not its wait.
            session.request_stop();
            session
        };
        session.wait().await
    }

    pub async fn restart<F, Fut>(
        &self,
        id: &str,
        generation: u32,
        prepare: F,
    ) -> Result<SessionSnapshot>
    where
        F: FnOnce(PlaybackStream) -> Fut,
        Fut: std::future::Future<Output = Result<LaunchSpec>>,
    {
        let session = self
            .registry
            .lock()
            .await
            .sessions
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, "Unknown or evicted session ID."))?;
        let _operation = session.restart_operation.try_lock().map_err(|_| {
            AppError::new(
                ErrorCode::RestartFailed,
                "This session is already restarting.",
            )
        })?;
        let (stream, revision, old_run) = {
            let registry = self.registry.lock().await;
            if registry.closing {
                return Err(AppError::new(
                    ErrorCode::Cancelled,
                    "The application is shutting down.",
                ));
            }
            if !registry
                .sessions
                .get(id)
                .is_some_and(|entry| Arc::ptr_eq(entry, &session))
            {
                return Err(AppError::new(
                    ErrorCode::NotFound,
                    "Session history was evicted.",
                ));
            }
            if session.snapshot().phase.terminal()
                && registry
                    .sessions
                    .values()
                    .filter(|s| {
                        let state = s.snapshot();
                        !state.phase.terminal() || state.restarting
                    })
                    .count()
                    >= MAX_ACTIVE
            {
                return Err(AppError::new(
                    ErrorCode::Capacity,
                    "Up to eight playback sessions can be active at once.",
                ));
            }
            let mut state = session.state.lock().expect("session mutex poisoned");
            if state.generation != generation {
                return Err(AppError::new(
                    ErrorCode::RestartFailed,
                    "The session has changed. Refresh its status before restarting.",
                ));
            }
            let stream = state.stream.clone().ok_or_else(|| {
                AppError::new(
                    ErrorCode::RestartFailed,
                    "This legacy session cannot be restarted.",
                )
            })?;
            state.generation = state.generation.checked_add(1).ok_or_else(|| {
                AppError::new(
                    ErrorCode::RestartFailed,
                    "Session generation limit reached.",
                )
            })?;
            state.restarting = true;
            session.restart_done.send_replace(false);
            (
                stream,
                session.stop_revision.load(Ordering::SeqCst),
                session.run.lock().expect("run mutex poisoned").clone(),
            )
        };
        let guard = RestartGuard(session.clone());
        session.stop_run();
        let result = async {
            Session::wait_run(&old_run).await?;
            {
                let _registry = self.registry.lock().await;
                let mut state = session.state.lock().expect("session mutex poisoned");
                if session.stop_revision.load(Ordering::SeqCst) != revision {
                    return Err(AppError::new(
                        ErrorCode::Cancelled,
                        "Restart was cancelled by Stop.",
                    ));
                }
                state.phase = SessionPhase::Starting;
                state.stop_requested = false;
                state.pid = 0;
                state.failure = None;
                state.ended_at = None;
                state.log(
                    LogSource::Supervisor,
                    "Restarting with current playback settings.".into(),
                );
            }
            let spec = build_command(prepare(stream).await?)?;
            let mut registry = self.registry.lock().await;
            if registry.closing || session.stop_revision.load(Ordering::SeqCst) != revision {
                return Err(AppError::new(
                    ErrorCode::Cancelled,
                    "Restart was cancelled.",
                ));
            }
            Self::spawn_session(&mut registry, spec, Some(session.clone())).await
        }
        .await;
        if let Err(error) = &result {
            let mut state = session.state.lock().expect("session mutex poisoned");
            let cancelled = session.stop_revision.load(Ordering::SeqCst) != revision
                || error.code == ErrorCode::Cancelled;
            state.phase = if !*old_run.done.borrow() {
                SessionPhase::Stopping
            } else if cancelled {
                SessionPhase::Exited
            } else {
                SessionPhase::Failed
            };
            state.failure = Some(if cancelled {
                ErrorCode::Cancelled
            } else {
                ErrorCode::RestartFailed
            });
            state.ended_at = Some(now());
            state.log(
                LogSource::Supervisor,
                format!("Restart did not launch: {:?}.", error.code),
            );
        }
        drop(guard);
        result.map(|_| session.snapshot())
    }

    pub async fn shutdown(&self) -> Result<()> {
        let sessions = {
            let mut registry = self.registry.lock().await;
            registry.closing = true;
            let sessions = registry.sessions.values().cloned().collect::<Vec<_>>();
            for session in &sessions {
                session.request_stop();
            }
            sessions
        };
        let mut result = Ok(());
        for session in sessions {
            if let Err(error) = session.wait().await {
                result = Err(error);
            }
        }
        result
    }
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        for session in self.registry.get_mut().sessions.values() {
            session.request_stop();
        }
    }
}

#[cfg(any(test, feature = "test-support"))]
pub fn build_arguments(request: &LaunchRequest) -> Result<Vec<String>> {
    let invalid = || {
        AppError::new(
            ErrorCode::InvalidInput,
            "Use an HTTPS Twitch channel URL without query parameters, credentials, or fragments, and a quality such as best, worst, or 720p60.",
        )
    };
    let url = url::Url::parse(&request.url).map_err(|_| invalid())?;
    let channel = url.path().trim_matches('/');
    if request.url.len() > 256
        || url.scheme() != "https"
        || !matches!(url.host_str(), Some("twitch.tv" | "www.twitch.tv"))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || channel.is_empty()
        || channel.len() > 25
        || !channel
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        || !valid_quality(&request.quality)
    {
        return Err(invalid());
    }
    Ok(vec![
        "--no-config".into(),
        "--loglevel".into(),
        "info".into(),
        "--".into(),
        url.to_string(),
        request.quality.clone(),
    ])
}

#[cfg(any(test, feature = "test-support"))]
fn valid_quality(quality: &str) -> bool {
    if matches!(quality, "best" | "worst" | "audio_only") {
        return true;
    }
    if let Some((height, rate)) = quality.split_once('p') {
        return !height.is_empty()
            && height.len() <= 4
            && height.chars().all(|c| c.is_ascii_digit())
            && rate.len() <= 3
            && rate.chars().all(|c| c.is_ascii_digit());
    }
    false
}

async fn read_output(
    mut reader: impl tokio::io::AsyncRead + Unpin,
    session: Arc<Session>,
    source: LogSource,
) {
    let mut buffer = [0; 1024];
    let mut line = Vec::with_capacity(MAX_LINE_BYTES);
    let mut oversized = false;
    loop {
        match reader.read(&mut buffer).await {
            Ok(0) => break,
            Ok(count) => {
                for &byte in &buffer[..count] {
                    if byte == b'\n' {
                        emit_line(&session, source, &line, oversized);
                        line.clear();
                        oversized = false;
                    } else if !oversized {
                        if line.len() == MAX_LINE_BYTES {
                            oversized = true;
                            line.clear();
                        } else {
                            line.push(byte);
                        }
                    }
                }
            }
            Err(_) => {
                session.log(
                    LogSource::Supervisor,
                    "Failed to read a process output pipe.".into(),
                );
                break;
            }
        }
    }
    if oversized || !line.is_empty() {
        emit_line(&session, source, &line, oversized);
    }
}

fn emit_line(session: &Session, source: LogSource, line: &[u8], oversized: bool) {
    if oversized {
        session.log(source, "[oversize diagnostic line omitted]".into());
        return;
    }
    let text = String::from_utf8_lossy(line);
    let lower = text.to_ascii_lowercase();
    // No app credentials are passed to Streamlink. This additionally suppresses
    // common credential-shaped output from external executables or plugins.
    if [
        "token",
        "oauth",
        "authorization",
        "bearer ",
        "password",
        "cookie",
    ]
    .iter()
    .any(|key| lower.contains(key))
    {
        session.log(source, "[credential-related diagnostic omitted]".into());
    } else {
        session.log(
            source,
            text.chars()
                .filter(|c| !c.is_control() || *c == '\t')
                .collect(),
        );
    }
}

async fn finish_reader(mut reader: JoinHandle<()>, session: &Session) {
    if timeout(Duration::from_secs(1), &mut reader).await.is_err() {
        reader.abort();
        let _ = reader.await;
        session.log(
            LogSource::Supervisor,
            "Output pipe remained open after process exit; capture stopped.".into(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arguments_are_small_and_cannot_inject_flags() {
        let request = LaunchRequest {
            url: "https://www.twitch.tv/example".into(),
            quality: "best".into(),
        };
        assert_eq!(
            build_arguments(&request).unwrap(),
            [
                "--no-config",
                "--loglevel",
                "info",
                "--",
                "https://www.twitch.tv/example",
                "best"
            ]
        );
        for quality in [
            "--player=evil",
            "best; touch file",
            "720p --player",
            "",
            "../../file",
        ] {
            assert!(
                build_arguments(&LaunchRequest {
                    quality: quality.into(),
                    ..request.clone()
                })
                .is_err()
            );
        }
        for url in [
            "http://twitch.tv/example",
            "https://twitch.tv.evil/example",
            "https://user:pass@twitch.tv/example",
            "https://twitch.tv/example?oauth=secret",
            "https://twitch.tv/example#secret",
            "https://twitch.tv/videos/123",
            "file:///tmp/evil",
        ] {
            assert!(
                build_arguments(&LaunchRequest {
                    url: url.into(),
                    ..request.clone()
                })
                .is_err()
            );
        }
        for quality in ["worst", "audio_only", "720p", "720p60"] {
            assert!(
                build_arguments(&LaunchRequest {
                    quality: quality.into(),
                    ..request.clone()
                })
                .is_ok()
            );
        }
    }
}
