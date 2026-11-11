# Test suite audit — 2026-11-10

Audited the working tree based on `95f93d6`, on macOS arm64 with Node 26.13.0 and Cargo 1.99.1. Read `AGENTS.md`, the testing policy, relevant architecture contracts, test discovery/configuration, test sources and their production boundaries. No application behavior, dependencies, generated DTOs, permissions or discovery settings were changed by this audit.

Five files already had test edits before work began: `build_info/commit.rs`, `desktop/mod.rs`, `streamlink/probe.rs`, `streamlink/supervisor.rs` and `twitch/api/tests.rs` under `src-tauri/src/`. Those edits were preserved and included in both baselines. In particular, the existing merge of `short_sha_is_preserved` into `valid_shas_are_shortened_to_seven_lowercase_characters` was already present; it is not a removal made by this audit.

## Comparable execution counts

Counts are runner results, including expanded table cases, rather than counts of source annotations. Feature configurations overlap and must not be added together as unique coverage.

| Suite | Before | After | Result |
| --- | ---: | ---: | --- |
| Vitest, five files | 297 | 297 | All pass |
| Rust, no default features: library | 207 | 207 | All pass with loopback access |
| Rust: build-info integration | 2 | 2 | All pass |
| Rust: client-ID build integration | 1 | 1 | All pass |
| Rust: native process integration on macOS | 42 | 42 | All pass |
| Desktop library tests | 14 | 14 | All pass |
| Desktop with notification acceptance | 18 | 18 | All pass |
| Release-tag recovery scripts | 17 | 17 | All pass |
| Website metadata scripts | 5 | 5 | All pass |
| Translation validator fixtures, through build | 4 | 4 | All pass |

The no-default-features run has 252 distinct passing test identifiers before and after. Comparing those sets shows only the monitor test rename described below. Linux/Windows-gated integration binaries discover zero tests on this host; that is not evidence of passing their platform scenarios.

## Changes and fault-detection evidence

| Classification | Test or group | Improvement and concrete defect detected |
| --- | --- | --- |
| MERGE / IMPROVE | `monitor::state::tests::pause_sleep_and_recovery_suppress_unobserved_transitions` → `rebaseline_suppresses_unobserved_transitions` | Collapse five identical executions to one. Preserve live-count retention, quiet recovery baseline and one subsequent event; additionally assert that the emitted stream/user IDs are exactly `300`/`3`, catching emission of the wrong event. |
| IMPROVE | `service_restart_uses_current_settings_and_survives_a_dropped_caller` in `process_lifecycle.rs` | Replace a 10 ms delay with an actual native probe gate. Confirm the caller is pending and its join reports cancellation before releasing the probe. A restart abandoned with its caller can no longer pass simply because it completed before the abort. Existing settings, immutable snapshot, logout and shutdown assertions remain. The gate lives only in the existing native test executable. |
| IMPROVE | `platform_adapter_rotates_and_deletes_one_entry_without_os_access` | Check both access and refresh tokens after initial save and replacement. Detect stale, swapped or incorrectly persisted access tokens even when the refresh token is correct. Keep empty-store, deletion and repeated deletion checks. |
| IMPROVE | `Live has loading, success and server-error retry states` | Assert the retry request's refresh/session/cursor values, restored result count and viewers, and disappearance of the error/retained-result labels. Detect a retry that dispatches incorrectly, loses results or leaves an error visible after success. |
| IMPROVE | `online channel details include viewers, category and start time` | Assert viewers and category inside the channel stream, plus the exact machine-readable timestamp. The former assertions could pass with those values missing or replaced by a generic “Started” label. |
| IMPROVE | `Enter in the profile editor saves only that profile and leaves global draft edits intact` | Actually create an unsaved global quality edit. Verify the exact profile mutation, editor completion, no implicit global save, and subsequent explicit save retaining both that edit and the accepted profile. Detect draft loss or accidental submission of the surrounding settings form. |
| IMPROVE | Four readability tests in `themeTokens.test.ts`: bundled palettes, dark-only custom, light custom and mixed custom surfaces | Use an independent WCAG contrast oracle instead of the production function that also adjusts colors. Retain all palette/mode/surface/threshold cases and validate generated hex colors. Detect a broken production calculation that otherwise validates its own incorrect output. |
| IMPROVE | Four `website metadata rejects …` cases | Assert the exact single diagnostic for canonical, Open Graph, robots sitemap or sitemap location. An unrelated error no longer satisfies rejection of the intended malformed field. The valid-input control remains. |

