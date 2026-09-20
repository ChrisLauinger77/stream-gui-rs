# Phase 6 Planning

Research date: **2026-09-20**. Status: **proposal only; no implementation authorized by this document**.

Baseline: Stream GUI RS **0.2.0**, repository HEAD `509ea811ff5de9d74dd75a187401dcb13b02f729`. The [v0.2.0 release][R-Release] was published on 2026-09-20. Upstream source was inspected at `f986efe14d814a4bec26809ad5587bab57bd5647`; its latest published release is **v2.5.3**, dated 2024-11-04. Current Streamlink behavior was checked against **8.6.1**, released 2026-09-16. These are dated observations, not promises about future versions.

## Executive summary

Recommend **Everyday viewing and trustworthy troubleshooting** as the Phase 6 / v0.3.0 theme, with five bounded improvements:

- Language filtering for Live and category streams.
- Direct lookup of an exact Twitch channel login.
- An opt-in low-latency preference, globally and per channel.
- Accessibility improvements to text scaling, focus return and status announcements.
- A safe, previewable support report built from explicitly permitted diagnostic fields.

This improves the common path from finding a channel to watching it and resolving problems. It preserves Rust ownership, the current external-player model, restrained settings and bounded resource use. It does not make parity the release criterion.

The largest useful parity gaps are language filters, external chat clients, low-latency configuration, team browsing, customizable focused shortcuts, localization and a small automation interface. They have different value and cost. External chat, player profiles, CLI/deep links and localization should not all land in one release. Profiles have a real use case for switching player configurations, but “Audio only” and “Low bandwidth” labels alone would largely duplicate existing quality choices.

Stream GUI RS is already stronger in credential separation, session-bound requests, explicit process ownership, monitor recovery and native notification integration. Multiple simultaneous streams, themes and channel notification overrides are already present in both applications; their existence alone is not an advantage. Tauri avoids shipping NW.js with Chromium and Node, but no comparative memory/CPU benchmark was performed.

## Evidence and research method

[AGENTS.md](../AGENTS.md) was read before source inspection. Current source takes precedence over historical plans and validation prose. The review covered the repository's application modules, settings and IPC definitions, test suites/fixtures, build/release workflows, README and architecture/authentication/Helix documentation. Phase [0](phase-0-validation.md), [1](phase-1-validation.md), [2](phase-2-validation.md), [3](phase-3-validation.md), [4](phase-4-validation.md) and [5](phase-5-validation.md) validation records were examined as historical evidence. Their statements such as “Phase 5 has not begun,” older schema numbers and old packaging commands do not describe the present tree.

Upstream research included README, wiki, current settings/configuration and service code, routing, authentication, notification polling, player launching, hotkeys, localization, release history, all **24 open issues** and the **100 most recently created closed issues**, with selected issue bodies and maintainer responses examined in depth. GitHub's repository-level open count was 25 because that count also includes pull requests. This is a bounded issue review, not a claim to have read every historical discussion. Related older requests linked from the inspected issues inform the themes below without being counted as independent votes.

Stream GUI RS had one issue, the automated [Dependency Dashboard][R-Issues], and **zero discussions** at inspection. There is not yet enough direct user feedback to infer a community ranking. Recommendations combine source-confirmed limitations, qualitative issue evidence and engineering judgment; “high value” does not mean survey-proven popularity.

Primary external references are linked near claims and collected as reference links at the end. Upstream source links are pinned to the inspected revision; wiki and rendered vendor documentation can change. The wiki checkout was `ed9031a20cf0e77085aab4cc455a2cfc628dfabe`.

## Current Stream GUI RS capabilities

| Area | Implemented now and precise boundary | Implementation / evidence |
| --- | --- | --- |
| Authentication | Public-client Device Code flow; startup/hourly validation; coordinated refresh; cancellation and logout; OS-only production credential persistence. Only `user:read:follows` is requested. Browser login and playback authentication are separate concerns. | [Auth][R-Auth], [vault][R-Vault], [platform store][R-Store], [authentication documentation](authentication.md) |
| Twitch access | One Rust HTTP pool, Helix client, rate coordinator and bounded cache. Original session follows retries, batches, pagination and enrichment, including re-login to the same account. | [Helix][R-Helix], [rate budget][R-Rate], [Helix documentation](helix.md) |
| Following | Live followed streams and all followed channels, with page-bounded metadata/live enrichment. Missing page entries do not establish offline status; failures remain unknown/partial. No follow/unfollow mutation. | [Browse service][R-Browse], [workspace][R-Workspace] |
| Discovery | Popular Live, top categories, category streams, separate channel/category search, channel description/details and live/offline/unknown states. Language is displayed. No language filter, exact-login lookup UI, team route, bookmarks, hidden-channel list or user-selectable sort. | [Browse DTOs][R-BrowseDTO], [details][R-Details], [components][R-Components] |
| Existing lower-level capabilities | `StreamFilter.languages`, `teams` and `channel_teams` exist in Helix. They are not exposed through the browsing command map/UI. `Stream.is_mature` is parsed but not presented; it must not become a new safety filter. | [Helix][R-Helix], [wire models][R-Models], [IPC][R-IPC] |
| Navigation | Back restores route, search/tab state, scroll and item focus. Twelve retained visits/snapshots; ten pages/300 items per view; 30-item requests; explicit Load more and Refresh; retained results labeled. Search debounce and late-response guards. No Forward history or automatic focus refresh. | [Workspace][R-Workspace], [page hook][R-Page], [browse tests][R-BrowseTests] |
| Playback | Streamlink 8.0+ discovery/probing; default, mpv, VLC and custom executable modes; bounded literal player arguments; stdin transport; Source/High/Medium/Low/Audio policies. High/Medium/Low prefer 720p30/540p30/360p30 but may fall back to higher source. | [Playback policy][R-Playback], [discovery][R-Discovery], [probe][R-Probe] |
| Process state | One Supervisor; eight active/reserved sessions, sixteen retained snapshots; explicit IDs and generations; independent Stop/Restart; owned Unix groups/Windows jobs; concurrent pipe draining/reaping. Running means Streamlink exists, not verified video. | [Supervisor][R-Supervisor], [platform ownership][R-Platform], [process tests][R-ProcessTests] |
| Settings | Strict schema v4, atomic persistence, migrations from v1/v2/v3; 256 KiB file and 1,000 channel-record bounds. Global player/path/arguments, quality, automatic chat, theme and background preferences. Channel overrides are **quality, automatic chat and notifications only**. | [Settings][R-Settings], [settings tests][R-SettingsTests], [settings UI][R-SettingsUI] |
| Effective configuration | Rust resolves global defaults, sparse channel overrides and optional request quality into an immutable launch/session snapshot. Save does not alter existing processes; Restart resolves current settings. No saved named profiles or per-channel player/argument override. | [Services][R-Services], [playback policy][R-Playback] |
| Chat | System-browser Twitch popout chat; manual opening supports offline channels; automatic chat runs once per successful launch/restart. Rust validates identity and constructs the URL. Chat failure does not fail playback. No external chat-client preset. | [Chat service][R-Chat], [browser adapter][R-Browser] |
| Monitoring | Opt-in, service-owned complete followed-live scans; 60/120/300-second intervals; foreground rate priority; quiet startup/resume/recovery baselines; stream-ID deduplication; bounded backoff and incomplete-scan reporting. Channel enable/disable/inherit. | [Monitor][R-Monitor], [transition state][R-MonitorState], [scan][R-Scan], [monitor tests][R-MonitorTests] |
| Notifications | Linux D-Bus, Windows WinRT/COM activation and macOS UserNotifications adapters. Click restores/selects the channel within the original session; never starts playback. Windows Notification Center targets survive banner timeout, with 32-entry/15-minute bounds and one-use opaque IDs; no cold launch after Quit. | [Native notifications][R-Notifications], [Windows policy][R-WindowsNotification] |
| Tray/background | Show/Hide, Pause/Resume, **Watching count already present**, Quit with active-stream count. Background close hides only with usable tray, otherwise minimizes; tray-host loss restores hidden Linux window. Explicit Quit drains owned work. | [Tray][R-Tray], [desktop composition][R-Desktop], [Linux background tests][R-BackgroundTests] |
| UI/accessibility | Compact desktop layout; System/Light/Dark; native buttons, visible focus, result regions, loading/error announcements, navigation/pagination focus protection. Fixed focused shortcuts. Text uses many fixed pixel sizes; no user scale setting, translation catalog or complete native screen-reader acceptance record. | [App][R-App], [shortcuts][R-Shortcuts], [CSS][R-CSS], [CSS tests][R-CSSTests] |
| Diagnostics | App version, seven-character build commit, OS/architecture, settings path and current settings in developer diagnostics; process phase, exit code, typed failure and bounded per-session output. Logs are not persisted. **Existing diagnostics contain local paths/arguments and are not a safe support bundle.** | [Diagnostics DTO][R-Diagnostics], [Watching][R-Watching], [build identity][R-BuildInfo] |
| Distribution | Tauri 2/Rust/React; Linux x86_64 packages, Windows x86_64 installer/portable package and universal macOS DMG; CI and exact-candidate publication workflow. No updater, CLI action router, deep-link registration, embedded video/chat, VOD UI or second service. | [README](../README.md), [CI][R-CI], [release workflow][R-ReleaseWorkflow], [releasing](releasing.md) |

Historical native evidence includes real playback/authentication on Linux, macOS and Windows in different development runs. The Phase 5 and release documents retain narrower acceptance gaps, especially installed notification behavior, desktop variants, RPM and Intel macOS. This research did not run the app, log in, test media or certify any current artifact.

## Upstream status

The maintainer's [maintenance note, #1045][U-Maintenance] explicitly ends active development while retaining critical fixes, dependency fixes and compatibility with Streamlink. The repository is **not archived**: the inspected September 2026 HEAD is a dependency/security maintenance change. “Abandoned and no longer functioning” would be an inaccurate summary.

