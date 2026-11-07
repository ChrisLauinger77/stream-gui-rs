# Stream GUI RS 1.9.0

Changes since Stream GUI RS 1.8.0.

This feature release lets Twitch playback request HEVC and AV1 in addition to H.264.

## Features

- Added **Settings → Playback → Twitch codecs** with **Also request HEVC (H.265)** and **Also request AV1** ([#109](https://github.com/ChrisLauinger77/stream-gui-rs/pull/109), [#107](https://github.com/ChrisLauinger77/stream-gui-rs/issues/107)). Both are off by default, and H.264 is always requested. Some Twitch broadcasts offer higher resolutions only with HEVC or AV1; your player and hardware must be able to decode the selected codecs. Some resolutions additionally require Twitch website sign-in, which the app still does not support. Kick and YouTube playback are unchanged. Save the setting and select **Restart** in Watching to apply it to a running session.
- Streamlink still runs without user configuration files (`--no-config`); useful Streamlink options are offered as typed settings instead.

## Dependencies

- Updated Vite from 8.3.5 to 8.3.6 ([#110](https://github.com/ChrisLauinger77/stream-gui-rs/pull/110)).
- Updated jsdom from 30.1.3 to 31.0.0 and adjusted the control-height test to compare against the computed height of an element using the same CSS token ([#111](https://github.com/ChrisLauinger77/stream-gui-rs/pull/111)).
- Refreshed the npm lockfile to pick up tough-cookie 6.1.0 and undici 8.12.4 ([#112](https://github.com/ChrisLauinger77/stream-gui-rs/pull/112)).

## Upgrade and platform support

Settings move from schema 11 to schema 12 when settings are next saved; existing values are unchanged and both extra codecs start off. Upgrade without signing out or clearing settings. Older versions reject the schema 12 settings file without overwriting it.

Streamlink 8.0+ and a separately installed player remain required. Package formats remain Windows x86_64 installer and portable/Scoop ZIP; Linux x86_64 AppImage, DEB and RPM; and a universal macOS DMG for Apple Silicon and Intel requiring macOS 11.0 or later. Verify downloads against the accompanying SHA-256 files.
