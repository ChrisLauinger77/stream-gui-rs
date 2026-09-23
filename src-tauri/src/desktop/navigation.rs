//! OS delivery ends at the same validated inbox used for cold startup.
use super::*;
use crate::navigation::{NavigationInbox, NavigationIntent};

#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(super) fn receive_arguments(app: &tauri::AppHandle, args: &[String]) {
    if let Ok(intent) = crate::navigation::parse_arguments(args) {
        receive(app, intent);
    }
}
pub(super) fn receive(app: &tauri::AppHandle, intent: Option<NavigationIntent>) {
    if app
        .try_state::<Arc<Lifecycle>>()
        .is_some_and(|life| life.stopping.load(Ordering::SeqCst))
    {
        return;
    }
    if let Some(intent) = intent {
        app.state::<NavigationInbox>().receive(intent);
    }
    // The callback may arrive before the event loop creates the window. Ready
    // repeats activation; the inbox exists before plugin initialization.
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || show_window(&handle));
}
