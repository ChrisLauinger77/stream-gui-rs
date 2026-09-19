//! Public build metadata, embedded at compile time; never resolves Git at runtime.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const COMMIT: &str = env!("STREAM_GUI_RS_BUILD_COMMIT");
pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

#[cfg(test)]
mod commit;
