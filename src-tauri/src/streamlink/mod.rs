pub mod discovery;
pub mod playback;
mod probe;
mod supervisor;

pub use probe::{ProbeResult, discover, parse_version, probe, validate_executable};
#[cfg(any(test, feature = "test-support"))]
pub use supervisor::build_arguments;
pub use supervisor::{LogEntry, LogSource, SessionPhase, SessionSnapshot, Supervisor};
