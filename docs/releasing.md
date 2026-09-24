# Maintainer release process

Feature work is reviewed and accepted on a branch before merge. The merged commit is the implementation baseline. Follow [testing policy](testing.md) for focused native acceptance; a metadata-only release preparation does not repeat the complete feature matrix.

The existing **Release** workflow has two paths. Manual dispatch on `main` builds a non-publishing candidate from GitHub's captured commit. A later annotated tag promotes that candidate's exact files without rebuilding. No release, tag, package-repository update or publication occurs during candidate creation.

## Prepare the release commit

1. Start from a clean, current `main` containing the accepted feature work. Check the intended version, previous release and merge/PR history. Do not prepare from a dirty tree or unexpected branch.
2. Draft user-facing notes from GitHub's **Generate release notes** for PRs merged since the previous tag. Review, correct and commit the result as `docs/release-notes-X.Y.Z.md`; the workflow currently requires this file and publishes it with `--notes-file`. GitHub generation is an editorial starting point, not a change to the publishing workflow. There is no configured release-note category file yet; useful editorial sections are Features, Fixes, Platform, Security, Documentation and Dependencies. Do not add labels retroactively to old PRs. A major release may add a short handwritten Highlights section. Keep `CHANGELOG.md` concise instead of duplicating the full notes.
3. Run `npm run release:version -- X.Y.Z`. It synchronizes Cargo, npm, both lockfiles and Tauri version metadata and moves the changelog's Unreleased section into a dated heading. The heading uses the preparation date until publication. Review every changed file and run `node scripts/set-version.mjs --check X.Y.Z` before committing.
4. Keep the preparation commit limited to version fields, corresponding lockfile/version metadata, release notes, changelog, release-facing documentation and required generated release metadata. Do not slip in a runtime feature, unrelated fix, dependency upgrade, native integration or packaging architecture change. If one is necessary, review and validate it as a real code change under [testing policy](testing.md), then prepare a new candidate baseline.
5. Commit and push the preparation to `main`. Wait for fresh Desktop checks and CodeQL on that exact commit. Documentation-only changes may be excluded from Desktop checks by its path filter, but a normal version preparation changes non-Markdown version files and therefore runs CI. Resolve any failure before building a candidate.

## Candidate and release gate

6. Dispatch **Release** manually on `main` after required CI passes. Record its first-attempt run ID and captured full SHA. Every platform job verifies that same SHA and frozen dependency lockfiles. A rerun does not create a new acceptable candidate identity; use a fresh dispatch if necessary. Candidates are retained for 30 days and excluded from the weekly one-day artifact cleanup. Expired candidates need a rebuild and renewed package checks.
7. Download the three artifact groups and audit the complete set against [the artifact contract](#artifact-contract). Verify each SHA-256 manifest and the Scoop ZIP hash, version and seven-character build identity in package metadata/About, Linux launcher/protocol metadata where applicable, Windows installer/portable identity, macOS minimum version, universal `arm64` and `x86_64` slices, and the macOS complete ad-hoc bundle signature. Check the actual signing/notarization state; never describe unsigned packages as trusted-signed. Preserve a record mapping each audited file to the run and SHA.
8. Perform the small, release-specific [exact-package smoke](testing.md#exact-package-smoke) on Linux, Windows and macOS, including at least one real previous-release profile upgrade. If the preparation was metadata-only and the merged branch already passed focused native feature acceptance, reuse that evidence. Repeat full native acceptance only under the triggers in [testing policy](testing.md#release-preparation). Record any untested format/architecture or accepted limitation explicitly; automated package inspection does not prove native execution.
9. If the source, configuration, dependencies or packaging must change, commit the reviewed fix, rerun required CI, dispatch a **new complete candidate**, and repeat the affected audit and acceptance. Do not combine candidate revisions, replace tested files or retag a different commit.

## Tag, publish and verify

10. After the release gate passes, create an annotated `vMAJOR.MINOR.PATCH` tag pointing to the exact reviewed release-preparation commit, not a later unvalidated commit. Its annotation must include a blank line followed by `Candidate-Run: <approved-run-id>`. The candidate must be a successful first-attempt manual run on `main` for that same full SHA. Lightweight tags, missing trailers, mismatched branches/revisions, failed runs and rerun candidates are rejected. Pushing this tag triggers publication from the tagged commit.
11. The tag workflow rechecks version sources, candidate provenance, the full checksummed set and the committed notes. It creates a draft, uploads the selected candidate bytes, downloads and byte-compares them, then publishes. Platform build jobs are skipped during promotion. Do not move tags, silently replace artifacts or publish manually around a failed promotion; investigate and resume only after the failed gate is resolved.
12. Verify the public release tag and latest-release page, all expected assets, downloaded checksums and equality with the approved candidate. Check Scoop and Homebrew installation/update after publication. Record package-repository lag separately; the release itself does not depend on immediate package-repository updates.

`PACKAGE_REPOSITORIES_TOKEN`, when configured, permits the workflow to request immediate Scoop and Homebrew repository updates. It must be a fine-grained token limited to `ChrisLauinger77/scoop-bucket` and `ChrisLauinger77/homebrew-cask` with the repository permission required for `repository_dispatch`. Without it, their daily updater jobs are the existing fallback. No new automation or automatic tagging is implied here.

## Artifact contract

For version `X.Y.Z`, the candidate and published release contain:

- `Stream-GUI-RS_X.Y.Z_windows_x86_64-setup.exe`
- `Stream-GUI-RS_X.Y.Z_windows_x86_64.zip`
- `stream-gui-rs.json`
- `Stream-GUI-RS_X.Y.Z_linux_x86_64.AppImage`
- `Stream-GUI-RS_X.Y.Z_linux_amd64.deb`
- `Stream-GUI-RS_X.Y.Z_linux_x86_64.rpm`
- `Stream-GUI-RS_X.Y.Z_macos_universal.dmg`
- one SHA-256 file for each platform group

The Windows manifest hashes the portable ZIP and carries Scoop `checkver` and `autoupdate` metadata. The macOS job verifies both executable slices with `lipo`, macOS 11.0 minimum, bundle version and identity, then strictly verifies its complete ad-hoc signature before upload. Linux verifies the package architectures and removes incompatible AppImage copies of Wayland/GLib host infrastructure libraries. The public Twitch client ID comes only from `TWITCH_CLIENT_ID_BUILD`; release packaging has no synthetic fallback and uses no client secret.

Release and Desktop checks pass `${{ github.sha }}` as `STREAM_GUI_RS_COMMIT` to each build. The build script validates and embeds its first seven hexadecimal characters; branch names and working-tree state do not supply release identity. See [building from source](../README.md#building-from-source) for local/source-archive fallback. Windows packages remain unsigned. macOS packages have an ad-hoc bundle signature but no Developer ID signature or notarization. Checksums establish file integrity, not trusted platform signing.
