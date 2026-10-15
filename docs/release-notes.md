# Stream GUI RS 1.5.5

Changes since Stream GUI RS 1.5.4.

This patch release updates the desktop runtime and locked Rust dependencies. There are no user-facing feature changes.

## Dependencies

- Updated Tauri to 2.12.4 and the single-instance plugin to 2.5.4 ([#72](https://github.com/ChrisLauinger77/stream-gui-rs/pull/72), [#73](https://github.com/ChrisLauinger77/stream-gui-rs/pull/73)).
- Refreshed locked Rust dependencies, including zbus 5.19.0, hyper 1.12.0, rustls 0.23.46, aws-lc-rs 1.18.2 and time 0.3.55 ([#74](https://github.com/ChrisLauinger77/stream-gui-rs/pull/74)).
- Updated the pinned Claude review workflow action ([#71](https://github.com/ChrisLauinger77/stream-gui-rs/pull/71)).

## Development

- Building from source now requires Rust 1.90 or later, matching Tauri 2.12's minimum supported Rust version.
- Optional Git hooks in `.githooks/` refresh npm and Cargo dependencies after pulls that change the lockfiles. Enable them per clone with `git config core.hooksPath .githooks`.

## Upgrade and platform support

Settings remain at schema 9. Upgrade without signing out or clearing settings.

Streamlink 8.0+ and a separately installed player remain required. Package formats remain Windows x86_64 installer and portable/Scoop ZIP; Linux x86_64 AppImage, DEB and RPM; and a universal macOS DMG for Apple Silicon and Intel requiring macOS 11.0 or later. Verify downloads against the accompanying SHA-256 files.

Windows packages are unsigned. macOS bundles have an ad-hoc signature, without Developer ID signing or notarization.

[Full changelog](https://github.com/ChrisLauinger77/stream-gui-rs/compare/v1.5.4...v1.5.5)
