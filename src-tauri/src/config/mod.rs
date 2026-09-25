pub mod discovery;
pub mod profiles;
pub mod shortcuts;
use profiles::{PlayerProfile, ProfileMutation};
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

pub const SETTINGS_VERSION: u32 = 8;
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum UiLanguage {
    #[default]
    System,
    En,
    De,
    Es,
    Fr,
}

/// Curated Twitch discovery languages (ISO 639-1) plus Helix's `other` value.
/// None means Any language; these values are query data, never arbitrary parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum StreamLanguage {
    Ar,
    Bg,
    Cs,
    Da,
    De,
    El,
    En,
    Es,
    Fi,
    Fr,
    Hi,
    Hu,
    Id,
    It,
    Ja,
    Ko,
    Ms,
    Nl,
    No,
    Pl,
    Pt,
    Ro,
    Ru,
    Sk,
    Sv,
    Th,
    Tr,
    Uk,
    Vi,
    Zh,
    Other,
}
impl StreamLanguage {
    pub fn code(self) -> String {
        // Serialization is closed over the enum, never frontend-supplied text.
        serde_json::to_value(self)
            .expect("language enum")
            .as_str()
            .expect("language code")
            .into()
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum TextScale {
    #[default]
    #[serde(rename = "100")]
    Normal,
    #[serde(rename = "125")]
    Large,
    #[serde(rename = "150")]
    Largest,
}

/// Deliberately small supported polling policy; pause is operational, not persisted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackgroundSettings {
    pub monitoring_enabled: bool,
    pub notifications_enabled: bool,
    pub close_to_background: bool,
    pub interval_seconds: u32,
}
impl Default for BackgroundSettings {
    fn default() -> Self {
        Self {
            monitoring_enabled: false,
            notifications_enabled: false,
            close_to_background: false,
            interval_seconds: 60,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ChatProvider {
    #[default]
    Browser,
    Chatterino,
}

/// Global preferences only. Channel records have their own narrow update operation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub discovery: discovery::DiscoveryPreferences,
    pub shortcuts: shortcuts::ShortcutBindings,
    pub streamlink_path: Option<String>,
    pub player: PlayerSettings,
    pub default_quality: QualityPolicy,
    pub automatic_chat: bool,
    pub theme: Theme,
    pub ui_language: UiLanguage,
    pub background: BackgroundSettings,
    pub discovery_language: Option<StreamLanguage>,
    pub low_latency: bool,
    pub text_scale: TextScale,
    pub chat_provider: ChatProvider,
    pub chatterino_path: Option<String>,
    pub profiles: Vec<PlayerProfile>,
    pub selected_profile_id: Option<String>,
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
        if !matches!(self.background.interval_seconds, 60 | 120 | 300) {
            return Err(settings_error(
                "Choose a monitoring interval of 60, 120 or 300 seconds.",
            ));
        }
        if self.chatterino_path.as_ref().is_some_and(|p| {
            p.len() > 4096 || p.chars().any(char::is_control) || !Path::new(p).is_absolute()
        }) {
            return Err(settings_error(
                "Chatterino path must be an absolute executable path.",
            ));
        }
        self.discovery.validate()?;
        self.shortcuts.validate()?;
        self.validate_profiles()?;
        self.player.validate()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChannelOverrides {
    pub quality: Option<QualityPolicy>,
    pub automatic_chat: Option<bool>,
    pub notifications: Option<bool>,
    pub low_latency: Option<bool>,
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
    pub profile_id: Option<String>,
    pub chat_provider: ChatProvider,
    pub chatterino_path: Option<String>,
    pub streamlink_path: Option<String>,
    pub player: PlayerSettings,
    pub quality: QualityPolicy,
    pub automatic_chat: bool,
    pub low_latency: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChannelSettings {
    pub broadcaster_id: String,
    pub overrides: ChannelOverrides,
    pub default_quality: QualityPolicy,
    pub default_automatic_chat: bool,
    pub default_low_latency: bool,
    pub default_notifications: bool,
    pub effective_notifications: bool,
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
        let mut result = match value.get("version").and_then(|v| v.as_u64()) {
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
            Some(3) => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct OldSettings {
                    streamlink_path: Option<String>,
                    player: PlayerSettings,
                    default_quality: QualityPolicy,
                    automatic_chat: bool,
                    theme: Theme,
                }
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct OldOverrides {
                    quality: Option<QualityPolicy>,
                    automatic_chat: Option<bool>,
                }
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct Legacy {
                    version: u32,
                    settings: OldSettings,
                    channel_overrides: BTreeMap<String, OldOverrides>,
                }
                let old: Legacy = serde_json::from_value(value)
                    .map_err(|_| settings_error("Invalid version 3 settings."))?;
                debug_assert_eq!(old.version, 3);
                Self {
                    version: SETTINGS_VERSION,
                    settings: Settings {
                        streamlink_path: old.settings.streamlink_path,
                        player: old.settings.player,
                        default_quality: old.settings.default_quality,
                        automatic_chat: old.settings.automatic_chat,
                        theme: old.settings.theme,
                        ..Settings::default()
                    },
                    channel_overrides: old
                        .channel_overrides
                        .into_iter()
                        .map(|(id, value)| {
                            (
                                id,
                                ChannelOverrides {
                                    quality: value.quality,
                                    automatic_chat: value.automatic_chat,
                                    ..ChannelOverrides::default()
                                },
                            )
                        })
                        .collect(),
                }
            }
            Some(4) => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct OldSettings {
                    streamlink_path: Option<String>,
                    player: PlayerSettings,
                    default_quality: QualityPolicy,
                    automatic_chat: bool,
                    theme: Theme,
                    background: BackgroundSettings,
                }
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct OldOverrides {
                    quality: Option<QualityPolicy>,
                    automatic_chat: Option<bool>,
                    notifications: Option<bool>,
                }
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct Legacy {
                    version: u32,
                    settings: OldSettings,
                    channel_overrides: BTreeMap<String, OldOverrides>,
                }
                let old: Legacy = serde_json::from_value(value)
                    .map_err(|_| settings_error("Invalid version 4 settings."))?;
                debug_assert_eq!(old.version, 4);
                Self {
                    version: SETTINGS_VERSION,
                    settings: Settings {
                        streamlink_path: old.settings.streamlink_path,
                        player: old.settings.player,
                        default_quality: old.settings.default_quality,
                        automatic_chat: old.settings.automatic_chat,
                        theme: old.settings.theme,
                        background: old.settings.background,
                        ..Settings::default()
                    },
                    channel_overrides: old
                        .channel_overrides
                        .into_iter()
                        .map(|(id, old)| {
                            let value = ChannelOverrides {
                                quality: old.quality,
                                automatic_chat: old.automatic_chat,
                                notifications: old.notifications,
                                ..ChannelOverrides::default()
                            };
                            (id, value)
                        })
                        .collect(),
                }
            }
            Some(5) => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct OldSettings {
                    streamlink_path: Option<String>,
                    player: PlayerSettings,
                    default_quality: QualityPolicy,
                    automatic_chat: bool,
                    theme: Theme,
                    background: BackgroundSettings,
                    discovery_language: Option<StreamLanguage>,
                    low_latency: bool,
                    text_scale: TextScale,
                }
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct Legacy {
                    version: u32,
                    settings: OldSettings,
                    channel_overrides: BTreeMap<String, ChannelOverrides>,
                }
                let old: Legacy = serde_json::from_value(value)
                    .map_err(|_| settings_error("Invalid version 5 settings."))?;
                debug_assert_eq!(old.version, 5);
                Self {
                    version: SETTINGS_VERSION,
                    settings: Settings {
                        streamlink_path: old.settings.streamlink_path,
                        player: old.settings.player,
                        default_quality: old.settings.default_quality,
                        automatic_chat: old.settings.automatic_chat,
                        theme: old.settings.theme,
                        background: old.settings.background,
                        discovery_language: old.settings.discovery_language,
                        low_latency: old.settings.low_latency,
                        text_scale: old.settings.text_scale,
                        ..Settings::default()
                    },
                    channel_overrides: old.channel_overrides,
                }
            }
            Some(6) => {
                let mut value = value;
                let fields = value
                    .get_mut("settings")
                    .and_then(|s| s.as_object_mut())
                    .ok_or_else(|| settings_error("Invalid version 6 settings."))?;
                if fields.contains_key("discovery")
                    || fields.contains_key("shortcuts")
                    || fields.contains_key("uiLanguage")
                {
                    return Err(settings_error("Invalid version 6 settings."));
                }
                fields.insert(
                    "discovery".into(),
                    serde_json::to_value(discovery::DiscoveryPreferences::default())
                        .expect("discovery defaults"),
                );
                fields.insert(
                    "shortcuts".into(),
                    serde_json::to_value(shortcuts::ShortcutBindings::default())
                        .expect("shortcut defaults"),
                );
                fields.insert("uiLanguage".into(), serde_json::json!("system"));
                value["version"] = SETTINGS_VERSION.into();
                serde_json::from_value(value)
                    .map_err(|_| settings_error("Invalid version 6 settings."))?
            }
            Some(7) => {
                let mut value = value;
                let fields = value
                    .get_mut("settings")
                    .and_then(|s| s.as_object_mut())
                    .ok_or_else(|| settings_error("Invalid version 7 settings."))?;
                if fields.contains_key("uiLanguage") {
                    return Err(settings_error("Invalid version 7 settings."));
                }
                fields.insert("uiLanguage".into(), serde_json::json!("system"));
                value["version"] = SETTINGS_VERSION.into();
                serde_json::from_value(value)
                    .map_err(|_| settings_error("Invalid version 7 settings."))?
            }
            Some(8) => serde_json::from_value(value).map_err(|_| {
                settings_error("Settings schema is invalid; the file was not changed.")
            })?,
            _ => {
                return Err(AppError::new(
                    ErrorCode::SettingsVersion,
                    "Unsupported or missing settings version; the file was not changed.",
                ));
            }
        };
        result.settings.shortcuts.expand_legacy_defaults();
        result.validate()?;
        result
            .channel_overrides
            .retain(|_, value| *value != ChannelOverrides::default());
        Ok(result)
    }
    fn effective(&self, id: &str, quality: Option<QualityPolicy>) -> EffectivePlaybackSettings {
        let overrides = self.channel_overrides.get(id).cloned().unwrap_or_default();
        let profile = self.settings.selected_profile();
        EffectivePlaybackSettings {
            profile_id: profile.map(|p| p.id.clone()),
            chat_provider: self.settings.chat_provider,
            chatterino_path: self.settings.chatterino_path.clone(),
            streamlink_path: self.settings.streamlink_path.clone(),
            player: profile
                .map(|p| &p.player)
                .unwrap_or(&self.settings.player)
                .clone(),
            quality: quality
                .or(overrides.quality)
                .or(profile.and_then(|p| p.quality))
                .unwrap_or(self.settings.default_quality),
            low_latency: overrides
                .low_latency
                .or(profile.and_then(|p| p.low_latency))
                .unwrap_or(self.settings.low_latency),
            automatic_chat: overrides
                .automatic_chat
                .unwrap_or(self.settings.automatic_chat),
        }
    }
    fn notifications(&self, id: &str) -> bool {
        self.channel_overrides
            .get(id)
            .and_then(|v| v.notifications)
            .unwrap_or(self.settings.background.notifications_enabled)
    }
    fn channel(&self, id: &str) -> ChannelSettings {
        ChannelSettings {
            broadcaster_id: id.into(),
            overrides: self.channel_overrides.get(id).cloned().unwrap_or_default(),
            default_quality: self
                .settings
                .selected_profile()
                .and_then(|p| p.quality)
                .unwrap_or(self.settings.default_quality),
            default_automatic_chat: self.settings.automatic_chat,
            default_low_latency: self
                .settings
                .selected_profile()
                .and_then(|p| p.low_latency)
                .unwrap_or(self.settings.low_latency),
            default_notifications: self.settings.background.notifications_enabled,
            effective_notifications: self.notifications(id),
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
    pub fn notifications(&self, id: &str) -> bool {
        self.value
            .lock()
            .expect("settings mutex poisoned")
            .notifications(id)
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
    pub fn set_discovery_language(&self, language: Option<StreamLanguage>) -> Result<Settings> {
        let mut value = self.value.lock().expect("settings mutex poisoned");
        let mut next = value.clone();
        next.settings.discovery_language = language;
        self.persist(&next)?;
        *value = next;
        Ok(value.settings.clone())
    }
    pub fn set_ui_language(&self, language: UiLanguage) -> Result<Settings> {
        let mut value = self.value.lock().expect("settings mutex poisoned");
        let mut next = value.clone();
        next.settings.ui_language = language;
        self.persist(&next)?;
        *value = next;
        Ok(value.settings.clone())
    }
    pub fn set_streamlink_path(&self, path: Option<String>) -> Result<()> {
        let mut value = self.value.lock().expect("settings mutex poisoned");
        let mut next = value.clone();
        next.settings.streamlink_path = path;
        self.persist(&next)?;
        *value = next;
        Ok(())
    }
    pub fn modify_profile(&self, request: ProfileMutation) -> Result<Settings> {
        let mut value = self.value.lock().expect("settings mutex poisoned");
        let mut next = value.clone();
        next.settings.modify_profile(request)?;
        self.persist(&next)?;
        *value = next;
        Ok(value.settings.clone())
    }
    pub fn update(&self, mut settings: Settings) -> Result<Settings> {
        let mut value = self.value.lock().expect("settings mutex poisoned");
        // These collections/references are read-only in global settings IPC.
        // Preserve them at the write boundary too, including cancelled callers.
        settings.discovery = value.settings.discovery.clone();
        settings.ui_language = value.settings.ui_language;
        settings.shortcuts = value.settings.shortcuts.clone();
        settings.profiles = value.settings.profiles.clone();
        settings.selected_profile_id = value.settings.selected_profile_id.clone();
        settings.validate()?;
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
