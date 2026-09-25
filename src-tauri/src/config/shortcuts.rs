//! Application-local actions only; no executable or native hotkey registration.
use super::{SettingsStore, settings_error};
use crate::domain::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutAction {
    Home,
    Following,
    Live,
    Categories,
    Search,
    Watching,
    OpenChannel,
    Bookmarks,
    Settings,
    Back,
    Forward,
    Refresh,
}
pub const ACTIONS: [ShortcutAction; 12] = [
    ShortcutAction::Home,
    ShortcutAction::Following,
    ShortcutAction::Live,
    ShortcutAction::Categories,
    ShortcutAction::Search,
    ShortcutAction::Watching,
    ShortcutAction::OpenChannel,
    ShortcutAction::Bookmarks,
    ShortcutAction::Settings,
    ShortcutAction::Back,
    ShortcutAction::Forward,
    ShortcutAction::Refresh,
];
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShortcutBinding {
    pub key: String,
    pub primary: bool,
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(transparent)]
pub struct ShortcutBindings(pub BTreeMap<ShortcutAction, Option<ShortcutBinding>>);
impl Default for ShortcutBindings {
    fn default() -> Self {
        Self::defaults(cfg!(target_os = "macos"))
    }
}
impl ShortcutBindings {
    pub fn defaults(mac: bool) -> Self {
        use ShortcutAction::*;
        Self(
            [
                (Home, "Home", false, true),
                (Following, "1", true, false),
                (Live, "2", true, false),
                (Categories, "3", true, false),
                (Search, "k", true, false),
                (Watching, "4", true, false),
                (OpenChannel, "5", true, false),
                (Bookmarks, "6", true, false),
                (Settings, ",", true, false),
                (Back, if mac { "[" } else { "ArrowLeft" }, mac, !mac),
                (Forward, if mac { "]" } else { "ArrowRight" }, mac, !mac),
                (Refresh, "r", true, false),
            ]
            .into_iter()
            .map(|(action, key, primary, alt)| {
                (
                    action,
                    Some(ShortcutBinding {
                        key: key.into(),
                        primary,
                        alt,
                        control: false,
                        shift: false,
                        meta: false,
                    }),
                )
            })
            .collect(),
        )
    }
    pub fn validate(&self) -> Result<()> {
        if self.0.len() != ACTIONS.len() || ACTIONS.iter().any(|a| !self.0.contains_key(a)) {
            return Err(settings_error(
                "Every supported shortcut action must be present.",
            ));
        }
        for mac in [false, true] {
            let mut seen = BTreeSet::new();
            for binding in self.0.values().flatten() {
                binding.validate()?;
                let control = binding.control || binding.primary && !mac;
                let meta = binding.meta || binding.primary && mac;
                if !seen.insert((&binding.key, control, meta, binding.alt, binding.shift)) {
                    return Err(settings_error("Shortcut bindings conflict."));
                }
            }
        }
        Ok(())
    }

    pub(super) fn expand_legacy_defaults(&mut self) {
        use ShortcutAction::*;
        // Only a complete pre-expansion map is eligible; malformed partial maps still fail validation.
        if self.0.len() != ACTIONS.len() - 2
            || ACTIONS.iter().any(|action| {
                !matches!(action, OpenChannel | Bookmarks) && !self.0.contains_key(action)
            })
        {
            return;
        }
        let defaults = Self::default();
        for action in [OpenChannel, Bookmarks] {
            let candidate = defaults.0[&action].as_ref().expect("default binding");
            let occupied = self.0.values().flatten().any(|existing| {
                existing.key == candidate.key
                    && [false, true].into_iter().any(|mac| {
                        let effective = |binding: &ShortcutBinding| {
                            (
                                binding.control || binding.primary && !mac,
                                binding.meta || binding.primary && mac,
                                binding.alt,
                                binding.shift,
                            )
                        };
                        effective(existing) == effective(candidate)
                    })
            });
            self.0.insert(
                action,
                if occupied {
                    None
                } else {
                    Some(candidate.clone())
                },
            );
        }
    }
}
impl ShortcutBinding {
    pub fn validate(&self) -> Result<()> {
        let key = self.key.as_str();
        let supported = key.len() == 1
            && key
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b",[]".contains(&c))
            || matches!(
                key,
                "ArrowLeft"
                    | "ArrowRight"
                    | "Home"
                    | "F6"
                    | "F7"
                    | "F8"
                    | "F9"
                    | "F10"
                    | "F11"
                    | "F12"
            );
        if !supported
            || !(self.primary || self.control || self.meta || self.alt)
            || self.primary && (self.control || self.meta)
            || self.control && self.meta
            || matches!(key, "q" | "w" | "n" | "t" | "l")
        {
            return Err(settings_error(
                "Unsupported or reserved shortcut. Choose a modified letter, number, comma, bracket, arrow, Home or F6–F12.",
            ));
        }
        Ok(())
    }
}
impl SettingsStore {
    pub fn set_shortcuts(&self, bindings: ShortcutBindings) -> Result<super::Settings> {
        bindings.validate()?;
        let mut value = self.value.lock().expect("settings mutex poisoned");
        let mut next = value.clone();
        next.settings.shortcuts = bindings;
        self.persist(&next)?;
        *value = next;
        Ok(value.settings.clone())
    }
}
