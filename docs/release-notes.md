# Stream GUI RS 1.8.0

Changes since Stream GUI RS 1.7.1.

This feature release can show the Twitch broadcaster, category and stream title in the mpv window title.

## Features

- Added **Settings → Playback → Show stream details in mpv title** ([#108](https://github.com/ChrisLauinger77/stream-gui-rs/pull/108), [#107](https://github.com/ChrisLauinger77/stream-gui-rs/issues/107)). It is off by default. When enabled and mpv is the selected player, Twitch playback shows `broadcaster - category - stream title` in the mpv window title. Other players, Kick and YouTube are unchanged. Save the setting and select **Restart** in Watching to apply it to a running session.

## Documentation

- Explained how unauthenticated Twitch playback affects ad breaks and the highest available qualities, and why signed-in Twitch playback is deferred ([#107](https://github.com/ChrisLauinger77/stream-gui-rs/issues/107)).

## Dependencies

- Updated webbrowser to 1.2.6, uuid to 1.29.1, toml to 1.2.1 and other locked Rust dependencies ([#99](https://github.com/ChrisLauinger77/stream-gui-rs/pull/99), [#100](https://github.com/ChrisLauinger77/stream-gui-rs/pull/100), [#103](https://github.com/ChrisLauinger77/stream-gui-rs/pull/103), [#106](https://github.com/ChrisLauinger77/stream-gui-rs/pull/106)).
- Updated Vitest to 5.1.0, @vitejs/plugin-react to 6.2.1 and CI tooling ([#101](https://github.com/ChrisLauinger77/stream-gui-rs/pull/101), [#102](https://github.com/ChrisLauinger77/stream-gui-rs/pull/102), [#104](https://github.com/ChrisLauinger77/stream-gui-rs/pull/104), [#105](https://github.com/ChrisLauinger77/stream-gui-rs/pull/105)).

## Upgrade and platform support

Settings move from schema 10 to schema 11 when settings are next saved; existing values are unchanged and the new option starts off. Upgrade without signing out or clearing settings. Older versions reject the schema 11 settings file without overwriting it.

Streamlink 8.0+ and a separately installed player remain required. Package formats remain Windows x86_64 installer and portable/Scoop ZIP; Linux x86_64 AppImage, DEB and RPM; and a universal macOS DMG for Apple Silicon and Intel requiring macOS 11.0 or later. Verify downloads against the accompanying SHA-256 files.