Fourteen existing test cases were strengthened, counting the renamed monitor case. No test functions were added or deleted.

### Deletion check for the duplicate monitor executions

1. **Behavior:** `Transitions::rebaseline()` retains the known live count, suppresses events on the next accepted baseline and permits a later new stream event.
2. **Defects:** clearing the count too early, notifying during recovery, suppressing all later notifications, or emitting the wrong stream.
3. **Retained protection:** `rebaseline_suppresses_unobserved_transitions` uses the exact same `Transitions::default()`, stream IDs, user IDs, call order and assertions in the same unit-test layer, with extra event-identity checks. The former `_reason` variable was never read or supplied to production code. Each iteration constructed a fresh identical state; none selected a different configuration or integration path. History (`17043cb`) introduced this as a generic transition-state test, and the current state API has no recovery-reason parameter.
4. **Lost checks:** four repetitions of identical setup and assertions, and misleading labels only. No distinct input, outcome or boundary disappears. This is consolidation of executions inside one test, not removal of four discovered tests.

Cause-specific integration checks remain unchanged, including `owned_monitor_cancels_pause_rebaselines_resume_and_drains_shutdown`, `monitor_backoff_recovery_sleep_and_channel_filtering_use_fresh_baselines`, `sleep_during_inflight_scan_discards_response_and_recovers_quietly`, and `repeated_cursor_partial_failure_and_malformed_identity_never_complete_scan`. The removed labels themselves never proved those paths.

### Temporary fault probe

In an isolated temporary copy, replaced production `contrast()` with a constant `21`. The original five palette tests all passed. With the independent oracle, four readability tests failed and the registry test passed. The real, unchanged production implementation passes all five improved tests. No mutated production file or probe artifact enters the repository. This is one targeted fault experiment, not a mutation score or exhaustive proof.

## Retained suites and distinct value

| Classification | Group | Why retain it |
| --- | --- | --- |
| KEEP | Auth lifecycle, credential serialization/vault and OAuth HTTP tests | Fake provider gates exercise real service coordination; local HTTP separately checks actual forms, headers, typed failures and one-use token handling. Store deletion failure, unverified deletion, caller cancellation and shutdown are different failure boundaries. No real secure-store entries are needed. |
| KEEP | Helix endpoints, browsing, cache, batching and session races | Typed model/null/private-field checks, exact endpoint/query encoding, partial versus missing data, live versus unknown/offline, cache policy and same-account replacement protect different contracts. Similar login/logout cases use different entry points and must not be merged solely for sharing helpers. |
| KEEP | Clock, rate coordinator and monitor unit/integration tests | Pure deadline/transition tests isolate arithmetic and state; HTTP/owned-monitor tests prove cancellation, pagination completeness, foreground priority and recovery wiring. Paused-time and wall-only jumps cover distinct suspend conditions. |
| KEEP | Settings versions 1–11, profiles, discovery and shortcuts | Each old schema is still supported by the current contract. In-memory migration, rejection without overwrite, stale drafts, sparse inheritance and atomic persistence are distinct. The codec-era parser cases added in `6a89d55` are not obsolete merely because older migration tests exist. |
| KEEP | Playback argv, locators, discovery and probe unit tests | Exact argument boundaries, encoding, quality fallbacks and per-service options catch defects before OS execution. A service-specific table calling the shared quality selector does not replace the separate test with literal expected quality arguments. |
| KEEP | All native lifecycle and Chatterino/Flatpak integration scenarios | Actual compiled children prove pipe draining, descendants, reaping, reservation cleanup and independent chat lifetimes. Twitch/Kick/YouTube differ in identity, authentication, latency flags and diagnostic retention. Unit mocks cannot replace these scenarios. |
| KEEP | `App.test.tsx` and `Browsing.test.tsx` | Developer tools and the browsing shell are separate reachable surfaces. Real React hooks/components run behind a mocked IPC boundary; deferred results exercise UI races, drafts, focus, pagination, localization and pending guards. Backend correctness remains separately tested in Rust. |
| KEEP | i18n, translation fixtures and CSS/palette checks | Runtime fallback/plurals, catalog structure/placeholders, default CSS and generated palettes protect separate outputs. Small registry/default assertions still encode active product requirements. No snapshot assertions or snapshot files were found in the test inventory. |
| KEEP | Build-info, client-ID and generated DTO contracts | Pure SHA normalization does not replace execution of the actual build script, Git ref/watch handling, distribution guards or exact Rust/TypeScript agreement. The generated declaration comparison is an intentional boundary contract. |
| KEEP | Release-tag and website scripts | Local/remote tag conflicts, annotation preservation and interrupted publication recovery have different setup and failure paths. Website fields are independent metadata surfaces. These tests use local fixtures, not real publication. |
| KEEP | Desktop permissions, notification state, tray, About and native localization | Local versus remote origins, feature gates, OS callback state, opaque notification IDs and native attributed About metadata are distinct from frontend presentation. The generic Windows notification-state tests run here; Win32 integration does not. |
| KEEP | Linux browser/startup/background and Windows subsystem tests | Platform isolation, packaged launch fields, WebKitGTK geometry and real PE subsystem output cannot be established by macOS or jsdom. In particular, `isolated_browser_dispatch` normally returns early but is also the deliberately selected child-test entry point; deleting it would break real browser-dispatch/reaping coverage. |

