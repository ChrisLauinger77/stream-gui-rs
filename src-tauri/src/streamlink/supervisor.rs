use crate::{
    domain::{AppError, ErrorCode, LaunchRequest, Result},
    platform::{ProcessTree, configure_process},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    path::Path,
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::AsyncReadExt,
    process::Command,
    sync::{Mutex as AsyncMutex, watch},
    task::JoinHandle,
    time::timeout,
};
use ts_rs::TS;

pub const MAX_LOG_ENTRIES: usize = 200;
pub const MAX_LINE_BYTES: usize = 2048;
const MAX_SESSIONS: usize = 16;
const MAX_ACTIVE: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SessionPhase {
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

struct Session {
    state: Mutex<SessionSnapshot>,
    stop: watch::Sender<bool>,
    done: watch::Sender<bool>,
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
        let mut state = self.state.lock().expect("session mutex poisoned");
        if !state.phase.terminal() {
            state.stop_requested = true;
            state.phase = SessionPhase::Stopping;
            self.stop.send_replace(true);
        }
    }
    async fn wait(&self) -> Result<SessionSnapshot> {
        let mut done = self.done.subscribe();
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
        Ok(self.snapshot())
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
    pub async fn launch(
        &self,
        executable: &Path,
        request: LaunchRequest,
    ) -> Result<SessionSnapshot> {
        let args = build_arguments(&request)?;
        let mut registry = self.registry.lock().await;
        if registry.closing {
            return Err(AppError::new(
                ErrorCode::ProcessFailed,
                "The application is shutting down.",
            ));
        }
        if registry
            .sessions
            .values()
            .filter(|s| !s.snapshot().phase.terminal())
            .count()
            >= MAX_ACTIVE
        {
            return Err(AppError::new(
                ErrorCode::Capacity,
                "The prototype supports up to eight active sessions.",
            ));
        }
        let mut command = Command::new(executable);
        command
            .args(args)
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
        let id = uuid::Uuid::new_v4().to_string();
        let (stop, mut stop_rx) = watch::channel(false);
        let (done, _) = watch::channel(false);
        let session = Arc::new(Session {
            state: Mutex::new(SessionSnapshot {
                id: id.clone(),
                phase: SessionPhase::Running,
                pid: child.id().expect("spawned PID"),
                url: request.url,
                quality: request.quality,
                exit_code: None,
                stop_requested: false,
                logs: VecDeque::new(),
                dropped_log_entries: 0,
            }),
            stop,
            done,
        });
        let initial = session.snapshot();
        while registry.sessions.len() >= MAX_SESSIONS {
            let index = registry
                .order
                .iter()
                .position(|id| registry.sessions[id].snapshot().phase.terminal())
                .expect("terminal history available");
            let old = registry.order.remove(index).expect("history ID");
            registry.sessions.remove(&old);
        }
        registry.order.push_back(id.clone());
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
                match status {
                    Ok(status) => {
                        state.exit_code = status.code();
                        state.phase = if status.success() || state.stop_requested {
                            SessionPhase::Exited
                        } else {
                            SessionPhase::Failed
                        };
                    }
                    Err(_) => {
                        state.phase = SessionPhase::Failed;
                        state.log(
                            LogSource::Supervisor,
                            "Failed to wait for the child process.".into(),
                        );
                    }
                }
            }
            session.done.send_replace(true);
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
        let session = self
            .registry
            .lock()
            .await
            .sessions
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, "Unknown or evicted session ID."))?;
        session.request_stop();
        session.wait().await
    }

    pub async fn shutdown(&self) -> Result<()> {
        let sessions = {
            let mut registry = self.registry.lock().await;
            registry.closing = true;
            registry.sessions.values().cloned().collect::<Vec<_>>()
        };
        for session in &sessions {
            session.request_stop();
        }
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
