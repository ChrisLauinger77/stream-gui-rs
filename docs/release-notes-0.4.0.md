# Stream GUI RS 0.4.0

Check for releases, open Twitch chat in Chatterino and switch between saved player configurations.

## Highlights

### Update awareness

- Check for stable releases manually in **Settings → Updates**. Versions are compared correctly, and a newer development build is never offered an older version as an upgrade.
- See **Checking for updates…** immediately, refresh a previous result and open the verified official GitHub release page in your browser.
- Checks are independent of Twitch sign-in. Nothing is downloaded or installed automatically, and no update checks run in the background.

### Chatterino

- Choose Chatterino as your external chat application, with automatic discovery or an explicit executable path. Browser chat remains available from channel details.
- Linux discovery also supports the official Chatterino Flatpak, `com.chatterino.chatterino`.
- Chatterino signs in independently; Stream GUI RS never forwards its Twitch credentials.
- Each channel request may open another Chatterino window/process, including when it is already running. Stream GUI RS does not attempt private IPC or process reuse. Chatterino stays open after Stop or Quit.

### Player profiles

- Save up to 16 named player configurations with stable identities, using the existing player modes, executable paths and literal argument controls. Profiles can also set quality and low-latency preferences.
- Playback uses global defaults, then the selected profile, channel overrides and finally any quality chosen for that launch.
- Switching, editing or deleting a profile leaves running playback unchanged. **Restart** applies current preferences; deleting the selected profile returns future launches to Default.
- Keyboard save/focus, update progress and chat error messages make these controls easier to use.

## Platform support and upgrade

Packages: Windows 11 x86_64 NSIS installer and portable/Scoop ZIP; Linux x86_64 AppImage, DEB and RPM; universal macOS DMG for **Apple Silicon and Intel**, requiring macOS **11.0 or later**. Verify downloads against the accompanying SHA-256 files.

Streamlink 8.0+ and an external player remain separate requirements. Chatterino is optional and must be installed and configured separately. Windows requires WebView2; Linux requires the documented GTK/WebKitGTK dependencies, a user D-Bus session and unlocked Secret Service; macOS uses system WebKit and Keychain. Packaged builds include the public Twitch client ID.

Upgrade over 0.3.0 without signing out or clearing settings. Schema 5 preferences migrate to schema 6 while preserving player settings, channel overrides, notifications/background behavior, language, low latency, text size and automatic chat. Existing native credentials keep the same storage identity. Browser chat remains the default, with no profiles selected or created. Update checks have no persisted preference and start unchecked.

Migration remains in memory until settings are saved. Downgrading to 0.3.0 after saving schema 6 is unsupported. Older settings schemas 1–4 remain supported for upgrade.

## Validation

Phase 7 native acceptance passed on Linux, macOS and Windows, including Chatterino from Flathub, Homebrew and WinGet respectively. Hosted desktop checks and CodeQL passed for the completed Phase 7 implementation. The release procedure additionally validates the versioned candidate and promotes its tested files without rebuilding.

## Known limitations

- Windows packages are unsigned. macOS bundles have a complete ad-hoc signature, without Developer ID signing or notarization; platform warnings may appear.
- Tray and notification integration depends on the desktop. Explicit Quit stops monitoring and owned playback; intentionally detached players can escape normal cleanup.
- Low latency is a playback preference, not a measured or guaranteed delay reduction. Native screen-reader and high-DPI coverage is not exhaustive.
- No embedded playback/chat, automatic updater, alternate transports or localization is included.

See the README for prerequisites and installation details.
