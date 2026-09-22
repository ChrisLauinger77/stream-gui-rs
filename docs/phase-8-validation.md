# Phase 8 validation — advanced discovery and desktop integration

Implementation date: 2026-09-22. Baseline: completed Phase 7 / v0.4.0,
`0c5397ca6cc1b2605d176f81f0f5f552d27c6069`, with a clean `main` matching
`origin/main` before work. Implementation branch: `codex/phase-8-discovery`.

The implementation and Linux checks described below are complete. Phase 8's
cross-platform completion gate remains open until hosted checks and the outstanding
installed-package/native acceptance checks are recorded. No version bump, release
preparation, tag, publication or later-phase work is part of this change.

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
| Hosted Linux/macOS/Windows Desktop checks and CodeQL | Not run; the user requested that this branch remain local |

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

Outstanding evidence required before declaring cross-platform Phase 8 complete:

- Linux installed package protocol association and actual OS URI cold launch; the
  current native fixture covers startup inbox delivery and actual process forwarding,
  not installer registration. Real Twitch Teams and real Streamlink/mpv/VLC audiovisual
  playback, keyboard-only/screen-reader acceptance remain unobserved in this run.
- macOS native WKWebView, Command semantics, installed universal app protocol
  cold/running/hidden activation, About/status item and Command-Q cleanup. No macOS
  runtime is available on this host.
- Windows WebView2, installed protocol cold/running/hidden activation, keyboard,
  no-console behavior, tray/background and Quit cleanup. No Windows runtime is
  available on this host.
- Hosted checks on the implementation commit. Baseline hosted success is not evidence
  for this branch. The user explicitly chose to keep this branch local after the
  implementation was committed and reviewed; no push or pull request was made.

## Commit and scope record

- `ab71ee1` — schema 7, bounded discovery and typed shortcut persistence/migrations.
- `5a51446` — local bookmark/hide actions and management.
- `b40af06` — Teams browsing and Home/Back/Forward history.
- `11e44b1` — shortcut capture, validation, defaults and remapping UI.
- `c29f438` — shortcut pending-save guard across editor remounts.
- `232abbf` — bound development file watching so native build output cannot exhaust Vite watchers.
- `088c866` — typed deep links, native activation, generated permissions and native/adversarial acceptance.
- `186c5a5` — README, architecture, durable AGENTS invariants and this validation report.
- A final documentation-only follow-up records the decision to keep the branch local.

Versions in package metadata, lockfiles and Tauri configuration remain **0.4.0**.
No release/version-preparation command, release tag, merge or publication was run.
No automatic updater, global hotkeys, localization, new service/transport, download,
remote-control API, arbitrary command/URL interface or other later-phase feature was
introduced.
