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

Advanced transports, more chat clients, legacy configuration import, an embedded player, automatic updating, a separate automation CLI and additional streaming services remain deferred. None is a prerequisite for 1.0; revisit them only for a demonstrated user workflow and a viable cross-platform maintenance plan.

## Native dependency maintenance after 1.0

The 1.0 localization work used the pre-migration native dependency stack. The post-1.0 migrations remain separate:

- **Credential storage completed:** The app moved from keyring 3.6.3 to keyring-core 1 with explicit macOS Keychain, Windows Credential Manager and Linux Secret Service providers. It retained the service/account identity, Linux target and stored record; existing-login upgrade and logout/relogin checks passed on all three platforms.
- **Direct Windows bindings completed:** The app's `windows` and `windows-core` dependencies moved together from 0.61 to 0.62.2. Tauri, Tao, Wry and WebView2 still use the 0.61 generation, so both generations remain in the lockfile. The separate `windows-sys` 0.61 dependency still serves low-level process and console APIs. Hosted checks and Windows 24H2 installed-package acceptance passed for the direct-binding change. Revisit framework alignment when Tauri/Wry and their Windows dependencies adopt a compatible newer generation; review the resulting lockfile and repeat affected Windows activation, notification, tray and process checks.
- **GTK 0.18 / GIO 0.18 deferred:** Tauri 2 and Wry currently use the same GTK/GIO generation as this app. Move with their native stack, then check Linux title bar, browser dispatch, tray and notifications in a graphical session.

## 1.0 release status

The merged implementation adds bundled English, German, Spanish and French UI catalogs, a System/explicit language setting, English fallback, native tray and notification text, semantic error messages, localized counts and dates, catalog validation, and a [translation contributor guide](translations.md). React integration tests cover locale resolution, switching, persistence and dialog updates. The existing Vitest harness remains the browser UI test layer after [Playwright evaluation](testing.md#interface-automation-and-localization). Two more local shortcuts open the exact channel lookup and bookmarks.

Focused Linux WebKitGTK checks covered language switching, System mode, text growth and zoom. Installed-package acceptance, a real Twitch/playback check, and macOS and Windows native locale checks still need recorded evidence before publication. Automated builds cannot establish real installed-package behavior, and native-language community review remains valuable for German, Spanish and French. Version preparation, candidate audit, exact-package smoke, tagging and publication are separate gates under [releasing](releasing.md).

After 1.0, prioritize the roadmap primarily from actual user and community demand rather than another large fixed phase plan.
