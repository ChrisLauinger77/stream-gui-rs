# Maintainer release process

The **Release** workflow has two paths: manual dispatch builds a non-publishing candidate from the exact `main` revision captured by GitHub; a later annotated tag promotes the selected candidate's existing artifacts without rebuilding. A manual candidate run never creates a GitHub Release or requests package-repository updates.

## Prepare and test a candidate

1. Confirm `main` is clean and current. Add user-facing changes beneath `## Unreleased` in `CHANGELOG.md`.
2. Run `npm run release:version -- X.Y.Z`. This synchronizes Cargo, npm, both lockfiles and Tauri metadata, and moves the changelog into a dated version heading. The date is the preparation date until publication.
3. Prepare `docs/release-notes-X.Y.Z.md`, inspect the version diff, and run the full validation in `AGENTS.md`.
4. Commit and push the preparation to `main`. Wait for Desktop checks and CodeQL, then dispatch **Release** on `main`. Record the captured full SHA and workflow run ID; every platform verifies that same SHA.
5. Download the three platform artifact archives and verify their SHA-256 files. Complete the [native smoke checklist](release-smoke-test.md) on those exact packages, including a real upgrade with existing credentials on at least one platform. Compilation is not native acceptance.
6. If a source/configuration fix is needed, commit it and dispatch a new complete candidate. Restart affected acceptance. Never combine revisions, replace tested files, or rerun a candidate run: a fresh dispatch receives a new identity. Candidates are retained for 30 days and excluded from the weekly one-day cleanup. Expired candidates must be rebuilt and retested.

## Publication after explicit approval

Only after the exact candidate passes all release gates, select its successful first-attempt run in an annotated `vMAJOR.MINOR.PATCH` tag. The annotation must contain a `Candidate-Run: <approved-run-id>` Git trailer after a blank line. The tag must point to the approved candidate commit. Lightweight tags, missing trailers, another revision/workflow/branch, failed runs and rerun candidates are rejected.

Pushing the approved annotated tag triggers promotion. The workflow validates all version sources, the selected run and complete checksummed asset set. It uses the committed release notes, creates a draft release, uploads the selected candidate bytes, downloads and compares them, then publishes. No package rebuild occurs during promotion. Do not move tags or substitute files. A failed promotion must be investigated before resuming its draft; do not publish manually around a failed check.

The workflow requests Scoop and Homebrew updates when `PACKAGE_REPOSITORIES_TOKEN` is configured. Both package repositories also check daily as a fallback. Check their installation after publication and verify the public release page and downloaded checksums.

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
