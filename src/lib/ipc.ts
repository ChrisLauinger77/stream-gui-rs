import { invoke } from "@tauri-apps/api/core";
import type {
  BrowseRequest, EntityRequest, SearchRequest, PagedResult, StreamSummary, ChannelSummary, CategorySummary, CategoryDetails, ChannelDetails,
  Account, AppError, AuthStatus, BackendDiagnostics, LaunchRequest, ProbeRequest,
  ProbeResult, SessionSnapshot, StopRequest,
} from "./generated";

// Only these named operations are available. DTOs are generated from Rust.
type Commands = {
  list_followed_streams: [BrowseRequest, PagedResult<StreamSummary>];
  list_followed_channels: [BrowseRequest, PagedResult<ChannelSummary>];
  list_streams: [BrowseRequest, PagedResult<StreamSummary>];
  list_categories: [BrowseRequest, PagedResult<CategorySummary>];
  list_category_streams: [EntityRequest, CategoryDetails];
  search_channels: [SearchRequest, PagedResult<ChannelSummary>];
  search_categories: [SearchRequest, PagedResult<CategorySummary>];
  get_channel: [EntityRequest, ChannelDetails];

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
  followedStreams: (request: BrowseRequest) => call("list_followed_streams", request),
  followedChannels: (request: BrowseRequest) => call("list_followed_channels", request),
  streams: (request: BrowseRequest) => call("list_streams", request),
  categories: (request: BrowseRequest) => call("list_categories", request),
  category: (request: EntityRequest) => call("list_category_streams", request),
  searchChannels: (request: SearchRequest) => call("search_channels", request),
  searchCategories: (request: SearchRequest) => call("search_categories", request),
  channel: (request: EntityRequest) => call("get_channel", request),

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
