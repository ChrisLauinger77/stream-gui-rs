// @vitest-environment jsdom
import { act, StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { App } from "./App";
import { Media } from "../browse/components";
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
  Object.defineProperty(navigator, "platform", { configurable: true, value: "Linux x86_64" });
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true }); vi.useFakeTimers(); vi.resetAllMocks(); localStorage.clear();
  vi.mocked(api.sessions).mockResolvedValue([]);
  vi.mocked(api.playbackSettings).mockResolvedValue({ discoveryLanguage: null, lowLatency: false, textScale: "100", background: { monitoringEnabled: false, notificationsEnabled: false, closeToBackground: false, intervalSeconds: 60 }, theme: "system", automaticChat: false, streamlinkPath: null, player: { mode: "default", executable: null, arguments: [] }, defaultQuality: "source" });
  vi.mocked(api.channelSettings).mockImplementation(async broadcasterId => ({ broadcasterId, overrides: { lowLatency: null, notifications: null, quality: null, automaticChat: null }, defaultQuality: "source", defaultAutomaticChat: false, defaultLowLatency: false, defaultNotifications: false, effectiveNotifications: false, effective: { lowLatency: false, streamlinkPath: null, player: { mode: "default", executable: null, arguments: [] }, quality: "source", automaticChat: false } }));
  vi.mocked(api.authStatus).mockResolvedValue(signedIn);
  vi.mocked(api.account).mockResolvedValue({ id: "viewer", login: "viewer", displayName: "Viewer", profileImageUrl: null });
  vi.mocked(api.followedStreams).mockResolvedValue(page([])); vi.mocked(api.streams).mockResolvedValue(page([stream]));
  vi.mocked(api.followedChannels).mockResolvedValue(page([channel])); vi.mocked(api.categories).mockResolvedValue(page([category]));
  vi.mocked(api.category).mockResolvedValue({ category, streams: page([stream]) }); vi.mocked(api.channel).mockResolvedValue(details);
  vi.mocked(api.searchChannels).mockResolvedValue(page([channel])); vi.mocked(api.searchCategories).mockResolvedValue(page([category]));
  container = document.createElement("div"); document.body.append(container); root = createRoot(container);
});
afterEach(async () => { await act(async () => root.unmount()); container.remove(); vi.useRealTimers(); vi.unstubAllGlobals(); });

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
  expect(api.category).toHaveBeenCalledWith({ id: "game-one", page: { page: { sessionId: "1", cursor: null, refresh: false }, language: null } }); expect(text()).toContain("A live broadcast");
  await click("Go back"); expect(document.activeElement).toBe(button("Open category Example Game")); expect(api.categories).toHaveBeenCalledOnce(); expect(text()).toContain("Previous results");
});
test("load more propagates cursors, deduplicates stream IDs and prevents double dispatch", async () => {
  vi.mocked(api.streams).mockResolvedValueOnce(page([stream], "next")); const pending = deferred<PagedResult<StreamSummary>>(); vi.mocked(api.streams).mockReturnValueOnce(pending.promise);
  await render(); await click("Live"); await act(async () => { button("Load more").click(); button("Load more").click(); });
  expect(api.streams).toHaveBeenCalledTimes(2); expect(api.streams).toHaveBeenLastCalledWith({ page: { sessionId: "1", cursor: "next", refresh: false }, language: null });
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
  button("Retry").focus(); await click("Retry"); expect(api.streams).toHaveBeenLastCalledWith({ page: { sessionId: "1", cursor: "next", refresh: false }, language: null });
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

test("successful manual refresh retries the same failed image URL without resetting loaded images", async () => {
  const items = [{ ...stream, previewUrl: "https://static-cdn.jtvnw.net/failed.jpg" }, { ...stream, streamId: "loaded", previewUrl: "https://static-cdn.jtvnw.net/loaded.jpg" }];
  vi.mocked(api.streams).mockResolvedValueOnce(page(items)); await render(); await click("Live");
  const [failed, loaded] = container.querySelectorAll<HTMLImageElement>(".media img");
  await act(async () => { failed.dispatchEvent(new Event("error")); loaded.dispatchEvent(new Event("load")); });
  const pending = deferred<PagedResult<StreamSummary>>(); vi.mocked(api.streams).mockReturnValueOnce(pending.promise); await click("Refresh");
  expect(container.querySelectorAll(".media img")).toHaveLength(1); expect(loaded.parentElement!.classList.contains("loaded")).toBe(true);
  await act(async () => pending.resolve(page(items)));
  expect(container.querySelectorAll(".media img")).toHaveLength(2);
  expect(container.querySelector<HTMLImageElement>('[data-focus="stream:stream-one"] img')!.src).toBe(items[0].previewUrl);
  expect(container.querySelector('[data-focus="stream:loaded"] img')).toBe(loaded); expect(loaded.parentElement!.classList.contains("loaded")).toBe(true);
});

test("ordinary image rerenders preserve the failed placeholder without retrying", async () => {
  const src = "https://static-cdn.jtvnw.net/failed.jpg";
  await act(async () => root.render(<Media src={src} />)); await act(async () => container.querySelector("img")!.dispatchEvent(new Event("error")));
  for (let n = 0; n < 3; n++) await act(async () => root.render(<Media src={src} />));
  expect(container.querySelector("img")).toBeNull(); expect(container.querySelector(".media.preview.failed")).not.toBeNull(); expect(text()).toContain("Image unavailable");
});

test("changing image src retries naturally with the original layout shape", async () => {
  await act(async () => root.render(<Media src="https://static-cdn.jtvnw.net/old.jpg" shape="artwork" />));
  await act(async () => container.querySelector("img")!.dispatchEvent(new Event("error")));
  await act(async () => root.render(<Media src="https://static-cdn.jtvnw.net/new.jpg" shape="artwork" />));
  expect(container.querySelector<HTMLImageElement>("img")!.src).toBe("https://static-cdn.jtvnw.net/new.jpg"); expect(container.querySelector(".media.artwork.loading")).not.toBeNull();
});

test("repeated image failures wait for another successful refresh", async () => {
  const items = [{ ...stream, previewUrl: "https://static-cdn.jtvnw.net/failed.jpg" }];
  vi.mocked(api.streams).mockResolvedValue(page(items)); await render(); await click("Live");
  const fail = async () => { await act(async () => container.querySelector(".media img")!.dispatchEvent(new Event("error"))); };
  await fail(); vi.mocked(api.streams).mockRejectedValueOnce({ code: "network" }); await click("Refresh"); expect(container.querySelector(".media img")).toBeNull();
  await click("Retry"); expect(container.querySelectorAll(".media img")).toHaveLength(1); await fail();
  await click("Settings"); await click("Appearance", ".settings-nav");
  await editControl("Appearance", "light", "select");
  vi.mocked(api.savePlaybackSettings).mockResolvedValue({ ...playbackSettings, theme: "light" });
  await click("Save settings");
  await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
  expect(container.querySelector(".media img")).toBeNull(); expect(api.streams).toHaveBeenCalledTimes(3);
  await click("Refresh"); expect(container.querySelectorAll(".media img")).toHaveLength(1); await fail();
  expect(container.querySelector(".media img")).toBeNull(); expect(container.querySelector(".media.preview.failed")).not.toBeNull();
});

test.each(["followed channels", "categories", "category details", "channel details", "channel search", "category search"])("successful refresh retries failed images in %s", async view => {
  const imageUrl = "https://static-cdn.jtvnw.net/image.jpg";
  const withImage = { ...channel, imageUrl }; const categoryImage = { ...category, imageUrl };
  vi.mocked(api.followedChannels).mockResolvedValue(page([withImage])); vi.mocked(api.categories).mockResolvedValue(page([categoryImage]));
  vi.mocked(api.category).mockResolvedValue({ category: categoryImage, streams: page([{ ...stream, previewUrl: imageUrl }]) });
  vi.mocked(api.channel).mockResolvedValue({ ...details, channel: withImage, stream: { ...stream, previewUrl: imageUrl } });
  vi.mocked(api.searchChannels).mockResolvedValue(page([withImage])); vi.mocked(api.searchCategories).mockResolvedValue(page([categoryImage]));
  await render();
  if (view === "followed channels") await click("All channels");
  else if (view === "categories" || view === "category details") { await click("Categories"); if (view === "category details") await click("Open category Example Game"); }
  else if (view === "channel details") { await click("Live"); await click("Open channel Example Channel"); }
  else { await click("Search"); await type("example"); if (view === "category search") await click("Categories", ".tabs"); await act(async () => { await vi.advanceTimersByTimeAsync(350); }); }
  const images = [...container.querySelectorAll(".media img")]; expect(images.length).toBeGreaterThan(0);
  await act(async () => { images.forEach(img => img.dispatchEvent(new Event("error"))); }); expect(container.querySelectorAll(".media img")).toHaveLength(0);
  await click("Refresh"); expect(container.querySelectorAll(".media img")).toHaveLength(images.length);
  expect([...container.querySelectorAll<HTMLImageElement>(".media img")].every(img => img.src === imageUrl)).toBe(true);
});

// Phase 3 uses the same application and navigation surfaces as the Phase 2 tests.
const playing = (id = "play-one", broadcasterId = "channel-one"): import("../lib/generated").SessionSnapshot => ({
  id, generation: 1, restarting: false, effectiveSettings: null, chatError: null, stream: { streamId: "stream-one", broadcasterId, login: "example", displayName: id === "play-one" ? "Example Channel" : "Second Channel", title: "A live broadcast", category: "Example Game" },
  qualityPolicy: "source", startedAt: 100, endedAt: null, failure: null, phase: "running", pid: 123,
  url: "https://www.twitch.tv/example", quality: "best", exitCode: null, stopRequested: false,
  logs: [{ sequence: 1, source: "stderr", text: "Synthetic diagnostic warning" }], droppedLogEntries: 5,
});
const playbackSettings: import("../lib/generated").Settings = { discoveryLanguage: null, lowLatency: false, textScale: "100", background: { monitoringEnabled: false, notificationsEnabled: false, closeToBackground: false, intervalSeconds: 60 }, theme: "system", automaticChat: false, streamlinkPath: null, player: { mode: "default", executable: null, arguments: [] }, defaultQuality: "source" };
async function editControl(label: string, value: string, kind: "input" | "select" = "input") {
  const control = [...container.querySelectorAll<HTMLInputElement | HTMLSelectElement>(kind)].find(el => el.labels?.[0]?.textContent?.startsWith(label));
  if (!control) throw new Error(`Missing control: ${label}`);
  await act(async () => {
    Object.getOwnPropertyDescriptor(kind === "input" ? HTMLInputElement.prototype : HTMLSelectElement.prototype, "value")!.set!.call(control, value);
    control.dispatchEvent(new Event(kind === "input" ? "input" : "change", { bubbles: true }));
  });
}
test.each(["Following", "Live", "Category"])("Watch launches trusted broadcaster identity from %s and keeps browsing mounted", async location => {
  vi.mocked(api.followedStreams).mockResolvedValue(page([stream]));
  vi.mocked(api.launch).mockResolvedValue(playing());
  await render();
  if (location === "Live") await click("Live");
  if (location === "Category") { await click("Categories"); await click("Open category Example Game"); }
  await click("Watch Example Channel");
  expect(api.launch).toHaveBeenCalledWith({ authSessionId: "1", broadcasterId: "channel-one", quality: null });
  expect(container.querySelector(".workspace")).not.toBeNull();
  expect(container.querySelector(".watching-panel")?.textContent).toContain("running");
  expect(text()).toContain("Streamlink started");
});
test("live search and Channel details offer Watch but offline and unknown channels do not", async () => {
  vi.mocked(api.searchChannels).mockResolvedValue(page([{ ...channel, liveState: "live" }, { ...channel, broadcasterId: "offline", displayName: "Offline" }, { ...channel, broadcasterId: "unknown", displayName: "Unknown", liveState: "unknown" }]));
  vi.mocked(api.launch).mockResolvedValue(playing());
  vi.mocked(api.channel).mockResolvedValue({ ...details, channel: { ...channel, liveState: "live" }, stream });
  await render(); await click("Search"); await type("example"); await act(async () => { await vi.advanceTimersByTimeAsync(350); });
  expect(container.querySelectorAll(".watch-button")).toHaveLength(1);
  await click("Watch Example Channel"); await click("Open channel Example Channel");
  await click("Watch Example Channel"); expect(api.launch).toHaveBeenCalledTimes(2);
  vi.mocked(api.channel).mockResolvedValue(details); await click("Refresh");
  expect(container.querySelector(".watch-button")).toBeNull();
});
test("a keyboard-focusable native Watch button supports keyboard activation and blocks repeat pending launches", async () => {
  const launch = deferred<ReturnType<typeof playing>>(); vi.mocked(api.launch).mockReturnValue(launch.promise);
  await render(); await click("Live");
  const watch = button("Watch Example Channel"); watch.focus();
  expect(document.activeElement).toBe(watch); expect(watch.tagName).toBe("BUTTON"); expect(watch.tabIndex).toBe(0);
  // jsdom has no native key default actions; a keyboard-generated click has detail 0.
  await act(async () => { watch.dispatchEvent(new MouseEvent("click", { bubbles: true, detail: 0 })); watch.click(); });
  expect(api.launch).toHaveBeenCalledOnce(); expect(watch.disabled).toBe(true);
  await act(async () => { launch.resolve(playing()); }); expect(watch.disabled).toBe(false);
});
test.each([
  ["streamlink_not_found", "Streamlink was not found"], ["player_not_found", "selected player was not found"],
  ["unsupported_streamlink", "Streamlink 8 or newer"], ["stream_offline", "no longer live"],
])("launch failure %s is useful without displaying raw backend errors", async (code, message) => {
  vi.mocked(api.launch).mockRejectedValue({ code, message: "PRIVATE BACKEND PAYLOAD" });
  await render(); await click("Live"); await click("Watch Example Channel");
  expect(text()).toContain(message); expect(text()).not.toContain("PRIVATE BACKEND PAYLOAD");
  expect(container.querySelectorAll(".session")).toHaveLength(0); expect(button("Watch Example Channel").disabled).toBe(false);
});
test("Watching reconstructs multiple sessions and stops only the selected one", async () => {
  const first = playing(); const second = playing("play-two", "channel-two");
  vi.mocked(api.sessions).mockResolvedValue([first, second]);
  vi.mocked(api.stop).mockResolvedValue({ ...second, phase: "exited", stopRequested: true, endedAt: 160 });
  await render(); await click("Watching");
  expect(container.querySelectorAll(".session")).toHaveLength(2);
  expect(button("Watching").textContent).toContain("2");
  await click("Stop", '[aria-label="Playback Second Channel"]');
  expect(api.stop).toHaveBeenCalledWith("play-two");
  expect(container.querySelector('[aria-label="Playback Example Channel"] .session-heading strong')?.textContent).toBe("running");
  expect(button("Stop", '[aria-label="Playback Second Channel"]').disabled).toBe(true);
  expect(button("Watching").textContent).toContain("1");
});
test("quality changes require an explicit restart and send the observed generation", async () => {
  vi.mocked(api.sessions).mockResolvedValue([playing()]);
  vi.mocked(api.restart).mockResolvedValue({ ...playing(), generation: 2, qualityPolicy: "audio", quality: "audio,audio_only" });
  await render(); await click("Watching"); await editControl("Restart quality", "audio", "select");
  expect(api.restart).not.toHaveBeenCalled();
  await click("Restart");
  expect(api.restart).toHaveBeenCalledWith({ sessionId: "play-one", generation: 1, quality: "audio" });
  expect(container.querySelector(".session")?.textContent).toContain("Audio");
});
test("Stop remains usable while Restart is pending and unrelated sessions remain usable", async () => {
  const pending = deferred<ReturnType<typeof playing>>();
  vi.mocked(api.sessions).mockResolvedValue([playing(), playing("play-two")]);
  vi.mocked(api.restart).mockReturnValue(pending.promise);
  vi.mocked(api.stop).mockResolvedValue({ ...playing(), generation: 2, phase: "exited", endedAt: 200 });
  await render(); await click("Watching"); await click("Restart", '[aria-label="Playback Example Channel"]');
  expect(button("Restart", '[aria-label="Playback Example Channel"]').disabled).toBe(true);
  expect(button("Stop", '[aria-label="Playback Example Channel"]').disabled).toBe(false);
  expect(button("Restart", '[aria-label="Playback Second Channel"]').disabled).toBe(false);
  await click("Stop", '[aria-label="Playback Example Channel"]'); expect(api.stop).toHaveBeenCalledWith("play-one");
  await act(async () => { pending.resolve({ ...playing(), generation: 2, phase: "exited", endedAt: 200 }); });
});
test("session diagnostics are collapsed, scoped and show bounded-log truncation", async () => {
  vi.mocked(api.sessions).mockResolvedValue([playing()]); await render(); await click("Watching");
  const diagnostics = container.querySelector<HTMLDetailsElement>(".session-diagnostics")!;
  expect(diagnostics.open).toBe(false);
  await act(async () => { diagnostics.querySelector("summary")!.click(); });
  expect(diagnostics.open).toBe(true);
  expect(diagnostics.querySelector('pre[aria-label="Diagnostics for session play-one"]')?.textContent).toContain("[stderr] Synthetic diagnostic warning");
  expect(diagnostics.textContent).toContain("5 older log entries discarded");
});
test("a late pre-launch session poll cannot erase the resulting active session", async () => {
  const old = deferred<ReturnType<typeof playing>[]>(); vi.mocked(api.sessions).mockReturnValueOnce(old.promise);
  vi.mocked(api.launch).mockResolvedValue(playing()); await render(); await click("Live"); await click("Watch Example Channel");
  await act(async () => { old.resolve([]); });
  expect(container.querySelectorAll(".session")).toHaveLength(1);
  expect(button("Watching").textContent).toContain("1");
});
test("frontend remount and logout recover and preserve Rust playback without relaunching or stopping", async () => {
  vi.mocked(api.sessions).mockResolvedValue([playing()]); await render(); await click("Watching");
  await act(async () => { root.render(null); }); await render(); await click("Watching");
  expect(container.querySelectorAll(".session")).toHaveLength(1);
  vi.mocked(api.logout).mockResolvedValue(signedOut); vi.mocked(api.authStatus).mockResolvedValue(signedOut);
  await click("Sign out");
  expect(container.querySelector(".workspace")).toBeNull(); expect(container.querySelectorAll(".session")).toHaveLength(1);
  expect(api.launch).not.toHaveBeenCalled(); expect(api.stop).not.toHaveBeenCalled();
});
test("settings save literal player arguments and restore persisted choices when reopened", async () => {
  await render(); await click("Settings"); await editControl("Player", "custom", "select");
  await editControl("Player executable", "/Applications/My Player/日本語");
  await editControl("Default quality", "low", "select"); await click("Add argument");
  await editControl("Player argument 1", `spaces 'quotes' {playerinput} $literal`); await click("Add argument");
  const saved = { ...playbackSettings, player: { mode: "custom" as const, executable: "/Applications/My Player/日本語", arguments: ["spaces 'quotes' {playerinput} $literal", ""] }, defaultQuality: "low" as const };
  vi.mocked(api.savePlaybackSettings).mockResolvedValue(saved);
  await click("Save settings"); expect(api.savePlaybackSettings).toHaveBeenCalledWith(saved);
  expect(text()).toContain("Settings saved"); await click("Settings"); await click("Settings");
  expect(container.querySelector<HTMLInputElement>('input[aria-label="Player executable"]')?.value).toBe(saved.player.executable);
  expect(container.querySelectorAll(".argument-row")).toHaveLength(2);
});
test("settings probe reports native path and version, and player discovery reports missing players", async () => {
  vi.mocked(api.probe).mockResolvedValue({ executable: "/opt/homebrew/bin/streamlink", version: "8.6.1" });
  vi.mocked(api.discoverPlayers).mockResolvedValue({ mpv: "/opt/homebrew/bin/mpv", vlc: null });
  await render(); await click("Settings"); await click("Test and save Streamlink path");
  expect(api.probe).toHaveBeenCalledWith({ customPath: null }); expect(text()).toContain("8.6.1"); expect(text()).toContain("/opt/homebrew/bin/streamlink");
  await click("Find installed players"); expect(text()).toContain("mpv: /opt/homebrew/bin/mpv"); expect(text()).toContain("VLC: Not found");
});
test("invalid player settings fail visibly without replacing saved settings", async () => {
  vi.mocked(api.savePlaybackSettings).mockRejectedValue({ code: "player_not_found", message: "PRIVATE" });
  await render(); await click("Settings"); await editControl("Player", "mpv", "select"); await click("Save settings");
  expect(text()).toContain("selected player was not found"); expect(text()).not.toContain("PRIVATE");
  await click("Settings"); await click("Settings");
  expect([...container.querySelectorAll<HTMLSelectElement>("select")].find(el => el.labels?.[0]?.textContent?.startsWith("Player"))?.value).toBe("default");
});

async function keypress(key: string, modifiers: KeyboardEventInit = { ctrlKey: true }, target: EventTarget = window) {
  const event = new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...modifiers });
  await act(async () => { target.dispatchEvent(event); });
  return event;
}

