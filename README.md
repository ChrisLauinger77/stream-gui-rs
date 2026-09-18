# Twitch GUI RS

An independent Rust/Tauri rewrite inspired by [Streamlink Twitch GUI](https://github.com/streamlink/streamlink-twitch-gui). It is not affiliated with Twitch or the original project. The project identifier is `twitch-gui-rs`.

**Phase 0 only:** a Tauri 2 + Rust + React + TypeScript + Vite foundation, a Twitch public-client authentication prototype, and a Streamlink process lifecycle prototype. The developer screen is temporary. This is not a feature-complete Twitch client.

## Prerequisites

- Current stable Rust and Node.js 22.12+ (Node 22 LTS is used in CI), with npm.
- [Tauri 2 development prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform: Xcode command-line tools on macOS; MSVC build tools and WebView2 on Windows; WebKitGTK 4.1 and the documented system packages on Linux.
- [Streamlink](https://streamlink.github.io/install.html) installed separately, available on the desktop application's `PATH` or selected by an absolute executable path. Windows requires the native `streamlink.exe`, not a `.bat`/`.cmd` wrapper. Development smoke-tested with Streamlink 8.6.1; older versions are not certified.
- A Streamlink-supported external player (for example VLC) for actual playback. Phase 0 uses Streamlink's default player discovery, with all Streamlink configuration files disabled. There is no embedded player or custom player profile.

Linux, Windows and macOS are development targets and have a CI matrix. A matrix definition is not evidence of a completed run; see [verification and limitations](docs/phase-0-validation.md).

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

## Prototype workflow

1. Leave the custom path empty to discover Streamlink on `PATH`, or enter its absolute executable path. **Probe and save** runs `--version` with a five-second timeout and saves the choice only if it succeeds.
2. Enter an HTTPS Twitch channel URL, choose a quality, and launch. Rust validates the arguments and executable. **Running means a process exists**, not that playback has been confirmed.
3. Stop a session, inspect its bounded diagnostic output, or close the app to stop all owned sessions. The backend supports independent session IDs; the screen lists each session.
4. Authentication is independent of playback. It requires your own **new public Twitch application registration**, configured via the backend-only `TWITCH_CLIENT_ID` environment variable. Follow [authentication setup](docs/authentication.md). No client secret is used, and credentials from the legacy GUI are never imported.

Tokens are held in Rust memory only. They are not sent to React, written to settings, or passed to Streamlink. Restarting requires login again. Logout clears local credentials and attempts remote revocation.

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

The tests use a native fake Streamlink executable and local HTTP fixtures; no Twitch credentials or live streams are required. The test-support feature is only for checks and is omitted from app builds. `--no-default-features` exercises the Tauri-independent backend without a WebView toolchain. The small UI has no frontend test runner; it is type-checked and built in CI.

After editing Rust DTOs, run `npm run bindings` and commit the generated `src/lib/generated.ts`. A Rust contract test detects drift. Command names and their small TypeScript map live in `src/lib/ipc.ts` and must stay aligned with the Rust commands and capability allowlist.

## Scope

Version 1 settings persist only a custom Streamlink path in Tauri's platform-specific application configuration directory; its exact path is shown in the Backend panel. Unsupported or malformed settings fail without overwriting the file.

There is no legacy migration, feature parity, chat, embedded playback, notifications, tray, teams, updater, or advanced player profiles. No Phase 1 work is included. See [architecture](docs/architecture.md) and [Phase 0 verification](docs/phase-0-validation.md).
