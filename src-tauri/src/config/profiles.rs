//! Named player configurations. Resolution and references stay in SettingsStore.
use super::{Settings, settings_error};
use crate::{
    domain::Result,
    streamlink::playback::{PlayerSettings, QualityPolicy},
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const MAX_PROFILES: usize = 16;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlayerProfile {
    pub id: String,
    pub name: String,
    pub player: PlayerSettings,
    pub quality: Option<QualityPolicy>,
    pub low_latency: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileDraft {
    pub name: String,
    pub player: PlayerSettings,
    pub quality: Option<QualityPolicy>,
    pub low_latency: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ProfileMutation {
    Create { profile: ProfileDraft },
    Update { id: String, profile: ProfileDraft },
    Delete { id: String },
    Select { id: Option<String> },
}
impl Settings {
    pub fn selected_profile(&self) -> Option<&PlayerProfile> {
        self.profiles
            .iter()
            .find(|p| Some(&p.id) == self.selected_profile_id.as_ref())
    }
    pub(super) fn validate_profiles(&self) -> Result<()> {
        if self.profiles.len() > MAX_PROFILES {
            return Err(settings_error("At most 16 player profiles are supported."));
        }
        let mut ids = std::collections::HashSet::new();
        let mut names = std::collections::HashSet::new();
        for profile in &self.profiles {
            if uuid::Uuid::parse_str(&profile.id).is_err()
                || profile.id.len() != 36
                || !ids.insert(&profile.id)
                || profile.name.trim() != profile.name
                || profile.name.is_empty()
                || profile.name.len() > 128
                || profile.name.chars().count() > 64
                || profile.name.chars().any(char::is_control)
                || !names.insert(profile.name.to_lowercase())
            {
                return Err(settings_error(
                    "Profile IDs and names must be valid and unique; names allow 64 characters (128 bytes).",
                ));
            }
            profile.player.validate()?;
        }
        if self.selected_profile_id.is_some() && self.selected_profile().is_none() {
            return Err(settings_error(
                "The selected player profile does not exist.",
            ));
        }
        Ok(())
    }
    pub(super) fn modify_profile(&mut self, mutation: ProfileMutation) -> Result<()> {
        match mutation {
            ProfileMutation::Create { profile } => self.profiles.push(PlayerProfile {
                id: uuid::Uuid::new_v4().to_string(),
                name: profile.name,
                player: profile.player,
                quality: profile.quality,
                low_latency: profile.low_latency,
            }),
            ProfileMutation::Update { id, profile } => {
                let target = self
                    .profiles
                    .iter_mut()
                    .find(|p| p.id == id)
                    .ok_or_else(|| settings_error("The player profile no longer exists."))?;
                *target = PlayerProfile {
                    id,
                    name: profile.name,
                    player: profile.player,
                    quality: profile.quality,
                    low_latency: profile.low_latency,
                };
            }
            ProfileMutation::Delete { id } => {
                if !self.profiles.iter().any(|p| p.id == id) {
                    return Err(settings_error("The player profile no longer exists."));
                }
                self.profiles.retain(|p| p.id != id);
                if self.selected_profile_id.as_ref() == Some(&id) {
                    self.selected_profile_id = None;
                }
            }
            ProfileMutation::Select { id } => self.selected_profile_id = id,
        }
        self.validate_profiles()
    }
}
