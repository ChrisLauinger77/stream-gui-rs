use crate::{commands, domain::services::Services};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tauri::Manager;
#[cfg(target_os = "macos")]
mod about;
pub(crate) mod browser;
mod notifications;
#[cfg(target_os = "linux")]
mod titlebar;
mod tray;

#[derive(Default)]
struct Lifecycle {
    stopping: AtomicBool,
    finished: AtomicBool,
    tray_created: AtomicBool,
    auth_stop: Mutex<Option<tokio::sync::watch::Sender<bool>>>,
}

struct NativeChatOpener;
impl crate::domain::chat::ChatOpener for NativeChatOpener {
    fn open(&self, target: &crate::domain::chat::ChatTarget) -> crate::domain::Result<()> {
        browser::open(target.url()).map_err(|_| crate::domain::chat::open_error())
    }
}

fn app_context() -> tauri::Context<tauri::Wry> {
    #[cfg(feature = "notification-acceptance")]
    let context =
        tauri::generate_context!(capabilities = ["capabilities/notification-acceptance.json"]);
    #[cfg(not(feature = "notification-acceptance"))]
    let context = tauri::generate_context!();
    context
}

fn build_app() -> tauri::Result<tauri::App> {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "linux")]
    let builder = builder.setup(|app| {
        // Configured windows exist only once Tauri enters setup. A decoration
        // adjustment must not prevent startup if a future GTK backend differs.
        let _ = titlebar::configure(app);
        Ok(())
    });
    #[cfg(target_os = "macos")]
    let builder = builder.menu(about::menu).on_menu_event(|app, event| {
        if event.id().as_ref() == about::MENU_ID {
            about::show(app);
        }
    });
    builder
        .invoke_handler(tauri::generate_handler![
            #[cfg(feature = "notification-acceptance")]
            commands::dev_notification_test,
            commands::desktop_status,
            commands::pause_monitor,
            commands::resume_monitor,
            commands::request_notification_permission,
            commands::acknowledge_desktop_action,
            commands::quit_application,
            commands::backend_diagnostics,
            commands::support_report,
            commands::streamlink_probe,
            commands::open_channel_chat,
            commands::channel_settings,
            commands::save_channel_settings,
            commands::playback_settings,
            commands::save_playback_settings,
            commands::discover_players,
            commands::streamlink_restart,
            commands::streamlink_launch,
            commands::streamlink_stop,
            commands::streamlink_sessions,
            commands::auth_status,
            commands::auth_login,
            commands::auth_open_verification,
            commands::auth_validate,
            commands::auth_refresh,
            commands::auth_logout,
            commands::auth_cancel,
            commands::auth_account,
            commands::list_followed_streams,
            commands::list_followed_channels,
            commands::list_streams,
            commands::save_discovery_language,
            commands::list_categories,
            commands::list_category_streams,
            commands::search_channels,
            commands::search_categories,
            commands::get_channel,
            commands::lookup_channel,
        ])
        .build(app_context())
}

pub fn run() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let app = build_app()?;
    // Tauri panics when a setup hook returns an error. Validate and install our
    // services before entering its event loop so malformed settings fail cleanly.
    let directory = app.path().app_config_dir()?;
    let client_id = crate::config::twitch_client_id::from_environment()?;
    let services = Arc::new(
        Services::new(&directory, Some(client_id))?.with_chat_opener(Arc::new(NativeChatOpener)),
    );
    install_services(&app, services);
    app.run(handle_event);
    Ok(())
}

fn install_services(app: &tauri::App, services: Arc<Services>) {
    let lifecycle = Arc::new(Lifecycle::default());
    app.manage(services.clone());
    app.manage(lifecycle.clone());
    let notifications = notifications::Notifications::new(app.handle().clone());
    services.monitor.set_sink(notifications.clone());
    app.manage(notifications);
    lifecycle.tray_created.store(
        tray::install(app.handle()).unwrap_or(false),
        Ordering::SeqCst,
    );
    tauri::async_runtime::spawn(async move {
        services.monitor.start(
            services.auth.clone(),
            services.helix.clone(),
            services.settings.clone(),
        );
        let stop = services.auth.start();
        let mut stored = lifecycle
            .auth_stop
            .lock()
            .expect("lifecycle mutex poisoned");
        if lifecycle.stopping.load(Ordering::SeqCst) {
            stop.send_replace(true);
        }
        *stored = Some(stop);
    });
}

fn handle_event(app: &tauri::AppHandle, event: tauri::RunEvent) {
    match event {
        tauri::RunEvent::ExitRequested { api, .. } => {
            if app
                .state::<Arc<Lifecycle>>()
                .finished
                .load(Ordering::SeqCst)
            {
                return;
            }
            api.prevent_exit();
            begin_shutdown(app);
        }
        tauri::RunEvent::WindowEvent {
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } => {
            api.prevent_close();
            if app
                .state::<Arc<Services>>()
                .settings
                .snapshot()
                .background
                .close_to_background
            {
                hide_window(app);
            } else {
                begin_shutdown(app);
            }
        }
        _ => {}
    }
}

