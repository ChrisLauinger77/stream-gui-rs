# 0.4.0 release readiness

Preparation date: 2026-09-22. This record describes the release source and gates;
it does not claim future candidate-package acceptance or publication.

## Baseline and scope

Preparation started from clean `main`, matching `origin/main` at
`cc74e13f0ebe19f374e39aa6815bfa0b0064dbdf`, the merge of Phase 7 PR #20.
The released baseline is v0.3.0. This preparation synchronizes version metadata,
adds release notes/changelog and updates the release-facing documentation and
smoke checklist. One existing semantic-version fixture gains the explicit
0.4.0-versus-0.4.0 case. Application behavior and dependency versions are unchanged.
No unrelated dependency PR, Phase 8 feature or new package-manager automation is
included.

The [release notes](release-notes-0.4.0.md) cover manual release awareness,
independent Chatterino (including Linux Flatpak), player profiles and schema 6.
The [Phase 7 validation record](phase-7-validation.md) separately records Linux,
macOS/Homebrew Chatterino and Windows/WinGet Chatterino acceptance. Those reports
do not accept newly built release-candidate packages.

## Migration and build identity

The released-schema-five regression checks every old settings field and sparse
channel override, including Unicode/space-containing executable paths, literal
arguments, quality, automatic chat, theme, monitoring/notifications/background,
language, low latency and text size. Opening does not rewrite the file; saving
writes schema 6. Defaults are Browser chat, no Chatterino override, empty profiles
and no selected profile. Existing schema 1–4 migrations remain covered. Update
awareness has no persistent settings or startup HTTP.

Credentials remain in the existing native store, outside the settings document.
The app identifier and credential-store identity are unchanged; synthetic settings
fixtures cannot prove a real stored-login upgrade. The maintainer reported that
only Phase 7 test builds remain; no untouched released v0.3.0 profile is available.
Preserve those profiles and record exact-candidate stored-login/settings restore
separately from the unavailable released-schema upgrade observation.

Cargo/npm/Tauri and both lockfiles are synchronized by `release:version`.
`build_info` remains the shared source for About, diagnostics and support reports.
Release/CI builds provide the full captured SHA through `STREAM_GUI_RS_COMMIT`;
the build script validates it and embeds the first seven characters. Candidate
checks must report **0.4.0** and the candidate's actual short SHA. The update
fixtures cover equal 0.4.0, local 0.4.0 versus remote 0.3.0, and local 0.3.0 versus
remote 0.4.0, without fake public releases.

## Local and hosted validation

Full local validation passed on the version-bumped source on 2026-09-22, with
Node 24.21.0 and Rust 1.95.0 on the graphical Linux development host:

| Check | Result |
| --- | --- |
| Generated bindings | PASS; byte-for-byte unchanged |
| TypeScript and production frontend build | PASS |
| Frontend tests | 202 PASS across 3 files |
| Backend unit tests | 170 PASS |
| Native process lifecycle tests | 42 PASS |
| Build identity / client-ID guards | 2 / 1 PASS |
| Candidate-promotion guards | PASS |
| Desktop-only tests | 10 PASS |
| Desktop-enabled library with notification acceptance | 184 PASS, including backend units |
| Rust format / all-target check / strict Clippy | PASS |
| Linux browser / startup checks | 2 / 1 PASS |
| Isolated Linux graphical scenarios | 4 PASS in 53.54 seconds |
| Non-publishing Tauri debug/custom-protocol build | PASS; synthetic compile-only public ID |
| Version consistency / documentation links / diff whitespace | PASS |

The graphical suite used synthetic settings, an isolated D-Bus notification
fixture and fake playback. It did not reproduce the historical allocator
diagnostic. These checks do not accept installed release packages or real stored
credentials. Hosted Desktop checks and CodeQL must subsequently pass on the
preparation commit; earlier Phase 7 checks remain historical evidence.

## Artifact and promotion contract

The release workflow and packaging policy are unchanged:

| Platform | Candidate contents |
| --- | --- |
| Windows x86_64 | NSIS installer, portable ZIP, generated Scoop manifest and platform SHA-256 file |
| Linux x86_64 | AppImage, amd64 DEB, x86_64 RPM and platform SHA-256 file |
| macOS universal | DMG containing arm64 and x86_64 executable slices, and platform SHA-256 file |

