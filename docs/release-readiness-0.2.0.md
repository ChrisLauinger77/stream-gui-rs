# Version 0.2.0 release hardening

Preparation date: 2026-09-19. Starting point: clean `main` at `5c8b522e1348bc8513fee7828815897f1c4e5d66`, synchronized with `origin/main`. The local annotated `v0.1.0` tag resolves to `e75338e14cb5df51021cbd4f6c0ea70d3d3129bf`. Current Git/release metadata takes precedence over older phase documents quoting earlier development revisions.

This is release preparation, with no new runtime features, dependency upgrades, tag or publication. Candidate identity and completed native results must be recorded with the workflow run rather than changing the tested source revision.

## Focused review

| Classification | Finding | Resolution / gate |
| --- | --- | --- |
| BLOCKER | The existing workflow required a publishing tag to build packages; it could not supply exact packages for pre-release acceptance. | Add manual candidate builds to the same workflow. Tag promotion selects a successful first-attempt candidate from the same commit with an annotated `Candidate-Run` trailer; no rebuild. |
| SHOULD FIX BEFORE 0.2.0 | Weekly cleanup could remove candidate artifacts after one day despite their declared retention. | Preserve release package artifacts for their existing 30-day retention. Expiry requires rebuilding and retesting. |
| SHOULD FIX BEFORE 0.2.0 | The v3 migration regression used empty paths/player arguments and did not reopen saved v4 preferences. | Extend the regression with absolute paths containing spaces, literal/empty player arguments, multiple channel overrides, restart persistence and incomplete-v3 rejection. Runtime migration code is unchanged. |
| BLOCKER until verified | Current exact candidate packages need successful builds, content/checksum audits, native Windows/Linux/macOS smoke acceptance and at least one real upgrade with stored login. | Use the run-specific results and [checklist](release-smoke-test.md). Historical development tests do not clear this gate. |

No additional concrete application release blocker was confirmed in this focused source review. This is not proof of native acceptance.

## Compatibility and ownership

- Application identifier remains `io.github.stream-gui-rs`. Credential adapter files are unchanged from v0.1.0: service `io.github.stream-gui-rs.oauth`, account key derived from the public client ID, same native storage backends and serialized vault. No token migration or insecure fallback is introduced. Each platform's actual stored-login upgrade remains a native check.
- The existing build guard still requires the registered public `TWITCH_CLIENT_ID_BUILD` for distribution builds. Candidate packages use the same repository secret, with no synthetic fallback and no value printed. Local compilation uses only the documented synthetic public ID.
- Settings v3 migrate in memory to v4 without rewriting on startup. The next accepted save atomically persists v4. Paths, player mode/arguments, quality, chat, theme and channel overrides survive; monitoring, notifications and background close default off. Malformed/unknown/oversized files fail without overwrite; failed persistence leaves memory unchanged. A saved v4 file is not readable by v0.1.0.
- About and diagnostics retain one build-commit source. CI supplies the exact checkout SHA; the existing build script embeds its first seven characters. Version remains semantic `0.2.0`. Local Git/source-archive fallback is unchanged and no runtime Git dependency exists.
- Notification clicks use retained, internal targets and the original session cancellation token. They restore/navigation-select rather than launching playback or accepting arbitrary URLs. Windows banner timeout retains bounded, one-use activation; dismissal, expiry, cancellation and Quit remove native handlers/history. Windows retention includes suspend.
- The monitor still uses shared Helix/cache/rate ownership, fresh quiet baselines, completion-time suspend reconciliation, bounded backoff and session-isolated deduplication retained across transient auth failures. Partial/failed scans cannot establish offline status. Background requests yield to foreground capacity/budget.
- Close-to-background remains opt-in; absent tray support minimizes, and losing the tray host restores a hidden window. Explicit Quit stops monitor/auth/chat work and owned playback, then shuts down notification resources. Platform process ownership and Windows GUI subsystem behavior are unchanged from v0.1.0.

## Bounds and development audit

Monitor scans are limited to 100 pages of 30 records and 45 seconds, history to 16,000 stream identities with quiet reset at capacity, and delivery to ten eligible transitions per scan. The native command queue and platform target registries each hold at most 32 entries; expiry, cancellation and shutdown bound retained callbacks. Windows history has a 15-minute deadline including sleep. Linux/macOS retain their existing monotonic 15-minute cleanup policy. One native worker owns the platform adapter; tray refresh uses one loop. Existing cache, settings, navigation and playback/log bounds remain unchanged.

