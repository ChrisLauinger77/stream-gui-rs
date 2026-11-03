# Stream GUI RS 1.7.1

Changes since Stream GUI RS 1.7.0.

This patch release fixes Streamlink detection and playback from the Linux AppImage.

## Fixes

- Fixed "Streamlink could not be detected" in the Linux AppImage when Streamlink is installed on the host with Python, for example through a distribution package, pip, pipx or a virtual environment ([#97](https://github.com/ChrisLauinger77/stream-gui-rs/issues/97), [#98](https://github.com/ChrisLauinger77/stream-gui-rs/pull/98)). The AppImage launcher set Python, library and search-path variables that pointed into the bundle, and Streamlink inherited them. Streamlink and the player now get the host environment back; other variables, such as proxy settings, pass through unchanged. DEB, RPM, Windows and macOS packages were not affected.

## Dependencies

- Updated tauri-plugin-single-instance to 2.5.5, tao to 0.37.2, wry to 0.57.1, uuid to 1.29.0, webbrowser to 1.2.5, aws-lc-rs to 1.19.0 and other locked Rust and npm dependencies ([#92](https://github.com/ChrisLauinger77/stream-gui-rs/pull/92), [#93](https://github.com/ChrisLauinger77/stream-gui-rs/pull/93), [#94](https://github.com/ChrisLauinger77/stream-gui-rs/pull/94), [#95](https://github.com/ChrisLauinger77/stream-gui-rs/pull/95)).
- CI and release builds now use Node.js 26 ([#91](https://github.com/ChrisLauinger77/stream-gui-rs/pull/91)).

## Upgrade and platform support

Settings stay at schema 10; upgrade from 1.7.0 without signing out or clearing settings.

Streamlink 8.0+ and a separately installed player remain required. Package formats remain Windows x86_64 installer and portable/Scoop ZIP; Linux x86_64 AppImage, DEB and RPM; and a universal macOS DMG for Apple Silicon and Intel requiring macOS 11.0 or later. Verify downloads against the accompanying SHA-256 files.
