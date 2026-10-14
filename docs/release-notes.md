# Stream GUI RS 1.5.4

Changes since Stream GUI RS 1.5.3.

This patch release gives the tray icon a monochrome status-area glyph and updates desktop and build dependencies.

## Features

- The tray now shows a monochrome glyph of the logo's stream lines and play mark instead of the colored application icon. macOS uses a template image that follows the menu bar appearance, Windows follows the taskbar's light or dark mode and updates when it changes, and Linux uses a white glyph with a dark outline that stays visible on light and dark panels. Windows, bundles and notifications keep the colored application icon ([#68](https://github.com/ChrisLauinger77/stream-gui-rs/pull/68)).

## Dependencies

- Updated Tauri to 2.12.3 and its frontend API to 2.12.3, and the single-instance plugin to 2.5.3 ([#60](https://github.com/ChrisLauinger77/stream-gui-rs/pull/60), [#61](https://github.com/ChrisLauinger77/stream-gui-rs/pull/61)).
- Updated tokio to 1.53.3, keyring-core to 1.0.1, uuid to 1.28.0 and serde_json to 1.0.154 ([#58](https://github.com/ChrisLauinger77/stream-gui-rs/pull/58), [#59](https://github.com/ChrisLauinger77/stream-gui-rs/pull/59), [#66](https://github.com/ChrisLauinger77/stream-gui-rs/pull/66), [#67](https://github.com/ChrisLauinger77/stream-gui-rs/pull/67)).
- Updated Vitest to 5.0.4 ([#63](https://github.com/ChrisLauinger77/stream-gui-rs/pull/63)).
- Updated and pinned GitHub Actions for the Claude review workflow ([#62](https://github.com/ChrisLauinger77/stream-gui-rs/pull/62), [#64](https://github.com/ChrisLauinger77/stream-gui-rs/pull/64), [#65](https://github.com/ChrisLauinger77/stream-gui-rs/pull/65)).

## Upgrade and platform support

Settings remain at schema 9. Upgrade without signing out or clearing settings.

Streamlink 8.0+ and a separately installed player remain required. Package formats remain Windows x86_64 installer and portable/Scoop ZIP; Linux x86_64 AppImage, DEB and RPM; and a universal macOS DMG for Apple Silicon and Intel requiring macOS 11.0 or later. Verify downloads against the accompanying SHA-256 files.

Windows packages are unsigned. macOS bundles have an ad-hoc signature, without Developer ID signing or notarization.

[Full changelog](https://github.com/ChrisLauinger77/stream-gui-rs/compare/v1.5.3...v1.5.4)
