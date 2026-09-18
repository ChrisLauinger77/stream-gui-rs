use crate::{
    diagnostics::BackendDiagnostics,
    domain::{
        AppError, ErrorCode, LaunchRequest, ProbeRequest, Result, StopRequest, services::Services,
    },
    streamlink::{ProbeResult, SessionSnapshot},
    twitch::AuthStatus,
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn backend_diagnostics(services: State<'_, Arc<Services>>) -> BackendDiagnostics {
    services.diagnostics()
}

#[tauri::command]
pub async fn streamlink_probe(
    services: State<'_, Arc<Services>>,
    request: ProbeRequest,
) -> Result<ProbeResult> {
    services.probe(request.custom_path).await
}

#[tauri::command]
pub async fn streamlink_launch(
    services: State<'_, Arc<Services>>,
    request: LaunchRequest,
) -> Result<SessionSnapshot> {
    services.launch(request).await
}

#[tauri::command]
pub async fn streamlink_stop(
    services: State<'_, Arc<Services>>,
    request: StopRequest,
) -> Result<SessionSnapshot> {
    services.sessions.stop(&request.session_id).await
}

#[tauri::command]
pub async fn streamlink_sessions(
    services: State<'_, Arc<Services>>,
) -> Result<Vec<SessionSnapshot>> {
    Ok(services.sessions.sessions().await)
}

#[tauri::command]
pub async fn auth_status(services: State<'_, Arc<Services>>) -> Result<AuthStatus> {
    Ok(services.auth.status().await)
}

#[tauri::command]
pub async fn auth_login(services: State<'_, Arc<Services>>) -> Result<AuthStatus> {
    services.auth.login().await
}

#[tauri::command]
pub async fn auth_validate(services: State<'_, Arc<Services>>) -> Result<AuthStatus> {
    services.auth.validate().await
}

#[tauri::command]
pub async fn auth_refresh(services: State<'_, Arc<Services>>) -> Result<AuthStatus> {
    services.auth.refresh().await
}

#[tauri::command]
pub async fn auth_logout(services: State<'_, Arc<Services>>) -> Result<AuthStatus> {
    services.auth.logout().await
}

#[tauri::command]
pub async fn auth_open_verification(services: State<'_, Arc<Services>>) -> Result<()> {
    // No URL or executable can be supplied by the webview.
    let uri = services.auth.verification_uri().await?;
    tauri::async_runtime::spawn_blocking(move || webbrowser::open(&uri))
        .await
        .map_err(|_| AppError::new(ErrorCode::Internal, "Browser opener failed."))?
        .map_err(|_| {
            AppError::new(
                ErrorCode::Internal,
                "Could not open the system browser. Use the displayed verification URL.",
            )
        })
}
