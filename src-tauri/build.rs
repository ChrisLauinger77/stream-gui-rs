fn main() {
    #[cfg(feature = "desktop")]
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "backend_diagnostics",
            "streamlink_probe",
            "streamlink_launch",
            "streamlink_stop",
            "streamlink_sessions",
            "auth_status",
            "auth_login",
            "auth_open_verification",
            "auth_validate",
            "auth_refresh",
            "auth_logout",
            "auth_cancel",
            "auth_account",
        ]),
    ))
    .expect("Tauri build configuration is invalid");
}
