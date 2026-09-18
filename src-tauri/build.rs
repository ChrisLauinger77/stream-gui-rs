#[path = "src/config/client_id_value.rs"]
mod client_id_value;

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
    // Tauri build enables custom-protocol, including --debug/--no-bundle.
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
    configure_twitch_client_id();
    #[cfg(feature = "desktop")]
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "backend_diagnostics",
            "streamlink_probe",
            "streamlink_launch",
            "streamlink_stop",
            "streamlink_sessions",
            "streamlink_restart",
            "playback_settings",
            "channel_settings",
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
            "list_categories",
            "list_category_streams",
            "search_channels",
            "search_categories",
            "get_channel",
        ]),
    ))
    .expect("Tauri build configuration is invalid");
}
