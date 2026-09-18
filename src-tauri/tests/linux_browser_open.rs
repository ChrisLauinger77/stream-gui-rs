#![cfg(all(target_os = "linux", feature = "desktop", feature = "test-support"))]

#[path = "../src/desktop/browser.rs"]
mod browser;

use std::{fs, path::Path, process::Command, time::Duration};

// Run the actual adapter in a separate process: GIO caches desktop associations,
// and ordinary tests must never use or change the user's browser configuration.
#[test]
fn desktop_browser_dispatch_uses_the_default_handler_and_reaps_its_launcher() {
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
    let result = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "isolated_browser_dispatch", "--nocapture"])
        .env("STREAM_GUI_BROWSER_TEST_MARKER", &marker)
        .env("XDG_CONFIG_HOME", directory.path())
        .env("XDG_DATA_HOME", directory.path())
        .env("XDG_DATA_DIRS", directory.path())
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
    assert_eq!(uri, "https://www.twitch.tv/popout/synthetic/chat");
}

#[test]
fn isolated_browser_dispatch() {
    let Some(marker) = std::env::var_os("STREAM_GUI_BROWSER_TEST_MARKER") else {
        return;
    };
    browser::open("https://www.twitch.tv/popout/synthetic/chat").unwrap();
    for _ in 0..150 {
        if let Ok(bytes) = fs::read(&marker) {
            if let Ok((pid, _)) = serde_json::from_slice::<(u32, String)>(&bytes) {
                if !Path::new(&format!("/proc/{pid}")).exists() {
                    return;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("The default browser helper was not launched and reaped");
}
