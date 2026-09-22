# Phase 7 macOS and Windows native acceptance

Current status: **macOS PASS**, reported by the user on 2026-09-22 with Homebrew
Chatterino; **Windows pending**. See the [validation record](phase-7-validation.md#native-observations-and-remaining-acceptance)
for the reported scope and build-identity details. The checklist below remains
the reference for both platforms.

Use the installers from the Phase 7 pull request's successful **Desktop checks**
run. These are development acceptance packages, version **0.3.0**, with the
registered public Twitch client ID and optional notification test controls.
They are not a new release. See [package details](notification-acceptance.md#ci-acceptance-packages).

Record OS/version, workflow run URL, artifact name, About version/commit, installed
player(s), and Chatterino installation method. The About commit is the tested PR
merge commit embedded by CI. Keep the existing login and settings when installing.
Download promptly because artifact cleanup can remove older acceptance builds.

## Shared focused checks

1. Start the installed app: existing Twitch login and settings are restored;
   About shows the expected version/commit.
2. Settings → Updates: **Check for updates** shows progress and then the correct
   result. **Refresh update check** replaces the old status while pending; allow
   the documented one-minute cooldown for another network request. **View release**
   opens the official GitHub release in the default browser without changing the
   accepted update status. The button also appears when already current.
3. Select profile A and start a real stream. Select profile B with a clearly
   different player setting while playback is running: the existing stream stays
   unchanged. **Restart** applies B; **Stop** cleans up that session. Record both
   profiles' settings and the real player used. Confirm another running session
   remains independent.
4. If Chatterino is installed, verify discovery and opening the requested channel
   with Chatterino stopped and already running. Record whether it opens a new
   window/process; reuse is not promised. Exercise automatic chat and the explicit
   browser fallback. Stop playback and Quit must leave Chatterino running.
   If unavailable, record Chatterino as untested and verify browser chat.
5. Quick regression: scrolling remains inside the results pane; header/footer
   stay fixed. About and its repository link work. Verify normal minimize,
   background close/restore and tray behavior where supported. Observe one native
   notification and click it to restore the app; the package's **Developer tools →
   Send test notification** can be used for this quick check.
6. Quit with playback running: the app and its owned playback processes exit.
   Restart the app and confirm the saved profile/settings and login remain.

## macOS specifics

- Install the matching-architecture `.dmg` app bundle; launch it as an installed
  application. Check Chatterino app-bundle/GUI discovery if installed.
- Native About's **Github Repository** link opens the correct repository.
- Check notification permission/delivery, background restore and **Command-Q**
  cleanup. The tray/menu-bar item depends on the desktop integration.

## Windows specifics

- Use the NSIS installer and installed Start-menu shortcut for notification
  identity. Check Chatterino `.exe` discovery, including a path with spaces if
  available.
- No console windows flash during app, player, Chatterino or browser launches.
- Check notification delivery/click restore, tray/background behavior and explicit
  **Quit** cleanup of owned playback. Chatterino remains independent.

Report PASS/FAIL/NOT TESTED for each item with the exact artifact identity.
Hosted compilation, packaging and signature/subsystem checks do not replace these
native observations. Do not repeat the full historical release matrix for this
focused Phase 7 acceptance pass.
