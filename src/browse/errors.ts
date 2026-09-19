import type { ErrorCode } from "../lib/generated";
const messages: Record<ErrorCode, string> = {
  incomplete: "Monitoring could not complete all followed-stream pages. Retrying automatically.",
  notification: "Desktop notifications are unavailable. Check your system notification settings.",
  browser_open: "Twitch chat could not be opened in your default browser. Playback is unaffected.",
  streamlink_not_found: "Streamlink was not found. Install it separately or set its path in Settings.",
  unsupported_streamlink: "Playback requires Streamlink 8 or newer. Update Streamlink and test it in Settings.",
  player_not_found: "The selected player was not found. Check its executable path in Settings.",
  invalid_player: "Check the player path and arguments in Settings. Enter one literal argument per row.",
  startup_failed: "Streamlink exited during startup. Open the session diagnostics for details.",
  streamlink_exited: "Streamlink exited with an error. Open the session diagnostics for details.",
  restart_failed: "The session could not be restarted. Check its status and diagnostics, then try again.",
  stream_offline: "This channel is no longer live. Refresh the browsing view.",
  network: "Twitch could not be reached. Check your connection and try again.",
  timeout: "Twitch took too long to respond. Try again.",
  twitch_server: "Twitch is temporarily unavailable. Try again shortly.",
  rate_limited: "Twitch’s request limit was reached. Wait a moment before trying again.",
  unauthorized: "Permission to read this information is missing. Sign out and reconnect to Twitch.",
  unauthenticated: "Your Twitch session has ended. Sign in to continue.",
  auth_invalid: "Your Twitch session is no longer valid. Sign in again.",
  auth_expired: "Your sign-in code expired. Start a new sign-in.",
  auth_denied: "Twitch authorization was declined. You can try signing in again.",
  auth_provider: "Twitch could not complete sign-in. Please try again.",
  credential_store: "Secure storage is unavailable. Unlock your system credential store and retry.",
  not_configured: "Twitch sign-in is not configured for this build. See Settings for setup instructions.",
  cancelled: "The request was cancelled.",
  not_found: "This channel or category is no longer available.",
  invalid_response: "Twitch returned incomplete or unexpected information. Try again.",
  invalid_input: "Check the search or selection and try again.",
  capacity: "Browsing is busy. Please try again shortly.",
  internal: "The application could not complete this request. Try again.",
  settings: "The saved settings could not be read.", settings_version: "These settings were created by a different application version.",
  invalid_executable: "Select a working Streamlink executable.", probe_failed: "Streamlink could not be detected.",
  spawn_failed: "Streamlink could not be started.", process_failed: "The Streamlink process stopped unexpectedly.",
};
export function errorCode(value: unknown): ErrorCode {
  if (typeof value === "object" && value !== null && "code" in value && typeof value.code === "string" && Object.hasOwn(messages, value.code)) return value.code as ErrorCode;
  return "internal";
}
export function friendlyError(value: unknown) { return messages[errorCode(value)]; }
export function errorText(code: ErrorCode) { return messages[code]; }
