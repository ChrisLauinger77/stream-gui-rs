# Phase 2 validation — Twitch browsing

Validated on macOS on 2026-09-18, after the Phase 1 cleanup and project rename. Phase 3 has not started. No dependencies were added.

## Implemented scope

The desktop shell provides Following (live and all followed channels), popular Live streams, categories and category streams, separate channel/category search, and channel details. Shared previews show channel/title/category/viewers/language; details include description, live/offline/unknown state and stream start time. Images have fixed dimensions, placeholders and failure handling. Native keyboard controls, bounded Back navigation, adaptive grids and system/dark/light themes are included. Playback remains the unchanged Phase 0 prototype under Settings → Developer tools.

The backend adds eight explicitly allowed browsing commands, intentional generated DTOs, per-query auth-session binding, bounded metadata enrichment and an eight-operation concurrency limit. No arbitrary Helix command, token DTO, frontend network client, second cache or follow mutation was added. See [architecture](architecture.md) for the exact contracts.

## Automated results

All checks below passed:

| Check | Result |
| --- | --- |
| Rust domain/HTTP/auth/query tests | 87 passed |
| Native fake-Streamlink process tests | 12 passed |
| **Total Rust tests** | **99 passed** |
| Frontend behavior tests | **28 passed** (7 preserved prototype, 21 new browsing/shell) |
| Rust formatting | Passed |
| All-target desktop Rust check | Passed |
| Strict all-target Clippy (`-D warnings`) | Passed |
| Frontend TypeScript and Vite production build | Passed |
| Native macOS Tauri debug `.app` build | Passed |
| Diff whitespace check | Passed |

Commands used (offline Cargo resolution uses the existing locked local dependencies):

```sh
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo test --offline --locked --manifest-path src-tauri/Cargo.toml --no-default-features --features test-support
cargo check --offline --locked --manifest-path src-tauri/Cargo.toml --all-targets --features test-support
cargo clippy --offline --locked --manifest-path src-tauri/Cargo.toml --all-targets --features test-support -- -D warnings
npm run tauri build -- --debug --bundles app --ci --config '{"bundle":{"active":true}}'
git diff --check
```

The temporary bundle override verifies `src-tauri/target/debug/bundle/macos/Stream GUI RS.app`; it does not enable release packaging, signing or publishing in the project configuration.

The twelve added Rust tests cover distinct stream/broadcaster IDs, cursor encoding, backend cache hits/refresh/invalidation, followed-channel batching and missing metadata, reuse of fresh followed-live matches, partial-page omissions, online/offline/unavailable distinctions, category selection/pagination, trimmed/encoded search, logout during enrichment, terminal authorization errors, permitted/sized image URLs, and browse capacity release/shutdown. Existing Phase 0 process and Phase 1 authentication/rate/cache contracts remain green.

New frontend tests cover signed-out/device-code/authenticated views, navigation without global account reload, Following and Live loading/success/empty/error/retry, categories and category navigation, Back focus/snapshot restoration, cursor propagation/deduplication/double dispatch, repeated-cursor termination and the ten-page limit/reset, search debounce/type isolation/late response suppression, online/offline/unknown details, immediate logout cleanup, same-account new-session isolation, auth loss, native focusable controls/search form submission, StrictMode initial-query deduplication and failed images. All fixtures are synthetic; no real Twitch credentials are needed.

## Interactive verification performed

A temporary local fixture preview exercised the actual application components with synthetic channel/account data. The preview file and server were removed after verification. Checked:

- Desktop layout at 1120 × 800 and minimum native size 620 × 600; the initial narrower browser viewport also adapted without a horizontal layout explosion.
- Dark/system rendering and explicit light appearance.
- Enter activation on a stream opens online channel details.
- Back restores focus to that stream and labels retained results.
- Enter activation on a category opens its stream view.
- Keyboard search submission, result activation and offline channel details.
- Sign-out removes browsing content and shows the clean sign-in screen.

This is fixture-based interface verification, not a live Twitch smoke test or proof of image delivery from Twitch's CDN. The native `.app` was built, but the new authenticated browsing flow has not yet been exercised against Twitch inside that bundle.

