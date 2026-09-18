# Twitch authentication

## Flow and setup

Verified against official Twitch documentation on **2026-09-18**:

- [Device Code Grant Flow](https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/#device-code-grant-flow) supports public clients without a client secret and recommends them for open desktop platforms. This application uses that flow in the system browser, with no embedded login or callback listener.
- Device refresh tokens rotate after use and expire after 30 days of inactivity. Access-token expiration comes from the provider's validation response.
- [Validation](https://dev.twitch.tv/docs/authentication/validate-tokens/) is required at startup and at least hourly. The Rust auth task performs both independently of UI activity.
- [Followed channels and streams](https://dev.twitch.tv/docs/api/reference/#get-followed-channels) require `user:read:follows`. This is the only requested scope; unrelated additional grants are tolerated, while a missing required grant requires a new login.

Create a **new Public application** in the [Twitch Developer Console](https://dev.twitch.tv/console/apps). Follow the [registration requirements](https://dev.twitch.tv/docs/authentication/register-app/) (verified email and 2FA). Do not reuse the legacy GUI's client ID or credentials. If registration requires a redirect URL, `http://localhost:3000` can satisfy the form; Device Flow does not use it. No client secret is generated, embedded, stored, or sent by this app.

Set the public client ID in the backend environment:

```sh
TWITCH_CLIENT_ID=your_public_client_id npm run tauri dev
```

Windows PowerShell:

```powershell
$env:TWITCH_CLIENT_ID = "your_public_client_id"
npm run tauri dev
```

A built executable needs the same runtime environment. Finder/Start menu launches may not inherit a terminal's environment. `.env` is not loaded and no `VITE_` credential configuration is used. Packaging a project-owned public client ID is a later distribution decision.

Click **Log in**, open the verification page, and enter the displayed user code. Rust alone receives the secret device code and tokens. The screen displays authorization status, the authenticated account, granted scopes, expiry, and the storage backend. **Cancel authorization** interrupts local polling; it does not undo consent already given in the browser. Validate and Refresh token remain development controls.

## Secure persistence

| Platform | Backend | Requirements / failure strategy |
| --- | --- | --- |
| macOS | Keychain | Allow the application's keychain access when prompted. A locked/denied entry is an explicit error. |
| Windows | Credential Manager | The current user's native credential store must be available. Native errors are reported without credential material. |
| Linux | Secret Service over encrypted D-Bus | A user session bus and unlocked service such as GNOME Keyring or KWallet are required. The build needs `libdbus-1-dev`. Headless sessions without a service cannot persist authentication. |

The `keyring` dependency enables each platform backend explicitly. There is **no plaintext, settings-file, or automatic memory fallback**. A missing entry is signed out; an unavailable/malformed store reports an error while playback remains usable. Unlocking a previously inaccessible store can recover on the next startup-validation retry. If another app instance owns `authentication.lock`, close it and restart the affected instance.

The secure entry uses service `io.github.stream-gui-rs.oauth` and the public client ID as its account key. A private versioned record stores only the client ID and access/refresh pair. The empty `authentication.lock` in the app configuration directory contains no credentials and is held for the store's lifetime to prevent concurrent local instances rotating an entry. Changing client IDs selects a different secure entry; log out under the old configuration first if that entry should be removed.

Blocking storage operations run off Tokio workers and are serialized. Save replaces the token pair in one entry. The old one-use pair is **deleted before refresh is dispatched**, and the replacement is saved before validation. Consequently an ambiguous network failure or crash cannot restore and replay a possibly consumed refresh token. The tradeoff is deliberate: a crash after deletion and before saving the response requires login again. Deletion is followed by a read that must report an absent entry; a retained or unreadable entry is a storage error even if the native delete call reported success. If deletion cannot be verified, no refresh request is sent. There is no retry of an uncertain token exchange.

Normal application exit waits for an already-running rotation/write without revoking the session. An OS storage prompt can therefore delay exit. Force termination cannot guarantee completion; the clear-before-refresh rule still prevents stale-token replay. The filesystem lock coordinates instances sharing the normal application directory; custom forks using the same credential service must also share that lock protocol.

## Lifecycle and errors

- Startup restores credentials, validates them, and only then exposes identity. A rejected access token gets one coordinated refresh attempt, including after offline restoration or explicit validation at expiry; invalid/revoked refresh credentials are cleared. Identity/client/scope mismatches remain terminal. Validation of a newly issued pair cannot recursively refresh again.
- Successful device authorization and refresh persist the pair and validate identity, public client ID, required scopes, and expiry. A refreshed token cannot change the authenticated account.
- Device polling honors the provider interval, slows down on 429, backs off transient failures, and expires locally. Login reuses an active grant. Cancel/logout cannot be followed by a late grant completion that restores identity.
- Hourly validation and known-expiry refresh run without a UI. A transient validation failure hides identity and retries after 60 seconds while preserving the unconsumed refresh token. A Helix 401 triggers one shared refresh and one retry; another 401 invalidates the session.
- Authentication deadlines reconcile wall-clock elapsed time with monotonic time so system sleep counts. Backward wall-clock steps cannot postpone deadlines; short task waits remain monotonic.
- Concurrent validation/refresh work is coalesced. Service-owned operations continue safely if an IPC caller disappears. Read-only auth snapshots and playback do not wait behind network/keychain operations.
- Logout cancels the grant and outstanding account requests, deletes local credentials, clears identity, and then attempts [remote revocation](https://dev.twitch.tv/docs/authentication/revoke-tokens/). Offline revocation reports that local deletion succeeded but remote revocation was not confirmed. Storage deletion failure is an explicit error, not successful logout; retry after unlocking the store.

OAuth and Helix share a fixed-endpoint HTTPS pool with redirects disabled, a 5-second connect deadline and a 15-second request deadline. OAuth JSON is bounded to 64 KiB. Provider bodies, token values, raw HTTP errors and OS error details are never returned in normal errors/logs/diagnostics. Token records cannot be frontend DTOs. Tokens are not passed to Streamlink or imported from the old GUI.

## Opt-in live acceptance checklist

Normal tests use synthetic credentials, mock storage and loopback HTTP only. On 2026-09-18, live macOS testing successfully verified Device Flow login, authenticated account retrieval, and restoration across a complete application restart. The registered public application is `stream-gui-rs`; the project is now **Stream GUI RS** / `stream-gui-rs`. The acceptance run preceded the application identity rename. The current names match by choice, not an OAuth requirement; the new credential namespace requires a fresh login. The following checklist is retained for other platforms and the remaining manual cases; see [the validation record](phase-1-validation.md):

1. Register/configure the new public client, authorize `user:read:follows`, and confirm account/avatar/scopes. **Login and account retrieval verified on macOS.**
2. Restart the desktop app with the same client ID and confirm automatic restoration plus validation. **Complete restart persistence verified on macOS.**
3. Refresh twice and restart again to check rotation with the platform store.
4. Cancel during device authorization; test denial/expiry and try again.
5. Disconnect the app in Twitch account settings, then Validate; local auth must clear.
6. Test an offline startup, recovery, and online/offline logout. After successful local logout, restart signed out.
7. On each OS, test denied/locked credential access, multiple app instances, and closing during refresh. Inspect settings and UI state for public data only; never copy real tokens into test output or bug reports.
8. Suspend past validation, token-expiry, cache and rate-reset deadlines, then resume. Deterministic clock tests cover these contracts; actual OS suspend/resume remains a manual check.
