//! Deliberately separate from full local diagnostics: only closed enums, numeric
//! values and compile-time public metadata may enter this shareable text.
use crate::{
    build_info,
    streamlink::{SessionSnapshot, playback::PlayerMode},
};
use serde::Serialize;
use std::fmt::Write;
use ts_rs::TS;

const MAX_RECORDS: usize = 16;
const MAX_BYTES: usize = 64 * 1024;

#[derive(Serialize, TS)]
pub struct SupportReport {
    pub text: String,
}

pub(crate) fn numeric_version(value: &str) -> Option<[u32; 3]> {
    let core = value.split(['-', '+']).next()?;
    let parts = core.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    Some([
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ])
}

// Called only on closed fieldless enums, never settings, error messages or snapshots.
fn label(value: impl Serialize) -> String {
    serde_json::to_value(value)
        .expect("enum serializes")
        .as_str()
        .expect("closed enum label")
        .to_owned()
}

pub fn support_report(
    mode: PlayerMode,
    version: Option<[u32; 3]>,
    sessions: &[SessionSnapshot],
) -> SupportReport {
    let info = build_info::snapshot();
    let version = version
        .map(|[major, minor, patch]| format!("{major}.{minor}.{patch}"))
        .unwrap_or_else(|| "not checked".into());
    let mut text = format!(
        "{} support report\nVersion: {} ({})\nOS: {}\nArchitecture: {}\nStreamlink: {}\nPlayer mode: {}\nAnonymous retained processes: {}\n",
        info.name,
        info.version,
        info.commit,
        std::env::consts::OS,
        std::env::consts::ARCH,
        version,
        label(mode),
        sessions.len().min(MAX_RECORDS)
    );
    for (index, session) in sessions.iter().take(MAX_RECORDS).enumerate() {
        let record = format!(
            "Process {}: phase={}; failure={}; exit={}\n",
            index + 1,
            label(session.phase),
            session.failure.map(label).unwrap_or_else(|| "none".into()),
            session
                .exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unavailable".into())
        );
        if text.len() + record.len() > MAX_BYTES {
            break;
        }
        text.write_str(&record).expect("string write");
    }
    SupportReport { text }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{EffectivePlaybackSettings, Settings},
        domain::ErrorCode,
        streamlink::{LogEntry, LogSource, SessionPhase, playback::PlaybackStream},
    };
    const PRIVATE: &str = "oauth:synthetic-token password=synthetic alice ACCOUNT CHANNEL /home/alice /Users/alice C:\\Users\\alice --password=secret\u{1b}[31m arbitrary child output";
    fn hostile() -> SessionSnapshot {
        let mut settings = Settings {
            streamlink_path: Some(PRIVATE.into()),
            ..Settings::default()
        };
        settings.player.executable = Some(PRIVATE.into());
        settings.player.arguments = vec![PRIVATE.into()];
        SessionSnapshot {
            id: PRIVATE.into(),
            generation: 77,
            restarting: false,
            stream: Some(PlaybackStream {
                broadcaster_id: PRIVATE.into(),
                stream_id: Some(PRIVATE.into()),
                login: PRIVATE.into(),
                display_name: PRIVATE.into(),
                title: Some(PRIVATE.into()),
                category: Some(PRIVATE.into()),
            }),
            quality_policy: None,
            effective_settings: Some(EffectivePlaybackSettings {
                streamlink_path: settings.streamlink_path,
                player: settings.player,
                quality: settings.default_quality,
                automatic_chat: true,
                low_latency: true,
            }),
            chat_error: None,
            started_at: u64::MAX,
            ended_at: Some(u64::MAX),
            failure: Some(ErrorCode::ProcessFailed),
            phase: SessionPhase::Failed,
            pid: u32::MAX,
            url: PRIVATE.into(),
            quality: PRIVATE.into(),
            exit_code: Some(-9),
            stop_requested: false,
            logs: [LogEntry {
                sequence: 1,
                source: LogSource::Stderr,
                text: PRIVATE.repeat(10000),
            }]
            .into(),
            dropped_log_entries: 5,
        }
    }
    #[test]
    fn report_is_allowlisted_anonymous_bounded_and_deterministic() {
        let source = vec![hostile(); 20];
        let report = support_report(PlayerMode::Custom, Some([8, 6, 1]), &source).text;
        assert_eq!(
            report,
            support_report(PlayerMode::Custom, Some([8, 6, 1]), &source).text
        );
        for forbidden in [
            "oauth",
            "password",
            "alice",
            "ACCOUNT",
            "CHANNEL",
            "/home",
            "/Users",
            "C:\\",
            "--password",
            "\u{1b}",
            "arbitrary",
            "18446744073709551615",
            "4294967295",
        ] {
            assert!(!report.contains(forbidden), "leaked {forbidden}");
        }
        assert_eq!(
            report
                .lines()
                .filter(|line| line.starts_with("Process "))
                .count(),
            16
        );
        assert!(report.contains("Process 16: phase=failed; failure=process_failed; exit=-9"));
        assert!(report.contains("Streamlink: 8.6.1"));
        assert!(report.len() < MAX_BYTES);
    }
    #[test]
    fn versions_exclude_all_free_form_suffixes_and_unchecked_state_is_explicit() {
        assert_eq!(
            numeric_version("8.6.1-password=/home/alice"),
            Some([8, 6, 1])
        );
        assert_eq!(numeric_version("8.6.1+arbitrary"), Some([8, 6, 1]));
        for value in [
            "8.6",
            "8.6.1.2",
            "8.6.1 secret",
            "8.6.4294967296",
            "+8.6.1",
            "8.6.１",
        ] {
            assert_eq!(numeric_version(value), None);
        }
        let report = support_report(PlayerMode::Default, None, &[]).text;
        assert!(report.contains("Streamlink: not checked"));
        assert!(report.contains("Anonymous retained processes: 0"));
    }
    #[tokio::test]
    async fn report_does_not_probe_a_configured_executable_or_access_unavailable_credentials() {
        let directory = tempfile::tempdir().unwrap();
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        struct CountingStore(Arc<AtomicUsize>);
        impl crate::credentials::CredentialStore for CountingStore {
            fn load(&self) -> crate::domain::Result<Option<crate::credentials::Credentials>> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(None)
            }
            fn save(&mut self, _: crate::credentials::Credentials) -> crate::domain::Result<()> {
                panic!("report wrote credentials")
            }
            fn clear(&mut self) -> crate::domain::Result<()> {
                panic!("report cleared credentials")
            }
            fn name(&self) -> &'static str {
                "synthetic"
            }
        }
        let reads = Arc::new(AtomicUsize::new(0));
        let mut services = crate::domain::services::Services::new(directory.path(), None).unwrap();
        services.auth = Arc::new(crate::twitch::AuthService::new(
            crate::twitch::HttpTwitchApi::new().unwrap(),
            None,
            Box::new(CountingStore(reads.clone())),
        ));
        services
            .settings
            .set_streamlink_path(Some("/nonexistent/private/executable".into()))
            .unwrap();
        // Either a probe or credential access would fail. Reporting only reads local snapshots.
        let report = services.support_report().await.text;
        assert!(report.contains("Streamlink: not checked"));
        assert!(!report.contains("private"));
        assert!(services.sessions.sessions().await.is_empty());
        assert_eq!(reads.load(Ordering::SeqCst), 0);
    }
}
