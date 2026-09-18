// @vitest-environment jsdom
import { act, StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { App } from "./App";
import { api } from "../lib/ipc";
import type { AuthStatus, CategorySummary, ChannelDetails, ChannelSummary, PagedResult, StreamSummary } from "../lib/generated";
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke: vi.fn() }));
vi.mock("../lib/ipc", async importOriginal => {
  const actual = await importOriginal<typeof import("../lib/ipc")>();
  return { ...actual, api: Object.fromEntries(Object.keys(actual.api).map(key => [key, vi.fn()])) };
});
const signedOut: AuthStatus = { phase: "signed_out", sessionId: null, user: null, authorization: null, error: null, credentialStorage: "test" };
const signedIn: AuthStatus = { ...signedOut, phase: "authenticated", sessionId: "1", user: { id: "viewer", login: "viewer", scopes: ["user:read:follows"], expiresIn: 3600 } };
const stream: StreamSummary = { streamId: "stream-one", broadcasterId: "channel-one", login: "example", displayName: "Example Channel", title: "A live broadcast", categoryId: "game-one", categoryName: "Example Game", previewUrl: null, viewerCount: 1200, language: "en", startedAt: "2026-09-18T08:00:00Z" };
const category: CategorySummary = { id: "game-one", name: "Example Game", imageUrl: null };
const channel: ChannelSummary = { broadcasterId: "channel-one", login: "example", displayName: "Example Channel", imageUrl: null, followedAt: "2026-01-01T00:00:00Z", liveState: "offline", title: null, categoryName: null, language: null };
const details: ChannelDetails = { channel, description: "An example description", stream: null, freshness: "network", ageSeconds: 0, warnings: [] };
const page = <T,>(items: T[], cursor: string | null = null): PagedResult<T> => ({ items, cursor, freshness: "network", ageSeconds: 0, warnings: [] });
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>(done => { resolve = done; }); return { promise, resolve }; }
let root: Root; let container: HTMLDivElement;
const text = () => container.textContent ?? "";
function button(label: string, area = "") {
  const found = [...container.querySelectorAll<HTMLButtonElement>(`${area} button`)].find(b => b.getAttribute("aria-label") === label || b.textContent?.trim() === label);
  if (!found) throw new Error(`Missing button: ${label}`);
  return found;
}
async function click(label: string, area = "") { await act(async () => { button(label, area).click(); }); }
async function type(value: string) {
  const input = container.querySelector<HTMLInputElement>('input[type="search"]')!;
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, value); input.dispatchEvent(new Event("input", { bubbles: true })); });
}
async function render(strict = false) { await act(async () => { root.render(strict ? <StrictMode><App /></StrictMode> : <App />); }); }
beforeEach(() => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true }); vi.useFakeTimers(); vi.resetAllMocks(); localStorage.clear();
  vi.mocked(api.authStatus).mockResolvedValue(signedIn);
  vi.mocked(api.account).mockResolvedValue({ id: "viewer", login: "viewer", displayName: "Viewer", profileImageUrl: null });
  vi.mocked(api.followedStreams).mockResolvedValue(page([])); vi.mocked(api.streams).mockResolvedValue(page([stream]));
  vi.mocked(api.followedChannels).mockResolvedValue(page([channel])); vi.mocked(api.categories).mockResolvedValue(page([category]));
  vi.mocked(api.category).mockResolvedValue({ category, streams: page([stream]) }); vi.mocked(api.channel).mockResolvedValue(details);
  vi.mocked(api.searchChannels).mockResolvedValue(page([channel])); vi.mocked(api.searchCategories).mockResolvedValue(page([category]));
  container = document.createElement("div"); document.body.append(container); root = createRoot(container);
});
afterEach(async () => { await act(async () => root.unmount()); container.remove(); vi.useRealTimers(); });

