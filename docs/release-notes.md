# Stream GUI RS 1.5.2

Changes since Stream GUI RS 1.5.1.

This patch release fixes clipped text in native dropdowns.

## Fixes

- Native dropdowns keep their consistent control height while allowing the webview to center text without clipping descenders ([#48](https://github.com/ChrisLauinger77/stream-gui-rs/pull/48)).

## Dependencies

- Updated the jsdom test dependency from 30.1.1 to 30.1.2 ([#47](https://github.com/ChrisLauinger77/stream-gui-rs/pull/47)).

## Upgrade and platform support

Settings remain at schema 9. Upgrade without signing out or clearing settings.

Streamlink 8.0+ and a separately installed player remain required. Package formats remain Windows x86_64 installer and portable/Scoop ZIP; Linux x86_64 AppImage, DEB and RPM; and a universal macOS DMG for Apple Silicon and Intel requiring macOS 11.0 or later. Verify downloads against the accompanying SHA-256 files.

Windows packages are unsigned. macOS bundles have an ad-hoc signature, without Developer ID signing or notarization.

[Full changelog](https://github.com/ChrisLauinger77/stream-gui-rs/compare/v1.5.1...v1.5.2)
