# Release artifact smoke test

Before publication, use only files downloaded from one successful manual Release candidate run. Record its URL, run ID, exact 40-character commit SHA, SHA-256 files, artifact names, tester, date, OS version/architecture, Linux distribution/desktop/session, Streamlink version and player version. After publication, verify that downloads have those same checksums. Do not rebuild or substitute files after testing.

## Scope for 0.4.0

Complete the integrity, focused 0.4.0 and relevant platform checks on Windows, Linux (prefer DEB) and macOS using the same candidate run. Perform the real 0.3.0 upgrade below on at least one platform **before** any logout, reset or uninstall. Preserve an existing profile on the other platforms where available; mark fresh-install credential checks separately. If only Phase 7 test profiles remain, preserve them and record their starting build/schema; their upgrade checks do not establish an untouched released-schema migration. Record that limitation explicitly for candidate acceptance rather than downgrading or resetting a user's profile.

The detailed historical background/Notification Center stress scenarios remain a reference for changes affecting those areas; they are not all repeated for this release. Record untested Linux formats, Intel/Apple Silicon execution, high-DPI and screen-reader coverage explicitly. Metadata/architecture checks still cover every package. An incomplete exhaustive screen-reader audit alone is not a blocker; a keyboard trap, unreachable action, invisible focus, unusable 150% layout or false status is.

## Integrity and common checks

- [ ] The candidate run identifies the intended commit and version. No tag is needed for candidate testing.
- [ ] Every downloaded filename identifies the release version, platform, and architecture.
- [ ] `SHA256SUMS_<platform>_<architecture>.txt` verifies every distributable in its artifact archive.
- [ ] Install or open the package without setting `TWITCH_CLIENT_ID` or `TWITCH_CLIENT_ID_BUILD`.
- [ ] The application identifies itself as Stream GUI RS X.Y.Z and the expected short commit in About, diagnostics and the support report on every platform.
- [ ] First verify restored credentials on upgrade profiles. Exercise Device Code sign-in separately on a fresh profile or after the upgrade result is recorded.
- [ ] Quit and relaunch; the authenticated session restores from native secure storage.
- [ ] Following, Live, Categories, Search, and channel details load real Twitch data.
- [ ] Streamlink 8.0+ is discovered or accepted by absolute path.
- [ ] The intended player is discovered or accepted by absolute path.
- [ ] Watch a real live channel and confirm video or audio in the external player.
- [ ] Stop playback and confirm the owned Streamlink/player processes exit.
- [ ] Restart playback and confirm one replacement session starts.
- [ ] Start two streams, stop each independently, then close the app with playback active and confirm owned processes exit.
- [ ] Automatic browser chat and manual **Open chat in browser** open the correct Twitch channel.
- [ ] System, Light, and Dark themes and the documented shortcuts work.
- [ ] Uninstall or remove the application and note any unexpected residue. User settings and native credentials may remain until explicitly cleared.

Never copy real OAuth tokens, refresh tokens, device codes, or credential-store contents into the test record.

## Windows 11 x86_64

Artifacts: `Stream-GUI-RS_X.Y.Z_windows_x86_64-setup.exe`, `Stream-GUI-RS_X.Y.Z_windows_x86_64.zip`, and `stream-gui-rs.json`

- [ ] Note the expected unsigned-publisher/SmartScreen warning; no unexpected publisher identity appears.
- [ ] Programs and Features shows Stream GUI RS, version X.Y.Z, and publisher ChrisLauinger77.
- [ ] Normal application startup displays no console window.
- [ ] Streamlink probes and playback start without helper console flashes.
- [ ] Uninstall succeeds.
- [ ] The portable ZIP contains `stream-gui-rs.exe`, `LICENSE`, and `README.md` at its root and launches without an installer or console window.
- [ ] After publication, add `https://github.com/ChrisLauinger77/scoop-bucket` as bucket `ChrisLauinger77`; `scoop install ChrisLauinger77/stream-gui-rs` installs the ZIP, creates the Stream GUI RS shortcut, launches, updates, and uninstalls successfully.

## Linux x86_64

