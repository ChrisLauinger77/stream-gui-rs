# Stream GUI RS

A modern Rust/Tauri desktop frontend for watching Twitch streams via Streamlink.

Twitch is currently the supported streaming-service integration. Rust owns native/backend functionality; React and TypeScript provide the frontend. Streamlink remains an external runtime dependency.

An independent rewrite inspired by [Streamlink Twitch GUI](https://github.com/streamlink/streamlink-twitch-gui). It is not affiliated with Twitch or the original project. The project identifier is `stream-gui-rs`.

**Phase 2:** authenticated Twitch browsing in a compact desktop shell: followed live streams and channels, popular streams, categories, search, and channel details. Rust owns Twitch HTTP, credentials, rate limits, and cache freshness. The Phase 0 playback prototype remains under Settings → Developer tools; browsing playback integration is reserved for Phase 3.

## Prerequisites

- Current stable Rust and Node.js 22.12+ (Node 22 LTS is used in CI), with npm.
- [Tauri 2 development prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform: Xcode command-line tools on macOS; MSVC build tools and WebView2 on Windows; WebKitGTK 4.1 and the documented system packages on Linux.
- [Streamlink](https://streamlink.github.io/install.html) installed separately, available on the desktop application's `PATH` or selected by an absolute executable path. Windows requires the native `streamlink.exe`, not a `.bat`/`.cmd` wrapper. Development smoke-tested with Streamlink 8.6.1; older versions are not certified.
- A Streamlink-supported external player (for example VLC) for actual playback. Phase 0 uses Streamlink's default player discovery, with all Streamlink configuration files disabled. There is no embedded player or custom player profile.

Linux, Windows and macOS are development targets and have a CI matrix. A matrix definition is not evidence of a completed run; see [Phase 2 verification and limitations](docs/phase-2-validation.md).

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

Stream previews are navigation to channel details, not playback controls. There is no background followed-stream monitoring.

### Isolated playback prototype

Under **Settings → Developer tools**, leave the custom path empty to discover Streamlink on `PATH`, or enter an absolute executable path. **Probe and save** checks `--version` with a five-second timeout. Enter an HTTPS Twitch channel URL and quality to launch. **Running means a process exists**, not that playback has been confirmed. Stop sessions or close the application to clean up owned processes; diagnostic output is bounded. Authentication remains independent of playback.

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

The tests use a native fake Streamlink executable and local HTTP fixtures; no Twitch credentials or live streams are required. The test-support feature is only for checks and is omitted from app builds. `--no-default-features` exercises the Tauri-independent backend without a WebView toolchain. Vitest tests cover browsing behavior, search races, bounded pagination, navigation focus, authentication, and the preserved developer prototype. See the [Phase 2 results and live smoke checklist](docs/phase-2-validation.md).

After editing Rust DTOs, run `npm run bindings` and commit the generated `src/lib/generated.ts`. A Rust contract test detects drift. Command names and their small TypeScript map live in `src/lib/ipc.ts` and must stay aligned with the Rust commands and capability allowlist.

## Scope

The pre-release identity rename uses new settings and credential namespaces: existing development settings are not imported, and one fresh Twitch login is required.

Version 1 settings persist only a custom Streamlink path in Tauri's platform-specific application configuration directory; its exact path is shown in the Backend panel. Unsupported or malformed settings fail without overwriting the file.

The Rust Helix client supports users, channels, follows, streams, games, search, and teams, with shared rate limits, caller-controlled pagination, bounded batching, and a bounded cache. Phase 2 exposes eight restricted browsing queries and safe account information. There is no teams UI, legacy migration, chat, embedded playback, notifications, tray, updater, or advanced player profiles. See [architecture](docs/architecture.md), [Helix contracts](docs/helix.md), [Phase 1 verification](docs/phase-1-validation.md), and the historical [Phase 0 verification](docs/phase-0-validation.md).