test("signed-out view offers login without fetching browsing data", async () => {
  vi.mocked(api.authStatus).mockResolvedValue(signedOut); await render();
  expect(text()).toContain("Find what’s live."); expect(button("Connect to Twitch").disabled).toBe(false);
  expect(api.followedStreams).not.toHaveBeenCalled();
});
test("device authorization shows code and permits cancellation", async () => {
  vi.mocked(api.authStatus).mockResolvedValue(signedOut); vi.mocked(api.login).mockResolvedValue({ ...signedOut, phase: "authorizing", authorization: { userCode: "ABCD", verificationUri: "https://www.twitch.tv/activate", expiresIn: 600 } });
  vi.mocked(api.cancel).mockResolvedValue({ ...signedOut, phase: "cancelled" });
  await render(); await click("Connect to Twitch"); expect(text()).toContain("ABCD"); await click("Cancel sign-in"); expect(api.cancel).toHaveBeenCalledOnce();
});
test("authenticated shell selects Following and preserves global account state during navigation", async () => {
  await render(); expect(button("Following").getAttribute("aria-current")).toBe("page"); expect(text()).toContain("Viewer");
  await click("Live"); await click("Categories"); expect(api.account).toHaveBeenCalledOnce(); expect(api.authStatus).toHaveBeenCalledOnce();
});
test("Following handles loading, success and distinct stream IDs", async () => {
  const pending = deferred<PagedResult<StreamSummary>>(); vi.mocked(api.followedStreams).mockReturnValue(pending.promise);
  await render(); expect(text()).toContain("Loading…");
  await act(async () => pending.resolve(page([stream]))); expect(text()).toContain("A live broadcast");
  await click("Open channel Example Channel"); expect(api.channel).toHaveBeenCalledWith({ id: "channel-one", page: { sessionId: "1", cursor: null, refresh: false } });
});
test("Following handles empty results and recoverable network errors", async () => {
  vi.mocked(api.followedStreams).mockRejectedValueOnce({ code: "network", message: "untrusted payload" }); await render();
  expect(text()).toContain("Check your connection"); expect(text()).not.toContain("untrusted payload");
  await click("Retry"); expect(text()).toContain("No followed channels are live right now");
});
test("followed channel list includes offline state and follow date", async () => {
  await render(); await click("All channels"); expect(text()).toContain("Offline"); expect(text()).toContain("Followed"); expect(text()).toContain("Example Channel");
});
test("Live has loading, success and server-error retry states", async () => {
  const pending = deferred<PagedResult<StreamSummary>>(); vi.mocked(api.streams).mockReturnValueOnce(pending.promise); await render(); await click("Live"); expect(text()).toContain("Loading…");
  await act(async () => pending.resolve(page([stream]))); expect(text()).toContain("1.2K viewers");
  vi.mocked(api.streams).mockRejectedValueOnce({ code: "twitch_server" }); await click("Refresh"); expect(text()).toContain("temporarily unavailable"); expect(text()).toContain("Previous results"); await click("Retry");
});
test("categories open their streams and Back restores the category focus", async () => {
  await render(); await click("Categories"); button("Open category Example Game").focus(); await click("Open category Example Game");
  expect(api.category).toHaveBeenCalledWith({ id: "game-one", page: { sessionId: "1", cursor: null, refresh: false } }); expect(text()).toContain("A live broadcast");
  await click("Go back"); expect(document.activeElement).toBe(button("Open category Example Game")); expect(api.categories).toHaveBeenCalledOnce(); expect(text()).toContain("Previous results");
});
test("load more propagates cursors, deduplicates stream IDs and prevents double dispatch", async () => {
  vi.mocked(api.streams).mockResolvedValueOnce(page([stream], "next")); const pending = deferred<PagedResult<StreamSummary>>(); vi.mocked(api.streams).mockReturnValueOnce(pending.promise);
  await render(); await click("Live"); await act(async () => { button("Load more").click(); button("Load more").click(); });
  expect(api.streams).toHaveBeenCalledTimes(2); expect(api.streams).toHaveBeenLastCalledWith({ sessionId: "1", cursor: "next", refresh: false });
  await act(async () => pending.resolve(page([stream, { ...stream, streamId: "stream-two", broadcasterId: "channel-two" }])));
  expect(container.querySelectorAll(".stream-card")).toHaveLength(2);
});
test("repeated cursors terminate pagination rather than looping", async () => {
  vi.mocked(api.streams).mockResolvedValue(page([stream], "same")); await render(); await click("Live"); await click("Load more");
  expect(text()).toContain("incomplete or unexpected"); expect([...container.querySelectorAll("button")].some(b => b.textContent === "Load more")).toBe(false);
});
test("search debounces, ignores empty input and prevents older results replacing a new query", async () => {
  const first = deferred<PagedResult<ChannelSummary>>(); vi.mocked(api.searchChannels).mockReturnValueOnce(first.promise).mockResolvedValueOnce(page([{ ...channel, displayName: "New Result" }]));
  await render(); await click("Search"); expect(api.searchChannels).not.toHaveBeenCalled(); await type("old");
  await act(async () => { await vi.advanceTimersByTimeAsync(349); }); expect(api.searchChannels).not.toHaveBeenCalled();
  await act(async () => { await vi.advanceTimersByTimeAsync(1); }); expect(api.searchChannels).toHaveBeenCalledOnce();
  await type("new"); await act(async () => { await vi.advanceTimersByTimeAsync(350); }); expect(text()).toContain("New Result");
  await act(async () => first.resolve(page([{ ...channel, displayName: "Old Result" }]))); expect(text()).not.toContain("Old Result");
  await type("   "); expect(text()).toContain("What are you looking for?"); expect(api.searchChannels).toHaveBeenCalledTimes(2);
});
test("category search uses its own result type and pagination", async () => {
  vi.mocked(api.searchCategories).mockResolvedValueOnce(page([category], "category-next")).mockResolvedValueOnce(page([]));
  await render(); await click("Search"); await type("example"); await click("Categories", ".tabs"); await act(async () => { await vi.advanceTimersByTimeAsync(350); });
  expect(container.querySelector(".category-card")).not.toBeNull(); await click("Load more"); expect(api.searchCategories).toHaveBeenLastCalledWith({ query: "example", page: { sessionId: "1", cursor: "category-next", refresh: false } });
});
test("online channel details include viewers, category and start time", async () => {
  vi.mocked(api.channel).mockResolvedValue({ ...details, channel: { ...channel, liveState: "live" }, stream });
  await render(); await click("Live"); await click("Open channel Example Channel"); expect(text()).toContain("LIVE NOW"); expect(text()).toContain("Started"); expect(text()).toContain("An example description");
});
test("offline and unavailable channel states are distinct", async () => {
  await render(); await click("Live"); await click("Open channel Example Channel"); expect(text()).toContain("Offline — no current live stream");
  vi.mocked(api.channel).mockResolvedValue({ ...details, channel: { ...channel, liveState: "unknown" }, warnings: ["network"] }); await click("Refresh");
  expect(text()).toContain("Live status unavailable"); expect(text()).not.toContain("Offline —"); expect(text()).toContain("Check your connection");
});
test("logout clears the workspace immediately and rejects late account and browse responses", async () => {
  const pending = deferred<PagedResult<StreamSummary>>(); const account = deferred<Awaited<ReturnType<typeof api.account>>>(); const logout = deferred<AuthStatus>();
  vi.mocked(api.followedStreams).mockReturnValue(pending.promise); vi.mocked(api.account).mockReturnValue(account.promise); vi.mocked(api.logout).mockReturnValue(logout.promise);
  await render(); await click("Sign out"); expect(container.querySelector(".workspace")).toBeNull();
  await act(async () => { pending.resolve(page([stream])); account.resolve({ id: "viewer", login: "viewer", displayName: "OLD IDENTITY", profileImageUrl: null }); logout.resolve(signedOut); });
  expect(text()).not.toContain("OLD IDENTITY"); expect(text()).not.toContain("Example Channel"); expect(text()).toContain("Connect to Twitch");
});
test("an account/session change discards navigation snapshots even for the same user", async () => {
  vi.mocked(api.followedStreams).mockResolvedValueOnce(page([stream])).mockResolvedValueOnce(page([])); await render();
  vi.mocked(api.authStatus).mockResolvedValue({ ...signedIn, sessionId: "2" }); await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
  expect(text()).not.toContain("Example Channel"); expect(api.followedStreams).toHaveBeenLastCalledWith({ sessionId: "2", cursor: null, refresh: false });
});
test("navigation and items use native focusable buttons and search submits from keyboard", async () => {
  await render(); button("Live").focus(); expect(document.activeElement).toBe(button("Live")); await click("Live");
  const item = button("Open channel Example Channel"); expect(item.tagName).toBe("BUTTON"); item.focus(); expect(document.activeElement).toBe(item);
  await click("Search"); await type("keyboard"); await act(async () => { container.querySelector("form")!.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })); }); expect(api.searchChannels).toHaveBeenCalledOnce();
});
test("StrictMode does not dispatch duplicate initial browsing queries", async () => { await render(true); expect(api.followedStreams).toHaveBeenCalledOnce(); });
test("failed images retain their aspect-ratio placeholder", async () => {
  vi.mocked(api.streams).mockResolvedValue(page([{ ...stream, previewUrl: "https://static-cdn.jtvnw.net/test.jpg" }])); await render(); await click("Live");
  await act(async () => { container.querySelector(".media img")!.dispatchEvent(new Event("error")); }); expect(text()).toContain("Image unavailable"); expect(container.querySelector(".media img")).toBeNull();
});

