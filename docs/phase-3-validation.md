# Phase 3 — basic Streamlink playback validation

Implemented on top of Phase 2 baseline `8a7e5dc`. No new crate/package or second process supervisor was added. Phase 4+ transport modes, embedded playback, recording, notifications, chat profiles, tray/background mode, updater and legacy GUI migration remain out of scope.

## Implementation commits

- `5b85cac` — quality policies, literal argv encoding, player discovery and versioned settings.
- `27ec17d` — restart generations, cancellation/capacity safeguards and native process tests.
- `7297721` — trusted Twitch identity, playback services, typed IPC/capabilities and close handling.
- `4a004ed` — Watching, diagnostics, playback settings and authoritative snapshot polling.
- `e28b4e7` — Watch actions throughout browsing and frontend behavior tests.
- The following documentation commit records architecture, usage and validation evidence.

## Automated verification

Final macOS suite (2026-09-18):

| Check | Result |
| --- | --- |
| Rust backend / HTTP / configuration / command contracts | 100 passed |
| Native fake-executable process contracts | 24 passed |
| Total Rust tests | 124 passed (baseline 99 preserved) |
| Frontend behavior and CSS contracts | 69 passed (baseline 51 preserved) |
| TypeScript and Vite production build | passed |
| Rust formatting, desktop all-target checks | passed |
| Strict Clippy (`-D warnings`) | passed |
| Native macOS debug Tauri `.app` bundle | passed |
| Patch whitespace check | passed |

The unchanged CI matrix runs frontend checks, backend/native process tests, desktop checks, strict Clippy and a Tauri binary build on Ubuntu 24.04, Windows and macOS. The macOS run above does **not** establish a remote CI result or native Linux/Windows playback verification; the separate Linux run is recorded below.

Command tests assert exact argv for all five quality policies, all four player modes, spaces/Unicode paths, quotes, Windows-style backslashes, braces, empty arguments and malicious-looking metadata. Invalid types, control characters, excessive argument counts/length and malformed channel identities are rejected. Settings tests preserve the app's own version 1 path, round-trip version 2 fields, reject unknown schemas, and leave persisted settings intact on invalid updates.

Native fake-child tests retain every Phase 0 scenario and add production argv/metadata, delayed output, split UTF-8, no final newline, stderr warnings/errors that do not change process state, early/later nonzero exits, multiple sessions, isolated Stop/Restart, generation checks, rapid concurrent restart, Stop/shutdown during pending restart, restart spawn failure, dropped callers, aborted lower-level restart futures, bounded capacity and cancelled authentication at the final spawn boundary. Existing tests cover simultaneous pipe draining, bounded/redacted logs, process-tree cleanup, symlink updates, idempotent stop, history eviction and shutdown/probe races. Helix tests check fresh trusted identity, changed login, offline/wrong broadcaster, malformed login and logout during lookup.

Frontend tests exercise Watch across Following/Live/category/search/details, offline/unknown exclusion, native keyboard-activatable button semantics, duplicate pending requests, typed failures without raw backend payloads, multiple sessions, isolated stop, explicit quality/restart, Stop during restart, diagnostics, literal settings persistence, player/probe feedback, remount reconstruction, logout independence and stale snapshot suppression. All Phase 2 navigation, focus, pagination, cache/freshness and image-retry tests remain intact. The keyboard unit test simulates a keyboard-generated native-button click because jsdom does not implement browser key default actions; end-to-end keyboard activation still merits native-platform confirmation.

Commands used:

```sh
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --offline --locked --manifest-path src-tauri/Cargo.toml --no-default-features --features test-support
cargo check --offline --locked --manifest-path src-tauri/Cargo.toml --all-targets --features test-support
cargo clippy --offline --locked --manifest-path src-tauri/Cargo.toml --all-targets --features test-support -- -D warnings
CARGO_NET_OFFLINE=true npm run tauri build -- --debug --bundles app --ci --config '{"bundle":{"active":true}}'
git diff --check
```

## Native macOS evidence

Real authenticated browsing and real Twitch streams were used locally. Account identifiers, channel history, credentials, media and screenshots are not checked into this repository.

