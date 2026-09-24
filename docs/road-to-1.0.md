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

## Deferred native dependency migrations

The 1.0 localization work uses the current native dependency stack. Revisit these migrations after 1.0 unless a confirmed security issue, required feature or defect makes one necessary sooner:

- **keyring 3.6.3 → 4.x:** Version 4 separates the core API from platform credential providers. Move only in a dedicated change that proves existing credentials still restore, rotate and delete through macOS Keychain, Windows Credential Manager and Linux Secret Service.
- **GTK 0.18 / GIO 0.18 → newer generations:** Tauri 2 and Wry currently use the same GTK/GIO generation as this app. Move with their native stack, then check Linux title bar, browser dispatch, tray and notifications in a graphical session.
- **windows / windows-core 0.61 → newer generations:** Tauri, Tao, Wry and WebView2 currently use the 0.61 Windows API generation. Review the app's two direct crates together when that stack moves, then check Windows activation, notifications, tray and process ownership on Windows.

The current 1.0 product goals are internationalization infrastructure with English as the source and default language, German, Spanish and French translations, system-locale selection with English fallback, and a contributor workflow for translations. Final UX, accessibility and documentation polish belong in this work. Engineering goals are consolidated documentation, a durable testing policy and a simpler release process that reuses accepted native feature testing when release preparation changes metadata only. These are goals, not claims that localization or final polish already exists.

After 1.0, prioritize the roadmap primarily from actual user and community demand rather than another large fixed phase plan.