test("pagination stops after ten explicit pages and refresh resets the bound", async () => {
  let next = 0;
  vi.mocked(api.streams).mockImplementation(async () => page([{ ...stream, streamId: `stream-${++next}` }], `page-${next}`));
  await render(); await click("Live");
  for (let n = 0; n < 9; n++) await click("Load more");
  expect(api.streams).toHaveBeenCalledTimes(10); expect(text()).toContain("Showing up to 300 items");
  expect([...container.querySelectorAll("button")].some(b => b.textContent === "Load more")).toBe(false);
  await click("Refresh"); expect(container.querySelectorAll(".stream-card")).toHaveLength(1); expect(button("Load more")).toBeDefined();
});
test("authentication loss removes retained browsing results", async () => {
  vi.mocked(api.followedStreams).mockResolvedValueOnce(page([stream])).mockRejectedValueOnce({ code: "unauthenticated" });
  await render(); await click("Refresh"); expect(container.querySelector(".workspace")).toBeNull(); expect(text()).not.toContain("Example Channel");
});

test("Back restores the historical search query, cursor, scroll and focus", async () => {
  const foo = { ...channel, broadcasterId: "foo", displayName: "Foo" };
  const second = { ...channel, broadcasterId: "foo-two", displayName: "Foo two" };
  vi.mocked(api.searchChannels).mockImplementation(async request => request.query === "foo"
    ? request.page.cursor ? page([second], "foo-third") : page([foo], "foo-next")
    : page([{ ...channel, broadcasterId: "bar", displayName: "Bar" }], "bar-next"));
  await render(); await click("Search"); await type("foo");
  await act(async () => { await vi.advanceTimersByTimeAsync(350); }); await click("Load more");
  container.querySelector("main")!.scrollTop = 240; button("Open channel Foo two").focus(); await click("Open channel Foo two");
  await click("Search"); await type("bar"); await act(async () => { await vi.advanceTimersByTimeAsync(350); });
  expect(text()).toContain("Bar"); await click("Go back"); await click("Go back");
  expect(container.querySelector<HTMLInputElement>('input[type="search"]')!.value).toBe("foo");
  expect(container.querySelectorAll(".channel-row")).toHaveLength(2); expect(text()).not.toContain("Bar");
  expect(container.querySelector("main")!.scrollTop).toBe(240); expect(document.activeElement).toBe(button("Open channel Foo two"));
  expect(api.searchChannels).toHaveBeenCalledTimes(3);
  await click("Load more"); expect(api.searchChannels).toHaveBeenLastCalledWith({ query: "foo", page: { sessionId: "1", cursor: "foo-third", refresh: false } });
});

