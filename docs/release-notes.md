# Stream GUI RS 1.11.0

Changes since Stream GUI RS 1.10.0.

This feature release groups bookmarks by type and adds Claude and GitHub color themes.

## Features

- Grouped **Bookmarks** under Categories, Twitch channels, Kick and YouTube ([#139](https://github.com/ChrisLauinger77/stream-gui-rs/pull/139)). Each populated group shows a count and sorts by saved label in the interface language. A group reopens when it gains its first bookmark or refills after becoming empty, and Back and Forward restore each visit's open groups, scroll position and focus. Kick and YouTube rows keep a visible service label, and the grouped view is the same while signed out of Twitch.
- Added **Claude** and **GitHub** to **Settings → Appearance → Theme** ([#142](https://github.com/ChrisLauinger77/stream-gui-rs/pull/142)). Both have dark and light variants that follow **Color mode**. GitHub uses GitHub's Primer colors and Claude uses Anthropic's brand colors; like the other bundled themes, text, accent and focus colors keep readable contrast on app surfaces.

## Dependencies

- Refreshed both lockfiles ([#123](https://github.com/ChrisLauinger77/stream-gui-rs/pull/123), [#125](https://github.com/ChrisLauinger77/stream-gui-rs/pull/125), [#129](https://github.com/ChrisLauinger77/stream-gui-rs/pull/129), [#132](https://github.com/ChrisLauinger77/stream-gui-rs/pull/132), [#134](https://github.com/ChrisLauinger77/stream-gui-rs/pull/134), [#136](https://github.com/ChrisLauinger77/stream-gui-rs/pull/136), [#138](https://github.com/ChrisLauinger77/stream-gui-rs/pull/138), [#141](https://github.com/ChrisLauinger77/stream-gui-rs/pull/141)), including tauri 2.12.5 → 2.12.7, wry 0.57.2 → 0.57.3, tao 0.37.2 → 0.37.3, keyring-core 1.0.1 → 1.1.0, dbus-secret-service 4.1.0 → 4.2.0, rustls-webpki 0.103.16 → 0.103.17, zeroize 1.9.1 → 1.9.2 and @tauri-apps/api 2.12.4 → 2.12.5.
- Updated frontend build and test tooling: Vite 8.3.6 → 8.3.8 and @vitejs/plugin-react 6.2.1 → 6.3.0 ([#122](https://github.com/ChrisLauinger77/stream-gui-rs/pull/122), [#127](https://github.com/ChrisLauinger77/stream-gui-rs/pull/127)), jsdom 31.0.0 → 31.0.1 ([#131](https://github.com/ChrisLauinger77/stream-gui-rs/pull/131)) and @tauri-apps/cli 2.12.4 → 2.12.5 ([#137](https://github.com/ChrisLauinger77/stream-gui-rs/pull/137)).
- Updated the `anthropics/claude-code-action` ([#124](https://github.com/ChrisLauinger77/stream-gui-rs/pull/124), [#126](https://github.com/ChrisLauinger77/stream-gui-rs/pull/126), [#130](https://github.com/ChrisLauinger77/stream-gui-rs/pull/130), [#133](https://github.com/ChrisLauinger77/stream-gui-rs/pull/133), [#135](https://github.com/ChrisLauinger77/stream-gui-rs/pull/135), [#140](https://github.com/ChrisLauinger77/stream-gui-rs/pull/140)) and `dtolnay/rust-toolchain` ([#128](https://github.com/ChrisLauinger77/stream-gui-rs/pull/128)) workflow digests.

## Upgrade and platform support

Settings remain at schema 13; upgrading from 1.10.0 needs no migration. Upgrade without signing out or clearing settings. Older versions reject settings saved with the Claude or GitHub theme without overwriting the file; switch to another theme before downgrading.

Streamlink 8.0+ and a separately installed player remain required. Package formats remain Windows x86_64 installer and portable/Scoop ZIP; Linux x86_64 AppImage, DEB and RPM; and a universal macOS DMG for Apple Silicon and Intel requiring macOS 11.0 or later. Verify downloads against the accompanying SHA-256 files.
