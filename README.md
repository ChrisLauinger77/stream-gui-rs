# Stream GUI RS

[![Github Latest Releases](https://img.shields.io/github/downloads/ChrisLauinger77/stream-gui-rs/latest/total)](<>)
[![Version](https://img.shields.io/github/v/release/ChrisLauinger77/stream-gui-rs)](<>)
[![Github All Releases](https://img.shields.io/github/downloads/ChrisLauinger77/stream-gui-rs/total.svg)](<>)
[![license](https://img.shields.io/github/license/ChrisLauinger77/stream-gui-rs)](<>)

<img src="https://raw.githubusercontent.com/ChrisLauinger77/stream-gui-rs/main/src/assets/app-icon.svg" alt="App icon" width="128">

A modern Rust/Tauri desktop frontend for watching Twitch streams via Streamlink.

Twitch is currently the supported streaming-service integration. Rust owns native/backend functionality; React and TypeScript provide the frontend. Streamlink remains an external runtime dependency.

An independent rewrite inspired by [Streamlink Twitch GUI](https://github.com/streamlink/streamlink-twitch-gui). It is not affiliated with Twitch or the original project. The project identifier is `stream-gui-rs`.

**Phase 4 MVP:** browse Twitch and launch live streams in an external player through Streamlink, with persistent global/channel preferences, browser chat, themes and focused shortcuts. Watch from Following, Live, category streams, live search results, or channel details. Watching shows independent sessions with Stop, explicit quality/restart, and bounded diagnostics. Rust owns Twitch credentials, settings, processes and session state.

## Prerequisites

- Current stable Rust and Node.js 22.12+ (Node 22 LTS is used in CI), with npm.
- [Tauri 2 development prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform: Xcode command-line tools on macOS; MSVC build tools and WebView2 on Windows; WebKitGTK 4.1 and the documented system packages on Linux.
- [Streamlink](https://streamlink.github.io/install.html) installed separately, available on the desktop application's `PATH` or selected by an absolute executable path. Windows requires the native `streamlink.exe`, not a `.bat`/`.cmd` wrapper. Playback requires Streamlink 8.0+; verified with 8.5.0 on Linux and 8.6.1 on macOS.
- A separately installed external player: Streamlink default, mpv, VLC, or a custom executable. No player is bundled or downloaded.

Linux, Windows and macOS are development targets and have a CI matrix. A matrix definition is not evidence of a completed run; see [Phase 4 native Linux verification and remaining platform checks](docs/phase-4-validation.md).

## Development

```sh
npm ci
TWITCH_CLIENT_ID=yourPublicClientId npm run tauri dev
```

Replace `yourPublicClientId` with a registered public Twitch client ID. This runtime override is for development; installed builds include their own public ID. See [authentication and build configuration](docs/authentication.md) for Windows commands and precedence.

`npm run dev` alone opens a frontend preview with backend controls unavailable. No mock backend success is displayed. The desktop application serves its Vite frontend on `127.0.0.1:1420` during development.

Build a local desktop binary without release packaging:

```sh
TWITCH_CLIENT_ID_BUILD=yourPublicClientId npm run tauri build -- --no-bundle
```

The result is in `src-tauri/target/release/` and authenticates without any runtime environment configuration. Release builds require a valid `TWITCH_CLIENT_ID_BUILD`; packaged debug builds must additionally enable `custom-protocol`, which applies the same embedded-ID guard and distribution behavior. Setting only `TWITCH_CLIENT_ID` cannot satisfy that build check. The public ID is embedded only in Rust. Windows CI packages debug NSIS installers for manual smoke tests; release signing, publishing and updating workflows remain out of scope.

## Browsing workflow

1. Open the installed app, connect to Twitch and complete the device code flow in your browser. The build includes the project's public client ID; users do not register an app or set environment variables. Developers building from source follow [authentication setup](docs/authentication.md). No client secret is needed.
2. **Following** switches between live streams and all followed channels. **Live** shows popular streams. **Categories** opens a category's streams. **Search** has separate channel and category results.
3. Select a stream/channel to read its details. **Back** restores the previous view, scroll position and focused item where retained. **Load more** fetches one page; **Refresh** checks for current information. Retained results are visibly labeled.
4. Navigate with Tab and activate items with Enter. **Settings → Appearance** saves System, Light or Dark. **Settings → Shortcuts** lists application-local navigation keys.

Preview/channel buttons open details; the separate **Watch** button starts playback. Offline or unknown channels have no Watch action. Rust rechecks live identity before launching, so an old browsing result cannot supply an arbitrary URL or executable.

### Playback

1. Open **Settings → Streamlink**. Leave Streamlink's path empty for discovery, or enter its absolute executable path. **Test and save Streamlink path** reports the executable and version, with a five-second probe limit.
2. In **Player**, choose Streamlink default, mpv, VLC, or custom. mpv/VLC support automatic discovery or an explicit override; custom requires a full executable path. **Find installed players** reports discovery results. Save the settings.
3. In **Playback**, set a default quality: Source, High, Medium, Low, or Audio. High/Medium/Low prefer caps of 720p30/540p30/360p30 and fall back to the unfiltered source when no suitable rendition exists; they are policies, not a rendition menu.
4. Select **Watch**. Open **Watching** to stop or restart an individual session. Restart uses current saved global/channel preferences unless you explicitly select a restart quality. Saving settings never changes a running process. Repeated Watch actions after the initial launch may create separate sessions; there are up to eight active sessions and sixteen retained snapshots.
5. Expand a session's **Diagnostics** for bounded stdout/stderr output and exit details. Running reports a live Streamlink process, not confirmed video rendering.

**Channel details → Channel settings** offers independent quality and browser-chat overrides. “Use global default” removes an override; the saved effective values are reported by Rust. Preferences use the stable Twitch broadcaster ID and apply across accounts on this device. Global settings and channel forms each have explicit Save/Cancel behavior; section changes preserve the global draft, while closing Settings discards unsaved edits. Testing a Streamlink path saves that path separately.

**Open chat in browser** opens the selected channel's Twitch popout chat in the system browser, including offline channels. Enable automatic chat in Playback to open it on each launch or explicit restart; a channel can inherit, enable or disable it. Browser chat uses the browser's own Twitch login. No tokens are shared, and a browser-opening failure leaves playback usable.

While the app is focused, use Ctrl+K for Search, Ctrl+1/2/3 for Following/Live/Categories, Ctrl+4 for Watching, Ctrl+, for Settings, Alt+Left for Back, and Ctrl+R to refresh the current browsing view. macOS uses Command in place of Control and Command+[ for Back. Shortcuts pause in inputs, selectors, editable content and modal dialogs. There are no global hotkeys.

Player arguments use one literal argument per row (up to 32 / 4 KiB total). Spaces, quotes, braces, Unicode and empty arguments are preserved; do not add shell quoting around paths. There is no shell expansion or user template substitution. Streamlink configuration files and sideloaded plugins are disabled. The mpv preset suppresses progress-meter spam; VLC requests exit at the end and separate instances where supported.

Navigation, frontend reload and Twitch logout leave existing streams running. After logout, existing sessions can still be stopped or restarted; new launches from browsing require authentication. **Closing the main window or quitting stops owned playback and exits the application**. There is no tray/background mode. Intentionally detached custom players are outside the process-tree cleanup boundary.

Tokens are owned by Rust and persisted in macOS Keychain, Windows Credential Manager, or Linux Secret Service. Startup restores and validates the session. They are never sent to React, written to settings, or passed to Streamlink. Logout deletes local credentials and attempts remote revocation. Unavailable secure storage is an explicit error; there is no plaintext fallback.

## Checks

```sh
npm run build
npm test
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --features test-support
cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets --features test-support
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets --features test-support -- -D warnings
TWITCH_CLIENT_ID_BUILD=ciCompileOnlyPublicClient123 npm run tauri build -- --debug --no-bundle --features custom-protocol --ci
```

The final build command uses a synthetic public ID for compilation only; replace it with the project's registered public ID when building an app for use or distribution. Configuration tests cover embedded-only startup, developer precedence, missing/invalid values, and execute the actual build script to verify release and packaged-debug guards without contacting Twitch.

The tests use a native fake Streamlink executable and local HTTP fixtures; no Twitch credentials or live streams are required. The test-support feature is only for checks and is omitted from app builds. `--no-default-features` exercises the Tauri-independent backend without a WebView toolchain. Vitest tests cover browsing behavior, search races, bounded pagination, navigation focus, authentication, playback/settings behavior, and independent session controls. See [Phase 4 results](docs/phase-4-validation.md) and the preserved [Phase 3 results](docs/phase-3-validation.md). On Linux, also run `cargo test --locked --manifest-path src-tauri/Cargo.toml --test linux_browser_open --features test-support` for isolated desktop browser dispatch and launcher cleanup; it never opens the real browser.

After editing Rust DTOs, run `npm run bindings` and commit the generated `src/lib/generated.ts`. A Rust contract test detects drift. Command names and their small TypeScript map live in `src/lib/ipc.ts` and must stay aligned with the Rust commands and capability allowlist.

## Scope

The pre-release identity rename uses new settings and credential namespaces: existing development settings are not imported, and one fresh Twitch login is required.

Version 3 settings persist the Streamlink path, player mode/path/literal arguments, default quality, automatic browser chat, theme and sparse channel overrides in Tauri's application configuration directory (XDG configuration on Linux). This app's versions 1 and 2 migrate in memory, preserving their playback preferences; the next successful save writes version 3. Appearance previously stored in the webview starts at System; choose and save a new preference once. No legacy GUI configuration is imported. Active sessions and logs are never persisted. Unsupported or malformed schemas fail without overwriting the file; a missing configured executable produces a diagnostic at use.

The Rust Helix client supports users, channels, follows, streams, games, search, and teams, with shared rate limits, caller-controlled pagination, bounded batching, and a bounded cache. Phase 2 exposes eight restricted browsing queries and safe account information. There is no teams UI, legacy migration, embedded chat/video, external chat-client integration, background followed-stream monitoring, notifications, tray, updater, or advanced player profiles. Phase 5 has not begun. See [architecture](docs/architecture.md), [Helix contracts](docs/helix.md), [Phase 1 verification](docs/phase-1-validation.md), and the historical [Phase 0 verification](docs/phase-0-validation.md).
