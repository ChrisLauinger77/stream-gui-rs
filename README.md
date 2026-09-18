# Stream GUI RS

A modern Rust/Tauri desktop frontend for watching Twitch streams via Streamlink.

Twitch is currently the supported streaming-service integration. Rust owns native/backend functionality; React and TypeScript provide the frontend. Streamlink remains an external runtime dependency.

An independent rewrite inspired by [Streamlink Twitch GUI](https://github.com/streamlink/streamlink-twitch-gui). It is not affiliated with Twitch or the original project. The project identifier is `stream-gui-rs`.

**Phase 3:** browse Twitch and launch live streams in an external player through Streamlink. Watch from Following, Live, category streams, live search results, or channel details. Watching shows independent sessions with Stop, explicit quality/restart, and bounded diagnostics. Rust owns Twitch credentials, settings, processes and session state.

## Prerequisites

- Current stable Rust and Node.js 22.12+ (Node 22 LTS is used in CI), with npm.
- [Tauri 2 development prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform: Xcode command-line tools on macOS; MSVC build tools and WebView2 on Windows; WebKitGTK 4.1 and the documented system packages on Linux.
- [Streamlink](https://streamlink.github.io/install.html) installed separately, available on the desktop application's `PATH` or selected by an absolute executable path. Windows requires the native `streamlink.exe`, not a `.bat`/`.cmd` wrapper. Playback requires Streamlink 8.0+; verified locally with 8.6.1.
- A separately installed external player: Streamlink default, mpv, VLC, or a custom executable. No player is bundled or downloaded.

Linux, Windows and macOS are development targets and have a CI matrix. A matrix definition is not evidence of a completed run; see [Phase 3 verification and limitations](docs/phase-3-validation.md).

## Development

```sh
npm ci
npm run tauri dev
```

`npm run dev` alone opens a frontend preview with backend controls unavailable. No mock backend success is displayed. The desktop application serves its Vite frontend on `127.0.0.1:1420` during development.

Build a local desktop binary without release packaging:

```sh
npm run tauri build -- --no-bundle
```

The result is in `src-tauri/target/release/`. Bundling, signing, release publishing and updating are intentionally not configured.

## Browsing workflow

1. Configure your own public Twitch application via the backend-only `TWITCH_CLIENT_ID` environment variable; follow [authentication setup](docs/authentication.md). Connect to Twitch and complete the device code flow in your browser. No client secret is needed.
2. **Following** switches between live streams and all followed channels. **Live** shows popular streams. **Categories** opens a category's streams. **Search** has separate channel and category results.
3. Select a stream/channel to read its details. **Back** restores the previous view, scroll position and focused item where retained. **Load more** fetches one page; **Refresh** checks for current information. Retained results are visibly labeled.
4. Navigate with Tab and activate items with Enter. Settings offers system, dark and light appearance.

Preview/channel buttons open details; the separate **Watch** button starts playback. Offline or unknown channels have no Watch action. Rust rechecks live identity before launching, so an old browsing result cannot supply an arbitrary URL or executable.

### Playback

1. Open **Settings**. Leave Streamlink's path empty for discovery, or enter its absolute executable path. **Test and save Streamlink path** reports the executable and version, with a five-second probe limit.
2. Choose Streamlink default, mpv, VLC, or custom. mpv/VLC support automatic discovery or an explicit override; custom requires a full executable path. **Find installed players** reports discovery results. Save the settings.
3. Set a default quality: Source, High, Medium, Low, or Audio. High/Medium/Low prefer caps of 720p30/540p30/360p30 and fall back to the unfiltered source when no suitable rendition exists; they are policies, not a rendition menu.
4. Select **Watch**. Open **Watching** to stop or restart an individual session. Changing its quality only applies when **Restart** is pressed. Repeated Watch actions after the initial launch may create separate sessions; there are up to eight active sessions and sixteen retained snapshots.
5. Expand a session's **Diagnostics** for bounded stdout/stderr output and exit details. Running reports a live Streamlink process, not confirmed video rendering.

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
npm run tauri build -- --debug --no-bundle --ci
```

The tests use a native fake Streamlink executable and local HTTP fixtures; no Twitch credentials or live streams are required. The test-support feature is only for checks and is omitted from app builds. `--no-default-features` exercises the Tauri-independent backend without a WebView toolchain. Vitest tests cover browsing behavior, search races, bounded pagination, navigation focus, authentication, playback/settings behavior, and independent session controls. See the [Phase 3 results and real-player checklist](docs/phase-3-validation.md).

After editing Rust DTOs, run `npm run bindings` and commit the generated `src/lib/generated.ts`. A Rust contract test detects drift. Command names and their small TypeScript map live in `src/lib/ipc.ts` and must stay aligned with the Rust commands and capability allowlist.

## Scope

The pre-release identity rename uses new settings and credential namespaces: existing development settings are not imported, and one fresh Twitch login is required.

Version 2 settings persist the Streamlink path, player mode/path/literal arguments and default quality in Tauri's application configuration directory. This app's version 1 path is preserved on upgrade; no legacy GUI configuration is imported. Active sessions and logs are never persisted. Unsupported or malformed schemas fail without overwriting the file; a missing configured executable produces a diagnostic at use.

The Rust Helix client supports users, channels, follows, streams, games, search, and teams, with shared rate limits, caller-controlled pagination, bounded batching, and a bounded cache. Phase 2 exposes eight restricted browsing queries and safe account information. There is no teams UI, legacy migration, chat, embedded playback, notifications, tray, updater, or advanced player profiles. See [architecture](docs/architecture.md), [Helix contracts](docs/helix.md), [Phase 1 verification](docs/phase-1-validation.md), and the historical [Phase 0 verification](docs/phase-0-validation.md).
