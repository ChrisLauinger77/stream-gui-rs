# Maintainer release process

Version 0.1.0 uses a candidate-first process. The manual workflow packages an exact commit but never creates a tag or GitHub Release.

1. Confirm `main` is clean, current, and passes the **Desktop checks** workflow on Linux, Windows, and macOS.
2. Confirm `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json` agree on the intended version.
3. Review the current tree and reachable history for credentials and private material. Confirm `TWITCH_CLIENT_ID_BUILD` exists in repository Actions secrets without printing its value.
4. Run the full local validation commands from `AGENTS.md`, including the isolated Linux browser test when working on Linux.
5. Commit the candidate and push it to `main`. Record the full 40-character commit SHA.
6. Manually run **Release candidate artifacts**, entering that exact SHA. The workflow fails if the input is not a full SHA, the checkout differs, the registered Twitch client ID is unavailable, or the native runner architecture differs from the release matrix.
7. Download all three artifact archives from that one workflow run. Verify their included SHA-256 files, preserve the archives, and complete [the exact-artifact smoke test](release-smoke-test.md) on each native platform.
8. If a candidate fails, fix the issue, create a new commit, and repeat from step 5. Do not reuse results from an older candidate.
9. After every exact artifact passes, replace `Unreleased` in `CHANGELOG.md` with the publication date and update draft release notes if observations changed. This documentation-only commit changes the release source, so run the workflow again and recheck its checksums and package metadata; repeat runtime smoke tests when the change could affect packaging or runtime behavior.
10. Create annotated tag `v0.1.0` at the approved exact commit and push the tag.
11. Create a GitHub Release from `v0.1.0`, using `docs/release-notes-0.1.0.md`. Upload the already approved distributables and checksum files extracted from the preserved workflow artifacts. Do not rebuild them.
12. Download the published assets once, verify their checksums again, and confirm the release page identifies every platform and architecture.
13. Publish the package-manager definitions only after the assets are available at their final release URLs:
    - copy the exact generated `stream-gui-rs.json` into `bucket/stream-gui-rs.json` in `ChrisLauinger77/scoop-bucket`, add `stream-gui-rs` to its update workflow and README, then validate installation from that bucket;
    - add `Casks/stream-gui-rs.rb` to `ChrisLauinger77/homebrew-cask`, using the published universal DMG URL and SHA-256, add it to `config/casks.json`, the workflow's validation list, and the README, then validate installation from that tap.
14. Dispatch or run each package repository's existing updater once. Confirm future versions are detected from the latest `v<version>` GitHub Release and that each repository's validation succeeds.

## Artifact contract

The workflow creates these archives for 30 days:

- `Stream-GUI-RS_0.1.0_windows_x86_64`
- `Stream-GUI-RS_0.1.0_linux_x86_64`
- `Stream-GUI-RS_0.1.0_macos_universal`

Each archive contains only its native distributable files plus one unambiguous checksum file. Windows contains the NSIS installer, a portable ZIP, and a Scoop manifest whose SHA-256 hash points to that ZIP; the manifest also carries the bucket's `checkver` and `autoupdate` metadata. Linux contains an AppImage, an `amd64.deb`, and an `x86_64.rpm`. macOS contains a universal arm64/x86_64 app in a disk image; the workflow verifies both executable slices with `lipo`. The workflow removes AppImage copies of Wayland and GLib infrastructure libraries that must match the host desktop before generating its checksum. The public Twitch client ID comes only from the repository secret. There is no synthetic release fallback and no client secret.

Version 0.1.0 is unsigned on Windows and has no Developer ID signature or notarization on macOS. Apple Silicon bundles may carry an ad-hoc signature. Adding trusted signing later requires its own reviewed credential and workflow design; checksums do not replace platform code signing.
