# Phase 0 verification

This file records implementation checks and the boundaries requiring manual verification. No release has been published.

## Automated checks

The Rust suite covers version parsing, PATH/custom executable validation, argument construction, spawn failure, native stdout/stderr and partial lines, nonzero exit, reaping, repeated/concurrent stop, independent sessions, descendant pipes, bounded output/history, settings round trips/version rejection/atomic replacement, generated IPC DTO drift, authentication state transitions and real HTTP request/response contracts against localhost fixtures.

Frontend validation includes strict TypeScript checking, a Vite production build, and Vitest/DOM regression tests for delayed auth/probe requests, independent session polling, and per-session Stop availability. CI runs these plus Rust formatting, backend tests, desktop check, Clippy and a Tauri debug build on Linux, Windows and macOS. There is no release workflow.

Cleanup regressions cover shutdown during standalone/pre-launch probes, rejection of queued/new operations during shutdown, paths containing spaces, stable symlinks across version replacement (Unix), and delayed OAuth poll/refresh responses racing logout. Windows additionally checks that no child code runs before job assignment and that closing the job terminates an immediately spawned descendant.

## Manual configuration and unresolved verification

- **Twitch registration:** supply a newly registered public client ID via `TWITCH_CLIENT_ID`. Until then, real provider authorization, scope-free grant acceptance and live token rotation are unverified. See [authentication setup](authentication.md).
- **Playback:** install Streamlink and an external player. The probe does not prove a live stream can play; verify with an available Twitch channel and your player on each platform.
- **Credential persistence:** session-memory storage is intentional. OS keychain storage is not implemented.
- **Process portability:** Windows suspended-start/job ownership and Linux behavior need their respective CI runners and live desktop smoke tests. Detaching players and forced Unix app termination remain limitations documented in [architecture](architecture.md).
- **Project hosting:** the initial directory had no Git metadata or remote. CI is provided as source and will run once this project is placed in a GitHub repository.

## Local results (2026-09-18, macOS / Apple Silicon)

- `npm run build`: passed (TypeScript and Vite).
- `npm test`: **3 frontend concurrency tests passed**.
- `cargo fmt --all -- --check`: passed.
- `cargo test --no-default-features --features test-support`: **37 tests passed** (25 unit/HTTP-contract tests and 12 native-process integration tests).
- `cargo check --all-targets --features test-support`: passed with desktop enabled.
- `cargo clippy --all-targets --features test-support -- -D warnings`: passed.
- `npm run tauri build -- --debug --no-bundle --ci`: passed.
- The Windows platform adapter also passed a cross-target Rust check and Clippy (`x86_64-pc-windows-msvc`, compiling the actual module in a temporary check crate with a locally built standard library). This verifies types and lint only; it does not execute Windows process ownership tests or build the full Windows app.
- A local debug macOS `.app` was also built with a one-time CLI bundle override for native UI testing. The checked-in configuration still disables packaging.
- The real Rust probe discovered the installed Streamlink **8.6.1** on PATH.
- Native UI smoke test: backend/version IPC worked; auth was correctly marked unconfigured with disabled login controls; the real Streamlink probe succeeded; a native fake session launched, reported `running`, emitted stdout, stopped and reported `exited / stop requested`. The custom path was then cleared and successfully re-probed to restore real Streamlink discovery.

The OAuth HTTP fixtures required localhost network access outside the execution sandbox. The initial sandbox-only run failed on binding those sockets; the permitted rerun passed. No test required live Twitch credentials or a network stream. Linux and Windows CI have not been executed from this local workspace.
