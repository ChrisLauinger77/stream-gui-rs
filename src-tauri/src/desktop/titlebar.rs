//! Adapt Tao's native Wayland decorations without replacing the realized title bar.
use gtk::prelude::*;
use tauri::Manager;

pub(super) fn configure(app: &tauri::App) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    let native = window.gtk_window()?;
    // X11 normally uses window-manager decorations and has no GTK title bar.
    let Some(titlebar) = native.titlebar() else {
        return Ok(());
    };
    let Some(events) = titlebar.downcast_ref::<gtk::EventBox>() else {
        return Ok(());
    };
    let Some(header) = events
        .child()
        .and_then(|child| child.downcast::<gtk::HeaderBar>().ok())
    else {
        return Ok(());
    };

    // Tao 0.35 places the EventBox above the header, intercepting button clicks.
    // Keep the title-bar/drag surface intact but give native controls first input.
    events.set_above_child(false);
    header.set_has_subtitle(false);
    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        b".stream-gui-compact-titlebar headerbar { min-height: 28px; padding: 0; }\
          .stream-gui-compact-titlebar headerbar button.titlebutton { min-height: 24px; min-width: 24px; padding: 0; margin: 0; }",
    ).map_err(|error| tauri::Error::Anyhow(error.into()))?;
    titlebar
        .style_context()
        .add_class("stream-gui-compact-titlebar");
    if let Some(screen) = titlebar.screen() {
        gtk::StyleContext::add_provider_for_screen(
            &screen,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
    Ok(())
}
