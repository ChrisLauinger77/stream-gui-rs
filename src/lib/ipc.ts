import { invoke } from "@tauri-apps/api/core";
import type {
  Account, AppError, AuthStatus, BackendDiagnostics, LaunchRequest, ProbeRequest,
  ProbeResult, SessionSnapshot, StopRequest,
} from "./generated";

// Only these named operations are available. DTOs are generated from Rust.
type Commands = {
  backend_diagnostics: [undefined, BackendDiagnostics];
  streamlink_probe: [ProbeRequest, ProbeResult];
  streamlink_launch: [LaunchRequest, SessionSnapshot];
  streamlink_stop: [StopRequest, SessionSnapshot];
  streamlink_sessions: [undefined, SessionSnapshot[]];
  auth_status: [undefined, AuthStatus];
  auth_login: [undefined, AuthStatus];
  auth_open_verification: [undefined, null];
  auth_validate: [undefined, AuthStatus];
  auth_refresh: [undefined, AuthStatus];
  auth_logout: [undefined, AuthStatus];
  auth_cancel: [undefined, AuthStatus];
  auth_account: [undefined, Account];
};

async function call<K extends keyof Commands>(
  command: K,
  ...args: Commands[K][0] extends undefined ? [] : [Commands[K][0]]
): Promise<Commands[K][1]> {
  return invoke<Commands[K][1]>(command, args.length ? { request: args[0] } : undefined);
}

export const api = {
  diagnostics: () => call("backend_diagnostics"),
  probe: (request: ProbeRequest) => call("streamlink_probe", request),
  launch: (request: LaunchRequest) => call("streamlink_launch", request),
  stop: (sessionId: string) => call("streamlink_stop", { sessionId }),
  sessions: () => call("streamlink_sessions"),
  authStatus: () => call("auth_status"),
  login: () => call("auth_login"),
  openVerification: () => call("auth_open_verification"),
  validate: () => call("auth_validate"),
  refresh: () => call("auth_refresh"),
  logout: () => call("auth_logout"),
  cancel: () => call("auth_cancel"),
  account: () => call("auth_account"),
};

export function errorMessage(error: unknown): string {
  if (typeof error === "object" && error !== null && "code" in error && "message" in error) {
    const typed = error as AppError;
    return `${typed.code}: ${typed.message}`;
  }
  // Do not stringify unknown IPC payloads into the UI or console.
  return "The backend could not complete the request. Run the desktop app with npm run tauri dev.";
}
