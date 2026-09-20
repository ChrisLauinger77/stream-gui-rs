# Stream GUI RS 0.3.0

Better discovery, optional low-latency viewing, accessible controls and safer troubleshooting.

## Highlights

- Filter Live and category streams by language; Following stays unfiltered.
- Open an exact Twitch channel by login, including offline channels, without starting playback.
- Enable low-latency Streamlink playback globally or per channel. Changes apply on a new launch or Restart; running streams keep their current settings.
- Scale application text to 100%, 125% or 150%, with improved wrapping, keyboard focus and live/offline/unknown state descriptions.
- Preview and manually copy a support report containing only build/platform metadata, validated version information and anonymous process summaries. No paths, account/channel identities, arguments or raw logs are included.
- Open About from the main interface or tray/status menu on every platform: native AppKit on macOS, an application dialog on Linux/Windows, with matching version/commit and a fixed repository link.
- Preferences remain consistent across overlapping saves and reopened settings panels. Native navigation reveals its destination when a dialog was open.

## Installation and upgrade

Packages: Windows 11 x86_64 installer and portable/Scoop ZIP; Linux x86_64 AppImage, DEB and RPM; universal macOS DMG with Apple Silicon and Intel slices. Verify downloads against the accompanying SHA-256 files.

Streamlink 8.0+ and an external player remain separate requirements. Windows requires WebView2. Linux requires GTK/WebKitGTK dependencies, a user D-Bus session and unlocked Secret Service. macOS requires 11.0 or later, system WebKit and Keychain. Packaged builds include the public Twitch client ID; users need no environment variables or client secret.

Upgrade over 0.2.0 without clearing settings or signing out. Existing native credentials and playback paths/arguments, quality, chat, theme, channel overrides, monitoring, notifications and background behavior are preserved. New preferences default to Any language, low latency Off, per-channel low latency Inherit and 100% text size. Migration remains in memory until a settings save writes schema 5; downgrading to 0.2.0 after that save is unsupported. Older settings versions 1–3 remain supported for upgrade.

## Known limitations

- Low latency is a Streamlink preference, not a measured or guaranteed latency reduction. It does not replace the external player.
- Language filtering applies to Live/category discovery, not Following or search. Exact lookup accepts a Twitch login, not a URL.
- Linux tray/notification actions depend on the desktop; missing tray support falls back to minimizing. Windows notification activation uses the installed identity and an active original session; portable integration may be unavailable. Monitoring stops at explicit Quit.
- Windows packages are unsigned. macOS bundles are ad-hoc signed, without Developer ID signing or notarization; SmartScreen/Gatekeeper warnings remain possible.
- A deliberately detached player can escape normal owned-process cleanup.
- No embedded playback/chat, alternate Streamlink transports, automatic updater or localization is included. Accessibility improvements are not formal screen-reader certification.

See the README for prerequisites and platform installation details.
