# Stream GUI RS 1.5.0

Play exact Kick live channels through Streamlink, including while signed out of Twitch.

## Features

**Open channel → Service → Kick** accepts a channel name and starts playback in your separately installed player. Enter a name, not a URL. The app normalizes the name to lowercase; it does not verify live status or retrieve Kick titles, categories or viewer counts ([#42](https://github.com/ChrisLauinger77/stream-gui-rs/pull/42)).

Kick uses existing global playback settings, the selected player profile and quality policy. Twitch channel overrides do not apply. The low-latency preference selects the appropriate Streamlink option for each service. Audio requests audio-only playback and has no video fallback if that rendition is unavailable.

Watching labels Twitch and Kick sessions. Both services share the existing eight-session limit, with independent Stop and Restart controls. Restart applies current settings; saving preferences leaves running playback unchanged.

**Open Kick browser chat** opens the channel's popout chat in your default browser, using the browser's own login. Automatic chat and Chatterino are disabled for Kick. Twitch browsing and chat retain their existing behavior.

## Requirements and limits

Streamlink 8.0+ and a separate player remain required. Kick browser challenges also need a Chromium-compatible browser discoverable by Streamlink. Streamlink owns challenge handling and its cached state; Stream GUI RS does not read or import browser profiles or cookies. A missing browser or failed challenge can prevent playback, and failure does not establish that a channel is offline.

Kick child output is drained without being retained in local diagnostics to protect challenge data. Fixed supervisor messages and process status remain available; the safe support report includes only the service in anonymous process summaries.

No Kick credentials or developer-app registration are required. Kick login, Following, discovery/search/categories, notifications, persistent channel preferences, bookmarks/hides, Chatterino, public deep links and VOD/clip UI are not supported. Official Kick discovery remains deferred. See [Kick playback](../README.md#kick-playback) for details.

## Maintenance

- Updated UUID to 1.27.0 ([#41](https://github.com/ChrisLauinger77/stream-gui-rs/pull/41)) and libc to 0.2.190 ([#44](https://github.com/ChrisLauinger77/stream-gui-rs/pull/44)).
- Updated the Rust toolchain action revision ([#43](https://github.com/ChrisLauinger77/stream-gui-rs/pull/43)).

## Upgrade and platform support

Settings remain at schema 9; this release adds no settings migration. Upgrade without signing out or clearing settings.

Package formats remain Windows x86_64 installer and portable/Scoop ZIP; Linux x86_64 AppImage, DEB and RPM; and a universal macOS DMG for Apple Silicon and Intel requiring macOS 11.0 or later. Verify downloads against the accompanying SHA-256 files.

Windows packages are unsigned. macOS bundles have an ad-hoc signature, without Developer ID signing or notarization. Automated package inspection does not establish native execution or browser-challenge cleanup on each platform.

[Full changelog](https://github.com/ChrisLauinger77/stream-gui-rs/compare/v1.1.0...v1.5.0)