test("Settings sections preserve drafts, cancel them explicitly, and save through Rust", async () => {
  await render(); await click("Settings");
  expect(document.activeElement).toBe(container.querySelector(".settings-header h2"));
  expect(container.querySelector('.settings-nav [aria-current="page"]')?.textContent).toBe("Playback");
  await editControl("Default quality", "high", "select");
  await click("Player", ".settings-nav"); await editControl("Player", "mpv", "select");
  await click("Playback", ".settings-nav");
  expect(container.querySelector<HTMLSelectElement>('label select')?.value).toBe("high");
  expect(api.savePlaybackSettings).not.toHaveBeenCalled();
  await click("Cancel changes");
  const saved = { ...playbackSettings, defaultQuality: "low" as const };
  await editControl("Default quality", "low", "select");
  vi.mocked(api.savePlaybackSettings).mockResolvedValue(saved);
  await click("Save settings"); expect(api.savePlaybackSettings).toHaveBeenCalledWith(saved);
  expect(container.querySelectorAll(".settings-content .setting-group:not([hidden])")).toHaveLength(1);
  await click("Shortcuts", ".settings-nav"); expect(text()).toContain("Ctrl+K");
});

test("settings loading waits for the backend and duplicate saves are guarded", async () => {
  const loading = deferred<typeof playbackSettings>(); vi.mocked(api.playbackSettings).mockReturnValue(loading.promise);
  await render(); await click("Settings"); expect(text()).toContain("Loading settings");
  await act(async () => loading.resolve(playbackSettings));
  const saving = deferred<typeof playbackSettings>(); vi.mocked(api.savePlaybackSettings).mockReturnValue(saving.promise);
  await act(async () => { button("Save settings").click(); button("Save settings").click(); });
  expect(api.savePlaybackSettings).toHaveBeenCalledOnce();
  await act(async () => saving.resolve(playbackSettings)); expect(text()).toContain("Settings saved");
});

