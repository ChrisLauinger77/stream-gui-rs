# 0.3.0 release readiness

Preparation date: 2026-09-20. This record accompanies the candidate source, not publication approval. No tag or GitHub Release is created during hardening.

## Baseline and scope

Starting branch: clean `main`, synchronized with local and remote `origin/main` at `9087c40511235cd63f44d81fdbc8d68af17c31be`. Released baseline: annotated `v0.2.0`, commit `c0bbb61c3c86c5de1665d7b35b02771302db4e51`. Starting [Desktop checks](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35505409979) and [CodeQL](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35505409973) succeeded on that starting SHA.

The initial hardening commit synchronizes 0.3.0 metadata, strengthens the realistic v4 upgrade regression and prepares documentation/forms. A follow-up fixes the extra outer scrollbar found during candidate acceptance: positioned accessibility descriptions now stay inside the results pane, so the header, navigation and About footer remain in place. Dependency versions are unchanged. See the [release notes](release-notes-0.3.0.md) and [changelog](../CHANGELOG.md) for user-visible scope. No later-phase features or Renovate updates are included.

The replacement candidate SHA is the follow-up commit containing this file. Its full SHA, hosted results, first-attempt Release run, artifact IDs/expiry/checksums and native observations must be recorded together in the run-specific completion record. Do not commit results after building merely to insert a self-referential SHA: that would create a different candidate revision. The checked-in checklist below deliberately claims no future results.

The follow-up also addresses the later macOS About report: credits now say “Github Repository,” using a native URL link value, system link color and underline. The existing AppKit panel and fixed repository destination remain. The macOS runner must pass the strengthened attributed-link regression, and actual link activation remains part of replacement native acceptance.

On 2026-09-21, the maintainer reported all three native platforms passed the first candidate's tests except the extra scrollbar/header/footer defect. That candidate was [run 35508256749](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35508256749), SHA `9c7264e99067867bef60f9b324615bc4748f26ff`. Its Desktop checks, CodeQL, artifact/signature/checksum audit passed. It is superseded by this source fix and must not be promoted. The maintainer also explicitly confirmed a previous Linux test build upgraded directly over 0.2.0 without logout/reset, preserving login/settings and applying Any language, Off/Inherit low latency and 100% text defaults. This is prior-build migration evidence for unchanged migration code, not an exact replacement-package upgrade. No untouched v4 profile remains. Replacement artifacts require focused native scrolling/text-size/keyboard/About checks on all three platforms and matching build identity; earlier native results remain recorded against their original build.

## Upgrade and credentials

The strengthened `version_four_migration_preserves_released_preferences_and_defaults_phase_six` regression opens a representative schema 4 profile with absolute Unicode/space-containing Streamlink/player paths, mpv mode, literal/quoted/braced/empty arguments, audio quality, browser chat, dark theme, enabled monitoring/notifications/background close and a five-minute interval. It verifies enabled and disabled channel quality/chat/notification overrides, removes an all-inherited record, and checks every field again after schema 5 save/reopen. New defaults are Any language, low latency Off, channel low latency Inherit and 100% text. Opening alone does not rewrite the original file. Existing v1/v2/v3 migrations, strict validation, bounds, atomic writes and concurrent global/channel preservation remain covered.

Credential adapters, storage format and client-ID validation/configuration have no diff from v0.2.0. The secure-store service remains `io.github.stream-gui-rs.oauth`, keyed by the public Twitch client ID; the application identifier remains `io.github.stream-gui-rs`. No credential migration or plaintext fallback is introduced. Release builds still require `TWITCH_CLIENT_ID_BUILD` from the project secret, with no synthetic release fallback or client secret. Its value is not included in this record. Code compatibility and synthetic tests do not prove a real stored-login upgrade; that native gate is pending until explicitly recorded.

## Focused security and blocker review

