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

use stream_gui_rs::streamlink::playback::{
    LaunchSpec, PlaybackStream, PlayerSettings, QualityPolicy, RestartRequest,
};
fn production_spec(channel: &str) -> LaunchSpec {
    LaunchSpec {
        executable: helper().to_path_buf(),
        player: None,
        player_settings: PlayerSettings::default(),
        quality: QualityPolicy::Source,
        stream: PlaybackStream {
            stream_id: Some(format!("stream-{channel}")),
            broadcaster_id: format!("broadcaster-{channel}"),
            login: channel.into(),
            display_name: format!("Channel {channel}"),
            title: Some("A test stream".into()),
            category: Some("Test category".into()),
        },
    }
}
async fn starting(supervisor: &Supervisor, id: &str) {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if supervisor
                .sessions()
                .await
                .iter()
                .any(|s| s.id == id && s.phase == SessionPhase::Starting)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn production_launch_keeps_metadata_and_uses_exact_native_argv() {
    let supervisor = Supervisor::default();
    let mut spec = production_spec("arguments");
    spec.quality = QualityPolicy::High;
    spec.player_settings.arguments = vec![
        "one literal argument".into(),
        String::new(),
        "{playerinput}".into(),
    ];
    let session = supervisor.launch_spec(spec).await.unwrap();
    let ended = terminal(&supervisor, &session.id).await;
    assert_eq!(ended.generation, 1);
    assert!(ended.started_at > 0);
    assert!(ended.ended_at.is_some());
    assert_eq!(
        ended.stream.as_ref().unwrap().stream_id.as_deref(),
        Some("stream-arguments")
    );
    assert_eq!(
        ended.stream.as_ref().unwrap().broadcaster_id,
        "broadcaster-arguments"
    );
    let args: Vec<String> = serde_json::from_str(
        &ended
            .logs
            .iter()
            .find(|l| l.source == LogSource::Stdout)
            .unwrap()
            .text,
    )
    .unwrap();
    assert_eq!(
        args,
        [
            "--no-config",
            "--no-plugin-sideloading",
            "--loglevel",
            "info",
            "--player-verbose",
            "--player-args",
            "'one literal argument' '' '{{playerinput}}' {playerinput}",
            "--stream-sorting-excludes",
            ">720p30",
            "--",
            "https://www.twitch.tv/arguments",
            "high,best,best-unfiltered"
        ]
    );
}

#[tokio::test]
async fn delayed_output_split_utf8_and_error_logs_do_not_infer_playback_state() {
    let supervisor = Supervisor::default();
    let session = supervisor
        .launch_spec(production_spec("delayed"))
        .await
        .unwrap();
    assert_eq!(session.phase, SessionPhase::Running);
    assert!(session.logs.is_empty());
    let ended = terminal(&supervisor, &session.id).await;
    assert_eq!(ended.phase, SessionPhase::Exited);
    assert_eq!(ended.failure, None);
    for value in [
        "Starting player: advisory only",
        "warning: synthetic warning",
        "error: synthetic diagnostic, still running",
        "split UTF-8: 日本語",
    ] {
        assert!(ended.logs.iter().any(|line| line.text == value), "{value}");
    }
}

#[tokio::test]
async fn early_failure_and_later_streamlink_exit_are_distinct_while_another_session_runs() {
    let supervisor = Supervisor::default();
    let other = supervisor
        .launch_spec(production_spec("holdb"))
        .await
        .unwrap();
    let early = supervisor
        .launch_spec(production_spec("fail"))
        .await
        .unwrap();
    assert_eq!(
        terminal(&supervisor, &early.id).await.failure,
        Some(ErrorCode::StartupFailed)
    );
    let later = supervisor
        .launch_spec(production_spec("latefail"))
        .await
        .unwrap();
    assert_eq!(
        terminal(&supervisor, &later.id).await.failure,
        Some(ErrorCode::StreamlinkExited)
    );
    assert_eq!(
        supervisor
            .sessions()
            .await
            .iter()
            .find(|s| s.id == other.id)
            .unwrap()
            .phase,
        SessionPhase::Running
    );
    supervisor.shutdown().await.unwrap();
}

#[tokio::test]
async fn restart_reaps_previous_generation_preserves_identity_and_leaves_other_session_alone() {
    let supervisor = Supervisor::default();
    let first = supervisor
        .launch_spec(production_spec("hold"))
        .await
        .unwrap();
    let second = supervisor
        .launch_spec(production_spec("holdb"))
        .await
        .unwrap();
    let restarted = supervisor
        .restart(&first.id, first.generation, |stream| async move {
            let mut spec = production_spec("hold");
            spec.stream = stream;
            spec.quality = QualityPolicy::Audio;
            Ok(spec)
        })
        .await
        .unwrap();
    assert_eq!(restarted.id, first.id);
    assert_eq!(restarted.generation, 2);
    assert_ne!(restarted.pid, first.pid);
    assert_process_exited(first.pid);
    assert_eq!(restarted.quality_policy, Some(QualityPolicy::Audio));
    assert_eq!(
        restarted.stream.unwrap().stream_id,
        first.stream.unwrap().stream_id
    );
    assert_eq!(
        supervisor
            .sessions()
            .await
            .iter()
            .find(|s| s.id == second.id)
            .unwrap()
            .pid,
        second.pid
    );
    supervisor.stop(&first.id).await.unwrap();
    assert_process_exited(restarted.pid);
    assert_eq!(
        supervisor
            .sessions()
            .await
            .iter()
            .find(|s| s.id == second.id)
            .unwrap()
            .phase,
        SessionPhase::Running
    );
    supervisor.shutdown().await.unwrap();
    assert_process_exited(second.pid);
}

#[tokio::test]
async fn rapid_restart_requests_cannot_overlap_or_reuse_an_old_generation() {
    let supervisor = std::sync::Arc::new(Supervisor::default());
    let first = supervisor
        .launch_spec(production_spec("hold"))
        .await
        .unwrap();
    let (release, wait) = tokio::sync::oneshot::channel::<()>();
    let work = {
        let supervisor = supervisor.clone();
        let id = first.id.clone();
        tokio::spawn(async move {
            supervisor
                .restart(&id, 1, |_| async {
                    wait.await.unwrap();
                    Ok(production_spec("hold"))
                })
                .await
        })
    };
    starting(&supervisor, &first.id).await;
    assert_eq!(
        supervisor
            .restart(&first.id, 1, |_| async { Ok(production_spec("hold")) })
            .await
            .unwrap_err()
            .code,
        ErrorCode::RestartFailed
    );
    release.send(()).unwrap();
    let restarted = work.await.unwrap().unwrap();
    assert_eq!(
        supervisor
            .restart(&first.id, 1, |_| async { Ok(production_spec("hold")) })
            .await
            .unwrap_err()
            .code,
        ErrorCode::RestartFailed
    );
    assert_eq!(supervisor.sessions().await.len(), 1);
    assert_eq!(restarted.generation, 2);
    supervisor.shutdown().await.unwrap();
}

#[tokio::test]
async fn stop_cancels_pending_restart_before_it_can_spawn() {
    let supervisor = std::sync::Arc::new(Supervisor::default());
    let first = supervisor
        .launch_spec(production_spec("hold"))
        .await
        .unwrap();
    let (release, wait) = tokio::sync::oneshot::channel::<()>();
    let work = {
        let supervisor = supervisor.clone();
        let id = first.id.clone();
        tokio::spawn(async move {
            supervisor
                .restart(&id, 1, |_| async {
                    wait.await.unwrap();
                    Ok(production_spec("hold"))
                })
                .await
        })
    };
    starting(&supervisor, &first.id).await;
    let stop = supervisor.stop(&first.id);
    tokio::pin!(stop);
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut stop)
            .await
            .is_err()
    );
    release.send(()).unwrap();
    assert_eq!(work.await.unwrap().unwrap_err().code, ErrorCode::Cancelled);
    let ended = stop.await.unwrap();
    assert_eq!(ended.phase, SessionPhase::Exited);
    assert_eq!(ended.failure, Some(ErrorCode::Cancelled));
    assert_process_exited(first.pid);
    supervisor.stop(&first.id).await.unwrap();
}

