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


## Adversarial-review cleanup — 2026-09-20

Cleanup starts at `e63289f` and adds focused commits without rewriting the original twelve-commit Phase 6 series:

| Commit | Fix |
| --- | --- |
| `9ab8c11` | Reconcile reopened Settings with accepted saves. |
| `e60483a` | Coordinate global/path/language settings mutations and accept successful snapshots in order. |
| `6641773` | Dismiss transient modals for accepted native navigation and focus the destination. |

The three approved MEDIUM/classification A findings are fixed:

1. **Reopened global draft:** forms now retain explicit field edits over the latest Rust snapshot, rather than copying one initial snapshot into a whole draft. Untouched fields update when an earlier save completes. Explicit nested player/background edits, including a deliberate change back to an original value, remain deliberate edits. A reopened form permits editing while the old save is pending, while Save/probe dispatch remains guarded. Failed saves preserve accepted settings and leave the active draft available for retry.
2. **Failed language attempts:** the existing application hook serializes global, path and language mutations, bounded to eight pending intents. Only a successful response advances the accepted settings revision and replaces the displayed snapshot. Failed attempts leave initialization and previous accepted values valid. A late initial read/error cannot supersede a newer successful mutation. Filter/panel remounts do not create another queue or settings authority; queued operations not yet dispatched are cancelled if their owning application component unmounts.
3. **Modal/native navigation:** session/workspace checks precede channel acceptance. Accepted channel/Watching actions dismiss About/support overlays, then focus the destination in an effect after modal unmount cleanup and native opener restoration. Repeated polls do not navigate, acknowledge or refocus again. Explicit navigation to the same destination still focuses it. Back/ordinary browsing shortcuts remain suppressed in modals; delayed exact lookup already defers navigation when modal focus represents newer user intent.

Production Rust, schema v5, IPC, generated DTOs, migration semantics, credentials, monitor and process ownership are unchanged. The only Rust edit extends the existing test-support graphical fixture. No dependency, version, release, push or Phase 7 changes were made.

### Deterministic regressions

The frontend baseline was 126 tests. Cleanup adds **23 cases**, for **149 passing tests**, and strengthens the existing close/reopen guard test by removing its masking Cancel click. The new cases use deferred IPC promises and existing fake timers, not arbitrary sleeps:

- Reopen during a pending save of low latency, then change only text scale and save; the accepted low-latency value survives.
- The same sequence for the Phase 5 monitoring preference.
- Reopen and edit background interval plus text scale before the old monitoring save completes; both deliberate edits and the accepted monitoring value survive.
- Explicitly toggle a reopened value back to its original value before reconciliation; that deliberate choice survives.
- Failed global save preserves accepted appearance and the editable draft/error, followed by successful retry.
- Failure after reopening preserves accepted values and new edits.
- Failed early language save followed by successful initial settings read applies saved appearance and finishes loading.
- German mutation succeeds after filter remount; the queued English mutation fails; later visits use German.
- Successful language mutation wins over a late initial read.
- Failed language mutation retains a previously accepted language.
- Language/global mutations in both orders preserve accepted appearance and background preferences.
- Queued language successes converge on the newer successful preference across navigation.
- A failed mutation releases the queue; subsequent success survives a late initialization error.
- Repeated filter remounts cannot retain more than eight mutation intents.
- About/support × channel/Watching matrix: modal dismissed, destination focused after simulated native opener restoration, one acknowledgement, no playback/monitor/Quit side effects, and no repeated focus stealing.
- Rejected stale-session channel action leaves either modal open without acknowledgement/navigation.
- About dismissal refocuses an already-selected channel or Watching destination.

The original review's failing sequences were reproduced before their corresponding fixes. Existing tests continue to pass.

### Cleanup validation results

All commands ran on Linux with Node 24.20.0 and Rust 1.95.0, using synthetic credentials, local fixtures and temporary settings:

| Check | Cleanup result |
| --- | --- |
| `npm run bindings` plus generated-file diff | Passed; generated file unchanged. |
| `npm run typecheck`, `npm run build` | Passed. |
| `npm test` | **149 passed** across three suites. |
| Candidate promotion guard tests | Passed; one Node test-file result. |
| Rust formatting | Passed. |
| Backend tests with `--no-default-features --features test-support` | **155 unit**, **30 process**, **2 build-info**, **1 client-ID build-script** tests passed. |
| Desktop all-target check | Passed. |
| Strict Clippy with `-D warnings` | Passed. |
| Desktop unit subset | **10 passed**; **14 passed** with `notification-acceptance`. |
| Isolated Linux browser regression | **2 passed**; no real browser opened. |
| Graphical `linux_background --ignored --test-threads=1` | **3 passed**. |
| Graphical `linux_startup --ignored` | **1 passed**. |
| Synthetic-ID Tauri custom-protocol debug/no-bundle build | Passed; synthetic compilation-only public ID, no bundle or publishing. |
| `git diff --check` | Passed. |