test("Back restores the historical search result type and its pagination", async () => {
  vi.mocked(api.searchCategories).mockResolvedValue(page([category], "category-next"));
  vi.mocked(api.searchChannels).mockResolvedValue(page([channel], "channel-next"));
  await render(); await click("Search"); await type("foo"); await click("Categories", ".tabs");
  await act(async () => { await vi.advanceTimersByTimeAsync(350); });
  button("Open category Example Game").focus(); await click("Open category Example Game");
  await click("Search"); await click("Channels", ".tabs"); await type("bar"); await act(async () => { await vi.advanceTimersByTimeAsync(350); });
  await click("Go back"); await click("Go back");
  expect(button("Categories", ".tabs").getAttribute("aria-pressed")).toBe("true");
  expect(container.querySelector<HTMLInputElement>('input[type="search"]')!.value).toBe("foo");
  expect(document.activeElement).toBe(button("Open category Example Game")); expect(container.querySelector(".channel-row")).toBeNull();
  await click("Load more"); expect(api.searchCategories).toHaveBeenLastCalledWith({ query: "foo", page: { sessionId: "1", cursor: "category-next", refresh: false } });
});

test("Back restores the Following tab belonging to that visit", async () => {
  await render(); await click("All channels"); button("Open channel Example Channel").focus(); await click("Open channel Example Channel");
  await click("Following"); await click("Live streams"); await click("Go back"); await click("Go back");
  expect(button("All channels").getAttribute("aria-pressed")).toBe("true");
  expect(document.activeElement).toBe(button("Open channel Example Channel")); expect(api.followedChannels).toHaveBeenCalledOnce();
});

test("a new session clears historical search and Following state", async () => {
  await render(); await click("All channels"); await click("Search"); await type("old account"); await click("Categories", ".tabs");
  await act(async () => { await vi.advanceTimersByTimeAsync(350); }); await click("Open category Example Game");
  vi.mocked(api.authStatus).mockResolvedValue({ ...signedIn, sessionId: "2" });
  await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
  expect(button("Go back").disabled).toBe(true); expect(button("Live streams").getAttribute("aria-pressed")).toBe("true");
  await click("Search"); expect(container.querySelector<HTMLInputElement>('input[type="search"]')!.value).toBe("");
  expect(button("Channels", ".tabs").getAttribute("aria-pressed")).toBe("true"); expect(container.querySelector(".category-card")).toBeNull();
});