## Final scope/security review

Reviewed the full Phase 2 changes against the preceding rename commit (`30fbad0`), including newly added files:

- Twitch OAuth/Helix HTTP remains Rust-owned; frontend source has no direct Twitch fetch/XHR client.
- Generated DTOs expose no access token, refresh token, client secret or private device secret. The public session marker only selects a Rust-validated auth generation. Device authorization exposes only the information required for the user to sign in.
- New error presentation uses fixed messages by code. No new logging of provider bodies, query credentials or secrets was introduced.
- Tauri commands remain explicit and local-window-only; CSP still blocks direct third-party API connections and permits only Twitch CDN images.
- Backend cache ownership and bounds are preserved. React retains at most twelve snapshots, each capped at ten pages/300 items, with no TTL or persisted account data.
- Search generations, component unmounts, public session keys and Rust lease cancellation prevent stale/account-crossing response reuse. Eight permits bound unfinished queries.
- No browsing playback controls, new player configuration, background monitoring, chat, follow mutations, teams UI or Phase 3 code was introduced.

## Live Twitch smoke checklist — still required

Use a local public client-ID configuration following [authentication setup](authentication.md). Do not record account names, credentials, authorization codes, tokens or account screenshots in the repository.

- [ ] Authenticate through Device Code flow; check account identity in the shell.
- [ ] Load followed live streams; check display names, titles, category, viewers and language.
- [ ] Load all followed channels, including offline channels and follow dates.
- [ ] Load popular streams.
- [ ] Browse top categories and open a category's live streams.
- [ ] Load another page; confirm earlier results remain, duplicates do not accumulate and Refresh resets the view.
- [ ] Search for a known channel, then edit the query quickly; old results must not replace the new query.
- [ ] Search for a known category and paginate results when a cursor exists.
- [ ] Open an online channel and inspect title, viewers, language and start time.
- [ ] Open an offline channel; compare with a simulated network error, which must show unavailable rather than offline.
- [ ] Verify real preview/profile/category images, sizes, placeholders and failed-image behavior.
- [ ] Use Tab/Enter throughout, including Search and Back focus restoration.
- [ ] Restart the native app and verify authentication persistence and fresh browsing data.
- [ ] Log out during a request; confirm account and browsing data disappear.
- [ ] Re-login (also with another account if available); confirm old view snapshots are absent.
- [ ] Repeat core interactions with light/system appearance and at the minimum window size.

Earlier real authentication/account/restart verification predates the identity rename and Phase 2. It does not substitute for this checklist; the renamed app uses a separate credential namespace and requires a fresh login.

## Platform gaps and intentional limitations

Linux, Windows and macOS CI remain configured in `.github/workflows/ci.yml`, including the new tests. This checkout has no Git remote, so no hosted run was triggered or claimed. Native Linux/WebKitGTK, Windows/WebView2, OS credential storage on those platforms and assistive-technology testing remain unverified locally. No Chromium-only browser API was introduced; native selects/buttons, CSS grids/aspect ratios and ordinary React/DOM behavior are used.

Browsing refresh is explicit. Navigation snapshots are labeled previous results; there is no followed-stream monitor or automatic stale-cache fallback. Load more stops at ten pages/300 items. Search supersedes old results but does not send a separate per-request cancellation IPC; unfinished work remains under the existing HTTP/rate bounds and the new concurrency cap. Cross-view reuse currently covers positive matches from the first fresh followed-stream page; it does not build a second entity cache. Virtualization, URL routing and external channel actions were not needed for this scope. Phase 3 playback integration and all other excluded features remain deferred.

## Behavioral references

[Streamlink Twitch GUI](https://github.com/streamlink/streamlink-twitch-gui) informed browsing semantics (follows, popular streams, games, search and language), not architecture or a copied UI. Endpoint behavior was checked against Twitch's official [Get Followed Channels](https://dev.twitch.tv/docs/api/reference/#get-followed-channels) and [Search Channels](https://dev.twitch.tv/docs/api/reference/#search-channels) references. The implementation uses the existing Rust Helix foundation and deterministic contract tests rather than the original application's API assumptions.
