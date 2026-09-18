use crate::{commands, domain::services::Services};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tauri::Manager;

#[derive(Default)]
struct Lifecycle {
    stopping: AtomicBool,
    finished: AtomicBool,
    auth_stop: Mutex<Option<tokio::sync::watch::Sender<bool>>>,
}

pub fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            let directory = app.path().app_config_dir()?;
            let client_id = std::env::var("TWITCH_CLIENT_ID").ok().map(|id| id.trim().to_owned());
            let services = Arc::new(Services::new(&directory, client_id)?);
            let lifecycle = Arc::new(Lifecycle::default());
            app.manage(services.clone());
            app.manage(lifecycle.clone());
            tauri::async_runtime::spawn(async move {
                let stop = services.auth.start();
                let mut stored = lifecycle.auth_stop.lock().expect("lifecycle mutex poisoned");
                if lifecycle.stopping.load(Ordering::SeqCst) { stop.send_replace(true); }
                *stored = Some(stop);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::backend_diagnostics, commands::streamlink_probe,
            commands::streamlink_launch, commands::streamlink_stop, commands::streamlink_sessions,
            commands::auth_status, commands::auth_login, commands::auth_open_verification,
            commands::auth_validate, commands::auth_refresh, commands::auth_logout,
            commands::auth_cancel, commands::auth_account,
        ])
        .build(tauri::generate_context!())
        .expect("Could not initialize Stream GUI RS; check application settings and system prerequisites");

    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            let lifecycle = app.state::<Arc<Lifecycle>>().inner().clone();
            if lifecycle.finished.load(Ordering::SeqCst) {
                return;
            }
            api.prevent_exit();
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
    });
}