function channelPreferences(broadcasterId = "channel-one", quality: "source" | "high" | "low" = "source") {
  return { broadcasterId, overrides: { lowLatency: null, notifications: null, quality: null, automaticChat: null }, defaultQuality: quality, defaultAutomaticChat: false, defaultLowLatency: false, defaultNotifications: false, effectiveNotifications: false,
    effective: { lowLatency: false, streamlinkPath: null, player: playbackSettings.player, quality, automaticChat: false } };
}

test("channel preferences display Rust defaults, save overrides, and return to inheritance", async () => {
  vi.mocked(api.channelSettings).mockResolvedValue(channelPreferences("channel-one", "high"));
  await render(); await click("Live"); await click("Open channel Example Channel");
  expect(text()).toContain("Use global default (High");
  await editControl("Channel quality", "low", "select"); await editControl("Channel browser chat", "on", "select");
  const saved = { ...channelPreferences("channel-one", "high"), overrides: { lowLatency: null, notifications: null, quality: "low" as const, automaticChat: true }, effective: { ...channelPreferences().effective, quality: "low" as const, automaticChat: true } };
  vi.mocked(api.saveChannelSettings).mockResolvedValue(saved);
  await click("Save channel settings");
  expect(api.saveChannelSettings).toHaveBeenCalledWith({ broadcasterId: "channel-one", overrides: { lowLatency: null, notifications: null, quality: "low", automaticChat: true } });
  expect(text()).toContain("Saved effective quality: Low");
  await editControl("Channel quality", "inherit", "select"); await editControl("Channel browser chat", "inherit", "select");
  vi.mocked(api.saveChannelSettings).mockResolvedValue(channelPreferences("channel-one", "high"));
  await click("Save channel settings");
  expect(api.saveChannelSettings).toHaveBeenLastCalledWith({ broadcasterId: "channel-one", overrides: { lowLatency: null, notifications: null, quality: null, automaticChat: null } });
  expect(text()).toContain("Saved effective quality: High");
  expect(api.launch).not.toHaveBeenCalled(); expect(api.restart).not.toHaveBeenCalled();
});

