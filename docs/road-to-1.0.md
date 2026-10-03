# Road to 1.0

Stream GUI RS has moved from a playback prototype to a cross-platform desktop app. This is a milestone summary, not a fixed implementation plan. Current contracts live in [architecture](architecture.md), and validation policy lives in [testing](testing.md).

| Release | Established capability |
| --- | --- |
| 0.1.0 | Twitch Device Code sign-in with native secure storage, followed/live/category/search browsing, Streamlink playback through external players, independent sessions, settings and native packages. |
| 0.2.0 | Opt-in followed-live monitoring, native notifications, tray/status controls and background close, with quiet startup and recovery. |
| 0.3.0 | Stream language filtering, exact channel lookup, low-latency preference, text scaling, safer support reports and cross-platform About. |
| 0.4.0 | Manual release awareness, independent Chatterino chat and reusable player profiles. |
| 0.5.0 | Teams, local bookmarks and hides, configurable app-local shortcuts, retained navigation state and strictly validated navigation links. |

Rust now owns authentication, Helix/cache/rate limits, settings, native integrations and the sole Streamlink process supervisor. React owns presentation and interaction through typed IPC. Linux, macOS and Windows builds share these boundaries. The core browsing, playback, settings, notification/background, discovery and installed-link capabilities are established; new work should preserve their security, session and lifecycle guarantees rather than restart the architecture.

Exact Kick live-channel playback and explicit browser chat are implemented through Streamlink and the existing playback services; official Kick discovery remains deferred as described below. Advanced transports, more chat clients, legacy configuration import, an embedded player, automatic updating, a separate automation CLI and further streaming services remain deferred. None is a prerequisite for 1.0; revisit them only for a demonstrated user workflow and a viable cross-platform maintenance plan.

## Kick discovery (Stage 2): deferred

**Decision B, revalidated 2026-10-03.** Kick Stage 2 remains deferred because the official discovery APIs still require authenticated tokens that cannot currently be obtained safely by a distributed desktop application without a confidential client secret or backend. The short revalidation found no material change to the access constraints established by the feasibility research earlier that day. Stage 1 playback and explicit browser chat remain available independently of Kick authentication.

Official sources checked:

