# Phase 1 verification

## Local results — 2026-09-18

Platform: macOS / Apple Silicon. Toolchain: Rust 1.98.1 (Homebrew), Node 26.8.2. CI continues to select stable Rust and Node 22 on Linux, Windows and macOS.

| Check | Result |
| --- | --- |
| `npm test` | **7 passed** |
| `npm run build` | Passed TypeScript and Vite production build |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | Passed |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --features test-support` | **73 passed: 61 unit/HTTP/model tests + 12 native process integration tests** |
| `cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets --features test-support` | Passed with desktop enabled |
| `cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets --features test-support -- -D warnings` | Passed |
| `npm run tauri build -- --debug --no-bundle --ci` | Passed; native debug executable built |
| `git diff --check` | Passed |

Cargo checks used the locked, locally cached dependencies (`--offline`) after dependency fetch. Loopback HTTP tests ran with permission to bind localhost. They do not contact Twitch. Native credential adapters are exercised with a keyring mock; auth restart tests share an injected persistent test store across service instances. Tests never read or overwrite the user's OS credential entries.

## Contract coverage

- Device authorization start, provider request forms, pending/slow-down/denial/expiry, successful authorization, cancellation during start/poll/validation, and restricted browser URLs.
- Startup restoration, known-expiry refresh, hourly validation, rejected tokens, wrong client/account IDs, missing/additional scopes, one-use refresh rotation, uncertain outcomes, offline validation/recovery, offline logout, and deletion failure.
- Coalesced concurrent validation/refresh, simultaneous Helix 401 responses, repeated 401 invalidation, caller disappearance during rotation, logout during refresh or HTTP work, immediate safe status snapshots, and shutdown draining rotation.
- Rust-to-TypeScript DTO drift and token/device-secret redaction; OAuth never includes a client secret, including refresh forms with reserved characters.
- All 11 Helix endpoint paths and representative synthetic fixtures for all eight model families. Stream IDs remain independent of broadcaster IDs; user email is discarded.
- Headers/query encoding, body caps, malformed JSON/models, timeout, cancellation, network failure, terminal 4xx/429, transient 5xx retry limits, and no automatic OAuth exchange retry.
- Shared remaining/reset state, concurrent reservations, out-of-order responses, 429 waits, cancellation, and reset probing.
- Caller-driven cursor pagination, repeated parameters, empty/100/101/1,000 ID boundaries, duplicate IDs, percent-encoded URL limits, incomplete pages, missing data, and chunk failures.
- Cache miss/hit, class-specific expiry, explicit stale reads, invalidation, capacity/byte limits, and session isolation.
- Frontend playback responsiveness during OAuth/probing, independent polling, restored account/scopes, device cancellation, late account completion after logout, and bounded account retry.
- All existing native Streamlink tests remain green; process/argument/log supervision modules were not changed for Phase 1. Service shutdown now also drains auth work concurrently with process cleanup.

## Security and correctness review

The final diff was checked against credential ownership, IPC allowlists/CSP, auth concurrency, cross-process rotation ownership, request/body/query bounds, retry termination, cache isolation, and Phase 1 scope. Review fixes include coalescing startup validation, preserving identity across refresh, cancellation during grant validation, refusing refresh when secure deletion fails, draining rotation on normal exit, distinguishing incomplete batches from missing IDs, limiting retryable server statuses, and counting encoded query bytes correctly.

The two additional IPC operations are cancel and authenticated account. The frontend receives public identity/status only, with no generic HTTP command or token-bearing DTO. The extra dependencies provide platform secure storage, an exclusive credential-entry lock, and cancellation tokens; the existing reqwest pool is shared. The CI Linux package list includes D-Bus development headers for Secret Service.

No Phase 2 browsing/search/channel/team UI, notifications, followed-stream polling, player expansion, legacy migration, or release machinery was added.

## Remaining verification gaps and tradeoffs

- **Live Twitch:** no new public client ID/account was supplied. Real Device Flow consent, provider rotation and authenticated account retrieval still require the opt-in checklist in [authentication](authentication.md). Deterministic tests and a successful desktop build do not establish live provider acceptance.
- **OS stores:** real Keychain access prompts, Windows Credential Manager and Linux Secret Service lock/unlock/session behavior were not exercised. All three adapters are explicitly configured; failures surface without an insecure fallback.
- **CI platforms:** the Linux/Windows/macOS workflow is retained, but no remote is configured and no hosted CI run was available. The complete app was built locally on macOS only. Phase 0's earlier Windows process-adapter cross-check is historical, not a Phase 1 Windows credential-store verification.
- **Rotating-token crash window:** the old pair is deleted before refresh. Force termination or an ambiguous response before saving the replacement requires sign-in again instead of replaying a possibly consumed token. Normal shutdown drains active rotations; OS credential prompts can delay exit.
- **Rate/cache policy:** the coordinator conservatively under-uses uncertain budgets. Cache TTLs and capacity are initial bounded policies, not a background polling system. Dynamic paginated streams may move between pages; consumers must deduplicate actual stream IDs.
- **Distribution:** runtime `TWITCH_CLIENT_ID` configuration remains required; packaging/signing and a project-owned public client ID are outside this phase.

Phase 1 code and local validation are complete. The above live/platform acceptance work remains explicit; Phase 2 has not begun.
