//! Host environment for children started from a Linux AppImage.
//!
//! The AppImageKit `AppRun` used by Tauri's AppImage, and linuxdeploy's GTK
//! hook, point Python, loader and toolkit variables into this bundle's private
//! mount before the app starts. Every child the app owns (Streamlink and,
//! through it, the player) is a host program, never part of the bundle: an
//! inherited `PYTHONHOME` alone aborts a host Python Streamlink at startup.
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    sync::OnceLock,
};
use tokio::process::Command;

/// Runtime-provided identity of this AppImage. A child that reads it, such as
/// an extracted AppImage started through its own `AppRun`, would otherwise
/// resolve its files relative to this bundle.
const IDENTITY: [&str; 4] = ["APPDIR", "APPIMAGE", "ARGV0", "OWD"];

/// Search paths the launcher prefixes with bundle entries while keeping the
/// previous value as the suffix, so removing bundle entries restores it.
const SEARCH_PATHS: [&str; 12] = [
    "PATH",
    "LD_LIBRARY_PATH",
    "PYTHONPATH",
    "XDG_DATA_DIRS",
    "PERLLIB",
    "GSETTINGS_SCHEMA_DIR",
    "QT_PLUGIN_PATH",
    "GI_TYPELIB_PATH",
    "GST_PLUGIN_PATH",
    "GST_PLUGIN_PATH_1_0",
    "GST_PLUGIN_SYSTEM_PATH",
    "GST_PLUGIN_SYSTEM_PATH_1_0",
];

/// Removes this AppImage's launcher environment from an owned child command.
/// Outside an AppImage this changes nothing.
pub(super) fn restore_host_environment(command: &mut Command) {
    for (key, value) in host_repairs(bundle(), std::env::vars_os()) {
        match value {
            Some(value) => command.env(key, value),
            None => command.env_remove(key),
        };
    }
}

fn bundle() -> &'static [PathBuf] {
    // The app never mutates APPDIR or moves its executable, so detect once.
    static ROOTS: OnceLock<Vec<PathBuf>> = OnceLock::new();
    ROOTS.get_or_init(|| match std::env::current_exe() {
        Ok(executable) => bundle_roots(std::env::var_os("APPDIR").as_deref(), &executable),
        Err(_) => Vec::new(),
    })
}

/// The AppDir spellings that bundle entries may use, or none when this
/// executable does not run from an AppDir (mounted or extracted).
fn bundle_roots(app_dir: Option<&OsStr>, executable: &Path) -> Vec<PathBuf> {
    let Some(raw) = app_dir.map(PathBuf::from).filter(|path| path.is_absolute()) else {
        return Vec::new();
    };
    let Ok(canonical) = std::fs::canonicalize(&raw) else {
        return Vec::new();
    };
    // A stray or inherited APPDIR must never classify the host filesystem or a
    // system prefix as bundle content: require an AppDir around this program.
    if canonical.parent().is_none()
        || !canonical.join("AppRun").exists()
        || !executable.starts_with(&canonical)
    {
        return Vec::new();
    }
    // AppRun writes its resolved directory while shell hooks reuse APPDIR as
    // given, so entries may use either spelling.
    let mut roots = vec![canonical];
    if raw != roots[0] {
        roots.push(raw);
    }
    roots
}

