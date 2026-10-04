# Stream GUI RS 1.5.1

This patch release makes single-line controls consistent across the interface and updates Tokio.

## Fixes

- Buttons, text inputs and selects share a consistent height across browsing, Settings and Watching. Long button labels can still wrap, and compact text links and checkboxes retain their existing sizing ([#46](https://github.com/ChrisLauinger77/stream-gui-rs/pull/46)).

## Dependencies

- Updated Tokio from 1.53.1 to 1.53.2 ([#45](https://github.com/ChrisLauinger77/stream-gui-rs/pull/45)).

## Upgrade and platform support

Settings remain at schema 9. Upgrade without signing out or clearing settings.

Streamlink 8.0+ and a separately installed player remain required. Package formats remain Windows x86_64 installer and portable/Scoop ZIP; Linux x86_64 AppImage, DEB and RPM; and a universal macOS DMG for Apple Silicon and Intel requiring macOS 11.0 or later. Verify downloads against the accompanying SHA-256 files.

Windows packages are unsigned. macOS bundles have an ad-hoc signature, without Developer ID signing or notarization.

[Full changelog](https://github.com/ChrisLauinger77/stream-gui-rs/compare/v1.5.0...v1.5.1)
