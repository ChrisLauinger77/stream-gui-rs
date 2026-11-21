# Stream GUI RS 1.11.1

Changes since Stream GUI RS 1.11.0.

## Linux packaging

- Fixed permissions in the AppImage so unprivileged users and sandboxes such as Firejail can reach the launcher. Packaging now checks directory permissions in the finished AppImage.
- Linux packages now build on Ubuntu 22.04. The AppImage build rejects bundled binaries that require glibc newer than 2.35.
- The AppImage download is now named `Stream-GUI-RS_1.11.1_x86_64.AppImage`.

The settings schema remains at version 13. Streamlink 8.0 or later and a separately installed player are still required.

[AppImage packaging fix](https://github.com/ChrisLauinger77/stream-gui-rs/pull/145) · [Full changelog](https://github.com/ChrisLauinger77/stream-gui-rs/compare/v1.11.0...v1.11.1)
