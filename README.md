# Stream GUI RS

[![Desktop checks](https://github.com/ChrisLauinger77/stream-gui-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/ChrisLauinger77/stream-gui-rs/actions/workflows/ci.yml)
[![Version](https://img.shields.io/github/v/release/ChrisLauinger77/stream-gui-rs)](<>)
[![Github All Releases](https://img.shields.io/github/downloads/ChrisLauinger77/stream-gui-rs/total.svg)](<>)
[![GPL-3.0](https://img.shields.io/github/license/ChrisLauinger77/stream-gui-rs)](LICENSE)

<img src="src/assets/app-icon.svg" alt="Stream GUI RS application icon" width="128">

Stream GUI RS is a native desktop application for browsing Twitch and watching live streams through [Streamlink](https://streamlink.github.io/). It uses a compact Tauri interface and launches video in a separately installed player. Twitch is currently the only supported streaming service.

This is an independent rewrite inspired by [Streamlink Twitch GUI](https://github.com/streamlink/streamlink-twitch-gui). It is not affiliated with Twitch or the original project.

![image](screenshot.png)

## Features

- Twitch Device Code sign-in with credentials stored in Keychain, Credential Manager, or Secret Service
- Following, popular live streams, categories, search, and channel details
- Streamlink 8.0+ discovery and external playback
- Streamlink default player, mpv, VLC, and custom executable modes
- Source, High, Medium, Low, and Audio quality policies
- Multiple simultaneous streams with Stop, Restart, Watching, and bounded diagnostics
- Persistent global playback settings and per-channel overrides
- Twitch chat in the system browser
- System, Light, and Dark themes plus focused application shortcuts
- Opt-in followed-stream monitoring, desktop notifications, and per-channel notification preferences
- Tray/menu-bar controls, Pause/Resume, and optional close-to-background behavior

Stream GUI RS does not bundle Streamlink or a media player. It does not contain an embedded player or chat client. Background features are available in 0.2.0 and later; v0.1.0 packages retain their original close-to-exit behavior.

## Downloads and platform support

Published releases provide these native artifacts:

| Platform   | Architecture                 | Artifact                                                   | Status                                                                                |
| ---------- | ---------------------------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| Windows 11 | x86_64                       | NSIS installer and portable/Scoop ZIP                      | Installer and portable package published                                              |
| Linux      | x86_64                       | AppImage, Debian `amd64` package, and RPM `x86_64` package | AppImage and Debian package natively tested; RPM installation remains a release check |
| macOS      | Universal (arm64 and x86_64) | Disk image (`.dmg`) containing the universal app           | Natively tested on Apple Silicon; native Intel execution remains a release check      |

ARM Linux/Windows packages are not currently published. The macOS disk image contains both Apple Silicon and Intel executable slices, but each architecture still requires native exact-artifact validation; cross-compilation alone is not treated as runtime proof.

The Windows installer is not code-signed, so Windows may show a SmartScreen warning. The macOS app bundle is ad-hoc signed but has no Developer ID signature or notarization, and Gatekeeper may block its first launch. Verify every downloaded file against the accompanying SHA-256 checksum.

## Runtime requirements

All platforms need:

- [Streamlink 8.0 or newer](https://streamlink.github.io/install.html), either on the desktop application's `PATH` or selected by absolute path in Settings
- a player supported by Streamlink; mpv and VLC have built-in discovery, or you can select another executable
- network access to Twitch and a Twitch account for browsing and playback launch

Platform requirements:

- **Windows:** Windows 11 x86_64 and the Microsoft Edge WebView2 Runtime. Windows 11 normally includes WebView2. Streamlink must be a native `streamlink.exe`; batch wrappers are rejected.
- **macOS:** macOS 11.0 or newer on Apple Silicon or Intel, with the system WebKit view and Keychain available. GUI applications can have a smaller `PATH` than Terminal, so select Streamlink/player paths in Settings when discovery does not find them.
- **Linux:** an x86_64 desktop, a user D-Bus session, and an unlocked Secret Service provider such as GNOME Keyring or KWallet. The Debian and RPM packages declare their native WebKitGTK and GTK dependencies. The AppImage bundles its application-side GTK/WebKit libraries but intentionally uses the host GLib, Wayland/Mesa, D-Bus, and Secret Service integration.

Secure credential storage is mandatory. There is no plaintext fallback. A locked or unavailable native credential store produces an explicit error.

## Install and connect

1. Download the artifact for your platform and its checksum file from [GitHub Releases](https://github.com/ChrisLauinger77/stream-gui-rs/releases).
2. Verify the SHA-256 checksum, then install or open the package.
3. Install Streamlink and a player if they are not already present.
4. Launch Stream GUI RS and select **Connect to Twitch**.
5. Open the Twitch verification page, enter the displayed code, and approve `user:read:follows`.
6. Open **Settings → Streamlink** to test discovery, then choose and test the player settings.
7. Browse a live channel and select **Watch**. Use **Watching** to stop or restart sessions.

Scoop users can install the portable Windows build from the author's bucket:

```powershell
scoop bucket add ChrisLauinger77 https://github.com/ChrisLauinger77/scoop-bucket
scoop install ChrisLauinger77/stream-gui-rs
```

Homebrew users can install the universal macOS build from the author's tap:

```sh
brew tap ChrisLauinger77/cask
brew install --cask stream-gui-rs
```

The Windows release also includes `stream-gui-rs.json` for direct Scoop installation. The bucket and cask definitions update from published GitHub Release assets.

Official installed builds contain the project's public Twitch application ID. Users do not set environment variables, register an application, or provide a client secret.

## Playback and privacy details

Rust owns Twitch credentials, settings, native processes, and session state. OAuth tokens stay outside the web interface, settings, diagnostics, and Streamlink arguments. Twitch browsing uses a bounded cache and explicit pagination. Streamlink configuration files and sideloaded plugins are disabled for launches from this app.

Navigation, interface reload, and Twitch logout leave existing streams running so they remain controllable from **Watching**. By default, closing the main window stops owned Streamlink/player process trees. With background close enabled, closing keeps monitoring and playback running; explicit Quit always cleans up owned processes. A custom player that deliberately detaches itself is outside that cleanup boundary.

The High, Medium, and Low selections prefer 720p30, 540p30, and 360p30 respectively, with source fallback when Twitch does not offer a matching rendition. They are preferences rather than guaranteed resolution caps.

## Background monitoring and notifications

In **Settings → Background**, enable **Monitor followed live streams**, choose a 1, 2, or 5 minute interval, and enable notifications. All background preferences default off, including when upgrading existing settings. Channel preferences can inherit, enable, or disable the global notification default. Monitoring still requires sign-in; notification permission is independent.

The first scan is quiet. Later newly observed Twitch stream IDs can notify with channel, title, and category. Pause stops polling until Resume or application restart. Resume, waking from sleep, and recovering from an outage establish a quiet baseline, without replaying missed transitions. Incomplete scans retain a marked previous count and retry with bounded backoff. Very large or slow scans can report incomplete monitoring; they never silently report a partial list as complete. At most ten eligible notifications are sent per scan; excess transitions are suppressed rather than queued into a later burst.

Notification clicks restore the app and select the channel while the original sign-in session remains valid; they do not start playback. **Watching** and **Quit** are available from the tray menu, and Quit is also in the main interface. Quit's label indicates active streams that it will stop.

**Keep Stream GUI RS running in the background when the window is closed** is opt-in. Normal minimize remains the OS minimize action. Background close hides the window when a tray is available, or minimizes it when there is no usable tray. Playback continues in both cases; restore the app to use Stop/Restart.

- **Linux:** notification delivery and click actions depend on the desktop notification server. The UI reports OS-managed delivery, not a permission grant. A StatusNotifier host and an Ayatana/AppIndicator library are needed for the tray; GNOME may need an indicator extension. A missing library/host falls back to minimize. Losing the host restores a hidden window.
- **Windows:** native toast notifications use the application's installed identity. After a banner times out, its Notification Center entry remains actionable for up to 15 minutes while the app and original monitoring/sign-in session remain active. Pause, logout and Quit retire those entries; notification clicks cannot reopen the app after Quit. Use the installer and its Start-menu shortcut for notification testing; an unregistered portable executable may not support delivery/activation. Installed Notification Center behavior still needs the [native acceptance checks](docs/release-smoke-test.md#windows-notification-center-acceptance).
- **macOS:** allow notifications explicitly in Settings, then manage denial in macOS System Settings. Notifications require an installed, correctly signed app bundle; a bare development executable reports unavailable. Authorization failures remain visible and can be retried after correcting the installation. See the [notification acceptance guide](docs/notification-acceptance.md) if no permission prompt appears.

See the [0.2.0 release checks](docs/release-readiness-0.2.0.md) for validation scope and native acceptance requirements.

## Current limitations

Stream GUI RS does not provide embedded video/chat, external chat applications, advanced Streamlink transports or player profiles, an updater, or legacy configuration import. Monitoring stops when the application is fully quit. Active sessions and logs are not persisted. See [architecture](docs/architecture.md) for the detailed contracts.

## Building from source

Build requirements are separate from the runtime requirements above:

- stable Rust 1.85 or newer
- Node.js 22.22.2+, 24.15.0+, or 26+ with npm; CI uses Node 24
- [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for the target platform
- on Debian/Ubuntu, `libayatana-appindicator3-dev` for tray support alongside the existing native prerequisites

Install locked frontend dependencies and start development with your own registered public Twitch client ID:

```sh
npm ci
TWITCH_CLIENT_ID=yourPublicClientId npm run tauri dev
```

Build a native release binary with an embedded public ID:

```sh
TWITCH_CLIENT_ID_BUILD=yourPublicClientId npm run tauri build -- --no-bundle --features custom-protocol
```

No client secret is used. The build fails if a distribution build has no valid embedded public ID. See [authentication](docs/authentication.md) for configuration precedence and PowerShell examples.

Builds embed a seven-character lowercase Git commit in native macOS About and backend diagnostics. CI/release jobs set `STREAM_GUI_RS_COMMIT` to GitHub's exact checkout SHA (including the merge commit for pull-request checks). Local builds use Git HEAD at build time when available; source archives or invalid metadata show `unknown`. An explicit variable takes precedence over local Git and accepts 7–64 hexadecimal characters. No Git installation or source checkout is needed at runtime. Local uncommitted changes are not reflected by a dirty suffix; the identifier describes HEAD. Normal users need no configuration, and application versioning still uses the existing release metadata.

Run the project checks from the repository root:

```sh
npm run build
npm test
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --features test-support
cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets --features test-support
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets --features test-support -- -D warnings
TWITCH_CLIENT_ID_BUILD=ciCompileOnlyPublicClient123 npm run tauri build -- --debug --no-bundle --features custom-protocol --ci
git diff --check
```

The synthetic ID in the final command is only for local compilation checks and cannot authenticate. Official release artifacts use the registered public ID from GitHub Actions. Tests use synthetic credentials, a native fake Streamlink executable, and loopback HTTP fixtures; they do not access real Twitch credentials.

For native notification acceptance without a Twitch live transition, build with the opt-in `notification-acceptance` feature and open **Settings → Developer tools → Send test notification**. It is restricted to debug/test builds; normal builds omit the command and permission. See [notification acceptance setup and platform steps](docs/notification-acceptance.md), including installed debug packages for Windows and macOS.

## Reporting issues

[Open a GitHub issue](https://github.com/ChrisLauinger77/stream-gui-rs/issues) with your operating system and version, Stream GUI RS version, Streamlink version, selected player, reproduction steps, and relevant sanitized session diagnostics. Do not paste OAuth tokens, refresh tokens, device codes, credential-store exports, or other secrets.

Release maintainers should use the [release process](docs/releasing.md) and [exact-artifact smoke checklist](docs/release-smoke-test.md). Historical implementation evidence remains in the phase validation documents; it is not proof of a current artifact.

## License and acknowledgements

Stream GUI RS is licensed under [GNU GPL version 3 only](LICENSE). It relies on open-source Rust and npm dependencies under their respective licenses. Playback is provided by the separately installed [Streamlink](https://streamlink.github.io/) project. No assets from Streamlink Twitch GUI are distributed here.
