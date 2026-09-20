# Phase 6 implementation and validation

Date: 2026-09-20. Baseline: `main` at `509ea811ff5de9d74dd75a187401dcb13b02f729` (published 0.2.0 documentation). This implements the approved Phase 6 request, including its added cross-platform About requirement, using [the planning research](phase-6-plan.md). Application/package versions remain **0.2.0**. No release, tag, push, or Phase 7 work is included.

## Delivered scope

| Area | Result |
| --- | --- |
| Settings | One strict version **5** schema. In-memory migrations from this app's versions 1–4 preserve prior values; new defaults are Any language, low latency off, 100% text, and inherited channel low latency. A successful atomic save writes v5. Invalid/unknown input remains untouched; 256 KiB and 1,000-record bounds remain. |
| Discovery language | Live/category Get Streams queries use a closed curated language enum, including Other, through existing Rust HTTP/cache/rate/session services. Language/category/session cursor scopes cannot be mixed accidentally. A filter change resets paging; Back restores the earlier visit's filter. Following, Search and the monitor remain unfiltered. |
| Exact lookup | A case-insensitive 1–25 character ASCII Twitch login uses session-bound Get Users and opens existing channel details, including offline channels. URLs and malformed input are rejected. No lookup autoplay or chat action. |
| Low latency | Global off; channel inherit/on/off. Rust resolves one immutable effective boolean and emits exactly one `--twitch-low-latency` flag when enabled. Existing quality/player arguments are preserved. Restart resolves current settings; other sessions remain unchanged. |
| Accessibility | Rust-persisted 100/125/150% text, rem text units, wrapping controls/cards, existing theme/focus contrast, panel opener/fallback focus restoration, explicit Close controls, meaningful card descriptions, and truthful process-level action feedback. |
| Support report | Previewable selectable text from a separate Rust allowlist: compile-time app version/commit, OS/architecture, known numeric Streamlink version or “not checked”, player mode, and at most sixteen anonymous phase/failure/exit summaries. Maximum 64 KiB; no automatic copy, save, upload or telemetry. |
| About | Tray/status-menu and main-interface intent. macOS keeps the existing native AppKit About panel, icon and link handling. Linux/Windows reuse one small modal in the existing main window, with icon/name/version/commit/fixed repository link. Hidden main windows restore. Parameterless repository IPC uses the existing native opener; no arbitrary URL permission. |

Support reports explicitly exclude account/channel identity, OAuth access/refresh/device credentials, usernames, absolute/settings/executable paths, player argument strings, environment, credential-store data, HTTP payloads, timestamps, PIDs, raw child output and arbitrary error strings. Full local Developer tools diagnostics are intentionally separate. Report preparation reads snapshots only: no executable probing, credential reads, environment inspection or Twitch HTTP. Its version cache is populated only by previously authorized normal probes/playback preparation and is associated with the configured path.

No packages/crates, lockfile changes, state frameworks, new process owners, Twitch clients/caches, downloads, transports, profiles, updater changes or deferred backlog features were added.

## Automated evidence

Host: Linux x86_64 graphical Wayland session, Node **24.20.0**, Rust **1.95.0**. Tests used synthetic credentials, local HTTP fixtures, temporary settings and native fake executables. No real Twitch authorization or personal credential-store entry was used for the automated checks.

| Check | Result |
| --- | --- |
| `npm run bindings` | Passed; generated TypeScript committed, with no residual diff on regeneration. |
| `npm run build` | Passed TypeScript checking and Vite production build. |
| `npm test` | **126 passed** across application/browsing/CSS suites. |
| `node --test scripts/verify-release-candidate.tests.mjs` | Passed (one Node test-file result containing the existing promotion-guard cases). |
| Backend `--no-default-features --features test-support` | **155 unit tests**, **30 native process tests**, **2 build-metadata integration tests**, **1 actual client-ID build-script test** passed. No failures. Platform-gated desktop/Windows cases are not counted as native passes here. |
| Desktop all-target `cargo check` | Passed with `test-support`. |
| Desktop unit subset | **10 passed** normally; **14 passed** with `notification-acceptance`. These include local-main-window capability restrictions for all six new commands. |
| `linux_browser_open` | **2 passed**, with isolated XDG handler dispatch and launcher reaping for Twitch chat and the fixed project repository; the helper never opens a real browser. |
| Formatting and strict Clippy | Passed; `-D warnings`. |
| Tauri custom-protocol debug/no-bundle build | Passed with the documented synthetic compilation-only public ID. No bundle or publishing. |
| `linux_background --ignored --test-threads=1` | **3 passed** in the graphical session; details below. |
| `linux_startup --ignored` | **1 passed**, checking malformed/unsupported settings are preserved and initialization exits cleanly. |
| `git diff --check` | Passed. |

Commands were run from the repository root following current CI. The non-publishing build was:

```sh
TWITCH_CLIENT_ID_BUILD=ciCompileOnlyPublicClient123 npm run tauri build -- --debug --no-bundle --features custom-protocol --ci
```

For the graphical Phase 6 webview regression, a local `npm run dev -- --host 127.0.0.1` server was running on port 1420. Run the graphical tests serially to avoid competing isolated accessibility buses:

```sh
cargo test --locked --manifest-path src-tauri/Cargo.toml --test linux_background --features test-support -- --ignored --test-threads=1
cargo test --locked --manifest-path src-tauri/Cargo.toml --test linux_startup --features test-support -- --ignored
```

