// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { AuthStatus, SessionSnapshot } from "../lib/generated";
import { api } from "../lib/ipc";
import { SettingsProvider } from "../settings/useSettings";
import { DeveloperTools as App } from "./DeveloperTools";

vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke: vi.fn() }));
vi.mock("../lib/ipc", () => ({
  errorMessage: () => "Request failed",
  api: {
    playbackSettings: vi.fn(), diagnostics: vi.fn(), authStatus: vi.fn(), sessions: vi.fn(),
    login: vi.fn(), stop: vi.fn(), probe: vi.fn(), launch: vi.fn(),
    account: vi.fn(), cancel: vi.fn(), logout: vi.fn(), validate: vi.fn(), refresh: vi.fn(),
  },
}));

const signedOut: AuthStatus = {
  phase: "signed_out", sessionId: null, user: null, authorization: null, error: null, credentialStorage: "memory",
};
const session = (id: string): SessionSnapshot => ({
  id, generation: 1, restarting: false, effectiveSettings: null, chatError: null, stream: null, qualityPolicy: "source", startedAt: 1, endedAt: null, failure: null, phase: "running", pid: 123, url: "https://www.twitch.tv/example", quality: "best",
  exitCode: null, stopRequested: false, logs: [], droppedLogEntries: 0,
});

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

let root: Root;
let container: HTMLDivElement;
function button(label: string, index = 0) {
  return [...container.querySelectorAll("button")].filter((button) => button.textContent === label)[index]!;
}

beforeEach(async () => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  vi.useFakeTimers();
  vi.resetAllMocks();
  vi.mocked(api.diagnostics).mockResolvedValue({
    name: "Stream GUI RS", version: "0.1.0", commit: "a1b2c3d", platform: "test", settingsPath: "settings.json",
    settings: { chatProvider: "browser", chatterinoPath: null, profiles: [], selectedProfileId: null, discoveryLanguage: null, lowLatency: false, textScale: "100", background: { monitoringEnabled: false, notificationsEnabled: false, closeToBackground: false, intervalSeconds: 60 }, theme: "system", automaticChat: false, streamlinkPath: null, player: { mode: "default", executable: null, arguments: [] }, defaultQuality: "source" }, authConfigured: true,
  });
  vi.mocked(api.playbackSettings).mockImplementation(async () => (await api.diagnostics()).settings);
  vi.mocked(api.authStatus).mockResolvedValue(signedOut);
  vi.mocked(api.sessions).mockResolvedValue([session("one"), session("two")]);
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  await act(async () => { root.render(<SettingsProvider><App /></SettingsProvider>); });
});

afterEach(async () => {
  await act(async () => { root.unmount(); });
  container.remove();
  vi.useRealTimers();
});

test("diagnostics display the application version and build commit", () => {
  expect(container.textContent).toContain("Ready · 0.1.0 (a1b2c3d) · test");
});

test("Stop stays available during OAuth and only disables its own session", async () => {
  const login = deferred<AuthStatus>();
  const stopped = deferred<SessionSnapshot>();
  vi.mocked(api.login).mockReturnValue(login.promise);
  vi.mocked(api.stop).mockReturnValue(stopped.promise);
  await act(async () => { button("Log in").click(); });
  expect(button("Log in").disabled).toBe(true);
  expect(button("Stop").disabled).toBe(false);
  await act(async () => { button("Stop").click(); });
  expect(api.stop).toHaveBeenCalledWith("two");
  expect(button("Stop").disabled).toBe(true);
  expect(button("Stop", 1).disabled).toBe(false);
  await act(async () => { stopped.resolve(session("two")); });
  expect(button("Stop").disabled).toBe(false);
  expect(button("Log in").disabled).toBe(true);
  await act(async () => { login.resolve(signedOut); });
});

