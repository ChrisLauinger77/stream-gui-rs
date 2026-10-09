# Stream GUI RS 1.5.3

Changes since Stream GUI RS 1.5.2.

This patch release updates desktop and build dependencies and improves release preparation.

## Dependencies

- Updated Tauri and its frontend API to 2.12.2 ([#56](https://github.com/ChrisLauinger77/stream-gui-rs/pull/56)).
- Updated the macOS Objective-C bindings, objc2, to 0.6.5 ([#49](https://github.com/ChrisLauinger77/stream-gui-rs/pull/49)).
- Updated zeroize to 1.9.1 and tokio-util to 0.7.20 ([#51](https://github.com/ChrisLauinger77/stream-gui-rs/pull/51), [#57](https://github.com/ChrisLauinger77/stream-gui-rs/pull/57)).
- Updated Vite to 8.3.4 and its React plugin to 6.1.2 ([#50](https://github.com/ChrisLauinger77/stream-gui-rs/pull/50), [#54](https://github.com/ChrisLauinger77/stream-gui-rs/pull/54)).
- Updated GitHub Actions and Rust toolchain setup ([#52](https://github.com/ChrisLauinger77/stream-gui-rs/pull/52), [#53](https://github.com/ChrisLauinger77/stream-gui-rs/pull/53), [#55](https://github.com/ChrisLauinger77/stream-gui-rs/pull/55)).

## Release preparation

- Consolidated the latest release notes into one maintained file; previous notes remain available on GitHub Releases.
- Added exact candidate tag commands to workflow summaries, switched CI to pull requests and manual dispatch, and removed the artifact cleanup workflow.

## Upgrade and platform support

Settings remain at schema 9. Upgrade without signing out or clearing settings.

Streamlink 8.0+ and a separately installed player remain required. Package formats remain Windows x86_64 installer and portable/Scoop ZIP; Linux x86_64 AppImage, DEB and RPM; and a universal macOS DMG for Apple Silicon and Intel requiring macOS 11.0 or later. Verify downloads against the accompanying SHA-256 files.

Windows packages are unsigned. macOS bundles have an ad-hoc signature, without Developer ID signing or notarization.

[Full changelog](https://github.com/ChrisLauinger77/stream-gui-rs/compare/v1.5.2...v1.5.3)
