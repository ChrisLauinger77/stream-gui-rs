use super::*;
use crate::domain::background::DesktopAction;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};

#[cfg(target_os = "linux")]
fn indicator_available() -> bool {
    [
        c"libayatana-appindicator3.so.1",
        c"libappindicator3.so.1",
        c"libayatana-appindicator3.so",
        c"libappindicator3.so",
    ]
    .iter()
    .any(|name| {
        // SAFETY: these are fixed system-library names with the ABI used by
        // Tauri. No symbols are invoked, and the successful handle is balanced.
        let handle = unsafe { libc::dlopen(name.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL) };
        if handle.is_null() {
            false
        } else {
            unsafe {
                libc::dlclose(handle);
            }
            true
        }
    })
}
pub(super) fn install(app: &tauri::AppHandle) -> tauri::Result<bool> {
    #[cfg(target_os = "linux")]
    if !indicator_available() {
        return Ok(false);
    }
    let show = MenuItem::with_id(app, "show", "Show Stream GUI RS", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide window", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "monitor", "Pause monitoring", false, None::<&str>)?;
    let watching = MenuItem::with_id(app, "watching", "Watching (0)", true, None::<&str>)?;
    let about = MenuItem::with_id(app, "about", "About Stream GUI RS", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &hide,
            &pause,
            &watching,
            &PredefinedMenuItem::separator(app)?,
            &about,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;
    let mut builder = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .tooltip("Stream GUI RS")
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_window(app),
            "hide" => hide_window(app),
            "monitor" => {
                let services = app.state::<Arc<Services>>();
                services.monitor.pause(!services.monitor.snapshot().paused);
            }
            "watching" => {
                app.state::<Arc<notifications::Notifications>>()
                    .shared
                    .state
                    .lock()
                    .expect("native notification state poisoned")
                    .action = Some(DesktopAction::Watching {
                    id: uuid::Uuid::new_v4().to_string(),
                });
                show_window(app);
            }
            "about" => show_about(app),
            "quit" => begin_shutdown(app),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    let tray = builder.build(app)?;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut previous = None;
        loop {
            if app
                .state::<Arc<Lifecycle>>()
                .stopping
                .load(Ordering::SeqCst)
            {
                break;
            }
            let services = app.state::<Arc<Services>>();
            let status = services.monitor.snapshot();
            let enabled = services.settings.snapshot().background.monitoring_enabled;
            let count = services
                .sessions
                .sessions()
                .await
                .iter()
                .filter(|s| {
                    s.restarting
                        || matches!(
                            s.phase,
                            crate::streamlink::SessionPhase::Starting
                                | crate::streamlink::SessionPhase::Running
                                | crate::streamlink::SessionPhase::Stopping
                        )
                })
                .count();
            let host = app
                .state::<Arc<notifications::Notifications>>()
                .shared
                .state
                .lock()
                .expect("native notification state poisoned")
                .tray_host;
            let current = (status.phase, status.paused, enabled, count, host);
            if previous != Some(current) {
                let _ = pause.set_text(if status.paused {
                    "Resume monitoring"
                } else {
                    "Pause monitoring"
                });
                let _ = pause.set_enabled(enabled);
                let _ = watching.set_text(format!("Watching ({count})"));
                let _ = quit.set_text(if count == 0 {
                    "Quit".into()
                } else {
                    format!("Quit (stops {count} streams)")
                });
                let _ = hide.set_enabled(host);
                let _ = tray.set_tooltip(Some(format!(
                    "Stream GUI RS · Monitoring: {:?} · Watching: {count}",
                    status.phase
                )));
                // An indicator host disappearing must not strand a hidden app.
                if !host
                    && app
                        .get_webview_window("main")
                        .is_some_and(|window| window.is_visible().is_ok_and(|visible| !visible))
                {
                    show_window(&app);
                }
                previous = Some(current);
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
    Ok(true)
}
