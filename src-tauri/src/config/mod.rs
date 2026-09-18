use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::{AppError, ErrorCode, Result};

pub const SETTINGS_VERSION: u32 = 2;
use crate::streamlink::playback::{PlayerSettings, QualityPolicy};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub version: u32,
    pub streamlink_path: Option<String>,
    pub player: PlayerSettings,
    pub default_quality: QualityPolicy,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            streamlink_path: None,
            player: PlayerSettings::default(),
            default_quality: QualityPolicy::Source,
        }
    }
}

impl Settings {
    pub fn validate(&self) -> Result<()> {
        if self.version != SETTINGS_VERSION {
            return Err(AppError::new(
                ErrorCode::SettingsVersion,
                "Unsupported settings version.",
            ));
        }
        if self.streamlink_path.as_ref().is_some_and(|p| {
            p.len() > 4096 || p.chars().any(char::is_control) || !Path::new(p).is_absolute()
        }) {
            return Err(settings_error(
                "Streamlink path must be an absolute executable path.",
            ));
        }
        self.player.validate()
    }
    pub fn from_json(input: &str) -> Result<Self> {
        let mut value: serde_json::Value = serde_json::from_str(input).map_err(|_| {
            settings_error("Settings contain invalid JSON; the file was not changed.")
        })?;
        // The sole migration is our own Phase 0 schema; unknown versions stay intact.
        if value.get("version").and_then(|v| v.as_u64()) == Some(1) {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase", deny_unknown_fields)]
            struct VersionOne {
                version: u32,
                streamlink_path: Option<String>,
            }
            let old: VersionOne = serde_json::from_value(value)
                .map_err(|_| settings_error("Invalid version 1 settings."))?;
            debug_assert_eq!(old.version, 1);
            value = serde_json::to_value(Self {
                streamlink_path: old.streamlink_path,
                ..Self::default()
            })
            .expect("settings serialization");
        }
        if value.get("version").and_then(|v| v.as_u64()) != Some(SETTINGS_VERSION.into()) {
            return Err(AppError::new(
                ErrorCode::SettingsVersion,
                "Unsupported or missing settings version; the file was not changed.",
            ));
        }
        serde_json::from_value(value)
            .map_err(|_| settings_error("Settings schema is invalid; the file was not changed."))
    }
}

pub struct SettingsStore {
    path: PathBuf,
    value: Mutex<Settings>,
}

impl SettingsStore {
    pub fn open(directory: &Path) -> Result<Self> {
        let path = directory.join("settings.json");
        let settings = match fs::read_to_string(&path) {
            Ok(text) => Settings::from_json(&text)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            Err(_) => return Err(settings_error("Could not read the application settings.")),
        };
        Ok(Self {
            path,
            value: Mutex::new(settings),
        })
    }

    pub fn snapshot(&self) -> Settings {
        self.value.lock().expect("settings mutex poisoned").clone()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Persist only after successful executable validation. tempfile::persist
    /// replaces atomically on Unix and Windows; no remove-then-rename gap.
    pub fn set_streamlink_path(&self, path: Option<String>) -> Result<()> {
        let mut value = self.value.lock().expect("settings mutex poisoned");
        let next = Settings {
            streamlink_path: path,
            ..value.clone()
        };
        self.persist(&next)?;
        *value = next;
        Ok(())
    }

    pub fn update(&self, next: Settings) -> Result<Settings> {
        next.validate()?;
        let mut value = self.value.lock().expect("settings mutex poisoned");
        self.persist(&next)?;
        *value = next.clone();
        Ok(next)
    }

    fn persist(&self, next: &Settings) -> Result<()> {
        let directory = self.path.parent().expect("settings parent");
        fs::create_dir_all(directory)
            .map_err(|_| settings_error("Could not create the application settings directory."))?;
        let mut temporary = tempfile::NamedTempFile::new_in(directory)
            .map_err(|_| settings_error("Could not create a temporary settings file."))?;
        serde_json::to_writer_pretty(&mut temporary, &next)
            .map_err(|_| settings_error("Could not serialize settings."))?;
        temporary
            .write_all(b"\n")
            .and_then(|_| temporary.as_file().sync_all())
            .map_err(|_| settings_error("Could not write settings."))?;
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
mod tests {
    use super::*;

    #[test]
    fn settings_version_and_round_trip() {
        let value = Settings {
            streamlink_path: Some("/path with spaces/streamlink".into()),
            ..Settings::default()
        };
        assert_eq!(
            Settings::from_json(&serde_json::to_string(&value).unwrap()).unwrap(),
            value
        );
        for text in ["{}", r#"{"version":3}"#, r#"{"version":0}"#] {
            assert_eq!(
                Settings::from_json(text).unwrap_err().code,
                ErrorCode::SettingsVersion
            );
        }
        assert_eq!(
            Settings::from_json("broken").unwrap_err().code,
            ErrorCode::Settings
        );
    }

    #[test]
    fn persist_replace_clear_and_preserve_future_versions() {
        let temp = tempfile::tempdir().unwrap();
        let store = SettingsStore::open(temp.path()).unwrap();
        assert!(!store.path().exists());
        store.set_streamlink_path(Some("/one".into())).unwrap();
        store.set_streamlink_path(Some("/two".into())).unwrap();
        assert_eq!(
            SettingsStore::open(temp.path())
                .unwrap()
                .snapshot()
                .streamlink_path
                .as_deref(),
            Some("/two")
        );
        store.set_streamlink_path(None).unwrap();
        assert_eq!(
            SettingsStore::open(temp.path()).unwrap().snapshot(),
            Settings::default()
        );
        fs::write(store.path(), r#"{"version":99}"#).unwrap();
        assert!(SettingsStore::open(temp.path()).is_err());
        assert_eq!(
            fs::read_to_string(store.path()).unwrap(),
            r#"{"version":99}"#
        );
    }
    #[test]
    fn migrates_only_our_version_one_path_and_roundtrips_playback_settings() {
        let root = tempfile::tempdir().unwrap();
        let path = root
            .path()
            .join("streamlink")
            .to_string_lossy()
            .into_owned();
        fs::write(
            root.path().join("settings.json"),
            serde_json::json!({"version":1,"streamlinkPath":path}).to_string(),
        )
        .unwrap();
        let store = SettingsStore::open(root.path()).unwrap();
        let mut next = store.snapshot();
        assert_eq!(next.version, 2);
        assert_eq!(next.streamlink_path.as_deref(), Some(path.as_str()));
        assert!(
            fs::read_to_string(store.path())
                .unwrap()
                .contains("\"version\":1")
        );
        next.default_quality = QualityPolicy::Audio;
        next.player = PlayerSettings {
            mode: crate::streamlink::playback::PlayerMode::Mpv,
            executable: None,
            arguments: vec!["--volume=25".into(), "literal spaces".into(), String::new()],
        };
        store.update(next.clone()).unwrap();
        assert_eq!(SettingsStore::open(root.path()).unwrap().snapshot(), next);
        let before = fs::read_to_string(store.path()).unwrap();
        next.player.arguments.push("bad\0argument".into());
        assert!(store.update(next).is_err());
        assert_eq!(fs::read_to_string(store.path()).unwrap(), before);
        assert!(
            Settings::from_json(r#"{"version":1,"streamlinkPath":null,"session":{}}"#).is_err()
        );
    }
}
