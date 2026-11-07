# Stream GUI RS 1.9.0

Changes since Stream GUI RS 1.8.0.

This feature release lets Twitch playback request HEVC and AV1 in addition to H.264.

## Features

- Added **Settings → Playback → Twitch codecs** with **Also request HEVC (H.265)** and **Also request AV1** ([#109](https://github.com/ChrisLauinger77/stream-gui-rs/pull/109), [#107](https://github.com/ChrisLauinger77/stream-gui-rs/issues/107)). Both are off by default, and H.264 is always requested. Some Twitch broadcasts offer higher resolutions only with HEVC or AV1; your player and hardware must be able to decode the selected codecs. Some resolutions additionally require Twitch website sign-in, which the app still does not support. Kick and YouTube playback are unchanged. Save the setting and select **Restart** in Watching to apply it to a running session.
- Streamlink still runs without user configuration files (`--no-config`); useful Streamlink options are offered as typed settings instead.

## Upgrade and platform support

Settings move from schema 11 to schema 12 when settings are next saved; existing values are unchanged and both extra codecs start off. Upgrade without signing out or clearing settings. Older versions reject the schema 12 settings file without overwriting it.

Streamlink 8.0+ and a separately installed player remain required. Package formats remain Windows x86_64 installer and portable/Scoop ZIP; Linux x86_64 AppImage, DEB and RPM; and a universal macOS DMG for Apple Silicon and Intel requiring macOS 11.0 or later. Verify downloads against the accompanying SHA-256 files.
