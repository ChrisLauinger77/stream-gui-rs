//! Small native executable search, shared by Streamlink and player settings.
use super::{
    playback::{PlayerMode, PlayerSettings},
    validate_executable,
};
use crate::domain::{AppError, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};
use ts_rs::TS;

#[derive(Clone)]
pub struct SearchLocations {
    pub os: String,
    pub path: Option<OsString>,
    pub home: Option<PathBuf>,
    pub program_files: Vec<PathBuf>,
    pub local_app_data: Option<PathBuf>,
}
impl SearchLocations {
    pub fn system() -> Self {
        Self {
            os: std::env::consts::OS.into(),
            path: std::env::var_os("PATH"),
            home: std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                .map(PathBuf::from),
            program_files: ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"]
                .iter()
                .filter_map(std::env::var_os)
                .map(PathBuf::from)
                .collect(),
            local_app_data: std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
        }
    }
    pub fn candidates(&self, name: &str) -> Vec<PathBuf> {
        let executable = if self.os == "windows" {
            format!("{name}.exe")
        } else {
            name.into()
        };
        let mut candidates: Vec<_> = self
            .path
            .as_ref()
            .map(|path| {
                std::env::split_paths(path)
                    .filter(|p| p.is_absolute())
                    .map(|p| p.join(&executable))
                    .take(128)
                    .collect()
            })
            .unwrap_or_default();
        if self.os == "windows" {
            let subdirs: &[&str] = match name {
                "streamlink" => &["Streamlink/bin", "Streamlink"],
                "vlc" => &["VideoLAN/VLC"],
                "mpv" => &["mpv"],
                "chatterino" => &["Chatterino", "Chatterino2"],
                _ => &[],
            };
            for base in self.program_files.iter().chain(self.local_app_data.iter()) {
                for subdir in subdirs {
                    candidates.push(base.join(subdir).join(&executable));
                }
            }
            if let Some(local) = &self.local_app_data {
                for subdir in subdirs {
                    candidates.push(local.join("Programs").join(subdir).join(&executable));
                }
            }
        } else {
            if self.os == "macos" {
                candidates.push(PathBuf::from("/opt/homebrew/bin").join(&executable));
            }
            for base in ["/usr/local/bin", "/usr/bin", "/bin"] {
                candidates.push(PathBuf::from(base).join(&executable));
            }
            if let Some(home) = &self.home {
                candidates.push(home.join(".local/bin").join(&executable));
            }
            if self.os == "macos" {
                let bundle = match name {
                    "vlc" => Some("VLC.app/Contents/MacOS/VLC"),
                    "mpv" => Some("mpv.app/Contents/MacOS/mpv"),
                    "chatterino" => Some("Chatterino.app/Contents/MacOS/chatterino"),
                    _ => None,
                };
                if let Some(bundle) = bundle {
                    candidates.push(PathBuf::from("/Applications").join(bundle));
                    if let Some(home) = &self.home {
                        candidates.push(home.join("Applications").join(bundle));
                    }
                }
            }
        }
        if self.os == "macos" && name == "chatterino" {
            candidates.push(PathBuf::from(
                "/Applications/chatterino.app/Contents/MacOS/chatterino",
            ));
            if let Some(home) = &self.home {
                candidates.push(home.join("Applications/chatterino.app/Contents/MacOS/chatterino"));
            }
        }
        candidates
    }
    pub fn find(&self, name: &str) -> Option<PathBuf> {
        self.candidates(name)
            .iter()
            .find_map(|p| validate_executable(p).ok())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDiscovery {
    pub mpv: Option<String>,
    pub vlc: Option<String>,
}
pub fn discover_players(locations: &SearchLocations) -> PlayerDiscovery {
    let find = |name| {
        locations
            .find(name)
            .and_then(|p| p.to_str().map(str::to_owned))
    };
    PlayerDiscovery {
        mpv: find("mpv"),
        vlc: find("vlc"),
    }
}
pub fn resolve_player(
    settings: &PlayerSettings,
    locations: &SearchLocations,
) -> Result<Option<PathBuf>> {
    settings.validate()?;
    if let Some(path) = &settings.executable {
        return validate_executable(Path::new(path)).map(Some).map_err(|_| {
            AppError::new(
                ErrorCode::PlayerNotFound,
                "The configured player is missing or is not an executable file.",
            )
        });
    }
    let name = match settings.mode {
        PlayerMode::Default => return Ok(None),
        PlayerMode::Mpv => "mpv",
        PlayerMode::Vlc => "vlc",
        PlayerMode::Custom => unreachable!("custom path validated"),
    };
    locations.find(name).map(Some).ok_or_else(|| {
        AppError::new(
            ErrorCode::PlayerNotFound,
            "The selected player was not found. Install it or set its executable path.",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn locations(os: &str, root: &Path) -> SearchLocations {
        SearchLocations {
            os: os.into(),
            path: None,
            home: Some(root.join("home")),
            program_files: vec![root.join("Program Files")],
            local_app_data: Some(root.join("Local App Data")),
        }
    }
    #[test]
    fn platform_fallbacks_include_gui_paths_bundles_and_windows_installations() {
        let temp = tempfile::tempdir().unwrap();
        let mac = locations("macos", temp.path());
        assert!(
            mac.candidates("streamlink")
                .contains(&PathBuf::from("/opt/homebrew/bin/streamlink"))
        );
        assert!(
            mac.candidates("streamlink")
                .contains(&PathBuf::from("/usr/local/bin/streamlink"))
        );
        assert!(
            mac.candidates("streamlink")
                .contains(&temp.path().join("home/.local/bin/streamlink"))
        );
        assert!(
            mac.candidates("vlc")
                .contains(&PathBuf::from("/Applications/VLC.app/Contents/MacOS/VLC"))
        );
        assert!(
            mac.candidates("mpv").contains(
                &temp
                    .path()
                    .join("home/Applications/mpv.app/Contents/MacOS/mpv")
            )
        );
        assert!(
            locations("linux", temp.path())
                .candidates("mpv")
                .contains(&PathBuf::from("/usr/bin/mpv"))
        );
        let windows = locations("windows", temp.path());
        assert!(
            windows
                .candidates("vlc")
                .contains(&temp.path().join("Program Files/VideoLAN/VLC/vlc.exe"))
        );
        assert!(
            windows.candidates("streamlink").contains(
                &temp
                    .path()
                    .join("Local App Data/Programs/Streamlink/bin/streamlink.exe")
            )
        );
    }
    #[test]
    fn player_discovery_and_explicit_override_validate_executable_files() {
        let root = tempfile::tempdir().unwrap();
        let path = root
            .path()
            .join(format!("mpv{}", std::env::consts::EXE_SUFFIX));
        std::fs::write(&path, "test executable").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let mut locations = locations(std::env::consts::OS, root.path());
        locations.path = Some(std::env::join_paths([root.path()]).unwrap());
        assert_eq!(
            discover_players(&locations).mpv,
            Some(path.canonicalize().unwrap().to_string_lossy().into_owned())
        );
        let player = PlayerSettings {
            mode: PlayerMode::Custom,
            executable: Some(path.to_string_lossy().into_owned()),
            arguments: vec![],
        };
        assert_eq!(
            resolve_player(&player, &locations).unwrap(),
            Some(path.canonicalize().unwrap())
        );
        std::fs::remove_file(&path).unwrap();
        assert_eq!(
            resolve_player(&player, &locations).unwrap_err().code,
            ErrorCode::PlayerNotFound
        );
    }
}

#[cfg(test)]
mod chatterino_tests {
    use super::*;
    #[test]
    fn chatterino_candidates_cover_native_installations_and_bound_path_search() {
        let root = tempfile::tempdir().unwrap();
        let mut locations = SearchLocations {
            os: "macos".into(),
            path: None,
            home: Some(root.path().into()),
            program_files: vec![root.path().join("Program Files")],
            local_app_data: Some(root.path().join("Local")),
        };
        let mac = locations.candidates("chatterino");
        for path in [
            "/opt/homebrew/bin/chatterino",
            "/usr/local/bin/chatterino",
            "/Applications/Chatterino.app/Contents/MacOS/chatterino",
            "/Applications/chatterino.app/Contents/MacOS/chatterino",
        ] {
            assert!(mac.contains(&PathBuf::from(path)));
        }
        assert!(
            mac.contains(
                &root
                    .path()
                    .join("Applications/Chatterino.app/Contents/MacOS/chatterino")
            )
        );
        locations.os = "windows".into();
        assert!(
            locations
                .candidates("chatterino")
                .contains(&root.path().join("Program Files/Chatterino/chatterino.exe"))
        );
        assert!(
            locations
                .candidates("chatterino")
                .contains(&root.path().join("Local/Programs/Chatterino/chatterino.exe"))
        );
        locations.os = "linux".into();
        assert!(
            locations
                .candidates("chatterino")
                .contains(&PathBuf::from("/usr/bin/chatterino"))
        );
        locations.path =
            Some(std::env::join_paths((0..1000).map(|n| root.path().join(n.to_string()))).unwrap());
        assert!(locations.candidates("chatterino").len() < 140);
    }
}
