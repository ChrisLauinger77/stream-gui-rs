pub mod build_info;
pub mod chatterino;
pub mod config;
pub mod credentials;
pub mod diagnostics;
pub mod domain;
pub mod helix;
pub mod monitor;
pub mod navigation;
pub mod platform;
pub mod streamlink;
mod time;
pub mod twitch;
pub mod twitch_http;
pub mod updates;

#[cfg(feature = "desktop")]
mod commands;
#[cfg(feature = "desktop")]
pub mod desktop;

#[cfg(test)]
mod test_http;