test("session polling progresses while auth status is waiting, without queuing auth polls", async () => {
  const auth = deferred<AuthStatus>();
  vi.mocked(api.authStatus).mockReturnValue(auth.promise);
  vi.mocked(api.sessions).mockResolvedValue([{ ...session("one"), phase: "exited", exitCode: 0 }]);
  await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
  expect(container.querySelector(".session-heading strong")?.textContent).toBe("exited");
  expect(api.authStatus).toHaveBeenCalledTimes(2);
  await act(async () => { await vi.advanceTimersByTimeAsync(3000); });
  expect(api.sessions).toHaveBeenCalledTimes(5);
  expect(api.authStatus).toHaveBeenCalledTimes(2);
  await act(async () => { auth.resolve(signedOut); });
});

test("a slow probe does not block stopping an existing session", async () => {
  const probe = deferred<Awaited<ReturnType<typeof api.probe>>>();
  vi.mocked(api.probe).mockReturnValue(probe.promise);
  vi.mocked(api.stop).mockResolvedValue(session("two"));
  await act(async () => { button("Probe and save").click(); });
  expect(button("Probe and save").disabled).toBe(true);
  expect(button("Stop").disabled).toBe(false);
  await act(async () => { button("Stop").click(); });
  expect(api.stop).toHaveBeenCalledWith("two");
  await act(async () => { probe.resolve({ executable: "/test/streamlink", version: "8.6.1" }); });
});

const authenticated: AuthStatus = {
  ...signedOut, phase: "authenticated", sessionId: "1",
  user: { id: "123", login: "example", scopes: ["user:read:follows"], expiresIn: 3600 },
};
const account = { id: "123", login: "example", displayName: "Example Account", profileImageUrl: null };

test("restored authentication loads a safe account and displays granted scopes", async () => {
  vi.mocked(api.authStatus).mockResolvedValue(authenticated);
  vi.mocked(api.account).mockResolvedValue(account);
  await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
  expect(api.account).toHaveBeenCalledOnce();
  expect(container.textContent).toContain("Example Account");
  expect(container.textContent).toContain("Granted scopes: user:read:follows");
  expect(button("Log in").disabled).toBe(true);
  expect(button("Log out").disabled).toBe(false);
});

test("cancel remains available while starting device authorization is pending", async () => {
  const login = deferred<AuthStatus>();
  vi.mocked(api.login).mockReturnValue(login.promise);
  await act(async () => { button("Log in").click(); });
  vi.mocked(api.authStatus).mockResolvedValue({ ...signedOut, phase: "authorizing" });
  await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
  expect(button("Log in").disabled).toBe(true);
  expect(button("Cancel authorization").disabled).toBe(false);
  const cancelled: AuthStatus = { ...signedOut, phase: "cancelled" };
  vi.mocked(api.cancel).mockResolvedValue(cancelled);
  vi.mocked(api.authStatus).mockResolvedValue(cancelled);
  await act(async () => { button("Cancel authorization").click(); });
  expect(api.cancel).toHaveBeenCalledOnce();
  expect(container.textContent).toContain("cancelled");
  await act(async () => { login.resolve(cancelled); });
});

test("an account request finishing after logout cannot restore account information", async () => {
  const pendingAccount = deferred<typeof account>();
  vi.mocked(api.account).mockReturnValue(pendingAccount.promise);
  vi.mocked(api.authStatus).mockResolvedValue(authenticated);
  await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
  vi.mocked(api.logout).mockResolvedValue(signedOut);
  vi.mocked(api.authStatus).mockResolvedValue(signedOut);
  await act(async () => { button("Log out").click(); });
  await act(async () => { pendingAccount.resolve(account); });
  expect(container.textContent).not.toContain("Example Account");
  expect(container.textContent).toContain("signed out");
});

test("account retrieval recovers after a temporary failure with bounded retries", async () => {
  vi.mocked(api.authStatus).mockResolvedValue(authenticated);
  vi.mocked(api.account).mockRejectedValueOnce(new Error("offline")).mockResolvedValue(account);
  await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
  expect(api.account).toHaveBeenCalledOnce();
  await act(async () => { await vi.advanceTimersByTimeAsync(59_000); });
  expect(api.account).toHaveBeenCalledOnce();
  await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
  expect(api.account).toHaveBeenCalledTimes(2);
  expect(container.textContent).toContain("Example Account");
});