/// Changes that give a child the environment it would have outside the
/// AppImage; `None` removes the variable. Variables without bundle entries,
/// including user settings such as proxies, are left untouched.
fn host_repairs(
    roots: &[PathBuf],
    variables: impl IntoIterator<Item = (OsString, OsString)>,
) -> Vec<(OsString, Option<OsString>)> {
    if roots.is_empty() {
        return Vec::new();
    }
    let bundled =
        |entry: &Path| entry.is_absolute() && roots.iter().any(|root| entry.starts_with(root));
    variables
        .into_iter()
        .filter_map(|(key, value)| {
            let name = key.to_str();
            if name.is_some_and(|name| IDENTITY.contains(&name)) {
                return Some((key, None));
            }
            if name.is_some_and(|name| SEARCH_PATHS.contains(&name)) {
                let entries: Vec<_> = std::env::split_paths(&value).collect();
                if !entries.iter().any(|entry| bundled(entry)) {
                    return None;
                }
                // An empty entry means the current directory. The launcher
                // leaves one behind when the previous value was empty or unset.
                let kept: Vec<_> = entries
                    .into_iter()
                    .filter(|entry| !entry.as_os_str().is_empty() && !bundled(entry))
                    .collect();
                // Split entries never contain the separator, so joining succeeds.
                let value = if kept.is_empty() {
                    None
                } else {
                    std::env::join_paths(kept).ok()
                };
                return Some((key, value));
            }
            // Single bundle paths: PYTHONHOME, GTK_PATH, GDK_PIXBUF_MODULE_FILE,
            // ... Lists in other variables are not reinterpreted.
            let single = std::env::split_paths(&value).nth(1).is_none();
            (single && bundled(Path::new(&value))).then_some((key, None))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn repairs(roots: &[PathBuf], variables: &[(&str, &str)]) -> BTreeMap<String, Option<String>> {
        host_repairs(
            roots,
            variables
                .iter()
                .map(|(key, value)| (OsString::from(key), OsString::from(value))),
        )
        .into_iter()
        .map(|(key, value)| {
            (
                key.into_string().unwrap(),
                value.map(|value| value.into_string().unwrap()),
            )
        })
        .collect()
    }

    #[test]
    fn restores_the_host_environment_from_the_released_launcher() {
        let root = "/tmp/.mount_StreamAbc";
        // Values as the 1.7.0 AppImage's AppRun and GTK hook produce them.
        let path = format!(
            "{root}/usr/bin/:{root}/usr/sbin/:{root}/usr/games/:{root}/bin/:{root}/sbin/:/usr/local/bin:/usr/bin"
        );
        let library = format!(
            "{root}/usr/lib/:{root}/usr/lib/i386-linux-gnu/:{root}/usr/lib/x86_64-linux-gnu/:{root}/usr/lib32/:{root}/usr/lib64/:{root}/lib/:{root}/lib/i386-linux-gnu/:{root}/lib/x86_64-linux-gnu/:{root}/lib32/:{root}/lib64/:/opt/host/lib"
        );
        let data = format!("{root}/usr/share/:{root}/usr/share:/usr/share:/usr/local/share/");
        let python_home = format!("{root}/usr/");
        let python_path = format!("{root}/usr/share/pyshared/:");
        let schemas = format!("{root}//usr/share/glib-2.0/schemas");
        let pixbuf = format!("{root}//usr/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache");
        let gtk = format!("{root}//usr/lib/gtk-3.0");
        let qt = format!("{root}/usr/lib/qt5/plugins/:");
        let lookalike = format!("{root}d/etc/xdg");
        let list = format!("{root}/usr/share:/opt/data");
        let variables = [
            ("APPDIR", root),
            ("APPIMAGE", "/home/viewer/Apps/Stream-GUI-RS.AppImage"),
            ("ARGV0", "Stream-GUI-RS.AppImage"),
            ("OWD", "/home/viewer"),
            ("PATH", &path),
            ("LD_LIBRARY_PATH", &library),
            ("XDG_DATA_DIRS", &data),
            ("PYTHONHOME", &python_home),
            ("PYTHONPATH", &python_path),
            ("GSETTINGS_SCHEMA_DIR", &schemas),
            ("GDK_PIXBUF_MODULE_FILE", &pixbuf),
            ("GTK_PATH", &gtk),
            ("QT_PLUGIN_PATH", &qt),
            // User or host values: no bundle entry, lookalike prefix, a list
            // outside the known search paths, and non-path launcher values.
            ("HTTPS_PROXY", "http://proxy.invalid:8080"),
            ("PERLLIB", "/usr/share/perl5:"),
            ("XDG_CONFIG_DIRS", &lookalike),
            ("STREAM_GUI_RS_LIST", &list),
            ("GTK_THEME", "Adwaita:dark"),
            ("PYTHONDONTWRITEBYTECODE", "1"),
        ];
        let expected: BTreeMap<String, Option<String>> = [
            ("APPDIR", None),
            ("APPIMAGE", None),
            ("ARGV0", None),
            ("OWD", None),
            ("PATH", Some("/usr/local/bin:/usr/bin")),
            ("LD_LIBRARY_PATH", Some("/opt/host/lib")),
            ("XDG_DATA_DIRS", Some("/usr/share:/usr/local/share/")),
            ("PYTHONHOME", None),
            ("PYTHONPATH", None),
            ("GSETTINGS_SCHEMA_DIR", None),
            ("GDK_PIXBUF_MODULE_FILE", None),
            ("GTK_PATH", None),
            ("QT_PLUGIN_PATH", None),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.map(str::to_owned)))
        .collect();
        assert_eq!(repairs(&[PathBuf::from(root)], &variables), expected);
        assert!(repairs(&[], &variables).is_empty());
    }

    #[test]
    fn matches_every_spelling_of_the_bundle() {
        let roots = [PathBuf::from("/tmp/.mount_x"), PathBuf::from("/link/app")];
        assert_eq!(
            repairs(
                &roots,
                &[
                    ("PATH", "/link/app/usr/bin:/tmp/.mount_x/bin:/usr/bin"),
                    ("PYTHONHOME", "/link/app/usr")
                ]
            ),
            BTreeMap::from([
                ("PATH".to_owned(), Some("/usr/bin".to_owned())),
                ("PYTHONHOME".to_owned(), None),
            ])
        );
    }

    #[test]
    fn detects_only_an_app_dir_around_this_executable() {
        let directory = tempfile::tempdir().unwrap();
        let base = std::fs::canonicalize(directory.path()).unwrap();
        let app_dir = base.join("squashfs-root");
        let executable = app_dir.join("usr/bin/stream-gui-rs");
        std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        std::fs::write(&executable, "").unwrap();
        let alias = base.join("alias");
        std::os::unix::fs::symlink(&app_dir, &alias).unwrap();
        let unmarked = base.join("unmarked");
        std::fs::create_dir_all(unmarked.join("usr/bin")).unwrap();
        // Not an AppDir until the launcher marker exists.
        assert!(bundle_roots(Some(app_dir.as_os_str()), &executable).is_empty());
        std::fs::write(app_dir.join("AppRun"), "").unwrap();

        assert_eq!(
            bundle_roots(Some(app_dir.as_os_str()), &executable),
            std::slice::from_ref(&app_dir)
        );
        assert_eq!(
            bundle_roots(Some(alias.as_os_str()), &executable),
            [app_dir.clone(), alias]
        );
        for (value, program) in [
            (None, executable.clone()),
            (Some(OsStr::new("squashfs-root")), executable.clone()),
            (Some(OsStr::new("/")), executable.clone()),
            (Some(base.join("missing").as_os_str()), executable.clone()),
            (Some(unmarked.as_os_str()), unmarked.join("usr/bin/app")),
            (
                Some(app_dir.as_os_str()),
                base.join("elsewhere/stream-gui-rs"),
            ),
        ] {
            assert!(
                bundle_roots(value, &program).is_empty(),
                "{value:?} {program:?}"
            );
        }
    }
}
