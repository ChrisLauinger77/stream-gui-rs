# Phase 1 verification

## Local results — 2026-09-18

Platform: macOS / Apple Silicon. Toolchain: Rust 1.98.1 (Homebrew), Node 26.8.2. CI continues to select stable Rust and Node 22 on Linux, Windows and macOS.

| Check | Result |
| --- | --- |
| `npm test` | **7 passed** |
| `npm run build` | Passed TypeScript and Vite production build |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | Passed |
| `cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --features test-support` | **87 passed: 75 unit/HTTP/model tests + 12 native process integration tests** |
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
- Deterministic wall-only time advancement covers hourly validation, access expiry, all cache classes and Unix rate reset, including an already-waiting reservation. Backward clock steps neither revive cache entries nor postpone validation/reset deadlines.
- Frontend playback responsiveness during OAuth/probing, independent polling, restored account/scopes, device cancellation, late account completion after logout, and bounded account retry.
- All existing native Streamlink tests remain green; process/argument/log supervision modules were not changed for Phase 1. Service shutdown now also drains auth work concurrently with process cleanup.

## Focused cleanup regression tests

All 14 tests below passed. Names are exact Rust test paths:

| Contract | Test |
| --- | --- |
| Native delete reports success but retains the entry, or its subsequent read fails; refresh stops, logout fails, errors redact | `twitch::tests::unverified_native_deletion_blocks_refresh_and_logout_without_leaking_secrets` |
| Offline restoration then rejected expired access token; concurrent recovery rotates once | `twitch::tests::offline_restoration_recovers_expired_access_with_one_coordinated_refresh` |
| Explicit validation of expired access recovers | `twitch::tests::explicit_validation_at_expiry_refreshes_instead_of_clearing_credentials` |
| Rejected validation after refresh cannot trigger another refresh | `twitch::tests::rejected_new_refresh_token_validation_does_not_refresh_again` |
| Delayed original caller resumes after A refresh/logout/B login, with no B request or cache entry | `helix::tests::delayed_401_retry_cannot_cross_logout_and_new_login` |
| Delayed caller terminates after logout alone | `helix::tests::delayed_401_retry_terminates_after_logout_without_relogin` |
| Both clocks advancing count once; wall-only sleep counts | `time::tests::sleep_counts_once_when_wall_and_monotonic_both_advance` |
| Backward wall step cannot rewind elapsed time or stop uptime progress | `time::tests::backward_wall_step_never_rewinds_elapsed_or_stops_uptime_progress` |
| Hourly validation becomes due after sleep and stays bounded after backward adjustment | `twitch::tests::hourly_validation_counts_suspend_and_backward_clock_does_not_postpone_it` |
| Expired access refreshes before a post-suspend lease is issued | `twitch::tests::access_expiry_during_suspend_refreshes_before_issuing_a_lease` |
| All cache classes expire during sleep and cannot become fresh again after clock rewind | `helix::cache::tests::suspend_expires_all_cache_classes_and_backward_clock_cannot_revive_them` |
| Unix reset passes during sleep; only one conservative probe is admitted | `twitch_http::rate::tests::suspend_past_unix_reset_allows_one_probe_without_additional_uptime` |
| A reservation already waiting before suspend reconciles within one second of resumed uptime | `twitch_http::rate::tests::already_waiting_reservation_reconciles_suspend_within_one_second` |
| Backward clock plus repeated headers cannot extend the original reset wait | `twitch_http::rate::tests::backward_wall_clock_and_repeated_headers_do_not_extend_reset_wait` |

## Security and correctness review

The final diff was checked against credential ownership, IPC allowlists/CSP, auth concurrency, cross-process rotation ownership, request/body/query bounds, retry termination, cache isolation, and Phase 1 scope. Review fixes include coalescing startup validation, preserving identity across refresh, cancellation during grant validation, refusing refresh when secure deletion fails, draining rotation on normal exit, distinguishing incomplete batches from missing IDs, limiting retryable server statuses, and counting encoded query bytes correctly.

The focused cleanup verifies credential absence after deletion, recovers rejected access tokens through one coordinated refresh, binds complete Helix retries/batches to their original session, and reconciles freshness across suspend. No new dependency, frontend authority, IPC command, or playback change was introduced by this cleanup. Auth/storage serialization and shutdown draining remain intact; no new lock is held across an await.

The two additional IPC operations are cancel and authenticated account. The frontend receives public identity/status only, with no generic HTTP command or token-bearing DTO. The extra dependencies provide platform secure storage, an exclusive credential-entry lock, and cancellation tokens; the existing reqwest pool is shared. The CI Linux package list includes D-Bus development headers for Secret Service.

No Phase 2 browsing/search/channel/team UI, notifications, followed-stream polling, player expansion, legacy migration, or release machinery was added.

## Live macOS acceptance — 2026-09-18

Real provider/native-store testing successfully verified:

- Twitch Device Code login through the system browser.
- Authenticated account retrieval in the application.
- Login persistence across a complete application exit and restart, confirmed by the user.

The registered public Twitch application is named **stream-gui-rs**. The project is now **Stream GUI RS** / `stream-gui-rs`; the registered application name does not need to match the project name. The same public client ID was supplied to the backend on restart. This records the live acceptance already performed before cleanup and the application identity rename, not a live run under the new bundle/credential identity. Cleanup tests did not access or modify the real credential entry. The new identity requires one fresh login.

## Remaining verification gaps and tradeoffs

- **Live Twitch:** live token refresh/rotation, revocation, and the remaining online/offline/cancellation cases in [authentication](authentication.md) have not been manually verified. Successful login/account retrieval/restart does not establish these additional contracts.
- **OS stores:** macOS persistence is verified, but real Keychain deletion failure/denial and other lock/access-prompt failure cases remain unverified. Native Windows Credential Manager and Linux Secret Service behavior, including lock/unlock/session handling, remain unverified. All three adapters are explicitly configured; failures surface without an insecure fallback.
- **Suspend/resume:** deterministic injected-clock tests passed; no actual machine suspend/resume acceptance run was performed.
- **CI platforms:** the Linux/Windows/macOS workflow is retained, but no remote is configured and no hosted CI run was available. The complete app was built locally on macOS only. The installed Homebrew Rust toolchain contains only the Apple Silicon macOS target; a current cross-target check was not practical. Windows/Linux runtime behavior remains unverified. Phase 0's earlier Windows process-adapter cross-check is historical, not a Phase 1 Windows credential-store verification.
- **Rotating-token crash window:** the old pair is deleted before refresh. Force termination or an ambiguous response before saving the replacement requires sign-in again instead of replaying a possibly consumed token. Normal shutdown drains active rotations; OS credential prompts can delay exit.
- **Rate/cache policy:** the coordinator conservatively under-uses uncertain budgets. Cache TTLs and capacity are initial bounded policies, not a background polling system. Dynamic paginated streams may move between pages; consumers must deduplicate actual stream IDs.
- **Distribution:** runtime `TWITCH_CLIENT_ID` configuration remains required; packaging/signing and a project-owned public client ID are outside this phase.

Phase 1 code and local validation are complete. The above live/platform acceptance work remains explicit; Phase 2 has not begun.
