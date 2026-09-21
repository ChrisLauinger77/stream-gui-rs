//! Deterministic native fixture, independent of host Flatpak and installed apps.
use std::{io::Write, path::Path, time::Duration};

pub fn run(args: &[String], executable: &Path) -> bool {
    if args
        .first()
        .is_some_and(|s| s == "--flatpak-missing-parent")
    {
        assert_eq!(
            stream_gui_rs::chatterino::resolve_test_candidates(
                None,
                None,
                Some(Path::new(&args[1]))
            )
            .unwrap_err()
            .code,
            stream_gui_rs::domain::ErrorCode::ChatterinoNotFound
        );
        return true;
    }
    if args.first().is_some_and(|s| s == "--flatpak-parent") {
        let source = stream_gui_rs::chatterino::resolve_test_candidates(
            None,
            None,
            Some(Path::new(&args[1])),
        )
        .unwrap();
        let chat = stream_gui_rs::chatterino::Chatterino::default();
        chat.open_source(&source, &args[2]).unwrap();
        if args[2] == "short" {
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            while chat.active_launchers() != 0 && std::time::Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(10));
            }
            assert_eq!(chat.active_launchers(), 0);
        }
        // Returning in hold mode exits the parent while independent chat is alive.
        return true;
    }
    if !args.first().is_some_and(|s| s == "info" || s == "run") {
        return false;
    }
    assert!(std::env::var_os("STREAM_GUI_RS_SYNTHETIC_TOKEN").is_none());
    assert!(std::env::var_os("TWITCH_CLIENT_ID").is_none());
    assert!(std::env::var_os("TWITCH_CLIENT_ID_BUILD").is_none());
    assert!(args[1] == "--user" || args[1] == "--system");
    assert_eq!(args[2], "app/com.chatterino.chatterino");
    if args[0] == "info" {
        assert_eq!(args.len(), 3);
        let mut record = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(executable.with_extension("probes"))
            .unwrap();
        writeln!(
            record,
            "{}",
            serde_json::to_string(&(std::process::id(), args)).unwrap()
        )
        .unwrap();
        let installed =
            std::fs::read_to_string(executable.with_extension("installed")).unwrap_or_default();
        if installed == "hang" {
            std::thread::sleep(Duration::from_secs(30));
        }
        // Probe output is discarded, rather than accumulated or parsed.
        println!("{}", "x".repeat(256 * 1024));
        eprintln!("{}", "y".repeat(256 * 1024));
        std::process::exit(
            if installed == "both" || installed == args[1].trim_start_matches("--") {
                0
            } else {
                1
            },
        );
    }
    assert_eq!(args.len(), 5);
    assert_eq!(args[3], "--channels");
    assert!(args[4].starts_with("t:"));
    std::fs::write(
        executable.with_extension("chat.json"),
        serde_json::to_vec(&(std::process::id(), args)).unwrap(),
    )
    .unwrap();
    if args[4] == "t:hold" {
        let deadline = std::time::Instant::now() + Duration::from_secs(15);
        while !executable.with_extension("release").exists() && std::time::Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    true
}
