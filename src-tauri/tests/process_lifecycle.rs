#![cfg(feature = "test-support")]

use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use stream_gui_rs::{
    domain::{ErrorCode, LaunchRequest, services::Services},
    streamlink::{self, LogSource, SessionPhase, SessionSnapshot, Supervisor},
};
#[cfg(target_os = "linux")]
#[path = "support/chatterino_flatpak.rs"]
mod chatterino_flatpak;

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

fn helper_directory() -> tempfile::TempDir {
    // Keep renamed helpers on the binary's filesystem so hard links work even
    // when the system temporary directory is on a different mount.
    tempfile::Builder::new()
        .prefix("streamlink paths ")
        .tempdir_in(helper().parent().unwrap())
        .unwrap()
}

fn renamed_helper(directory: &Path, name: &str) -> PathBuf {
    let path = directory.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    // Avoid writing executable bytes while other tests spawn children: Linux
    // can reject immediate execution of freshly copied helpers with ETXTBSY.
    // These fixtures need distinct names and never modify the helper bytes.
    std::fs::hard_link(helper(), &path).unwrap();
    path
}

#[tokio::test]
async fn probe_success_invalid_version_nonzero_and_timeout() {
    let result = streamlink::probe(helper().to_str(), Duration::from_secs(2))
        .await
        .unwrap();
    assert_eq!(result.version, "8.6.1");
    let directory = helper_directory();
    for (name, code) in [
        ("badversion", ErrorCode::InvalidExecutable),
        ("badexit", ErrorCode::ProbeFailed),
        ("timeout", ErrorCode::Timeout),
    ] {
        let helper = renamed_helper(directory.path(), name);
        let error = streamlink::probe(helper.to_str(), Duration::from_millis(300))
            .await
            .unwrap_err();
        assert_eq!(error.code, code, "{name}: {error:?}");
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
async fn flood_is_drained_with_bounded_logs() {
    let supervisor = Supervisor::default();
    let session = supervisor.launch(helper(), request("flood")).await.unwrap();
    let ended = terminal(&supervisor, &session.id).await;
    assert_eq!(ended.phase, SessionPhase::Exited);
    assert_eq!(ended.exit_code, Some(0));
    assert_eq!(ended.logs.len(), 200);
    // Both 4,000-line pipes must drain completely, but their relative read order
    // is unspecified. Either pipe can evict the other one's final diagnostics.
    assert_eq!(ended.dropped_log_entries, 7800);
    assert!(ended.logs.iter().all(|line| line.text.len() <= 2048));
    assert!(matches!(
        ended.logs.back().unwrap().text.as_str(),
        "stdout 3999" | "stderr 3999"
    ));
}

#[tokio::test]
async fn oversized_and_credential_diagnostics_are_suppressed_on_both_pipes() {
    let supervisor = Supervisor::default();
    let session = supervisor
        .launch(helper(), request("diagnostics"))
        .await
        .unwrap();
    let ended = terminal(&supervisor, &session.id).await;
    assert_eq!(ended.phase, SessionPhase::Exited);
    assert_eq!(ended.exit_code, Some(0));
    // Keep this fixture below the retention cap so cross-pipe scheduling cannot
    // evict evidence of sanitization or make the no-secret assertion vacuous.
    assert_eq!(ended.dropped_log_entries, 0);
    assert_eq!(ended.logs.len(), 6);
    for source in [LogSource::Stdout, LogSource::Stderr] {
        let lines: Vec<_> = ended
            .logs
            .iter()
            .filter(|line| line.source == source)
            .map(|line| line.text.as_str())
            .collect();
        assert_eq!(
            lines,
            [
                "[oversize diagnostic line omitted]",
                "[credential-related diagnostic omitted]",
                "last line",
            ]
        );
    }
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
    let directory = helper_directory();
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
    let directory = helper_directory();
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
    let directory = helper_directory();
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
        settings: stream_gui_rs::config::EffectivePlaybackSettings {
            profile_id: None,
            chat_provider: stream_gui_rs::config::ChatProvider::Browser,
            chatterino_path: None,
            low_latency: false,
            streamlink_path: None,
            player: PlayerSettings::default(),
            quality: QualityPolicy::Source,
            automatic_chat: false,
        },
        stream: PlaybackStream {
            stream_id: Some(format!("stream-{channel}")),
            broadcaster_id: if channel == "holdb" { "456" } else { "123" }.into(),
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
    spec.settings.quality = QualityPolicy::High;
    spec.settings.player.arguments = vec![
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
    assert_eq!(ended.stream.as_ref().unwrap().broadcaster_id, "123");
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
            spec.settings.quality = QualityPolicy::Audio;
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

#[tokio::test]
async fn settings_changes_preserve_runs_and_restart_resolves_channel_then_request_overrides() {
    use stream_gui_rs::config::{ChannelOverrides, SaveChannelSettingsRequest};
    let root = tempfile::tempdir().unwrap();
    let services = std::sync::Arc::new(Services::new(root.path(), None).unwrap());
    services
        .settings
        .set_streamlink_path(Some(helper().to_string_lossy().into_owned()))
        .unwrap();
    let first = services
        .sessions
        .launch_spec(production_spec("hold"))
        .await
        .unwrap();
    let second = services
        .sessions
        .launch_spec(production_spec("holdb"))
        .await
        .unwrap();
    let original = first.effective_settings.clone().unwrap();
    let mut global = services.settings.snapshot();
    global.default_quality = QualityPolicy::High;
    global.low_latency = true;
    global.player.arguments = vec!["--volume=20".into()];
    services.save_settings(global).await.unwrap();
    services
        .save_channel_settings(SaveChannelSettingsRequest {
            broadcaster_id: "123".into(),
            overrides: ChannelOverrides {
                low_latency: Some(false),
                notifications: None,
                quality: Some(QualityPolicy::Low),
                automatic_chat: None,
            },
        })
        .await
        .unwrap();
    let current = services.sessions.sessions().await;
    assert_eq!(current[0].pid, first.pid);
    assert_eq!(current[0].effective_settings.as_ref().unwrap(), &original);
    assert_eq!(current[1].pid, second.pid);
    let restarted = services
        .restart_playback(RestartRequest {
            session_id: first.id.clone(),
            generation: 1,
            quality: None,
        })
        .await
        .unwrap();
    assert_process_exited(first.pid);
    assert_eq!(restarted.id, first.id);
    assert!(!restarted.effective_settings.as_ref().unwrap().low_latency);
    assert!(!original.low_latency);
    assert_eq!(restarted.quality_policy, Some(QualityPolicy::Low));
    assert_eq!(
        restarted
            .effective_settings
            .as_ref()
            .unwrap()
            .player
            .arguments,
        ["--volume=20"]
    );
    assert_eq!(services.sessions.sessions().await[1].pid, second.pid);
    services
        .save_channel_settings(SaveChannelSettingsRequest {
            broadcaster_id: "123".into(),
            overrides: ChannelOverrides::default(),
        })
        .await
        .unwrap();
    let inherited = services
        .restart_playback(RestartRequest {
            session_id: first.id.clone(),
            generation: restarted.generation,
            quality: None,
        })
        .await
        .unwrap();
    assert_eq!(inherited.quality_policy, Some(QualityPolicy::High));
    assert!(inherited.effective_settings.as_ref().unwrap().low_latency);
    let explicit = services
        .restart_playback(RestartRequest {
            session_id: first.id.clone(),
            generation: inherited.generation,
            quality: Some(QualityPolicy::Audio),
        })
        .await
        .unwrap();
    assert_eq!(explicit.quality_policy, Some(QualityPolicy::Audio));
    assert!(explicit.effective_settings.as_ref().unwrap().low_latency);
    assert!(
        !services.sessions.sessions().await[1]
            .effective_settings
            .as_ref()
            .unwrap()
            .low_latency
    );
    assert_eq!(
        services.settings.channel("123").unwrap().overrides,
        ChannelOverrides::default()
    );
    services.shutdown().await.unwrap();
    assert_process_exited(explicit.pid);
    assert_process_exited(second.pid);
}

#[tokio::test]
async fn unsupported_streamlink_probe_does_not_replace_saved_path() {
    let root = helper_directory();
    let services = Services::new(root.path(), None).unwrap();
    services
        .probe(Some(helper().to_string_lossy().into_owned()))
        .await
        .unwrap();
    let before = std::fs::read(services.settings.path()).unwrap();
    let unsupported = renamed_helper(root.path(), "unsupported");
    assert_eq!(
        services
            .probe(Some(unsupported.to_string_lossy().into_owned()))
            .await
            .unwrap_err()
            .code,
        ErrorCode::UnsupportedStreamlink
    );
    assert_eq!(std::fs::read(services.settings.path()).unwrap(), before);
    let mut settings = services.settings.snapshot();
    settings.streamlink_path = Some(unsupported.to_string_lossy().into_owned());
    assert_eq!(
        services.save_settings(settings).await.unwrap_err().code,
        ErrorCode::UnsupportedStreamlink
    );
    assert_eq!(std::fs::read(services.settings.path()).unwrap(), before);
    services.shutdown().await.unwrap();
}

#[tokio::test]
async fn automatic_chat_uses_effective_preferences_once_per_successful_run() {
    use stream_gui_rs::{
        config::{ChannelOverrides, SaveChannelSettingsRequest},
        domain::chat::{ChatOpener, ChatTarget},
    };
    #[derive(Default)]
    struct Recorder(std::sync::Mutex<Vec<String>>);
    impl ChatOpener for Recorder {
        fn open(&self, target: &ChatTarget) -> stream_gui_rs::domain::Result<()> {
            self.0.lock().unwrap().push(target.url().into());
            Ok(())
        }
    }
    let recorder = std::sync::Arc::new(Recorder::default());
    let root = tempfile::tempdir().unwrap();
    let services = std::sync::Arc::new(
        Services::new(root.path(), None)
            .unwrap()
            .with_chat_opener(recorder.clone()),
    );
    services
        .settings
        .set_streamlink_path(Some(helper().to_string_lossy().into_owned()))
        .unwrap();
    let mut session = services
        .sessions
        .launch_spec(production_spec("hold"))
        .await
        .unwrap();
    for (global, override_value, expected_opens) in [
        (false, None, 0),
        (true, None, 1),
        (true, Some(false), 1),
        (false, Some(true), 2),
        (false, None, 2),
    ] {
        let mut settings = services.settings.snapshot();
        settings.automatic_chat = global;
        services.save_settings(settings).await.unwrap();
        services
            .save_channel_settings(SaveChannelSettingsRequest {
                broadcaster_id: "123".into(),
                overrides: ChannelOverrides {
                    low_latency: None,
                    notifications: None,
                    automatic_chat: override_value,
                    quality: None,
                },
            })
            .await
            .unwrap();
        assert_eq!(services.sessions.sessions().await[0].pid, session.pid);
        session = services
            .restart_playback(RestartRequest {
                session_id: session.id,
                generation: session.generation,
                quality: None,
            })
            .await
            .unwrap();
        assert_eq!(session.phase, SessionPhase::Running);
        assert!(session.chat_error.is_none());
        assert_eq!(recorder.0.lock().unwrap().len(), expected_opens);
    }
    assert!(
        recorder
            .0
            .lock()
            .unwrap()
            .iter()
            .all(|url| url == "https://www.twitch.tv/popout/hold/chat")
    );
    let stale = services
        .sessions
        .record_chat_result(
            &session.id,
            session.generation - 1,
            Some(ErrorCode::BrowserOpen),
        )
        .await
        .unwrap();
    assert!(stale.chat_error.is_none());
    services.shutdown().await.unwrap();
}

#[tokio::test]
async fn failed_browser_open_keeps_playback_running_and_failed_restart_opens_nothing() {
    use stream_gui_rs::domain::chat::{ChatOpener, ChatTarget};
    #[derive(Default)]
    struct Failing(std::sync::atomic::AtomicUsize);
    impl ChatOpener for Failing {
        fn open(&self, _: &ChatTarget) -> stream_gui_rs::domain::Result<()> {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err(stream_gui_rs::domain::AppError::new(
                ErrorCode::Internal,
                "synthetic private native detail",
            ))
        }
    }
    let recorder = std::sync::Arc::new(Failing::default());
    let root = tempfile::tempdir().unwrap();
    let services = std::sync::Arc::new(
        Services::new(root.path(), None)
            .unwrap()
            .with_chat_opener(recorder.clone()),
    );
    let mut settings = services.settings.snapshot();
    settings.automatic_chat = true;
    settings.streamlink_path = Some(helper().to_string_lossy().into_owned());
    services.save_settings(settings).await.unwrap();
    let session = services
        .sessions
        .launch_spec(production_spec("hold"))
        .await
        .unwrap();
    let next = services
        .restart_playback(RestartRequest {
            session_id: session.id.clone(),
            generation: 1,
            quality: None,
        })
        .await
        .unwrap();
    assert_eq!(next.phase, SessionPhase::Running);
    assert_eq!(next.chat_error, Some(ErrorCode::BrowserOpen));
    assert!(
        !serde_json::to_string(&next)
            .unwrap()
            .contains("private native detail")
    );
    services
        .settings
        .set_streamlink_path(Some(
            root.path().join("missing").to_string_lossy().into_owned(),
        ))
        .unwrap();
    assert!(
        services
            .restart_playback(RestartRequest {
                session_id: session.id,
                generation: next.generation,
                quality: None
            })
            .await
            .is_err()
    );
    assert_eq!(recorder.0.load(std::sync::atomic::Ordering::SeqCst), 1);
    services.shutdown().await.unwrap();
}

#[tokio::test]
async fn support_report_reuses_validated_version_without_spawning_the_custom_executable() {
    let directory = helper_directory();
    let executable = renamed_helper(directory.path(), "reportprobe");
    let marker = executable.with_extension("count");
    let services = Services::new(directory.path(), None).unwrap();
    services
        .settings
        .set_streamlink_path(Some(executable.to_string_lossy().into()))
        .unwrap();
    assert!(
        services
            .support_report()
            .await
            .text
            .contains("Streamlink: not checked")
    );
    assert!(!marker.exists());
    services
        .probe(Some(executable.to_string_lossy().into()))
        .await
        .unwrap();
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "1");
    for _ in 0..3 {
        assert!(
            services
                .support_report()
                .await
                .text
                .contains("Streamlink: 8.6.1")
        );
    }
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "1");
    services.settings.set_streamlink_path(None).unwrap();
    assert!(
        services
            .support_report()
            .await
            .text
            .contains("Streamlink: not checked")
    );
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), "1");
}

async fn chat_fixture_result(executable: &Path) -> (u32, Vec<String>) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(data) = std::fs::read(executable.with_extension("chat.json")) {
                if let Ok(result) = serde_json::from_slice(&data) {
                    return result;
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn chatterino_launchers_are_reaped_and_independent_apps_survive_playback_shutdown() {
    use stream_gui_rs::{chatterino::Chatterino, config::ChatProvider};
    let directory = helper_directory();
    let chat_path = renamed_helper(directory.path(), "Chatterino 空 格");
    let chat = Chatterino::default();
    chat.open(chat_path.to_str(), "short").unwrap();
    let (pid, args) = chat_fixture_result(&chat_path).await;
    assert_eq!(args, ["--channels", "t:short"]);
    tokio::time::timeout(Duration::from_secs(3), async {
        while chat.active_launchers() != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_process_exited(pid);
    std::fs::remove_file(chat_path.with_extension("chat.json")).unwrap();
    let root = tempfile::tempdir().unwrap();
    let services = std::sync::Arc::new(Services::new(root.path(), None).unwrap());
    let mut settings = services.settings.snapshot();
    settings.streamlink_path = Some(helper().to_string_lossy().into_owned());
    settings.automatic_chat = true;
    settings.chat_provider = ChatProvider::Chatterino;
    settings.chatterino_path = Some(chat_path.to_string_lossy().into_owned());
    services.save_settings(settings).await.unwrap();
    let initial = services
        .sessions
        .launch_spec(production_spec("hold"))
        .await
        .unwrap();
    let restarted = services
        .restart_playback(RestartRequest {
            session_id: initial.id,
            generation: 1,
            quality: None,
        })
        .await
        .unwrap();
    assert_eq!(restarted.phase, SessionPhase::Running);
    assert_eq!(restarted.chat_error, None);
    let (chat_pid, _) = chat_fixture_result(&chat_path).await;
    services.sessions.stop(&restarted.id).await.unwrap();
    services.shutdown().await.unwrap();
    #[cfg(unix)]
    {
        // SAFETY: signal zero only checks the fixture PID, without sending a signal.
        assert_eq!(unsafe { libc::kill(chat_pid as i32, 0) }, 0);
        // Chat starts a separate session, outside all playback-owned process groups.
        assert_eq!(unsafe { libc::getsid(chat_pid as i32) }, chat_pid as i32);
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, WAIT_TIMEOUT},
            System::Threading::*,
        };
        // SAFETY: query-only handle for the known fixture; closed once.
        unsafe {
            let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, chat_pid);
            assert!(!handle.is_null());
            let state = WaitForSingleObject(handle, 0);
            CloseHandle(handle);
            assert_eq!(state, WAIT_TIMEOUT);
        }
    }
    std::fs::write(chat_path.with_extension("release"), "done").unwrap();
    #[cfg(unix)]
    tokio::time::timeout(Duration::from_secs(3), async {
        // SAFETY: signal zero queries only the known fixture process.
        while unsafe { libc::kill(chat_pid as i32, 0) } == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_process_exited(chat_pid);
}
#[tokio::test]
async fn chatterino_missing_and_spawn_failure_do_not_fail_playback_or_open_browser() {
    use stream_gui_rs::{
        config::ChatProvider,
        domain::chat::{ChatOpener, ChatTarget},
    };
    struct NoBrowser;
    impl ChatOpener for NoBrowser {
        fn open(&self, _: &ChatTarget) -> stream_gui_rs::domain::Result<()> {
            panic!("no silent fallback")
        }
    }
    let root = tempfile::tempdir().unwrap();
    let services = std::sync::Arc::new(
        Services::new(root.path(), None)
            .unwrap()
            .with_chat_opener(std::sync::Arc::new(NoBrowser)),
    );
    let mut settings = services.settings.snapshot();
    settings.streamlink_path = Some(helper().to_string_lossy().into_owned());
    settings.automatic_chat = true;
    settings.chat_provider = ChatProvider::Chatterino;
    settings.chatterino_path = Some(
        root.path()
            .join("absent.exe")
            .to_string_lossy()
            .into_owned(),
    );
    services.save_settings(settings.clone()).await.unwrap();
    let initial = services
        .sessions
        .launch_spec(production_spec("hold"))
        .await
        .unwrap();
    let restarted = services
        .restart_playback(RestartRequest {
            session_id: initial.id,
            generation: 1,
            quality: None,
        })
        .await
        .unwrap();
    assert_eq!(restarted.phase, SessionPhase::Running);
    assert_eq!(restarted.chat_error, Some(ErrorCode::ChatterinoNotFound));
    let invalid = root
        .path()
        .join(format!("invalid{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(&invalid, "not a native executable").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&invalid, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    settings.chatterino_path = Some(invalid.to_string_lossy().into_owned());
    services.save_settings(settings).await.unwrap();
    let failed_chat = services
        .restart_playback(RestartRequest {
            session_id: restarted.id,
            generation: restarted.generation,
            quality: None,
        })
        .await
        .unwrap();
    assert_eq!(failed_chat.phase, SessionPhase::Running);
    assert_eq!(failed_chat.chat_error, Some(ErrorCode::ChatLaunch));
    services.shutdown().await.unwrap();
}
#[tokio::test]
async fn profile_mutations_and_stale_global_drafts_preserve_sessions_and_restart_resolves_current_selection()
 {
    use stream_gui_rs::config::profiles::{ProfileDraft, ProfileMutation::*};
    let root = tempfile::tempdir().unwrap();
    let services = std::sync::Arc::new(Services::new(root.path(), None).unwrap());
    services
        .settings
        .set_streamlink_path(Some(helper().to_string_lossy().into_owned()))
        .unwrap();
    let first = services
        .sessions
        .launch_spec(production_spec("hold"))
        .await
        .unwrap();
    let second = services
        .sessions
        .launch_spec(production_spec("holdb"))
        .await
        .unwrap();
    let draft = ProfileDraft {
        name: "Desk".into(),
        player: PlayerSettings {
            arguments: vec!["--volume=23".into()],
            ..Default::default()
        },
        quality: Some(QualityPolicy::High),
        low_latency: Some(true),
    };
    let created = services
        .modify_profile(Create {
            profile: draft.clone(),
        })
        .await
        .unwrap();
    let id = created.profiles[0].id.clone();
    services
        .modify_profile(Select {
            id: Some(id.clone()),
        })
        .await
        .unwrap();
    let stale = services.settings.snapshot();
    let restarted = services
        .restart_playback(RestartRequest {
            session_id: first.id.clone(),
            generation: 1,
            quality: None,
        })
        .await
        .unwrap();
    assert_eq!(
        restarted
            .effective_settings
            .as_ref()
            .unwrap()
            .profile_id
            .as_ref(),
        Some(&id)
    );
    assert_eq!(restarted.quality_policy, Some(QualityPolicy::High));
    assert!(restarted.effective_settings.as_ref().unwrap().low_latency);
    let changed = ProfileDraft {
        name: "Renamed".into(),
        quality: Some(QualityPolicy::Low),
        ..draft
    };
    services
        .modify_profile(Update {
            id: id.clone(),
            profile: changed,
        })
        .await
        .unwrap();
    let second_run = services
        .restart_playback(RestartRequest {
            session_id: second.id.clone(),
            generation: 1,
            quality: None,
        })
        .await
        .unwrap();
    assert_eq!(second_run.quality_policy, Some(QualityPolicy::Low));
    assert_eq!(
        services.sessions.sessions().await[0].effective_settings,
        restarted.effective_settings
    );
    services.modify_profile(Delete { id }).await.unwrap();
    let mut stale = stale;
    stale.text_scale = stream_gui_rs::config::TextScale::Largest;
    let accepted = services.save_settings(stale).await.unwrap();
    assert!(accepted.profiles.is_empty());
    assert!(accepted.selected_profile_id.is_none());
    let default = services
        .restart_playback(RestartRequest {
            session_id: first.id,
            generation: restarted.generation,
            quality: None,
        })
        .await
        .unwrap();
    assert!(
        default
            .effective_settings
            .as_ref()
            .unwrap()
            .profile_id
            .is_none()
    );
    assert_eq!(default.quality_policy, Some(QualityPolicy::Source));
    assert_eq!(
        services.sessions.sessions().await[1].effective_settings,
        second_run.effective_settings
    );
    services.shutdown().await.unwrap();
}

#[tokio::test]
async fn chatterino_waiter_capacity_is_bounded_and_released_after_short_lived_clients() {
    let directory = helper_directory();
    let executable = renamed_helper(directory.path(), "bounded-chat");
    let chat = stream_gui_rs::chatterino::Chatterino::default();
    for _ in 0..16 {
        chat.open(executable.to_str(), "hold").unwrap();
    }
    assert_eq!(chat.active_launchers(), 16);
    assert_eq!(
        chat.open(executable.to_str(), "hold").unwrap_err().code,
        ErrorCode::ChatterinoCapacity
    );
    assert_eq!(chat.active_launchers(), 16);
    std::fs::write(executable.with_extension("release"), "done").unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while chat.active_launchers() != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    chat.open(executable.to_str(), "short").unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        while chat.active_launchers() != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn chatterino_child_does_not_inherit_a_synthetic_token_from_its_parent_environment() {
    let directory = helper_directory();
    let executable = renamed_helper(directory.path(), "environment-chat");
    let mut command = std::process::Command::new(helper());
    command
        .arg("--chatterino-parent")
        .arg(&executable)
        .env("STREAM_GUI_RS_SYNTHETIC_TOKEN", "oauth:synthetic-test-only");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);
    }
    assert!(command.status().unwrap().success());
    let (pid, args) = chat_fixture_result(&executable).await;
    assert_process_exited(pid);
    assert_eq!(args, ["--channels", "t:short"]);
}

#[tokio::test]
async fn selected_profile_support_report_exposes_mode_without_name_path_or_arguments() {
    use stream_gui_rs::config::profiles::{ProfileDraft, ProfileMutation::*};
    let root = tempfile::tempdir().unwrap();
    let services = Services::new(root.path(), None).unwrap();
    let created = services
        .modify_profile(Create {
            profile: ProfileDraft {
                name: "Private living room name".into(),
                player: PlayerSettings {
                    mode: stream_gui_rs::streamlink::playback::PlayerMode::Custom,
                    executable: Some(helper().to_string_lossy().into_owned()),
                    arguments: vec!["--private-device-name".into()],
                },
                quality: None,
                low_latency: None,
            },
        })
        .await
        .unwrap();
    services
        .modify_profile(Select {
            id: Some(created.profiles[0].id.clone()),
        })
        .await
        .unwrap();
    let report = services.support_report().await.text;
    assert!(report.contains("custom"));
    for forbidden in [
        "Private living room name",
        "--private-device-name",
        &created.profiles[0].id,
        helper().to_str().unwrap(),
    ] {
        assert!(!report.contains(forbidden));
    }
    services.shutdown().await.unwrap();
}
