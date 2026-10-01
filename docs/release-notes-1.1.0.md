# Stream GUI RS 1.1.0

Choose from eight bundled color themes or supply your own color palette.

## Features

**Settings → Appearance → Theme** now offers Default, Catppuccin, Dracula, Everforest, Gruvbox, Nord, Rosé Pine, Solarized and Tokyo Night. Color mode remains a separate System/Light/Dark preference. Dracula resolves to Dark while selected and preserves the saved color mode. Theme choices persist across restarts ([#32](https://github.com/ChrisLauinger77/stream-gui-rs/pull/32)).

A validated `custom-theme.json` beside the settings file enables Custom Theme. It accepts colors only, with a required complete dark palette and an optional complete light palette. Changes to a selected palette apply within about a second. Missing or invalid files fall back to Default while retaining the Custom selection. See the [theme instructions and example](../README.md#discovery-appearance-and-support) for setup; check text and focus contrast when choosing custom colors.

The project has a new [homepage](https://chrislauinger77.github.io/stream-gui-rs/) with installation and project information ([#33](https://github.com/ChrisLauinger77/stream-gui-rs/pull/33)).

## Fixes and maintenance

- Corrected the German following hint.
- Migrated native credential-store backends ([#28](https://github.com/ChrisLauinger77/stream-gui-rs/pull/28)) and updated Windows native bindings ([#30](https://github.com/ChrisLauinger77/stream-gui-rs/pull/30), [#40](https://github.com/ChrisLauinger77/stream-gui-rs/pull/40)).
- Updated Tauri and single-instance integration ([#34](https://github.com/ChrisLauinger77/stream-gui-rs/pull/34), [#35](https://github.com/ChrisLauinger77/stream-gui-rs/pull/35), [#36](https://github.com/ChrisLauinger77/stream-gui-rs/pull/36), [#37](https://github.com/ChrisLauinger77/stream-gui-rs/pull/37)).
- Updated Vite and Vitest ([#29](https://github.com/ChrisLauinger77/stream-gui-rs/pull/29), [#38](https://github.com/ChrisLauinger77/stream-gui-rs/pull/38), [#39](https://github.com/ChrisLauinger77/stream-gui-rs/pull/39)); GTK/GIO migration remains deferred ([#31](https://github.com/ChrisLauinger77/stream-gui-rs/pull/31)).

## Upgrade and platform support

Settings schema 8 migrates to schema 9 in memory, preserving existing preferences and selecting the Default palette. Migration is written on the next successful settings save; downgrading to 1.0.0 after that save is unsupported. Upgrade without signing out or clearing settings.

Packages remain Windows x86_64 installer and portable/Scoop ZIP; Linux x86_64 AppImage, DEB and RPM; and a universal macOS DMG for Apple Silicon and Intel requiring macOS 11.0 or later. Streamlink 8.0+ and a separate player remain required. Verify downloads against the accompanying SHA-256 files.

Windows packages are unsigned. macOS bundles have an ad-hoc signature, without Developer ID signing or notarization. Native execution on each architecture is separate from automated package inspection.

[Full changelog](https://github.com/ChrisLauinger77/stream-gui-rs/compare/v1.0.0...v1.1.0)