Artifacts: `Stream-GUI-RS_X.Y.Z_linux_x86_64.AppImage`, `Stream-GUI-RS_X.Y.Z_linux_amd64.deb`, and `Stream-GUI-RS_X.Y.Z_linux_x86_64.rpm`

Record each format separately on matching desktop sessions; 0.4.0 requires a native Linux candidate smoke test, preferably DEB. Content/metadata checks cover all three formats; do not infer RPM/AppImage execution from DEB success.

- [ ] AppImage is executable and starts from the desktop environment.
- [ ] AppImage startup emits no GVFS `undefined symbol` error and no `EGL_BAD_PARAMETER` abort.
- [ ] Debian package installs with declared dependencies, creates the expected launcher/icon, and starts from that launcher.
- [ ] RPM installs with declared dependencies on an RPM-based x86_64 distribution, creates the expected launcher/icon, and starts from that launcher.
- [ ] Authentication persists through the session's Secret Service provider.
- [ ] XDG configuration is written under the current user, with no credentials in settings files.
- [ ] Browser sign-in and chat opening do not show a remote-screen permission prompt caused by the application itself.
- [ ] Debian package uninstall succeeds.
- [ ] RPM uninstall succeeds.

## macOS universal

Artifact: `Stream-GUI-RS_X.Y.Z_macos_universal.dmg`

- [ ] The disk image mounts and contains `Stream GUI RS.app`.
- [ ] Info.plist declares macOS 11.0 as its minimum, matching the native notification APIs. Record the actual tested macOS version; this metadata check alone does not prove execution on the oldest supported system.
- [ ] `lipo -archs` reports both `arm64` and `x86_64` for the executable under `Stream GUI RS.app/Contents/MacOS`.
- [ ] Record the expected Gatekeeper/quarantine behavior for the app without Developer ID signing or notarization.
- [ ] `codesign --verify --deep --strict --verbose=2 '/Applications/Stream GUI RS.app'` succeeds. The installed bundle has sealed resources and a bound Info.plist, not merely a linker-generated executable signature.
- [ ] After the tester explicitly allows it, the app starts without terminal environment variables.
- [ ] Finder reports version X.Y.Z and the same exact disk image runs natively on Apple Silicon.
- [ ] Open **Stream GUI RS → About Stream GUI RS**: the native panel shows `Stream GUI RS` and `Version X.Y.Z (<first seven characters of the release commit>)`, with no duplicate application version. Click the repository link and confirm the default browser opens `https://github.com/ChrisLauinger77/stream-gui-rs`. Check both light and dark appearance and confirm the other standard application menu actions still work.
- [ ] The same exact disk image runs natively on an Intel Mac.
- [ ] Keychain persistence, browser sign-in, chat opening, Streamlink discovery, playback, and cleanup pass.
- [ ] Dragging the app to Applications and removing it both work as expected.
- [ ] After publication, `brew tap ChrisLauinger77/cask` and `brew install --cask stream-gui-rs` install the same universal DMG from the release, and `brew uninstall --cask stream-gui-rs` succeeds.

## Extended background regression reference (0.2.0 and later)

- [ ] Upgrade a version 3 settings file; playback/theme/channel preferences survive and monitoring, notifications and background close default off.
- [ ] Enable monitoring/notifications, grant permission where requested, and confirm the initial live list is quiet.
- [ ] Observe a genuinely new followed stream; one notification contains the correct channel/title/category. Repeated polls and a brief disappearance do not repeat it.
- [ ] Channel notification Inherit/Enabled/Disabled overrides behave as displayed.
- [ ] Click a notification while hidden/minimized: restore/focus the app and select the channel without launching playback. An old-account notification cannot navigate after logout/re-login.
- [ ] Pause from Settings and tray; Resume quietly. Reload the interface and confirm native monitoring/playback/tray state reconstructs.
- [ ] Disconnect/reconnect networking and suspend/resume the computer: recovery is quiet, the next future transition can notify, and foreground browsing remains responsive.
- [ ] Tray Show/Hide/Watching/monitor controls reflect Rust state. Normal minimize remains an OS minimize.
- [ ] With background close enabled, close while two real streams play, restore, then Stop/Restart each independently. Explicit Quit stops owned processes and ends monitoring.
- [ ] With background close disabled, closing exits and cleans up playback.
- [ ] Linux: test both an action-capable notification server and one without actions; verify app identity. Test with/without a StatusNotifier host and indicator library, including loss of a host while hidden.
- [ ] Windows: test the installed Start-menu identity, denied notifications, click activation, and tray controls. Record portable-build limitations separately.
- [ ] macOS: test the installed bundle, first permission prompt, denial/re-enable through System Settings, click focus, menu-bar controls, and Command-Q with playback.