The graphical Phase 6 fixture again exercised hidden/repeated About, compiled metadata/icon, all text sizes at minimum window size, 200% WebKitGTK zoom with 150% text in Light/Dark, support preview, unchanged fake playback/monitor state, and Quit cleanup. It now also delivers native Watching actions while About and support-report dialogs are open, verifies each modal disappears, destination focus wins, and the action is acknowledged. Channel/session rejection and the full channel/Watching matrix are covered deterministically in frontend tests; no real Twitch identity was introduced into the graphical fixture. Synthetic notifications stayed on private D-Bus sessions. The local Vite server was stopped after validation; no repository application, test fixture or server process remained, and the native tests verified owned fake playback was reaped.

Manual/platform gaps remain unchanged: Linux real playback/low latency, physical tray/keyboard acceptance, Orca and actual desktop DPI; macOS native Phase 6/About/tray/link, playback, VoiceOver/scaling/focus and notification/background acceptance; Windows native Phase 6/About/tray/link, playback, Narrator/high DPI, console and Notification Center/background acceptance. Compilation and automated WebKit checks do not complete those gates.

### Additional channel-preferences finding — fixed

**MEDIUM — A, confirmed implementation defect, now resolved.** A global-save `settingsRevision` change in `ChannelPreferences` invalidated the pending channel mutation's generation, cleared its guard and reloaded its draft. If that reload returned old overrides, the successful channel result was ignored. A later unrelated channel save could silently restore those stale overrides.

The exact regression was retained in `src/app/Browsing.test.tsx`: channel low latency On save starts → global text-scale save completes while old channel overrides remain readable → channel save succeeds → change only channel notifications → save again, without Cancel/reset. Before the fix it failed with `lowLatency: null`; after the fix it submits `lowLatency: true`. Earlier isolated review reproduced the same defect on original Phase 6 commit `e63289f` and cleanup HEAD `906ae89`.

The bounded mutation coordination introduced in `e60483a` is reused in `src/settings/useSettings.tsx`, now owned by the window above browsing and Developer tools. Global/path/language mutations retain their serialized queue. Channel mutations share its eight-intent bound, keyed by broadcaster ID, with a guard against overlapping full drafts for one channel. Global and channel mutations remain independent and can succeed in either order. Only accepted successes advance the shared revision. Failures preserve accepted snapshots and deliberate edits.

Channel forms overlay explicit field edits on the accepted Rust snapshot. Global refreshes neither reset save guards nor invalidate a channel mutation. Reads wait for relevant pending writes, discard superseded reads/errors and obtain current effective previews from Rust. Navigation back to a pending channel waits for that save before loading the next draft. Broadcaster/session keys and unmount cancellation prevent late UI callbacks crossing channel, logout, account-replacement or same-account/new-session boundaries. Preferences still apply across accounts through a fresh Rust read; a previous session's success message is never replayed.

Production Rust, schema v5, commands/DTOs, capabilities and dependency versions are unchanged. One Rust regression was added against the existing `SettingsStore`; no backend revision or patch contract was needed. Effective quality, chat, notifications and low latency, atomic persistence and sparse record removal remain Rust-owned.

### Final settings-authority audit

The focused review from pre-Phase-6 `509ea81` covers every frontend persistence entry point:

| Entry point | Coordination and reconciliation |
| --- | --- |
| Global Playback/Player/Appearance/Background save | Shared global guard and queue; only accepted snapshots replace settings. Existing explicit field overlays preserve new edits through close/reopen. |
| Language filter | Same global queue, bounded across navigation; failures preserve initialization and prior accepted settings. Original filter callbacks remain scoped to their mounted view. |
| Settings Streamlink probe | Same global guard, including across closure; successful probe is followed by a Rust snapshot read before the next queued mutation. |
| Developer tools Streamlink probe | Now uses that same guard across switching to/from browsing. Its path is an explicit edit over accepted settings; late diagnostics do not reset it. |
| Channel overrides | Same coordinator capacity and success revision, independent broadcaster scope; read barriers and identity-bound forms preserve accepted overrides and deliberate edits. Same-channel remounts wait for the prior write. |

