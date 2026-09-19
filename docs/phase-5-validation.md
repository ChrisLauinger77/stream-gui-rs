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