test("channel settings errors preserve the draft and never expose backend messages", async () => {
  await render(); await click("Live"); await click("Open channel Example Channel");
  await editControl("Channel quality", "audio", "select");
  vi.mocked(api.saveChannelSettings).mockRejectedValue({ code: "settings", message: "PRIVATE" });
  await click("Save channel settings"); expect(text()).not.toContain("PRIVATE");
  const select = [...container.querySelectorAll("label")].find(label => label.textContent?.startsWith("Channel quality"))!.querySelector("select")!;
  expect(select.value).toBe("audio"); expect(text()).toContain("could not be saved or read");
});

test("late channel settings cannot populate a different channel or a signed-out workspace", async () => {
  const pending = deferred<ReturnType<typeof channelPreferences>>();
  vi.mocked(api.channelSettings).mockReturnValueOnce(pending.promise);
  await render(); await click("Live"); await click("Open channel Example Channel");
  await click("Go back");
  await act(async () => pending.resolve(channelPreferences("channel-one", "low")));
  expect(container.querySelector(".channel-preferences")).toBeNull();
  vi.mocked(api.channelSettings).mockResolvedValue(channelPreferences("channel-one", "high"));
  await click("Open channel Example Channel"); expect(text()).toContain("Saved effective quality: High");
  vi.mocked(api.logout).mockResolvedValue(signedOut); await click("Sign out");
  expect(container.querySelector(".channel-preferences")).toBeNull();
});