macOS remains universal, minimum **11.0**, with a complete ad-hoc bundle signature
and no Developer ID/notarization. The workflow checks both slices with `lipo`,
version/identifier/minimum metadata and the bundle signature. Windows remains
unsigned and must pass its GUI-subsystem check. The existing Linux AppImage
host-library compatibility repair remains in place.

A first-attempt manual Release run captures one exact `main` SHA and requires the
registered public Twitch client ID; there is no synthetic release fallback. Its
three archives are retained for 30 days and excluded from weekly cleanup. The
complete ten-file set and all platform checksums are verified. Record the run URL,
SHA, artifact identities, checksums and native results together outside this
source snapshot rather than changing it to insert its own commit hash.

After the exact candidate passes the [native smoke checklist](release-smoke-test.md),
the annotated `v0.4.0` tag must point to that commit and include
`Candidate-Run: <accepted-run-id>` after a blank line. Promotion validates the run,
uploads its existing files, downloads and compares them, and only then publishes.
It does not rebuild. The user's release request authorizes publication after the
documented gates; missing native evidence must be resolved or explicitly accepted
before tagging.

## Native gates and remaining evidence

At preparation, exact 0.4.0 candidate package checks are **NOT TESTED** on Linux,
Windows and macOS. Complete installation/build identity, existing login/settings
restore, real profiles/Restart/Stop, updates/release opening, available Chatterino,
browser fallback and focused UI/background/notification/Quit checks. Record
untested Linux formats, native CPU execution, high-DPI and screen readers honestly.
The missing untouched v0.3.0 profile remains an explicit upgrade-evidence limitation.

The historical allocator diagnostic remains recorded unchanged in Phase 7
validation: one occurrence, source/PID unknown, twelve app repetitions and twenty
desktop-service probes without recurrence, and subsequent passing graphical runs.
It is neither a harmlessness claim nor a current confirmed defect. If it recurs
during candidate validation, investigate before tagging; no speculative code fix
is part of preparation. It is not included in the user-facing release notes.

Existing Scoop/Homebrew update notifications and daily fallback remain in place.
The initial WinGet review is a separate follow-up and does not block this release;
no new WinGet automation is added.

## Maintainer-approved release-procedure exception — 2026-09-22

The maintainer explicitly accepted a release-procedure exception for **v0.4.0**
on the basis that its runtime code is the already native-validated Phase 7 code.
Relative to the merged Phase 7 baseline
`cc74e13f0ebe19f374e39aa6815bfa0b0064dbdf`, preparation commit
`05729fe5c090cc8d5589019c7b3b44fe92ec0b2e` changes version/release metadata and
documentation, plus one non-runtime test case for equal 0.4.0 versions. It changes
no application behavior, dependencies, packaging policy or credential identity.

This exception accepts the existing Linux, macOS and Windows Phase 7 native
results without repeating acceptance on the exact 0.4.0 packages. Those package
checks, including fresh About observations and stored-login/settings restoration,
are **not newly tested**. The untouched released v0.3.0/schema-5 upgrade check is
also **unavailable** because only Phase 7 test profiles remain. Automated schema-5
migration coverage passed; it is not recorded as a real stored-credential upgrade.
The limitations in the earlier native record remain unchanged. This is a specific
maintainer acceptance decision for this release, not a general relaxation of
[the release procedure](releasing.md).

The accepted candidate is first-attempt
[Release run 35756564645](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35756564645),
built from the exact preparation commit above. Full local validation passed;
[Desktop checks](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35754721200)
passed on Linux, macOS and Windows, and
[CodeQL](https://github.com/ChrisLauinger77/stream-gui-rs/actions/runs/35754720976)
passed on that same commit. All three candidate builds and the complete artifact
check passed. Independent inspection verified all ten files and their checksums,
Windows packaging, Linux package metadata/AppImage host-library exclusions, and
the actual macOS DMG's Intel/Apple Silicon slices and minimum version 11.0.
The macOS runner also verified the complete ad-hoc bundle signature.

The exception authorizes promotion of that candidate's existing bytes. The
annotated `v0.4.0` tag must retain the candidate commit and
`Candidate-Run: 35756564645`; this documentation-only acceptance record is a
separate follow-up and does not change or rebuild the candidate. No Phase 8 work
is included.
