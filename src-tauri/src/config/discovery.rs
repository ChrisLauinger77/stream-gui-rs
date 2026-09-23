//! Bounded local preferences. IDs are identity; labels are presentation only.
use super::{SettingsStore, settings_error, validate_broadcaster_id};
use crate::domain::Result;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const MAX_SAVED_ITEMS: usize = 200;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Channel,
    Category,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedItem {
    pub kind: ItemKind,
    pub id: String,
    pub name: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiscoveryPreferences {
    pub bookmarks: Vec<SavedItem>,
    pub hidden: Vec<SavedItem>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryList {
    Bookmarks,
    Hidden,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiscoveryMutation {
    pub list: DiscoveryList,
    pub item: SavedItem,
    pub present: bool,
}
impl SavedItem {
    fn validate(&self) -> Result<()> {
        validate_broadcaster_id(&self.id)?;
        if self.name.is_empty()
            || self.name.trim() != self.name
            || self.name.len() > 256
            || self.name.chars().count() > 128
            || self.name.chars().any(|c| {
                c.is_control() || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
            })
        {
            return Err(settings_error(
                "Saved item labels must contain 1–128 visible characters (256 bytes).",
            ));
        }
        Ok(())
    }
}
impl DiscoveryPreferences {
    pub fn validate(&self) -> Result<()> {
        for list in [&self.bookmarks, &self.hidden] {
            if list.len() > MAX_SAVED_ITEMS {
                return Err(settings_error(
                    "Each discovery list supports at most 200 items.",
                ));
            }
            let mut ids = std::collections::BTreeSet::new();
            for item in list {
                item.validate()?;
                if !ids.insert((item.kind, &item.id)) {
                    return Err(settings_error("Saved items must have unique IDs."));
                }
            }
        }
        Ok(())
    }
}
impl SettingsStore {
    pub fn modify_discovery(&self, request: DiscoveryMutation) -> Result<super::Settings> {
        request.item.validate()?;
        let mut value = self.value.lock().expect("settings mutex poisoned");
        let mut next = value.clone();
        let list = match request.list {
            DiscoveryList::Bookmarks => &mut next.settings.discovery.bookmarks,
            DiscoveryList::Hidden => &mut next.settings.discovery.hidden,
        };
        let index = list
            .iter()
            .position(|item| item.kind == request.item.kind && item.id == request.item.id);
        match (request.present, index) {
            (true, Some(index)) => list[index] = request.item,
            (true, None) => list.push(request.item),
            (false, Some(index)) => {
                list.remove(index);
            }
            (false, None) => {}
        }
        self.persist(&next)?;
        *value = next;
        Ok(value.settings.clone())
    }
}