Latest published [v2.5.3][U-Release] is from November 2024; current master has later fixes. For example, the [current provider configuration][U-StreamingConfig] requires Streamlink 7.5.0, while the [wiki][U-StreamlinkWiki] still says 6.0.0. Master removed the obsolete ad-toggle integration after [#1043][U-AdRemoval]. The comparison below uses master for current implementation and identifies release/wiki differences where material.

The app remains an Ember application in NW.js. Its current strengths include a broad configurable interface, ten registered locales, player presets, external chat and automation. Its maintainer describes API removals and runtime/build maintenance as recurring burdens. Those are reasons to select features carefully, not reasons to dismiss every existing feature. [README][U-Readme], [settings][U-Settings], [locales][U-Locales].

## Feature-gap matrix

Complexity is the estimated cost of a sensible Stream GUI RS addition, including tests and native acceptance; it is not a measure of upstream code size. “Already better” means retain the existing RS solution without adding parity work; the separate parity column distinguishes **Equivalent** from an actual advantage. “Reject” rejects the proposed parity addition, not necessarily the external tool's legitimate use elsewhere. Items missing in both apps are identified explicitly.

### Browsing and interaction

| Feature | Upstream behavior | Stream GUI RS behavior | Parity status | Still useful in 2026? | Architectural fit | Security implications | Complexity | Recommendation | Reason |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Following live | Followed-stream list | Session-bound, paginated list | Equivalent | Yes | Existing | Account isolation | Low | Already better | Preserve implementation; monitor is separate |
| All followed channels | Live/offline channel list | Live/offline/unknown, partial enrichment | Stream GUI RS ahead | Yes | Existing | Failure must not imply offline | Low | Already better | More explicit uncertainty |
| Popular/category browsing | Popular streams and games/categories | Live, categories and category streams | Equivalent | Yes | Existing | Rust queries only | Low | Already better | No parity work needed |
| Channel/category search | Combined or filtered search route | Separate channel/category results | Intentionally different | Yes | Good | Bounded queries and generations | Low | Already better | Separate modes are clear; no need to clone combined view |
| Exact channel entry | No dedicated exact-match flow; #940 asks for it | Fuzzy search only | Missing (both) | Yes | Good | Validate login and bind lookup to session | Medium | Phase 6 | Known channel should not be buried in search |
| Channel details | Channel info, stream, team routes and actions | Description, stream state/details, Watch/chat/preferences | Partial | Yes | Good | Untrusted text/images | Medium | Later | Add teams only when their separate value is justified |
| Teams | Channel teams, team live streams/members/info | Lower-level endpoints/models only | Partial | Niche, valid | Good with bounded paging | Team info can contain HTML | Medium | Later | Do not mistake backend methods for a shipped UI |
| Language filtering | Fade unmatched streams or query-filter Live/category routes | Displays language; no filter in browse DTO | Partial | Yes | Good | Validate query values/cache keys | Medium | Phase 6 | Useful discovery improvement without broad new machinery |
| Dual channel/broadcast language | Wiki describes two legacy language concepts | Current stream language plus channel metadata | Obsolete upstream feature | Not as two independent filters | Poor | Do not invent authoritative metadata | Low | Reject | Design around current Helix fields |
| Mature-content toggle | No reliable modern CCL filter found in current settings | Deprecated `is_mature` only in wire model | Obsolete upstream feature | CCL labels useful; old flag no | Requires proper channel enrichment | False reassurance if using deprecated flag | Medium | Later | Evaluate CCL labels, never promise parental filtering from `is_mature` |
| Sort all streams by newest/name | Not provided as complete global sorting; #898/#907 requests | API order only | Missing (both) | Limited | Loaded-result sort only | Do not imply exhaustive results | Medium | Later | Global order cannot be reconstructed from one page |
| Hide channels/categories | Requested in #617; not implemented hide-list | No hide-list | Missing (both) | Yes for some users | Bounded local preference | Keep hiding separate from offline | Medium | Later | Needs honest empty/partial-page semantics |
| Favorite categories | Requested in #689; not implemented bookmarks | No bookmarks | Missing (both) | Yes for repeat discovery | Good | Local preference, not Twitch follow | Medium | Later | Reasonable follow-up after core discovery |
| Back/navigation memory | Router history and configurable home route | Explicit bounded Back, scroll/focus snapshots | Intentionally different | Yes | Existing | Clear on session change | Low | Already better | Bound and preserve current behavior |
| Forward/home customization | Forward shortcut and home preference | Neither exposed | Missing | Modest | Good | Session-keyed history | Medium | Later | Secondary to exact lookup and accessibility |
| Pagination | Infinite-scroll route mixins | Explicit bounded Load more | Intentionally different | Yes | Existing | Finite requests and memory | Low | Already better | Retain deliberate resource/focus contract |
| Refresh on return | Optional focus/restore interval | Explicit Refresh; retained-results labels | Partial | Yes, optional | Rust freshness needed | No frontend TTL authority | Medium | Later | Avoid silently replacing focused results |
| Following mutations | Removed API functionality; web workaround | Not implemented | Obsolete upstream feature | Desire remains, API absent | Poor as native mutation | Reject private API/token scraping | High | Reject | A fixed channel-page opener could be considered separately |
| Focused shortcuts | Configurable DOM shortcuts, including modal actions | Fixed navigation/search/settings/refresh map | Partial | Yes | Good | No arbitrary action strings | Medium | Later | First improve keyboard completion/focus |
| Global hotkeys | No OS registration found in current hotkey service | None | Missing (both) | Niche | Platform-heavy | Ambient actions/Wayland permissions | High | Reject | Not a v0.3.0 need; player keys belong to player |
| Text/focus accessibility | Keyboard-rich, configurable UI; not audited here | Good base; scaling/focus/announcements gaps | Partial | Yes | Good | Avoid credential announcements | Medium | Phase 6 | Benefits more users than a full key-binding editor |
| Localization | Ten registered locales and ICU-style messages | English strings across React and native UI | Missing | Yes | Good but broad | Keep safe errors independent of translation | High | Later | Needs contributors, native strings and ongoing QA |

Sources: [upstream routes][U-Router], [language implementation][U-Languages], [stream settings][U-StreamsSettings], [GUI settings][U-GUISettings], [hotkeys][U-Hotkeys], [older language wiki][U-WatchingWiki], [Twitch reference][T-API]; RS evidence is in the inventory above. The current Twitch reference marks `is_mature` deprecated and always false; modern content classification belongs to channel information.

### Playback, player configuration and chat

| Feature | Upstream behavior | Stream GUI RS behavior | Parity status | Still useful in 2026? | Architectural fit | Security implications | Complexity | Recommendation | Reason |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Discovery/version check | PATH/fallbacks/custom path, timed version check, native or Python provider | Native executable discovery and bounded 8.0+ probe | Intentionally different | Yes | Existing | Explicit path is trust, not sandbox | Low | Already better | Keep the narrower supported launch boundary; both already probe versions |
| Python/script provider | Separate interpreter and script configuration | No Windows script wrappers/provider framework | Intentionally different | Niche only | Poor | Broadens executable trust and packaging failure modes | High | Reject | Native Streamlink installation is the supported dependency |
| Source/High/Medium/Low/Audio | Same policy family, editable selectors | Fixed, tested policy family with source fallback | Equivalent | Yes | Existing | Literal controlled argv | Low | Already better | Keep predictable defaults |
| Exact rendition/custom quality expression | Editable quality presets | No rendition discovery or custom expressions | Partial | Niche, valid | Typed future design | Bound strings; protect policy semantics | Medium | Later | Do not label preferences as strict bitrate caps |
| Low latency | Global and channel override | Missing | Missing | Yes | Excellent | Boolean/enum, no custom command text | Medium | Phase 6 | One meaningful Streamlink control |
| Codec selection / higher resolutions | Possible through advanced custom parameters | Streamlink default H.264; no codec setting | Missing | Increasingly useful | Typed allowlist needed | No OAuth workaround; decoder capability varies | Medium | Later | 8.0+ supports codec choice; native decode testing required |
| Custom Streamlink arguments/config | Advanced free-form parameters; config files can affect launch | No arbitrary Streamlink args; config/sideload disabled | Intentionally different | Individual options useful | Poor as blanket escape hatch | Can override isolation, output, plugins or auth headers | High | Reject | Add specific typed options only |
| Environment handling | Inherits desktop environment; launcher supports internal overrides, no general settings-map editor found | Inherits desktop environment; no editor or app-token injection | Equivalent | Inheritance yes; editor niche | Existing | Neither is an executable sandbox; avoid secret/environment exports | Low | Already better | No environment editor needed |
| Default stdin | Supported/default | Only supported transport | Equivalent | Yes | Excellent | Existing process ownership | Low | Already better | Best fit for current mpv/VLC use |
| FIFO/named pipe | Selectable | Missing | Missing | Niche, valid | Moderate | Temporary pipe lifecycle | Medium | Later | Compatibility escape hatch, not better default |
| HTTP/continuous HTTP | “HTTP” means continuous HTTP | Missing | Missing | Niche, valid | Moderate | Local listener, reconnect and cleanup | High | Later | Require concrete player incompatibility first |
| HLS passthrough | Selectable | Missing | Missing | Niche, valid | Conditional | Media URLs reach player; different ownership/transport tests | High | Later | Modern waiting behavior differs from old wiki warning |
| External HTTP serving | Available via generic Streamlink options | Missing | Intentionally different | Remote-player niche | Poor for local-session app | Network exposure and unauthenticated media listener | High | Reject | Separate product problem |
| Buffer/thread/retry knobs | Broad advanced controls | Streamlink defaults | Partial | Selected options valid | Typed finite controls only | Avoid unlimited retries/resource use | Medium | Later | Low-latency preset covers most immediate value |
| Player modes | Default, mpv, VLC, MPC-HC, PotPlayer, custom | Default, mpv, VLC, custom | Partial | Yes | Good | Local executable trust | Medium | Later | New presets need evidence; custom already covers many cases |
| Player arguments | Strings, presets and metadata substitutions | Bounded literal vector with safe Formatter/shlex encoding | Intentionally different | Yes | Existing | No shell or metadata interpolation | Low | Already better | Preserve robust literal argument semantics |
| Player input placement/title templates | User placeholder placement and rich titles | Owned input appended; metadata never command text | Partial | Niche | Typed design only | Two parsing layers and player-specific interpretation | Medium | Later | Do not copy arbitrary substitution language |
| Reusable named profiles | Separate remembered player presets; no general Home/Low-bandwidth profile system | One global player setup | Missing (general profiles) | Real for multi-player workflows | New explicit model | Snapshot precedence, path validation | High | Later | Not simple upstream parity |
| Per-channel player/volume arguments | Requested in #977; overrides omit player fields | Overrides also omit player fields | Missing (both) | Niche, valid | Prefer future profile reference | Avoid per-channel arbitrary Streamlink arguments | Medium | Later | Concrete use case, no broad demand evidence |
| Multiple sessions | Supported | Supported with explicit bounded IDs | Equivalent | Yes | Existing | Isolation/capacity | Low | Already better | Advantage is supervision, not multiple streams alone |
| Stop/Restart/cleanup | Child lifecycle plus quality-triggered relaunch | Generation-checked service-owned replacement and group/job cleanup | Stream GUI RS ahead | Yes | Existing | Reap before replace; cancellation | Low | Already better | Keep tested race/ownership contracts |
| Logging/error handling | Parses Streamlink output for launch success/errors | OS process state; bounded sanitized output and typed failures | Stream GUI RS ahead | Yes | Existing | Output remains untrusted | Low | Already better | No lifecycle claims from prose logs |
| Browser chat | Default browser and configurable URLs | Fixed validated Twitch popout destination | Intentionally different | Yes | Existing | No arbitrary URL IPC | Low | Already better | Safe default remains useful |
| Chatterino | Explicit preset in current source | Missing | Missing | Yes | Bounded typed launch feasible | Client owns its own login | Medium | Later | First external-chat candidate, after lifecycle design |
| Chatty / standalone | Java+JAR and Windows executable presets | Missing | Missing | Yes | More platform/setup complexity | **Reject upstream token handoff** | High | Later | Independent Chatty auth makes safe integration possible |
| Chromium app-mode chat | Custom browser executable/arguments | No app-mode launcher | Missing | Modest/niche | Platform-heavy | Browser profile/argument trust | Medium | Reject | Native chat clients or normal browser cover the need |
| Arbitrary chat application/token variables | Custom executable/URL/arguments, `{token}` support | No such API | Intentionally different | Token forwarding unacceptable | Conflicts | Credential disclosure/command surface | High | Reject | Presets must never receive this app's OAuth tokens |

Sources: [upstream provider parameters][U-ParametersCode], [version validation][U-Validate], [environment inheritance][U-ChildSpawn], [player presets][U-Players], [channel overrides][U-ChannelSettings], [launch handling][U-Launch], [chat presets][U-ChatConfig], [Chatty launch code][U-ChattyCode], [Streamlink CLI][S-CLI], [current player implementation][S-PlayerCode].

### Desktop, settings, automation and new opportunities

| Feature | Upstream behavior | Stream GUI RS behavior | Parity status | Still useful in 2026? | Architectural fit | Security implications | Complexity | Recommendation | Reason |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Native credentials / auth separation | JS token model persisted through localStorage adapter | Rust-only tokens in OS secure store | Stream GUI RS ahead | Essential | Existing | No plaintext/React/export path | Low | Already better | Preserve stronger boundary |
| Monitor startup/dedup | Quiet initial fill, cached transition comparison | Quiet startup/resume/recovery, bounded stream-ID history | Stream GUI RS ahead | Yes | Existing | Original-session cancellation | Low | Already better | More explicit recovery and completeness policy |
| Notification backend | Chromium/native, SnoreToast, Growl, rich providers | Direct native adapters and permission states | Stream GUI RS ahead | Yes | Existing | Opaque validated callbacks | Low | Already better | Do not recreate provider-selection settings |
| Per-channel notifications | Global allow/deny mode plus override | Global default plus nullable override | Equivalent | Yes | Existing | Preferences do not grant authority | Low | Already better | Existing model expresses opt-in and opt-out |
| Notification launch actions | Configurable click can launch stream/chat | Click selects channel only | Intentionally different | Watch action useful | Moderate | Fresh identity, one-use intent, session checks | High | Later | Navigation already works; avoid accidental launches |
| Notification grouping/history | Grouped alerts; recent history requested in #897 | Bounded delivery and OS history; no app inbox | Partial | Modest | Bounded in-memory view feasible | Viewing habits/session retention | Medium | Later | No durable notification archive by default |
| Tray/background close | Tray, pause and close-to-tray | Native tray with usable-host fallback and shared shutdown | Stream GUI RS ahead | Yes | Existing | Never strand window or bypass cleanup | Low | Already better | Count and Quit behavior already implemented |
| Richer tray live list | Live badge/count mechanisms | Watching count and monitor phase, not live channel list | Partial | Modest | Reuse monitor snapshot | Stale/unknown counts must remain explicit | Medium | Later | No second poller or unbounded menu |
| Themes/settings drafts | Broad persistent settings, light/dark/system | Compact sections, drafts, strict native schema | Intentionally different | Yes | Existing | Atomic validation and sparse records | Low | Already better | Keep breadth proportionate to problems |
| Import/export/reset | Broad settings; import/export and reset requests exist | No legacy import/export UI | Missing | Limited | Typed preference-only future export | Never move tokens/local trust implicitly | Medium | Later | Separate from safe support report |
| CLI actions | Launch/goto/window/theme actions to running app | No action CLI | Missing | Niche, valid | Rust intent router feasible | Single instance, bounded trusted local transport | High | Later | Validate an automation workflow before adding it |
| Deep links | Requested/rejected in #1013, not implemented | None | Missing (both) | Niche, valid | Requires OS registration | All URL input untrusted; show before play | High | Later | Exact lookup solves immediate in-app problem |
| Update awareness | Checks GitHub releases, suppressible | No checker | Missing | Yes | Fixed-host Rust request | No arbitrary URLs or rendered release HTML | Medium | Later | Useful without an automatic updater |
| Automatic updater | Release awareness is not proof of safe in-place updating | None | Missing | Eventually | Distribution work | Signing, rollback, supply chain | High | Later | Separate release/security project |
| Safe support report | Persistent debug logs; users told to remove sensitive data | Developer view and bounded logs, no safe export | Partial | Yes | Excellent if allowlisted | Never serialize existing settings/log DTO wholesale | Medium | Phase 6 | Improves support without credentials/path disclosure |
| Recently watched | Watching is not a durable private channel history | Retained process snapshots only | Missing (both) | Yes for some users | Separate bounded Rust store | Opt-in, retention and clear/delete | Medium | Later | Privacy choice should be deliberate |
| Detailed session health | Launch detection from log text | Process phases only, explicitly limited | Intentionally different | Useful where observable | Limited without player protocol | Never infer rendered video from logs | High | Later | Improve truthful labels before new telemetry |
| More services | Twitch-specific; requests for Kick/YouTube | Twitch-specific despite generic project name | Missing (both) | Potentially | Needs concrete second provider | Separate auth/cache/session/API constraints | High | Later | Feasibility research, not abstraction now |

Sources: [upstream auth storage][U-AuthStore], [polling][U-Polling], [notification settings][U-NotificationSettings], [CLI wiki][U-CLIWiki], [version-check service][U-VersionCheck] and the focused issue analysis below.

## Areas where Stream GUI RS is already ahead

These are implementation advantages, not a claim that the current binaries were exhaustively tested during this research.

| Area | Supported advantage and qualification |
| --- | --- |
| Credential boundary | Tokens stay in Rust and zeroizing buffers, with serialized OS-store access and verified deletion. Upstream's auth model uses its JavaScript/localStorage data layer. RS must not inherit that storage model or token substitution in chat arguments. |
| Device Code authentication | A native public-client flow keeps credentials out of the webview. Startup/hourly validation, single-use refresh sequencing, save-before-validation and logout cancellation are explicit and tested. Device Code itself does not authorize media access or external chat-client login. |
| Session isolation | Original-session leases/generations survive retries, enrichment and pagination. Deterministic regressions cover account changes, including same-user re-login. The reviewed upstream code does not establish equivalent end-to-end guarantees. |
| Shared rate budget | Foreground browsing and background scans share native HTTP/cache/rate coordination, with foreground priority and finite retries. This avoids an independent notification fetch loop competing with browsing. |
| Sleep/resume and recovery | Suspend-aware deadlines and quiet monitor baselines prevent stale credentials/cache/rate deadlines or reconnects being treated as ordinary continuous operation. An incomplete scan cannot establish offline status. Upstream has quiet startup and transition caching, but its bounded polling can return a prefix after its query limit without the same explicit completeness contract. |
| Process ownership | The sole Supervisor owns independent IDs, reservations, cancellation, restart generations and reaping, backed by native process fixtures. Upstream also launches multiple processes, but uses human-readable Streamlink output to determine launch success and has a different detached-child cleanup model. Neither application can guarantee cleanup of a deliberately detached player. |
| Native notifications | Direct Linux, Windows and macOS adapters replace third-party provider selection. Click targets are bounded, opaque and session-validated. Windows COM activation supports Notification Center clicks after banner expiry while the app remains running; this is not cold-start activation after Quit. |
| Background lifecycle | Tray availability determines hide versus minimize; Linux tray-host loss cannot leave the window inaccessible. Explicit Quit coordinates monitor shutdown and owned playback. Per-channel notification overrides themselves are parity, not an exclusive RS feature. |
| Apple Silicon and artifacts | Universal macOS output contains native arm64 support; upstream's open Apple Silicon request remains unresolved. RS also builds Windows and Linux artifacts and promotes a tested candidate without rebuilding. Universal output and CI do not prove every Intel/Apple Silicon desktop behavior. |
| Build identity | A seven-character Git commit is available in diagnostics/About alongside the version. This identifies the compiled revision; it is not a dirty-tree attestation. |
| Runtime composition | Tauri uses each OS's webview instead of bundling NW.js/Chromium/Node. That reduces the bundled runtime responsibility, while introducing WebKitGTK/WKWebView/WebView2 differences. No RAM, launch-time or download-size superiority is claimed without a controlled benchmark. |

Evidence: the current-capability source links, [auth storage][U-AuthStore], [upstream launch handling][U-Launch], [polling][U-Polling], [Apple Silicon issue][U-AppleSilicon], and the [release pipeline][R-ReleaseWorkflow]. Retain these strengths when adding convenience features; broad customization is not worth weakening them.

## Valuable parity gaps and modern equivalents

### Browsing

**Language filtering** has the clearest broad use case: Live/category discovery otherwise returns a large amount of content the user cannot understand. Use the current Helix stream-language parameter; do not locally filter a single page and present it as an exhaustive result. Upstream's alternative of fading nonmatching cards is optional presentation breadth, not a requirement. Following should remain complete in Phase 6, with no hidden favorites caused by a discovery filter.

**Exact channel lookup** is a useful improvement beyond upstream, whose [#940][U-ExactLookup] describes the same fuzzy-search limitation. Resolve a validated login with Get Users, then enter the existing channel-details flow. Distinguish “no such channel,” offline, unknown and network failure. It should not require an arbitrary Twitch URL parser or bypass the existing fresh-identity launch path.

**Teams** remain supported by Twitch's API and useful for communities, but they are niche compared with language/exact lookup. Existing RS models are a head start, not a complete implementation. A later feature needs bounded member enrichment, safe plain-text rendering of team descriptions, unavailable-team states and session-safe navigation. **Bookmarks and hide lists** have repeated issue history, but require a separate preference model and honest handling of pages partly hidden by local choices. A “newest” sort can describe loaded results only; it cannot promise a complete global order from opaque paginated results. [Twitch reference][T-API], [hide-list request][U-Hide], [category bookmarks][U-Bookmarks].

### Streamlink integration: classify the old controls before adding any

The classifications below concern the option or behavior, not whether it is selected for Phase 6. They use Streamlink 8.6.1 source and documentation; RS must still support its documented 8.0 minimum unless a future change separately justifies raising it.

| Legacy transport/configuration feature | Current classification | Modern behavior and RS decision |
| --- | --- | --- |
| Native executable/PATH/custom path | Still useful | Existing Rust discovery and bounded version probe cover it. Preserve paths/symlinks and validate at use. |
| Separate Python interpreter/script provider | Niche but valid | Some installations need it, but RS deliberately supports native executables rather than a provider framework. No Windows script-wrapper support. |
| `streamlinkw.exe` selection | Undesirable for Stream GUI RS | Upstream uses the windowless launcher on Windows. RS already hides consoles with native process flags and owns jobs; do not add another launcher merely for parity. |
| Default stdin | Still useful | Streamlink writes to the player's stdin. Remains the smallest default for mpv/VLC, subject to actual player behavior. |
| `--player-fifo` / Windows named pipe | Niche but valid | Still implemented; not obsolete. Useful for player compatibility. Requires pipe-creation/error/cancellation tests on each OS, so defer until a reproducible need. |
| `--player-http` | Niche but valid | Local HTTP output remains a compatibility alternative for players or cases where stdin fails. A future typed mode needs port/bind policy and ownership tests. |
| `--player-continuous-http` | Niche but valid | Current upstream's “HTTP” setting actually selects this mode. It permits repeated player connections; it is not equivalent to ordinary HTTP output. Useful only with a concrete reconnect scenario. |
| `--player-external-http` | Undesirable for Stream GUI RS | Serving another device is a distinct network-server product surface, with binding/access/lifetime questions. Do not expose it through a generic argument field. |
| `--player-passthrough hls` | Niche but valid | The player receives the HLS URL and handles transport. Modern Streamlink calls the player and waits; the old wiki claim of inevitable detached/uncontrollable playback must not be repeated as current fact. Player/network behavior and quality/low-latency semantics differ; no native acceptance was performed here. |
| `--player-no-close` | Niche but valid | Can preserve a player window when transport ends, but complicates the app's session/Stop expectations. Reject parity until a clear owned-process contract exists. |
| `--fifo`, `--verbose-player`, old HLS-only thread naming | Obsolete spelling | Use current `--player-fifo`, `--player-verbose` and `--stream-segment-threads`. Compatibility aliases are not separate user features. |
| Source/quality preferences | Still useful | Existing bounded policies already solve common choices. Source fallback means bandwidth is not capped. |
| Free-form quality selector | Niche but valid | Specific resolutions/codecs can matter, as #1049 illustrates. Defer validated selectors or codec preference rather than promising every stream has every quality. |
| `--twitch-low-latency` | Still useful | Enables Twitch low-latency handling, including segment prefetch and a live-edge adjustment. Select a boolean preference, not an advanced transport editor. End-to-end delay still depends on the broadcast/network/player. |
| Manual live-edge/segment-thread/retry tuning | Niche but valid | These are real current options, but defaults are usually preferable. Multiple retry knobs can create long ambiguous startup; do not add them without diagnosed failures and bounded behavior. |
| Disable ads | Handled automatically by modern Streamlink | Current Twitch plugin suppresses ad segments automatically; buffering/pauses may still occur. The old `--twitch-disable-ads` argument is a suppressed no-op, not an effective user preference. |
| Disable hosting / disable reruns | Obsolete | Current Twitch plugin retains these as suppressed no-ops. Do not revive removed Twitch behavior or represent a title-based rerun heuristic as authoritative metadata. |
| Codec support preference | Niche but valid | Current Twitch plugin defaults to H.264 and offers additional codecs. New qualities can require player support and separate website authentication. Do not forward RS Helix OAuth or capture browser cookies to unlock them. |
| Web-browser API engine | Handled automatically by modern Streamlink | The Twitch plugin can use Streamlink's browser API when required. Chromium availability or a special executable path can still be a legitimate setup problem; it does not justify adding a browser-automation subsystem to RS. |
| Extra Streamlink arguments/config files/plugin loading | Undesirable for Stream GUI RS | Raw options could override owned player/output/session behavior or send credentials elsewhere. Keep `--no-config`, `--no-plugin-sideloading` and `--`; expose only independently justified typed fields. |
| Environment editor/player environment overrides | Niche but valid elsewhere; undesirable here | Current child processes inherit the desktop environment; RS does not inject its stored OAuth tokens. This is not a hermetic sandbox. An editable environment map adds secret/path/support hazards and is not needed for this release. |
| Dynamic player title/template variables | Niche but valid | Useful labels are possible through validated metadata and fixed templates, but a second general interpolation language is unnecessary. Keep literal argument encoding and the owned `{playerinput}` contract. |

Sources: [current argument parser][S-ArgumentsCode], [Twitch plugin implementation][S-TwitchCode], [passthrough dispatch][S-MainCode], [player output][S-PlayerCode], [player compatibility guide][S-Players], [Twitch plugin guide][S-Twitch], [upstream parameters][U-ParametersCode], [old integration wiki][U-StreamlinkWiki]. No transport is declared broken solely because it is old. Optional HTTP/FIFO support is a defensible later compatibility feature; passthrough is not selected merely because the implementation has become easier to own.

### Players and reusable profiles

Upstream includes mpv, VLC, MPC-HC and PotPlayer configuration plus title/buffer/minimal-interface switches and custom executable/arguments. RS already covers mpv/VLC/custom **modes**, and literal arguments can express many advanced player settings. Upstream's channel overrides are quality, low latency, chat and notifications; it also does **not** provide the proposed general per-channel player-profile system. [Player presets][U-Players], [channel settings][U-ChannelSettings].

A profile becomes valuable when someone repeatedly switches a **bundle**: living-room VLC with a chosen device, desktop mpv with a different window/audio policy, or a custom player with reusable arguments. Recreating Audio and Low bandwidth as named presets alone adds duplication. Quality already changes per launch/channel, and “Low bandwidth” cannot honestly promise a cap with current source fallback. Issue [#977][U-Profiles] is a concrete per-channel player/volume use case, not evidence that every user needs profiles.

A later bounded Rust profile model could contain stable ID, name, player configuration and optional quality/low-latency preferences. Channel records may reference a profile; effective resolution and profile deletion behavior must be explicit. Reuse global → channel/profile → request precedence only after resolving ambiguous combinations; do not casually insert a second settings authority. Sessions retain immutable effective snapshots, and deleting/editing a profile cannot mutate running processes. Paths remain explicit local trust decisions. Start with named user configurations if validated demand exists, rather than shipping six overlapping mandatory presets.

IINA is still a real macOS player option; current Streamlink documentation describes stdin requirements. Its old issue does not justify a cross-platform preset by itself. Existing custom mode is the initial path, with a preset considered after tested installation/bundle discovery. Do not resurrect QuickTime support just because an ancient issue remains open. [Players guide][S-Players], [IINA request][U-IINA], [QuickTime request][U-QuickTime].

### External chat clients

| Client | Current relevance / platforms | Launch and login semantics | Recommendation |
| --- | --- | --- | --- |
| System browser | Current default on all three OSes | RS constructs a fixed Twitch popout URL from validated identity; browser owns its login | Retain as universal fallback |
| Chatterino | Active; v2.5.5 released 2026-03-22; Linux, macOS and Windows | `--channels t:login` creates a supplied layout and disables saving that layout; `--activate t:login` activates/adds a tab in the started app. These arguments alone do **not** prove forwarding to an existing process. Client uses its own saved account/anonymous mode. | Preferred first later integration; test reuse/new-window behavior and lifecycle before promising it |
| Chatty | Active; v0.28 released 2025-12-19; Java distribution across OSes, standalone Windows package | `-single` forwards supported arguments to an already running Chatty instance using its local port when configured consistently. `-channel`/`-connect` can select chat using Chatty's independently configured login. Java/JAR versus bundled executable discovery differs. | Legitimate later alternative; greater installation/process complexity |
| Chromium/Chrome app mode | Still technically possible, already represented upstream | Requires browser executable/profile/window flags; ordinary browser login remains separate | Do not add another preset family in Phase 6 |
| Arbitrary custom client/token templates | Broad upstream escape hatch | Can pass `{token}` or arbitrary URL/arguments | Reject; any future additional client needs a reviewed typed launch contract |

Sources: [Chatterino release][C-Release], [Chatterino argument parser][C-Arguments], [Chatty releases][H-Release], [Chatty command help][H-Help], [standalone setup][H-Standalone], [upstream Chatterino preset][U-ChatterinoCode]. Current upstream already supports Chatterino even though the chat wiki's provider list is older.

The user authenticates in each chat application. RS must not read that client's credentials, transfer its own access/refresh token, or add a token CLI field. Upstream Chatty launch code supplies user/token arguments when its authentication option is enabled; copy the safe channel-selection concept, **not** that credential handoff. Launch chat once per accepted playback generation; failures remain nonfatal to playback. A standalone chat application often should outlive a stream and can reuse an existing process, so neither adding it to the Streamlink process group nor leaving an unreaped child is an acceptable accidental design. Specify launcher reaping, independent-client lifetime and Quit behavior before implementation. Never kill an existing user's chat instance when stopping a stream.

### Shortcuts, CLI and deep links

Upstream's hotkeys are **focused DOM shortcuts**, with configurable bindings and navigation/modal/stream-control actions; they are not an OS global-hotkey implementation. RS already has fixed application-local shortcuts and protections for inputs, editable content, composition, repeated keys and dialogs. A later editor should use a closed action enum, conflict checks, reset/defaults, Control/Command conventions and Rust persistence. Layout handling must work in WebKit as well as WebView2; upstream's Chromium keyboard-layout mapping is not automatically portable. Playback shortcuts would mean launch/open Watching/Stop or Restart a selected explicit session, not video pause/volume/seek in an external player. Accessibility completion takes priority over a binding editor. [Hotkey service][U-Hotkeys], [RS shortcuts][R-Shortcuts].

Upstream's actual argv implementation includes launch, goto, show-state/minimize/maximize/tray, reset-window, theme, log settings and version-check suppression; runtime external actions are opt-in. It does not establish a current typed quality/stop-session protocol. [CLI wiki][U-CLIWiki], [argv implementation][U-Argv].

A useful future CLI could expose only `show`, `open-channel <login>`, `play-channel <login> [quality]` and `stop-session <id>` if real launcher/script workflows justify them. Rust validates each intent, resolves fresh session-bound identity, applies capacity and hands playback to the existing services. No raw executable, URL, provider token or generic JSON command is accepted. Launching a second process must forward bounded actions to one authorized instance; the credential-entry lock is not a complete application single-instance/action router. Handle absent authentication by showing sign-in, never queuing a launch to run silently as a later account.

A URL scheme is better for clickable channel references; a CLI is better for explicit local automation and session IDs. They can share a typed internal intent handler without exposing the whole IPC map. A link should initially **navigate**, not autoplay, because websites can trigger custom schemes. Reject unknown host/action, duplicate fields, arbitrary URL/path payloads and malformed/oversized login values; discard stale intents across logout. Tauri has maintained [deep-link][A-DeepLink] and [single-instance][A-SingleInstance] facilities, but platform registration, packaged cold starts and running-instance delivery need separate tests. Upstream [#1013][U-DeepLinkIssue] is a feature request, not evidence it already has a URL protocol.

### Localization and accessibility

Upstream registers English, German, Spanish, French, Italian, Japanese, Brazilian Portuguese, Russian, Simplified Chinese and Traditional Chinese. RS currently has English strings spread across React, fixed safe error text and native Rust notifications/tray/About. There is no translation catalog or locale-preference architecture. [Locale configuration][U-Locales], [frontend errors][R-Errors], [native desktop][R-Desktop].

Localization is high-value for non-English users but high maintenance: extract messages, support plurals/interpolation, translate native and web content consistently, establish fallback and locale selection, preserve safe error categories, accommodate text expansion and test keyboard layouts. Introduce it when maintainers/contributors can sustain at least one additional language. This task adds neither a library nor translations; it should follow Phase 6 rather than become an unfinished string-extraction project inside it.

Accessibility can deliver concrete benefits now. Existing semantic controls, result announcements, focus styles, image dimensions and theme tokens are useful foundations. Proposed work is targeted: scalable text/layout; predictable focus return from Settings/Watching; concise announcement of accepted playback-request results; and card names/descriptions that convey channel plus live/offline/unknown state without requiring vision. Channel-only accessible names deserve inspection where they suppress useful descendant text. Do not turn continuously arriving player logs into an ARIA live region. Existing CSS does not justify inventing a pervasive animation problem: respect reduced motion for any new transitions and preserve the already restrained presentation.

Automated DOM/CSS tests cannot certify screen-reader usability. Native checks must cover Orca/WebKitGTK, VoiceOver/WKWebView and Narrator/WebView2, large text, light/dark/system contrast, and notification activation returning to a usable channel view. Scale/layout and focus completion have broader immediate value than global hotkeys or copying every upstream click-modifier preference.

## Settings: A–E classification

A smaller settings surface should cover clear choices; legitimate advanced controls can still wait. Existing implemented settings should not be churned to match upstream naming.

| Class | Upstream settings / concepts | RS treatment |
| --- | --- | --- |
| **A. Valuable modern setting** | Quality, player/executable, theme, language filter, low latency, chat choice, notification defaults/overrides, close-to-background, locale, selected focused shortcuts | Most playback/theme/background choices exist. Add language and low latency in Phase 6; defer chat presets, locale and configurable bindings. Display effective values and defaults. |
| **A. Valuable modern setting** | Window restoration/home preference, optional refresh on focus | Valid convenience, lower priority. Preserve focus and original-session state; automatic refresh still asks Rust to determine freshness. |
| **B. Advanced/niche but legitimate** | Literal player arguments, alternate player presets, FIFO/HTTP/continuous HTTP, passthrough, codec preference, exact quality selector, HLS edge/thread/retry tuning, custom executable paths | Keep existing bounded player arguments. Add other controls only for demonstrated workflows, in a compact advanced section with typed validation. |
| **B. Advanced/niche but legitimate** | Notification sound/grouping/click action, modifier-click behaviors, fade versus hide language mismatch, detailed stream-title/uptime/name presentation, tray badge preferences, minimize/restore on playback | Most are optional polish, not missing core functionality. Prefer OS notification preferences and a small set of consistent interactions. |
| **C. Automatically handled today** | Twitch ad-segment suppression; browser API engine selection when needed | Do not expose old toggles. Explain relevant playback/setup failures without promising uninterrupted ads-free playback. |
| **C. Automatically handled today** | Default player launch, native notification provider choice, hidden Windows consoles | Delegate to Streamlink/default player or existing OS adapters. Native adapter selection is not a preference menu. |
| **D. Obsolete** | Hosting/rerun-disable switches, effective ad-disable toggle, old aliases, two independent legacy language dimensions, deprecated `is_mature` filter, removed follow API mutations | Remove from consideration; labels based on title heuristics or deprecated fields are not reliable replacements. |
| **E. Unsafe / conflicts with RS architecture** | OAuth substitution into chat/player commands, webview token storage, arbitrary command/URL templates, generic Streamlink options that defeat owned configuration, plugin sideload/config-file execution | Reject the parity surface. A narrow typed feature can be considered independently if it preserves Rust authority and process ownership. |
| **E. Unsafe / conflicts with RS architecture** | Unrestricted persistent debug logging, export of complete settings/environment/auth state, generic external command endpoint | Replace with bounded safe diagnostics and explicit intents. Existing custom executable selection remains an explicit local trust decision, not a sandbox guarantee. |

Classification does not mean upstream always uses a shell: its launcher constructs executable/argument arrays. The unsafe parts are the breadth of control, credential forwarding and differing ownership, not an unsupported allegation of shell invocation. [Settings model][U-Settings], [streaming settings][U-StreamingSettings], [GUI settings][U-GUISettings], [chat launch][U-ChattyCode], [spawn implementation][U-Spawn].

## Community and issue signal

Issue numbers below identify evidence, not a vote count. Maintenance mode changes how quickly even useful requests can be addressed; closed issues include support, duplicates and declined ideas, not only fixed bugs.

| Signal | Representative evidence | Interpretation for RS |
| --- | --- | --- |
| Recurring playback/setup friction | [#1054][I-1054] timeout, [#1049][I-1049] higher quality, [#1044][I-1044] mpv startup-timing request, [#1027][I-1027] macOS launch | A recurring support/request class, though individual causes differ and some are external network/plugin failures rather than GUI bugs. Better safe support information is more useful than adding speculative retry knobs. |
| Runtime/platform friction | [#1017][I-1017], [#1015][I-1015], [#969][U-AppleSilicon] | Platform issues recur. Tauri/native arm64 and CI address some causes differently, but do not prove immunity to webview/packaging issues. |
| Finding/filtering content | [#940][U-ExactLookup], [#617][U-Hide], [#689][U-Bookmarks], [#907][I-907], [#898][I-898] | Exact lookup is bounded and useful. Hide/bookmark requests have long histories; global sorting is constrained by API semantics. No basis to call any single feature a majority demand. |
| Keyboard/modal ergonomics | [#1039][I-1039] and linked prior requests | Repeated workflow friction supports focus/accessibility and later remapping. It does not establish demand for global OS hooks. |
| Notifications | [#960][I-960], [#959][I-959], [#897][I-897], [#847][I-847] | Native delivery and lifecycle are already addressed in RS. History/title-change alerts are separate, lower-priority features that could increase noise and API work. |
| Player/profile specialization | [#977][U-Profiles], [#480][U-IINA], [#511][U-QuickTime], [#917][I-917] | Niche but real custom-player/audio needs. Existing custom arguments cover some; profiles are useful when switching repeated configurations becomes burdensome. |
| Automation | [#1013][U-DeepLinkIssue] and linked protocol request | Valid integration need, limited evidence of broad use. A safe navigate intent can precede a full CLI. |
| Ads/removed Twitch functionality | [#1043][U-AdRemoval], [#933][I-933], [#754][I-754] | Separate removed switches from ongoing interruptions. The app cannot promise ad-free video or authoritative ad-break status from log text. |
| Service expansion / VOD | [#1040][I-1040], [#875][I-875], [#900][I-900], [#81][I-81] | Persistent expansion requests, but new discovery/auth models and media workflows create a much larger product scope. Streamlink plugin support alone is insufficient. |
| Settings migration/export | [#992][I-992], [#865][I-865] | Legitimate portability/reset requests. A safe report is not a configuration backup; importing credentials or local executable trust would be wrong. |
| Already solved differently | [#1010][I-1010] close-to-tray, notification provider issues, Apple Silicon request | Preserve current background behavior and platform adapters; do not put implemented capabilities back on the feature backlog. |

The current RS [Dependency Dashboard][R-Issues] supplies maintenance information, not user-feature evidence. Further prioritization should solicit concrete workflows after v0.2.0 usage, especially before profiles, translation infrastructure or automation.

## New Stream GUI RS opportunities

| Opportunity | Value and boundaries | Decision |
| --- | --- | --- |
| **A. Sanitized support report** | App version/commit, OS/architecture, validated known Streamlink version, player mode, supported capability/availability summaries, typed failure codes and bounded structured session records. Exclude raw settings, paths, arguments, environment, account/channel identities and arbitrary child output. Preview locally; user chooses whether/where to share. | **Phase 6**; strict allowlist rather than a ZIP of logs |
| **B. Recently watched** | Opt-in local history can make returning to an unfollowed channel easier. Store a bounded channel identity/last-opened time, not a claim of actual video watched. A proposed default retention of 30 days/100 entries would need UX review; expose clear-all and disable-and-delete. Session/account changes must not leak another account's private view. | Later; separate history store and privacy decision |
| **C. Playback/player profiles** | Reusable player/argument/quality combinations solve repeat switching. Not justified by renamed quality buttons alone. Stable IDs, bounded count, deletion/inheritance behavior and immutable launch snapshots need design. | Near-term follow-up if workflows validate demand |
| **D. Native notification actions** | “Open channel” is already the default click. An explicit “Watch” action is a new launch intent and must revalidate session, fresh live identity, capacity and one-use target before launching. OS action limits and installed-app behavior differ. | Later; no arbitrary callback payload or automatic post-login replay |
| **E. Better session health** | “Starting,” process running, stopping, exit code and failure are observable now. Player launched/exited or stream ended may not be independently observable without supported player/Streamlink protocol. State uncertainty clearly; don't classify from output wording or time elapsed. | Phase 6 may improve existing labels within accessibility/report work; new telemetry is Later and separately gated |
| **F. Update awareness** | A bounded Rust request to the fixed project's release endpoint could display “new version available,” version and a validated release link. No token or account data; opt-in/manual check first, finite cache/backoff and safe plain text. This does not install anything. | Near-term after Phase 6; automatic updater remains separate signing/rollback work |
| **G. Better tray information** | Active playback count already exists. A live-followed count or short recent-live list can reuse a published monitor snapshot. Show unavailable/stale/paused distinctly and never perform a second scan from the tray. | Later; cap menu size and retain Linux fallback |
| **H. Safe deep links** | Navigate to a validated Twitch login through a fixed scheme/host/action; confirmation before any optional playback. Cold/running delivery and original-session handling are the hard part. | Later; share typed intent logic with a CLI only if needed |
| **I. More services** | Kick and YouTube are concrete Streamlink-supported candidates with official discovery APIs. Neither currently justifies changing the Twitch ownership model for this release. | Maybe: a bounded feasibility investigation, not a multi-provider framework |

### Concrete second-service feasibility

**Kick** has an official public API for channels/categories/livestreams and OAuth documentation in 2026; dismissing it as “no API” would be stale. Streamlink includes a Kick plugin. However, the documented OAuth authorization-code/PKCE exchange and refresh still require a `client_secret`; PKCE alone does not establish a supported secretless desktop-client flow. Never embed that secret or introduce a credential proxy casually. Before recommending Kick, verify public-client authorization support, terms/quotas, useful authenticated following discovery, identity/session boundaries and reliable playback. Public unauthenticated discovery could be a smaller exploration, but it would be a different feature contract. [Kick OAuth][K-OAuth], [official API explorer][K-API], [official livestream docs][K-Livestreams], [Streamlink plugins][S-Plugins].

**YouTube** has official `search.list` live-event/channel filters and `subscriptions.list`, plus Streamlink support. Subscription discovery is not a direct equivalent of Twitch's complete followed-live endpoint; aggregating it can incur substantial request/quota cost. Current quota documentation must be checked at implementation time rather than repeating old per-search numbers. Validate installed-app OAuth, API policies relevant to this external-player use, quota feasibility and private/member/age-restricted behavior before committing. [YouTube search][Y-Search], [subscriptions][Y-Subscriptions].

**Decision:** no second service in Phase 6 and no generic `Provider` abstraction now. Reopen only with a concrete user workflow, a supported auth model and a discovery budget that can be demonstrated. The project name permits future expansion; it does not require it.

## Value, complexity, risk and maintenance

These are qualitative estimates of an end-to-end feature, including migration, cancellation, accessibility, packaging and native acceptance. “Low risk” still requires appropriate checks. Grouped candidates share the listed integration boundary; implementation would split them only when scope is selected.

| Serious candidate | User value | Complexity | Risk | Maintenance | Tradeoff / disposition |
| --- | --- | --- | --- | --- | --- |
| Language filters | High | Medium | Low | Low | Existing Helix field; query/cursor/cache identity must include filter. Select. |
| Exact channel lookup | High | Medium | Low | Low | Existing Get Users capability and channel view; precise empty/error states. Select. |
| Typed low latency | Medium | Medium | Medium | Low | Small option, but affects settings migration, restart and real playback acceptance. Select. |
| Accessibility/text scale/focus | High | Medium | Medium | Medium | Broad everyday benefit; native screen readers and small-window layouts require QA. Select. |
| Safe support report | High | Medium | Medium | Low | Valuable across repeated support failures; strict field allowlist avoids an unbounded redaction project. Select. |
| Chatterino preset | Medium | Medium | Medium | Medium | Useful desktop chat; process reuse/lifetime and packaging vary. Near-term. |
| Chatty preset | Medium | High | Medium | Medium | Java/standalone discovery and independent login; follows first client integration. Later. |
| Reusable profiles/per-channel reference | Medium | High | Medium | Medium | Real repeat-switching need, but precedence/migration/deletion/UI increase scope. Near-term discovery, implementation only with validated workflows. |
| Focused shortcut remapping | Medium | Medium | Medium | Medium | Accessibility/conflict/layout handling cost exceeds a key-value editor. Later. |
| Teams | Low | Medium | Low | Medium | Backend head start; bounded enrichment and UI still needed. Later. |
| Category bookmarks/hide lists | Medium | Medium | Medium | Medium | Personalized discovery; hidden partial pages and persistence need honest semantics. Later. |
| Forward, home, focus refresh and loaded-result sorting | Medium | Medium | Medium | Low | Incremental navigation convenience; must preserve focus, session and freshness authority. Later, selectively. |
| HTTP/FIFO transport choice | Medium | High | Medium | Medium | Can fix real compatibility cases; native ownership tests on all OSes dominate. Later when demonstrated. |
| Passthrough / advanced retry-buffer tuning | Low | High | High | High | External transport behavior and diagnosis become more complex. Maybe for concrete cases, no broad panel. |
| Codec / validated exact quality choice | Medium | Medium | Medium | Medium | Useful new formats, but availability and player/auth compatibility vary. Later feasibility. |
| Notification Watch action | Medium | High | High | Medium | Ambient inputs can spawn processes; native activation and original-session checks essential. Later. |
| Tray live list / recent notification view | Medium | Medium | Medium | Medium | Reuse existing scan, finite history and truthful stale state; avoid noise. Later. |
| Recently opened/watched history | Medium | Medium | Medium | Medium | Privacy, account separation, retention and clear/delete behavior. Later, opt-in. |
| Update awareness | High | Medium | Low | Low | Fixed-host metadata check; useful without installer work. Near-term. |
| Localization foundation and first maintained locale | High | High | Medium | High | React and Rust messages, plurals, layout and recurring translations. Later with contributors. |
| CLI / deep-link intents | Medium | High | High | Medium | New untrusted entry points, second-instance and packaged OS delivery. Later, initial navigation only. |
| Preference-only export/reset | Medium | Medium | Medium | Medium | Separate portable preferences from credentials and local executable trust. Later. |
| Content-classification labels | Medium | Medium | Medium | Medium | Current channel enrichment required; cannot promise comprehensive safety filtering. Maybe. |
| Detailed player/media health | Medium | High | High | High | No universal player protocol; unsupported inference is worse than honest uncertainty. Maybe. |
| Second streaming service | Medium | High | High | High | Separate auth/discovery/quota/policy and playback acceptance; concrete feasibility gate. Maybe. |
| Automatic updater | Medium | High | High | High | Signing, platform trust, rollback and release operations; outside feature parity. Later release project. |

Global hotkeys, token forwarding and broad command/configuration editors fail the value/security filter before becoming serious candidates. The selected set mixes visible discovery/playback improvements with accessibility/support work; five features are a scope ceiling, not five independent expansion tracks.

## Security considerations

Every proposed feature keeps native authority in Rust. The security filter is a design constraint, not a later review checkbox.

| Feature family | Required safe boundary |
| --- | --- |
| Language/exact lookup/teams/content labels | Validate bounded inputs in Rust; bind to original `RequestSession`; include query dimensions in cache/cursor identity; reuse Helix/rate budget. React renders safe text and validated CDN images only. |
| Low latency/profiles/transports/codecs | Strict settings schema and pure argv builder; retain `--no-config`, `--no-plugin-sideloading`, literal player arguments and `--`. One Supervisor owns reservations, cancellation, restart and child cleanup. No generic executable/argument IPC. |
| External chat | Typed reviewed client preset and validated identity; independent client login; no OAuth, secret device code or credential-store reads for launch. Document launcher versus existing-client ownership. |
| Accessibility/shortcuts/localization | Closed actions; preserve input/modal guards and focus intent. Translate fixed safe messages, not raw provider errors. Scale/locale persist through Rust; no localStorage preference authority. |
| Support report | Dedicated allowlisted DTO, no serialization of developer diagnostics/settings. Never read credentials, environment, raw HTTP bodies or arbitrary child output. A preview cannot make unsafe inclusion acceptable. No automatic upload. |
| CLI/deep links/notification actions | Treat payloads as untrusted intents, not authority. Bounded syntax, original-session validation, fresh identity at launch, one-use targets and capacity checks. No later-account replay, arbitrary URLs or generic command dispatch. |
| History/bookmarks/tray | Bound retention and menus; clear account-scoped snapshots; deliberate private-history policy; never equate failed refresh with offline or zero live channels. |
| Update awareness/import-export/second service | Fixed validated destinations and safe data; tokens never enter files/React/external programs. Imported paths require a fresh local trust decision. A second service must have a real supported auth model without embedded secrets. |

No feature is permitted to introduce plaintext credentials, shell construction, frontend Twitch HTTP, token-bearing diagnostic output, an independent process owner/cache or unsafe ambient launch actions. Ordinary typed external-player configuration already trusts the locally selected executable; these rules do not pretend to sandbox that executable.

## Cross-platform considerations

| Selected Phase 6 feature | Linux / WebKitGTK | macOS / WKWebView | Windows / WebView2 |
| --- | --- | --- | --- |
| Language filtering | Same Rust API/cache contract; native selector and keyboard check | Same API; verify menu/focus behavior and Command conventions | Same API; high-DPI and keyboard selection check |
| Exact channel lookup | Same validated lookup; desktop focus/Back behavior | Same; VoiceOver labeling and focus restoration | Same; Narrator feedback and restored window focus |
| Low latency | Real external Streamlink + mpv/VLC; PATH/exec bits; group cleanup | GUI PATH/app bundle discovery; Apple Silicon and Intel acceptance where available; no Linux VLC switches | Native `.exe`, spaces/Unicode and hidden consoles; job cleanup |
| Accessibility | Orca, min-size window and desktop scaling; no Chromium-only APIs | VoiceOver, native text/zoom behavior and Command shortcuts | Narrator, high DPI, keyboard/focus and native scaling |
| Safe report | Common Rust schema; XDG/paths/username must never leak | Same schema; omit bundle/home paths and Keychain details | Same schema; omit profile/drive paths and Credential Manager details |

None of the five selected features requires a new platform notification provider or transport. Native playback quality/latency remains dependent on the broadcast and external player. Cross-platform CI is necessary but not a substitute for these manual checks.

Later native features carry additional costs: Chatty has Java/standalone differences; Chatterino's process reuse must be observed per package; deep links require installed protocol registration and running-instance forwarding; notifications require installed-app permissions/action delivery; FIFO versus Windows named pipes and HTTP listeners need OS ownership tests. These are reasons to keep them outside the initial release, not claims that they are infeasible.

## Proposed Phase 6 scope

Theme: **Everyday viewing and trustworthy troubleshooting**. Exactly five work items are recommended. If capacity forces a smaller release, remove an entire item rather than weaken its safety/native acceptance contract; do not fill the space with unchecked parity features.

### Language filtering for Live and category streams

- **User problem:** discovery is noisy when most streams are in languages the user does not understand.
- **Scope:** an explicit Any language / one stream-language selector, including Twitch's `other` value where applicable. Apply server-side only to Live and category streams. Show the active filter and a clear reset; distinguish no matches from network/partial errors. Preserve the selection through navigation and Rust settings. Keep Following unfiltered in this release. Multiple-language selection and fading nonmatches can wait.
- **Architecture:** extend the narrow browse request with a validated optional language; use existing `StreamFilter.languages`. Reset cursors/pages on change, include language in Rust cache/query identity and React navigation snapshot keys, and retain original-session generations. Selection is a preference; freshness remains Rust-owned.
- **Dependencies:** existing Helix endpoint, settings migration, generated DTOs and IPC tests; no new HTTP client, cache or package expected. Use a bounded documented language-code set with safe handling of API `other` values.
- **Testing:** request/cursor/filter isolation; changing language mid-request; logout/re-login; accepted-refresh image retry; empty/error states; Back and load-more focus; migration from current settings. Verify no filtering-induced false offline status.
- **Cross-platform:** shared backend behavior; native selector, keyboard and screen-reader checks in all three webviews.
- **Non-goals:** filtering Following, global sorting, a mature-content safety filter, translating the app or every upstream fade/display preference.

### Direct exact channel lookup

- **User problem:** a known login can be difficult to find among fuzzy search results.
- **Scope:** a clearly labeled “Open channel” flow accepting a channel login. Resolve the exact account and navigate to existing channel details; offer existing Watch/chat controls there. Preserve ordinary search. Show distinct invalid login, not found, unavailable and offline outcomes.
- **Architecture:** a narrow typed Rust browse operation using Get Users by login, followed by the current channel-detail path. Normalize case safely and validate bounded syntax; use account IDs for subsequent work. Do not expose arbitrary URLs or generic HTTP. Existing launch still resolves fresh live identity and rechecks authentication at spawn.
- **Dependencies:** existing Helix users lookup, navigation and error vocabulary; explicit command registration/manifest/permissions/capability alignment if a new command is chosen; generated DTOs. No new dependency expected.
- **Testing:** exact match versus fuzzy results, case normalization, nonexistent and offline accounts, transport failure, session switch during lookup, double submission, Back focus and keyboard activation. Test that lookup never starts playback or opens an arbitrary native URL.
- **Cross-platform:** same service; verify focus and accessible result announcements in each webview.
- **Non-goals:** Twitch URL paste parsing, custom protocols, CLI, follow mutation, autoplay, multiple services or a new Twitch cache.

### Opt-in low-latency preference

- **User problem:** viewers using interactive chat may want less delay than the default Streamlink behavior.
- **Scope:** global boolean default **off**, plus per-channel inherit/on/off. Display the effective choice in the launch/settings preview; explain that it can trade buffering resilience for delay and does not guarantee a target latency. An accepted change affects future launches/restarts only.
- **Architecture:** Rust schema migration and sparse override resolution; immutable effective value in `LaunchSpec`/session; pure command construction adds only the supported `--twitch-low-latency` flag when enabled. Keep the Supervisor, quality policy and restart generation/cancellation contract unchanged.
- **Dependencies:** current supported Streamlink 8.0+ already has the option; verify both minimum/current supported behavior. No added crate/package or browser integration expected. Plan settings changes together with language/scale rather than producing multiple temporary schema revisions.
- **Testing:** migration/default/inheritance/remove-fully-inherited-record behavior; exact argv for on/off; source-quality fallback unchanged; settings save does not mutate a process; Restart picks current settings; stop/restart race regressions. Real mpv/VLC/default/custom acceptance includes audio, Source and a preferred lower quality, poor-network behavior, concurrent sessions and cleanup.
- **Cross-platform:** real Streamlink/player checks on Linux/macOS/Windows; preserve platform-specific player presets and existing process ownership. Observe latency qualitatively or with a defined measurement, never infer it from “Running.”
- **Non-goals:** FIFO/HTTP/passthrough, automatic retries/reconnect, codec switching, ad controls, buffer/edge/thread sliders, authenticated-media token injection or named profiles.

### Accessibility: text scale, focus and meaningful status

- **User problem:** small text and incomplete focus/status feedback make repeated desktop use harder, especially with keyboards and assistive technology.
- **Scope:** Rust-persisted application text scale with a small set of choices (proposed 100%, 125%, 150%); use scalable tokens and responsive wrapping within the existing compact layout. Restore focus to the originating control after Settings/Watching closes. Provide concise accepted-action/error feedback and meaningful card state descriptions. Preserve existing focus guards and semantic controls.
- **Architecture:** theme/appearance settings remain Rust-owned; React applies the accepted scale snapshot alongside theme. Track focus origins in UI interaction state, not global application authority. Announce meaningful transitions once without live-reading every player log line or claiming verified video.
- **Dependencies:** existing CSS tokens, appearance hook and behavior-test setup; no component framework expected. Review native notification return-to-channel focus without changing click authority.
- **Testing:** scale persistence/migration; settings validation; keyboard-only primary flows; failed/late requests; focus return when the opener disappears; card descriptions for live/offline/unknown; no focus stealing after the user moves elsewhere. Test 200% system/browser text enlargement as well as selected app scales, minimum window size, both themes, contrast and reduced-motion preferences. Automated semantics tests plus native screen-reader checks are required.
- **Cross-platform:** Orca/WebKitGTK, VoiceOver/WKWebView and Narrator/WebView2, with Linux desktop scaling, macOS conventions and Windows high DPI. Record any unavailable platform acceptance explicitly.
- **Non-goals:** full shortcut editor, global hotkeys, localization, visual redesign, oversized cards or a claim of formal accessibility certification.

### Safe, previewable support report

- **User problem:** playback/setup reports lack comparable version and failure information; manually copying diagnostics can reveal paths or other private data.
- **Scope:** a local “Prepare support report” action producing bounded, readable text from a dedicated safe DTO. Include app version/build commit, OS family/architecture, last validated known Streamlink version or “not checked,” selected player **mode**, and structured process summaries such as phase/typed failure/exit code. Omit account/channel identity and absolute times by default. Show exactly what will be shared in a selectable preview; the user copies it using the normal OS action. No automatic upload or clipboard side effect.
- **Architecture:** Rust builds an explicit allowlist. Reuse bounded retained session snapshots to produce at most sixteen anonymous structured records, with a proposed 64 KiB total report cap; never serialize `BackendDiagnostics`, full settings or raw session output. Export only numeric major/minor/patch version components, omitting free-form suffixes; use failure-code enums, never error-message strings. Do not copy arbitrary executable output into a version or diagnostic field. Report generation must not probe/execute an unknown custom player, read the keyring or inspect the parent environment.
- **Log policy:** raw Streamlink/player logs are **excluded**, even with an “include logs” checkbox. Current redaction cannot guarantee arbitrary executable output is credential-free. Structured state/error records supply safe diagnostic history without pretending to redact every secret. Any later log inclusion needs a separate parser that emits only known templates/numeric fields and drops unknown text; it is outside this release.
- **Dependencies:** existing diagnostics/build identity and supervisor snapshots; one narrow safe-report command/DTO if needed, generated bindings and explicit permissions. No ZIP, crash uploader, clipboard or telemetry dependency expected for a selectable-text first version.
- **Testing:** adversarial fixtures put synthetic secrets, code-like strings, account names, path fragments and control sequences in excluded fields; assert none reaches the report. Test field/size/count bounds, unavailable versions, phase/exit accuracy, stable anonymous labels, no keyring reads and no new processes. Review the format directly as a privacy contract, not just a broad snapshot.
- **Cross-platform:** identical field contract; test Linux/macOS/Windows path shapes and usernames are excluded. Verify keyboard selection/copy and accessible preview in each webview; no new native save-dialog requirement for the initial scope.
- **Non-goals:** raw-log export, settings backup, environment dump, browser/account capture, telemetry, remote submission, a general logging framework or inferred video health.

### Sequencing and release acceptance

Start with a single reviewed settings/DTO design for language, low latency and text scale, preserving migrations from all supported schemas and bounds. Deliver focused vertical slices with their tests; exact lookup and support-report DTO design can be developed independently once their boundaries are settled. This is an implementation sequence proposal, not authorization to begin it.

Before a future v0.3.0 release, run the repository's full application checks from current AGENTS.md/CI, regenerate DTOs for every Rust contract change, and execute applicable native playback/accessibility/browser/desktop acceptance. Record real observations separately from mocks and CI. Linux browser regression must remain isolated; real credentials never enter automated tests. Do not infer successful video, native notifications or screen-reader behavior from process status or build success. Update user/architecture documentation alongside implemented behavior and retain a focused five-feature scope.

## Upstream features we should intentionally not reproduce

| Feature or parity approach | Why not reproduce it |
| --- | --- |
| Tokens in JavaScript/localStorage or external chat/player arguments | Violates the Rust/OS credential boundary. External clients authenticate independently. |
| Generic execute/open-URL/HTTP IPC, shell snippets or unrestricted command templates | Makes a convenience feature a broad native authority surface. Use typed intent and validated data. |
| Generic Streamlink configuration editor / plugin sideloading / environment map | Can undermine the owned process/argv contract, leak secrets and vastly expand support obligations. Expose only justified typed options. |
| Ad-disable/hosting/rerun controls and deprecated argument aliases | No-op/removed behavior or obsolete spelling in current Streamlink. Good defaults and current option names are sufficient. |
| `is_mature` safety filtering or title-based rerun certainty | Deprecated or heuristic signals cannot deliver the promised protection/classification. Modern CCL labels would be separate work. |
| Growl/SnoreToast/provider-selector framework | Native adapters already supersede it; recreating provider UI adds maintenance without a missing core capability. |
| A separate Python/script/windowless-launcher provider framework | Duplicates discovery/process ownership and expands wrapper/runtime failure modes. Existing native dependency support is deliberate. |
| Chromium/Chrome app-mode preset zoo | Browser-specific packaging/profile flags add burden for a niche window preference. System browser and reviewed native chat clients are sufficient directions. |
| Rebuilding Twitch's removed follow mutations through private APIs | Unsupported API/token scraping would break the auth/security model. |
| Global OS hotkey system | Not an upstream capability to recover, high platform/permission cost, unclear demand. Keep focused shortcuts and let players own media controls. |
| General custom chat URLs/apps with OAuth template variables | Credential handoff is unacceptable; safe reviewed presets can be added individually. |
| External HTTP server / leave-player-running parity without ownership design | Adds network exposure or ambiguous Stop/Quit behavior beyond the current app contract. |
| Automatic launches on notification receipt/reconnect or after later login | Ambient events must not silently create processes under a changed session. Explicit safe Watch actions would be different work. |
| Persistent raw debug logs / full settings or credential migration | Privacy and credential risk; neither a safe support report nor a preference export should capture these. |
| Complete duplication of click modifiers, display switches and low-level buffer knobs | Poor value/maintenance ratio absent specific user evidence; preserve progressive disclosure and compact UI. |

FIFO, HTTP, passthrough, custom player arguments, teams and localization are **not** labeled obsolete merely because they are deferred. The rejection is of unsupported/unsafe or unjustified parity, not of all advanced workflows.

## Explicit non-goals for Phase 6

No external chat presets, named profiles, per-channel player selection, alternate transports, codec/custom-quality editor, teams/bookmarks/hide lists, configurable/global shortcuts, notification actions/history, richer tray menu, persistent watch history, CLI/deep links, localization, second service, VOD/downloads, embedded chat/video, automatic retries, update awareness/updater, settings import/export or new phase of architecture abstraction. Existing error labels may improve where necessary for accessibility/report accuracy; that is not a new player-health telemetry feature.

This planning task makes no application, IPC, settings, dependency, version or release changes. The proposed scope is subject to a later implementation request.

## Post-Phase-6 backlog

Ordering within each group expresses judgment, not numeric scores or promised releases. Revisit with actual v0.2/v0.3 user workflows; do not convert the list into a mandatory parity roadmap.

### Near-term

- **Update awareness:** small fixed-source/manual check with safe release information, independent of installing updates.
- **Chatterino integration:** independent login, verified launch/reuse semantics, explicit launcher/client lifetime and browser fallback.
- **Player-profile workflow validation:** test concrete repeated player/device/argument switching; implement a bounded profile model only if it removes real friction.

### Later

- **Discovery/navigation refinements:** category bookmarks or hide lists, teams, Forward/home and optional focus refresh; choose from observed usage rather than bundle all of them.
- **Keyboard and language reach:** focused shortcut remapping and localization with committed translation maintainers, including native strings.
- **Playback compatibility:** typed HTTP/FIFO modes and validated codec/quality choices for demonstrated failures; Chatty after the first safe chat-client integration.
- **Native convenience:** explicit notification Watch action, bounded live tray list/recent alert view, opt-in private recently opened history.
- **Desktop integration:** navigate-first deep links and a minimal typed CLI only for concrete launch/automation workflows; preference-only export/reset separately.
- **Release operations:** automatic updater only after a separate signing, verification and rollback design.

### Maybe

- Current content-classification labels with bounded enrichment; no promise of a comprehensive parental-control filter.
- Passthrough or isolated retry/buffer controls for a reproducible compatibility case.
- Richer player/media health only with a reliable supported observation protocol.
- Kick or YouTube feasibility spike with actual supported OAuth, useful discovery and demonstrated quota/policy fit; no provider abstraction beforehand.

### Rejected

- Credential forwarding/storage regressions, broad command/URL/HTTP APIs, plugin/config/environment escape hatches and shell construction.
- Legacy ad/hosting/rerun toggles, deprecated mature-content flag, private follow APIs and third-party notification-provider parity.
- Global hotkeys, browser app-mode/provider frameworks, generic custom chat/token templates and external HTTP serving without a separate product requirement.
- Raw-log/full-settings diagnostic exports, silent ambient autoplay and unsupported lifecycle claims from log text.

## Proposed v0.3.0 direction

A hypothetical v0.3.0 makes the everyday viewing loop easier: find streams in a language you understand, go straight to a channel you know, and choose low latency when conversation matters. Text can be enlarged without losing the compact desktop layout, keyboard focus returns predictably, and meaningful results are announced clearly. When playback fails, a small local report gives maintainers useful facts without exposing credentials or personal paths.

The application still feels like a focused desktop frontend for external Streamlink and players. It gains practical polish and diagnosability while retaining the security, session isolation and native lifecycle work already completed in v0.2.0. This is a product direction, not final release notes or a commitment to ship every deferred feature.

## Validation of this planning change

- Current capability claims were checked against implementation and existing test contracts, with historical Phase 0–5 documents treated as dated evidence.
- Upstream claims were checked against pinned source, current Streamlink source/documentation and the bounded issue/release review described above. Wiki/master differences are identified explicitly.
- Local file references and pinned source paths were checked; external documentation/release/issue URLs were checked for resolution. The research date and revisions make the findings reproducible, while live pages may change.
- The recommendations retain Rust authority, strict settings/IPC, session isolation, process ownership, bounded work and native-platform acceptance requirements.
- Only this planning document changed. No credentials, personal filesystem paths, provider captures, application/dependency/version changes or generated bindings were introduced.
- `git diff --check` and a whitespace check of this new untracked document passed. Application builds/tests and native/manual acceptance were not run: this task changes documentation only and makes no runtime-validation claim.
- No commit, push, tag or release was created. The working tree contains the new uncommitted planning document.

## Source references

The source links below support the adjacent sections and tables. GitHub implementation links use inspected commit IDs; official product documentation and issue pages were current at the research date.

[A-DeepLink]: https://v2.tauri.app/plugin/deep-linking/
[A-SingleInstance]: https://v2.tauri.app/plugin/single-instance/
[C-Arguments]: https://github.com/Chatterino/chatterino2/blob/1bc6e5aa718379652dcce530f7f88cf615948320/src/common/Args.cpp
[C-Release]: https://github.com/Chatterino/chatterino2/releases/tag/v2.5.5
[H-Help]: https://chatty.github.io/help/help.html
[H-Release]: https://github.com/chatty/chatty/releases/tag/v0.28
[H-Standalone]: https://chatty.github.io/help/help-standalone.html
[I-1010]: https://github.com/streamlink/streamlink-twitch-gui/issues/1010
[I-1015]: https://github.com/streamlink/streamlink-twitch-gui/issues/1015
[I-1017]: https://github.com/streamlink/streamlink-twitch-gui/issues/1017
[I-1027]: https://github.com/streamlink/streamlink-twitch-gui/issues/1027
[I-1039]: https://github.com/streamlink/streamlink-twitch-gui/issues/1039
[I-1040]: https://github.com/streamlink/streamlink-twitch-gui/issues/1040
[I-1044]: https://github.com/streamlink/streamlink-twitch-gui/issues/1044
[I-1049]: https://github.com/streamlink/streamlink-twitch-gui/issues/1049
[I-1054]: https://github.com/streamlink/streamlink-twitch-gui/issues/1054
[I-754]: https://github.com/streamlink/streamlink-twitch-gui/issues/754
[I-81]: https://github.com/streamlink/streamlink-twitch-gui/issues/81
[I-847]: https://github.com/streamlink/streamlink-twitch-gui/issues/847
[I-865]: https://github.com/streamlink/streamlink-twitch-gui/issues/865
[I-875]: https://github.com/streamlink/streamlink-twitch-gui/issues/875
[I-897]: https://github.com/streamlink/streamlink-twitch-gui/issues/897
[I-898]: https://github.com/streamlink/streamlink-twitch-gui/issues/898
[I-900]: https://github.com/streamlink/streamlink-twitch-gui/issues/900
[I-907]: https://github.com/streamlink/streamlink-twitch-gui/issues/907
[I-917]: https://github.com/streamlink/streamlink-twitch-gui/issues/917
[I-933]: https://github.com/streamlink/streamlink-twitch-gui/issues/933
[I-959]: https://github.com/streamlink/streamlink-twitch-gui/issues/959
[I-960]: https://github.com/streamlink/streamlink-twitch-gui/issues/960
[I-992]: https://github.com/streamlink/streamlink-twitch-gui/issues/992
[K-API]: https://api.kick.com/swagger/index.html
[K-Livestreams]: https://github.com/KickEngineering/KickDevDocs/blob/main/apis/livestreams.md
[K-OAuth]: https://docs.kick.com/getting-started/generating-tokens-oauth2-flow
[R-App]: ../src/app/App.tsx
[R-Auth]: ../src-tauri/src/twitch/mod.rs
[R-BackgroundTests]: ../src-tauri/tests/linux_background.rs
[R-Browse]: ../src-tauri/src/helix/browse/mod.rs
[R-BrowseDTO]: ../src-tauri/src/helix/browse/dto.rs
[R-BrowseTests]: ../src/app/Browsing.test.tsx
[R-Browser]: ../src-tauri/src/desktop/browser.rs
[R-BuildInfo]: ../src-tauri/src/build_info.rs
[R-CI]: ../.github/workflows/ci.yml
[R-CSS]: ../src/styles/base.css
[R-CSSTests]: ../src/styles/base.test.js
[R-Chat]: ../src-tauri/src/domain/chat.rs
[R-Components]: ../src/browse/components.tsx
[R-Desktop]: ../src-tauri/src/desktop/mod.rs
[R-Details]: ../src/browse/details.tsx
[R-Diagnostics]: ../src-tauri/src/diagnostics/mod.rs
[R-Discovery]: ../src-tauri/src/streamlink/discovery.rs
[R-Errors]: ../src/browse/errors.ts
[R-Helix]: ../src-tauri/src/helix/mod.rs
[R-IPC]: ../src/lib/ipc.ts
[R-Issues]: https://github.com/ChrisLauinger77/stream-gui-rs/issues/2
[R-Models]: ../src-tauri/src/helix/models.rs
[R-Monitor]: ../src-tauri/src/monitor/mod.rs
[R-MonitorState]: ../src-tauri/src/monitor/state.rs
[R-MonitorTests]: ../src-tauri/src/monitor/mod.rs
[R-Notifications]: ../src-tauri/src/desktop/notifications/mod.rs
[R-Page]: ../src/browse/usePage.ts
[R-Platform]: ../src-tauri/src/platform/mod.rs
[R-Playback]: ../src-tauri/src/streamlink/playback.rs
[R-Probe]: ../src-tauri/src/streamlink/probe.rs
[R-ProcessTests]: ../src-tauri/tests/process_lifecycle.rs
[R-Rate]: ../src-tauri/src/twitch_http/rate.rs
[R-Release]: https://github.com/ChrisLauinger77/stream-gui-rs/releases/tag/v0.2.0
[R-ReleaseWorkflow]: ../.github/workflows/release.yml
[R-Scan]: ../src-tauri/src/helix/monitoring.rs
[R-Services]: ../src-tauri/src/domain/services.rs
[R-Settings]: ../src-tauri/src/config/mod.rs
[R-SettingsTests]: ../src-tauri/src/config/tests.rs
[R-SettingsUI]: ../src/features/PlaybackSettings.tsx
[R-Shortcuts]: ../src/app/shortcuts.ts
[R-Store]: ../src-tauri/src/credentials/platform.rs
[R-Supervisor]: ../src-tauri/src/streamlink/supervisor.rs
[R-Tray]: ../src-tauri/src/desktop/tray.rs
[R-Vault]: ../src-tauri/src/credentials/mod.rs
[R-Watching]: ../src/features/Playback.tsx
[R-WindowsNotification]: ../src-tauri/src/desktop/notifications/windows_state.rs
[R-Workspace]: ../src/browse/Workspace.tsx
[S-ArgumentsCode]: https://github.com/streamlink/streamlink/blob/a009ac0e2ebec00496b78c0349eaba5ac28ab1b2/src/streamlink_cli/argparser.py
[S-CLI]: https://streamlink.github.io/cli.html
[S-MainCode]: https://github.com/streamlink/streamlink/blob/a009ac0e2ebec00496b78c0349eaba5ac28ab1b2/src/streamlink_cli/main.py
[S-PlayerCode]: https://github.com/streamlink/streamlink/blob/a009ac0e2ebec00496b78c0349eaba5ac28ab1b2/src/streamlink_cli/output/player.py
[S-Players]: https://streamlink.github.io/players.html
[S-Plugins]: https://streamlink.github.io/plugins.html
[S-Twitch]: https://streamlink.github.io/cli/plugins/twitch.html
[S-TwitchCode]: https://github.com/streamlink/streamlink/blob/a009ac0e2ebec00496b78c0349eaba5ac28ab1b2/src/streamlink/plugins/twitch.py
[T-API]: https://dev.twitch.tv/docs/api/reference/
[U-AdRemoval]: https://github.com/streamlink/streamlink-twitch-gui/issues/1043
[U-AppleSilicon]: https://github.com/streamlink/streamlink-twitch-gui/issues/969
[U-Argv]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/nwjs/argv.js
[U-AuthStore]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/data/models/auth/adapter.js
[U-Bookmarks]: https://github.com/streamlink/streamlink-twitch-gui/issues/689
[U-CLIWiki]: https://github.com/streamlink/streamlink-twitch-gui/wiki/Parameters
[U-ChannelSettings]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/data/models/channel-settings/model.js
[U-ChatConfig]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/config/chat.json
[U-ChatterinoCode]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/services/chat/providers/chatterino.js
[U-ChattyCode]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/services/chat/providers/chatty.js
[U-DeepLinkIssue]: https://github.com/streamlink/streamlink-twitch-gui/issues/1013
[U-ExactLookup]: https://github.com/streamlink/streamlink-twitch-gui/issues/940
[U-GUISettings]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/data/models/settings/gui/fragment.js
[U-Hide]: https://github.com/streamlink/streamlink-twitch-gui/issues/617
[U-Hotkeys]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/services/hotkey.js
[U-IINA]: https://github.com/streamlink/streamlink-twitch-gui/issues/480
[U-Languages]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/ui/routes/-mixins/routes/filter-languages.js
[U-Launch]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/services/streaming/launch/index.js
[U-Locales]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/config/locales.json
[U-Maintenance]: https://github.com/streamlink/streamlink-twitch-gui/issues/1045
[U-NotificationSettings]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/data/models/settings/notification/fragment.js
[U-ParametersCode]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/services/streaming/provider/parameters.js
[U-Players]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/config/players.json
[U-Polling]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/services/notification/polling.js
[U-Profiles]: https://github.com/streamlink/streamlink-twitch-gui/issues/977
[U-QuickTime]: https://github.com/streamlink/streamlink-twitch-gui/issues/511
[U-Readme]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/README.md
[U-Release]: https://github.com/streamlink/streamlink-twitch-gui/releases/tag/v2.5.3
[U-Router]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/router.js
[U-Settings]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/data/models/settings/model.js
[U-Spawn]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/services/streaming/spawn.js
[U-StreamingConfig]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/config/streaming.json
[U-StreamingSettings]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/data/models/settings/streaming/fragment.js
[U-StreamlinkWiki]: https://github.com/streamlink/streamlink-twitch-gui/wiki/Streamlink-configuration
[U-StreamsSettings]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/data/models/settings/streams/fragment.js
[U-VersionCheck]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/services/versioncheck.js
[Y-Search]: https://developers.google.com/youtube/v3/docs/search/list
[Y-Subscriptions]: https://developers.google.com/youtube/v3/docs/subscriptions/list
[U-Validate]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/services/streaming/provider/validate.js
[U-ChildSpawn]: https://github.com/streamlink/streamlink-twitch-gui/blob/f986efe14d814a4bec26809ad5587bab57bd5647/src/app/utils/node/child_process/spawn.js
[U-WatchingWiki]: https://github.com/streamlink/streamlink-twitch-gui/wiki/Watching-streams
