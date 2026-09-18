use crate::{
    config::Settings,
    streamlink::{
        discovery::PlayerDiscovery,
        playback::{PlaybackRequest, RestartRequest},
    },
};
use crate::{
    diagnostics::BackendDiagnostics,
    domain::{AppError, ErrorCode, ProbeRequest, Result, StopRequest, services::Services},
    streamlink::{ProbeResult, SessionSnapshot},
    twitch::AuthStatus,
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn playback_settings(services: State<'_, Arc<Services>>) -> Settings {
    services.settings.snapshot()
}

#[tauri::command]
pub async fn save_playback_settings(
    services: State<'_, Arc<Services>>,
    request: Settings,
) -> Result<Settings> {
    services.save_settings(request).await
}

#[tauri::command]
pub fn discover_players(services: State<'_, Arc<Services>>) -> PlayerDiscovery {
    services.players()
}

#[tauri::command]
pub async fn streamlink_restart(
    services: State<'_, Arc<Services>>,
    request: RestartRequest,
) -> Result<SessionSnapshot> {
    services.restart_playback(request).await
}

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
    request: PlaybackRequest,
) -> Result<SessionSnapshot> {
    services.play(request).await
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
    services.helix.clear_cache();
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

#[tauri::command]
pub async fn auth_cancel(services: State<'_, Arc<Services>>) -> Result<AuthStatus> {
    services.auth.cancel().await
}

#[tauri::command]
pub async fn auth_account(
    services: State<'_, Arc<Services>>,
) -> Result<crate::helix::models::Account> {
    services
        .helix
        .account(&tokio_util::sync::CancellationToken::new())
        .await
}

use crate::helix::browse::*;

#[tauri::command]
pub async fn list_followed_streams(
    services: State<'_, Arc<Services>>,
    request: BrowseRequest,
) -> Result<PagedResult<StreamSummary>> {
    let _permit = services.browse_permit()?;
    services
        .helix
        .browse_followed_streams(request, &tokio_util::sync::CancellationToken::new())
        .await
}

#[tauri::command]
pub async fn list_followed_channels(
    services: State<'_, Arc<Services>>,
    request: BrowseRequest,
) -> Result<PagedResult<ChannelSummary>> {
    let _permit = services.browse_permit()?;
    services
        .helix
        .browse_followed_channels(request, &tokio_util::sync::CancellationToken::new())
        .await
}

#[tauri::command]
pub async fn list_streams(
    services: State<'_, Arc<Services>>,
    request: BrowseRequest,
) -> Result<PagedResult<StreamSummary>> {
    let _permit = services.browse_permit()?;
    services
        .helix
        .browse_streams(request, &tokio_util::sync::CancellationToken::new())
        .await
}

#[tauri::command]
pub async fn list_categories(
    services: State<'_, Arc<Services>>,
    request: BrowseRequest,
) -> Result<PagedResult<CategorySummary>> {
    let _permit = services.browse_permit()?;
    services
        .helix
        .browse_categories(request, &tokio_util::sync::CancellationToken::new())
        .await
}

#[tauri::command]
pub async fn list_category_streams(
    services: State<'_, Arc<Services>>,
    request: EntityRequest,
) -> Result<CategoryDetails> {
    let _permit = services.browse_permit()?;
    services
        .helix
        .browse_category(request, &tokio_util::sync::CancellationToken::new())
        .await
}

#[tauri::command]
pub async fn search_channels(
    services: State<'_, Arc<Services>>,
    request: SearchRequest,
) -> Result<PagedResult<ChannelSummary>> {
    let _permit = services.browse_permit()?;
    services
        .helix
        .browse_search_channels(request, &tokio_util::sync::CancellationToken::new())
        .await
}

#[tauri::command]
pub async fn search_categories(
    services: State<'_, Arc<Services>>,
    request: SearchRequest,
) -> Result<PagedResult<CategorySummary>> {
    let _permit = services.browse_permit()?;
    services
        .helix
        .browse_search_categories(request, &tokio_util::sync::CancellationToken::new())
        .await
}

#[tauri::command]
pub async fn get_channel(
    services: State<'_, Arc<Services>>,
    request: EntityRequest,
) -> Result<ChannelDetails> {
    let _permit = services.browse_permit()?;
    services
        .helix
        .browse_channel(request, &tokio_util::sync::CancellationToken::new())
        .await
}

#[tauri::command]
pub fn channel_settings(
    services: State<'_, Arc<Services>>,
    request: crate::config::ChannelSettingsRequest,
) -> Result<crate::config::ChannelSettings> {
    services.settings.channel(&request.broadcaster_id)
}

#[tauri::command]
pub async fn save_channel_settings(
    services: State<'_, Arc<Services>>,
    request: crate::config::SaveChannelSettingsRequest,
) -> Result<crate::config::ChannelSettings> {
    services.save_channel_settings(request).await
}

#[tauri::command]
pub async fn open_channel_chat(
    services: State<'_, Arc<Services>>,
    request: crate::domain::chat::ChatRequest,
) -> Result<()> {
    services.open_chat(request).await
}
