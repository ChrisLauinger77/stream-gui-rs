//! Phase 7 presentation in the real WebKitGTK view, with synthetic settings only.
use super::phase_six::{click, evaluate, until};
use super::*;

pub(super) async fn check(app: &tauri::AppHandle) {
    let window = app.get_webview_window("main").unwrap();
    let services = app.state::<Arc<Services>>();
    until(
        app,
        "window.__streamGuiSmokeReload !== true && !!document.querySelector('.app-footer')",
    )
    .await;
    click(app, "Settings").await;
    window.set_size(tauri::LogicalSize::new(620, 600)).unwrap();
    for (scale, theme) in [("100", "system"), ("125", "light"), ("150", "dark")] {
        click(app, "Appearance").await;
        evaluate(app, &format!("(() => {{ const s=[...document.querySelectorAll('label')].find(l=>l.textContent.startsWith('Text size')).querySelector('select'); s.value='{scale}'; s.dispatchEvent(new Event('change',{{bubbles:true}})); return true; }})()")).await;
        evaluate(app, &format!("(() => {{ const s=[...document.querySelectorAll('label')].find(l=>l.textContent.startsWith('Appearance')).querySelector('select'); s.value='{theme}'; s.dispatchEvent(new Event('change',{{bubbles:true}})); return true; }})()")).await;
        click(app, "Save settings").await;
        until(app, &format!("document.documentElement.dataset.textScale === '{scale}' && document.documentElement.dataset.theme === '{theme}' && !document.querySelector('.playback-settings fieldset').disabled")).await;
        click(app, "Player").await;
        click(app, "Add profile").await;
        until(
            app,
            "document.activeElement === document.querySelector('.profile-editor input')",
        )
        .await;
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
            assert_eq!(
                evaluate(
                    app,
                    "document.documentElement.scrollWidth <= innerWidth + 1 && document.querySelector('.settings-panel').scrollWidth <= document.querySelector('.settings-panel').clientWidth + 1"
                )
                .await,
                true,
                "profile overflow {scale}% / {zoom}"
            );
        }
        window.set_zoom(1.0).unwrap();
        evaluate(app, "(() => { const s=document.querySelector('.profile-editor input'); Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(s, 'Synthetic_' + 'x'.repeat(54)); s.dispatchEvent(new Event('input',{bubbles:true})); return true; })()").await;
        click(app, "Save profile").await;
        until(app, "!document.querySelector('.profile-editor') && document.querySelector('[aria-label=\"Player profiles\"] select').options.length === 2").await;
        evaluate(app, "(() => { const s=document.querySelector('[aria-label=\"Player profiles\"] select'); s.value=s.options[1].value; s.dispatchEvent(new Event('change',{bubbles:true})); return true; })()").await;
        until(app, "[...document.querySelectorAll('button')].some(b=>b.textContent==='Edit profile' && !b.disabled)").await;
        click(app, "Use profile").await;
        wait_until(|| services.settings.snapshot().selected_profile_id.is_some()).await;
        click(app, "Delete profile").await;
        until(app, "document.querySelector('[aria-label=\"Player profiles\"] select').options.length === 1").await;
        assert!(services.settings.snapshot().selected_profile_id.is_none());
        click(app, "Playback").await;
        evaluate(app, "(() => { const s=[...document.querySelectorAll('label')].find(l=>l.textContent.startsWith('Chat application')).querySelector('select'); s.value='chatterino'; s.dispatchEvent(new Event('change',{bubbles:true})); return true; })()").await;
        until(app, "[...document.querySelectorAll('label')].some(l=>l.textContent.startsWith('Chatterino executable'))").await;
        window.set_zoom(2.0).unwrap();
        until(app, "innerWidth < 400").await;
        assert_eq!(
            evaluate(
                app,
                "document.documentElement.scrollWidth <= innerWidth + 1 && document.querySelector('.settings-panel').scrollWidth <= document.querySelector('.settings-panel').clientWidth + 1"
            )
            .await,
            true,
            "chat overflow"
        );
        click(app, "Updates").await;
        until(
            app,
            "!!document.querySelector('[aria-label=\"Update awareness\"]')",
        )
        .await;
        assert_eq!(
            evaluate(
                app,
                "document.documentElement.scrollWidth <= innerWidth + 1 && document.querySelector('.settings-panel').scrollWidth <= document.querySelector('.settings-panel').clientWidth + 1"
            )
            .await,
            true,
            "update overflow"
        );
        assert_eq!(
            services.updates.status().phase,
            crate::updates::UpdatePhase::NotChecked
        );
        window.set_zoom(1.0).unwrap();
    }
    click(app, "Close Settings").await;
    assert_eq!(services.sessions.sessions().await.len(), 2);
}
