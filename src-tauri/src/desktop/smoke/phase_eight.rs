//! Real native forwarding and WebKit presentation, all on an isolated D-Bus and
//! settings directory. Public browse DTOs are synthetic, not live Twitch proof.
use super::phase_six::{click, evaluate, until};
use super::*;

pub(super) async fn check(app: &tauri::AppHandle) {
    use crate::navigation::{NavigationInbox, NavigationIntent};
    let window = app.get_webview_window("main").unwrap();
    let inbox = app.state::<NavigationInbox>();
    assert!(matches!(
        inbox.snapshot().unwrap().intent,
        NavigationIntent::Team { .. }
    ));
    until(
        app,
        "window.__streamGuiSmokeReload !== true && !!document.querySelector('.app-footer')",
    )
    .await;
    until(
        app,
        "document.body.textContent.includes('Connect to Twitch to open the requested destination')",
    )
    .await;
    assert_eq!(evaluate(app, include_str!("phase_eight.js")).await, true);
    until(app, "window.__phaseEightReady === true").await;
    until(app,"!!document.querySelector('.workspace') && document.body.textContent.includes('Synthetic native rendering fixture')").await;
    wait_until(|| inbox.snapshot().is_none()).await;
    assert_eq!(
        evaluate(
            app,
            "document.activeElement === document.querySelector('h1')"
        )
        .await,
        true
    );
    assert_eq!(evaluate(app,"(() => { const b=document.querySelector('[aria-label=\"Open channel Synthetic Channel\"]'); b.focus(); b.click(); return true; })()").await,true);
    click(app, "Bookmark channel").await;
    until(
        app,
        "document.body.textContent.includes('Bookmark saved locally')",
    )
    .await;
    click(app, "Remove bookmark").await;
    until(
        app,
        "document.body.textContent.includes('Bookmark removed.')",
    )
    .await;
    click(app, "Bookmark channel").await;
    until(
        app,
        "document.body.textContent.includes('Bookmark saved locally.')",
    )
    .await;
    click(app, "Hide channel from discovery").await;
    until(
        app,
        "document.body.textContent.includes('Hidden from discovery.')",
    )
    .await;
    click(app, "← Back").await;
    until(
        app,
        "document.activeElement?.dataset.focus === 'channel:123'",
    )
    .await;
    click(app, "Forward →").await;
    until(app, "!!document.querySelector('.channel-detail')").await;
    click(app, "Settings").await;
    click(app, "Hidden items").await;
    click(app, "Restore").await;
    until(
        app,
        "document.body.textContent.includes('No hidden items.')",
    )
    .await;
    window.set_size(tauri::LogicalSize::new(620, 600)).unwrap();
    for (scale, theme) in [("100", "system"), ("125", "light"), ("150", "dark")] {
        click(app, "Appearance").await;
        evaluate(app,&format!("(() => {{ for (const [label,value] of [['Text size','{scale}'],['Color mode','{theme}']]) {{ const s=[...document.querySelectorAll('label')].find(l=>l.textContent.startsWith(label)).querySelector('select'); s.value=value; s.dispatchEvent(new Event('change',{{bubbles:true}})); }} return true; }})()")).await;
        click(app, "Save settings").await;
        until(app,&format!("document.documentElement.dataset.textScale === '{scale}' && !document.querySelector('.playback-settings fieldset').disabled")).await;
        click(app, "Shortcuts").await;
        for zoom in [1.0, 2.0] {
            window.set_zoom(zoom).unwrap();
            until(
                app,
                if zoom == 2.0 {
                    "innerWidth < 400"
                } else {
                    "innerWidth >= 600"
                },
            )
            .await;
            assert_eq!(evaluate(app,"document.documentElement.scrollWidth <= innerWidth + 1 && document.querySelector('.settings-panel').scrollWidth <= document.querySelector('.settings-panel').clientWidth + 1").await,true,"shortcut layout {scale}% / {zoom}");
        }
        window.set_zoom(1.0).unwrap();
        click(app, "Close Settings").await;
        click(app, "Bookmarks").await;
        check_layout(app).await;
        click(app, "Synthetic Channel").await;
        until(app, "!!document.querySelector('.channel-detail')").await;
        check_layout(app).await;
        inbox.receive(NavigationIntent::Team {
            name: "synthetic-team".into(),
        });
        until(
            app,
            "document.body.textContent.includes('Synthetic native rendering fixture')",
        )
        .await;
        wait_until(|| inbox.snapshot().is_none()).await;
        check_layout(app).await;
        click(app, "Settings").await;
        click(app, "Shortcuts").await;
    }
    evaluate(app,"(() => { const b=document.querySelector('[aria-label=\"Change Focus Search shortcut\"]'); b.focus(); b.click(); return true; })()").await;
    until(app, "document.body.textContent.includes('Listening…')").await;
    evaluate(app,"(() => { document.activeElement.dispatchEvent(new KeyboardEvent('keydown',{key:'b',ctrlKey:true,shiftKey:true,bubbles:true,cancelable:true})); return true; })()").await;
    click(app, "Save shortcuts").await;
    until(
        app,
        "document.body.textContent.includes('Shortcuts saved.')",
    )
    .await;
    click(app, "Close Settings").await;
    evaluate(app,"(() => { document.querySelector('h1').focus(); document.activeElement.dispatchEvent(new KeyboardEvent('keydown',{key:'b',ctrlKey:true,shiftKey:true,bubbles:true,cancelable:true})); return true; })()").await;
    until(
        app,
        "document.activeElement === document.querySelector('input[type=search]')",
    )
    .await;
    // A second actual application process must forward and exit without touching
    // credentials. The primary fixture already owns this isolated bus identity.
    let executable = std::env::current_exe()
        .unwrap()
        .with_file_name("stream-gui-rs");
    for hidden in [false, true] {
        if hidden {
            window.hide().unwrap();
            wait_until(|| !window.is_visible().unwrap()).await;
        }
        let executable = executable.clone();
        let status = tokio::task::spawn_blocking(move || {
            std::process::Command::new(executable)
                .arg("stream-gui-rs://channel/synthetic")
                .status()
                .unwrap()
        })
        .await
        .unwrap();
        assert!(status.success());
        wait_until(|| window.is_visible().unwrap()).await;
        until(app,"!!document.querySelector('.channel-detail') && document.activeElement === document.querySelector('h1')").await;
        wait_until(|| inbox.snapshot().is_none()).await;
    }
    assert_eq!(app.webview_windows().len(), 1);
    assert_eq!(
        app.state::<Arc<Services>>().sessions.sessions().await.len(),
        2
    );
}

async fn check_layout(app: &tauri::AppHandle) {
    let window = app.get_webview_window("main").unwrap();
    for zoom in [1.0, 2.0] {
        window.set_zoom(zoom).unwrap();
        until(
            app,
            if zoom == 2.0 {
                "innerWidth < 400"
            } else {
                "innerWidth >= 600"
            },
        )
        .await;
        assert_eq!(evaluate(app, "document.documentElement.scrollWidth <= innerWidth + 1 && document.querySelector('.browse-content').scrollWidth <= document.querySelector('.browse-content').clientWidth + 1").await, true, "Phase 8 browsing overflow at zoom {zoom}");
    }
    window.set_zoom(1.0).unwrap();
}