test("final pagination focuses the first newly appended result", async () => {
  const pending = deferred<PagedResult<StreamSummary>>();
  vi.mocked(api.streams).mockResolvedValueOnce(page([stream], "next")).mockReturnValueOnce(pending.promise);
  await render(); await click("Live"); const more = button("Load more"); more.focus(); await click("Load more");
  expect(document.activeElement).toBe(more); expect(text()).toContain("Loading more information");
  await act(async () => pending.resolve(page([stream, { ...stream, streamId: "new-result" }])));
  expect(more.isConnected).toBe(false); expect(document.activeElement).toBe(container.querySelector('[data-focus="stream:new-result"]'));
});

test("an empty final page focuses the stable accessible results target", async () => {
  const pending = deferred<PagedResult<StreamSummary>>();
  vi.mocked(api.streams).mockResolvedValueOnce(page([stream], "next")).mockReturnValueOnce(pending.promise);
  await render(); await click("Live"); button("Load more").focus(); await click("Load more");
  expect(text()).toContain("Loading more information");
  await act(async () => pending.resolve(page([])));
  const results = container.querySelector('[role="region"][aria-label="Results"]');
  expect(results).not.toBeNull(); expect(document.activeElement).toBe(results); expect(results!.querySelectorAll(".stream-card")).toHaveLength(1);
});

test("reaching 300 items focuses a result from the final permitted page", async () => {
  let number = 0;
  const items = (offset: number) => Array.from({ length: 30 }, (_, i) => ({ ...stream, streamId: `item-${offset + i}` }));
  vi.mocked(api.streams).mockImplementation(async () => page(items(30 * number), `page-${++number}`));
  await render(); await click("Live"); for (let n = 0; n < 8; n++) await click("Load more");
  const pending = deferred<PagedResult<StreamSummary>>(); vi.mocked(api.streams).mockReturnValueOnce(pending.promise);
  button("Load more").focus(); await click("Load more"); expect(container.querySelectorAll(".stream-card")).toHaveLength(270);
  await act(async () => pending.resolve(page(items(270), "beyond-limit")));
  expect(container.querySelectorAll(".stream-card")).toHaveLength(300); expect(text()).toContain("Showing up to 300 items");
  expect(container.querySelector(".load-more")).toBeNull(); expect(document.activeElement).toBe(container.querySelector('[data-focus="stream:item-270"]'));
});

test("a pending pagination error preserves control focus, results and retry cursor", async () => {
  let reject!: (error: unknown) => void;
  const pending = new Promise<PagedResult<StreamSummary>>((_, fail) => { reject = fail; });
  vi.mocked(api.streams).mockResolvedValueOnce(page([stream], "next")).mockReturnValueOnce(pending);
  await render(); await click("Live"); const more = button("Load more"); more.focus(); await click("Load more");
  expect(text()).toContain("Loading more information"); await act(async () => reject({ code: "network" }));
  expect(document.activeElement).toBe(more); expect(container.querySelectorAll(".stream-card")).toHaveLength(1); expect(text()).toContain("Check your connection");
  button("Retry").focus(); await click("Retry"); expect(api.streams).toHaveBeenLastCalledWith({ sessionId: "1", cursor: "next", refresh: false });
});

test.each(["another control", "the document body"])("final pagination does not steal focus after moving to %s", async destination => {
  const pending = deferred<PagedResult<StreamSummary>>();
  vi.mocked(api.streams).mockResolvedValueOnce(page([stream], "next")).mockReturnValueOnce(pending.promise);
  await render(); await click("Live"); const more = button("Load more"); more.focus(); await click("Load more");
  await act(async () => { button("Settings").focus(); if (destination === "the document body") button("Settings").blur(); });
  const target = document.activeElement; expect(target).not.toBe(more); expect(text()).toContain("Loading more information");
  await act(async () => pending.resolve(page([{ ...stream, streamId: "new-result" }])));
  expect(document.activeElement).toBe(target);
});

test("final pagination does not move focus when Load more was not focused", async () => {
  const pending = deferred<PagedResult<StreamSummary>>();
  vi.mocked(api.streams).mockResolvedValueOnce(page([stream], "next")).mockReturnValueOnce(pending.promise);
  await render(); await click("Live"); const target = button("Settings"); target.focus(); await click("Load more");
  expect(text()).toContain("Loading more information"); await act(async () => pending.resolve(page([{ ...stream, streamId: "new-result" }])));
  expect(document.activeElement).toBe(target);
});
