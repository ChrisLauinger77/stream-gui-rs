# Phase 5 implementation and validation

Date: 2026-09-19. Baseline: clean `main` at `451b5320939a6c15d36186dacaf1127b3feffcda`; released `v0.1.0` resolves to `154260da596f4ccdf6f81ef9caef78281492eb28`. This records the Phase 5 development commit series, not a new release or proof about published artifacts. Version numbers remain 0.1.0. Phase 6 work remains deferred.

## Inspection and design

Before editing, reviewed repository status, AGENTS, current architecture/authentication/Helix/release documents, Phase 0–4 validation history, shared auth/HTTP/cache/rate ownership, process cleanup, Tauri window handling and existing native dependencies. The existing infrastructure already owned credentials, suspend-aware timing, foreground Following pages, cache/rate budgets, settings, and playback cleanup. The genuinely new components are the service-owned followed-live state machine, native notification adapters and tray/window behavior. The implementation plan was presented before edits.

The original [Streamlink Twitch GUI notification behavior](https://github.com/streamlink/streamlink-twitch-gui/wiki/Desktop-notifications) informed quiet startup, ephemeral pause and channel overrides. Current [Twitch followed-stream API](https://dev.twitch.tv/docs/api/reference/#get-followed-streams), [Tauri tray APIs](https://v2.tauri.app/api/system-tray/) and [Tauri desktop notification implementation](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/notification/src/desktop.rs) were inspected. Direct native adapters were chosen to support meaningful desktop permission/click handling. Their GIO, WinRT, objc2 and block2 dependencies already existed in the lockfile; no new package versions or Rust minimum-version increase were introduced.

See [architecture](architecture.md#background-monitor-and-native-lifecycle), [Helix contracts](helix.md#background-followed-live-scans), and [README](../README.md#background-monitoring-and-notifications) for product policies and precise resource bounds. Settings migrate strictly to version 4; new preferences default off. Pause is not persisted.

## Automated evidence

- Frontend production build/typecheck and 90 Vitest behavior/CSS tests passed.
- Backend unit suite: 132 tests, including strict settings migration, inheritance, baseline suppression, stream-ID dedup, duplicate pages/records, missing-result tolerance, new stream sessions, bounds, network/429/partial recovery, pause/resume cancellation, sleep between and during scans, low rate budget/reset/capacity, cache reuse, logout/session isolation and shutdown ownership.
- Actual build-script guard regression and 28 native fake-Streamlink lifecycle tests passed, preserving cleanup, restart, pipe draining and concurrent-session contracts.
- Generated DTO equality check passed. New commands are explicitly aligned across manifest, registration, IPC map, generated permissions and local-window capability.
- Desktop all-target check, strict Clippy, formatting and diff whitespace checks passed. The packaged-protocol debug/no-bundle build passed with the synthetic compilation-only public client ID.
- Linux native browser dispatch regression uses isolated XDG associations and the fake executable, never a real browser.
- Linux startup regression uses isolated malformed settings and synthetic configuration, never a real credential entry.
- Linux graphical `linux_background` regression runs two scenarios, each in a private `dbus-run-session`, with two fake native playback processes. It asserts normal minimize/restore, background close/fallback minimize, restore, webview reload retaining backend sessions, close-to-exit and explicit Quit. Parent success markers and `/proc` PID checks prove each scenario completed and children were reaped.
- The same graphical fixture hosts a synthetic freedesktop notification server. It verifies native GIO delivery, escaped title/body, advertised click action, restore, safe internal channel target, rejection of stale-session targets, cancellation/CloseNotification, ignored late clicks and no playback launched by notification activation. It also exposes a synthetic StatusNotifier watcher to check the distinction between a watcher and a registered host, hide with a registered host, and restore after the host disappears. No real desktop notification or Twitch request is sent.
- macOS and Windows native adapter source type-checks passed in temporary minimal harnesses against their actual dependencies (`aarch64-apple-darwin` and `x86_64-pc-windows-msvc`). These check API usage only, not a full cross-platform app build or native operation. A Windows GNU attempt was unavailable because the host lacks MinGW dlltool; the MSVC metadata check succeeded.

The Linux graphical check exposed a real GTK/Wayland issue: deiconify alone did not restore the minimized surface. Remapping that minimized surface before presenting it fixed the regression. Private-bus runs can emit benign portal/GVFS/indicator warnings; success depends on assertions and process reaping, not those logs. The fixture tests backend lifetime across an actual webview reload; React navigation/reconstruction behavior is covered separately by Vitest, not inferred from an empty development webview.

Commands (repository root):

```sh
npm run build
npm test
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --features test-support
cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets --features test-support
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets --features test-support -- -D warnings
cargo test --locked --manifest-path src-tauri/Cargo.toml --test linux_browser_open --features test-support
cargo test --locked --manifest-path src-tauri/Cargo.toml --test linux_startup --features test-support -- --ignored
cargo test --locked --manifest-path src-tauri/Cargo.toml --test linux_background --features test-support -- --ignored
TWITCH_CLIENT_ID_BUILD=ciCompileOnlyPublicClient123 npm run tauri build -- --debug --no-bundle --features custom-protocol --ci
git diff --check
```

## Native/manual gaps

No real Twitch sign-in, secure-store rotation, real stream transition, video/audio playback, human-visible desktop notification or manual tray-menu interaction was performed for Phase 5. Automated Linux tests use synthetic services and owned fake processes. Linux distro/notification-server variants, actual suspend/network changes and foreground rate responsiveness with real Twitch still need manual checks.

Full Windows/macOS native builds and installed-artifact tests were not run on this Linux host. Windows toast identity/Start-menu registration, denial/click activation and portable limitations require installed Windows testing. macOS bundle authorization, denial/re-enable, focus/menu-bar/Command-Q and both native architectures require macOS testing. Tray-host behavior still needs real desktop coverage beyond the synthetic watcher regression. Missing indicator libraries and action-less Linux notification servers also need native environment coverage beyond fallback logic and the synthetic action-capable server.

The [release smoke checklist](release-smoke-test.md#phase-5-background-behavior-releases-containing-phase-5) records these required observations. No push, tag, package publication or release was performed.

## Adversarial review follow-up — 2026-09-19

The four original Phase 5 commits (`7a7855e`, `17043cb`, `ee539f4`, `8ce41e5`) were preserved. Four focused follow-up commits address the confirmed findings:

| Commit | Finding and fix |
| --- | --- |
| `054b8cb` | An in-flight response could win the post-resume race against the heartbeat and erase the suspend boundary. Result acceptance now checks elapsed time against the heartbeat before updating `last_tick` or accepting transitions. Both response-first and heartbeat-first recovery discard historical transitions and establish a quiet baseline. |
| `1589457` | A pre-Pause fresh cache hit could establish an obsolete Resume baseline. Every quiet baseline now requests `Refresh` for all pages; ordinary scans retain `Fresh` cache reuse. Suspend recovery no longer globally invalidates live cache pages. |
| `7b62f7d` | Transient auth validation failure hid public identity and discarded deduplication. The monitor retains the original session ID/cancellation token and history across temporary unavailability, resetting on actual cancellation or replacement, including the same account's new login session. |
| `e5de28a` | Windows banner timeout retired handlers/targets still needed by Notification Center. Timeout now retains the target; a running COM activator and the banner callback share bounded, one-use opaque-ID lookup under the existing AUMID. Expiry, dismissal, cancellation and shutdown remove native handlers/history. A second instance cannot replace the first instance's route. |

Windows retention is limited to 32 records for 15 minutes, including sleep. No activation executable or arbitrary URL is registered; Quit removes the running activation route and factory. Startup clears orphaned history only after acquiring notification identity ownership. The Windows `implement` macro requires a direct optional `windows-core` dependency; version 0.61.2 was already in the lockfile, so this adds no package version. The registration follows the [Windows desktop AUMID activation model](https://github.com/CommunityToolkit/WindowsCommunityToolkit/blob/main/Microsoft.Toolkit.Uwp.Notifications/Toasts/Compat/ToastNotificationManagerCompat.cs) and [running COM class registration](https://learn.microsoft.com/en-us/windows/win32/com/registering-a-running-exe-server). Installed Windows behavior remains unverified here.

### Added regressions

These eleven new tests ran successfully on Linux:

- `helix::tests::monitor_tests::response_before_suspend_heartbeat_is_discarded_before_transition_acceptance`: establishes a baseline, gates HTTP, freezes the heartbeat timer, advances only the suspend-aware wall clock, releases HTTP first, and checks quiet recovery followed by a genuine notification.
- `monitor::tests::ordinary_slow_scan_keeps_a_current_heartbeat`: normal elapsed uptime with heartbeat progress does not trigger suspend recovery.
- `helix::tests::monitor_tests::resume_baseline_bypasses_pre_pause_cache_and_ordinary_scans_reuse_it`: keeps a warm pre-Pause cache, fetches the newly live channel quietly on Resume, then proves ordinary scans reuse a foreground cache refresh and notify only a later transition.
- `helix::tests::monitor_tests::suspend_baseline_bypasses_warm_cache_without_global_invalidation`: exercises the same observation boundary after suspend while deliberately leaving the shared cache warm.
- `twitch::tests::monitor_tests::transient_validation_failure_keeps_same_session_stream_history`: a previously notified stream is absent from the recovery baseline, then reappears with the same ID without a duplicate notification.
- `twitch::tests::monitor_tests::logout_and_other_account_login_reset_monitor_history`: cancellation and another account's new session receive isolated history.
- `twitch::tests::monitor_tests::logout_and_same_account_new_session_reset_monitor_history`: re-login to the same account also resets the old session's operational history.
- `desktop::notifications::windows_state::tests::banner_timeout_keeps_notification_center_target_and_activation_is_once`: timeout retains the correct target; subsequent activation consumes it once.
- `desktop::notifications::windows_state::tests::explicit_retirement_and_user_dismissal_release_native_ids_once`: explicit removal/dismissal returns each native cleanup ID once.
- `desktop::notifications::windows_state::tests::retention_is_bounded_and_expiration_includes_suspend`: capacity rejects a 33rd target, and elapsed sleep expires retained entries.
- `desktop::notifications::windows_state::tests::old_account_unknown_payload_and_shutdown_cannot_activate`: rejects cancelled sessions, another AUMID, unknown payloads and shutdown; draining shutdown leaves no retained targets.

The existing `helix::tests::monitor_tests::sleep_during_inflight_scan_discards_response_and_recovers_quietly` heartbeat-first regression also passed. HTTP gates and controllable clocks determine the new monitor race tests; a real-time deadline only bounds a failed test. Undelayed fixture HTTP responses no longer schedule a zero-duration timer, so a response can complete while the heartbeat timer remains frozen.

Two additional Windows-only tests were source/API checked, **not executed**:

- `desktop::notifications::windows::activation::tests::activation_owner_excludes_competitors_and_releases_on_drop`
- `desktop::notifications::windows::activation::tests::activation_payload_decoding_is_bounded_and_rejects_invalid_utf16`

The four platform-neutral Windows policy tests verify target lifetime and cleanup IDs; they do not prove Windows removes native handlers/history or invokes COM on a Notification Center click. The [Windows native acceptance procedure](release-smoke-test.md#windows-notification-center-acceptance) explicitly covers those observations, installed identity, expiry/suspend, old-account clicks, competing instances and Quit.

### Validation performed after the fixes

All commands in the earlier command block were rerun successfully for this follow-up, plus:

```sh
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib --features test-support
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib --features test-support desktop::notifications::windows_state::tests
```

| Check | Current result |
| --- | --- |
| Production frontend build and TypeScript | Passed |
| Frontend Vitest suite | 90 passed |
| Backend unit suite, desktop disabled | 139 passed, including generated DTO equality |
| Actual client-ID build-script guard | 1 passed |
| Native fake-Streamlink process lifecycle | 28 passed |
| Desktop-enabled library suite | 144 passed: the 139 backend tests plus 5 notification tests |
| Linux isolated browser regression | 2 passed |
| Linux graphical startup regression | 1 passed, covering malformed and unknown-version settings |
| Linux graphical background regression | 1 passed, containing both Close and Quit scenarios; 20.88 seconds |
| Formatting, all-target check and strict Clippy | Passed |
| Tauri debug/no-bundle custom-protocol build | Passed with the synthetic compilation-only public ID |
| Diff whitespace | Passed |

The Linux graphical background regression again used private D-Bus sessions, a synthetic notification server, isolated settings and two fake playback processes per scenario. Its native notification, safe navigation, tray-host loss, restore/reload and process-reaping assertions passed. No real Twitch sign-in, live playback, real desktop notification or manual menu/Notification Center test was performed. Private-bus portal/GVFS warnings did not fail the assertions.

Temporary minimal harnesses checked the actual Windows and macOS adapter source against its native dependencies. Windows `x86_64-pc-windows-msvc` all-target strict Clippy (including the two native helper tests) and macOS `aarch64-apple-darwin` source/API checking passed using the available Rust standard-library sources. These were metadata checks with `-Z build-std=std,panic_abort`, **not** full application builds, links or native execution. Linux has no installed Windows/macOS target runtime. The final Windows-only ownership guard was covered by the Windows check; the platform-neutral policy suite and formatting were rerun afterward.

### Final review and remaining acceptance

The cumulative `451b532..HEAD` implementation was inspected again for monitor completion races, baseline/cache behavior, session cancellation, bounded Windows target/handler ownership, shutdown and safe navigation. The fixes preserve the shared Helix/rate infrastructure, Rust authority, narrow IPC and settings v4 architecture. The original commits were not rewritten. No additional defect was confirmed in this focused pass; Windows shell behavior remains a native acceptance gap.

All test-owned playback/app processes exited; the graphical fixture checked recorded child PIDs, and a final host process-name check found no Stream GUI RS, background-smoke or fake-Streamlink process. Tests used temporary configuration and synthetic credentials; personal preferences and secure-store entries were not changed, so no restoration was needed. Temporary settings were cleaned up by their fixtures.

Windows still requires an installed-artifact build/run and the full Notification Center acceptance procedure linked above. macOS still requires installed-bundle notification permission/denial/re-enable, click focus, menu-bar controls and Command-Q cleanup on Apple Silicon and Intel. The earlier Linux real-service/desktop-variant gaps remain. No push, release, history rewrite or Phase 6 work was performed.

## Development-only notification acceptance follow-up

2026-09-19: added the opt-in debug/test `notification-acceptance` feature and [manual platform procedure](notification-acceptance.md). The fixed test target uses production native delivery and activation without Helix or monitor transitions. Normal builds omit the command and capability; the actual build-script regression rejects the feature in release/non-debug profiles, including configurations with debugging enabled.

Automated checks on Linux:

- Frontend build/type checking and 97 tests passed, including signed-out send/clear, native-action routing from Developer tools, acknowledgement/remount guards and rejection of disabled/non-test targets.
- Backend suite: 142 unit tests, 28 process lifecycle tests, two build-metadata tests and the build-script configuration regression passed; generated TypeScript matches Rust.
- Desktop tests: six passed in the default configuration; nine with `test-support,notification-acceptance`, including the real generated capability's local-main/remote access restrictions and Windows timeout/consume/test-only cancellation behavior.
- All-target check and Clippy with warnings denied passed with and without `notification-acceptance`; rustfmt and diff whitespace checks passed.
- The two Linux browser regressions and graphical startup regression passed. The graphical background regression passed both close and Quit scenarios with `test-support,notification-acceptance`: private D-Bus delivery of the fixed test, restore, synthetic action, acknowledgement, native cancellation cleanup, unchanged monitor/settings snapshots, and existing fake-process cleanup. Expected private-bus portal/GVFS warnings did not fail assertions.
- Tauri debug/custom-protocol no-bundle builds passed with and without the opt-in feature, using the synthetic compile-only public client ID.

No human-visible real-server notification, Twitch request, real playback, Windows/macOS native build or installed-artifact acceptance was performed for this follow-up. The graphical Linux test used the production adapter with a synthetic private server; frontend navigation was tested separately with mocked IPC. Windows Notification Center timeout/history/COM activation and macOS bundle behavior remain manual platform checks, now reachable with the developer action.