- Automatic discovery found Streamlink 8.6.1 from Homebrew with a successful native version probe. Both mpv 0.41.0 and VLC 3.0.23 were detected; saved choices survived app restart.
- Watch launched Source playback in mpv. The user confirmed video was playing; the automation could not directly inspect the non-bundled Homebrew mpv window. Restart preserved the session ID and replaced its process. Stop removed the owned processes.
- The first real VLC test exposed an unsupported macOS `--no-one-instance` option. The preset was corrected to emit it only on Linux/Windows; local VLC help confirmed `--play-and-exit`. The corrected build displayed actual live video in VLC, verified visually.
- Two different streams ran in independent Streamlink process groups with separate VLC children. Stopping the first removed its tree; the second session remained running and its video continued, verified visually.
- An explicit Audio restart remained running under the same session identity. Audio rendition/argv mapping is covered by exact command tests; audibility was not independently verified.
- Explicit Streamlink and custom VLC executable paths were saved and used for a real restart. The resulting supervised native processes were observed. Visual confirmation was obtained for the VLC preset, not separately for every custom/default launch.
- Bounded diagnostic output and discarded-entry reporting were inspected. Real mpv progress output was noisy, so the final mpv preset uses its documented `--quiet` option to suppress the status meter while retaining useful messages.
- Streamlink default mode launched two retained sessions with automatic player discovery. Before closing the window, the two Streamlink and two VLC process identities/groups were recorded. Closing the main window ended the application and **all four owned child/descendant processes**; a subsequent process-table check found none remaining. The earlier explicit Quit path also exited cleanly.
- Original automatic Streamlink/default-player/Source preferences were restored after the smoke test. No active process/session state was persisted. Frontend reconstruction and logout independence were tested automatically; live logout was not performed, to preserve the existing login. Native webview reload and keyboard-only launch remain manual follow-ups.

## Native Linux baseline verification — 2026-09-18

Baseline: `f650bab50780f3a029c1d8b819f2c0d1d42a05d5` on `main`. The working tree was clean before testing, and a live `git ls-remote` check confirmed that HEAD and `origin/main` matched GitHub's current `main`. No application source, dependency declarations, lockfiles or repository configuration were changed. This validation record is the only tracked change; no Phase 4 work was started.

Environment: Debian GNU/Linux forky/sid, x86_64, kernel `7.1.13+deb14-amd64`, GNOME Wayland session; Debian Rust/Cargo 1.95.0, Node 24.20.0 and npm 12.0.2. The native prerequisites included build-essential, WebKitGTK 4.1 (2.52.6), GTK 3.24.52, OpenSSL 3.6.4, D-Bus 1.16.2, librsvg 2.62.1 and libxdo development files. The user session provided GNOME Keyring's Secret Service.

Rustfmt, Clippy and patchelf were initially absent. Matching Debian rustfmt/Clippy 1.95.0 packages and patchelf 0.18.0 were downloaded and extracted under a temporary directory, then added to PATH for verification. No system packages were installed. Future checks require these tools on PATH; this was an environment setup gap, not an application compatibility defect. Tauri's rustup warning reflects the Debian-managed Rust installation; the native checks and build succeeded with that installation.

| Check | Result |
| --- | --- |
| Locked frontend installation (`npm ci`) | passed |
| Frontend behavior and CSS contracts (`npm test`) | 69 passed |
| Explicit TypeScript check and production frontend build | passed |
| Rust backend/unit/HTTP contracts | 106 passed |
| Actual client-ID build-script guard test | 1 passed |
| Native fake-executable process contracts | 24 passed |
| Total Rust tests | 131 passed |
| Rust formatting, desktop all-target checks | passed |
| Strict Clippy (`-D warnings`) | passed |
| Native Linux Tauri debug executable (`--debug --no-bundle --ci`) | passed |
| Patch whitespace check | passed |

Checks used the repository's documented locked Cargo commands and `test-support` only for tests/checks. The initial compilation check used the synthetic public client ID; the app was then rebuilt with the registered public ID supplied for this run, without `test-support`. Native acceptance used that second binary with embedded configuration. Network-dependent dependency setup, loopback HTTP tests and native session access ran outside the execution sandbox. Tests used synthetic credentials; real authentication was exercised only through the native app.

Native acceptance evidence:

- The app launched successfully. The user confirmed completing fresh Twitch sign-in during this run. Authenticated Following results then loaded in the native webview.
- Settings' automatic Streamlink probe reported `/usr/bin/streamlink`, version **8.5.0**. The app's player discovery reported `/usr/bin/mpv` and `/usr/bin/vlc`; installed versions were **mpv 0.41.0** and **VLC 3.0.23**.
- Native UI accessibility inspection verified real Following results, 30 Live results, a live channel's details, 30 categories and a category's live streams. The query status reported **Updated from Twitch**, with no visible error alerts. Back navigation returned from channel details.
- A real stream launched from the app with the mpv preset and Source quality. Process inspection observed Streamlink parented by Stream GUI RS and mpv parented by Streamlink, sharing the owned playback process group. Both remained running for several minutes. **Moving video and audible audio remain pending user confirmation; process existence alone is not visual or audio acceptance.**
- Stop was exercised through the native Watching control. The UI reported **Playback stopped** and **exited**. Both recorded Streamlink/mpv PIDs disappeared, and a process-table check found no remaining member of their process group; the app remained running.
- Normal window close exited the app successfully. After relaunch with the same embedded public ID, authentication restored automatically and fresh Following results loaded without another sign-in. This verifies persistence/restoration through Linux Secret Service. No playback processes remained after restart.

