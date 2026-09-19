# Development-only native notification acceptance

The `notification-acceptance` Cargo feature exposes **Settings → Developer tools → Native notification acceptance**. It is off by default and rejected by the build script in release/non-debug profiles, even if debug assertions are enabled. Release builds do not expose its command, permission or UI controls. No version change or release is needed.

Each **Send test notification** queues fixed, clearly labeled test content through the same production adapter as followed-live notifications. Clicking uses the real OS callback, opaque activation lookup, window restore/focus and acknowledged desktop action. The destination is **TEST notification · Synthetic channel**, a local view with no channel request, playback or chat action. Twitch sign-in, HTTP and monitoring are not required. Sending or clearing does not change settings, monitor baselines or deduplication.

## CI acceptance packages

After a successful **Desktop checks** run for a push to `main`, download the matching `stream-gui-rs-notification-acceptance-<OS>-<architecture>-<commit>` artifact: Linux `.deb`, Windows NSIS installer, or macOS `.dmg`. These debug packages include the test action and require the repository's registered public client ID during packaging. Pull requests run checks without uploading packages; the normal packaged-mode build is still checked without the acceptance feature.

The macOS CI artifact targets the runner's native architecture, recorded in its name; it is not the universal release image. Install the matching package and use the manual checks below. CI creation does not prove notification behavior on the target desktop. Artifacts request 14-day retention, but the repository's weekly cleanup can remove them once they are more than 24 hours old; download the intended build promptly. These artifacts are development builds, not GitHub Releases.

## Build and run

Use the repository's normal native prerequisites and `npm ci`. The synthetic public ID below is for this signed-out acceptance build only; it cannot authenticate and must never be used for a release. Stay signed out and leave monitoring disabled for this test. Quit other Stream GUI RS instances before starting, especially on Windows where one instance owns notification identity.

Linux development:

```sh
TWITCH_CLIENT_ID=ciCompileOnlyPublicClient123 npm run tauri dev -- --features notification-acceptance
```

Windows PowerShell, installed debug build:

```powershell
$env:TWITCH_CLIENT_ID_BUILD = "ciCompileOnlyPublicClient123"
npm run tauri build -- --debug --bundles nsis --features custom-protocol,notification-acceptance --ci
```

Install the generated installer under `src-tauri/target/debug/bundle/nsis/` and launch its Start-menu shortcut. A bare/portable executable is insufficient to verify installed notification identity. The test build uses the normal app identity; install the normal app again after testing.

macOS, bundled debug build:

```sh
TWITCH_CLIENT_ID_BUILD=ciCompileOnlyPublicClient123 npm run tauri build -- --debug --bundles app --features custom-protocol,notification-acceptance --ci
```

Open `src-tauri/target/debug/bundle/macos/Stream GUI RS.app` as an application bundle. In Developer tools, use **Allow desktop notifications** if permission is not requested. If denied, re-enable in System Settings. A bare executable cannot exercise the macOS notification adapter. Test separately on the supported native architectures.

An installed Linux debug package can be built with the same command using `--bundles deb` (or the package format appropriate to the host). Linux notification history/action support varies by desktop; record the notification server and whether it advertises actions.

## Windows retained notification acceptance

1. Open Developer tools and select **Send test notification**. “Queued” confirms submission to the worker, not delivery by the OS. Confirm the visible notification has the app identity and starts with **TEST notification**.
2. Let the banner time out naturally. Do not dismiss it. Confirm it remains in Notification Center.
3. Minimize Stream GUI RS, keep its process running, and click that retained entry within 15 minutes. Confirm the app restores, focuses and leaves Developer tools for **TEST notification · Synthetic channel**. No stream should launch.
4. Confirm the consumed entry disappears and cannot navigate twice. Return to Developer tools for another trial, this time clicking the visible banner.
5. Send another test and explicitly dismiss it; verify removal. Send another, let it remain for 15 minutes (also test with suspend/resume), and verify expiration removes it and prevents activation.
6. Send a retained test, then select **Clear test notifications**. Verify its history entry is removed and a delayed click cannot open the target. Send again to verify cleanup does not disable later tests.
7. To exercise the existing bound, send up to 32 unconsumed tests within the retention interval; the 33rd cannot add another retained record. After clearing/dismissing/expiry, delivery resumes. Native delivery failures use the existing notification-error status.
8. Quit with retained tests and verify history cleanup. Clicking an old entry must not cold-launch the app. Normal Quit also stops owned playback, so use a session without streams for this focused test.

The production policy remains 32 retained native records for 15 minutes, with cleanup on activation, dismissal, cancellation, expiry and shutdown. Test records share that capacity with real notifications. **Clear test notifications** cancels only the test generation; it exercises native cancellation cleanup without pausing/reconfiguring the monitor. Test records deliberately survive Twitch logout and monitor Pause because they have no real auth session. Use real followed-stream notifications for the separate [account/monitor cancellation checks](release-smoke-test.md#windows-notification-center-acceptance).

On macOS and Linux, also verify permission/denial, real delivery, click restore/focus, the synthetic view and test cleanup, accounting for each desktop's history policy. Record OS/version, artifact commit, build command, installation method and observed results. Automated fixtures do not establish acceptance of Windows Notification Center, macOS Notification Center or the user's Linux desktop server.
