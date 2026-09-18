pub mod config;
pub mod credentials;
pub mod diagnostics;
pub mod domain;
pub mod platform;
pub mod streamlink;
pub mod twitch;
pub mod twitch_http;

#[cfg(feature = "desktop")]
mod commands;
#[cfg(feature = "desktop")]
pub mod desktop;

#[cfg(test)]
mod test_http;