- Support reports still use a separate explicit allowlist: compiled app version/commit, OS/architecture, previously validated numeric Streamlink version, player mode and at most sixteen anonymous phase/typed-failure/exit summaries. Service inspection and regressions cover no credential access, HTTP, probe, process spawn or automatic upload/copy/save. Paths, identities, arguments, settings dumps, arbitrary errors and raw output cannot enter the formatter. Each native candidate still needs a visual privacy check.
- Language remains a server query and cache/cursor identity component; Following is unfiltered. Exact lookup validates a bounded login and binds Get Users to the original session without autoplay. Low latency adds exactly one `--twitch-low-latency` when enabled; shell-free argv boundaries, immutable running configuration and Restart semantics remain intact.
- Settings-save race regressions are retained unchanged. About uses common compiled build metadata, native AppKit on macOS and a bounded single main-window dialog elsewhere. The fixed repository operation grants no arbitrary URL authority.
- Frontend invocation, Rust registration, build manifest, generated permissions and local-main capability agree. No frontend Twitch HTTP or generic HTTP/URL/executable/settings mutation was added. Normal capabilities exclude the development notification trigger; build guards reject its release-profile feature. Development localhost/test controls are gated infrastructure, not release actions. No release-facing TODO/FIXME/HACK was found.
- Reachable-history pattern audit inspected 816 blobs (808 text blobs) at the starting revision. Credential-looking literals were synthetic fixtures in Twitch/Helix tests; personal-path matches were hostile synthetic privacy-test input. No private-key/provider-token pattern, tracked `.env`, credential export or signing material was found. Current tracked changes were inspected separately. This is a focused audit, not a guarantee that pattern matching detects every possible secret. No real secure-store entries were read and no history was rewritten.
- Version sources are npm/Cargo/Tauri, synchronized by the existing script. About, diagnostics and support report derive the same build info; workflows embed the captured SHA, without runtime Git. Bundle/installer metadata derives from these sources.

The initial review found no concrete runtime blocker. **SHOULD FIX BEFORE 0.3.0**, resolved in preparation: stale README upgrade defaults/version framing; bug-form macOS-only About guidance; smoke checklist still targeting 0.1.0 and allowing authentication reset before upgrade evidence; incomplete representative migration assertions. Subsequent native acceptance found the double-scroll defect on all three platforms. Its focused CSS fix includes native WebKitGTK coverage for long stream/channel lists at two widths and all text sizes, including focus-driven scrolling. Exact replacement hosted/artifact/native gates remain required, not assumed passed.

## Initial preparation automated validation

Run on Debian forky/sid x86_64, GNOME Wayland, Node 24.20.0 and Rust 1.95.0. Results are finalized before committing this record.

| Check | Result |
| --- | --- |
| Generated bindings / TypeScript / production frontend build | PASS; generated DTOs unchanged |
| Frontend | 175 PASS across 3 files |
| Backend units | 156 PASS |
| Native process lifecycle integration | 30 PASS |
| Build identity / client-ID build guards | 2 + 1 PASS |
| Candidate-promotion guard | 1 Node test file PASS |
| Rust format / all-target check / strict Clippy | PASS |
| Desktop normal / opt-in notification acceptance | 10 + 14 PASS |
| Isolated Linux browser dispatch | 2 PASS |
| Graphical Linux background / startup | 3 + 1 PASS (isolated native WebKitGTK fixtures) |
| Native packaged-debug/no-bundle build | PASS with synthetic compile-only public ID |
| Issue forms / command permissions / version consistency / diff whitespace | PASS; 41 closed commands aligned |

Loopback HTTP and native fixtures use synthetic credentials; graphical fixtures use isolated settings/D-Bus/notification service/fake playback. They do not authenticate to real Twitch or accept a release package. Hosted Desktop checks and CodeQL must subsequently pass on the candidate SHA on Linux, Windows and macOS as configured.

