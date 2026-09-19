# Release candidate smoke test

Use only the files downloaded from a single successful **Release candidate artifacts** workflow run. Record the workflow URL, exact 40-character commit SHA, artifact names, tester, date, OS version, Streamlink version, and player version. Do not rebuild or substitute files after testing.

## Integrity and common checks

- [ ] The workflow SHA is the intended release candidate commit.
- [ ] Every downloaded filename identifies version 0.1.0, platform, and architecture.
- [ ] `SHA256SUMS_<platform>_<architecture>.txt` verifies every distributable in its artifact archive.
- [ ] Install or open the package without setting `TWITCH_CLIENT_ID` or `TWITCH_CLIENT_ID_BUILD`.
- [ ] The application identifies itself as Stream GUI RS 0.1.0 where version metadata is shown.
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

Artifact: `Stream-GUI-RS_0.1.0_windows_x86_64-setup.exe`

- [ ] Note the expected unsigned-publisher/SmartScreen warning; no unexpected publisher identity appears.
- [ ] Programs and Features shows Stream GUI RS, version 0.1.0, and publisher ChrisLauinger77.
- [ ] Normal application startup displays no console window.
- [ ] Streamlink probes and playback start without helper console flashes.
- [ ] Uninstall succeeds.

## Linux x86_64

Artifacts: `Stream-GUI-RS_0.1.0_linux_x86_64.AppImage` and `Stream-GUI-RS_0.1.0_linux_x86_64.deb`

Test both formats separately on supported desktop sessions.

- [ ] AppImage is executable and starts from the desktop environment.
- [ ] Debian package installs with declared dependencies, creates the expected launcher/icon, and starts from that launcher.
- [ ] Authentication persists through the session's Secret Service provider.
- [ ] XDG configuration is written under the current user, with no credentials in settings files.
- [ ] Browser sign-in and chat opening do not show a remote-screen permission prompt caused by the application itself.
- [ ] Debian package uninstall succeeds.

## macOS Apple Silicon arm64

Artifact: `Stream-GUI-RS_0.1.0_macos_arm64.dmg`

- [ ] The disk image mounts and contains `Stream GUI RS.app`.
- [ ] Record the expected Gatekeeper/quarantine behavior for the app without Developer ID signing or notarization.
- [ ] After the tester explicitly allows it, the app starts without terminal environment variables.
- [ ] Finder reports version 0.1.0 and the app runs natively as arm64.
- [ ] Keychain persistence, browser sign-in, chat opening, Streamlink discovery, playback, and cleanup pass.
- [ ] Dragging the app to Applications and removing it both work as expected.
