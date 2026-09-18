//! Playback policy and argv construction. No process creation or shell parsing.
use crate::domain::{AppError, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum QualityPolicy {
    #[default]
    Source,
    High,
    Medium,
    Low,
    Audio,
}
impl QualityPolicy {
    pub fn selection(self) -> (&'static str, Option<&'static str>) {
        match self {
            Self::Source => ("best", None),
            Self::High => ("high,best,best-unfiltered", Some(">720p30")),
            Self::Medium => ("medium,best,best-unfiltered", Some(">540p30")),
            Self::Low => ("low,best,best-unfiltered", Some(">360p30")),
            Self::Audio => ("audio,audio_only", None),
        }
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum PlayerMode {
    #[default]
    Default,
    Mpv,
    Vlc,
    Custom,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlayerSettings {
    pub mode: PlayerMode,
    pub executable: Option<String>,
    // Each entry is exactly one literal argument, including empty strings.
    pub arguments: Vec<String>,
}
impl PlayerSettings {
    pub fn validate(&self) -> Result<()> {
        let invalid = || {
            AppError::new(
                ErrorCode::InvalidPlayer,
                "Check the player executable and arguments.",
            )
        };
        if self.arguments.len() > 32
            || self.arguments.iter().map(String::len).sum::<usize>() > 4096
            || self
                .arguments
                .iter()
                .any(|arg| arg.chars().any(char::is_control))
            || self.executable.as_ref().is_some_and(|path| {
                path.len() > 4096
                    || path.chars().any(char::is_control)
                    || !std::path::Path::new(path).is_absolute()
            })
            || (self.mode == PlayerMode::Default && self.executable.is_some())
            || (self.mode == PlayerMode::Custom && self.executable.is_none())
        {
            return Err(invalid());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaybackRequest {
    pub auth_session_id: String,
    pub broadcaster_id: String,
    pub quality: Option<QualityPolicy>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestartRequest {
    pub session_id: String,
    pub generation: u32,
    pub quality: Option<QualityPolicy>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackStream {
    pub stream_id: Option<String>,
    pub broadcaster_id: String,
    pub login: String,
    pub display_name: String,
    pub title: Option<String>,
    pub category: Option<String>,
}
impl From<crate::helix::browse::StreamSummary> for PlaybackStream {
    fn from(stream: crate::helix::browse::StreamSummary) -> Self {
        Self {
            stream_id: Some(stream.stream_id),
            broadcaster_id: stream.broadcaster_id,
            login: stream.login,
            display_name: stream.display_name,
            title: Some(stream.title),
            category: stream.category_name,
        }
    }
}

/// Resolved native paths, policy and immutable Twitch metadata. This is never an
/// IPC input: the service constructs it from settings and Rust-owned Helix data.
pub struct LaunchSpec {
    pub executable: PathBuf,
    pub player: Option<PathBuf>,
    pub settings: crate::config::EffectivePlaybackSettings,
    pub stream: PlaybackStream,
}
pub struct CommandSpec {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub url: String,
    pub stream: Option<PlaybackStream>,
    pub quality: String,
    pub policy: Option<QualityPolicy>,
    pub effective_settings: Option<crate::config::EffectivePlaybackSettings>,
}

pub fn channel_url(login: &str) -> Result<String> {
    if login.is_empty()
        || login.len() > 25
        || !login
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_')
    {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Invalid Twitch channel identity.",
        ));
    }
    Ok(format!(
        "https://www.twitch.tv/{}",
        login.to_ascii_lowercase()
    ))
}

/// Streamlink 8 uses Formatter followed by POSIX shlex.split on every platform.
/// Escape formatting braces, then encode literal tokens with single quotes.
/// Always supply our own input placeholder: Streamlink's substring detection
/// also sees escaped user text such as {{playerinput}} and would omit stdin.
pub fn encode_player_arguments(arguments: &[String]) -> String {
    arguments
        .iter()
        .map(|argument| {
            let literal = argument.replace('{', "{{").replace('}', "}}");
            format!("'{}'", literal.replace('\'', "'\"'\"'"))
        })
        .chain(std::iter::once("{playerinput}".to_owned()))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn build_command(spec: LaunchSpec) -> Result<CommandSpec> {
    spec.settings.player.validate()?;
    let effective_settings = spec.settings.clone();
    let url = channel_url(&spec.stream.login)?;
    let (selection, exclude) = spec.settings.quality.selection();
    let mut arguments = vec![
        "--no-config".into(),
        "--no-plugin-sideloading".into(),
        "--loglevel".into(),
        "info".into(),
        "--player-verbose".into(),
    ];
    if let Some(player) = spec.player {
        let path = player.to_str().ok_or_else(|| {
            AppError::new(
                ErrorCode::InvalidPlayer,
                "Player path must be valid Unicode.",
            )
        })?;
        arguments.extend(["--player".into(), path.into()]);
    }
    let mut player_args: Vec<String> = match spec.settings.player.mode {
        PlayerMode::Mpv => vec!["--keep-open=no".into(), "--quiet".into()],
        // macOS VLC always starts its native instance and does not expose
        // the one-instance option used on Linux/Windows.
        PlayerMode::Vlc if cfg!(target_os = "macos") => vec!["--play-and-exit".into()],
        PlayerMode::Vlc => vec!["--no-one-instance".into(), "--play-and-exit".into()],
        _ => vec![],
    };
    player_args.extend(spec.settings.player.arguments);
    if !player_args.is_empty() {
        arguments.extend([
            "--player-args".into(),
            encode_player_arguments(&player_args),
        ]);
    }
    if let Some(exclude) = exclude {
        arguments.extend(["--stream-sorting-excludes".into(), exclude.into()]);
    }
    arguments.extend(["--".into(), url.clone(), selection.into()]);
    Ok(CommandSpec {
        executable: spec.executable,
        arguments,
        url,
        stream: Some(spec.stream),
        quality: selection.into(),
        policy: Some(spec.settings.quality),
        effective_settings: Some(effective_settings),
    })
}

pub fn check_version(version: &str) -> Result<()> {
    if version
        .split('.')
        .next()
        .and_then(|v| v.parse::<u32>().ok())
        .is_none_or(|major| major < 8)
    {
        return Err(AppError::new(
            ErrorCode::UnsupportedStreamlink,
            "Streamlink 8.0 or newer is required.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn spec(quality: QualityPolicy, mode: PlayerMode) -> LaunchSpec {
        LaunchSpec {
            executable: PathBuf::from("/tools/Stream link/streamlink"),
            player: (mode != PlayerMode::Default)
                .then(|| PathBuf::from("/播放器 with spaces/player")),
            settings: crate::config::EffectivePlaybackSettings {
                streamlink_path: None,
                automatic_chat: false,
                quality,
                player: PlayerSettings {
                    mode,
                    executable: (mode == PlayerMode::Custom).then(|| {
                        std::env::current_exe()
                            .unwrap()
                            .to_string_lossy()
                            .into_owned()
                    }),
                    arguments: vec![],
                },
            },
            stream: PlaybackStream {
                stream_id: Some("456".into()),
                broadcaster_id: "123".into(),
                login: "Example_1".into(),
                display_name: "not --an-argument".into(),
                title: Some("$(never execute) {playerinput}".into()),
                category: Some("game; nope".into()),
            },
        }
    }
    #[test]
    fn exact_source_default_argv_has_no_metadata_or_shell() {
        let command = build_command(spec(QualityPolicy::Source, PlayerMode::Default)).unwrap();
        assert_eq!(
            command.executable,
            PathBuf::from("/tools/Stream link/streamlink")
        );
        assert_eq!(
            command.arguments,
            [
                "--no-config",
                "--no-plugin-sideloading",
                "--loglevel",
                "info",
                "--player-verbose",
                "--",
                "https://www.twitch.tv/example_1",
                "best"
            ]
        );
        assert_eq!(command.stream.unwrap().stream_id.as_deref(), Some("456"));
    }
    #[test]
    fn exact_high_medium_low_and_audio_policy_argv() {
        for (quality, selection, exclude) in [
            (
                QualityPolicy::High,
                "high,best,best-unfiltered",
                Some(">720p30"),
            ),
            (
                QualityPolicy::Medium,
                "medium,best,best-unfiltered",
                Some(">540p30"),
            ),
            (
                QualityPolicy::Low,
                "low,best,best-unfiltered",
                Some(">360p30"),
            ),
            (QualityPolicy::Audio, "audio,audio_only", None),
        ] {
            let command = build_command(spec(quality, PlayerMode::Default)).unwrap();
            let mut expected = vec![
                "--no-config",
                "--no-plugin-sideloading",
                "--loglevel",
                "info",
                "--player-verbose",
            ];
            if let Some(exclude) = exclude {
                expected.extend(["--stream-sorting-excludes", exclude]);
            }
            expected.extend(["--", "https://www.twitch.tv/example_1", selection]);
            assert_eq!(command.arguments, expected);
        }
    }
    #[test]
    fn exact_mpv_vlc_and_custom_player_paths_are_single_unquoted_argv_entries() {
        for (mode, parameters) in [
            (
                PlayerMode::Mpv,
                Some("'--keep-open=no' '--quiet' {playerinput}"),
            ),
            (
                PlayerMode::Vlc,
                Some(if cfg!(target_os = "macos") {
                    "'--play-and-exit' {playerinput}"
                } else {
                    "'--no-one-instance' '--play-and-exit' {playerinput}"
                }),
            ),
            (PlayerMode::Custom, None),
        ] {
            let command = build_command(spec(QualityPolicy::Source, mode)).unwrap();
            let mut expected = vec![
                "--no-config",
                "--no-plugin-sideloading",
                "--loglevel",
                "info",
                "--player-verbose",
                "--player",
                "/播放器 with spaces/player",
            ];
            if let Some(parameters) = parameters {
                expected.extend(["--player-args", parameters]);
            }
            expected.extend(["--", "https://www.twitch.tv/example_1", "best"]);
            assert_eq!(command.arguments, expected);
        }
    }
    #[test]
    fn player_arguments_encode_spaces_quotes_braces_empty_unicode_and_windows_paths() {
        let input = [
            "space value",
            "",
            "a'b",
            "\"quoted\"",
            r"C:\播放器\file name",
            "{playerinput}",
            "{unknown}",
            "$HOME;$(command)",
        ];
        let encoded = encode_player_arguments(&input.map(String::from));
        assert_eq!(
            encoded,
            "'space value' '' 'a'\"'\"'b' '\"quoted\"' 'C:\\播放器\\file name' '{{playerinput}}' '{{unknown}}' '$HOME;$(command)' {playerinput}"
        );
        let mut launch = spec(QualityPolicy::Source, PlayerMode::Default);
        launch.settings.player.arguments = input.map(String::from).to_vec();
        assert_eq!(build_command(launch).unwrap().arguments[6], encoded);
    }
    #[test]
    fn structured_player_arguments_reject_malformed_types_controls_and_unbounded_input() {
        for json in [
            r#"{"mode":"default","executable":null,"arguments":"--flag value"}"#,
            r#"{"mode":"default","executable":null,"arguments":[1]}"#,
            r#"{"mode":"shell","executable":null,"arguments":[]}"#,
        ] {
            assert!(serde_json::from_str::<PlayerSettings>(json).is_err());
        }
        for arguments in [
            vec!["bad\0arg".into()],
            vec!["bad\narg".into()],
            vec!["x".repeat(4097)],
            vec![String::new(); 33],
        ] {
            assert_eq!(
                PlayerSettings {
                    arguments,
                    ..PlayerSettings::default()
                }
                .validate()
                .unwrap_err()
                .code,
                ErrorCode::InvalidPlayer
            );
        }
        assert!(
            PlayerSettings {
                mode: PlayerMode::Custom,
                ..PlayerSettings::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            PlayerSettings {
                executable: Some("relative player".into()),
                ..PlayerSettings::default()
            }
            .validate()
            .is_err()
        );
        assert!(
            PlayerSettings {
                arguments: vec!["unclosed ' quote is literal".into(), String::new()],
                ..PlayerSettings::default()
            }
            .validate()
            .is_ok()
        );
    }
    #[test]
    fn twitch_url_uses_only_validated_login_and_normalizes_case() {
        assert_eq!(
            channel_url("User_123").unwrap(),
            "https://www.twitch.tv/user_123"
        );
        for login in [
            "",
            "-flag",
            "with space",
            "name/path",
            "name?token=secret",
            "{playerinput}",
            "用户",
            "abcdefghijklmnopqrstuvwxyz",
            "user\narg",
        ] {
            assert!(channel_url(login).is_err());
        }
    }
    #[test]
    fn unsupported_versions_are_typed_before_playback() {
        for version in ["6.0.0", "7.6.0", "invalid"] {
            assert_eq!(
                check_version(version).unwrap_err().code,
                ErrorCode::UnsupportedStreamlink
            );
        }
        for version in ["8.0.0", "8.6.1", "9.0.0"] {
            assert!(check_version(version).is_ok());
        }
    }
}
