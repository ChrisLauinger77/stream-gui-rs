# Phase 8 validation — advanced discovery and desktop integration

Implementation date: 2026-09-22. Baseline: completed Phase 7 / v0.4.0,
`0c5397ca6cc1b2605d176f81f0f5f552d27c6069`, with a clean `main` matching
`origin/main` before work. Implementation branch: `codex/phase-8-discovery`.

The implementation, hosted checks and user-reported Linux, macOS and Windows native
results are described below. All three native acceptance gates have reported passes;
PR readiness additionally requires green hosted checks on its current HEAD. No version
bump, release preparation, tag, publication or later-phase work is part of this change.

## Preparation and decisions

Before editing, the repository guide, status/HEAD, README, architecture, Phase 6/7
validation, current settings/coordinator, browsing snapshots, shortcuts, native
lifecycle, IPC permissions and existing tests were inspected. The plan selected one
settings migration, existing services and view components, bounded navigation
intents and narrow IPC, followed by review and automated/native validation.

- Teams enter through Search → Teams and typed links, avoiding another permanent
  sidebar section. Twitch supports exact team-name lookup, not team search or member
  pagination. [Official Get Teams reference](https://dev.twitch.tv/docs/api/reference/#get-teams).
- Refresh-on-focus is deferred. Frontend navigation snapshots do not own cache
  freshness; existing explicit Refresh avoids network churn and focus resets.
- A separate CLI is deferred. The forwarding plugin does not provide an
  application-level acknowledgement/result exit status. The executable's one URI
  argument is an OS protocol entry point, with no verbs, playback or settings API.
- Support-report fields were reviewed and left unchanged. Counts are not necessary
  for these features. No names, IDs, shortcut contents or link history were added.
- The only new direct dependency is `tauri-plugin-single-instance` (locked 2.4.5),
  optional under `desktop` and restricted to Linux/Windows. It supplies the existing
  native forwarding mechanism instead of a custom server. Its Linux dependencies
  add the zbus 5 stack alongside existing dependencies; Cargo also re-resolved some
  compatible Windows dependency edges. No frontend dependency was added.
- macOS uses LaunchServices and Tauri `Opened`/`Reopen` events. Review of the plugin's
  macOS source found `read_to_string` without a bound on its extra Unix socket, so
  that implementation is not compiled. Native installed-bundle activation avoids
  that extra receiver; direct concurrent launches of bare macOS binaries are outside
  this contract. [Apple activation semantics](https://developer.apple.com/documentation/appkit/nsworkspace/openconfiguration/createsnewapplicationinstance),
  [Tauri deep links](https://v2.tauri.app/plugin/deep-linking/),
  [Tauri single instance](https://v2.tauri.app/plugin/single-instance/).

## Implemented contracts

### Settings, bookmarks and hides

Schema **7** is the sole revision. Versions **1–6** migrate in memory; only a
successful save writes v7. v6 adds empty discovery lists and platform shortcut
defaults. Old versions reject smuggled new fields. Unknown versions/fields and
malformed settings remain errors without overwrite. Atomic replacement, the
256-KiB file bound and 1,000 sparse channel override bound remain intact.

Each discovery list supports 200 channel/category entries, identified by kind plus
stable positive decimal Twitch ID (1–32 digits, no leading zero). A saved label is
presentation only: 1–128 characters, at most 256 UTF-8 bytes, trimmed, with controls
and bidi overrides/isolates rejected. Add/remove is idempotent; duplicate persisted
IDs are rejected. Global playback drafts preserve discovery, shortcuts and profiles
atomically. Services use the existing operation lock and blocking persistence; the
frontend shares the existing bounded mutation coordinator across remounts.

Bookmarks are local preferences, not Twitch follows, and remain available across
accounts. Channel/category details provide accessible Bookmark/Hide buttons;
Bookmarks opens normal detail views and can remove stale IDs without HTTP.
Settings → Hidden items restores entries without resolving Twitch identities.

| View | Hide behavior |
| --- | --- |
| Live | Excludes hidden channels and streams in hidden categories |
| Categories | Excludes hidden categories |
| Category streams | Excludes hidden channels |
| Following, Search, Teams | No passive hide filtering |
| Bookmarks, exact lookup, deep links | Explicit destinations remain reachable |
| Direct hidden category | Opens the category; hidden channels within it stay filtered |

Filtering does not mutate retained query snapshots, provider cursors or freshness.
An empty filtered page can still Load more. Monitoring, notifications and running
playback are unaffected.

### Teams and navigation

`get_team` uses the shared authenticated Helix client, rate budget, session binding,
metadata cache, response-body bound and foreground permit. It accepts an exact
1–100-character ASCII alphanumeric/underscore/hyphen name, normalized to lowercase.
Provider metadata is bounded (description 16 KiB, display name 256 bytes, at most
10,000 input members under the existing body limit). Members are validated, sorted
by login, deduplicated by ID and limited to 300 displayed members with a visible
limit notice. No invented provider cursor or unbounded scan is used.

Description markup is rendered literally as text; only validated CDN images survive.
Membership gives **unknown** live status. Opening a member uses existing channel
resolution/details and Watch, preserving auth/session/playback boundaries. Missing,
empty, failed and retained data remain distinct.

Home selects Following. Back/Forward share at most twelve history entries and reuse
the existing twelve retained view snapshots and ten-page/300-item list bounds.
Visits retain route, search query/type, Following mode, category language, scroll and
meaningful focus. New navigation or search/type/language changes clear Forward.
Restoration uses a present enabled control or the heading; filtered/removed items do
not send focus to a hidden element. Evicted/pending results reload through Rust;
late responses cannot change a newer route or replacement account workspace.

### Shortcuts

The ten typed actions are Home, Following, Live, Categories, Search, Watching,
Settings, Back, Forward and Refresh. Bindings contain a key and modifier booleans;
there is no executable text. All actions must exist in the stored map; null explicitly
unassigns an action. Defaults preserve the primary existing shortcuts and add Home
and Forward. Primary is Command on macOS and Ctrl on Linux/Windows.

Modified ASCII letters/digits, comma/brackets, arrows, Home and F6–F12 are supported;
Q/W/N/T/L are reserved. Modifier-only and unsupported input cannot save. Conflicts
are checked under both platform modifier interpretations, displayed before saving,
and rejected again in Rust. Reset changes the draft; Save persists. Escape cancels
capture, Tab exits normally, labels/statuses are readable and global handling pauses
in editable inputs, composition/repeats and modal dialogs. OS-reserved combinations
may be intercepted before reaching the app.

A confirmed review regression allowed a reopened editor to edit an old full map while
a previous save was pending. A coordinator-owned guard now disables the reopened
editor until that save settles; a regression proves subsequent edits preserve the
accepted prior binding.

### Deep links and native delivery

```text
stream-gui-rs://show
stream-gui-rs://channel/<login>
stream-gui-rs://category/<positive-decimal-id>
stream-gui-rs://team/<team-name>
```

The Rust parser permits at most 256 ASCII bytes. Channel logins contain 1–25 letters,
digits or underscores; category IDs follow the stable-ID rule; team names follow
the team lookup rule. Scheme/action spelling is fixed; login/team case normalizes.
Unknown actions, percent escapes, Unicode identifiers, userinfo, ports, queries,
fragments, extra segments, dot segments and whitespace are rejected as supplied to
the parser. Errors never echo the input. macOS supplies already parsed OS URLs, so
raw lexical spelling before OS normalization is not available to this adapter.

The one-entry Rust inbox is installed before native initialization, including cold
input. Latest valid intent wins. UUID-matched acknowledgement cannot remove a newer
intent. The frontend waits for settings/auth restoration and its session workspace;
signed-out browsing links wait for sign-in. A route-owned login lookup prevents late
resolution from overwriting newer navigation. Failed acknowledgement retries without
repeating navigation. Links only show/navigate: no autoplay, arbitrary URL, process,
path, credential, setting mutation or generic payload is accepted. Nothing persists
as link history.

Linux/Windows register the forwarding plugin before service/credential startup. A
second valid launch forwards and exits; the existing window is restored on its native
thread. macOS installed bundles use native URL/reopen events, processing at most
sixteen URLs per event and retaining only the last valid intent. Packaging protocol
metadata comes from `plugins.deep-link.desktop`; no runtime association writes or
JavaScript plugin permissions are required. Development binaries and unintegrated
AppImages/portable Windows copies do not automatically become OS protocol handlers.

The new IPC is limited to `get_team`, `modify_discovery`, `save_shortcuts` and
`acknowledge_navigation_intent`; `desktop_status` adds a safe pending-intent DTO.
Registration, build manifest, generated permissions, local-window capability, typed
command map and generated DTOs are aligned. CSP is unchanged.

## Adversarial review

A read-only review of the complete Phase 8 implementation covered settings races,
input injection, startup and replacement intent races, duplicate-instance handling,
shortcut conflicts, navigation/history bounds, hide precedence, session replacement,
IPC/capabilities, support-report privacy and existing process/auth ownership.
Confirmed findings were fixed and rechecked: the shortcut remount race above, the
unbounded macOS plugin receiver avoided by native activation, and development-server
watch exhaustion from watching Cargo output (Vite now ignores `src-tauri`). No
unrelated cleanup or speculative framework changes were made.

Focused regressions exercise list bounds/duplicates and malformed schemas, competing
global/profile/channel/shortcut writes, stale bookmark deletion, hide during pending
pagination, bookmark changes during failed refresh, navigation while fetching,
Back/Forward focus/scroll, same-user auth replacement during Teams HTTP, logout/login
preference retention, shortcut save remounts, hostile links, 1,000 inbox replacements,
old acknowledgements, startup auth/settings gating, superseded login resolution and
failed acknowledgement retry. Synthetic fixtures are used throughout.

## Automated validation

Run from the repository root on graphical Linux. Initial fixture/dev-server failures
were diagnosed and corrected before recording passes. Existing ts-rs notices about
unsupported serde attributes remain; Rust serde validation still applies.

| Check | Result |
| --- | --- |
| `npm run bindings` | Passed; generated DTOs and application permissions included |
| `npm run build` | Passed, including TypeScript and production Vite build |
| `npm test` | **221 passed** across three files |
| `node --test scripts/verify-release-candidate.tests.mjs` | Passed |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | Passed |
| Backend command from AGENTS, no default features + test-support | **181 unit**, **42 process**, **2 build-info**, **1 client-ID build guard** passed |
| All-target desktop check + test-support | Passed |
| Desktop unit tests | **10 passed** |
| Desktop unit tests with notification-acceptance | **14 passed** |
| Linux browser dispatch | **2 passed**, isolated native launcher/reaping |
| Strict all-target Clippy | Passed |
| Linux startup, explicitly `--ignored` | **1 passed**, malformed settings isolated from the normal instance |
| Linux background, explicitly `--ignored --test-threads=1` | **5 passed**, including Phase 6/7/8 and Wayland titlebar |
| Non-publishing Tauri custom-protocol debug build | Passed using `ciCompileOnlyPublicClient123` |
| Final generated-file/permission consistency and `git diff --check` | Passed; regeneration is byte-for-byte stable |
| Hosted Linux/macOS/Windows Desktop checks and CodeQL | Initially not run under the local-only instruction; see the PR finalization record below |

## Native evidence and remaining acceptance

The Linux tests used a graphical session, real GTK/WebKitGTK windows, isolated
`dbus-run-session`, temporary XDG settings/data/cache and a synthetic notification
server. Phase 8 used synthetic public browse DTOs injected through the dev module;
settings, desktop status, persistence, acknowledgements, window activation and native
processes stayed real. These fixtures do not access real Twitch credentials.

Phase 8 observed Teams/member navigation, bookmark add/remove, hide/restore,
Back/Forward focus, shortcut capture/save/activation, signed-out startup retention,
first ready navigation, visible and hidden-window forwarding from a second actual
application process, one retained main window, two unchanged fake playback sessions
and Quit reaping. Teams, bookmarks, channel actions/navigation and shortcuts were
checked at 100/125/150% text scale, System/Light/Dark, and 100/200% WebKit zoom at the
620-pixel minimum width. These are automated native DOM/geometry/focus checks;
physical keyboard/screen-reader usability was not manually observed.

Existing Phase 6/7 graphical regressions also passed for About/support reports,
profiles/chat/update presentation and background restore/reload/Quit. The titlebar
scenario ran through the Wayland backend; Phase 8 itself used the configured X11 GTK
backend in this graphical session. No real notification server was substituted.

Evidence outstanding after the initial local run, before PR finalization:

- Linux installed package protocol association and actual OS URI cold launch; the
  current native fixture covers startup inbox delivery and actual process forwarding,
  not installer registration. Real Twitch Teams and real Streamlink/mpv/VLC audiovisual
  playback, keyboard-only/screen-reader acceptance remain unobserved in this run.
- macOS native WKWebView, Command semantics, installed app-bundle protocol
  cold/running/hidden activation, About/status item and Command-Q cleanup. No macOS
  runtime is available on this host.
- Windows WebView2, installed protocol cold/running/hidden activation, keyboard,
  no-console behavior, tray/background and Quit cleanup. No Windows runtime is
  available on this host.
- Current-commit hosted checks and installed native acceptance. The initial
  local-only decision was superseded by the explicit PR finalization request below;
  baseline hosted success is not evidence for this branch.

## PR finalization — 2026-09-23

The user subsequently authorized a normal push and PR finalization, superseding the
earlier local-only decision. Before pushing, `codex/phase-8-discovery` was clean at
`1e64c427916655d12573225101e126e709c3be44`, with no untracked files. Local and remote
`main` matched `0c5397ca6cc1b2605d176f81f0f5f552d27c6069`. The existing platform-badge
commit `7ae118a` was preserved. Package metadata, both lockfiles and Tauri configuration
still declared 0.4.0. No history was rewritten.

The branch was pushed normally and [draft PR #21](https://github.com/ChrisLauinger77/stream-gui-rs/pull/21)
was opened against `main`. No merge, release or tag is authorized by this request.

### Fresh adversarial review

The initial review of the complete `main...1e64c42` diff was read-only. Follow-up
pointer-event and built-package probes confirmed two defects. Both were reproduced
before fixing them. No critical/high finding was found. The earlier implementation
fixes listed above remain in the branch.

**MEDIUM — A: confirmed Linux packaging defect, fixed.** The DEB from
run `35879294673` advertised the protocol MIME type but its desktop entry had
`Exec=stream-gui-rs`, without a URI argument field. An isolated GIO launcher probe
received no arguments. Opening a channel/team link through the OS could show the app
but could not deliver the requested destination. `src-tauri/tauri.conf.json` now
selects `src-tauri/packaging/linux.desktop` for DEB/RPM, with `Exec={{exec}} %u`;
AppImage generation reuses the Debian template. This uses Tauri's existing template
hook without another dependency or runtime association writes. The upstream
[desktop-entry generator](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.4/crates/tauri-bundler/src/bundle/linux/freedesktop/mod.rs)
and [default template](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.4/crates/tauri-bundler/src/bundle/linux/freedesktop/main.desktop)
explain why MIME metadata alone was insufficient.

The regression `packaged_protocol_dispatch_passes_the_exact_uri_to_the_launcher`
in `src-tauri/tests/linux_browser_open.rs` extracts the actual built DEB, retains
its launcher argument fields, and replaces only the executable with the existing
native fixture. Isolated XDG associations deliver channel, team and malformed-query
URIs unchanged; the existing Rust parser remains responsible for rejection. It
failed on the downloaded CI package, then passed on the corrected locally built
DEB. All **3 Linux browser/package checks** passed together, as did formatting,
all-target desktop check, strict Clippy, the Tauri custom-protocol DEB build and
`git diff --check`. CI now runs this package regression before uploading Linux
acceptance artifacts. These synthetic checks do not establish real-account native
acceptance or installed AppImage/RPM integration.

Packaging fix commit: `e91531b4cd177632aaa34d7881fd169631a6aa4a`
(`fix(packaging): forward Linux protocol links to the app`). To repeat the focused
package check after building a DEB, set `STREAM_GUI_TEST_DEB` to its absolute path
and run:

```sh
cargo test --locked --manifest-path src-tauri/Cargo.toml --test linux_browser_open --features test-support packaged_protocol_dispatch_passes_the_exact_uri_to_the_launcher -- --ignored
```

**LOW — A: confirmed defect, fixed.** In
`src/features/ShortcutEditor.tsx`, moving focus from the listening Change button to
Cancel capture ran the blur handler first. That removed Cancel before its click
handler could run, leaving focus without its target and the stale "Press a shortcut"
status. Moving focus elsewhere also left that status after capture ended. Reproduce
by starting capture and clicking Cancel, or clicking another focusable control.
The fix explicitly focuses Change when capture begins, keeps that focus during the
Cancel pointer press, and announces ordinary blur termination. Two regressions in
`src/app/Browsing.test.tsx` prove cancellation retains focus/announces completion and
ordinary blur leaves the binding unchanged with an accurate status. Escape, Tab,
conflict and modifier tests also pass. No confirmed defect remains unresolved.

Fix commit: `385969fe730e855ab8610385c95a3fb0e14b1949`
(`fix(ui): preserve shortcut capture cancellation focus`). Post-fix local validation:
**223 frontend tests passed**, including both previously failing regressions;
`npm run build` (TypeScript and production build) and `git diff --check` passed.
Rust, generated DTOs, permissions and dependencies were unchanged by this fix; the
fresh hosted matrix runs the full backend/native/build suite as well.

Review covered all schema 1–6 migrations, particularly preservation of schema 6
profiles, selection, Chatterino, channel overrides, background/notification,
language, low-latency and text-scale preferences. Manual update awareness has no
persisted preference to migrate. It also covered concurrent global/profile/channel/
discovery/shortcut writes, remount and failure handling, list bounds and passive-hide
precedence, Teams authentication/session/cache bounds, twelve-entry history and focus,
the ten shortcut actions, parser rejection cases, startup/latest-intent delivery,
Linux/Windows forwarding, separate macOS activation, permissions and support-report
privacy. Existing auth, supervisor, monitoring, notifications, chat and update
ownership remain intact. The separate CLI remains deferred.

Installed protocol behavior and physical keyboard/screen-reader interaction are
**classification C: native/manual verification gaps**, not inferred defects.
Reproduction requires the corresponding installed package and desktop OS. The
smallest next step is the focused acceptance below; no speculative code change or
new regression is warranted without a reproduced failure.

### Hosted and native acceptance record

Hosted checks for the pushed implementation are tracked in
[Desktop checks 35878094850](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35878094850)
and [CodeQL 35878091677](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35878091677).
All three desktop jobs and all three CodeQL analyses (Actions, JavaScript/TypeScript,
Rust) passed for `1e64c42`; this is pre-fix evidence, not the final acceptance build.
Each desktop job passed frontend build/tests, candidate-promotion guards, Rust
formatting, backend/build/process tests, desktop checks/tests, strict Clippy,
non-publishing Tauri build, registered-client-ID packaging and artifact upload.
Linux browser tests, macOS bundle signature verification and Windows GUI-subsystem
verification also passed. No hosted failure required diagnosis or a CI workaround.

The fix is checked separately by
[Desktop checks 35879294673](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35879294673)
and [CodeQL 35879288525](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35879288525).
All three desktop jobs and CodeQL analyses passed for `385969f`, but package inspection
then found the Linux URI-forwarding defect above. Those artifacts are superseded.
Native acceptance must use a package containing both fixes, rather than an earlier
artifact with the same 0.4.0 version label.

The corrected application commit is `e91531b4cd177632aaa34d7881fd169631a6aa4a`.
[Desktop checks 35881346703](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35881346703)
builds its PR merge commit `68b606e9b971589dc4a0cd719291d945c278f5dd`.
[CodeQL 35881344020](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35881344020)
passed all three analyses and the aggregate gate; the PR-ref open-alert query returned
no findings. All three desktop jobs passed, including every applicable build guard,
format/type/lint check, packaged-mode build and artifact upload. Linux passed the
new actual-DEB dispatch regression; macOS passed bundle signature verification;
Windows passed GUI-subsystem verification. The job steps and test logs were inspected
individually; no failure, retry or workaround was needed for this run.

| Hosted test group | Linux | macOS | Windows |
| --- | --- | --- | --- |
| Frontend | 223 passed | 223 passed | 223 passed |
| Backend unit | 181 passed | 181 passed | 181 passed |
| Native process lifecycle | 42 passed | 36 passed | 36 passed |
| Build-info/client-ID guards | 3 passed | 3 passed | 3 passed |
| Candidate promotion guards | 2 passed | 2 passed | 2 passed |
| Desktop unit, normal / acceptance feature | 10 / 14 passed | 11 / 15 passed | 12 / 16 passed |
| Browser / actual-DEB protocol dispatch | 2 / 1 passed | Not applicable | Not applicable |

The eligible Linux artifact is
[Linux X64 acceptance package](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35881346703/artifacts/10760629215),
named `stream-gui-rs-notification-acceptance-Linux-X64-68b606e9b971589dc4a0cd719291d945c278f5dd`.
Its About identity is **0.4.0 / 68b606e**. Artifact archive SHA-256:
`77e1f167a9615c2f0a6e9a9ab48c6272d9881d3fc73d2289d7853b1bae2c475d`.
The downloaded `Stream GUI RS_0.4.0_amd64.deb` has SHA-256
`b5926326408af0f15e704fc835458ef8a5c8f635289ca8b883614fd9139a4ce5`.
Its extracted desktop entry was independently checked for the scheme MIME type and
`Exec=stream-gui-rs %u`. This package inspection alone did not establish OS
association or real app activation; the user-reported Linux test below did.

The same run produced [macOS ARM64](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35881346703/artifacts/10761054683)
and [Windows X64](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35881346703/artifacts/10761680983)
acceptance artifacts for merge commit `68b606e`. User-reported native results for both
platforms are recorded below.
Subsequent documentation-only follow-ups change no application/build input; current
HEAD check results are available on [PR #21's checks](https://github.com/ChrisLauinger77/stream-gui-rs/pull/21/checks).

After the Linux package handoff, the user reported that all new Phase 8 behavior and
real streaming worked on Linux. In a follow-up, the user explicitly confirmed that
installed protocol links worked while closed, running and hidden; a malformed link
was rejected without autoplay; and keyboard focus, shortcut cancellation, text sizes
and 200% zoom checks passed on Debian Forky under Wayland. This is **user-reported
manual acceptance**, distinct from the earlier synthetic graphical checks and
hosted CI. The desktop environment and observed About commit were not separately
reported. The user subsequently confirmed that the full focused Windows 11 checklist
passed using the PR's NSIS package, with About showing the expected commit. This
includes retained login/settings and Teams, installed links while closed/running/hidden
with malformed-link rejection and no autoplay, real playback with Stop/Restart,
tray/background and Quit cleanup without console flashes, plus keyboard/shortcuts,
text sizes and 200% zoom. The user also confirmed that the full focused macOS
checklist passed using the Phase 8 PR app package on Apple Silicon: retained
login/settings and real Teams, installed links while closed/running/hidden with
malformed-link rejection and no autoplay, real playback with Stop/Restart,
status-item and Command-Q behavior, plus keyboard/shortcuts, text sizes and 200%
zoom. The exact About text was not transcribed on either platform. These are
user-reported manual results; no real Twitch credentials were read or captured by
the agent during this review.

| Evidence | Linux | macOS | Windows |
| --- | --- | --- | --- |
| Installed protocol registration; cold/running/hidden links; malformed links; no autoplay | Passed, user reported | Passed, user reported | Passed, user reported |
| Existing login/settings; real Teams/channel data | Passed as part of "all new" report; details not itemized | Passed, user reported | Passed, user reported |
| Real playback, Stop/Restart, tray/background and Quit cleanup | Real streaming passed; lifecycle details not itemized | Passed, user reported; Command-Q | Passed, user reported; no console flashes |
| Manual keyboard, focus, shortcut cancellation, text scaling and 200% zoom | Passed, user reported; screen reader not separately identified | Passed, user reported; screen reader not separately identified | Passed, user reported; screen reader not separately identified |

### Focused installed-package checklist

Record OS/desktop, artifact/run identity, About commit, and pass/fail/not-run for each
item. Report symptoms without credentials, sign-in codes, private settings or logs.
Use the PR acceptance package; it is a debug test build, not a release. CI macOS
packages use the runner architecture and are not evidence of universal-binary
acceptance.

1. Quit the previous app, install the matching package, and confirm existing login
   and settings are retained. Check profiles, chat selection and one channel override.
2. Open a real team through Search → Teams; open a member and use Back/Forward.
   Bookmark/unbookmark and hide/restore a channel/category. Confirm hidden items stay
   reachable through Bookmarks, Search, Teams and direct navigation.
3. With keyboard only, reach Teams, bookmark/hide controls and Back/Forward. Capture a
   custom shortcut, cancel with Escape and the Cancel capture button, check a conflict,
   save a valid binding, use it, and reset/save defaults. Check focus returns to a
   meaningful control.
4. Confirm the installed OS association for `stream-gui-rs`. Open
   `stream-gui-rs://show`, a channel link, a category link and a team link through the
   OS handler while closed, running and hidden/backgrounded. Check one retained app
   instance, visible destination focus, and no autoplay. Send a malformed link with
   a query or extra path segment, then a valid link; the valid link must still work.
5. Start one real live stream, verify actual audio/video, Stop and Restart. Check
   tray/background restoration and Quit cleanup of owned playback. On Windows check
   for console flashes/duplicate instances; on macOS check Command semantics,
   installed-bundle activation, About and Command-Q.
6. Check new views at 100/125/150% text scale and 200% browser/system zoom. Confirm
   controls remain reachable, focus visible, and status/conflict messages and labels
   understandable; record screen-reader checks as not run if unavailable.

All three native acceptance gates have user-reported passes. The PR may be marked
Ready once hosted checks on the final PR HEAD pass; this does not authorize a merge.
Documentation updates alone do not require repeating the unchanged expensive local
suite; `git diff --check` and claim/path review still apply.

## Commit and scope record

- `ab71ee1` — schema 7, bounded discovery and typed shortcut persistence/migrations.
- `5a51446` — local bookmark/hide actions and management.
- `b40af06` — Teams browsing and Home/Back/Forward history.
- `11e44b1` — shortcut capture, validation, defaults and remapping UI.
- `c29f438` — shortcut pending-save guard across editor remounts.
- `232abbf` — bound development file watching so native build output cannot exhaust Vite watchers.
- `088c866` — typed deep links, native activation, generated permissions and native/adversarial acceptance.
- `186c5a5` — README, architecture, durable AGENTS invariants and this validation report.
- `7ae118a` — existing platform-support badge, preserved during finalization.
- `1e64c42` — the initial local-only decision, later superseded by PR authorization.
- `385969f` — shortcut cancellation focus/status regression fix.
- `e91531b` — Linux package URI argument forwarding and actual-DEB regression.

Versions in package metadata, lockfiles and Tauri configuration remain **0.4.0**.
No release/version-preparation command, release tag, merge or publication was run.
No automatic updater, global hotkeys, localization, new service/transport, download,
remote-control API, arbitrary command/URL interface or other later-phase feature was
introduced.
