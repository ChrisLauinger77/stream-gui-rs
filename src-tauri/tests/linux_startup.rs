#![cfg(all(target_os = "linux", feature = "desktop"))]

use std::{fs, process::Command};

#[test]
#[ignore = "requires a Linux graphical session; uses only isolated synthetic configuration"]
fn malformed_settings_exit_cleanly_without_overwriting_or_panicking() {
    for contents in ["{invalid-json", r#"{"version":999}"#] {
        let directory = tempfile::tempdir().unwrap();
        let config = directory.path().join("io.github.stream-gui-rs");
        fs::create_dir(&config).unwrap();
        let path = config.join("settings.json");
        fs::write(&path, contents).unwrap();
        let result = Command::new("dbus-run-session")
            .args(["--", env!("CARGO_BIN_EXE_stream-gui-rs")])
            .env("XDG_CONFIG_HOME", directory.path())
            .env("TWITCH_CLIENT_ID", "syntheticConfigValidation123")
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(error.contains("Could not initialize Stream GUI RS"));
        assert!(!error.contains("panicked"));
        assert_eq!(fs::read_to_string(path).unwrap(), contents);
    }
}
