#![cfg(feature = "test-support")]

use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use stream_gui_rs::{
    domain::{ErrorCode, LaunchRequest, services::Services},
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

async fn shutdown_during_probe(launch: bool) {
    let directory = tempfile::tempdir().unwrap();
    let executable = renamed_helper(directory.path(), "timeout");
    let services = std::sync::Arc::new(Services::new(directory.path(), None).unwrap());
    let playing = services
        .sessions
        .launch(helper(), request("hold"))
        .await
        .unwrap();
    services
        .settings
        .set_streamlink_path(Some(executable.to_str().unwrap().into()))
        .unwrap();
    let operation = {
        let services = services.clone();
        tokio::spawn(async move {
            if launch {
                services.launch(request("hold")).await.map(|_| ())
            } else {
                services
                    .probe(Some(executable.to_str().unwrap().into()))
                    .await
                    .map(|_| ())
            }
        })
    };
    let marker = directory.path().join("timeout.pid");
    let pid: u32 = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(pid) = std::fs::read_to_string(&marker) {
                if let Ok(pid) = pid.parse() {
                    break pid;
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    // Queue work behind the active probe before shutdown starts.
    let queued = services.probe(Some(helper().to_str().unwrap().into()));
    tokio::pin!(queued);
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut queued)
            .await
            .is_err()
    );
    let shutdown = services.shutdown();
    tokio::pin!(shutdown);
    // Shutdown must remain pending while the child is alive.
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut shutdown)
            .await
            .is_err()
    );
    // Existing playback stops without waiting for the unrelated probe timeout.
    let ended = tokio::time::timeout(
        Duration::from_secs(1),
        terminal(&services.sessions, &playing.id),
    )
    .await
    .unwrap();
    assert!(ended.stop_requested);
    assert_eq!(
        services.probe(None).await.unwrap_err().code,
        ErrorCode::ProcessFailed
    );
    assert_eq!(
        services.launch(request("hold")).await.unwrap_err().code,
        ErrorCode::ProcessFailed
    );
    let (closed, queued) = tokio::join!(shutdown, queued);
    closed.unwrap();
    assert_eq!(queued.unwrap_err().code, ErrorCode::ProcessFailed);
    assert_eq!(
        operation.await.unwrap().unwrap_err().code,
        ErrorCode::Timeout
    );
    let sessions = services.sessions.sessions().await;
    assert_eq!(
        sessions.len(),
        1,
        "shutdown must not launch another session"
    );
    assert_eq!(sessions[0].phase, SessionPhase::Exited);
    assert_process_exited(pid);
    services.shutdown().await.unwrap();
}

fn assert_process_exited(pid: u32) {
    #[cfg(unix)]
    {
        // SAFETY: signal 0 only queries the helper process we created.
        assert_eq!(unsafe { libc::kill(pid as i32, 0) }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, ERROR_INVALID_PARAMETER, WAIT_OBJECT_0},
            System::Threading::*,
        };
        // SAFETY: query-only handle for our helper PID; close it exactly once.
        unsafe {
            let process = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
            if !process.is_null() {
                let result = WaitForSingleObject(process, 5000);
                CloseHandle(process);
                assert_eq!(result, WAIT_OBJECT_0);
            } else {
                assert_eq!(
                    std::io::Error::last_os_error().raw_os_error(),
                    Some(ERROR_INVALID_PARAMETER as i32)
                );
            }
        }
    }
}

#[tokio::test]
async fn service_shutdown_waits_for_standalone_probe_and_rejects_queued_work() {
    shutdown_during_probe(false).await;
}

#[tokio::test]
async fn service_shutdown_waits_for_launch_probe_without_creating_session() {
    shutdown_during_probe(true).await;
}

#[tokio::test]
async fn custom_path_with_spaces_roundtrips_and_launches() {
    let directory = tempfile::Builder::new()
        .prefix("streamlink paths ")
        .tempdir()
        .unwrap();
    let executable = renamed_helper(directory.path(), "custom streamlink");
    let selected = executable.to_str().unwrap().to_owned();
    let services = Services::new(directory.path(), None).unwrap();
    services.probe(Some(selected.clone())).await.unwrap();
    drop(services);
    let services = Services::new(directory.path(), None).unwrap();
    assert_eq!(services.settings.snapshot().streamlink_path, Some(selected));
    let session = services.launch(request("example")).await.unwrap();
    assert_eq!(
        terminal(&services.sessions, &session.id).await.exit_code,
        Some(0)
    );
    services.shutdown().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn saved_symlink_survives_retargeting_and_removal_of_old_version() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir().unwrap();
    let old = renamed_helper(directory.path(), "streamlink-v1");
    let new = renamed_helper(directory.path(), "streamlink-v2");
    let selected = directory.path().join("streamlink");
    symlink(&old, &selected).unwrap();
    let services = Services::new(directory.path(), None).unwrap();
    let probe = services
        .probe(Some(selected.to_str().unwrap().into()))
        .await
        .unwrap();
    assert_eq!(Path::new(&probe.executable), old.canonicalize().unwrap());
    drop(services);
    std::fs::remove_file(&selected).unwrap();
    symlink(&new, &selected).unwrap();
    std::fs::remove_file(old).unwrap();
    let services = Services::new(directory.path(), None).unwrap();
    assert_eq!(
        services.settings.snapshot().streamlink_path.as_deref(),
        selected.to_str()
    );
    let session = services.launch(request("example")).await.unwrap();
    assert_eq!(
        terminal(&services.sessions, &session.id).await.exit_code,
        Some(0)
    );
    services.shutdown().await.unwrap();
}

#[cfg(windows)]
#[tokio::test]
async fn windows_child_cannot_execute_before_job_assignment_and_descendant_is_owned() {
    use stream_gui_rs::platform::{ProcessTree, configure_process};
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("descendant.pid");
    let mut command = tokio::process::Command::new(helper());
    command.arg("--immediate-tree").arg(&marker);
    configure_process(&mut command);
    let mut child = command.spawn().unwrap();
    let parent_pid = child.id().unwrap();
    // Deliberately widen the old race: no application code may execute here.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        !marker.exists(),
        "child ran before assignment to its cleanup job"
    );
    let tree = ProcessTree::attach(&child).unwrap();
    let descendant = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(contents) = std::fs::read_to_string(&marker) {
                if let Ok(pid) = contents.parse::<u32>() {
                    break pid;
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    drop(tree);
    child.wait().await.unwrap();
    assert_process_exited(parent_pid);
    assert_process_exited(descendant);
}
