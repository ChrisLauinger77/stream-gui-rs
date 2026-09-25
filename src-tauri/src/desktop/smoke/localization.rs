//! Native WebKitGTK language and text-growth check with isolated settings.
use super::*;
use crate::config::UiLanguage;

pub(super) async fn check(app: &tauri::AppHandle) {
    use phase_six::{click, evaluate, until};
    let window = app.get_webview_window("main").unwrap();
    let services = app.state::<Arc<Services>>();
    until(
        app,
        "window.__streamGuiSmokeReload !== true && !!document.querySelector('.app-footer')",
    )
    .await;
    window.set_size(tauri::LogicalSize::new(620, 600)).unwrap();
    until(app, "innerWidth === 620").await;
    click(app, "Settings").await;
    click(app, "Appearance").await;
    for (value, locale, settings, expected) in [
        ("de", UiLanguage::De, "Einstellungen", "de"),
        ("es", UiLanguage::Es, "Ajustes", "es"),
        ("fr", UiLanguage::Fr, "Paramètres", "fr"),
    ] {
        let script = format!(
            "(() => {{const s=[...document.querySelectorAll('select')].find(s=>[...s.options].some(o=>o.value==='fr'));s.value='{value}';s.dispatchEvent(new Event('change',{{bubbles:true}}));return true;}})()"
        );
        assert_eq!(evaluate(app, &script).await, true);
        until(app, &format!("document.documentElement.lang === '{expected}' && [...document.querySelectorAll('.settings-header h2')].some(h=>h.textContent==='{settings}')")).await;
        assert_eq!(services.settings.snapshot().ui_language, locale);
        assert_eq!(
            evaluate(
                app,
                "document.documentElement.scrollWidth <= innerWidth + 1"
            )
            .await,
            true
        );
        if value == "de" {
            assert_eq!(evaluate(app, "(() => {const s=[...document.querySelectorAll('select')].find(s=>[...s.options].some(o=>o.value==='150'));s.value='150';s.dispatchEvent(new Event('change',{bubbles:true}));return true;})()").await, true);
            click(app, "Einstellungen speichern").await;
            until(app, "document.documentElement.dataset.textScale === '150' && !document.querySelector('.playback-settings fieldset').disabled").await;
        }
        window.set_zoom(2.0).unwrap();
        until(app, "innerWidth < 400").await;
        assert_eq!(
            evaluate(
                app,
                "document.documentElement.scrollWidth <= innerWidth + 1"
            )
            .await,
            true,
            "horizontal overflow in {value}"
        );
        window.set_zoom(1.0).unwrap();
        until(app, "innerWidth >= 600").await;
    }
    assert_eq!(evaluate(app, "(() => {const s=[...document.querySelectorAll('select')].find(s=>[...s.options].some(o=>o.value==='fr'));s.value='system';s.dispatchEvent(new Event('change',{bubbles:true}));return true;})()").await, true);
    until(app, "document.documentElement.lang === 'de'").await;
    assert_eq!(services.settings.snapshot().ui_language, UiLanguage::System);
}