test("global saves refresh the current channel's backend inheritance preview", async () => {
  await render(); await click("Live"); await click("Open channel Example Channel");
  await click("Settings"); await editControl("Default quality", "high", "select");
  vi.mocked(api.savePlaybackSettings).mockResolvedValue({ ...playbackSettings, defaultQuality: "high" });
  vi.mocked(api.channelSettings).mockResolvedValue(channelPreferences("channel-one", "high"));
  await click("Save settings"); await click("Settings");
  expect(text()).toContain("Use global default (High"); expect(api.channelSettings).toHaveBeenCalledTimes(2);
});

test("manual browser chat supplies only broadcaster and session IDs and prevents duplicate requests", async () => {
  const pending = deferred<null>(); vi.mocked(api.openChat).mockReturnValue(pending.promise);
  await render(); await click("Live"); await click("Open channel Example Channel");
  await act(async () => { button("Open chat in browser").click(); button("Open chat in browser").click(); });
  expect(api.openChat).toHaveBeenCalledExactlyOnceWith({ broadcasterId: "channel-one", authSessionId: "1" });
  await act(async () => pending.resolve(null)); expect(text()).toContain("Twitch chat opened");
});

test("automatic chat remains backend-owned and browser failure does not hide playback", async () => {
  vi.mocked(api.playbackSettings).mockResolvedValue({ ...playbackSettings, automaticChat: true });
  vi.mocked(api.launch).mockResolvedValue({ ...playing(), chatError: "browser_open" });
  await render(); await click("Live"); await click("Watch Example Channel");
  expect(api.openChat).not.toHaveBeenCalled();
  expect(text()).toContain("Browser chat did not open"); expect(button("Stop").disabled).toBe(false);
});

test("Restart inherits current Rust preferences unless the user explicitly selects a quality", async () => {
  vi.mocked(api.sessions).mockResolvedValue([playing()]); vi.mocked(api.restart).mockResolvedValue({ ...playing(), generation: 2, qualityPolicy: "high" });
  await render(); await click("Watching"); await click("Restart");
  expect(api.restart).toHaveBeenCalledWith({ sessionId: "play-one", generation: 1, quality: null });
  expect(text()).toContain("High");
});

test.each(["Linux x86_64", "Win32"])("focused Control shortcuts navigate, focus Search, refresh and go Back on %s", async platform => {
  Object.defineProperty(navigator, "platform", { configurable: true, value: platform });
  await render(); await keypress("2"); expect(button("Live").getAttribute("aria-current")).toBe("page");
  expect(document.activeElement).toBe(container.querySelector("h1"));
  await keypress("r"); expect(api.streams).toHaveBeenCalledTimes(2);
  await keypress("3"); expect(button("Categories").getAttribute("aria-current")).toBe("page");
  await keypress("ArrowLeft", { altKey: true }); expect(button("Live").getAttribute("aria-current")).toBe("page");
  await keypress("1"); expect(button("Following").getAttribute("aria-current")).toBe("page");
  await keypress("k"); expect(document.activeElement).toBe(container.querySelector('input[type="search"]'));
  await keypress("4"); expect(container.querySelector(".watching-panel")).not.toBeNull();
  await keypress(","); expect(container.querySelector(".settings-panel")).not.toBeNull();
  await keypress("ArrowLeft", { altKey: true }); expect(container.querySelector(".settings-panel")).toBeNull();
});

test("macOS uses Command shortcuts and ignores Control navigation", async () => {
  Object.defineProperty(navigator, "platform", { configurable: true, value: "MacIntel" });
  await render(); await keypress("2"); expect(api.streams).not.toHaveBeenCalled();
  await keypress("2", { metaKey: true }); expect(api.streams).toHaveBeenCalledOnce();
  await keypress("[", { metaKey: true }); expect(button("Following").getAttribute("aria-current")).toBe("page");
  await keypress("k", { metaKey: true }); expect(document.activeElement).toBe(container.querySelector('input[type="search"]'));
  await keypress(",", { metaKey: true }); await click("Shortcuts", ".settings-nav"); expect(text()).toContain("⌘K");
});

test("shortcuts ignore typing, editable content, composition, repeats and modal dialogs", async () => {
  Object.defineProperty(navigator, "platform", { configurable: true, value: "Linux" });
  await render(); await click("Search");
  const input = container.querySelector("input")!;
  expect((await keypress("2", { ctrlKey: true }, input)).defaultPrevented).toBe(false);
  for (const tag of ["textarea", "select", "div"]) {
    const element = document.createElement(tag); if (tag === "div") element.contentEditable = "true";
    if (tag === "div") element.setAttribute("contenteditable", "true");
    container.append(element); await keypress("2", { ctrlKey: true }, element); element.remove();
  }
  await keypress("2", { ctrlKey: true, isComposing: true }); await keypress("2", { ctrlKey: true, repeat: true });
  const dialog = document.createElement("div"); dialog.setAttribute("role", "dialog"); dialog.setAttribute("aria-modal", "true"); container.append(dialog);
  await keypress("2"); dialog.remove(); expect(api.streams).not.toHaveBeenCalled();
  await keypress("2"); expect(api.streams).toHaveBeenCalledOnce();
});

test("saved theme follows System changes and persists explicit Light and Dark through remount", async () => {
  let dark = false;
  const listeners = new Set<() => void>();
  vi.stubGlobal("matchMedia", vi.fn(() => ({ get matches() { return dark; }, addEventListener: (_: string, listener: () => void) => listeners.add(listener), removeEventListener: (_: string, listener: () => void) => listeners.delete(listener) })));
  let settings = { ...playbackSettings };
  vi.mocked(api.playbackSettings).mockImplementation(async () => settings);
  vi.mocked(api.savePlaybackSettings).mockImplementation(async value => { settings = value; return value; });
  await render(); expect(document.documentElement.dataset.theme).toBe("system"); expect(document.documentElement.dataset.resolvedTheme).toBe("light");
  await act(async () => { dark = true; listeners.forEach(listener => listener()); }); expect(document.documentElement.dataset.resolvedTheme).toBe("dark");
  for (const theme of ["light", "dark"] as const) {
    await click("Settings"); await click("Appearance", ".settings-nav"); await editControl("Appearance", theme, "select");
    expect(document.documentElement.dataset.theme).toBe(settings.theme);
    await click("Save settings"); expect(document.documentElement.dataset.theme).toBe(theme);
    await act(async () => { dark = !dark; listeners.forEach(listener => listener()); }); expect(document.documentElement.dataset.resolvedTheme).toBe(theme);
    await act(async () => root.unmount()); root = createRoot(container); await render();
    expect(document.documentElement.dataset.theme).toBe(theme);
    expect(localStorage.getItem("stream-gui-theme")).toBeNull();
  }
  await click("Settings"); await click("Appearance", ".settings-nav"); await editControl("Appearance", "system", "select"); await click("Save settings");
  expect(document.documentElement.dataset.theme).toBe("system");
  vi.unstubAllGlobals();
});