#[tokio::test]
async fn restart_failure_retains_the_session_and_diagnostics_without_a_child() {
    let supervisor = Supervisor::default();
    let first = supervisor
        .launch_spec(production_spec("hold"))
        .await
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let error = supervisor
        .restart(&first.id, 1, |_| async {
            let mut spec = production_spec("hold");
            spec.executable = directory.path().join("missing");
            Ok(spec)
        })
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::SpawnFailed);
    assert_process_exited(first.pid);
    let failed = supervisor.sessions().await.pop().unwrap();
    assert_eq!(failed.failure, Some(ErrorCode::RestartFailed));
    assert!(!failed.restarting);
    assert!(failed.logs.iter().any(|l| l.text.contains("SpawnFailed")));
    supervisor.shutdown().await.unwrap();
}

#[tokio::test]
async fn service_restart_uses_current_settings_and_survives_a_dropped_caller() {
    let root = tempfile::tempdir().unwrap();
    let mut services = Services::new(root.path(), None).unwrap();
    services.auth = std::sync::Arc::new(stream_gui_rs::twitch::AuthService::new(
        stream_gui_rs::twitch::HttpTwitchApi::new().unwrap(),
        Some("synthetic-client".into()),
        Box::<stream_gui_rs::credentials::MemoryCredentialStore>::default(),
    ));
    let services = std::sync::Arc::new(services);
    services
        .settings
        .set_streamlink_path(Some(helper().to_string_lossy().into_owned()))
        .unwrap();
    let first = services
        .sessions
        .launch_spec(production_spec("hold"))
        .await
        .unwrap();
    let mut settings = services.settings.snapshot();
    settings.default_quality = QualityPolicy::Low;
    services.settings.update(settings).unwrap();
    let caller = {
        let services = services.clone();
        let id = first.id.clone();
        tokio::spawn(async move {
            services
                .restart_playback(RestartRequest {
                    session_id: id,
                    generation: 1,
                    quality: None,
                })
                .await
        })
    };
    tokio::time::sleep(Duration::from_millis(10)).await;
    caller.abort();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let state = services.sessions.sessions().await.pop().unwrap();
            if state.generation == 2 && state.phase == SessionPhase::Running && !state.restarting {
                assert_eq!(state.quality_policy, Some(QualityPolicy::Low));
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    services.auth.logout().await.unwrap();
    assert_eq!(
        services.sessions.sessions().await[0].phase,
        SessionPhase::Running
    );
    services.shutdown().await.unwrap();
}

#[tokio::test]
async fn shutdown_cancels_restart_and_cleans_multiple_owned_sessions() {
    let supervisor = std::sync::Arc::new(Supervisor::default());
    let first = supervisor
        .launch_spec(production_spec("hold"))
        .await
        .unwrap();
    let second = supervisor
        .launch_spec(production_spec("holdb"))
        .await
        .unwrap();
    let (release, wait) = tokio::sync::oneshot::channel::<()>();
    let work = {
        let supervisor = supervisor.clone();
        let id = first.id.clone();
        tokio::spawn(async move {
            supervisor
                .restart(&id, 1, |_| async {
                    wait.await.unwrap();
                    Ok(production_spec("hold"))
                })
                .await
        })
    };
    starting(&supervisor, &first.id).await;
    let shutdown = supervisor.shutdown();
    tokio::pin!(shutdown);
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut shutdown)
            .await
            .is_err()
    );
    release.send(()).unwrap();
    shutdown.await.unwrap();
    assert_eq!(work.await.unwrap().unwrap_err().code, ErrorCode::Cancelled);
    assert_process_exited(first.pid);
    assert_process_exited(second.pid);
    assert!(
        supervisor
            .sessions()
            .await
            .iter()
            .all(|s| s.phase == SessionPhase::Exited && !s.restarting)
    );
}

