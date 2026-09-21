# Phase 7 implementation and validation

## Baseline and scope

Work started from clean `main` at `40652131014287c752e371a30a161b1faf8ee005`
(`chore(deps): update tauri (#19)`). Released v0.3.0 is
`22e773775373ff8d55f25263433e3a2d43a8ccfd`. The intervening Tauri maintenance
update was already complete and is not part of this feature change.

Implementation commit: `a2f5dc2` (`feat(prefs): add profiles, Chatterino and release awareness`).
The following documentation commit records contracts, validation and native gaps.

This phase implements manual release awareness, independent Chatterino chat and
reusable player profiles. Application/package versions remain **0.3.0**. No tag,
release preparation or publication is part of this work.

The implementation plan followed the existing ownership boundaries: one schema
migration; a small Rust release-check service; typed Chatterino dispatch behind
the existing chat service; profile resolution in SettingsStore before launch;
compact settings UI using the accepted-settings coordinator; then hostile tests,
native Linux fixtures and a read-only review. No second playback owner, Twitch
HTTP client, settings authority or frontend preference store was introduced.

## Accepted contracts

### Settings and player profiles

- Schema **6** migrates schemas **1–5** in memory, preserving every released
  v0.3.0 preference and sparse channel override. It keeps the 256-KiB file limit,
  1,000-channel limit, atomic replacement and malformed/unknown-schema rejection
  without overwriting the source file.
- Migration defaults are Browser chat, no Chatterino path, an empty profile list
  and no selected profile. Existing global player settings remain the Default
  configuration. Manual update checks need no persistent preference.
- At most 16 profiles use Rust-assigned stable UUIDs, unique trimmed names of
  1–64 Unicode characters / at most 128 UTF-8 bytes, existing bounded literal
  player settings, nullable quality and nullable low latency. Names never enter
  command construction; no Streamlink argument editor was added.
- Precedence is **global defaults → selected profile → channel overrides →
  explicit launch quality → immutable session snapshot**. A profile replaces
  the player configuration; absent profile quality/latency inherit global values.
  Explicit channel `false` still overrides profile/global `true`.
- Profiles are globally selectable in Settings. There are no channel profile
  references or per-card profile selectors. Delete atomically clears an active
  selection; malformed/dangling stored selections are rejected. Existing runs
  retain their accepted snapshot; Restart resolves current preferences.
- Profile mutations reuse the global frontend coordinator and Rust operation
  lock. Global saves preserve profile records/selection at the store's atomic
  write boundary. Player fields retain partial draft overlays. Create/edit/select
  validate the player path; launch/restart validate again. The management picker
  lets an unavailable, unselected profile be repaired/deleted without activating it.
- The support report exposes only the selected effective player-mode category,
  never profile names/IDs, paths or arguments.

### Release awareness

- Settings → Updates performs manual checks only. Startup, navigation, About,
  monitoring and tray do not start network checks.
- Rust owns the fixed public endpoint:
  `https://api.github.com/repos/ChrisLauinger77/stream-gui-rs/releases/latest`.
  A separate unauthenticated reqwest client disables redirects and retries,
  imposes a ten-second deadline and bounds both declared and streamed bodies to
  256 KiB. It never receives Twitch credentials or uses Twitch HTTP.
- Structured semantic version precedence distinguishes newer/current/development
  builds. Draft/prerelease responses cannot offer an update. Stable remote tags
  require `vMAJOR.MINOR.PATCH`, no build metadata and the exact official release
  URL for that tag. Remote title/body/HTML are ignored.
- One service-owned request updates a Rust cache even if its IPC caller leaves.
  Results and failures are cached for 24 hours; manual refresh has a 60-second
  minimum interval. Shutdown cancels pending HTTP. No recurring timers or retries.
- The UI receives only a closed status and accepted version. Parameterless
  View release reconstructs the official URL in Rust and uses the existing native
  browser adapter. No generic network/open-URL API or installer download/execution.

GitHub's [latest-release endpoint contract](https://docs.github.com/en/rest/releases/releases#get-the-latest-release)
was checked against the implementation. `semver` is the only new direct
dependency; it was already resolved in Cargo.lock, so no new package/version was
introduced. It avoids maintaining a home-grown semantic-version parser.

### Chatterino

- Browser remains the default. Provider and optional absolute executable path
  are global; sparse channel automatic-chat preferences remain independent.
  Chatterino failures are explicit and nonfatal to playback. Channel details
  retain an explicit Open chat in browser action; there is no silent fallback.
- Discovery reuses existing executable validation and searches up to 128 absolute
  PATH entries plus native locations: Linux package paths, macOS Homebrew and
  system/user Applications bundles, and Windows Program Files/Local App Data.
  There is no recursive scan, downloader, wrapper command or custom chat argv.
