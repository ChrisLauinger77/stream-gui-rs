//! Native title-bar checks; the parent provides an isolated graphical session.
use super::*;
use gtk::prelude::*;

fn button(widget: &gtk::Widget, name: &str) -> Option<gtk::Button> {
    if let Some(button) = widget.downcast_ref::<gtk::Button>()
        && button.style_context().has_class(name)
    {
        return Some(button.clone());
    }
    let mut found = None;
    if let Some(container) = widget.downcast_ref::<gtk::Container>() {
        container.forall(|child| {
            if found.is_none() {
                found = button(child, name);
            }
        });
    }
    found
}

// Return false for X11 server-side decorations. GTK widgets stay on the UI thread.
pub(super) async fn activate(app: &tauri::AppHandle, action: &'static str) -> bool {
    loop {
        let (send, receive) = tokio::sync::oneshot::channel();
        let handle = app.clone();
        app.run_on_main_thread(move || {
            let window = handle
                .get_webview_window("main")
                .unwrap()
                .gtk_window()
                .unwrap();
            let result = window.titlebar().map(|widget| {
                let events = widget.downcast::<gtk::EventBox>().unwrap();
                let header = events
                    .child()
                    .unwrap()
                    .downcast::<gtk::HeaderBar>()
                    .unwrap();
                let snapshot = (
                    events.is_above_child(),
                    header.has_subtitle(),
                    header.allocated_height(),
                );
                // Ready precedes Wayland's first frame allocation. Do not measure
                // or activate controls until the compositor has laid them out.
                if snapshot.2 > 1 {
                    button(header.upcast_ref(), action)
                        .expect("native window button")
                        .emit_clicked();
                }
                snapshot
            });
            let _ = send.send(result);
        })
        .unwrap();
        let Some((intercepts, subtitle, height)) = receive.await.unwrap() else {
            return false;
        };
        if height <= 1 {
            tokio::time::sleep(Duration::from_millis(10)).await;
            continue;
        }
        assert!(
            !intercepts,
            "title-bar wrapper must let window buttons receive pointer input"
        );
        assert!(
            !subtitle,
            "title bar must not reserve an unused subtitle row"
        );
        assert!(
            (28..=36).contains(&height),
            "expected a compact title bar, got {height}px"
        );
        return true;
    }
}

pub(super) async fn check_maximize(app: &tauri::AppHandle) {
    assert!(
        activate(app, "maximize").await,
        "requires native Wayland decorations"
    );
    let window = app.get_webview_window("main").unwrap();
    wait_until(|| window.is_maximized().unwrap_or(false)).await;
    assert!(activate(app, "maximize").await);
    wait_until(|| !window.is_maximized().unwrap_or(true)).await;
}