The test-notification command is a fixed synthetic intent, behind the opt-in `notification-acceptance` feature. The real build-script test rejects that feature in release profiles. Normal package commands omit it and `test-support`; the local-main capability/command is absent. The test target bypasses neither real authentication nor monitor state and never uses Twitch HTTP. Production-artifact notification acceptance therefore needs a real live transition; a debug package is supporting evidence only.

TODO/FIXME/HACK and development-string review found no unfinished release path. Image/input placeholders are intentional UI states; loopback endpoints belong to test fixtures or Vite development configuration. Production `expect` sites reviewed here protect internal mutex/state invariants; provider/user failures use categorized errors. Test fixtures contain synthetic values and are feature/test gated. Developer diagnostics remain intentional, with the notification controls unavailable in normal releases.

A path-only pattern audit examined 639 reachable historical text blobs for private-key headers, concrete secret assignments, provider-token forms and personal home paths; it found no matches or credential/log/signing-file names. This is a bounded pattern audit, not a guarantee against every possible secret encoding. Credential/provider error redaction, narrow IPC, CSP, executable-plus-argv construction and ignored build/dependency/log directories remain intact. No real credential store was read or copied.

## Distribution and remaining work

The [artifact contract](releasing.md#artifact-contract) remains Windows x86_64 NSIS + portable ZIP/Scoop manifest; Linux x86_64 AppImage, Debian and RPM; macOS universal DMG. All platforms retain SHA-256 files. macOS bundles are explicitly ad-hoc signed and strictly verified, without Developer ID signing or notarization. Windows remains unsigned. Candidate builds use `npm ci`, locked Cargo resolution and a lockfile-diff guard; dependency resolutions and Renovate deferrals are frozen at the starting commit.

Prior native acceptance was reported by the maintainer on Windows, Linux and macOS. The named Linux environment is Debian Forky on Wayland; the desktop environment was not specified. The previously repaired macOS 27.0 Apple Silicon bundle produced a permission prompt and test notification after complete ad-hoc signing. These observations concern earlier development packages, not this candidate. RPM/other Linux desktops and native Intel execution must be recorded honestly when unavailable.

Draft user-facing notes are in [release-notes-0.2.0.md](release-notes-0.2.0.md), with the matching changelog and README. Remaining product limits include desktop-dependent Linux notification/tray behavior, installed-identity Windows notifications, no monitoring after Quit, unsigned/unnotarized distribution, external Streamlink/player requirements and no embedded player/chat or updater.

Post-release only: deliberate GTK/GIO alignment, Windows API crate and keyring migrations covered by existing Renovate deferrals; trusted platform signing; deferred advanced functionality. None is implemented as incidental release work.

## Validation record

Local validation on Debian Forky/Wayland, Rust 1.95.0 and Node 24.20.0 passed:

| Check | Result |
| --- | --- |
| TypeScript and production frontend build | Passed |
| Frontend behavior/CSS tests | 102 passed |
| Backend unit/HTTP/DTO contracts | 142 passed |
| Build metadata / real build-script guards | 2 + 1 passed |
| Native fake-process lifecycle | 29 passed |
| Desktop-enabled library suite with acceptance feature | 155 passed (includes the backend library contracts) |
| Isolated Linux browser dispatch | 2 passed |
| Graphical background/close/Quit and Wayland title bar | 2 passed, with fake playback and a private notification server |
| Malformed-settings graphical startup | 1 passed |
| Candidate promotion guard cases | 2 passed; isolated annotated-trailer extraction also passed |
| Generated bindings | Regenerated with no diff |
| Formatting, all-target check and strict Clippy | Passed |
| Native Tauri debug/custom-protocol build | Passed with the synthetic compile-only public ID and locked Cargo resolution |
| Workflow syntax/expressions | actionlint passed; only its outdated runner-label catalogue was excluded for the existing Ubuntu 26.04/macOS 26 labels |
| Version synchronization, dependency freeze, diff whitespace | Passed |

These fixtures did not use Twitch HTTP, real credentials or real media playback. Expected private-D-Bus portal warnings did not fail native assertions. Windows/macOS runtime checks were not executed on this Linux host.

The exact candidate run/commit and hosted build results are recorded with the task result after they complete. The working tree and candidate must not be changed merely to insert their own SHA into this document. Native platform rows start **NOT TESTED** and can become **PASS** only with observations of the exact candidate bytes. No commit is approved for tagging until those gates pass.
