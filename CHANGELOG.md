# Changelog

## Unreleased

## 0.5.0 - 2026-09-23

### Added

- Twitch Teams browsing with direct access to member channels and the existing Watch flow.
- Local channel and category bookmarks, plus hide lists for passive discovery; direct navigation and other browsing views remain reachable.
- Configurable application-local shortcuts with conflict detection, unassign and reset controls.
- Strict `stream-gui-rs://` navigation links for Show, channels, categories and teams, without automatic playback.
- Settings schema 7 migration with safe defaults for the new discovery lists and shortcuts.

### Improved

- Home, Back and Forward navigation retains bounded search, scroll and focus state.
- Installed links activate the existing application instance on Linux and Windows and use bundle activation on macOS.

## 0.4.0 - 2026-09-22

### Added

- Manual update checks against the official GitHub release source, with safe release-page opening and no automatic downloads or installation.
- External Chatterino chat with native discovery, an explicit executable override and Linux support for the official Flatpak; browser chat remains available.
- Up to 16 reusable player profiles with optional quality and low-latency preferences. Running streams keep their settings; Restart applies the selected profile.
- Settings schema 6 migration preserves existing preferences and starts with browser chat and no selected profile.

### Improved

- Update checks announce progress immediately and retain clear failure/retry states.
- Player-profile editing supports keyboard save and focus restoration, including profiles whose player is temporarily unavailable.
- Chatterino capacity errors explain how to continue; independent chat windows remain open after playback stops or Stream GUI RS quits.
- Coordinated settings saves preserve profile changes when requests overlap or Settings is reopened.

## 0.3.0 - 2026-09-20

### Added

- Language filtering for Live and category streams, with saved defaults and Back navigation.
- Direct channel lookup by exact Twitch login, including offline channels.
- Optional low-latency Twitch playback, globally or per channel.
- Application text sizes of 100%, 125% and 150%.
- Previewable support reports with anonymous, allowlisted troubleshooting information.
- About access from the main interface and tray/status menu on every platform, with consistent version, build commit and repository link.

### Improved

- Keyboard focus restoration, wrapping at larger text sizes, and accessible live/offline/unknown channel descriptions.

### Fixed

- Long channel lists scroll within the results pane, keeping the header, navigation and About footer visible without a second page scrollbar.
- The native macOS About panel presents a labeled, underlined repository link.
- Saved preferences stay consistent when Settings closes/reopens or language, global and channel saves overlap; deliberate unsaved edits are retained.
- Tray and notification navigation reveal their destination when a modal dialog was open.

## 0.2.0 - 2026-09-20

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
