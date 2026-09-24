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

The current 1.0 product goals are internationalization infrastructure with English as the source and default language, German, Spanish and French translations, system-locale selection with English fallback, and a contributor workflow for translations. Final UX, accessibility and documentation polish belong in this work. Engineering goals are consolidated documentation, a durable testing policy and a simpler release process that reuses accepted native feature testing when release preparation changes metadata only. These are goals, not claims that localization or final polish already exists.

After 1.0, prioritize the roadmap primarily from actual user and community demand rather than another large fixed phase plan.
