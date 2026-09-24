//! Native UI text uses the same bundled catalogs as the webview.
use crate::{config::UiLanguage, domain::services::Services, monitor::MonitorPhase};
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};
use tauri::{AppHandle, Manager, Runtime};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Locale {
    En,
    De,
    Es,
    Fr,
}

fn from_tag(tag: &str) -> Locale {
    let language = tag.trim().split(['-', '_', '.', ':']).next().unwrap_or("");
    match language.to_ascii_lowercase().as_str() {
        "de" => Locale::De,
        "es" => Locale::Es,
        "fr" => Locale::Fr,
        _ => Locale::En,
    }
}

fn system_locale() -> Locale {
    #[cfg(target_os = "macos")]
    {
        return from_tag(
            &objc2_foundation::NSLocale::currentLocale()
                .languageCode()
                .to_string(),
        );
    }
    #[cfg(windows)]
    {
        let mut name = [0_u16; 85];
        // SAFETY: the system writes at most the supplied capacity to this stack buffer.
        let length = unsafe {
            windows_sys::Win32::Globalization::GetUserDefaultLocaleName(
                name.as_mut_ptr(),
                name.len() as i32,
            )
        };
        return if length > 1 {
            from_tag(&String::from_utf16_lossy(&name[..length as usize - 1]))
        } else {
            Locale::En
        };
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        // WebKitGTK uses the desktop language even when a terminal overrides
        // LC_ALL for diagnostics; match the webview-facing preference here.
        ["LANGUAGE", "LC_MESSAGES", "LANG", "LC_ALL"]
            .iter()
            .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
            .map_or(Locale::En, |tag| from_tag(&tag))
    }
}

pub(super) fn selected(language: UiLanguage) -> Locale {
    match language {
        UiLanguage::System => system_locale(),
        UiLanguage::En => Locale::En,
        UiLanguage::De => Locale::De,
        UiLanguage::Es => Locale::Es,
        UiLanguage::Fr => Locale::Fr,
    }
}

pub(super) fn system_language() -> UiLanguage {
    match system_locale() {
        Locale::En => UiLanguage::En,
        Locale::De => UiLanguage::De,
        Locale::Es => UiLanguage::Es,
        Locale::Fr => UiLanguage::Fr,
    }
}

pub(super) fn current<R: Runtime>(app: &AppHandle<R>) -> Locale {
    selected(app.state::<Arc<Services>>().settings.snapshot().ui_language)
}

fn catalog(locale: Locale) -> &'static HashMap<String, String> {
    static EN: OnceLock<HashMap<String, String>> = OnceLock::new();
    static DE: OnceLock<HashMap<String, String>> = OnceLock::new();
    static ES: OnceLock<HashMap<String, String>> = OnceLock::new();
    static FR: OnceLock<HashMap<String, String>> = OnceLock::new();
    let (slot, json) = match locale {
        Locale::En => (&EN, include_str!("../../../src/i18n/en.json")),
        Locale::De => (&DE, include_str!("../../../src/i18n/de.json")),
        Locale::Es => (&ES, include_str!("../../../src/i18n/es.json")),
        Locale::Fr => (&FR, include_str!("../../../src/i18n/fr.json")),
    };
    slot.get_or_init(|| serde_json::from_str(json).expect("validated bundled translation catalog"))
}

pub(super) fn text(locale: Locale, key: &str) -> &'static str {
    catalog(locale)
        .get(key)
        .or_else(|| catalog(Locale::En).get(key))
        .map(String::as_str)
        .unwrap_or("⟦missing native translation⟧")
}

pub(super) fn named(locale: Locale, key: &str, name: &str) -> String {
    text(locale, key).replace("{name}", name)
}

pub(super) fn counted(locale: Locale, key: &str, count: usize) -> String {
    text(locale, key).replace("{count}", &count.to_string())
}

pub(super) fn phase(locale: Locale, phase: MonitorPhase) -> &'static str {
    let key = match phase {
        MonitorPhase::Disabled => "native.phase.disabled",
        MonitorPhase::SignedOut => "native.phase.signedOut",
        MonitorPhase::Paused => "native.phase.paused",
        MonitorPhase::Baseline => "native.phase.baseline",
        MonitorPhase::Running => "native.phase.running",
        MonitorPhase::Recovering => "native.phase.recovering",
        MonitorPhase::Stopped => "native.phase.stopped",
    };
    text(locale, key)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_locale_selection_and_catalogs() {
        for (tag, expected) in [
            ("en-GB", Locale::En),
            ("de_AT", Locale::De),
            ("es-MX", Locale::Es),
            ("fr-CA", Locale::Fr),
            ("ja-JP", Locale::En),
            ("fr:de", Locale::Fr),
        ] {
            assert_eq!(from_tag(tag), expected);
        }
        for locale in [Locale::En, Locale::De, Locale::Es, Locale::Fr] {
            assert!(!named(locale, "native.live", "Alice").contains("{name}"));
            assert!(!counted(locale, "native.watching", 2).contains("{count}"));
            assert_ne!(
                phase(locale, MonitorPhase::Running),
                "⟦missing native translation⟧"
            );
        }
        assert_eq!(selected(UiLanguage::De), Locale::De);
    }
}
