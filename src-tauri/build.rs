fn main() {
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
