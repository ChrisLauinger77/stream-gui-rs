# Stream GUI RS 1.0.0

Use Stream GUI RS in English, German, Spanish or French, and reach channels and bookmarks with two more configurable shortcuts.

## Highlights

### Interface languages

Choose **System**, **English**, **Deutsch**, **Español** or **Français** in **Settings → Appearance → Language**. System follows a supported operating-system language and falls back to English otherwise. The choice is saved, and changing it updates the current interface, including visible feedback and errors. The four catalogs are bundled; no translation download is needed. Native tray, notification and macOS About labels use the selected language. Twitch's separate **Stream language** filter still controls discovery results.

The interface also formats visible counts and dates for the selected language and translates accessible names and status messages. [Translation contributors](translations.md) can review wording and report corrections.

### Channel and bookmark shortcuts

**Open channel** and **Bookmarks** join the configurable application shortcuts. Their defaults are Ctrl+5 and Ctrl+6 on Linux and Windows, or Command+5 and Command+6 on macOS. Existing complete shortcut maps gain the two actions without replacing custom bindings; a default that conflicts with a saved binding remains unassigned. Shortcuts work while the app has focus and pause in editable fields and dialogs.

## Fixes and maintenance

- Twitch discovery accepts otherwise usable Helix responses when unused stream metadata is absent or malformed ([#25](https://github.com/ChrisLauinger77/stream-gui-rs/pull/25)).
- The development dependencies Vite and jsdom were updated ([#24](https://github.com/ChrisLauinger77/stream-gui-rs/pull/24), [#23](https://github.com/ChrisLauinger77/stream-gui-rs/pull/23)). Deferred native dependency migrations are documented in the [roadmap](road-to-1.0.md) ([#22](https://github.com/ChrisLauinger77/stream-gui-rs/pull/22)).

The interface languages were introduced in [#26](https://github.com/ChrisLauinger77/stream-gui-rs/pull/26), and the new shortcuts in [#27](https://github.com/ChrisLauinger77/stream-gui-rs/pull/27).

## Upgrade and platform support

Upgrade over 0.5.0 without signing out or clearing settings. Settings schema 7 migrates to schema 8 in memory, preserving playback, channel, discovery, profile, appearance, monitoring and shortcut preferences. Interface language starts at System. The two new shortcut defaults are added when free; existing custom bindings remain. Migration is written on the next settings save, after which downgrading to 0.5.0 is unsupported. Native credential-store identity is unchanged.

Packages remain Windows 11 x86_64 installer and portable/Scoop ZIP; Linux x86_64 AppImage, DEB and RPM; and a universal macOS DMG for Apple Silicon and Intel requiring macOS 11.0 or later. Verify downloads against the accompanying SHA-256 files. Streamlink 8.0+ and a separate player remain required.

Windows packages are unsigned. macOS bundles have an ad-hoc signature but no Developer ID signature or notarization, so platform warnings may appear. Native Intel execution requires separate verification from the universal binary's architecture.

See the [README](../README.md) for installation requirements and current feature details.
