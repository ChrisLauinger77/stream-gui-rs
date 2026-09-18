use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::{AppError, ErrorCode, Result};

pub const SETTINGS_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub version: u32,
    pub streamlink_path: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            streamlink_path: None,
        }
    }
}

impl Settings {
    pub fn from_json(input: &str) -> Result<Self> {
        let value: serde_json::Value = serde_json::from_str(input).map_err(|_| {
            settings_error("Settings contain invalid JSON; the file was not changed.")
        })?;
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
        *value = next;
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
            version: 1,
            streamlink_path: Some("/path with spaces/streamlink".into()),
        };
        assert_eq!(
            Settings::from_json(&serde_json::to_string(&value).unwrap()).unwrap(),
            value
        );
        for text in ["{}", r#"{"version":2}"#, r#"{"version":0}"#] {
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
}