Initial reads, failed writes, late reads/errors, panel closure, application remounts, language navigation, same-channel and cross-channel changes were checked. Removing the old App `settingsRevision`/channel attempt-generation coupling leaves no independent settings attempt counter capable of invalidating accepted state. There is no new frontend persistence authority or effective-settings resolver.

### Additional regression coverage

The cleanup baseline was **149 frontend tests**. This fix adds **26 deterministic cases**, for **175 frontend tests**. Deferred IPC promises and existing fake timers control ordering; there are no arbitrary sleeps. Coverage includes the exact failure above, both global/channel completion orders, all four effective fields, failure after either kind of accepted mutation, a pending channel failure after global success, same-channel remount after success/failure, A→B isolation, fully inherited submissions during global saves in both orders, unsaved edits including an explicit return to inherit, logout/account/session replacement, stale refresh results/errors, late initial global reads after channel success, Developer tools probe guards/drafts/failures/remounts, language acceptance during channel saves and the shared eight-intent bound.

The additional Rust test uses two workers and barriers to exercise explicit overrides and full inheritance removal in both global/channel write orders. It reopens the file, checks global settings and the other channel remain intact, asserts all four effective values and verifies the fully inherited record is absent from persisted JSON. Backend unit count is now **156** (155 existing plus this regression); process count remains **30**.

### Final follow-up validation — 2026-09-20

All required commands were rerun on Linux with Node **24.20.0** and Rust **1.95.0** using synthetic credentials, loopback HTTP fixtures, temporary settings and fake native playback:

| Check | Result |
| --- | --- |
| Bindings generation and generated-file diff | Passed; `src/lib/generated.ts` unchanged. |
| TypeScript and production frontend build | Passed. |
| Full frontend suite | **175 passed** across three suites. |
| Candidate promotion guard tests | Passed; one Node test-file result. |
| Rust formatting | Passed. |
| Backend and integration suite | **156 unit**, **30 process**, **2 build-info**, **1 client-ID build-script** tests passed. |
| Desktop all-target check and strict Clippy | Passed with `test-support`; `-D warnings`. |
| Desktop unit subsets | **10 passed** normally; **14 passed** with `notification-acceptance`. |
| Isolated Linux browser regressions | **2 passed**; no real browser opened. |
| Graphical Linux background regressions | **3 passed**, serial `--ignored` run. |
| Graphical Linux startup regression | **1 passed**, `--ignored` run. |
| Synthetic-ID Tauri custom-protocol debug/no-bundle build | Passed; no packaging or publishing. |
| `git diff --check` | Passed. |

The graphical tests used the existing private D-Bus/fake notification and playback fixtures. All assertions passed; transient WebKit/portal/GVFS/indicator diagnostics did not fail the tests. The local Vite server was stopped, no repository application/test/server processes remained, and owned playback cleanup passed. No new manual/native acceptance is claimed: real playback/low latency, Orca/physical interaction/DPI and all macOS/Windows native acceptance gaps above remain unchanged.

The final settings-authority review found no further confirmed correctness defect. This follow-up changes no version, dependencies or release behavior, rewrites no existing commit, performs no push, and starts no Phase 7 work. Phase 6 cleanup is complete and ready to push for CI/native acceptance.

### Windows CI fixture correction — 2026-09-20

[Desktop checks run 35504984027, Windows job](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35504984027/job/106063156074) exposed a test-fixture portability error at `f5b74e9`. The support-report no-probe/no-credential-access test configured `/nonexistent/private/executable`, which is not an absolute Windows path. Settings validation rejected it before the report assertions ran; **155 backend tests passed and this one failed**. The frontend checks passed on all three runners.

The fixture now derives its nonexistent executable path from its temporary directory and explicitly asserts that it is absolute and absent. The report/privacy assertions remain intact. Production code, settings validation, IPC and dependencies are unchanged.

Local Linux revalidation passed all **3 support-report tests**, the full **156 backend / 30 process / 2 build-info / 1 client-ID** suite, formatting, strict all-target Clippy and `git diff --check`. The correction still requires a new Windows CI run; Linux results do not establish Windows native acceptance. No release or push was performed for this correction.


## Maintainer native report before 0.3.0 hardening

