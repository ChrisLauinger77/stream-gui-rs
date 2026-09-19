# Maintainer release process

Releases are tag-driven. Pushing an annotated `vMAJOR.MINOR.PATCH` tag starts the native Linux, Windows, and macOS builds and publishes their exact artifacts after every job succeeds. There is no manual release-workflow dispatch.

When preparing version `X.Y.Z`:

1. Confirm `main` is clean and current. Add user-facing changes beneath `## Unreleased` in `CHANGELOG.md` during normal development.
2. Run `npm run release:version -- X.Y.Z`. The command updates `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json`, and moves the accumulated changelog entry under a dated `X.Y.Z` heading.
3. Review the version diff and run the full validation commands from `AGENTS.md`, including native checks relevant to the changes.
4. Commit the prepared release as `chore(release): prepare X.Y.Z`, push `main`, and wait for Desktop checks and CodeQL to pass.
5. Create an annotated tag at that exact commit with `git tag -a vX.Y.Z -m "Stream GUI RS X.Y.Z"`, then push it with `git push origin vX.Y.Z`.
6. The **Release** workflow validates that the tag, application metadata, Cargo metadata, npm metadata, Tauri configuration, and dated changelog heading all identify the same version. It then builds native packages, verifies architectures and checksums, creates a draft GitHub Release, uploads and compares the complete asset set, and publishes the release. A failed platform build leaves no public release.
7. The workflow requests Scoop and Homebrew updates immediately when `PACKAGE_REPOSITORIES_TOKEN` is configured. Both package repositories also check daily as a fallback.
8. Run [the release artifact smoke test](release-smoke-test.md) against the files downloaded from the published release. Record any unsupported or unverified native target honestly.

Do not create lightweight release tags, move a published tag, rebuild assets outside the tag workflow, or publish a release whose workflow failed.

## Package repository notification

Immediate package updates require an Actions secret named `PACKAGE_REPOSITORIES_TOKEN` in `stream-gui-rs`. Use a fine-grained GitHub token limited to `ChrisLauinger77/scoop-bucket` and `ChrisLauinger77/homebrew-cask`, with repository Contents read/write permission so it can send `repository_dispatch` events. If the secret is absent, the release still publishes and each package repository's daily updater discovers the latest release automatically.

## Artifact contract

For version `X.Y.Z`, the workflow publishes:

- `Stream-GUI-RS_X.Y.Z_windows_x86_64-setup.exe`
- `Stream-GUI-RS_X.Y.Z_windows_x86_64.zip`
- `stream-gui-rs.json`
- `Stream-GUI-RS_X.Y.Z_linux_x86_64.AppImage`
- `Stream-GUI-RS_X.Y.Z_linux_amd64.deb`
- `Stream-GUI-RS_X.Y.Z_linux_x86_64.rpm`
- `Stream-GUI-RS_X.Y.Z_macos_universal.dmg`
- one SHA-256 file for each platform group

The Windows manifest hashes the portable ZIP and carries Scoop `checkver` and `autoupdate` metadata. The macOS workflow verifies both `arm64` and `x86_64` executable slices with `lipo`. The Linux workflow removes AppImage copies of Wayland and GLib infrastructure libraries that must match the host desktop. The public Twitch client ID comes only from `TWITCH_CLIENT_ID_BUILD`; releases have no synthetic fallback and use no client secret.

Release and Desktop checks workflows pass `${{ github.sha }}` as `STREAM_GUI_RS_COMMIT` to every build, including the separate debug notification-acceptance packages on all three CI platforms. The build script validates and embeds its first seven hexadecimal characters; it does not derive release identity from a branch name or working-tree state. The existing version synchronization and Twitch client-ID handling are unchanged. See [build metadata](../README.md#building-from-source) for the local/source-archive fallback.

Windows packages remain unsigned. macOS packages use a complete ad-hoc bundle signature, verified before upload, but have no Developer ID signature or notarization. This binds the app identity, Info.plist and resources for native APIs; the linker's temporary executable signature is insufficient. Adding trusted signing requires its own reviewed credential and workflow design; checksums do not replace platform signing.
