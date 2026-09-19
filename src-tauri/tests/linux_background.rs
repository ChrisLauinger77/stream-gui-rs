#![cfg(all(target_os = "linux", feature = "desktop", feature = "test-support"))]
use std::{
    process::Command,
    time::{Duration, Instant},
};

#[test]
#[ignore = "requires a graphical Linux session; synthetic isolated services only"]
fn close_background_restore_reload_and_quit_preserve_then_reap_playback() {
    for action in ["close", "quit"] {
        let directory = tempfile::tempdir().unwrap();
        let marker = directory.path().join("result");
        let mut child = Command::new("dbus-run-session")
            .args(["--", env!("CARGO_BIN_EXE_background-smoke")])
            .args([env!("CARGO_BIN_EXE_fake-streamlink"), action])
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
            if start.elapsed() > Duration::from_secs(30) {
                let _ = child.kill();
                let _ = child.wait();
                panic!("native background smoke timed out");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