The complete local suite was rerun after the scrolling correction on 2026-09-21, with all the same counts passing. The graphical Phase 6 test additionally covers twelve real WebKitGTK layout cases; its retained regression fails against the original CSS and passes with the correction. See the [follow-up validation record](phase-6-validation.md#candidate-scrolling-correction--2026-09-21).

## Candidate artifacts and promotion

Use the existing [release process](releasing.md): dispatch Release on the green candidate `main` commit, never rerun that candidate. Three immutable archives are retained for 30 days and protected from one-day cleanup. Publication and downstream updates must be skipped for dispatch. Download all archives, verify run identity with `scripts/verify-release-candidate.mjs`, and verify all platform SHA-256 manifests.

| Archive | Expected contents |
| --- | --- |
| `Stream-GUI-RS_0.3.0_linux_x86_64` | `Stream-GUI-RS_0.3.0_linux_x86_64.AppImage`, `Stream-GUI-RS_0.3.0_linux_amd64.deb`, `Stream-GUI-RS_0.3.0_linux_x86_64.rpm`, `SHA256SUMS_linux_x86_64.txt` |
| `Stream-GUI-RS_0.3.0_windows_x86_64` | `Stream-GUI-RS_0.3.0_windows_x86_64-setup.exe`, `Stream-GUI-RS_0.3.0_windows_x86_64.zip`, `stream-gui-rs.json`, `SHA256SUMS_windows_x86_64.txt` |
| `Stream-GUI-RS_0.3.0_macos_universal` | `Stream-GUI-RS_0.3.0_macos_universal.dmg`, `SHA256SUMS_macos_universal.txt` |

Expected signing policy: Linux packages unsigned; Windows unsigned (SmartScreen possible); macOS complete ad-hoc signature, no Developer ID/notarization (Gatekeeper warning possible). Verify actual candidate results, Windows GUI subsystem/installer metadata, Linux dependencies/desktop/icon/version, macOS identifier/minimum 11.0/both executable slices/signature, all package contents and build identity. Scan for accidental source/dependencies/test fixtures/credentials/signing material/logs/private paths. A successful compile alone does not prove signing or native behavior. Checksums belong in the run-specific audit, never manually invented here.

After explicit approval only, the annotated `v0.3.0` tag must point to the accepted SHA and contain `Candidate-Run: <accepted-run-id>` after a blank line. Tag-triggered promotion validates and publishes those retained bytes and committed release notes without rebuilding. This hardening task stops before that step.

## Native acceptance and remaining gates

The maintainer reported historical Phase 6 native testing and real Twitch playback on Linux, Windows and macOS; see [the historical record](phase-6-validation.md#maintainer-native-report-before-030-hardening). It does not accept a new candidate. Follow the single [focused smoke checklist](release-smoke-test.md), recording actual environment, package/hash, identity, low-latency behavior, report privacy, About, keyboard/text scaling, essential background/playback lifecycle and relaunch persistence. At least one real 0.2.0 profile with stored login must be upgraded before logout/reset.

| Exact-candidate gate at preparation | Linux | Windows | macOS |
| --- | --- | --- | --- |
| Native package smoke / About-build consistency | NOT TESTED | NOT TESTED | NOT TESTED |
| Real stored-login/settings upgrade | NOT TESTED | NOT TESTED | NOT TESTED |
| Low latency Off/On/Restart and cleanup | NOT TESTED | NOT TESTED | NOT TESTED |
| Visual support-report privacy | NOT TESTED | NOT TESTED | NOT TESTED |
| Text scaling/keyboard/focus | NOT TESTED | NOT TESTED | NOT TESTED |
| Orca/Narrator/VoiceOver | NOT TESTED | NOT TESTED | NOT TESTED |

Do not mark READY TO TAG until exact hosted CI, artifact checks, native smoke on all three OSes and at least one real upgrade pass. Optional exhaustive screen-reader coverage, other Linux desktops/formats and untested native CPU coverage are explicit gaps, not invented failures.

## Deferred work and limitations

Dependency graph frozen. Renovate Dashboard #2 has a pending Tauri update; native GTK/GIO, Windows API and keyring migrations remain deferred under existing configuration. No open repository PR was present at baseline. The initial [WinGet submission](https://github.com/microsoft/winget-pkgs/pull/437560) is pending and there is no integrated WinGet release automation; it does not block 0.3.0. Existing Scoop manifest/hash/autoupdate and post-publication Scoop/Homebrew notifications remain unchanged.

Current user-visible limits are documented in the release notes: external Streamlink/player, desktop-dependent tray/notification integration, no monitoring after Quit, detached-player cleanup boundary, unsigned/non-notarized distribution, no measured latency promise, and no formal accessibility certification. Later-phase features and new distribution systems remain out of scope.
