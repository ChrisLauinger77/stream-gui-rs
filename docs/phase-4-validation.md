# Phase 4 validation — settings and MVP completion

Verified natively on Linux on 2026-09-18. Phase 4 implements persistent global/channel preferences, Rust effective-settings resolution, compact Settings, external browser chat, themes and focused shortcuts. Phase 5 has not begun: no followed-stream monitor, notifications, tray, badges or minimize-to-tray behavior was added.

## Baseline and environment

The preceding [Phase 0–3 Linux baseline](phase-3-validation.md#native-linux-baseline-verification--2026-09-18) passed before application behavior changed. That record includes clean/up-to-date repository verification, 131 Rust and 69 frontend tests, native build, fresh user-completed Device Flow, Secret Service restoration, real browsing, user-confirmed mpv video and Stop/process cleanup. Historical macOS evidence remains unchanged.

Phase 4 used Debian GNU/Linux forky/sid, x86_64, kernel 7.1.13+deb14-amd64, GNOME Wayland with GNOME Keyring Secret Service; the tested Tauri window used the X11/XWayland GDK backend. Tools: Rust/Cargo 1.95.0, Node 24.20.0, npm 12.0.2, GTK 3.24.52, WebKitGTK 4.1 2.52.6, OpenSSL 3.6.4 and D-Bus 1.16.2. Build-essential, libxdo and librsvg development prerequisites were present. Matching Debian rustfmt/Clippy and patchelf were temporarily extracted onto PATH; no system package installation or application dependency workaround was used.

Native runtime dependencies were Streamlink 8.5.0, mpv 0.41.0 and VLC 3.0.23, discovered under `/usr/bin`. Firefox ESR was the configured default browser. GNOME exposed `prefer-dark` during System-theme verification. Builds used the registered public Twitch client ID provided for this run, embedded in Rust. No credential values were inspected or copied. Account captures, browsing history, raw diagnostic logs and build products are not committed.

## Automated checks

| Check | Result |
| --- | --- |
| Rust unit/domain/HTTP contracts | 119 passed |
| Actual client-ID build-script guards | 1 passed |
| Native fake-executable process contracts | 28 passed |
| Backend total (`--no-default-features --features test-support`) | 148 passed |
| Isolated Linux default-browser dispatch/launcher test and subprocess helper | 2 passed |
| Frontend behavior and CSS contracts | 84 passed |
| TypeScript and production Vite build | passed |
| Rust formatting, all-target desktop check, strict Clippy | passed |
| Native Tauri debug build, no bundle, no test-support | passed |
| Patch whitespace check | passed |

Commands from the repository root:

```sh
npm run build
npm test
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --locked --manifest-path src-tauri/Cargo.toml --no-default-features --features test-support
cargo test --locked --manifest-path src-tauri/Cargo.toml --test linux_browser_open --features test-support
cargo check --locked --manifest-path src-tauri/Cargo.toml --all-targets --features test-support
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets --features test-support -- -D warnings
TWITCH_CLIENT_ID_BUILD=yourPublicClientId npm run tauri build -- --debug --no-bundle --ci
git diff --check
```

The browser regression isolates XDG associations in a subprocess and dispatches a synthetic Twitch URL to the existing native fake executable. It verifies the exact URL and that the launcher PID disappears; it never opens the user's browser. Linux CI now runs this test. Normal HTTP/auth tests continue to use local fixtures and synthetic credentials, not Twitch or the user's keyring.

New contracts cover schema 1/2 migration without premature rewriting, defaults/round trips, strict malformed/unknown schemas, invalid channel IDs, bounded files/records, atomic-write failure, global/channel/request precedence, false versus inherit, removing overrides, immutable launch snapshots, current-settings restart, isolation and failed executable probes preserving saved values. Existing process ownership, argv escaping, cancellation and session tests remain intact.

Chat tests cover fixed-origin URL construction, injection rejection, fresh session-bound identity including offline users, logout during lookup, queued cancellation, safe native errors, automatic on/off/inheritance, generation-checked late results and playback continuing when chat fails. Frontend tests cover section drafts and Save/Cancel, loading/duplicate guards, channel errors/late responses, effective-preview refresh, narrow manual chat, backend-owned automatic chat, restart inheritance, System changes/persistence, Control/Command conventions, focus/Back/Refresh, input/modal suppression and light/dark contrast.

## Native Linux acceptance

Checks used the actual Tauri/WebKitGTK application, Linux accessibility controls, process-table/group inspection and app/player window inspection. User observations are identified explicitly.

| Area | Observed result |
| --- | --- |
| Authentication | Fresh sign-in was completed in the baseline run. Phase 4 repeatedly restored the same login after normal app exit and loaded fresh Following data without another sign-in. |
| Real browsing | Following and channel details loaded real Twitch data; the user also exercised real channel search while checking shortcuts. Baseline Live/categories/category-stream checks remain recorded separately. |
| Discovery | The native Settings probe reported Streamlink 8.5.0 and `/usr/bin/streamlink`; Find installed players returned `/usr/bin/mpv` and `/usr/bin/vlc`. |
| Global quality | Saving High left the existing Source Streamlink/mpv PIDs unchanged. A subsequent launch used `high,best,best-unfiltered`; the original run retained `best`. |
| Channel quality | Saving Low in channel details produced a new `low,best,best-unfiltered` run. Returning to global inheritance showed High in the Rust preview. Explicit Restart replaced the Low run with High. |
| Restart and isolation | Restart reaped the replaced Streamlink/player pair and retained the other sessions. Stopping one of two concurrent sessions removed only its tree. These native concurrent runs used the same live channel; different-channel isolation is covered by the native fixture suite and earlier macOS evidence. |
| Player change/video | Saving VLC left mpv playback unchanged until explicit Restart. The replacement used `/usr/bin/vlc`; actual live video was visually observed in VLC. Returning to mpv/Source produced the expected replacement process tree. The user confirmed mpv video during the Linux baseline. |
| Manual chat | Open chat in browser succeeded; the user confirmed the Twitch chat page opened in Firefox. Retested successfully after the GIO cleanup fix. |
| Automatic chat | Enabled globally and exercised on Restart and a new launch. An explicit channel-off override suppressed the next browser dispatch. Returning to inheritance enabled it again. The user confirmed automatic chat opened correctly. |
| Themes | System, Light and Dark were saved through the native UI. Light and Dark were visually inspected; System matched the desktop's dark preference. Dark survived app restart. A temporary light desktop/GTK preference propagated through the portal/WebKitGTK to System appearance; restoring the original preferences returned it to Dark without restarting the app. |
| Shortcuts | The user confirmed native Ctrl+2, Ctrl+3, Alt+Left, Ctrl+4, Ctrl+comma and Ctrl+K; Search received focus and Ctrl+2 while typing did not navigate. Ctrl+1/Refresh and macOS Command behavior additionally have automated coverage. |
| Persistence | Restart restored High, VLC, Dark and the explicit channel chat-off override. No playback session was restored after app exit. The original schema 2 preferences migrated to schema 3 on save. |
| Cleanup | Normal close with two active VLC streams removed the app and both recorded Streamlink/VLC trees. Final Stop removed the remaining mpv test tree. Browser dispatch after the GIO fix left no zombie children. |

The secure credential-store result is native storage/restoration through the existing Secret Service implementation, not an inference from a CI matrix. The production implementation has no plaintext or automatic memory fallback. Real logout/deletion, refresh rotation, locked-keyring and denied-access scenarios were not exercised in this phase; earlier deterministic tests continue to cover their contracts. The user's authenticated session was preserved.

After smoke checks, original automatic Streamlink/mpv/Source preferences were restored, automatic chat was disabled, appearance returned to System and all test channel overrides were removed. The app was left authenticated with no test playback running. Browser chat tabs opened during verification belong to the browser and are not playback children.

## Defects found and focused fixes

1. **Linux browser launcher cleanup:** the existing `webbrowser` dependency's Linux background path spawned and discarded its child handle. Native manual/automatic chat left short-lived Firefox launcher zombies under the app. Linux dispatch now uses GIO's default URI handler and awaits dispatch on a private blocking-worker context; authentication opening uses the same adapter. Native retest showed zero zombie children. The isolated regression verifies launcher disappearance without opening a real browser. GIO was already resolved through Tauri/GTK; only an explicit optional Linux desktop dependency was added, with no new resolved package. macOS/Windows dispatch remains unchanged.
2. **Malformed startup configuration:** store validation preserved invalid files, but Tauri's setup-hook error path panicked. Service/config initialization now occurs before the event loop and propagates a safe startup error. A graphical-session regression uses isolated temporary XDG configuration and a synthetic client ID, checks invalid JSON and a future schema, requires exit code 1 without panic, and verifies byte-for-byte preservation. It is ignored in ordinary headless runs and was explicitly run natively:

   ```sh
   cargo test --locked --manifest-path src-tauri/Cargo.toml --test linux_startup -- --ignored
   ```

   Result: 1 passed. It does not access the user's configuration or secure-store entry.
3. **Settings validation:** explicit global Streamlink-path saves now require a successful supported-version probe before persistence, matching the dedicated Test path. Regression tests prove unsupported probes/saves leave the valid settings file unchanged.

## Review and remaining platform checks

Rust remains authoritative for settings and effective resolution. Channel preferences intentionally apply across accounts on this device; Twitch browsing/chat/launch authority remains session-bound. New IPC accepts typed settings or stable IDs, never arbitrary keys, URLs, shell text or filesystem destinations. The command map, manifest, generated DTOs, permissions and local-window capability are aligned. Existing CSP, credential boundaries, player argument encoding, Unix groups and Windows jobs are preserved.

Native Windows/macOS were not run here; their existing CI matrix and OS-specific adapters remain structurally supported, which is not a claim of new native validation. Not every empty/error/diagnostic state was visually rechecked in both themes; shared CSS contrast and behavior tests cover these surfaces. Audio audibility, custom/default-player variations, every quality rendition, real logout/refresh, offline manual chat, and native webview reload remain manual follow-ups; they are not claimed as observed successes.

No media transport, embedded chat/video, chat-client launcher, profile framework, recording/VOD feature, legacy import, updater, publishing workflow or Phase 5 behavior was introduced.