- [Kick OAuth guide](https://docs.kick.com/getting-started/generating-tokens-oauth2-flow): authorization-code exchange requires both PKCE and `client_secret`; app tokens use `client_credentials` with `client_secret`. No public/native secretless client registration, secretless PKCE exchange or device flow is documented.
- [Kick API specification](https://api.kick.com/swagger/doc.yaml): discovery operations require an app or user token; no anonymous discovery access or viewer Following-list operation is documented.
- [Kick documentation index](https://docs.kick.com/llms.txt): no additional public/native authorization flow is listed.
- [Official changelog](https://github.com/KickEngineering/KickDevDocs/blob/main/changelog.md): V2 categories and livestreams plus user-ID livestream lookup are documented; no new secretless desktop access model is announced. V1 category discovery and V1 livestream browsing are deprecated.

Relevant official operations at `https://api.kick.com` remain:

| Operation | Discovery capability | Required access |
| --- | --- | --- |
| `GET /public/v2/categories` | Categories with filters and cursor pagination | App or user token |
| `GET /public/v2/livestreams` | Live browsing, including category filtering, with cursor pagination | App or user token |
| `GET /public/v1/channels` | Exact channel metadata by slug or broadcaster ID | App token, or user token with `channel:read` |
| `GET /public/v1/users/livestreams` | Active streams for supplied user IDs | App or user token |

The last operation requires known IDs; it does not return a viewer's Following list. Token authorization and a Following-list endpoint are separate prerequisites. Existing follow events do not supply that list.

**Revisit trigger:** Kick must officially document access to the needed discovery operations through anonymous requests, secretless public/native OAuth (PKCE or device flow), or another token model that does not require a confidential application secret. Verify that both registration and the complete token lifecycle support distributed desktop clients before implementation. A future official model requiring a backend/shared secret needs explicit product approval before changing this architecture. Following remains separately deferred until an official viewer Following-list endpoint and its authorization requirements are documented.

**Current implementation boundary:** no Kick discovery client, OAuth, dependencies, secret handling or backend is added. Do not replace the blocked official access with website scraping, undocumented endpoints, Streamlink-based browsing or user-supplied developer registrations. No Kick developer-app scopes are required for the deferred work or existing playback/browser chat; the registration remains unused.

## Native dependency maintenance after 1.0

The 1.0 localization work used the pre-migration native dependency stack. The post-1.0 migrations remain separate:

- **Credential storage completed:** The app moved from keyring 3.6.3 to keyring-core 1 with explicit macOS Keychain, Windows Credential Manager and Linux Secret Service providers. It retained the service/account identity, Linux target and stored record; existing-login upgrade and logout/relogin checks passed on all three platforms.
- **Direct Windows bindings completed:** The app's `windows` and `windows-core` dependencies moved together from 0.61 to 0.62.2. Tauri, Tao, Wry and WebView2 still use the 0.61 generation, so both generations remain in the lockfile. The separate `windows-sys` 0.61 dependency still serves low-level process and console APIs. Hosted checks and Windows 24H2 installed-package acceptance passed for the direct-binding change. Revisit framework alignment when Tauri/Wry and their Windows dependencies adopt a compatible newer generation; review the resulting lockfile and repeat affected Windows activation, notification, tray and process checks.
- **GTK 0.18 / GIO 0.18 deferred:** Keep the app's direct Linux bindings aligned with Tauri/Wry until their native stack supports a newer generation. The current constraint and revisit trigger are below.

### GTK/GIO deferral (2026-09-25)

The app directly requests GTK 0.18 and GIO 0.18 (locked at 0.18.2 and 0.18.4). Tauri 2.11.6, its Wry runtime, Tao, WebKitGTK, and the tray/menu dependencies also resolve GTK/GIO 0.18. [GTK 0.19](https://docs.rs/crate/gtk/0.19.0/source/Cargo.toml) requires GIO/GLib 0.22 and Rust 1.92; this app's current Rust minimum is 1.88. Updating only the direct bindings would leave two incompatible GTK/GIO generations in the desktop process, rather than migrate its WebView, title bar, tray and browser integration together. No Linux runtime or package change was made for this decision, so native Linux acceptance is **NOT TESTED** for GTK/GIO 0.19.

Revisit when a published Wry release includes its [GTK 0.19 migration](https://github.com/tauri-apps/wry/pull/1843), the matching Tao/WebKitGTK/JavaScriptCore and tray/menu crates are available, and a stable Tauri release adopts that coherent stack. The Wry migration is still a draft and waits on those upstream changes; even [Wry 0.57.0](https://docs.rs/crate/wry/0.57.0/source/Cargo.toml) still declares GTK 0.18. The earliest possible compatible versions are therefore later than Wry 0.57.0 and Tauri 2.11.6, but upstream has not assigned exact release numbers. At that point, review Rust and native package requirements, update the coherent dependency set, then run Linux build, browser/deep-link, tray, notification, playback/Restart, Chatterino and Quit checks in a graphical session at normal and enlarged text scale. The Renovate GTK/GIO breaking-update suppressions remain intentional until this trigger is met.

## 1.0 release status

The merged implementation adds bundled English, German, Spanish and French UI catalogs, a System/explicit language setting, English fallback, native tray and notification text, semantic error messages, localized counts and dates, catalog validation, and a [translation contributor guide](translations.md). React integration tests cover locale resolution, switching, persistence and dialog updates. The existing Vitest harness remains the browser UI test layer after [Playwright evaluation](testing.md#interface-automation-and-localization). Two more local shortcuts open the exact channel lookup and bookmarks.

Focused Linux WebKitGTK checks covered language switching, System mode, text growth and zoom. Installed-package acceptance, a real Twitch/playback check, and macOS and Windows native locale checks still need recorded evidence before publication. Automated builds cannot establish real installed-package behavior, and native-language community review remains valuable for German, Spanish and French. Version preparation, candidate audit, exact-package smoke, tagging and publication are separate gates under [releasing](releasing.md).

After 1.0, prioritize the roadmap primarily from actual user and community demand rather than another large fixed phase plan.
