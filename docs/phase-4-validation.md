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

## CI fixture follow-up — 2026-09-18

[Actions run 35365353122](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35365353122), at `b568504e17cbf13de92b3575a8c4c1053b9ed1db`, passed the Ubuntu and macOS jobs, including native compilation. Windows passed the frontend build/tests and Rust formatting, then failed one HTTP contract: 118 unit tests passed and `timeout_cancellation_and_network_failures_are_distinct` reported `Timeout` where the fixture expected `Network`. Later Windows steps were skipped. These CI results do not establish interactive macOS/Windows acceptance.

The network-error fixture assumed a closed loopback port would fail within 50 ms. It now accepts each request and closes the connection without a response, preserving the two-attempt retry assertion without depending on platform refusal timing or releasing the port for reuse. The cancellation case now waits for a request held by a response gate, independently of the short timeout case. Production HTTP classification and retry behavior are unchanged.

Local validation also reproduced Linux `ETXTBSY` (`Text file busy`) when parallel process tests immediately executed freshly copied fake binaries. Renamed helpers now use hard links in temporary directories on the build binary's filesystem; their executable bytes are never rewritten. The existing path-with-spaces, symlink, version, shutdown and process contracts exercise these aliases. Five consecutive runs of all 28 process tests passed after this fixture change.

Final local validation passed all 148 backend/build-script/process tests, Rust formatting, strict all-target Clippy and `git diff --check`. Only test fixtures and this record changed; the frontend and interactive native smoke checks were not repeated. The corrected fixtures still require a new Windows CI run; this local Linux verification does not claim that result.

## CodeQL alert 1 triage — 2026-09-18

[Alert 1](https://github.com/ChrisLauinger77/stream-gui-rs/security/code-scanning/1), from CodeQL 2.27.0 at `b568504e17cbf13de92b3575a8c4c1053b9ed1db`, is a false positive in `rust/cleartext-transmission`. Its SARIF path starts at `http.oauth_base.clone()`, follows the endpoint through the synthetic refresh-error test, and ends at the formatted `/token` request URL. It does not trace an access or refresh token into that URL. The production endpoint is fixed to `https://id.twitch.tv/oauth2`; redirects are disabled, and refresh tokens are form fields sent over HTTPS. The test-only endpoint uses loopback HTTP with synthetic credentials.

The analyzer's [sensitive-name heuristic](https://github.com/github/codeql/blob/codeql-cli/v2.27.0/shared/concepts/codeql/concepts/internal/SensitiveDataHeuristics.qll) classifies `oauth` names as potentially sensitive and excludes names explicitly identifying URLs. The field and constructor parameter now use `oauth_base_url` to describe their actual contents. This is a naming clarification without changes to endpoints, transport, credential handling or tests, and without suppressing a rule or excluding files. A new CodeQL analysis is required to confirm the alert clears; it was not dismissed through GitHub.

Local validation passed all 148 backend/build-script/process tests, Rust formatting, strict all-target Clippy and `git diff --check`. No new credential exchange or native GUI check was needed for this internal rename. The CodeQL CLI is not installed locally, so analyzer confirmation remains pending.

## CI authentication-test synchronization — 2026-09-18

[Actions run 35367936557](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35367936557), at `9436837d1a6fd0b752675a85811c347c591f99b8`, passed every Windows and macOS check. Windows additionally built and uploaded `stream-gui-rs-windows-x86_64`, confirming the NSIS smoke-test packaging workflow on the hosted runner. This establishes installer creation, not interactive Windows acceptance.

The overall run failed on Ubuntu: `offline_restoration_recovers_expired_access_with_one_coordinated_refresh` counted four validations instead of three. Its `join!` started public callers but did not guarantee that their service-owned tasks overlapped. If recovery finished before the explicit validation began, another validation was legitimate. The corrected test holds recovery at the existing refresh gate and explicitly polls the competing owned validation until it waits on the state lock, then releases recovery. The exact three-validation and single-refresh assertions remain intact; application code and CI checks are unchanged.

The original failure did not recur in 100 isolated local attempts. The corrected test passed 100 consecutive runs, and a temporary negative control disabling validation coalescing failed with four calls instead of three. The production implementation was restored byte-for-byte after that check. Final local validation passed all 148 backend/build-script/process tests, Rust formatting, strict all-target Clippy and `git diff --check`. Hosted Ubuntu confirmation requires a new run.

## Windows packaged console finding — 2026-09-18

The user tested the Windows installer from [Actions run 35367936557](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35367936557) in a Windows 11 VM and reported successful installation, Twitch authentication, credential persistence across application restart, real browsing, Streamlink/VLC video playback, multiple streams and process cleanup. A black console appeared immediately on application launch, before playback.

The console belongs to **Stream GUI RS itself**. Inspection of `stream-gui-rs.exe` extracted from that run's `stream-gui-rs-windows-x86_64` NSIS artifact found PE subsystem **3 (Windows CUI)**. CI intentionally packages a debug build; the previous entry-point attribute selected the GUI subsystem only when debug assertions were disabled. Consequently, the installed debug application allocated a console on normal Windows launch.

The entry point now selects the Windows GUI subsystem for Tauri's `custom-protocol` builds as well as non-debug builds. This includes the debug CI installer while preserving the console for ordinary debug development. Existing error reporting, failure exit codes, the default panic hook and captured playback diagnostics remain intact. The Windows subsystem attribute has no effect on Linux or macOS.

Streamlink playback and version probes already share `CREATE_NO_WINDOW | CREATE_SUSPENDED`, followed by kill-on-close Job Object assignment and resume. Those flags, pipe capture, process ownership, Stop/Restart and shutdown are unchanged. Player discovery does not spawn a player; Streamlink launches the selected player. Browser opening uses the existing Windows default-browser adapter and is not invoked automatically at app startup. No player, browser or helper spawn flags were changed.

Regression coverage now includes:

- A Windows-only test that compiles the actual entry point with a synthetic startup implementation, checks PE subsystem values for all four debug/release and development/packaged combinations, and verifies startup errors and panics retain nonzero exits and captured stderr.
- Native fake-Streamlink assertions that both probe and playback invocations have no attached Windows console, exercised by the existing process integration suite.
- A Windows CI check of the actual Tauri application's PE header before installer packaging; a console-subsystem executable fails the job.

Local Linux validation passed the complete application checks: 148 backend/build-script/process tests, 84 frontend tests, two isolated browser regression tests, TypeScript and production frontend build, Rust formatting, all-target desktop checking, strict Clippy, native Tauri debug build without bundling, workflow lint and `git diff --check`. No IPC/DTO contract changed. Windows-only tests and the PE workflow step require the next Windows CI run; macOS was not rerun locally.

**Final Windows acceptance is pending a fresh CI installer containing this fix.** Install/update it in the Windows 11 VM and check normal app launch, Streamlink probing and real playback without consoles; then verify Stop, Restart, two concurrent streams and application close with active playback clean up owned processes. The earlier successful credential-persistence result is recorded above, but no post-fix native Windows result is claimed here. No Phase 5 work was started.
