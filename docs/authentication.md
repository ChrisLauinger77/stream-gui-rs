# Twitch authentication prototype

## Decision and documentation check

Checked against Twitch's official documentation on **2026-09-18**:

- [Device Code Grant Flow](https://dev.twitch.tv/docs/authentication/getting-tokens-oauth/#device-code-grant-flow) supports public clients and explicitly permits obtaining and refreshing tokens without a client secret. It recommends public clients for open platforms such as Windows. This fits a distributable desktop app that cannot protect an embedded secret. The flow's device-oriented user experience is acceptable for this technical prototype.
- Device-flow refresh tokens rotate after use and expire after 30 days of inactivity. Access-token expiry must be taken from validation, rather than assuming it never changes.
- [Token validation](https://dev.twitch.tv/docs/authentication/validate-tokens/) is required at startup and at least hourly while maintaining an OAuth session.
- [Application registration](https://dev.twitch.tv/docs/authentication/register-app/) requires a developer account, verified email and 2FA. The client ID is public.
- [Token revocation](https://dev.twitch.tv/docs/authentication/revoke-tokens/) uses the client ID and access token. Local clearing must still succeed if revocation is offline.

Device Flow is the selected public-client prototype; no implicit-flow fallback, embedded web login, client secret, token pasted into the UI, or legacy credential reuse is implemented.

## Manual registration boundary

No Twitch application registration or client ID was supplied. Registration is an account-owner action, so live authentication stops here until configured. Offline tests prove the code contracts; they do **not** establish that a live Twitch account has authorized this new app.

1. In the [Twitch Developer Console](https://dev.twitch.tv/console/apps), create a **new application** for this project. Choose a unique name and an appropriate desktop/application category. Choose **Public** as the client type. Do not reuse the old Streamlink Twitch GUI client ID.
2. If the registration form requires an OAuth Redirect URL, add `http://localhost:3000`. Device Flow does not use a redirect or start a callback listener; the URL is only a registration-form requirement.
3. Copy the client ID. No client secret should be generated, stored, sent or embedded for this flow.
4. Set the variable for the backend process, then start the desktop app:

   macOS/Linux:
   ```sh
   TWITCH_CLIENT_ID=your_public_client_id npm run tauri dev
   ```

   Windows PowerShell:
   ```powershell
   $env:TWITCH_CLIENT_ID = "your_public_client_id"
   npm run tauri dev
   ```

   For a built app, launch its executable from an environment containing the same variable. It is intentionally runtime configuration in Phase 0. A Finder/Start menu launch may not inherit your shell environment. `.env` is not loaded; do not use `VITE_` for credentials.

5. Click **Log in**, open the system-browser verification page and authorize. Rust polls Twitch; the screen receives only a user code, activation URL and public identity/status. The secret device code and OAuth tokens remain in Rust.
6. Exercise **Validate**, **Refresh token**, then **Log out**. Restarting the app loses in-memory credentials and requires login again.

The prototype sends the required `scopes` field as an empty string: identity validation does not need a privileged API scope. Acceptance of that request must be confirmed with the new public application during the live smoke test; do not add unrelated scopes just to make a test pass.

## Implemented behavior

Rust uses fixed Twitch OAuth HTTPS endpoints, disallows redirects, imposes a 15-second HTTP timeout and bounds JSON bodies to 64 KiB. Provider response bodies and raw HTTP errors are not logged or returned to the frontend. Secret token structures cannot be serialized to IPC.

Device polling handles Twitch's documented `message: authorization_pending` as well as RFC-style `error` fields, rate limiting/slow-down, denial, expiry and transient failures. It waits at least the provider interval. Repeated login during an active grant reuses that grant.

Tokens are validated before an authenticated identity is exposed. Wrong client IDs, missing identity, expiry and rejection clear credentials. Validation runs after login/refresh, on startup if a future storage adapter restores credentials, and hourly. Temporary validation failures hide the authenticated identity and retry after a minute. Expiring tokens refresh proactively; refresh can also be exercised manually. Refresh outcomes that may have consumed a one-use token require a new login rather than retrying stale credentials.

Logout cancels the pending grant, clears the credential store and identity, then attempts remote revocation. Offline revocation produces a status message; local logout remains complete. Browser authorization itself cannot be undone by canceling the local poll. Closing the app drops memory; it does not perform a remote revocation request.

## Live acceptance checklist (manual)

- Public registration accepts the scope-free device request and browser consent completes.
- The UI shows the validated account, but no tokens in UI state, settings, console or process arguments.
- Manual and automatic refresh rotate credentials; a second refresh works with the new token.
- Cancel during authorization cannot authenticate later; denied/expired codes require login again.
- Disconnect the application in Twitch account settings, then validate: the local session is cleared.
- Logout works online and offline; restart is signed out.

Secure persistent credential storage, production distribution of the project's public client ID, and provider verification with a real account remain future decisions. No Phase 1 functionality is implied.
