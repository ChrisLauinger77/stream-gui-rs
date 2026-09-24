import type { ErrorCode } from "../lib/generated";
import { currentLocale, translate, type Locale, type MessageKey } from "../i18n";
const keys: Record<ErrorCode, MessageKey> = {
  incomplete: "errors.incomplete",
  notification: "errors.notification",
  chatterino_not_found: "errors.chatterinoNotFound",
  chatterino_capacity: "errors.chatterinoCapacity",
  chat_launch: "errors.chatLaunch",
  browser_open: "errors.browserOpen",
  streamlink_not_found: "errors.streamlinkNotFound",
  unsupported_streamlink: "errors.unsupportedStreamlink",
  player_not_found: "errors.playerNotFound",
  invalid_player: "errors.invalidPlayer",
  startup_failed: "errors.startupFailed",
  streamlink_exited: "errors.streamlinkExited",
  restart_failed: "errors.restartFailed",
  stream_offline: "errors.streamOffline",
  network: "errors.network",
  timeout: "errors.timeout",
  twitch_server: "errors.twitchServer",
  rate_limited: "errors.rateLimited",
  unauthorized: "errors.unauthorized",
  unauthenticated: "errors.unauthenticated",
  auth_invalid: "errors.authInvalid",
  auth_expired: "errors.authExpired",
  auth_denied: "errors.authDenied",
  auth_provider: "errors.authProvider",
  credential_store: "errors.credentialStore",
  not_configured: "errors.notConfigured",
  cancelled: "errors.cancelled",
  not_found: "errors.notFound",
  invalid_response: "errors.invalidResponse",
  invalid_input: "errors.invalidInput",
  capacity: "errors.capacity",
  internal: "errors.internal",
  settings: "errors.settings", settings_version: "errors.settingsVersion",
  invalid_executable: "errors.invalidExecutable", probe_failed: "errors.probeFailed",
  spawn_failed: "errors.spawnFailed", process_failed: "errors.processFailed",
};
export function errorCode(value: unknown): ErrorCode {
  if (typeof value === "object" && value !== null && "code" in value && typeof value.code === "string" && Object.prototype.hasOwnProperty.call(keys, value.code)) return value.code as ErrorCode;
  return "internal";
}
export function friendlyError(value: unknown, locale: Locale = currentLocale()) { return translate(locale, keys[errorCode(value)]); }
export function errorText(code: ErrorCode, locale: Locale = currentLocale()) { return translate(locale, keys[code]); }