async function toggleControl(label: string) {
  const input = [...container.querySelectorAll("label")].find(node => node.textContent?.startsWith(label))!.querySelector<HTMLInputElement>('input[type="checkbox"]')!;
  await act(async () => input.click());
}
const desktopSnapshot: import("../lib/generated").DesktopStatus = {
  monitor: { phase: "running", paused: false, liveCount: 3, stale: false, error: null, retryInSeconds: 60, notificationError: false },
  notificationPermission: "not_requested", notificationClickSupported: true, notificationTestAvailable: false, trayAvailable: true, action: null,
};
const testNotification = {
  kind: "channel" as const, id: "native-test", authSessionId: "notification-acceptance", broadcasterId: "0", displayName: "TEST notification: Synthetic channel",
};
test("native acceptance controls require the backend build feature", async () => {
  vi.mocked(api.desktopStatus).mockResolvedValue(desktopSnapshot);
  vi.mocked(api.diagnostics).mockRejectedValue({ code: "internal" });
  await render(); await click("Settings"); await click("Developer tools");
  expect(text()).not.toContain("Send test notification");
  expect(api.devNotificationTest).not.toHaveBeenCalled();
});
test("signed-out developer tools send and clear fixed tests and route native actions", async () => {
  vi.mocked(api.authStatus).mockResolvedValue(signedOut);
  vi.mocked(api.diagnostics).mockRejectedValue({ code: "internal" });
  const enabled = { ...desktopSnapshot, notificationTestAvailable: true, notificationPermission: "os_managed" as const };
  vi.mocked(api.desktopStatus).mockResolvedValue(enabled);
  vi.mocked(api.devNotificationTest).mockResolvedValue(null);
  vi.mocked(api.acknowledgeDesktopAction).mockResolvedValue(null);
  await render(); await click("Settings"); await click("Developer tools");
  await click("Send test notification");
  expect(api.devNotificationTest).toHaveBeenLastCalledWith("send");
  expect(text()).toContain("Test notification queued");
  await click("Clear test notifications");
  expect(api.devNotificationTest).toHaveBeenLastCalledWith("clear");
  vi.mocked(api.desktopStatus).mockResolvedValue({ ...enabled, action: testNotification });
  await act(async () => { await vi.advanceTimersByTimeAsync(2100); });
  expect(text()).toContain("TEST notification · Synthetic channel");
  expect(document.activeElement?.textContent).toBe("TEST notification · Synthetic channel");
  expect(api.acknowledgeDesktopAction).toHaveBeenCalledWith("native-test");
  expect(api.channel).not.toHaveBeenCalled(); expect(api.followedStreams).not.toHaveBeenCalled();
  expect(api.account).not.toHaveBeenCalled(); expect(api.launch).not.toHaveBeenCalled();
  expect(api.pauseMonitor).not.toHaveBeenCalled(); expect(api.savePlaybackSettings).not.toHaveBeenCalled();
  // A still-cached snapshot must not replay an acknowledged action on remount.
  await click("Developer tools"); await click("← Back to browsing");
  expect(text()).not.toContain("TEST notification · Synthetic channel");
  expect(api.acknowledgeDesktopAction).toHaveBeenCalledOnce();
});
test.each(["unknown", "not_requested", "denied", "unavailable"] as const)("native acceptance cannot queue a test with %s permission", async notificationPermission => {
  vi.mocked(api.diagnostics).mockRejectedValue({ code: "internal" });
  vi.mocked(api.desktopStatus).mockResolvedValue({ ...desktopSnapshot, notificationTestAvailable: true, notificationPermission });
  await render(); await click("Settings"); await click("Developer tools");
  const send = [...container.querySelectorAll("button")].find(button => button.textContent === "Send test notification")!;
  expect(send.disabled).toBe(true);
  await act(async () => send.click());
  expect(api.devNotificationTest).not.toHaveBeenCalled();
  expect(text()).not.toContain("Test notification queued");
});
test("failed native authorization stays visible and can be retried before a test", async () => {
  vi.mocked(api.diagnostics).mockRejectedValue({ code: "internal" });
  const enabled = { ...desktopSnapshot, notificationTestAvailable: true };
  vi.mocked(api.desktopStatus).mockResolvedValue(enabled);
  vi.mocked(api.requestNotificationPermission).mockResolvedValue(null);
  vi.mocked(api.devNotificationTest).mockResolvedValue(null);
  await render(); await click("Settings"); await click("Developer tools");
  expect(text()).toContain("Allow desktop notifications before sending a test");
  vi.mocked(api.desktopStatus).mockResolvedValue({ ...enabled, notificationPermission: "unavailable" });
  await click("Allow desktop notifications");
  expect(text()).toContain("Native notifications unavailable");
  expect(text()).toContain("code signature");
  vi.mocked(api.desktopStatus).mockResolvedValue({ ...enabled, notificationPermission: "granted" });
  await click("Retry desktop notifications");
  expect(api.requestNotificationPermission).toHaveBeenCalledTimes(2);
  await click("Send test notification");
  expect(api.devNotificationTest).toHaveBeenCalledWith("send");
  expect(text()).toContain("OS delivery is not confirmed");
});
test("test activation retries acknowledgement without reopening the dismissed target", async () => {
  vi.mocked(api.authStatus).mockResolvedValue(signedOut);
  vi.mocked(api.desktopStatus).mockImplementation(async () => ({ ...desktopSnapshot, notificationTestAvailable: true, action: testNotification }));
  vi.mocked(api.acknowledgeDesktopAction).mockRejectedValueOnce({ code: "internal" }).mockResolvedValue(null);
  await render(); expect(text()).toContain("TEST notification · Synthetic channel");
  await click("Back to browsing");
  await act(async () => { await vi.advanceTimersByTimeAsync(2100); });
  expect(text()).not.toContain("TEST notification · Synthetic channel");
  expect(api.acknowledgeDesktopAction).toHaveBeenCalledTimes(2);
});
test.each([
  [false, testNotification],
  [true, { ...testNotification, broadcasterId: "123" }],
  [true, { ...testNotification, authSessionId: "old-session" }],
])("test navigation rejects disabled builds or non-test identities (%s, %j)", async (notificationTestAvailable, action) => {
  vi.mocked(api.authStatus).mockResolvedValue(signedOut);
  vi.mocked(api.desktopStatus).mockResolvedValue({ ...desktopSnapshot, notificationTestAvailable, action });
  await render();
  expect(text()).not.toContain("TEST notification · Synthetic channel");
  expect(api.acknowledgeDesktopAction).not.toHaveBeenCalled(); expect(api.channel).not.toHaveBeenCalled();
});
test("Background settings persist intent and pause independently of browsing", async () => {
  vi.mocked(api.desktopStatus).mockResolvedValue(desktopSnapshot);
  vi.mocked(api.savePlaybackSettings).mockImplementation(async value => value);
  await render(); await click("Settings"); await click("Background", ".settings-nav");
  expect(text()).toContain("Monitoring followed streams"); expect(text()).toContain("3 followed live");
  await toggleControl("Monitor followed live streams");
  await toggleControl("Notify when followed channels go live");
  await editControl("Check for live streams", "120", "select");
  await toggleControl("Keep Stream GUI RS running");
  await click("Save settings");
  expect(api.savePlaybackSettings).toHaveBeenCalledWith(expect.objectContaining({ background: {
    monitoringEnabled: true, notificationsEnabled: true, closeToBackground: true, intervalSeconds: 120,
  } }));
  vi.mocked(api.desktopStatus).mockResolvedValue({ ...desktopSnapshot, monitor: { ...desktopSnapshot.monitor, phase: "paused", paused: true } });
  await click("Pause monitoring"); expect(api.pauseMonitor).toHaveBeenCalledOnce();
  await click("Resume monitoring"); expect(api.resumeMonitor).toHaveBeenCalledOnce();
  await click("Allow desktop notifications"); expect(api.requestNotificationPermission).toHaveBeenCalledOnce();
  expect(api.logout).not.toHaveBeenCalled(); expect(api.followedStreams).toHaveBeenCalledOnce();
});
test("notification actions survive reconstruction, navigate once and never launch playback", async () => {
  vi.mocked(api.desktopStatus).mockResolvedValue({ ...desktopSnapshot, action: {
    kind: "channel", id: "action-1", authSessionId: "1", broadcasterId: "channel-one", displayName: "Example Channel",
  } });
  vi.mocked(api.acknowledgeDesktopAction).mockResolvedValue(null);
  await render();
  expect(api.channel).toHaveBeenCalledWith({ id: "channel-one", page: { sessionId: "1", cursor: null, refresh: false } });
  expect(api.acknowledgeDesktopAction).toHaveBeenCalledWith("action-1");
  await act(async () => { await vi.advanceTimersByTimeAsync(2100); });
  expect(api.acknowledgeDesktopAction).toHaveBeenCalledOnce();
  expect(api.launch).not.toHaveBeenCalled();
});
test("a late desktop poll cannot replace the accepted Pause snapshot", async () => {
  vi.mocked(api.desktopStatus).mockResolvedValue(desktopSnapshot);
  await render(); await click("Settings"); await click("Background", ".settings-nav");
  const old = deferred<import("../lib/generated").DesktopStatus>();
  vi.mocked(api.desktopStatus).mockReturnValueOnce(old.promise);
  await act(async () => { await vi.advanceTimersByTimeAsync(2100); });
  vi.mocked(api.desktopStatus).mockResolvedValue({ ...desktopSnapshot, monitor: { ...desktopSnapshot.monitor, phase: "paused", paused: true } });
  await click("Pause monitoring");
  await act(async () => { old.resolve(desktopSnapshot); });
  expect(text()).toContain("Monitoring is paused");
  expect(text()).toContain("Resume monitoring");
});
test("retrying a notification acknowledgement does not steal navigation again", async () => {
  vi.mocked(api.desktopStatus).mockImplementation(async () => ({ ...desktopSnapshot, action: {
    kind: "channel", id: "retry-action", authSessionId: "1", broadcasterId: "channel-one", displayName: "Example Channel",
  } }));
  vi.mocked(api.acknowledgeDesktopAction).mockRejectedValueOnce({ code: "internal" }).mockResolvedValue(null);
  await render();
  expect(api.channel).toHaveBeenCalledOnce();
  await click("Live");
  await act(async () => { await vi.advanceTimersByTimeAsync(2100); });
  expect(api.acknowledgeDesktopAction).toHaveBeenCalledTimes(2);
  expect(api.channel).toHaveBeenCalledOnce();
  expect(api.launch).not.toHaveBeenCalled();
});
test("old account notification cannot navigate the replacement workspace", async () => {
  vi.mocked(api.desktopStatus).mockResolvedValue({ ...desktopSnapshot, action: {
    kind: "channel", id: "old-action", authSessionId: "previous", broadcasterId: "channel-one", displayName: "Old channel",
  } });
  await render(); expect(api.channel).not.toHaveBeenCalled(); expect(api.launch).not.toHaveBeenCalled();
  expect(api.acknowledgeDesktopAction).not.toHaveBeenCalled();
});
test("channel notification suppression uses a nullable stable-ID override", async () => {
  await render(); await click("Live"); await click("Open channel Example Channel");
  await editControl("Channel notifications", "off", "select");
  vi.mocked(api.saveChannelSettings).mockResolvedValue({ ...channelPreferences(), overrides: { lowLatency: null, quality: null, automaticChat: null, notifications: false } });
  await click("Save channel settings");
  expect(api.saveChannelSettings).toHaveBeenCalledWith({ broadcasterId: "channel-one", overrides: { lowLatency: null, quality: null, automaticChat: null, notifications: false } });
  await editControl("Channel notifications", "inherit", "select"); await click("Save channel settings");
  expect(api.saveChannelSettings).toHaveBeenLastCalledWith({ broadcasterId: "channel-one", overrides: { lowLatency: null, quality: null, automaticChat: null, notifications: null } });
});

