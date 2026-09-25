# Testing and native acceptance

Test the behavior that changes, record the exact revision and platform observed, and keep automated results distinct from real native observations. The current local commands and CI matrix are in [AGENTS.md](../AGENTS.md#tests-and-validation). Historical release notes describe past versions; they do not prove a new candidate.

## Feature and development branches

Develop application changes on a feature branch. Run affected unit, frontend, backend/process and desktop tests, then the appropriate full checks in AGENTS.md. Hosted Desktop checks cover Linux, macOS and Windows; they prove build and automated contracts, not real playback, secure-store or OS integration behavior. Review risky surfaces such as credentials, session isolation, process ownership, native activation and untrusted inputs adversarially.

Before a branch is mergeable, complete focused native acceptance on the platforms affected by its changed behavior. For example, auth changes need real sign-in, restore and deletion/rotation checks; playback changes need real video or audio plus Stop/Restart/cleanup; native integration changes need installed-package activation on relevant operating systems. Record PASS, FAIL or NOT TESTED, package/revision, environment and limitations. Fix confirmed defects before merge and repeat affected checks on the corrected build. Do not repeat unrelated historical feature tests.

## Interface automation and localization

Vitest with React/jsdom already exercises full mocked browsing, navigation, settings, profiles, shortcuts, deep-link delivery, focus restoration and error flows. Language tests cover supported and unsupported system locales, explicit overrides, immediate switching, persistence on a remount, and a dialog that remains open during a save. The catalog check runs in `npm run build` and fails for missing, extra or unused keys, placeholder mismatches, malformed resources and HTML.

Playwright was evaluated for browser-layer flows. It would need a second mocked Tauri IPC harness and a browser runtime that differs from Linux WebKitGTK, macOS WKWebView and Windows WebView2. Most proposed flows already have full React integration coverage in the existing harness. For this milestone, keep one UI test stack and add focused regressions there. Reconsider Playwright when a concrete browser interaction cannot be tested reliably with jsdom; it would still not certify installed protocol handling, tray, notification, player, signing or package behavior.

For localization acceptance, inspect German first across Settings, playback, profiles, updates, Chatterino, Teams, bookmarks/hides, shortcuts, links and About. Review Spanish and French terminology and layout. At 100%, 125% and 150% app text sizes and 200% desktop/webview scale, check keyboard navigation, focus, accessible names, live status announcements and dialog sizing. On each supported OS, verify System locale selection, explicit switching and relaunch persistence, native tray and notification text, and platform shortcut modifier names. Keep the normal Twitch/playback sanity check small and record which locale and installed package were observed.

Scope OS-specific checks to the change: on Windows, installed identity, native notification activation and console-free launch when affected; on Linux, Secret Service/D-Bus, desktop launcher, protocol delivery and tray fallback when affected; on macOS, Keychain, notification permission, bundle activation, status item and Command-Q when affected. Screen-reader and high-DPI observations remain separate from automated DOM/CSS checks. A debug notification package can prove its own native path but cannot certify the release package.

For a pull request, CI runs for its current head (GitHub may check out a merge commit); confirm the final branch head and merge result match the reviewed behavior. Native acceptance is scoped to changed native behavior. The merged commit is the accepted implementation baseline once the required branch checks and review pass. A merge or release-prep version bump alone does not require full feature re-acceptance. Subsequent runtime changes reopen the affected acceptance gate.

The [notification acceptance procedure](notification-acceptance.md) remains a specialized development guide. Use its synthetic debug packages for delivery/activation mechanics and real followed-stream transitions when monitor or account-lifecycle behavior changes. It is not a standing requirement for every release.

## Release preparation

A normal release-preparation commit changes version fields, corresponding lockfile/version metadata, release notes, the changelog, release-facing documentation and required generated release metadata only. It does not carry runtime features, unrelated bug fixes, dependency upgrades, native integration or packaging architecture changes. Review and test such work as a real code change before it becomes a release candidate.

Earlier native acceptance remains valid when the feature branch passed its required platform checks, was merged, and release preparation leaves runtime code, dependencies, packaging, signing and native integration unchanged. Normal CI still runs on the exact preparation commit. Audit candidate artifacts, version/build identity, platform metadata, signatures where applicable, macOS universal slices and checksums. Perform only the release-specific exact-package smoke below. Do not repeat the full Linux/macOS/Windows feature matrix merely because the version changed.

Full native re-acceptance of affected behavior is required if, after prior acceptance, runtime/application code changes; a dependency change may affect runtime or native behavior; packaging logic, signing/notarization, native integration, protocol handling, tray, notifications or process ownership changes; the build toolchain changes materially; or the preparation diff is not metadata-only. A change on one platform calls for that platform's focused native checks; a shared behavior change calls for every affected platform. Record the reason and new evidence before tagging.

## Exact-package smoke

Use packages from one successful, first-attempt candidate run at the exact release-prep commit. Record its run URL, full SHA, package names/hashes, tester, date, OS/architecture and any untested format or architecture. Do not treat a source build or debug acceptance package as the release package. Do not copy credentials or device codes into the record.

For a metadata-only preparation, keep installed-package testing small and release-specific:

- Install and launch one supported release package on each of Linux, Windows and macOS; record the exact format/architecture, expected unsigned/ad-hoc signing and platform warnings. A platform gap needs an explicit release decision, not an inferred PASS.
- Confirm About and the safe support report show the expected version and seven-character commit; confirm native package metadata and application identity.
- Perform at least one real upgrade from the previous release with existing login and representative settings, without logout/reset; save and relaunch, then confirm both persist. Check other platforms' existing profiles where available. If no untouched previous-version profile exists, record the missing gate and obtain an explicit release decision rather than calling a fresh install an upgrade.
- Start one real stream and Stop it on each platform, confirming ordinary exit/owned-process cleanup. Check installed protocol registration only when the release changes or relies on it.
- Record PASS, FAIL or NOT TESTED for each platform and gap. A DEB run does not establish RPM/AppImage execution; an Apple Silicon run does not establish Intel execution.

Every feature, profile, navigation flow, shortcut, notification case or deep-link variant is outside this small smoke unless it changed after prior acceptance. A confirmed package defect requires a fix, fresh CI, a new complete candidate run and affected native retesting. The [release process](releasing.md) defines artifact provenance and publication gates.