Synthetic native fixtures and cross-target API checks do not substitute for these exact-artifact observations. Record missing checks explicitly.

### Windows Notification Center acceptance

Run against the exact installed Windows candidate, launched from its Start-menu shortcut. Linux policy tests and Windows source/API checks do not verify the shell's activation behavior.

The [development-only notification trigger](notification-acceptance.md) lets an installed debug build exercise delivery, natural banner timeout, retained activation and cleanup without waiting for Twitch. Use its synthetic target for those checks; retain real-stream/account checks below for monitor/session acceptance and repeat release acceptance against the exact release candidate using a real followed-channel transition. The synthetic control must remain absent from normal release packages; a separate debug acceptance package is supporting evidence only.

- [ ] Confirm the notification's Stream GUI RS identity/icon and the matching entry in Windows notification settings. Enable monitoring and notifications; let the initial baseline complete quietly.
- [ ] Observe a new followed stream, let its banner time out naturally, and confirm its entry remains in Notification Center. Hide/minimize the app, then click that retained entry within 15 minutes: the app restores/focuses and selects the correct channel exactly once, with no playback launch.
- [ ] Click a visible banner in a separate trial; verify the same navigation and no second navigation from another activation callback.
- [ ] Dismiss an entry explicitly, then check that it disappears and cannot activate. Let another entry expire for 15 minutes, including a suspend/resume trial; it must be removed and cannot navigate after expiration.
- [ ] Retain an entry, then Pause, logout, or replace the login (test both another account and the same account under a new session). The old entry is retired; a delayed click must not restore an old account's channel workspace.
- [ ] Exercise enough transitions to reach the 32-entry limit: delivery remains bounded and safely reports failure when full. Dismiss/expire entries and confirm later delivery resumes without replaying suppressed transitions.
- [ ] Open a second app instance while the first retains a notification. The second reports notifications unavailable and must not clear or take over the first instance's notification; the first entry still activates its original window.
- [ ] Quit with retained entries and two owned streams: notifications disappear, owned playback and background work exit, and delayed activation cannot relaunch the app. Relaunch and confirm monitoring starts quietly. After a separately controlled forced termination, relaunch and confirm orphaned notification history is removed.
- [ ] Deny notifications in Windows settings, then re-enable them; verify permission reporting, subsequent delivery, Notification Center activation and tray controls. Record portable-executable behavior separately.

On a Windows development checkout, also run the two native helper tests (isolated kernel-object ownership and bounded COM payload decoding):

```sh
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib --features test-support desktop::notifications::windows::activation::tests
```

These helper tests do not replace the installed-artifact checks above. macOS bundle permission, denial/re-enable, notification focus, menu-bar controls and Command-Q acceptance remain separate pending checks on both supported native architectures.

## Upgrade from 0.3.0 and result record

At least one real native upgrade is required before approval. Quit 0.3.0 normally with a stored Twitch login and representative Streamlink/player paths (including spaces where available), arguments, quality, automatic browser chat, theme, monitoring/notification/close settings, language, low latency, text size and channel quality/chat/notification/low-latency overrides. Install the exact candidate over it without logout, reset, uninstall or clearing native credentials. Never inspect, copy or print token values.