async function selectLanguage(value: string) {
  const select = container.querySelector<HTMLSelectElement>(".language-filter select")!;
  await act(async () => { select.value = value; select.dispatchEvent(new Event("change", { bubbles: true })); });
}
test("language filter persists, isolates pending results and resets paging without filtering Following", async () => {
  vi.mocked(api.saveDiscoveryLanguage).mockImplementation(async discoveryLanguage => ({ ...playbackSettings, discoveryLanguage }));
  const old = deferred<PagedResult<StreamSummary>>();
  vi.mocked(api.streams).mockReturnValueOnce(old.promise).mockResolvedValueOnce(page([], "en-next")).mockResolvedValueOnce(page([stream]));
  await render(); await click("Live"); await selectLanguage("en");
  expect(api.saveDiscoveryLanguage).toHaveBeenCalledWith("en");
  expect(api.streams).toHaveBeenLastCalledWith({ page: { sessionId: "1", cursor: null, refresh: false }, language: "en" });
  expect(text()).toContain("No streams in this language");
  await act(async () => old.resolve(page([{ ...stream, displayName: "Old language result" }], "old-cursor")));
  expect(text()).not.toContain("Old language result");
  await click("Load more"); expect(api.streams).toHaveBeenLastCalledWith({ page: { sessionId: "1", cursor: "en-next", refresh: false }, language: "en" });
  await click("Following"); expect(container.querySelector(".language-filter")).toBeNull();
  expect(api.followedStreams).toHaveBeenCalledWith({ sessionId: "1", cursor: null, refresh: false });
  await click("Live"); await click("Reset language");
  expect(api.saveDiscoveryLanguage).toHaveBeenLastCalledWith(null);
  expect(api.streams).toHaveBeenLastCalledWith({ page: { sessionId: "1", cursor: null, refresh: false }, language: null });
});
test("Back restores a discovery visit's language and focus while new visits use saved preference", async () => {
  vi.mocked(api.playbackSettings).mockResolvedValue({ ...playbackSettings, discoveryLanguage: "en" });
  vi.mocked(api.saveDiscoveryLanguage).mockImplementation(async discoveryLanguage => ({ ...playbackSettings, discoveryLanguage }));
  await render(); await click("Live");
  button("Example Game").focus(); await click("Example Game"); await selectLanguage("other");
  expect(api.category).toHaveBeenLastCalledWith({ id: "game-one", page: { page: { sessionId: "1", cursor: null, refresh: false }, language: "other" } });
  await click("Go back");
  expect(container.querySelector<HTMLSelectElement>(".language-filter select")!.value).toBe("en");
  expect(document.activeElement).toBe(button("Example Game"));
  await click("Refresh"); expect(api.streams).toHaveBeenLastCalledWith({ page: { sessionId: "1", cursor: null, refresh: true }, language: "en" });
});
test("language save errors preserve accepted preference and filtered network errors are not empty", async () => {
  vi.mocked(api.playbackSettings).mockResolvedValue({ ...playbackSettings, discoveryLanguage: "de" });
  vi.mocked(api.saveDiscoveryLanguage).mockRejectedValue({ code: "settings" });
  vi.mocked(api.streams).mockRejectedValue({ code: "network" });
  await render(); await click("Live"); await selectLanguage("en");
  expect(container.querySelector<HTMLSelectElement>(".language-filter select")!.value).toBe("de");
  expect(text()).not.toContain("No streams in this language");
  expect(text()).toContain("Check your connection");
});

