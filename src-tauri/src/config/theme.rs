use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, path::Path};
use ts_rs::TS;

const MAX_CUSTOM_THEME_BYTES: u64 = 16 * 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum ThemePalette {
    #[default]
    Default,
    Catppuccin,
    Dracula,
    Everforest,
    Gruvbox,
    Nord,
    RosePine,
    Solarized,
    TokyoNight,
    Custom,
}

/// Only complete hexadecimal colour palettes cross the IPC boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SchemeColours {
    pub base: String,
    pub mantle: String,
    pub crust: String,
    pub surface0: String,
    pub surface1: String,
    pub surface2: String,
    pub overlay: String,
    pub text: String,
    pub subtext1: String,
    pub subtext0: String,
    pub accent: String,
    pub accent_hover: String,
}

impl SchemeColours {
    fn valid(&self) -> bool {
        [
            &self.base,
            &self.mantle,
            &self.crust,
            &self.surface0,
            &self.surface1,
            &self.surface2,
            &self.overlay,
            &self.text,
            &self.subtext1,
            &self.subtext0,
            &self.accent,
            &self.accent_hover,
        ]
        .into_iter()
        .all(|colour| {
            colour.len() == 7
                && colour.starts_with('#')
                && colour[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CustomTheme {
    pub dark: SchemeColours,
    pub light: SchemeColours,
}

#[derive(Deserialize)]
struct CustomThemeFile {
    dark: SchemeColours,
    light: Option<SchemeColours>,
}

pub fn parse_custom_theme(input: &str) -> Option<CustomTheme> {
    let value: serde_json::Value = serde_json::from_str(input).ok()?;
    if value.get("light").is_some_and(|light| !light.is_object()) {
        return None;
    }
    let file: CustomThemeFile = serde_json::from_value(value).ok()?;
    if !file.dark.valid() || file.light.as_ref().is_some_and(|light| !light.valid()) {
        return None;
    }
    Some(CustomTheme {
        light: file.light.unwrap_or_else(|| file.dark.clone()),
        dark: file.dark,
    })
}

pub fn read_custom_theme(directory: &Path) -> Option<CustomTheme> {
    let path = directory.join("custom-theme.json");
    let metadata = std::fs::metadata(&path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_CUSTOM_THEME_BYTES {
        return None;
    }
    let mut input = String::new();
    File::open(path)
        .ok()?
        .take(MAX_CUSTOM_THEME_BYTES + 1)
        .read_to_string(&mut input)
        .ok()?;
    if input.len() as u64 > MAX_CUSTOM_THEME_BYTES {
        return None;
    }
    parse_custom_theme(&input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> String {
        serde_json::json!({"dark":{
            "base":"#1e1e2e","mantle":"#181825","crust":"#11111b","surface0":"#313244",
            "surface1":"#45475a","surface2":"#585b70","overlay":"#6c7086","text":"#cdd6f4",
            "subtext1":"#bac2de","subtext0":"#a6adc8","accent":"#f38ba8","accentHover":"#eba0ac"
        }})
        .to_string()
    }

    #[test]
    fn custom_theme_requires_complete_hex_dark_and_optional_complete_light() {
        let parsed = parse_custom_theme(&valid()).unwrap();
        assert_eq!(parsed.light, parsed.dark);
        assert!(parse_custom_theme(include_str!("../../../docs/custom-theme.json")).is_some());
        let mut value: serde_json::Value = serde_json::from_str(&valid()).unwrap();
        value["light"] = value["dark"].clone();
        value["light"]["base"] = "#FFFFFF".into();
        assert_eq!(
            parse_custom_theme(&value.to_string()).unwrap().light.base,
            "#FFFFFF"
        );
        value["light"]["accent"] = "red".into();
        assert!(parse_custom_theme(&value.to_string()).is_none());
        value["light"] = serde_json::Value::Null;
        assert!(parse_custom_theme(&value.to_string()).is_none());
        value.as_object_mut().unwrap().remove("light");
        value["dark"].as_object_mut().unwrap().remove("accentHover");
        assert!(parse_custom_theme(&value.to_string()).is_none());
    }

    #[test]
    fn custom_file_missing_invalid_or_oversized_is_unavailable() {
        let root = tempfile::tempdir().unwrap();
        assert!(read_custom_theme(root.path()).is_none());
        let path = root.path().join("custom-theme.json");
        std::fs::write(&path, valid()).unwrap();
        assert!(read_custom_theme(root.path()).is_some());
        std::fs::write(&path, "x".repeat(MAX_CUSTOM_THEME_BYTES as usize + 1)).unwrap();
        assert!(read_custom_theme(root.path()).is_none());
        std::fs::write(&path, "not json").unwrap();
        assert!(read_custom_theme(root.path()).is_none());
    }
}
