# Phase 0 verification

This file records implementation checks and the boundaries requiring manual verification. No release has been published.

## Automated checks

The Rust suite covers version parsing, PATH/custom executable validation, argument construction, spawn failure, native stdout/stderr and partial lines, nonzero exit, reaping, repeated/concurrent stop, independent sessions, descendant pipes, bounded output/history, settings round trips/version rejection/atomic replacement, generated IPC DTO drift, authentication state transitions and real HTTP request/response contracts against localhost fixtures.

Frontend validation consists of strict TypeScript checking and a Vite production build. CI runs Rust formatting, backend tests, desktop check, Clippy and a Tauri debug build on Linux, Windows and macOS. There is no release workflow.

## Manual configuration and unresolved verification

- **Twitch registration:** supply a newly registered public client ID via `TWITCH_CLIENT_ID`. Until then, real provider authorization, scope-free grant acceptance and live token rotation are unverified. See [authentication setup](authentication.md).
- **Playback:** install Streamlink and an external player. The probe does not prove a live stream can play; verify with an available Twitch channel and your player on each platform.
- **Credential persistence:** session-memory storage is intentional. OS keychain storage is not implemented.
- **Process portability:** Windows job ownership and Linux behavior need their respective CI runners and live desktop smoke tests. Detaching players, the small Windows spawn-to-job-assignment race, and forced Unix app termination are documented in [architecture](architecture.md).
- **Project hosting:** the initial directory had no Git metadata or remote. CI is provided as source and will run once this project is placed in a GitHub repository.

## Local results (2026-09-18, macOS / Apple Silicon)

- `npm run build`: passed (TypeScript and Vite).
- `cargo fmt --all -- --check`: passed.
- `cargo test --no-default-features --features test-support`: **32 tests passed** (24 unit/HTTP-contract tests and 8 native-process integration tests).
- `cargo check --all-targets --features test-support`: passed with desktop enabled.
- `cargo clippy --all-targets --features test-support -- -D warnings`: passed.
- `npm run tauri build -- --debug --no-bundle --ci`: passed.
- A local debug macOS `.app` was also built with a one-time CLI bundle override for native UI testing. The checked-in configuration still disables packaging.
- The real Rust probe discovered the installed Streamlink **8.6.1** on PATH.
- Native UI smoke test: backend/version IPC worked; auth was correctly marked unconfigured with disabled login controls; the real Streamlink probe succeeded; a native fake session launched, reported `running`, emitted stdout, stopped and reported `exited / stop requested`. The custom path was then cleared and successfully re-probed to restore real Streamlink discovery.

The OAuth HTTP fixtures required localhost network access outside the execution sandbox. The initial sandbox-only run failed on binding those sockets; the permitted rerun passed. No test required live Twitch credentials or a network stream. Linux and Windows CI have not been executed from this local workspace.