async function lookupLogin(login: string) {
  const input = container.querySelector<HTMLInputElement>(".exact-lookup input")!;
  await act(async () => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, login); input.dispatchEvent(new Event("input", { bubbles: true })); input.focus(); });
}
async function submitLookup() {
  await act(async () => { container.querySelector<HTMLFormElement>(".exact-lookup")!.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true })); });
}
test("exact lookup uses its own command, opens offline details without autoplay and restores input focus", async () => {
  vi.mocked(api.lookupChannel).mockResolvedValue({ broadcasterId: "channel-one", displayName: "Example Channel" });
  await render(); await click("Open channel", ".side-nav"); await lookupLogin("ExAmPlE"); await submitLookup();
  expect(api.lookupChannel).toHaveBeenCalledWith({ sessionId: "1", login: "ExAmPlE" });
  expect(api.searchChannels).not.toHaveBeenCalled(); expect(api.launch).not.toHaveBeenCalled();
  expect(text()).toContain("Offline — no current live stream");
  await click("Go back"); expect(document.activeElement).toBe(container.querySelector(".exact-lookup input"));
  expect(container.querySelector<HTMLInputElement>(".exact-lookup input")!.value).toBe("ExAmPlE");
});
test("exact lookup distinguishes invalid, missing and network failures", async () => {
  await render(); await click("Open channel", ".side-nav"); await lookupLogin("example");
  for (const [code, message] of [["invalid_input", "Do not enter a URL"], ["not_found", "No channel has that Twitch login"], ["network", "Check your connection"]]) {
    vi.mocked(api.lookupChannel).mockRejectedValueOnce({ code, message: "untrusted" }); await submitLookup();
    expect(text()).toContain(message); expect(text()).not.toContain("untrusted");
  }
  expect(api.channel).not.toHaveBeenCalled(); expect(api.launch).not.toHaveBeenCalled();
});
test("lookup blocks duplicate submission and ignores changed login or account responses", async () => {
  const old = deferred<Awaited<ReturnType<typeof api.lookupChannel>>>();
  vi.mocked(api.lookupChannel).mockReturnValue(old.promise);
  await render(); await click("Open channel", ".side-nav"); await lookupLogin("first"); await submitLookup(); await submitLookup();
  expect(api.lookupChannel).toHaveBeenCalledOnce(); await lookupLogin("second");
  await act(async () => old.resolve({ broadcasterId: "old", displayName: "Old result" }));
  expect(api.channel).not.toHaveBeenCalled();
  const later = deferred<Awaited<ReturnType<typeof api.lookupChannel>>>(); vi.mocked(api.lookupChannel).mockReturnValue(later.promise);
  await submitLookup(); vi.mocked(api.authStatus).mockResolvedValue({ ...signedIn, sessionId: "2" });
  await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
  await act(async () => later.resolve({ broadcasterId: "old", displayName: "Old session" }));
  expect(api.channel).not.toHaveBeenCalled(); expect(text()).not.toContain("Old session");
});
test("late lookup results offer navigation without stealing focus from Settings", async () => {
  const pending = deferred<Awaited<ReturnType<typeof api.lookupChannel>>>(); vi.mocked(api.lookupChannel).mockReturnValue(pending.promise);
  await render(); await click("Open channel", ".side-nav"); await lookupLogin("example"); await submitLookup(); await click("Settings");
  const focus = document.activeElement;
  await act(async () => pending.resolve({ broadcasterId: "channel-one", displayName: "Example Channel" }));
  expect(document.activeElement).toBe(focus); expect(api.channel).not.toHaveBeenCalled();
  await click("Open resolved channel"); expect(api.channel).toHaveBeenCalledOnce();
});