No whole-test removal met the required equivalence or retired-requirement standard. No replacement tests, new test files or new frameworks were introduced.

## REVIEW and remaining weaknesses

- **REVIEW — CSS layout contracts:** `single-line controls share sizing…` and `Open channel aligns action buttons independently of feedback height` inspect CSS/computed declarations in jsdom. They cannot establish actual glyph clipping, wrapped geometry or native WebView alignment. Preserve them: no retained automated test was established to replace their exact declaration checks across native platforms. A native geometry observation and an equivalent assertion would be needed before replacing them. Their synthetic markup can also drift from React markup.
- **REVIEW — graphical/platform smoke:** retained Linux startup/background/browser packaging and Windows native tests were source-reviewed but not executed here. Native smoke scripts also use presentation selectors and localized text; harmless UI changes may require test maintenance. Keep them until an equivalent native boundary check is demonstrated, not merely because macOS tests pass.
- **REVIEW — explicit command inventory:** the pre-existing expanded `every_command_requires_the_local_main_window` test was preserved. It checks actual Tauri authority for listed commands, but its manually maintained list can drift from registration/build manifests. This audit did not redesign that user-edited test or change registration/discovery.
- **Remaining assertion weakness:** some retained tests assert counts or an allowed set of errors rather than exact identities/codes, for example monitor scan failure categorization and session-history retention. They still detect meaningful failures; no deletion equivalence was established. They do not prove every failure subtype or which old history entry is evicted.
- No coverage provider or mutation framework is configured in the project; the installed frontend packages lack the Vitest coverage providers/Stryker, and Cargo lists no coverage/mutation command. No dependencies were installed. The targeted contrast experiment supplies limited additional evidence, not comprehensive mutation analysis.

## Validation record and limits

The following commands ran before and after the changes and passed, except for the explicitly explained sandbox attempt:

```sh
npm test
npm run build
node --test scripts/create-release-tag.tests.mjs scripts/check-website.tests.mjs
cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --features test-support
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib --features test-support desktop::
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib --features test-support,notification-acceptance desktop::
```

The first sandboxed backend baseline failed at loopback listener creation: 143 library tests passed and 64 failed with `PermissionDenied` / `Operation not permitted`; integration tests were not reached. Repeating the same command with loopback access passed all 252 tests. Final backend validation used that same access. No baseline product/test failure or new final-suite failure remained. Existing ts-rs serde-attribute warnings and Vite's >500 kB bundle warning appeared before and after.

Additional final checks:

```sh
npm run website:check
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets --features test-support
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets --features test-support -- -D warnings
git diff --check
```

The native restart test also passed in isolation after adding its gate. Build includes translation validation (612 keys in each of four catalogs), four validator fixture tests and TypeScript checking. Final diff review preserved the original assertions and all distinct cases, except the proven identical monitor repetitions.

No real Device Flow, OS credential-store access, real streaming/player/browser interaction, graphical native acceptance, Linux execution, Windows execution or package build/signature/installed-package smoke was performed. The Tauri no-bundle build was not rerun for this test-only change. Automated macOS fixture success does not establish those behaviors. No loss of distinct fault detection was identified; native platform gaps and the remaining assertion weaknesses above remain unverified.

This validation record precedes branch, commit and pull-request preparation. The five pre-existing edited files were preserved throughout the audit. No merge, tag or release publication is part of the audit.