// Explicit Quit always drains the same service owners, regardless of visibility.
pub(crate) fn begin_shutdown(app: &tauri::AppHandle) {
    let lifecycle = app.state::<Arc<Lifecycle>>().inner().clone();
    if lifecycle.stopping.swap(true, Ordering::SeqCst) {
        return;
    }
    if let Some(stop) = lifecycle
        .auth_stop
        .lock()
        .expect("lifecycle mutex poisoned")
        .as_ref()
    {
        stop.send_replace(true);
    }
    let services = app.state::<Arc<Services>>().inner().clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if services.shutdown().await.is_err() {
            eprintln!("Streamlink cleanup did not complete within its deadline.");
        }
        app.state::<Arc<notifications::Notifications>>()
            .shutdown()
            .await;
        lifecycle.finished.store(true, Ordering::SeqCst);
        app.exit(0);
    });
}

fn show_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        // GTK/Wayland can retain iconified state after deiconify alone. Remap a
        // minimized surface before presenting it, including notification restores.
        #[cfg(target_os = "linux")]
        if window.is_minimized().unwrap_or(false) {
            let _ = window.hide();
        }
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
fn hide_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if tray_available(app) {
            if window.hide().is_err() {
                let _ = window.minimize();
            }
        } else {
            let _ = window.minimize();
        }
    }
}
fn tray_available(app: &tauri::AppHandle) -> bool {
    app.state::<Arc<Lifecycle>>()
        .tray_created
        .load(Ordering::SeqCst)
        && app
            .state::<Arc<notifications::Notifications>>()
            .shared
            .state
            .lock()
            .expect("native notification state poisoned")
            .tray_host
}
pub(crate) fn status(app: &tauri::AppHandle) -> crate::domain::background::DesktopStatus {
    let services = app.state::<Arc<Services>>();
    let notifications = app.state::<Arc<notifications::Notifications>>();
    let mut state = notifications
        .shared
        .state
        .lock()
        .expect("native notification state poisoned");
    let expired = state.action.as_ref().is_some_and(|action| match action {
        #[cfg(feature = "notification-acceptance")]
        action if notifications::acceptance::is_test_action(action) => false,
        crate::domain::background::DesktopAction::Channel {
            auth_session_id, ..
        } => services
            .auth
            .monitor_session()
            .is_none_or(|(id, _)| id.to_string() != *auth_session_id),
        _ => false,
    });
    if expired {
        state.action = None;
    }
    crate::domain::background::DesktopStatus {
        monitor: services.monitor.snapshot(),
        notification_permission: state.permission,
        notification_click_supported: state.clicks,
        notification_test_available: cfg!(feature = "notification-acceptance"),
        tray_available: app
            .state::<Arc<Lifecycle>>()
            .tray_created
            .load(Ordering::SeqCst)
            && state.tray_host,
        action: state.action.clone(),
    }
}
pub(crate) fn acknowledge_action(app: &tauri::AppHandle, id: &str) {
    let notifications = app.state::<Arc<notifications::Notifications>>();
    let mut state = notifications
        .shared
        .state
        .lock()
        .expect("native notification state poisoned");
    if state.action.as_ref().is_some_and(|a| a.id() == id) {
        state.action = None;
    }
}
pub(crate) fn request_permission(app: &tauri::AppHandle) -> crate::domain::Result<()> {
    app.state::<Arc<notifications::Notifications>>()
        .request_permission()
}

#[cfg(feature = "notification-acceptance")]
pub(crate) fn notification_test(
    app: &tauri::AppHandle,
    request: crate::domain::background::NotificationTestAction,
) -> crate::domain::Result<()> {
    app.state::<Arc<notifications::Notifications>>()
        .test_notification(request)
}

#[cfg(all(feature = "test-support", target_os = "linux"))]
mod smoke;
#[cfg(all(feature = "test-support", target_os = "linux"))]
pub use smoke::run as run_background_smoke;

#[cfg(test)]
mod tests {
    #[test]
    fn notification_acceptance_permission_requires_feature_and_local_main_window() {
        use tauri::ipc::Origin;
        let mut context = super::app_context();
        let authority = context.runtime_authority_mut();
        assert_eq!(
            authority
                .resolve_access("dev_notification_test", "main", "main", &Origin::Local)
                .is_some(),
            cfg!(feature = "notification-acceptance")
        );
        assert!(
            authority
                .resolve_access("dev_notification_test", "other", "other", &Origin::Local)
                .is_none()
        );
        assert!(
            authority
                .resolve_access(
                    "dev_notification_test",
                    "main",
                    "main",
                    &Origin::Remote {
                        url: "https://example.invalid".parse().unwrap()
                    }
                )
                .is_none()
        );
        assert!(
            authority
                .resolve_access("desktop_status", "main", "main", &Origin::Local)
                .is_some()
        );
    }
}
