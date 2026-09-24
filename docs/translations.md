# Contributing translations

Stream GUI RS ships four local interface catalogs: `src/i18n/en.json` (canonical English), `de.json`, `es.json` and `fr.json`. The same catalogs supply React and the small set of native tray, notification and macOS About labels. Twitch names, user profile names, paths, versions, URLs, raw process output and diagnostic field keys remain data.

## Update a message

Choose a stable key for the meaning and location, such as `settings.language` or `profiles.delete.confirm`. Do not copy a whole English sentence into a key. Add the English message and its German, Spanish and French versions in the corresponding catalogs in alphabetical key order. Every locale must have the same set of keys; do not leave English placeholders in another catalog to satisfy the check. Brand names (Stream GUI RS, Twitch, Streamlink, Chatterino, mpv, VLC and GitHub) stay unchanged.

Call `t("key")` from `useI18n()` in a component. For a number-dependent message, add `.one` and `.other` keys and call `count("key", value)`. The count is formatted for the active locale. Use named parameters for values that vary: `t("updates.available", { version })` uses `{version}` in each translation. Keep all placeholders in every locale and let translators move them to natural positions. Avoid concatenating translated sentence fragments. React escapes text parameters; catalogs cannot contain HTML. Use `number` and `date` from the i18n hook for visible values, while keeping identifiers and versions unchanged.

The English catalog is the fallback for a missing translated key. A key absent even in English appears as `⟦key⟧`, and an omitted interpolation value appears as `⟦name⟧`, so development mistakes remain visible. The build check rejects missing/extra keys, mismatched or malformed placeholders, malformed JSON, HTML and unused keys. Run `npm run i18n:check`, `npm run typecheck` and the relevant `npm test` flows before proposing a change.

The shareable support report omits both the selected and detected locale. They are not needed for its process/version diagnostics, and this avoids adding a personal language signal to a report meant for public sharing. A translation report can instead name the chosen UI language in its description.

## Add a locale

Adding a language requires more than a JSON file: extend the closed Rust `UiLanguage` enum and exported TypeScript binding, the language selector, frontend locale resolution and catalog map, native locale resolution, validation locale list, and the locale tests. Run `npm run bindings` after changing the Rust enum. In a desktop build, System mode uses the native locale command so webview and native labels agree; browser preview uses `navigator.languages`. Keep unsupported-language fallback at English. No catalog is fetched or translated at runtime.

## Review

Review the English intent and parameter names first. Have speakers review each translation in Settings, playback, profiles, update checks, Teams, local bookmarks/hides, shortcuts, links and About. Check plural grammar, button length, accessible names and status announcements at 100%, 125% and 150% app text size, plus 200% system/webview scaling. Desktop language switching and native tray/notification text need Linux, macOS and Windows acceptance; browser tests cannot prove those OS surfaces. A community language review is welcome even after automated checks pass.