On 2026-09-20, the maintainer reported completed Phase 6 native testing on Linux, Windows and macOS, with real Twitch playback verified on all three and CI green. This is user-reported development evidence; exact package hashes, environment details and individual accessibility/privacy checks were not supplied with that report. It does not accept the subsequently built 0.3.0 candidate. That candidate uses the focused [release smoke checklist](release-smoke-test.md) and a run-specific result record.

## Candidate scrolling correction — 2026-09-21

The maintainer reported the first 0.3.0 candidate's native tests passed on Linux, Windows and macOS except a second scrollbar: scrolling a long channel list moved the header/navigation out of view and left blank space below the About footer. Candidate [35508256749](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35508256749), commit `9c7264e99067867bef60f9b324615bc4748f26ff`, is superseded by the correction. Detailed platform versions, screen-reader results and additional CPU coverage were not supplied; those are not inferred from the general pass report.

The absolutely positioned accessibility descriptions in stream cards/channel rows had no positioned ancestor inside the scrolling pane. Descriptions below its viewport expanded the outer document. Making `.browse-content` a positioning container keeps those descriptions within its existing overflow boundary. The header, sidebar and About footer retain their existing layout while only results scroll. No descriptions, keyboard behavior, settings, IPC, native permissions or dependencies are removed or changed.

The retained graphical WebKitGTK regression uses production CSS with forty synthetic stream cards or channel rows at 620/1120 logical-pixel window widths and 100/125/150% text. It verifies the document does not grow or scroll, the results pane does scroll, focusing its final Watch action reveals it inside that pane, and the header/footer remain at the viewport edges. The final regression failed with the original CSS and passed with the correction. It runs within the existing isolated Phase 6 fixture, with temporary settings, private D-Bus and fake playback; it does not access Twitch or the real credential store.

The maintainer also confirmed a previous Linux test build upgraded directly over 0.2.0 without logout/reset, preserving saved login and preferences and applying Any language, low latency Off/Inherit and 100% text defaults. No untouched v4 profile remains. This is prior-build evidence for unchanged migration behavior, not an exact replacement-package upgrade result. The new candidate still requires focused scrolling, text-size, keyboard/focus and About/build-identity acceptance on all three platforms.

Full local revalidation passed: **175 frontend, 156 backend, 30 process, 2 build-info, 1 client-ID, 10 normal desktop, 14 notification-feature desktop, 2 isolated browser, 3 graphical background and 1 graphical startup tests**, plus the Node candidate-promotion guard. Generated bindings remain unchanged. TypeScript/production frontend build, Rust formatting, all-target check, strict Clippy, synthetic-ID custom-protocol debug/no-bundle build, version consistency and diff whitespace passed. The graphical background count includes the twelve new layout cases. These are automated Linux results; replacement hosted CI, packaging and native acceptance are recorded separately against the final commit/run.

## macOS About repository link correction — 2026-09-21

The maintainer subsequently reported the native About URL on **macOS 27** looked like plain text and did not act as a clickable repository link, requesting the label “Github Repository.” The original credits carried an `NSLink` attribute with an `NSString` URL but no explicit link styling. The credits now use that label, the fixed repository destination as `NSURL` (Apple's preferred link value), the system link color and a single underline. The standard AppKit panel, shared menu/tray action, compiled version/commit, icon and fixed URL boundary are retained. Only feature declarations for existing Objective-C bindings were added; no package versions or lockfiles changed.

The macOS unit regression verifies the label and native URL, color and underline attributes across its entire range, plus the separate marketing version/build values. Apple documents [the preferred native URL attribute](https://developer.apple.com/documentation/appkit/nslinkattributename) and [explicit link styling](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/AttributedStrings/Tasks/ChangingAttrStrings.html). Linux validation cannot prove AppKit event handling. The replacement macOS candidate must demonstrate that clicking “Github Repository” opens the correct repository, including About invoked from the tray while the main window is hidden; this remains an explicit native gate.

After this macOS-only follow-up, bindings/lockfile consistency, all **175 frontend / 156 backend / 30 process / 2 build-info / 1 client-ID / 10 normal desktop / 14 notification-feature desktop** tests, the candidate guard, formatting, TypeScript/build, all-target check, strict Clippy and synthetic-ID native build passed again on Linux. The isolated Linux browser/background/startup results immediately above still cover the unchanged Linux implementation. Actual AppKit compilation and the strengthened native attribute test require the new hosted macOS run.
