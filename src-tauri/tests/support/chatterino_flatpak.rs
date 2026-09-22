use super::*;
use stream_gui_rs::chatterino::{Chatterino, LaunchSource, resolve_test_candidates as discover};

fn probes(executable: &Path) -> Vec<(u32, Vec<String>)> {
    std::fs::read_to_string(executable.with_extension("probes"))
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
async fn reaped(chat: &Chatterino) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while chat.active_launchers() != 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

#[test]
fn missing_flatpak_or_app_is_normal_absence() {
    assert_eq!(
        discover(None, None, None).unwrap_err().code,
        ErrorCode::ChatterinoNotFound
    );
    let root = helper_directory();
    let executable = renamed_helper(root.path(), "flatpak");
    assert_eq!(
        discover(None, None, Some(&executable)).unwrap_err().code,
        ErrorCode::ChatterinoNotFound
    );
    let attempts = probes(&executable);
    assert_eq!(attempts.len(), 2);
    assert_eq!(
        attempts[0].1,
        ["info", "--user", "app/com.chatterino.chatterino"]
    );
    assert_eq!(
        attempts[1].1,
        ["info", "--system", "app/com.chatterino.chatterino"]
    );
    for (pid, _) in attempts {
        assert_process_exited(pid);
    }
    assert!(!executable.with_extension("chat.json").exists());
}

#[test]
fn native_discovery_and_explicit_override_precede_flatpak_without_probing() {
    let root = helper_directory();
    let native = renamed_helper(root.path(), "native chatterino");
    let explicit = renamed_helper(root.path(), "explicit chatterino");
    let flatpak = renamed_helper(root.path(), "flatpak");
    std::fs::write(flatpak.with_extension("installed"), "both").unwrap();
    assert_eq!(
        discover(None, Some(&native), Some(&flatpak)).unwrap(),
        LaunchSource::Native(native.canonicalize().unwrap())
    );
    let source = discover(explicit.to_str(), Some(&native), Some(&flatpak)).unwrap();
    assert_eq!(
        source,
        LaunchSource::Native(explicit.canonicalize().unwrap())
    );
    assert_eq!(
        source.description(),
        explicit.canonicalize().unwrap().to_string_lossy()
    );
    assert_eq!(
        discover(Some("relative"), Some(&native), Some(&flatpak))
            .unwrap_err()
            .code,
        ErrorCode::ChatterinoNotFound
    );
    std::fs::remove_file(&explicit).unwrap();
    assert_eq!(
        discover(explicit.to_str(), Some(&native), Some(&flatpak))
            .unwrap_err()
            .code,
        ErrorCode::ChatterinoNotFound
    );
    assert!(probes(&flatpak).is_empty());
}

#[tokio::test]
async fn user_system_and_duplicate_installations_launch_the_selected_fixed_app() {
    for (installed, scope, count) in [
        ("user", "--user", 1),
        ("system", "--system", 2),
        ("both", "--user", 1),
    ] {
        let root = helper_directory();
        let executable = renamed_helper(root.path(), "flatpak 空 格");
        std::fs::write(executable.with_extension("installed"), installed).unwrap();
        let source = discover(None, None, Some(&executable)).unwrap();
        assert!(matches!(source, LaunchSource::Flatpak { .. }));
        assert_eq!(source.description(), "Installed");
        assert_eq!(probes(&executable).len(), count);
        let chat = Chatterino::default();
        for login in [
            "",
            "-flag",
            "name;other",
            "a b",
            "name/evil",
            "oauth:synthetic",
            "ユーザー",
            "a\narg",
        ] {
            assert_eq!(
                chat.open_source(&source, login).unwrap_err().code,
                ErrorCode::InvalidInput
            );
        }
        assert_eq!(chat.active_launchers(), 0);
        assert!(!executable.with_extension("chat.json").exists());
        chat.open_source(&source, "ExAmPlE_1").unwrap();
        let (pid, args) = chat_fixture_result(&executable).await;
        assert_eq!(
            args,
            [
                "run",
                scope,
                "app/com.chatterino.chatterino",
                "--channels",
                "t:example_1"
            ]
        );
        reaped(&chat).await;
        assert_process_exited(pid);
        for (pid, _) in probes(&executable) {
            assert_process_exited(pid);
        }
    }
}

#[test]
fn flatpak_probe_timeout_reaps_and_never_uses_an_implicit_shell() {
    use std::os::unix::fs::PermissionsExt;
    let root = helper_directory();
    let invalid = root.path().join("invalid-flatpak");
    let marker = root.path().join("shell-was-used");
    std::fs::write(&invalid, format!("touch '{}'\n", marker.display())).unwrap();
    std::fs::set_permissions(&invalid, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        discover(None, None, Some(&invalid)).unwrap_err().code,
        ErrorCode::ChatterinoNotFound
    );
    assert!(!marker.exists());

    let replaced = renamed_helper(root.path(), "replaced-flatpak");
    std::fs::write(replaced.with_extension("installed"), "user").unwrap();
    let source = discover(None, None, Some(&replaced)).unwrap();
    // The helper may be a hard link shared by other tests; replace only this name.
    std::fs::remove_file(&replaced).unwrap();
    std::fs::write(&replaced, format!("touch '{}'\n", marker.display())).unwrap();
    std::fs::set_permissions(&replaced, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        Chatterino::default()
            .open_source(&source, "short")
            .unwrap_err()
            .code,
        ErrorCode::ChatLaunch
    );
    assert!(!marker.exists());

    // A separate parent keeps the intentional timeout independent of concurrent
    // discovery tests, and exercises the actual production two-second deadline.
    let executable = renamed_helper(root.path(), "flatpak-timeout");
    std::fs::write(executable.with_extension("installed"), "hang").unwrap();
    let started = std::time::Instant::now();
    assert!(
        std::process::Command::new(helper())
            .arg("--flatpak-missing-parent")
            .arg(&executable)
            .status()
            .unwrap()
            .success()
    );
    assert!(started.elapsed() < Duration::from_secs(5));
    let attempts = probes(&executable);
    assert_eq!(attempts.len(), 1);
    assert_process_exited(attempts[0].0);
}

#[tokio::test]
async fn flatpak_capacity_stays_typed_and_reaping_releases_all_slots_after_stop_and_shutdown() {
    let root = helper_directory();
    let executable = renamed_helper(root.path(), "flatpak-bounded");
    std::fs::write(executable.with_extension("installed"), "user").unwrap();
    let source = discover(None, None, Some(&executable)).unwrap();
    let chat = Chatterino::default();
    for _ in 0..16 {
        chat.open_source(&source, "hold").unwrap();
    }
    assert_eq!(chat.active_launchers(), 16);
    assert_eq!(
        chat.open_source(&source, "hold").unwrap_err().code,
        ErrorCode::ChatterinoCapacity
    );
    let (pid, _) = chat_fixture_result(&executable).await;
    let services = Services::new(root.path(), None).unwrap();
    let session = services
        .sessions
        .launch_spec(production_spec("hold"))
        .await
        .unwrap();
    services.sessions.stop(&session.id).await.unwrap();
    services.shutdown().await.unwrap();
    // SAFETY: query only the known fixture process/session, without a signal.
    assert_eq!(unsafe { libc::kill(pid as i32, 0) }, 0);
    assert_eq!(unsafe { libc::getsid(pid as i32) }, pid as i32);
    assert_eq!(chat.active_launchers(), 16);
    std::fs::write(executable.with_extension("release"), "done").unwrap();
    reaped(&chat).await;
    assert_process_exited(pid);
    chat.open_source(&source, "short").unwrap();
    reaped(&chat).await;
}

#[tokio::test]
async fn flatpak_children_receive_no_credentials_and_survive_parent_exit() {
    for login in ["short", "hold"] {
        let root = helper_directory();
        let executable = renamed_helper(root.path(), "flatpak-parent");
        std::fs::write(executable.with_extension("installed"), "system").unwrap();
        assert!(
            std::process::Command::new(helper())
                .arg("--flatpak-parent")
                .arg(&executable)
                .arg(login)
                .env("STREAM_GUI_RS_SYNTHETIC_TOKEN", "oauth:synthetic-test-only")
                .env("TWITCH_CLIENT_ID", "synthetic-public-id")
                .env("TWITCH_CLIENT_ID_BUILD", "synthetic-public-id")
                .status()
                .unwrap()
                .success()
        );
        let (pid, _) = chat_fixture_result(&executable).await;
        if login == "short" {
            assert_process_exited(pid);
        } else {
            // SAFETY: query only the fixture; parent exit must leave chat alive.
            assert_eq!(unsafe { libc::kill(pid as i32, 0) }, 0);
            assert_eq!(unsafe { libc::getsid(pid as i32) }, pid as i32);
            std::fs::write(executable.with_extension("release"), "done").unwrap();
            tokio::time::timeout(Duration::from_secs(3), async {
                // SAFETY: query only the orphaned fixture until its OS reaper runs.
                while unsafe { libc::kill(pid as i32, 0) } == 0 {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            assert_process_exited(pid);
        }
    }
}
