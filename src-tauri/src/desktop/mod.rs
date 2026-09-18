use crate::{commands, domain::services::Services};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tauri::Manager;
pub(crate) mod browser;

#[derive(Default)]
struct Lifecycle {
    stopping: AtomicBool,
    finished: AtomicBool,
    auth_stop: Mutex<Option<tokio::sync::watch::Sender<bool>>>,
}

struct NativeChatOpener;
impl crate::domain::chat::ChatOpener for NativeChatOpener {
    fn open(&self, target: &crate::domain::chat::ChatTarget) -> crate::domain::Result<()> {
        browser::open(target.url()).map_err(|_| crate::domain::chat::open_error())
    }
}

pub fn run() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let app = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::backend_diagnostics,
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
            commands::list_categories,
            commands::list_category_streams,
            commands::search_channels,
            commands::search_categories,
            commands::get_channel,
        ])
        .build(tauri::generate_context!())?;

    // Tauri panics when a setup hook returns an error. Validate and install our
    // services before entering its event loop so malformed settings fail cleanly.
    let directory = app.path().app_config_dir()?;
    let client_id = crate::config::twitch_client_id::from_environment()?;
    let services = Arc::new(
        Services::new(&directory, Some(client_id))?.with_chat_opener(Arc::new(NativeChatOpener)),
    );
    let lifecycle = Arc::new(Lifecycle::default());
    app.manage(services.clone());
    app.manage(lifecycle.clone());
    tauri::async_runtime::spawn(async move {
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

    app.run(|app, event| match event {
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
            begin_shutdown(app);
        }
        _ => {}
    });
    Ok(())
}

// There is no background/tray mode: closing the window and Quit both reap
// owned playback before exiting. Navigation or webview reload does not enter here.
fn begin_shutdown(app: &tauri::AppHandle) {
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
        lifecycle.finished.store(true, Ordering::SeqCst);
        app.exit(0);
    });
}