The dev server and test processes were stopped after validation. Temporary diagnostic output and compiled artifacts are not committed.

## Regression and privacy coverage

- Settings migration/roundtrip, closed enum validation, sparse inheritance removal, invalid input preserving files, and low-latency global/channel/request-quality precedence.
- Language query parameters, Any/Other, cache reuse and explicit refresh, paging scope rejection across category/language/session, logout cancellation and same-account re-login isolation. Frontend late responses, paging reset, retained Back/focus, persistence failure, and a save finishing after navigation are covered.
- Exact normalized identity, case handling, no fuzzy substitution, empty results, malformed URLs/Unicode/control characters, original-session cancellation, delayed results after editing/unmount, duplicate submit and focus moved elsewhere.
- Low-latency argv matrix across every quality policy and player mode. Native processes verify old snapshots stay immutable, Restart uses current settings, and another session keeps its original PID/settings.
- Text-size persistence and accepted-save application, no preference localStorage authority, panel focus restoration after opener disappearance, deliberate later focus moves, pending-save closure, returned terminal process announcements, live/offline/unknown card descriptions, shortcut/modal suppression and selectable report text.
- Hostile excluded report fields include synthetic tokens/passwords/names, Unix/macOS/Windows paths, arguments, controls and very large raw child output. None enter the report. Tests cover deterministic formatting, sixteen-record/output bounds, numeric-only version components, missing version, typed failure/exit codes, and a counting credential store with **zero reads**. The native report probe fixture records exactly one explicit probe and no additional process launches during repeated report preparation.
- About repeated activation reuses/focuses one modal; metadata/error responses arriving after closure do not reopen it. Escape, focus restoration, fixed-opener dispatch, safe errors and an older-WebKit dialog fallback are covered. Generated capabilities reject remote and non-main origins.

Review found and fixed a Settings race: closing/reopening the form discarded its local pending-save guard. The guard now remains in the existing application playback/settings hook until the mutation completes. A regression first reproduced the duplicate dispatch, then passed after the fix. No second settings authority was introduced.

The first native UI run exposed an incorrect test wait that checked the old 100% value before Save finished; the test now waits for the actual form's pending state. The fixture also ensures an assertion failure still enters normal child cleanup. Final serial native results passed. Private test D-Bus runs emit portal/GVFS/indicator warnings; success is based on assertions and PID reaping, not those messages.

## Native evidence and remaining acceptance

**Linux automated native evidence:** the existing fixture exercises normal minimize/restore, close-to-background/fallback, notification activation/cancellation and quiet lifecycle ownership, synthetic tray-host loss/recovery, webview reload, close-to-exit and explicit Quit. Its native Wayland title-bar scenario still passes. Synthetic notifications run on a private `dbus-run-session`, not the user's notification server; fake playback processes are checked reaped after exit.

The added Phase 6 scenario loads the actual React app in **WebKitGTK**, opens About with the main window hidden, checks matching compiled metadata and loaded icon, repeats activation, verifies one main window/one modal, closes/reopens it, saves all three text sizes at the minimum 620×600 window size, and checks no root horizontal overflow. It also checks **200% webview zoom combined with 150% text** in Light and Dark themes, a selectable support preview, unchanged fake playback PIDs/monitor phase, and normal Quit with About still open. This is automated native webview evidence, not a claim of manual visual or assistive-technology acceptance.

**Not manually observed in this implementation run:** real Twitch Device Flow/storage/logout; real Streamlink/mpv/VLC rendered video/audio, Stop/Restart and perceived low-latency behavior; keyboard-only and pointer interaction through the physical tray; Orca speech/focus announcements; actual desktop display/DPI scaling; visual review of every populated browsing/channel/player-argument state. These remain explicit acceptance checks. Native zoom assertions do not prove desktop DPI or screen-reader behavior.

**macOS:** native AppKit code was preserved and now reads the shared compiled metadata. No macOS host was available; WKWebView, Keychain, tray/menu/native About activation and repository opening, VoiceOver, Command conventions, player bundles and real playback were not run. Linux execution of shared policy tests is not native macOS acceptance.

**Windows:** no Windows host was available; WebView2, tray/hidden-window About, repository opening, Narrator/high DPI, native executable/player behavior and real playback were not run. Source review and platform-neutral tests are not native Windows acceptance.

No accessibility certification or measured latency guarantee is claimed. These gaps must be completed for a future release acceptance gate; Phase 6 implementation did not release v0.3.0 or begin Phase 7.

## Commit series

| Commit | Purpose |
| --- | --- |
| `7836f4c` | Record approved planning research. |
| `9d3cacf` | Unified settings v5 and migrations. |
| `132a1ed` | Server-side language discovery and persisted default. |
| `737528f` | Exact-login lookup with session isolation. |
| `7ca9288` | Low-latency setting, argv and Restart behavior. |
| `956d265` | Text scaling, panel focus and process feedback. |
| `85e7ebd` | Safe report allowlist and preview. |
| `39cc18d` | Cross-platform About, fixed opener and native regression. |
| `2963cec` | Save guard surviving Settings panel closure. |
| `92a2e9a` | Native zoom and scalable-text checks. |
| `63f4875` | Live/offline/unknown accessible-description matrix. |

The final documentation commit records this evidence. README, architecture and the durable AGENTS schema/privacy map were updated. See Git history for their final hashes; no unrelated working-tree changes were discarded.
