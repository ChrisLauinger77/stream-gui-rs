# Stream GUI RS 0.5.0

Find Twitch Teams, keep local bookmarks, tailor discovery and move around the app more quickly.

## Highlights

### Teams

Browse a Twitch team by its exact name, open a member's channel and use the existing Watch flow. Team membership does not imply that a member is live.

### Bookmarks and hides

Bookmark channels and categories locally, independently of Twitch follows. Hide unwanted channels and categories from passive discovery. Hidden items remain available through Following, Search, Teams, bookmarks, exact lookup and direct links; hiding never blocks an explicit visit.

### Navigation and shortcuts

Home, Back and Forward retain bounded search, scroll and focus history. Configure application-local shortcuts, detect conflicts, unassign actions or reset defaults. Modifier labels follow the platform; shortcuts are not system-wide hotkeys.

### Application links

Installed packages support these navigation links:

```text
stream-gui-rs://show
stream-gui-rs://channel/<login>
stream-gui-rs://category/<id>
stream-gui-rs://team/<name>
```

Links are strictly validated and never start playback. They activate a running app or deliver the latest pending destination after a cold start. Linux and Windows forward to the existing instance; macOS uses installed-bundle activation.

## Platform support and upgrade

Packages: Windows 11 x86_64 NSIS installer and portable/Scoop ZIP; Linux x86_64 AppImage, DEB and RPM; universal macOS DMG for Apple Silicon and Intel, requiring macOS 11.0 or later. Verify downloads against the accompanying SHA-256 files.

Upgrade over 0.4.0 without signing out or clearing settings. Settings schema 6 migrates to schema 7 in memory, preserving playback and channel preferences, profiles, Chatterino, appearance, monitoring and background behavior. Bookmarks and hide lists start empty; shortcuts receive platform defaults. The native credential-store identity is unchanged. Migration is written when settings are next saved; downgrading after that save is unsupported.

Streamlink 8.0+ and a separate player remain required. The application links provide navigation, not a separate command-line interface or playback commands.

## Known limitations

Windows packages are unsigned. macOS bundles have an ad-hoc signature but no Developer ID signature or notarization, so platform warnings may appear. Native Intel execution remains to be verified separately from the universal binary's architecture. There is no embedded playback, automatic updater or global shortcut service.

See the README for prerequisites and installation details.
