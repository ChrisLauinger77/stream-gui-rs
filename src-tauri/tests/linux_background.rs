#![cfg(all(target_os = "linux", feature = "desktop", feature = "test-support"))]
use std::{
    process::Command,
    time::{Duration, Instant},
};

#[test]
#[ignore = "requires a graphical Linux session; synthetic isolated services only"]
fn close_background_restore_reload_and_quit_preserve_then_reap_playback() {
    run_scenarios(&["close", "quit"]);
}

#[test]
#[ignore = "requires a Wayland graphical session; synthetic isolated services only"]
fn native_wayland_titlebar_is_compact_and_preserves_window_actions() {
    run_scenarios(&["titlebar"]);
}

#[test]
#[ignore = "requires a graphical Linux session and the frontend dev server; synthetic isolated services only"]
fn phase_six_about_support_and_text_scale_use_the_native_webview() {
    run_scenarios(&["phase6"]);
}

#[test]
#[ignore = "requires a graphical Linux session and the frontend dev server; synthetic isolated services only"]
fn phase_seven_profiles_chat_and_updates_use_the_native_webview() {
    run_scenarios(&["phase7"]);
}

#[test]
#[ignore = "requires a graphical Linux session and frontend dev server; isolated Phase 8 fixtures"]
fn phase_eight_navigation_preferences_and_native_links() {
    run_scenarios(&["phase8"]);
}

#[test]
#[ignore = "requires a graphical Linux session and frontend dev server; synthetic isolated services only"]
fn localized_settings_switch_and_scale_in_native_webview() {
    run_scenarios(&["localization"]);
}

fn run_scenarios(actions: &[&str]) {
    for action in actions {
        let directory = tempfile::tempdir().unwrap();
        let marker = directory.path().join("result");
        let mut command = Command::new("dbus-run-session");
        if *action == "titlebar" {
            command
                .env("GDK_BACKEND", "wayland")
                .env("GTK_THEME", "Adwaita");
        }
        if *action == "localization" {
            command.env("LANG", "de_DE.UTF-8").env_remove("LC_ALL");
        }
        let mut child = command
            .args(["--", env!("CARGO_BIN_EXE_background-smoke")])
            .args([env!("CARGO_BIN_EXE_fake-streamlink"), *action])
            .arg(&marker)
            .env("XDG_CONFIG_HOME", directory.path().join("config"))
            .env("XDG_DATA_HOME", directory.path().join("data"))
            .env("XDG_CACHE_HOME", directory.path().join("cache"))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success());
                assert_eq!(
                    std::fs::read_to_string(&marker).unwrap(),
                    "passed",
                    "native scenario did not reach its final assertion"
                );
                let pids: Vec<u32> = serde_json::from_str(
                    &std::fs::read_to_string(marker.with_extension("pids")).unwrap(),
                )
                .unwrap();
                for pid in pids {
                    assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
                }
                break;
            }
            if start.elapsed() > Duration::from_secs(60) {
                let _ = child.kill();
                let _ = child.wait();
                panic!("native background smoke timed out");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
