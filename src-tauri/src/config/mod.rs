#[cfg(any(feature = "desktop", test))]
mod client_id_value;
#[cfg(any(feature = "desktop", test))]
pub(crate) mod twitch_client_id;

use crate::{
    domain::{AppError, ErrorCode, Result},
    streamlink::playback::{PlayerSettings, QualityPolicy},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};
use ts_rs::TS;

pub const SETTINGS_VERSION: u32 = 3;
const MAX_SETTINGS_BYTES: u64 = 256 * 1024;
const MAX_CHANNEL_OVERRIDES: usize = 1000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

/// Global preferences only. Channel records have their own narrow update operation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub streamlink_path: Option<String>,
    pub player: PlayerSettings,
    pub default_quality: QualityPolicy,
    pub automatic_chat: bool,
    pub theme: Theme,
}
impl Settings {
    pub fn validate(&self) -> Result<()> {
        if self.streamlink_path.as_ref().is_some_and(|p| {
            p.len() > 4096 || p.chars().any(char::is_control) || !Path::new(p).is_absolute()
        }) {
            return Err(settings_error(
                "Streamlink path must be an absolute executable path.",
            ));
        }
        self.player.validate()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChannelOverrides {
    pub quality: Option<QualityPolicy>,
    pub automatic_chat: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChannelSettingsRequest {
    pub broadcaster_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveChannelSettingsRequest {
    pub broadcaster_id: String,
    pub overrides: ChannelOverrides,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EffectivePlaybackSettings {
    pub streamlink_path: Option<String>,
    pub player: PlayerSettings,
    pub quality: QualityPolicy,
    pub automatic_chat: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChannelSettings {
    pub broadcaster_id: String,
    pub overrides: ChannelOverrides,
    pub default_quality: QualityPolicy,
    pub default_automatic_chat: bool,
    pub effective: EffectivePlaybackSettings,
}
pub fn validate_broadcaster_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 32
        || !id.bytes().all(|c| c.is_ascii_digit())
        || id.starts_with('0')
    {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Invalid Twitch broadcaster ID.",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SettingsDocument {
    version: u32,
    settings: Settings,
    channel_overrides: BTreeMap<String, ChannelOverrides>,
}
impl Default for SettingsDocument {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            settings: Settings::default(),
            channel_overrides: BTreeMap::new(),
        }
    }
}
impl SettingsDocument {
    fn validate(&self) -> Result<()> {
        if self.version != SETTINGS_VERSION {
            return Err(AppError::new(
                ErrorCode::SettingsVersion,
                "Unsupported settings version; the file was not changed.",
            ));
        }
        self.settings.validate()?;
        if self.channel_overrides.len() > MAX_CHANNEL_OVERRIDES {
            return Err(settings_error("Too many channel overrides."));
        }
        for id in self.channel_overrides.keys() {
            validate_broadcaster_id(id)?;
        }
        Ok(())
    }
    fn from_json(input: &str) -> Result<Self> {
        let value: serde_json::Value = serde_json::from_str(input).map_err(|_| {
            settings_error("Settings contain invalid JSON; the file was not changed.")
        })?;
        // Migrate only this application's schemas, in memory until the next save.
        let result = match value.get("version").and_then(|v| v.as_u64()) {
            Some(1) => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct Legacy {
                    version: u32,
                    streamlink_path: Option<String>,
                }
                let old: Legacy = serde_json::from_value(value)
                    .map_err(|_| settings_error("Invalid version 1 settings."))?;
                debug_assert_eq!(old.version, 1);
                Self {
                    settings: Settings {
                        streamlink_path: old.streamlink_path,
                        ..Settings::default()
                    },
                    ..Self::default()
                }
            }
            Some(2) => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct Legacy {
                    version: u32,
                    streamlink_path: Option<String>,
                    player: PlayerSettings,
                    default_quality: QualityPolicy,
                }
                let old: Legacy = serde_json::from_value(value)
                    .map_err(|_| settings_error("Invalid version 2 settings."))?;
                debug_assert_eq!(old.version, 2);
                Self {
                    settings: Settings {
                        streamlink_path: old.streamlink_path,
                        player: old.player,
                        default_quality: old.default_quality,
                        ..Settings::default()
                    },
                    ..Self::default()
                }
            }
            Some(3) => serde_json::from_value(value).map_err(|_| {
                settings_error("Settings schema is invalid; the file was not changed.")
            })?,
            _ => {
                return Err(AppError::new(
                    ErrorCode::SettingsVersion,
                    "Unsupported or missing settings version; the file was not changed.",
                ));
            }
        };
        result.validate()?;
        Ok(result)
    }
    fn effective(&self, id: &str, quality: Option<QualityPolicy>) -> EffectivePlaybackSettings {
        let overrides = self.channel_overrides.get(id).cloned().unwrap_or_default();
        EffectivePlaybackSettings {
            streamlink_path: self.settings.streamlink_path.clone(),
            player: self.settings.player.clone(),
            quality: quality
                .or(overrides.quality)
                .unwrap_or(self.settings.default_quality),
            automatic_chat: overrides
                .automatic_chat
                .unwrap_or(self.settings.automatic_chat),
        }
    }
    fn channel(&self, id: &str) -> ChannelSettings {
        ChannelSettings {
            broadcaster_id: id.into(),
            overrides: self.channel_overrides.get(id).cloned().unwrap_or_default(),
            default_quality: self.settings.default_quality,
            default_automatic_chat: self.settings.automatic_chat,
            effective: self.effective(id, None),
        }
    }
}

pub struct SettingsStore {
    path: PathBuf,
    value: Mutex<SettingsDocument>,
}
impl SettingsStore {
    pub fn open(directory: &Path) -> Result<Self> {
        let path = directory.join("settings.json");
        let settings = match fs::File::open(&path) {
            Ok(file) => {
                let mut text = String::new();
                file.take(MAX_SETTINGS_BYTES + 1)
                    .read_to_string(&mut text)
                    .map_err(|_| settings_error("Could not read the application settings."))?;
                if text.len() as u64 > MAX_SETTINGS_BYTES {
                    return Err(settings_error(
                        "Settings file is too large; it was not changed.",
                    ));
                }
                SettingsDocument::from_json(&text)?
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => SettingsDocument::default(),
            Err(_) => return Err(settings_error("Could not read the application settings.")),
        };
        Ok(Self {
            path,
            value: Mutex::new(settings),
        })
    }
    pub fn snapshot(&self) -> Settings {
        self.value
            .lock()
            .expect("settings mutex poisoned")
            .settings
            .clone()
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn channel(&self, id: &str) -> Result<ChannelSettings> {
        validate_broadcaster_id(id)?;
        Ok(self
            .value
            .lock()
            .expect("settings mutex poisoned")
            .channel(id))
    }
    pub fn effective(
        &self,
        id: &str,
        quality: Option<QualityPolicy>,
    ) -> Result<EffectivePlaybackSettings> {
        validate_broadcaster_id(id)?;
        Ok(self
            .value
            .lock()
            .expect("settings mutex poisoned")
            .effective(id, quality))
    }
    pub fn set_channel(&self, request: SaveChannelSettingsRequest) -> Result<ChannelSettings> {
        validate_broadcaster_id(&request.broadcaster_id)?;
        let mut value = self.value.lock().expect("settings mutex poisoned");
        let mut next = value.clone();
        if request.overrides == ChannelOverrides::default() {
            next.channel_overrides.remove(&request.broadcaster_id);
        } else {
            next.channel_overrides
                .insert(request.broadcaster_id.clone(), request.overrides);
        }
        self.persist(&next)?;
        *value = next;
        Ok(value.channel(&request.broadcaster_id))
    }
    pub fn set_streamlink_path(&self, path: Option<String>) -> Result<()> {
        let mut value = self.value.lock().expect("settings mutex poisoned");
        let mut next = value.clone();
        next.settings.streamlink_path = path;
        self.persist(&next)?;
        *value = next;
        Ok(())
    }
    pub fn update(&self, settings: Settings) -> Result<Settings> {
        settings.validate()?;
        let mut value = self.value.lock().expect("settings mutex poisoned");
        let next = SettingsDocument {
            settings: settings.clone(),
            ..value.clone()
        };
        self.persist(&next)?;
        *value = next;
        Ok(settings)
    }
    fn persist(&self, next: &SettingsDocument) -> Result<()> {
        next.validate()?;
        let data = serde_json::to_vec_pretty(next)
            .map_err(|_| settings_error("Could not serialize settings."))?;
        if data.len() as u64 + 1 > MAX_SETTINGS_BYTES {
            return Err(settings_error("Settings file would be too large."));
        }
        let directory = self.path.parent().expect("settings parent");
        fs::create_dir_all(directory)
            .map_err(|_| settings_error("Could not create the application settings directory."))?;
        let mut temporary = tempfile::NamedTempFile::new_in(directory)
            .map_err(|_| settings_error("Could not create a temporary settings file."))?;
        temporary
            .write_all(&data)
            .and_then(|_| temporary.write_all(b"\n"))
            .and_then(|_| temporary.as_file().sync_all())
            .map_err(|_| settings_error("Could not write settings."))?;
        // Replace atomically on Unix and Windows; never remove the old file first.
        temporary
            .persist(&self.path)
            .map_err(|_| settings_error("Could not replace the settings file."))?;
        Ok(())
    }
}
fn settings_error(message: &str) -> AppError {
    AppError::new(ErrorCode::Settings, message)
}
#[cfg(test)]
mod tests;
