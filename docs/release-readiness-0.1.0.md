# Version 0.1.0 release readiness

Audit date: 2026-09-19. Candidate source before hardening: `685250ad9816bc19b3afc03e6913bfba3c88b84c` on clean `main`, synchronized with `origin/main`.

## Audit result

- Version is consistently 0.1.0 in Cargo, npm, lockfiles, and Tauri configuration.
- Current release-facing identity is Stream GUI RS / `stream-gui-rs`. No old project name remains in current tracked release-facing content.
- GPL-3.0-only is the project license. The original application is credited as inspiration; its assets are not distributed. The icon entered the repository in the maintainer-authored initial scaffold and has no third-party marker. Package dependencies retain their own licenses.
- GitHub contains the `TWITCH_CLIENT_ID_BUILD` Actions secret by name. Its value was not read or printed. Distribution builds require that embedded public ID; there is no client secret and no release fallback.
- A path-only scan of the current tree and every reachable commit found no private-key headers, concrete token/password/key assignments, `.env` files, credential exports, diagnostic logs, personal home paths, or returned private documentation. Token-pattern matches were ordinary field assignments in credential/provider adapters. Synthetic fixtures remain clearly synthetic.
- The only GitHub code-scanning alert is fixed. No tags or releases existed at audit time.
- The generated stream-to-play icon has one project source and valid PNG, ICO, and ICNS outputs. Product name, executable, publisher, identifier, and version agree with the tested application identity.
- Dependency versions remain frozen at the previously validated lockfiles.

## Release matrix

| Platform | Architecture | Package | Signing status |
| --- | --- | --- | --- |
| Windows 11 | x86_64 | NSIS installer | Unsigned |
| Linux | x86_64 | AppImage and Debian package | SHA-256 only |
| macOS | Apple Silicon arm64 | DMG containing `.app` | No Developer ID signature or notarization; may be ad-hoc signed |

Intel macOS and ARM Linux/Windows are deferred until exact native artifacts can be validated. Flatpak, Snap, RPM, updater, and sandboxed packaging are outside version 0.1.0.

## Adversarial release review

### Blockers

- Candidate status remains blocked on a successful workflow run and native smoke acceptance of every exact artifact.

### Should fix before 0.1.0

- Completed: remove phase terminology from the release landing page and reachable developer screen.
- Completed: replace installed-build OAuth setup advice with a safe broken-build message.
- Completed: add license/repository/author metadata and clear install, runtime, issue-reporting, signing, checksum, and artifact guidance.
- Completed: add explicit package category and descriptions after native package inspection found empty generated values.
- Completed: replace the explicitly temporary developer icon with a distinctive stream-to-play mark and regenerate matching native formats.

### Post-release

- Add Windows code signing and macOS signing/notarization after a dedicated credential/workflow design.
- Evaluate Intel macOS and other architectures only with native artifact testing.
- Consider additional Linux package formats only when their host integration and external-process model are intentionally validated.

## Automated and manual evidence

The established automated suite covers bounded settings, malformed schema rejection, missing/unsupported Streamlink, missing players, credential-store failure, OAuth configuration guards, safe errors, process ownership/reaping, log bounds/redaction, typed IPC, and restricted browser destinations. The application builds executable-plus-argv directly and does not invoke a shell. Windows packaged debug/release subsystem behavior has a regression test.

Prior native validation records successful Twitch authentication, browsing, external playback, Stop/Restart, concurrent streams, browser chat, shortcuts, persistence, and owned-process cleanup on Linux, macOS Apple Silicon, and Windows 11. The Windows record also covers an Actions-built NSIS installer and absence of console windows. These historical observations guide the release matrix; they do not replace the exact-artifact checklist for the candidate run.

## Local hardening validation

On Debian forky/sid x86_64 with Rust 1.95.0 and Node 24.20.0:

- Generated TypeScript bindings matched the tracked file.
- TypeScript and the Vite production build passed.
- All 84 frontend tests passed.
- Rust formatting, all-target desktop checking, and strict Clippy passed.
- The Rust suite passed 119 unit/HTTP tests, one build-script guard, and 28 native process-lifecycle tests. HTTP fixtures used loopback access only.
- Both isolated Linux browser-dispatch/launcher-cleanup tests passed.
- The feature-enabled native Tauri debug/no-bundle build passed.
- A native optimized Linux release build produced an executable x86_64 AppImage and Debian package. Inspection confirmed product/version, maintainer, homepage, WebKitGTK dependency, Video desktop category, icon, non-terminal launch, descriptions, and executable mode.
- `git diff --check` passed.

The local packages used the documented synthetic compile-only client ID and are not release candidates. Only the hosted workflow packages built from the final exact commit with the registered public ID qualify for the manual release smoke test.

Current candidate workflow and artifact URLs are recorded in the task result after they run, avoiding a documentation-only commit that would change the tested candidate SHA.
