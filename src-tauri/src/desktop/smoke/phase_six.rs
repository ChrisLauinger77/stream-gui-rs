//! Real WebKitGTK assertions, with synthetic processes and isolated settings/bus.
use super::*;

async fn evaluate(app: &tauri::AppHandle, script: &str) -> serde_json::Value {
    let (send, mut receive) = tokio::sync::mpsc::unbounded_channel();
    app.get_webview_window("main")
        .unwrap()
        .eval_with_callback(script, move |value| {
            let _ = send.send(value);
        })
        .unwrap();
    serde_json::from_str(&receive.recv().await.unwrap()).unwrap_or_default()
}
async fn until(app: &tauri::AppHandle, script: &str) {
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            if evaluate(app, script).await == true {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("WebKitGTK condition failed: {script}"));
}
async fn click(app: &tauri::AppHandle, text: &str) {
    let label = serde_json::to_string(text).unwrap();
    let script = format!(
        "(() => {{ const button = [...document.querySelectorAll('button')].find(b => b.textContent === {label}); if (!button) return false; button.focus(); button.click(); return true; }})()"
    );
    assert_eq!(evaluate(app, &script).await, true);
}

async fn navigate_to_watching(app: &tauri::AppHandle) {
    // Same native action slot as the tray; no Twitch identity or production bus.
    app.state::<Arc<notifications::Notifications>>()
        .shared
        .state
        .lock()
        .unwrap()
        .action = Some(crate::domain::background::DesktopAction::Watching {
        id: uuid::Uuid::new_v4().to_string(),
    });
    show_window(app);
    until(app, "!document.querySelector('dialog') && !!document.querySelector('.watching-panel h2') && document.activeElement === document.querySelector('.watching-panel h2')").await;
    wait_until(|| {
        app.state::<Arc<notifications::Notifications>>()
            .shared
            .state
            .lock()
            .unwrap()
            .action
            .is_none()
    })
    .await;
}

pub(super) async fn check(app: &tauri::AppHandle) {
    let window = app.get_webview_window("main").unwrap();
    let services = app.state::<Arc<Services>>();
    let before = services.sessions.sessions().await;
    let monitor = services.monitor.snapshot().phase;
    until(app, "!!document.querySelector('.app-footer')").await;
    window.hide().unwrap();
    wait_until(|| !window.is_visible().unwrap_or(true)).await;
    show_about(app);
    until(app, "!!document.querySelector('.about-dialog[open] a')").await;
    assert!(window.is_visible().unwrap());
    let info = crate::build_info::snapshot();
    let version =
        serde_json::to_string(&format!("Version {} ({})", info.version, info.commit)).unwrap();
    assert_eq!(
        evaluate(
            app,
            &format!("document.querySelector('.about-dialog').textContent.includes({version})")
        )
        .await,
        true
    );
    assert_eq!(
        evaluate(
            app,
            "document.querySelector('.about-dialog img').naturalWidth > 0"
        )
        .await,
        true
    );
    evaluate(
        app,
        "document.querySelector('.about-dialog a').focus(); true",
    )
    .await;
    show_about(app);
    until(
        app,
        "document.activeElement === document.querySelector('.about-dialog h2')",
    )
    .await;
    assert_eq!(
        evaluate(app, "document.querySelectorAll('dialog[open]').length").await,
        1
    );
    assert_eq!(app.webview_windows().len(), 1);
    click(app, "Close").await;
    until(app, "!document.querySelector('dialog')").await;
    show_about(app);
    until(app, "!!document.querySelector('.about-dialog[open] a')").await;
    navigate_to_watching(app).await;
    click(app, "Settings").await;
    click(app, "Appearance").await;
    window.set_size(tauri::LogicalSize::new(620, 600)).unwrap();
    for scale in ["100", "125", "150"] {
        let script = format!(
            "(() => {{const s=[...document.querySelectorAll('label')].find(l=>l.textContent.startsWith('Text size')).querySelector('select'); s.value='{scale}'; s.dispatchEvent(new Event('change',{{bubbles:true}})); return true;}})()"
        );
        evaluate(app, &script).await;
        click(app, "Save settings").await;
        until(app, &format!("document.documentElement.dataset.textScale === '{scale}' && !document.querySelector('.playback-settings fieldset')?.disabled")).await;
        assert_eq!(
            evaluate(
                app,
                "document.documentElement.scrollWidth <= innerWidth + 1"
            )
            .await,
            true,
            "horizontal overflow at {scale}%"
        );
    }
    for theme in ["light", "dark"] {
        let script = format!(
            "(() => {{const s=[...document.querySelectorAll('label')].find(l=>l.textContent.startsWith('Appearance')).querySelector('select'); s.value='{theme}'; s.dispatchEvent(new Event('change',{{bubbles:true}})); return true;}})()"
        );
        evaluate(app, &script).await;
        click(app, "Save settings").await;
        until(app, &format!("document.documentElement.dataset.theme === '{theme}' && !document.querySelector('.playback-settings fieldset').disabled")).await;
        window.set_zoom(2.0).unwrap();
        until(app, "innerWidth < 400").await;
        assert_eq!(
            evaluate(
                app,
                "document.documentElement.scrollWidth <= innerWidth + 1"
            )
            .await,
            true,
            "horizontal overflow at 200% zoom, 150% text, {theme}"
        );
        window.set_zoom(1.0).unwrap();
        until(app, "innerWidth >= 600").await;
    }
    click(app, "Prepare support report").await;
    until(app, "!!document.querySelector('.support-report textarea')").await;
    assert_eq!(
        evaluate(
            app,
            "document.querySelector('.support-report textarea').value.includes('Process 1:')"
        )
        .await,
        true
    );
    assert_eq!(
        evaluate(
            app,
            "document.querySelector('.support-report textarea').readOnly"
        )
        .await,
        true
    );
    assert_eq!(evaluate(app, "document.querySelector('.support-report').scrollWidth <= document.querySelector('.support-report').clientWidth + 1").await, true);
    navigate_to_watching(app).await;
    // Reopen from the actual frontend/native intent, then Quit with the modal open.
    click(app, "About Stream GUI RS").await;
    until(app, "!!document.querySelector('.about-dialog[open] a')").await;
    assert_eq!(services.monitor.snapshot().phase, monitor);
    let after = services.sessions.sessions().await;
    assert_eq!(
        before.iter().map(|s| (&s.id, s.pid)).collect::<Vec<_>>(),
        after.iter().map(|s| (&s.id, s.pid)).collect::<Vec<_>>()
    );
    assert!(after.iter().all(|s| s.phase == SessionPhase::Running));
}