#[tokio::test]
async fn restarting_terminal_history_cannot_exceed_the_active_session_limit() {
    let supervisor = Supervisor::default();
    let finished = supervisor
        .launch_spec(production_spec("hold"))
        .await
        .unwrap();
    supervisor.stop(&finished.id).await.unwrap();
    for _ in 0..8 {
        supervisor
            .launch_spec(production_spec("holdb"))
            .await
            .unwrap();
    }
    assert_eq!(
        supervisor
            .restart(&finished.id, 1, |_| async { Ok(production_spec("hold")) })
            .await
            .unwrap_err()
            .code,
        ErrorCode::Capacity
    );
    assert_eq!(
        supervisor
            .launch_spec(production_spec("hold"))
            .await
            .unwrap_err()
            .code,
        ErrorCode::Capacity
    );
    assert_eq!(
        supervisor
            .sessions()
            .await
            .iter()
            .filter(|s| s.phase == SessionPhase::Running)
            .count(),
        8
    );
    supervisor.shutdown().await.unwrap();
}

#[tokio::test]
async fn dropping_a_restart_future_clears_the_reservation_without_leaking_a_child() {
    let supervisor = std::sync::Arc::new(Supervisor::default());
    let first = supervisor
        .launch_spec(production_spec("hold"))
        .await
        .unwrap();
    let work = {
        let supervisor = supervisor.clone();
        let id = first.id.clone();
        tokio::spawn(async move {
            supervisor
                .restart(&id, 1, |_| async {
                    std::future::pending::<()>().await;
                    Ok(production_spec("hold"))
                })
                .await
        })
    };
    starting(&supervisor, &first.id).await;
    work.abort();
    let _ = work.await;
    let snapshot = supervisor.sessions().await.pop().unwrap();
    assert!(!snapshot.restarting);
    assert_eq!(snapshot.phase, SessionPhase::Exited);
    assert_eq!(snapshot.failure, Some(ErrorCode::Cancelled));
    assert_process_exited(first.pid);
    let next = supervisor
        .restart(&first.id, 2, |_| async { Ok(production_spec("hold")) })
        .await
        .unwrap();
    assert_eq!(next.generation, 3);
    supervisor.shutdown().await.unwrap();
    assert_process_exited(next.pid);
}

#[tokio::test]
async fn cancelled_authentication_cannot_spawn_a_prepared_launch() {
    let supervisor = Supervisor::default();
    let cancel = tokio_util::sync::CancellationToken::new();
    cancel.cancel();
    assert_eq!(
        supervisor
            .launch_authenticated(production_spec("hold"), &cancel)
            .await
            .unwrap_err()
            .code,
        ErrorCode::Unauthenticated
    );
    assert!(supervisor.sessions().await.is_empty());
}