No Linux application defect was found in the completed checks. Full playback acceptance remains open until video/audio is confirmed. VLC playback, live token rotation/logout and locked-store cases, all quality/player variants, keyboard-only interaction, concurrent real streams and closing with active playback were not verified in this run. Native Windows and macOS were not retested. Historical platform results and the broader checklist remain separate evidence.

## Repeatable manual checklist

Use two currently live public channels; do not put account names or tokens in this document.

1. Launch the native app; confirm sign-in restoration and live browsing. Check that no old playback session is restored after a complete application exit.
2. In Settings, test automatic Streamlink discovery and version. Repeat with an explicit executable path (including spaces/Unicode where available). Try a missing file and an unsupported version; confirm typed errors and preserved settings.
3. Find installed mpv/VLC; save each preset and test Watch. Confirm actual video, not merely Running or a `Starting player` log. Test an explicit player override and custom mode; try a missing player.
4. Exercise Source, High, Medium, Low and Audio when offered. Verify actual selected renditions in diagnostics; High/Medium/Low can intentionally exceed their cap via the final source fallback. Verify Audio audibility locally.
5. Launch from Following, Live, category streams, live search results and live channel details. Confirm offline/unknown channels have no Watch. Tab to Watch and activate it with Enter/Space.
6. Run two different streams. Stop A twice; B must continue. Restart A and verify B is unchanged. Change A's quality without restarting (nothing should relaunch), then press Restart. Try rapid repeated restart and Stop during restart.
7. Open session diagnostics, keyboard-focus/scroll the log, and inspect an early failure and later process exit. Confirm logs remain bounded and that stderr text cannot itself mark a process failed.
8. Navigate, open/close Settings/Watching, and reload the webview with streams running; reconstruct from Rust without relaunching. Optionally sign out: existing sessions continue and remain stoppable/restartable, but browsing launch requires sign-in.
9. Close the main window with two active sessions; confirm the app and owned Streamlink/player processes exit. Repeat with explicit Quit. Document custom players that intentionally detach, since those escape normal process-tree ownership.
10. Repeat on native Linux and Windows. On Windows check `.exe` validation, Program Files/Unicode paths, hidden consoles and job cleanup. On Linux check executable bits and usual system/pipx locations. On macOS test Finder/GUI PATH limitations and both Homebrew prefixes where available.

## Input boundary and review

Only two process-creation sites remain: the version probe and the existing supervisor. Both use a path and argv vector with concurrent bounded stdout/stderr handling; no shell is constructed. Production launch accepts a broadcaster ID/public auth generation, then fetches trusted live identity. Prototype URL/quality DTOs and adapters compile only for tests/test-support. Titles, display names and category names never enter argv. Auth cancellation is checked again under the process registry lock before spawn. No OAuth token or refresh token occurs in playback/frontend DTOs or command construction.

Restart serializes by session, checks generation, reserves capacity, waits for prior reaping and reader completion, and checks Stop/shutdown cancellation before replacement. Error/abort paths release reservations and retain diagnostics. Rust remains authoritative; frontend pending keys are request state only. No dependency, capability for generic execution/filesystem access, background monitoring or future transport architecture was added.

## CLI contract references

- [Current Streamlink CLI documentation](https://streamlink.github.io/cli.html): stream preference lists, sorting exclusions, `best-unfiltered`, default player discovery, `--player`, `--player-args`, `--player-verbose`, and disabling configuration/plugin sideloading.
- [Streamlink 8.6.1 player argument implementation](https://github.com/streamlink/streamlink/blob/8.6.1/src/streamlink_cli/output/player.py): executable paths are separate from the argument string; Formatter precedes POSIX `shlex.split` on every platform. The installed 8.6.1 implementation was directly exercised: nine edge cases (quotes, braces including literal `{playerinput}`/`{playertitleargs}`, Unicode, empty arguments, Windows paths and shell-looking strings) round-tripped exactly, with stdin appended once.
- [Original Streamlink Twitch GUI quality policies](https://github.com/streamlink/streamlink-twitch-gui/blob/master/src/app/data/models/stream/-qualities.js): policy mapping was compared with the local original-app checkout and current Streamlink syntax.
- Installed mpv 0.41.0 manual: `--quiet` suppresses its status line; it does not mean the stronger `--really-quiet`/`--terminal=no`. Native VLC 3.0.23 help verified the macOS option difference.
