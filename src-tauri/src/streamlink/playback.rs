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
pub struct KickPlaybackRequest {
    pub slug: String,
    pub quality: Option<QualityPolicy>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestartRequest {
    pub session_id: String,
    pub generation: u32,
    pub quality: Option<QualityPolicy>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TwitchPlaybackStream {
    pub stream_id: Option<String>,
    pub broadcaster_id: String,
    pub login: String,
    pub display_name: String,
    pub title: Option<String>,
    pub category: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum StreamingService {
    Twitch,
    Kick,
}

/// A locator, not a verified account identity. The application accepts 1–100
/// ASCII letters/digits/underscores/hyphens and normalizes letters to lowercase.
/// This bound is an input limit, not a claim about Kick's account naming policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(try_from = "String")]
pub struct KickSlug(String);
impl KickSlug {
    pub fn parse(value: &str) -> Result<Self> {
        if value.is_empty()
            || value.len() > 100
            || !value
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        {
            return Err(AppError::new(
                ErrorCode::InvalidKickSlug,
                "Invalid Kick channel name.",
            ));
        }
        Ok(Self(value.to_ascii_lowercase()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn channel_url(&self) -> String {
        format!("https://kick.com/{}", self.0)
    }
}
impl TryFrom<String> for KickSlug {
    type Error = AppError;
    fn try_from(value: String) -> Result<Self> {
        Self::parse(&value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "service", rename_all = "snake_case")]
pub enum PlaybackStream {
    Twitch(TwitchPlaybackStream),
    Kick { slug: KickSlug },
}
impl PlaybackStream {
    pub fn service(&self) -> StreamingService {
        match self {
            Self::Twitch(_) => StreamingService::Twitch,
            Self::Kick { .. } => StreamingService::Kick,
        }
    }
}
impl From<crate::helix::browse::StreamSummary> for PlaybackStream {
    fn from(stream: crate::helix::browse::StreamSummary) -> Self {
        Self::Twitch(TwitchPlaybackStream {
            stream_id: Some(stream.stream_id),
            broadcaster_id: stream.broadcaster_id,
            login: stream.login,
            display_name: stream.display_name,
            title: Some(stream.title),
            category: stream.category_name,
        })
    }
}

/// Resolved native paths, policy and immutable service identity. This is never
/// an IPC input: Twitch metadata comes from Helix; Kick contains only a locator.
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

pub fn normalize_login(login: &str) -> Result<String> {
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
    Ok(login.to_ascii_lowercase())
}

pub fn channel_url(login: &str) -> Result<String> {
    Ok(format!("https://www.twitch.tv/{}", normalize_login(login)?))
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
    let url = match &spec.stream {
        PlaybackStream::Twitch(stream) => channel_url(&stream.login)?,
        PlaybackStream::Kick { slug } => slug.channel_url(),
    };
    let (selection, exclude) = spec.settings.quality.selection();
    let mut arguments = vec![
        "--no-config".into(),
        "--no-plugin-sideloading".into(),
        "--loglevel".into(),
        "info".into(),
        "--player-verbose".into(),
    ];
    if spec.settings.low_latency {
        arguments.push(
            match spec.stream.service() {
                StreamingService::Twitch => "--twitch-low-latency",
                StreamingService::Kick => "--kick-low-latency",
            }
            .into(),
        );
    }
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
                profile_id: None,
                chat_provider: crate::config::ChatProvider::Browser,
                chatterino_path: None,
                low_latency: false,
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
            stream: PlaybackStream::Twitch(TwitchPlaybackStream {
                stream_id: Some("456".into()),
                broadcaster_id: "123".into(),
                login: "Example_1".into(),
                display_name: "not --an-argument".into(),
                title: Some("$(never execute) {playerinput}".into()),
                category: Some("game; nope".into()),
            }),
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
        let Some(PlaybackStream::Twitch(stream)) = command.stream else {
            panic!("expected Twitch identity")
        };
        assert_eq!(stream.stream_id.as_deref(), Some("456"));
    }
    #[test]
    fn kick_slug_is_bounded_normalized_and_cannot_contain_a_destination() {
        for (input, expected) in [
            ("Example_1", "example_1"),
            ("Example-Channel", "example-channel"),
            ("123", "123"),
        ] {
            let slug = KickSlug::parse(input).unwrap();
            assert_eq!(slug.as_str(), expected);
            assert_eq!(slug.channel_url(), format!("https://kick.com/{expected}"));
            let restored: KickSlug =
                serde_json::from_str(&serde_json::to_string(&slug).unwrap()).unwrap();
            assert_eq!(restored, slug);
        }
        assert!(KickSlug::parse(&"a".repeat(100)).is_ok());
        for input in [
            "",
            " ",
            " a",
            "a b",
            "a\nb",
            "a\tb",
            "a/b",
            "a\\b",
            "a?x",
            "a#x",
            "https://kick.com/a",
            "../a",
            "..",
            "a;b",
            "$(id)",
            "a&b",
            "a|b",
            "a`id`",
            "a%2fb",
            "a@b",
            "用户",
            "é",
            "a\0b",
            &"a".repeat(101),
        ] {
            let error = KickSlug::parse(input).unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidKickSlug);
            assert_eq!(error.message, "Invalid Kick channel name.");
            assert!(
                serde_json::from_str::<KickSlug>(&serde_json::to_string(input).unwrap()).is_err()
            );
        }
        assert!(
            serde_json::from_str::<KickPlaybackRequest>(
                r#"{"slug":"example","quality":null,"url":"https://evil.example"}"#
            )
            .is_err()
        );
    }

    #[test]
    fn kick_argv_and_latency_are_service_specific_for_every_quality() {
        for quality in [
            QualityPolicy::Source,
            QualityPolicy::High,
            QualityPolicy::Medium,
            QualityPolicy::Low,
            QualityPolicy::Audio,
        ] {
            for low_latency in [false, true] {
                let mut kick = spec(quality, PlayerMode::Default);
                kick.stream = PlaybackStream::Kick {
                    slug: KickSlug::parse("Example-1").unwrap(),
                };
                kick.settings.low_latency = low_latency;
                let command = build_command(kick).unwrap();
                let (selection, exclude) = quality.selection();
                let mut expected = vec![
                    "--no-config",
                    "--no-plugin-sideloading",
                    "--loglevel",
                    "info",
                    "--player-verbose",
                ];
                if low_latency {
                    expected.push("--kick-low-latency");
                }
                if let Some(exclude) = exclude {
                    expected.extend(["--stream-sorting-excludes", exclude]);
                }
                expected.extend(["--", "https://kick.com/example-1", selection]);
                assert_eq!(command.arguments, expected);
                assert_eq!(command.stream.unwrap().service(), StreamingService::Kick);
                if quality == QualityPolicy::Audio {
                    assert_eq!(selection, "audio,audio_only");
                    assert!(exclude.is_none());
                    assert!(!command.arguments.iter().any(|arg| arg.contains("best")));
                }
                let mut twitch = spec(quality, PlayerMode::Default);
                twitch.settings.low_latency = low_latency;
                let arguments = build_command(twitch).unwrap().arguments;
                assert!(!arguments.iter().any(|arg| arg == "--kick-low-latency"));
                assert_eq!(
                    arguments.iter().any(|arg| arg == "--twitch-low-latency"),
                    low_latency
                );
            }
        }
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
    #[test]
    fn low_latency_adds_only_one_owned_flag_for_every_quality_and_player() {
        for quality in [
            QualityPolicy::Source,
            QualityPolicy::High,
            QualityPolicy::Medium,
            QualityPolicy::Low,
            QualityPolicy::Audio,
        ] {
            for mode in [
                PlayerMode::Default,
                PlayerMode::Mpv,
                PlayerMode::Vlc,
                PlayerMode::Custom,
            ] {
                let normal = build_command(spec(quality, mode)).unwrap();
                let mut low = spec(quality, mode);
                low.settings.low_latency = true;
                let low = build_command(low).unwrap();
                assert!(low.effective_settings.as_ref().unwrap().low_latency);
                assert!(!normal.effective_settings.as_ref().unwrap().low_latency);
                assert_eq!(
                    low.arguments
                        .iter()
                        .filter(|arg| *arg == "--twitch-low-latency")
                        .count(),
                    1
                );
                assert_eq!(
                    low.arguments
                        .iter()
                        .filter(|arg| *arg != "--twitch-low-latency")
                        .cloned()
                        .collect::<Vec<_>>(),
                    normal.arguments
                );
                assert!(
                    low.arguments
                        .iter()
                        .position(|arg| arg == "--twitch-low-latency")
                        .unwrap()
                        < low.arguments.iter().position(|arg| arg == "--").unwrap()
                );
            }
        }
    }
}
