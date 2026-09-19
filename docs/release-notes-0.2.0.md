# Stream GUI RS 0.2.0

Stream GUI RS can now stay in the background and notify you when followed Twitch channels go live.

## Highlights

- Opt-in followed-channel monitoring and native desktop live notifications.
- Click a notification to restore the application and open the channel without starting playback.
- Tray/menu-bar controls, Pause/Resume, and optional close-to-background while streams keep playing.
- Per-channel notification preferences alongside existing playback and browser-chat settings.
- Quiet initial scans and recovery after sleep or network interruptions, with deduplication and priority for browsing.
- Build commit information in diagnostics and the native macOS About panel, plus a clickable repository link.
- Compact Linux Wayland title-bar controls and corrected input routing.
- Windows Notification Center activation after banner timeout and complete macOS bundle signing for notification permission.

Twitch browsing/search, Streamlink playback, mpv/VLC/custom players, multiple sessions, quality policies, per-channel settings, browser chat, themes and keyboard shortcuts remain available.

## Installation and upgrade

Packages: Windows 11 x86_64 installer and portable/Scoop ZIP; Linux x86_64 AppImage, Debian and RPM packages; universal macOS DMG with Apple Silicon and Intel slices. Verify downloads against the accompanying SHA-256 files.

Install Streamlink 8.0+ and an external player separately. Windows needs WebView2; Linux needs GTK/WebKitGTK dependencies, a user D-Bus session and an unlocked Secret Service provider. macOS requires version 11.0 or newer, with system WebKit and Keychain. Installed builds include the public Twitch client ID; no environment setup or client secret is needed.

Upgrades preserve existing playback, chat, theme and channel preferences and retain the same native credential identity. Monitoring, notifications and close-to-background start disabled. Enable them in **Settings → Background** and grant native notification permission where required. The first scan is quiet. Settings saved by 0.2.0 use schema 4, which 0.1.0 cannot read.

With background close enabled, closing hides the window if a tray is available or minimizes it otherwise. **Quit** in the main interface or tray/menu bar always ends monitoring and stops owned playback. On macOS, Command-Q also quits.

## Known limitations

- Linux notification actions and tray support depend on the desktop environment; missing tray support falls back to minimize.
- Windows notifications are intended for the installed Start-menu identity. Retained entries expire after 15 minutes and require the app and original monitoring/sign-in session to remain active. Pause, logout and Quit retire them; clicks cannot relaunch a fully quit app. Portable notification integration can be unavailable.
- Windows packages are unsigned. macOS bundles are ad-hoc signed, without Developer ID signing or notarization. SmartScreen/Gatekeeper warnings remain possible.
- Monitoring stops when the app quits. No missed-transition replay, embedded player/chat, advanced Streamlink transports, updater or legacy configuration import is included.
- A player that deliberately detaches from its owned process tree is outside normal cleanup guarantees.

See the README for runtime prerequisites and platform-specific installation details.
