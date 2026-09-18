# Phase 2 validation — Twitch browsing

Validated on macOS on 2026-09-18, including the focused cleanup after the Phase 2 adversarial review. Phase 3 has not started. No dependencies were added.

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
| Frontend behavior tests | **51 passed** (7 preserved prototype, 42 browsing/shell, 2 CSS contracts; 23 cleanup regressions added) |
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
CARGO_NET_OFFLINE=true npm run tauri build -- --debug --bundles app --ci --config '{"bundle":{"active":true}}'
git diff --check
```

The Rust HTTP tests require loopback listeners. An initial sandboxed run could not bind those listeners; the complete rerun with local-listener permission passed all 99 tests. No live credentials or Twitch API requests were used by the tests.

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

These original interface checks used synthetic fixtures and do not establish live CDN delivery. Separately, the user has now confirmed real macOS smoke testing of Phase 2:

- [x] Existing Twitch authentication restored successfully.
- [x] Real followed streams and followed channels loaded.
- [x] Browsing worked with real Twitch data; the application appeared functionally healthy during general browsing.

This records the user's reported observations, not a claim that every detailed scenario below was exercised. The cleanup build was rebuilt successfully; no new live account operations were performed during cleanup verification.

### Cleanup fixture checks

A separate temporary browser fixture used the actual `PageFrame`, `usePage` and stylesheet with synthetic data and delayed responses. Keyboard Enter kept focus on Load more while pending, then moved it to the newly appended result. An empty final page moved focus to the labeled Results region. Pressing Tab during a pending request left focus on the chosen control after completion. The fixture server/tab were closed after verification.

Manual browser palette inspection confirmed dark primary text `#102c23` on `#8bd8bd` (9.00:1), and light primary text `#ffffff` on `#216b54` (6.37:1). Both exceed 4.5:1. The explicit primary-hover selector uses these same foreground/background declarations; automated CSS tests check that rule and both theme palettes. Keyboard focus was observed with a 2px outline and 3px offset, and disabled primary opacity was 0.5. The preview browser's pointer actions did not establish a DOM `:hover` state, so an actual native pointer-hover check remains outstanding; computed palette inspection is not presented as that check. Non-primary hover declarations remain unchanged.

## Final scope/security review

Reviewed the full Phase 2 changes against the preceding rename commit (`30fbad0`), including newly added files:

- Twitch OAuth/Helix HTTP remains Rust-owned; frontend source has no direct Twitch fetch/XHR client.
- Generated DTOs expose no access token, refresh token, client secret or private device secret. The public session marker only selects a Rust-validated auth generation. Device authorization exposes only the information required for the user to sign in.
- New error presentation uses fixed messages by code. No new logging of provider bodies, query credentials or secrets was introduced.
- Tauri commands remain explicit and local-window-only; CSP still blocks direct third-party API connections and permits only Twitch CDN images.
- Backend cache ownership and bounds are preserved. React retains at most twelve snapshots, each capped at ten pages/300 items, with no TTL or persisted account data.
- Search generations, component unmounts, public session keys and Rust lease cancellation prevent stale/account-crossing response reuse. Eight permits bound unfinished queries.
- No browsing playback controls, new player configuration, background monitoring, chat, follow mutations, teams UI or Phase 3 code was introduced.

## Detailed live Twitch smoke checklist — still outstanding

Use a local public client-ID configuration following [authentication setup](authentication.md). Do not record account names, credentials, authorization codes, tokens or account screenshots in the repository.

- [ ] Authenticate through Device Code flow; check account identity in the shell.
- [ ] Inspect followed-live display names, titles, category, viewers and language individually (general loading is confirmed above).
- [ ] Inspect offline followed channels and follow dates individually (general loading is confirmed above).
- [ ] Load popular streams.
- [ ] Browse top categories and open a category's live streams.
- [ ] Load another page; confirm earlier results remain, duplicates do not accumulate and Refresh resets the view.
- [ ] Search for a known channel, then edit the query quickly; old results must not replace the new query.
- [ ] Search for a known category and paginate results when a cursor exists.
- [ ] Open an online channel and inspect title, viewers, language and start time.
- [ ] Open an offline channel; compare with a simulated network error, which must show unavailable rather than offline.
- [ ] Verify real preview/profile/category images, sizes, placeholders and failed-image behavior.
- [ ] Use Tab/Enter throughout, including Search and Back focus restoration.
- [ ] Repeat restart with the cleanup build and verify fresh browsing data (restoration of existing authentication is confirmed above).
- [ ] Log out during a request; confirm account and browsing data disappear.
- [ ] Re-login (also with another account if available); confirm old view snapshots are absent.
- [ ] Repeat core interactions with light/system appearance and at the minimum window size; verify actual native primary-button pointer hover in both themes.

