# Stream GUI RS 0.1.0

First public release.

## Highlights

- Browse Twitch Following, popular live streams, categories, search results, and channel details.
- Connect through Twitch Device Code sign-in with credentials protected by the operating system's secure store.
- Watch through Streamlink 8.0+ using its default player, mpv, VLC, or a custom executable.
- Run and control multiple streams with Source, High, Medium, Low, and Audio quality policies.
- Save global and per-channel playback preferences.
- Open Twitch chat in the system browser.
- Use System, Light, and Dark themes plus focused application shortcuts.

## Platforms

- Windows 11 x86_64: NSIS installer
- Linux x86_64: AppImage and Debian package
- macOS Apple Silicon arm64: disk image containing the app

Intel macOS and ARM Linux/Windows packages are deferred until their exact artifacts can be natively tested. The Windows installer is unsigned and may trigger SmartScreen. The macOS app has no Developer ID signature or notarization and may be blocked by Gatekeeper on first launch.

## Requirements

Install Streamlink 8.0 or newer and a Streamlink-compatible external player. mpv and VLC have built-in discovery. Linux authentication also requires a working Secret Service provider in the desktop session.

## Known limitations

- No background followed-stream monitoring, notifications, or tray mode
- No embedded player or chat client
- No advanced Streamlink transports or player-profile system
- No updater or legacy configuration import
- Intentionally detached custom players cannot be stopped by application process-tree cleanup

## Acknowledgements

Playback is provided by [Streamlink](https://streamlink.github.io/). Stream GUI RS is an independent rewrite inspired by [Streamlink Twitch GUI](https://github.com/streamlink/streamlink-twitch-gui). Thanks to the Rust, Tauri, React, and other open-source projects used by this application.
