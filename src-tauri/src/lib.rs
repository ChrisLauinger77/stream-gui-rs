pub mod config;
pub mod credentials;
pub mod diagnostics;
pub mod domain;
pub mod platform;
pub mod streamlink;
pub mod twitch;

#[cfg(feature = "desktop")]
mod commands;
#[cfg(feature = "desktop")]
pub mod desktop;
