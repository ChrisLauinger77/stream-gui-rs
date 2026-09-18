#![cfg(feature = "test-support")]

use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use twitch_gui_rs::{
    domain::{ErrorCode, LaunchRequest},
    streamlink::{self, LogSource, SessionPhase, SessionSnapshot, Supervisor},
};

fn helper() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_fake-streamlink"))
}
fn request(channel: &str) -> LaunchRequest {
    LaunchRequest {
        url: format!("https://www.twitch.tv/{channel}"),
        quality: "best".into(),
    }
}

async fn terminal(supervisor: &Supervisor, id: &str) -> SessionSnapshot {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let snapshot = supervisor
                .sessions()
                .await
                .into_iter()
                .find(|s| s.id == id)
                .unwrap();
            if matches!(snapshot.phase, SessionPhase::Exited | SessionPhase::Failed) {
                return snapshot;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("session should finish")
}

fn renamed_helper(directory: &Path, name: &str) -> PathBuf {
    let path = directory.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(helper(), &path).unwrap();
    path
}

#[tokio::test]
async fn probe_success_invalid_version_nonzero_and_timeout() {
    let result = streamlink::probe(helper().to_str(), Duration::from_secs(2))
        .await
        .unwrap();
    assert_eq!(result.version, "8.6.1");
    let directory = tempfile::tempdir().unwrap();
    for (name, code) in [
        ("badversion", ErrorCode::InvalidExecutable),
        ("badexit", ErrorCode::ProbeFailed),
        ("timeout", ErrorCode::Timeout),
    ] {
        let helper = renamed_helper(directory.path(), name);
        let error = streamlink::probe(helper.to_str(), Duration::from_millis(300))
            .await
            .unwrap_err();
        assert_eq!(error.code, code);
    }
}

#[tokio::test]
async fn spawn_failure_does_not_create_a_session() {
    let supervisor = Supervisor::default();
    let directory = tempfile::tempdir().unwrap();
    let error = supervisor
        .launch(&directory.path().join("missing"), request("example"))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::SpawnFailed);
    assert!(supervisor.sessions().await.is_empty());
}

#[tokio::test]
async fn reads_both_pipes_and_partial_lines_and_reaps_exit() {
    let supervisor = Supervisor::default();
    let session = supervisor
        .launch(helper(), request("example"))
        .await
        .unwrap();
    let ended = terminal(&supervisor, &session.id).await;
    assert_eq!(ended.phase, SessionPhase::Exited);
    assert_eq!(ended.exit_code, Some(0));
    for (source, text) in [
        (LogSource::Stdout, "stdout ready"),
        (LogSource::Stderr, "stderr ready"),
        (LogSource::Stdout, "partial stdout"),
        (LogSource::Stderr, "partial stderr"),
    ] {
        assert!(
            ended
                .logs
                .iter()
                .any(|entry| entry.source == source && entry.text == text),
            "{text}"
        );
    }
    #[cfg(unix)]
    {
        let mut status = 0;
        // SAFETY: checking whether our specific already-finished child is reaped.
        assert_eq!(
            unsafe { libc::waitpid(session.pid as i32, &mut status, libc::WNOHANG) },
            -1
        );
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
    }
    let stopped = supervisor.stop(&session.id).await.unwrap();
    assert!(!stopped.stop_requested);
    assert_eq!(stopped.exit_code, Some(0));
}

#[tokio::test]
async fn nonzero_exit_is_failed() {
    let supervisor = Supervisor::default();
    let session = supervisor.launch(helper(), request("fail")).await.unwrap();
    let ended = terminal(&supervisor, &session.id).await;
    assert_eq!(ended.phase, SessionPhase::Failed);
    assert_eq!(ended.exit_code, Some(7));
}

#[tokio::test]
async fn repeated_concurrent_stop_is_idempotent_and_sessions_are_independent() {
    let supervisor = Supervisor::default();
    let first = supervisor.launch(helper(), request("hold")).await.unwrap();
    let second = supervisor.launch(helper(), request("hold")).await.unwrap();
    assert_ne!(first.id, second.id);
    let (a, b) = tokio::join!(supervisor.stop(&first.id), supervisor.stop(&first.id));
    assert_eq!(a.unwrap().phase, SessionPhase::Exited);
    assert!(b.unwrap().stop_requested);
    assert_eq!(
        supervisor.stop(&first.id).await.unwrap().phase,
        SessionPhase::Exited
    );
    assert_eq!(supervisor.sessions().await[1].phase, SessionPhase::Running);
    supervisor.shutdown().await.unwrap();
    assert!(
        supervisor
            .sessions()
            .await
            .iter()
            .all(|s| s.phase == SessionPhase::Exited)
    );
    assert!(supervisor.launch(helper(), request("hold")).await.is_err());
}

#[tokio::test]
async fn flood_is_drained_with_bounded_sanitized_logs() {
    let supervisor = Supervisor::default();
    let session = supervisor.launch(helper(), request("flood")).await.unwrap();
    let ended = terminal(&supervisor, &session.id).await;
    assert_eq!(ended.exit_code, Some(0));
    assert_eq!(ended.logs.len(), 200);
    assert!(ended.dropped_log_entries > 7000);
    assert!(ended.logs.iter().all(|line| line.text.len() <= 2048));
    assert!(
        ended
            .logs
            .iter()
            .any(|line| line.text == "[oversize diagnostic line omitted]")
    );
    assert!(ended.logs.iter().any(|line| line.text == "last line"));
    assert!(
        !serde_json::to_string(&ended)
            .unwrap()
            .contains("DO_NOT_LEAK")
    );
}

#[tokio::test]
async fn session_history_is_bounded() {
    let supervisor = Supervisor::default();
    for _ in 0..20 {
        let session = supervisor
            .launch(helper(), request("example"))
            .await
            .unwrap();
        terminal(&supervisor, &session.id).await;
    }
    assert_eq!(supervisor.sessions().await.len(), 16);
}

#[tokio::test]
async fn stop_cleans_up_inherited_descendant_pipes() {
    let supervisor = Supervisor::default();
    let session = supervisor.launch(helper(), request("tree")).await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if supervisor.sessions().await[0]
                .logs
                .iter()
                .any(|line| line.text == "ready")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let ended = supervisor.stop(&session.id).await.unwrap();
    assert_eq!(ended.phase, SessionPhase::Exited);
    assert!(
        !ended
            .logs
            .iter()
            .any(|line| line.text.contains("pipe remained open"))
    );
}
