#[path = "src/config/client_id_value.rs"]
mod client_id_value;
#[path = "src/build_info/commit.rs"]
mod commit;

fn configure_build_commit() {
    println!("cargo:rerun-if-env-changed=STREAM_GUI_RS_COMMIT");
    println!("cargo:rerun-if-changed=src/build_info/commit.rs");
    // An explicit build input wins, including invalid/missing-Unicode values.
    // CI supplies the exact checkout SHA; local builds may consult Git once here.
    let supplied = std::env::var_os("STREAM_GUI_RS_COMMIT");
    let local = supplied.is_none().then(local_commit).flatten();
    let value = supplied
        .as_ref()
        .and_then(|v| v.to_str())
        .or(local.as_deref());
    println!(
        "cargo:rustc-env=STREAM_GUI_RS_BUILD_COMMIT={}",
        commit::short_commit(value)
    );
}

fn local_commit() -> Option<String> {
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_dir().ok())?;
    let root = manifest.parent()?;
    let dot_git = root.join(".git");
    // A source archive inside someone else's repository must not inherit its SHA.
    if !dot_git.exists() {
        println!("cargo:rerun-if-changed={}", dot_git.display());
        return None;
    }
    if dot_git.is_file() {
        println!("cargo:rerun-if-changed={}", dot_git.display());
    }
    let git = |args: &[&str]| -> Option<String> {
        let output = std::process::Command::new("git")
            .current_dir(root)
            .args(args)
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8(output.stdout).ok())?
            .map(|value| value.trim().to_owned())
    };
    // Resolve through Git for linked worktrees and packed refs as well as normal
    // checkouts. New commits must invalidate Cargo's cached build-script output.
    // Watch the refs directory too: the first commit on a packed branch creates
    // a loose ref without changing HEAD or the packed-refs file.
    for name in ["HEAD", "packed-refs", "refs"] {
        if let Some(path) = git(&["rev-parse", "--git-path", name]) {
            let path = root.join(path);
            if path.exists() {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
    git(&["rev-parse", "--verify", "HEAD^{commit}"])
}

fn configure_twitch_client_id() {
    use client_id_value::{BUILD_ENV, validate};
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/config/client_id_value.rs");
    println!("cargo:rerun-if-env-changed={BUILD_ENV}");

    let input = std::env::var_os(BUILD_ENV);
    let embedded = input.as_deref().map(|value| {
        validate(value).unwrap_or_else(|reason| panic!("{BUILD_ENV} {reason}. Supply the project's public Twitch client ID, never a client secret."))
    });
    // Cargo's release profile stays "release" even with debug assertions.
    // Packaged debug commands explicitly enable custom-protocol.
    let profile = std::env::var("PROFILE").expect("Cargo build profile is missing");
    let distribution =
        profile != "debug" || std::env::var_os("CARGO_FEATURE_CUSTOM_PROTOCOL").is_some();
    assert!(
        !distribution || embedded.is_some(),
        "Release/distribution builds require TWITCH_CLIENT_ID_BUILD. Set it to the project's public Twitch client ID and rebuild. TWITCH_CLIENT_ID is only a runtime developer override and cannot satisfy this build requirement. No client secret is needed."
    );

    // Always overwrite the internal compiler value, including unconfigured dev
    // builds, so removing the build input cannot reuse a previously embedded ID.
    println!(
        "cargo:rustc-env=STREAM_GUI_RS_EMBEDDED_TWITCH_CLIENT_ID={}",
        embedded.unwrap_or_default()
    );
}

fn main() {
    assert!(
        std::env::var_os("CARGO_FEATURE_NOTIFICATION_ACCEPTANCE").is_none()
            || std::env::var("PROFILE").as_deref() == Ok("debug"),
        "notification-acceptance is only available in debug/test builds"
    );
    configure_twitch_client_id();
    configure_build_commit();
    #[cfg(feature = "desktop")]
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            #[cfg(feature = "notification-acceptance")]
            "dev_notification_test",
            "desktop_status",
            "pause_monitor",
            "resume_monitor",
            "request_notification_permission",
            "acknowledge_desktop_action",
            "quit_application",
            "backend_diagnostics",
            "open_repository",
            "update_status",
            "check_updates",
            "refresh_updates",
            "open_update_release",
            "modify_player_profile",
            "modify_discovery",
            "save_shortcuts",
            "discover_chatterino",
            "open_browser_chat",
            "show_about",
            "app_info",
            "support_report",
            "streamlink_probe",
            "streamlink_launch",
            "streamlink_stop",
            "streamlink_sessions",
            "streamlink_restart",
            "playback_settings",
            "channel_settings",
            "open_channel_chat",
            "save_channel_settings",
            "save_playback_settings",
            "discover_players",
            "auth_status",
            "auth_login",
            "auth_open_verification",
            "auth_validate",
            "auth_refresh",
            "auth_logout",
            "auth_cancel",
            "auth_account",
            "list_followed_streams",
            "list_followed_channels",
            "list_streams",
            "save_discovery_language",
            "list_categories",
            "list_category_streams",
            "search_channels",
            "search_categories",
            "get_channel",
            "lookup_channel",
        ]),
    ))
    .expect("Tauri build configuration is invalid");
}
