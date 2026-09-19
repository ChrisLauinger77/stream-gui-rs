# Changelog

## Unreleased

## 0.2.0 - 2026-09-19

### Added

- Optional background monitoring of followed Twitch channels, with native live notifications and click-to-channel navigation.
- Tray/menu-bar controls, Pause/Resume monitoring, and opt-in close-to-background while playback continues.
- Per-channel notification preferences, preserving existing playback, chat and theme settings on upgrade.
- Build commit information in diagnostics and the native macOS About panel, with a clickable repository link.

### Improved

- Quiet startup and recovery after sleep or network interruptions, with notification deduplication and priority for foreground browsing.

### Fixed

- Windows notifications remain actionable from Notification Center after the banner times out, within the running app's retention policy.
- macOS bundles receive a complete ad-hoc signature so notification permission and delivery can work.
- Linux Wayland title bars use compact native controls with working minimize, maximize and close input routing.

## 0.1.0 - 2026-09-19

First public release.

- Browse followed channels, live streams, categories, search results, and channel details on Twitch.
- Sign in through Twitch Device Code authorization and store credentials in the native secure store.
- Launch Streamlink 8.0+ with default, mpv, VLC, or custom players and five quality policies.
- Control multiple sessions with Stop, Restart, Watching, and bounded diagnostics.
- Save global playback settings, per-channel overrides, browser chat, appearance, and shortcuts.
- Provide an installer and Scoop-ready ZIP for Windows x86_64; AppImage, Debian, and RPM packages for Linux x86_64; and a universal macOS disk image.