- A validated fresh/session-bound Twitch login yields exactly
  `--channels`, `t:<lowercase login>`. Automatic chat uses the immutable launch
  snapshot and generation-checked error reporting. Display metadata is never argv.
- Chatterino authenticates independently. No app access/refresh/device tokens,
  headers, credential files or custom environment are transferred. Child stdio
  is null; the inherited environment is cleared and only an explicit list of
  OS/desktop/session essentials is restored.
- Unix starts a separate session and uses direct `execve`, preventing the implicit
  ENOEXEC shell fallback for malformed executables. Windows uses a validated
  native `.exe` and `CREATE_NO_WINDOW`, outside playback's Job Object.
- At most 16 direct children have independent waiter threads. A successful spawn
  returns promptly; each waiter reaps its child and releases capacity. Stop and
  Quit do not kill Chatterino. Shutdown drains chat dispatch, not the independent
  application's lifetime; surviving children are left to the OS on parent exit.

Upstream stable v2.5.5 sources were inspected:
[typed channels and channel-only settings behavior](https://github.com/Chatterino/chatterino2/blob/v2.5.5/src/common/Args.cpp),
[startup](https://github.com/Chatterino/chatterino2/blob/v2.5.5/src/RunGui.cpp),
[application](https://github.com/Chatterino/chatterino2/blob/v2.5.5/src/Application.cpp),
[window creation](https://github.com/Chatterino/chatterino2/blob/v2.5.5/src/singletons/WindowManager.cpp),
[Windows installation](https://github.com/Chatterino/chatterino2/blob/v2.5.5/.CI/chatterino-installer.iss)
and [macOS bundle naming](https://github.com/Chatterino/chatterino2/blob/v2.5.5/src/CMakeLists.txt).
Channel-only mode disables settings saving: configure/sign in using Chatterino's
normal launcher first. A new window/process may remain. **Instance reuse has not
been natively observed on any platform and is not promised.** Flatpak/Snap command
wrappers are not integrated; native executables/AppImages may use the override.

## Automated validation

All standard tests use synthetic credentials, local HTTP and native fixtures;
they do not require real Twitch, GitHub or Chatterino.

| Check | Local Linux result |
| --- | --- |
| Generated bindings (`npm run bindings`) | PASS; generated DTOs committed with Rust changes |
| TypeScript and production frontend (`npm run build`) | PASS |
| Frontend (`npm test`) | **191 passed**, 3 files |
| Candidate promotion guards | PASS |
| Rust formatting | PASS |
| Backend unit tests, no desktop | **169 passed** |
| Native process lifecycle integration | **36 passed** |
| Build metadata / public client-ID guards | **2 / 1 passed** |
| Desktop-enabled library with notification acceptance | **183 passed** (includes backend tests) |
| Desktop-only tests without acceptance feature | **10 passed** |
| All-target Rust check / strict Clippy | PASS |
| Native debug desktop build, no bundle | PASS; synthetic compile-only public ID |
| Linux browser dispatch/reaping | **2 passed** |
| Linux startup | **1 passed** |
| Linux graphical background/titlebar/Phase 6/Phase 7 | **4 passed** |
| Final Phase 7 graphical layout scenario | **1 passed**, after long-name/overflow checks |
| Whitespace/diff review | PASS |
| Hosted Linux/macOS/Windows CI | **NOT RUN** for this change yet |

Commands run from the repository root:

```sh
npm run bindings
npm run build
npm test
node --test scripts/verify-release-candidate.tests.mjs
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --features test-support
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib --features test-support desktop::
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib --features test-support,notification-acceptance
cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets --features test-support
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets --features test-support -- -D warnings
TWITCH_CLIENT_ID_BUILD=ciCompileOnlyPublicClient123 npm run tauri build -- --debug --no-bundle --features custom-protocol --ci
cargo test --locked --manifest-path src-tauri/Cargo.toml --test linux_browser_open --features test-support
cargo test --locked --manifest-path src-tauri/Cargo.toml --test linux_startup --features test-support -- --ignored
# In a graphical session, with npm run dev -- --strictPort serving the frontend:
cargo test --locked --manifest-path src-tauri/Cargo.toml --test linux_background --features test-support -- --ignored
git diff --check
```

`ts-rs` emits an unsupported-serde-attribute diagnostic for
the strict tagged mutation enum. Generated types are correct; hostile deserialization
tests prove Rust still rejects unknown fields. Rust checking and strict Clippy pass.

New regressions cover realistic v5 migration without pre-save rewrite; profile
CRUD/name/count/ID/dangling-reference/argument validation; precedence and inherited
booleans; immutable concurrent runs and Restart; stale global/profile/language/
channel ordering; profile Enter/focus and plain-text names; update response/URL/
semver/body/deadline/cache/cancellation/header bounds; manual and automatic chat;
hostile logins, Unicode/spaced paths, missing/spawn failures, child environment,
reaping, capacity recovery, independent lifetime, and support-report privacy.

## Native observations and remaining acceptance

Local host: **Debian forky/sid, GNOME, Wayland, WebKitGTK**.

- The isolated graphical harness passed profile create/select/delete with active
  deletion falling back to Default, initial editor focus, Chatterino controls and
  Updates rendering without HTTP on navigation. It paired 100% text with System,
  125% with Light and 150% with Dark, checking 1×/2× zoom and no document/settings
  pane horizontal overflow. This is not a full scale/theme Cartesian matrix.
- Existing Phase 6 About/support-report/text-scale and scrolling fixtures passed.
  Existing background close/restore/reload/Quit, titlebar and synthetic notification
  fixtures passed, including owned playback cleanup. Real notification service,
  user credentials and user settings were not used.
- One final targeted run emitted an unattributed allocator diagnostic after its
  successful test summary. Both the application scenario and test runner exited
  successfully; the preceding full four-scenario run passed without that message.
  Isolated desktop portal/accessibility teardown also emits host warnings. The
  allocator diagnostic's originating process was not established; it is not
  evidence of a verified application crash or a verified fix.
- A separate, temporary manual backend invocation contacted the real fixed GitHub
  endpoint: current `0.3.0` returned **Current**, latest `0.3.0`, and the reconstructed
  destination matched the official v0.3.0 release. The temporary network example
  was removed. This does **not** prove native View release browser activation.

| Acceptance area | Linux | macOS | Windows |
| --- | --- | --- | --- |
| Existing real login/settings restore | NOT TESTED for Phase 7; synthetic migrations pass | NOT TESTED | NOT TESTED |
| Real Twitch browsing/video/audio and mpv/VLC/custom profiles | NOT TESTED; native fake-process contracts pass | NOT TESTED | NOT TESTED |
| Profile switch/edit/delete with real player, two sessions and Restart | NOT TESTED; deterministic native executable fixtures pass | NOT TESTED | NOT TESTED |
| Official live update response | PASS, backend manual check | NOT TESTED | NOT TESTED |
| View release opens official page | NOT TESTED; fixed destination and browser adapter covered separately | NOT TESTED | NOT TESTED |
| Actual Chatterino discovery/manual/automatic chat | NOT TESTED: Chatterino not installed | NOT TESTED | NOT TESTED |
| Actual Chatterino reuse/new process, repeated channels, Stop/Quit | NOT TESTED; independent native fixture lifetime/reaping pass | NOT TESTED | NOT TESTED |
| Native UI/scaling/About/background/Quit | Isolated WebKitGTK fixtures PASS as described above | NOT TESTED on WKWebView | NOT TESTED on WebView2 |
| Screen reader / OS high-DPI / packaged console behavior | Screen reader/OS scaling NOT TESTED; webview zoom checked | VoiceOver NOT TESTED | Narrator/high-DPI/console NOT TESTED |

Remaining manual acceptance must use the Phase 7 build: preserve an existing
login/profile, browse and play real video/audio, exercise representative mpv/VLC
profiles and Restart, open the release page, and verify background notifications,
About and Quit. Where installed, test Chatterino from both stopped and running
states, repeated channels and explicit paths; record actual process behavior
separately on each OS. macOS also needs app-bundle/GUI-PATH discovery and Command-Q;
Windows needs `.exe` discovery, spaces/Unicode, no console flashes and Job Object
cleanup. Previous v0.3.0 acceptance is historical evidence, not a Phase 7 pass.

## Adversarial review and boundaries

The complete implementation diff was reviewed read-only after the fixes and tests.
Review covered settings ordering/references, precedence, immutable snapshots,
process ownership/reaping, child credentials, fixed update destinations/bounds,
shutdown, support-report allowlisting, IPC permissions, About and accessibility.
No remaining confirmed application correctness/security finding was identified.
Hosted CI and the native gaps above remain open validation gates.

Confirmed issues addressed during implementation include the Unix ENOEXEC shell
fallback, stale global saves overwriting narrow profile mutations, inability to
repair an unavailable unselected profile, profile Enter/focus behavior and a late
initial update-status snapshot. Focused regressions cover each. A graphical test
initially acted before its deliberate reload finished; it now waits for the fresh
document rather than adding a timing sleep.

README and architecture describe user behavior and ownership; AGENTS records the
durable schema/IPC/process invariants. No automatic updater, Chatty/arbitrary chat,
token forwarding, alternate transports, codec editor, CLI/deep links, localization,
notification Watch action, history/bookmarks, second service, embedded player/chat,
generic import/export or later-phase work was added.
