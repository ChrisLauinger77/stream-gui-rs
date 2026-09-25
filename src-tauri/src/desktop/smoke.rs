//! Isolated graphical regression entry point. Not compiled into app builds.
use super::*;
use crate::{domain::LaunchRequest, streamlink::SessionPhase};
use std::{path::Path, time::Duration};
mod localization;
mod notification_server;
mod phase_eight;
mod phase_seven;
mod phase_six;
mod titlebar;

pub fn run(
    helper: &Path,
    action: &str,
    marker: &Path,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let phase_six = action == "phase6";
    let phase_seven = action == "phase7";
    let phase_eight = action == "phase8";
    let localization = action == "localization";
    let quit = action == "quit" || phase_six || phase_seven || phase_eight || localization;
    let titlebar_only = action == "titlebar";
    let directory = tempfile::tempdir()?;
    // No client ID: this fixture cannot open any production credential entry.
    let services = Arc::new(Services::new(directory.path(), None)?);
    let mut settings = services.settings.snapshot();
    settings.streamlink_path = Some(helper.to_string_lossy().into_owned());
    settings.background.close_to_background = true;
    services.settings.update(settings)?;
    // Existing native UI assertions use English labels regardless of host locale.
    services
        .settings
        .set_ui_language(crate::config::UiLanguage::En)?;
    let marker = marker.to_owned();
    // The parent launches this executable under dbus-run-session. Never register
    // a fixture on the user's desktop bus or deliver a real desktop notification.
    let notifications = Arc::new(notification_server::Server::start());
    let initial = phase_eight
        .then(|| crate::navigation::parse_link("stream-gui-rs://team/synthetic-team").unwrap());
    let app = build_app(initial)?;
    install_services(&app, services.clone());
    let outcome = Arc::new(Mutex::new(None));
    let result = outcome.clone();
    app.run(move |app, event| {
        if matches!(event, tauri::RunEvent::Ready) {
            let app = app.clone();
            let result = result.clone();
            let marker = marker.clone();
            let notifications = notifications.clone();
            tauri::async_runtime::spawn(async move {
                let check_app = app.clone();
                let check_marker = marker.clone();
                // A failed UI assertion must still drain the real fixture children.
                let checked = tokio::time::timeout(
                    Duration::from_secs(45),
                    tokio::spawn(async move {
                        scenario(
                            &check_app,
                            &check_marker,
                            &notifications,
                            titlebar_only,
                            localization,
                        )
                        .await?;
                        if phase_six {
                            phase_six::check(&check_app).await;
                        }
                        if phase_eight {
                            phase_eight::check(&check_app).await;
                        }
                        if phase_seven {
                            phase_seven::check(&check_app).await;
                        }
                        if localization {
                            localization::check(&check_app).await;
                        }
                        Ok::<(), crate::domain::AppError>(())
                    }),
                )
                .await;
                let passed = matches!(checked, Ok(Ok(Ok(()))));
                *result.lock().unwrap() = Some(passed);
                if passed {
                    std::fs::write(&marker, "passed").unwrap();
                    if quit {
                        begin_shutdown(&app);
                    } else {
                        let services = app.state::<Arc<Services>>();
                        let mut settings = services.settings.snapshot();
                        settings.background.close_to_background = false;
                        services.settings.update(settings).unwrap();
                        if titlebar_only {
                            assert!(titlebar::activate(&app, "close").await);
                        } else {
                            app.get_webview_window("main").unwrap().close().unwrap();
                        }
                    }
                } else {
                    begin_shutdown(&app);
                }
            });
        }
        handle_event(app, event);
    });
    assert_eq!(
        *outcome.lock().unwrap(),
        Some(true),
        "native background scenario failed"
    );
    let sessions = tauri::async_runtime::block_on(services.sessions.sessions());
    assert_eq!(sessions.len(), 2);
    assert!(
        sessions
            .iter()
            .all(|s| matches!(s.phase, SessionPhase::Exited | SessionPhase::Failed))
    );
    for session in sessions {
        // SAFETY: query only the fixture's recorded child PID after normal reaping.
        assert_eq!(unsafe { libc::kill(session.pid as i32, 0) }, -1);
    }
    Ok(())
}
async fn wait_until(check: impl Fn() -> bool) {
    while !check() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
async fn scenario(
    app: &tauri::AppHandle,
    marker: &Path,
    server: &notification_server::Server,
    titlebar_only: bool,
    localization_only: bool,
) -> crate::domain::Result<()> {
    let services = app.state::<Arc<Services>>();
    for channel in ["hold", "holdb"] {
        services
            .launch(LaunchRequest {
                url: format!("https://www.twitch.tv/{channel}"),
                quality: "best".into(),
            })
            .await?;
    }
    std::fs::write(marker, "playback started").unwrap();
    let ids: Vec<_> = services
        .sessions
        .sessions()
        .await
        .iter()
        .map(|s| (s.id.clone(), s.pid))
        .collect();
    let window = app.get_webview_window("main").unwrap();
    std::fs::write(
        marker.with_extension("pids"),
        serde_json::to_string(&ids.iter().map(|(_, pid)| pid).collect::<Vec<_>>()).unwrap(),
    )
    .unwrap();
    if titlebar_only {
        titlebar::check_maximize(app).await;
        assert!(titlebar::activate(app, "minimize").await);
        // Wayland does not expose an iconified state reliably. Remap before the
        // close-button check; actual pointer minimize/drag remains manual acceptance.
        window.hide().unwrap();
        show_window(app);
        return Ok(());
    }
    window.minimize().unwrap();
    wait_until(|| window.is_minimized().unwrap_or(false)).await;
    std::fs::write(marker, "minimized").unwrap();
    show_window(app);
    wait_until(|| !window.is_minimized().unwrap_or(true)).await;
    std::fs::write(marker, "restored; requesting close").unwrap();
    window.close().unwrap();
    wait_until(|| !window.is_visible().unwrap_or(true) || window.is_minimized().unwrap_or(false))
        .await;
    assert!(
        !app.state::<Arc<Lifecycle>>()
            .stopping
            .load(Ordering::SeqCst)
    );
    assert_eq!(
        services
            .sessions
            .sessions()
            .await
            .iter()
            .map(|s| (s.id.clone(), s.pid))
            .collect::<Vec<_>>(),
        ids
    );
    show_window(app);
    wait_until(|| window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(true))
        .await;
    let sessions = services.sessions.sessions().await;
    assert!(sessions.iter().all(|s| s.phase == SessionPhase::Running));
    if !localization_only {
        check_notifications(app, server).await?;
    }
    // A webview reload reconstructs from the same Supervisor snapshots.
    window
        .eval("window.__streamGuiSmokeReload = true; location.reload()")
        .unwrap();
    assert_eq!(services.sessions.sessions().await.len(), 2);
    Ok(())
}

async fn check_notifications(
    app: &tauri::AppHandle,
    server: &notification_server::Server,
) -> crate::domain::Result<()> {
    use crate::{
        domain::background::NotificationPermission,
        monitor::{LiveNotification, NotificationSink},
    };
    use tokio_util::sync::CancellationToken;
    let native = app.state::<Arc<notifications::Notifications>>();
    wait_until(|| {
        native.shared.state.lock().unwrap().permission == NotificationPermission::OsManaged
    })
    .await;
    assert!(native.shared.state.lock().unwrap().clicks);
    assert!(!status(app).tray_available); // A watcher alone is not a registered host.
    let event = LiveNotification {
        auth_session_id: "synthetic-session".into(),
        broadcaster_id: "123".into(),
        display_name: "Synthetic channel".into(),
        title: "<Test> & title".into(),
        category: "Category".into(),
        session_cancel: CancellationToken::new(),
        monitor_cancel: CancellationToken::new(),
    };
    hide_window(app);
    native.deliver(event.clone())?;
    wait_until(|| server.delivered() == 1).await;
    server.click(1);
    wait_until(|| native.shared.state.lock().unwrap().action.is_some()).await;
    assert!(matches!(native.shared.state.lock().unwrap().action,
        Some(crate::domain::background::DesktopAction::Channel { ref broadcaster_id, .. }) if broadcaster_id == "123"));
    let window = app.get_webview_window("main").unwrap();
    wait_until(|| window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(true))
        .await;
    // Without a matching signed-in session, the safe snapshot discards this target.
    assert!(status(app).action.is_none());
    native.deliver(event.clone())?;
    wait_until(|| server.delivered() == 2).await;
    event.session_cancel.cancel();
    wait_until(|| server.closed(2)).await;
    server.click(2);
    native.deliver(event)?;
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert_eq!(server.delivered(), 2);
    assert!(native.shared.state.lock().unwrap().action.is_none());
    #[cfg(feature = "notification-acceptance")]
    {
        use crate::domain::background::NotificationTestAction::{Clear, Send};
        let services = app.state::<Arc<Services>>();
        let before = serde_json::to_value(services.monitor.snapshot()).unwrap();
        let settings = serde_json::to_value(services.settings.snapshot()).unwrap();
        crate::commands::dev_notification_test(app.clone(), Send)?;
        wait_until(|| server.delivered() == 3).await;
        hide_window(app);
        server.click(3);
        wait_until(|| native.shared.state.lock().unwrap().action.is_some()).await;
        let snapshot = status(app);
        assert!(snapshot.notification_test_available);
        let action = snapshot.action.unwrap();
        assert!(notifications::acceptance::is_test_action(&action));
        wait_until(|| {
            window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(true)
        })
        .await;
        acknowledge_action(app, action.id());
        assert!(status(app).action.is_none());
        server.click(3); // The consumed target cannot activate again.
        crate::commands::dev_notification_test(app.clone(), Send)?;
        wait_until(|| server.delivered() == 4).await;
        crate::commands::dev_notification_test(app.clone(), Clear)?;
        wait_until(|| server.closed(4)).await;
        server.click(4);
        assert_eq!(
            serde_json::to_value(services.monitor.snapshot()).unwrap(),
            before
        );
        assert_eq!(
            serde_json::to_value(services.settings.snapshot()).unwrap(),
            settings
        );
        assert!(status(app).action.is_none());
    }
    assert_eq!(
        app.state::<Arc<Services>>().sessions.sessions().await.len(),
        2
    );
    if app
        .state::<Arc<Lifecycle>>()
        .tray_created
        .load(Ordering::SeqCst)
    {
        server.set_host(true);
        wait_until(|| status(app).tray_available).await;
        hide_window(app);
        wait_until(|| !window.is_visible().unwrap_or(true)).await;
        server.set_host(false);
        wait_until(|| window.is_visible().unwrap_or(false)).await;
        assert!(!status(app).tray_available);
    }
    Ok(())
}
