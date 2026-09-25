#![cfg(all(target_os = "linux", feature = "desktop", feature = "test-support"))]

#[path = "../src/desktop/browser.rs"]
mod browser;

use std::{fs, path::Path, process::Command, time::Duration};

// Run the actual adapter in a separate process: GIO caches desktop associations,
// and ordinary tests must never use or change the user's browser configuration.
#[test]
fn desktop_browser_dispatch_uses_the_default_handler_and_reaps_its_launcher() {
    for destination in [
        "https://www.twitch.tv/popout/synthetic/chat",
        env!("CARGO_PKG_REPOSITORY"),
    ] {
        check_destination(destination);
    }
}
fn check_destination(destination: &str) {
    let directory = tempfile::tempdir().unwrap();
    let applications = directory.path().join("applications");
    fs::create_dir(&applications).unwrap();
    let marker = directory.path().join("opened.json");
    fs::write(
        applications.join("test-browser.desktop"),
        format!(
            "[Desktop Entry]\nType=Application\nName=Test browser\nExec=\"{}\" --browser-probe \"{}\" %u\nMimeType=x-scheme-handler/https;\n",
            env!("CARGO_BIN_EXE_fake-streamlink"), marker.display()
        ),
    ).unwrap();
    fs::write(
        directory.path().join("mimeapps.list"),
        "[Default Applications]\nx-scheme-handler/https=test-browser.desktop;\n",
    )
    .unwrap();
    check_handler(directory.path(), &marker, destination);
}

#[test]
#[ignore = "requires a built DEB at STREAM_GUI_TEST_DEB; never launches the real app"]
fn packaged_protocol_dispatch_passes_the_exact_uri_to_the_launcher() {
    let package = std::env::var_os("STREAM_GUI_TEST_DEB").expect("set STREAM_GUI_TEST_DEB");
    let extracted = tempfile::tempdir().unwrap();
    assert!(
        Command::new("dpkg-deb")
            .arg("--extract")
            .arg(package)
            .arg(extracted.path())
            .status()
            .unwrap()
            .success()
    );
    let entry = fs::read_to_string(
        extracted
            .path()
            .join("usr/share/applications/Stream GUI RS.desktop"),
    )
    .unwrap();
    assert!(
        entry.lines().any(
            |line| line.strip_prefix("MimeType=").is_some_and(|types| types
                .split(';')
                .any(|value| value == "x-scheme-handler/stream-gui-rs"))
        )
    );
    for destination in [
        "stream-gui-rs://channel/example_login",
        "stream-gui-rs://team/example-team",
        "stream-gui-rs://channel/example_login?unexpected=1",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let applications = directory.path().join("applications");
        fs::create_dir(&applications).unwrap();
        let marker = directory.path().join("opened.json");
        // Substitute only the executable; exercise the packaged argument fields
        // through GIO without starting the app or using the user's associations.
        let probe = entry.replacen(
            "Exec=stream-gui-rs",
            &format!(
                "Exec=\"{}\" --browser-probe \"{}\"",
                env!("CARGO_BIN_EXE_fake-streamlink"),
                marker.display()
            ),
            1,
        );
        assert_ne!(entry, probe);
        fs::write(applications.join("test-browser.desktop"), probe).unwrap();
        fs::write(
            directory.path().join("mimeapps.list"),
            "[Default Applications]\nx-scheme-handler/stream-gui-rs=test-browser.desktop;\n",
        )
        .unwrap();
        check_handler(directory.path(), &marker, destination);
    }
}

fn check_handler(directory: &Path, marker: &Path, destination: &str) {
    let result = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "isolated_browser_dispatch", "--nocapture"])
        .env("STREAM_GUI_BROWSER_TEST_MARKER", marker)
        .env("STREAM_GUI_BROWSER_TEST_DESTINATION", destination)
        .env("XDG_CONFIG_HOME", directory)
        .env("XDG_DATA_HOME", directory)
        .env("XDG_DATA_DIRS", directory)
        .env("XDG_CURRENT_DESKTOP", "")
        .env("GIO_USE_VFS", "local")
        .env(
            "DBUS_SESSION_BUS_ADDRESS",
            "unix:path=/nonexistent-stream-gui-test-bus",
        )
        .env_remove("BROWSER")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let (_, uri): (u32, String) = serde_json::from_slice(&fs::read(marker).unwrap()).unwrap();
    assert_eq!(uri, destination);
}

#[test]
fn isolated_browser_dispatch() {
    let Some(marker) = std::env::var_os("STREAM_GUI_BROWSER_TEST_MARKER") else {
        return;
    };
    browser::open(&std::env::var("STREAM_GUI_BROWSER_TEST_DESTINATION").unwrap()).unwrap();
    for _ in 0..150 {
        if let Ok(bytes) = fs::read(&marker)
            && let Ok((pid, _)) = serde_json::from_slice::<(u32, String)>(&bytes)
            && !Path::new(&format!("/proc/{pid}")).exists()
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("The default browser helper was not launched and reaped");
}
