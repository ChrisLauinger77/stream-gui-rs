# Release artifact smoke test

Use only the files downloaded from one published tag-driven release. Record the release URL, tag, exact 40-character commit SHA, artifact names, tester, date, OS version, Streamlink version, and player version. Do not rebuild or substitute files after testing.

## Integrity and common checks

- [ ] The release tag points to the intended release commit.
- [ ] Every downloaded filename identifies the release version, platform, and architecture.
- [ ] `SHA256SUMS_<platform>_<architecture>.txt` verifies every distributable in its artifact archive.
- [ ] Install or open the package without setting `TWITCH_CLIENT_ID` or `TWITCH_CLIENT_ID_BUILD`.
- [ ] The application identifies itself as Stream GUI RS X.Y.Z where version metadata is shown.
- [ ] Connect to Twitch through the browser Device Code flow.
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

Test all three formats separately on matching supported desktop sessions.

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
- [ ] `lipo -archs` reports both `arm64` and `x86_64` for the executable under `Stream GUI RS.app/Contents/MacOS`.
- [ ] Record the expected Gatekeeper/quarantine behavior for the app without Developer ID signing or notarization.
- [ ] After the tester explicitly allows it, the app starts without terminal environment variables.
- [ ] Finder reports version X.Y.Z and the same exact disk image runs natively on Apple Silicon.
- [ ] The same exact disk image runs natively on an Intel Mac.
- [ ] Keychain persistence, browser sign-in, chat opening, Streamlink discovery, playback, and cleanup pass.
- [ ] Dragging the app to Applications and removing it both work as expected.
- [ ] After publication, `brew tap ChrisLauinger77/cask` and `brew install --cask stream-gui-rs` install the same universal DMG from the release, and `brew uninstall --cask stream-gui-rs` succeeds.