The confirmed Phase 2 observations above supersede the earlier note that no real browsing had been exercised. They do not establish the detailed error, pagination, account-switching or accessibility scenarios in this checklist.

## Focused cleanup regression tests

All previous 28 frontend tests are retained. The following 23 cases were added; names below match the test runner, including expanded parameterized cases. New defect tests were run against the unfixed implementation first: history restoration, final-page focus, same-URL image recovery and the primary-hover rule failed as expected. Preservation cases verify previously working contracts. Focus responses are deliberately deferred, with assertions while requests are pending.

In `src/app/Browsing.test.tsx`:

1. `Back restores the historical search query, cursor, scroll and focus`
2. `Back restores the historical search result type and its pagination`
3. `Back restores the Following tab belonging to that visit`
4. `a new session clears historical search and Following state`
5. `final pagination focuses the first newly appended result`
6. `an empty final page focuses the stable accessible results target`
7. `reaching 300 items focuses a result from the final permitted page`
8. `a pending pagination error preserves control focus, results and retry cursor`
9. `final pagination does not steal focus after moving to another control`
10. `final pagination does not steal focus after moving to the document body`
11. `final pagination does not move focus when Load more was not focused`
12. `successful manual refresh retries the same failed image URL without resetting loaded images`
13. `ordinary image rerenders preserve the failed placeholder without retrying`
14. `changing image src retries naturally with the original layout shape`
15. `repeated image failures wait for another successful refresh`
16. `successful refresh retries failed images in followed channels`
17. `successful refresh retries failed images in categories`
18. `successful refresh retries failed images in category details`
19. `successful refresh retries failed images in channel details`
20. `successful refresh retries failed images in channel search`
21. `successful refresh retries failed images in category search`

In `src/styles/base.test.js` (CSSOM rules and palette contrast, not simulated browser hover):

22. `primary hover explicitly retains a readable palette in dark and light themes`
23. `button hover keeps non-primary styling, disabled opacity and visible focus`

### Cleanup diff review

The four fixes preserve the existing architecture. History still holds at most twelve visits, now adding only a query and two tab choices; it does not duplicate results or TTL data. Restoring those choices in the Back event selects the matching bounded query snapshot and cursor before scroll/focus restoration. Session-keyed workspace replacement still drops both history and snapshots.

Pagination retains only a set of up to 300 visible focus IDs while a focused request is pending. Blur, completion or unmount discards it. The pending button uses `aria-disabled` plus dispatch guards so disabling it does not itself lose keyboard focus. Failed pagination leaves results/cursor/control intact; the existing Rust ownership and request-generation checks are unchanged.

Image retry generation advances only after an accepted successful refresh, including retrying a failed refresh. Failed refreshes, ordinary renders, pagination and theme changes do not advance it. Only failed media reset; loaded images keep their nodes and loaded state. URLs and fixed-shape placeholders are unchanged. The primary hover selector preserves its palette without changing other hover selectors or keyboard focus styling.

No backend, IPC, credential, dependency, CI or playback code changed. No Phase 3 work was introduced.

## Platform gaps and intentional limitations

Linux, Windows and macOS CI remain configured in `.github/workflows/ci.yml`, including the new tests. This checkout has no Git remote, so no hosted run was triggered or claimed. Native Linux/WebKitGTK, Windows/WebView2, OS credential storage on those platforms and assistive-technology testing remain unverified locally. No Chromium-only browser API was introduced; native selects/buttons, CSS grids/aspect ratios and ordinary React/DOM behavior are used.

Browsing refresh is explicit. Navigation snapshots are labeled previous results; there is no followed-stream monitor or automatic stale-cache fallback. Load more stops at ten pages/300 items. Search supersedes old results but does not send a separate per-request cancellation IPC; unfinished work remains under the existing HTTP/rate bounds and the new concurrency cap. Cross-view reuse currently covers positive matches from the first fresh followed-stream page; it does not build a second entity cache. Virtualization, URL routing and external channel actions were not needed for this scope. Phase 3 playback integration and all other excluded features remain deferred.

## Behavioral references

[Streamlink Twitch GUI](https://github.com/streamlink/streamlink-twitch-gui) informed browsing semantics (follows, popular streams, games, search and language), not architecture or a copied UI. Endpoint behavior was checked against Twitch's official [Get Followed Channels](https://dev.twitch.tv/docs/api/reference/#get-followed-channels) and [Search Channels](https://dev.twitch.tv/docs/api/reference/#search-channels) references. The implementation uses the existing Rust Helix foundation and deterministic contract tests rather than the original application's API assumptions.
