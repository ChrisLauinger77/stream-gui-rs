//! Fixed Chatterino application only; no user-controlled Flatpak IDs or options.
use super::isolated_command;
use std::{
    path::Path,
    sync::{Mutex, TryLockError},
    time::{Duration, Instant},
};

// The app/ prefix prevents Flatpak from interpreting a runtime as a shell target.
pub(super) const APP_REF: &str = "app/com.chatterino.chatterino";
const LIMIT: Duration = Duration::from_secs(2);
static PROBE: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Installation {
    User,
    System,
}
impl Installation {
    pub(super) fn flag(self) -> &'static str {
        match self {
            Self::User => "--user",
            Self::System => "--system",
        }
    }
}

pub(super) fn detect(executable: &Path) -> Option<Installation> {
    // One host probe at a time; lock contention and both local queries share a
    // deadline. IPC discovery never creates an unbounded set of probe children.
    let deadline = Instant::now() + LIMIT;
    let _guard = loop {
        match PROBE.try_lock() {
            Ok(guard) => break guard,
            Err(TryLockError::Poisoned(_)) => return None,
            Err(TryLockError::WouldBlock) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return None,
        }
    };
    // Explicit scopes make detection and launch agree when both are installed.
    for installation in [Installation::User, Installation::System] {
        if Instant::now() >= deadline {
            return None;
        }
        let mut child = isolated_command(
            executable,
            &["info".into(), installation.flag().into(), APP_REF.into()],
        )
        .ok()?
        .spawn()
        .ok()?;
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if status.success() {
                        return Some(installation);
                    }
                    break;
                }
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                _ => {
                    // Only the bounded info probe owns this process group. Real
                    // Chatterino launches never take this termination path.
                    // SAFETY: isolated_command creates a new session for this
                    // unreaped child, so its PID is the owned probe group ID.
                    unsafe {
                        libc::kill(-(child.id() as i32), libc::SIGKILL);
                    }
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
            }
        }
    }
    None
}