Confirm startup without migration errors, automatic sign-in and unchanged old preferences. New defaults are Browser chat, no Chatterino override, an empty profile list and Default configuration selected; update awareness starts unchecked and makes no startup request. Migration remains in memory until a settings save writes schema 6; save once, Quit/relaunch and verify settings/login/playback again. Downgrading to 0.3.0 after a schema 6 save is unsupported. Automated migrations 1–5 complement this real-profile check; credentials remain in native storage and are not part of the settings fixture.

## Focused 0.4.0 candidate checks

Run on each of Linux, Windows and macOS and record PASS, FAIL or NOT TESTED with the exact artifact/hash and environment:

- [ ] Check for updates and Refresh show immediate progress and the correct result. Before publication, 0.4.0 is newer than stable 0.3.0; after publication it is up to date. View release opens only the official page and leaves the accepted update status unchanged. Check quiet failure/recovery where practical.
- [ ] Start real playback with profile A, select profile B and verify the running stream stays unchanged. Restart applies B; Stop cleans up only that session. Delete the selected profile and confirm future launches use Default. Record player/profile settings and independent-session behavior.
- [ ] If installed, select Chatterino and open the requested channel from stopped and running states. Record its installation method and actual window/process behavior. Automatic chat and explicit browser fallback work; Stop/Quit leave Chatterino independent. If unavailable, record Chatterino as untested and verify browser chat.
- [ ] Real Twitch browsing, language selection, exact lookup and saved channel overrides still work. Errors remain distinct from empty results. Scrolling stays inside the results pane with header/footer fixed.
- [ ] 100/125/150% text remains usable; primary actions, Settings, support report and About are keyboard accessible with visible/restored focus and truthful states. Record high-DPI checks and Orca/Narrator/VoiceOver separately without claiming certification.
- [ ] Preview a real support report. Visually verify absence of username, profile/drive/path, executable paths, player arguments, Twitch account/channel/title, OAuth data and raw logs. Preview performs no automatic upload/copy/save. Copy only deliberately after review.
- [ ] Main-interface and tray/status About agree with diagnostics/report: `0.4.0 (<candidate SHA first seven characters>)`, icon/name and the fixed repository link. Repeated activation reuses the dialog; hidden-window activation restores access; closing About leaves playback/monitoring running. macOS app/menu-bar entries invoke the same native AppKit panel.
- [ ] Existing background monitoring remains healthy; test notification activation where practical. Close-to-background preserves playback, restored Watching reconstructs state, and explicit Quit (including tray Quit/Command-Q) cleans up owned playback. Relaunch preserves login/settings. Windows startup/probes/playback produce no unwanted console.

| Native gate | Windows | Linux | macOS |
| --- | --- | --- | --- |
| Exact package install/startup, signing behavior and build identity | NOT TESTED | NOT TESTED | NOT TESTED |
| Existing login and v5 → v6 settings upgrade (at least one required; record unavailable profiles) | NOT TESTED | NOT TESTED | NOT TESTED |
| Manual update check/Refresh/progress and official release-page opening | NOT TESTED | NOT TESTED | NOT TESTED |
| Chatterino discovery/chat, browser fallback and independent lifetime | NOT TESTED | NOT TESTED | NOT TESTED |
| Real player profiles, unchanged active run, Restart/Stop and Default fallback | NOT TESTED | NOT TESTED | NOT TESTED |
| Browsing/language/lookup, channel preferences and scrolling regression | NOT TESTED | NOT TESTED | NOT TESTED |
| Text sizes, keyboard/focus and status feedback | NOT TESTED | NOT TESTED | NOT TESTED |
| Support report privacy preview | NOT TESTED | NOT TESTED | NOT TESTED |
| About, hidden/repeated activation, identity and repository link | NOT TESTED | NOT TESTED | NOT TESTED |
| Background/Watching, explicit Quit and relaunch persistence | NOT TESTED | NOT TESTED | NOT TESTED |
| Notification activation where practical | NOT TESTED | NOT TESTED | NOT TESTED |
| Orca / Narrator / VoiceOver (record gaps) | NOT TESTED | NOT TESTED | NOT TESTED |

This is a blank checklist, not candidate acceptance. Store completed results with the candidate run identity without changing its source commit. Any post-test source/configuration change requires a new candidate.
