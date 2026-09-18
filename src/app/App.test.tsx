// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { AuthStatus, SessionSnapshot } from "../lib/generated";
import { api } from "../lib/ipc";
import { App } from "./App";

vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke: vi.fn() }));
vi.mock("../lib/ipc", () => ({
  errorMessage: () => "Request failed",
  api: {
    diagnostics: vi.fn(), authStatus: vi.fn(), sessions: vi.fn(),
    login: vi.fn(), stop: vi.fn(), probe: vi.fn(), launch: vi.fn(),
  },
}));

const signedOut: AuthStatus = {
  phase: "signed_out", user: null, authorization: null, error: null, credentialStorage: "memory",
};
const session = (id: string): SessionSnapshot => ({
  id, phase: "running", pid: 123, url: "https://www.twitch.tv/example", quality: "best",
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
    name: "Twitch GUI RS", version: "0.1.0", platform: "test", settingsPath: "settings.json",
    settings: { version: 1, streamlinkPath: null }, authConfigured: true,
  });
  vi.mocked(api.authStatus).mockResolvedValue(signedOut);
  vi.mocked(api.sessions).mockResolvedValue([session("one"), session("two")]);
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
  await act(async () => { root.render(<App />); });
});

afterEach(async () => {
  await act(async () => { root.unmount(); });
  container.remove();
  vi.useRealTimers();
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
