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
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true }); vi.useFakeTimers(); vi.resetAllMocks(); localStorage.clear();
  vi.mocked(api.sessions).mockResolvedValue([]);
  vi.mocked(api.playbackSettings).mockResolvedValue({ theme: "system", automaticChat: false, streamlinkPath: null, player: { mode: "default", executable: null, arguments: [] }, defaultQuality: "source" });
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
  await click("Settings"); await act(async () => { const select = container.querySelector<HTMLSelectElement>(".settings-panel select")!; select.value = "light"; select.dispatchEvent(new Event("change", { bubbles: true })); await vi.advanceTimersByTimeAsync(1000); });
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
const playbackSettings: import("../lib/generated").Settings = { theme: "system", automaticChat: false, streamlinkPath: null, player: { mode: "default", executable: null, arguments: [] }, defaultQuality: "source" };
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
  await click("Save playback settings"); expect(api.savePlaybackSettings).toHaveBeenCalledWith(saved);
  expect(text()).toContain("Playback settings saved"); await click("Settings"); await click("Settings");
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
  await render(); await click("Settings"); await editControl("Player", "mpv", "select"); await click("Save playback settings");
  expect(text()).toContain("selected player was not found"); expect(text()).not.toContain("PRIVATE");
  await click("Settings"); await click("Settings");
  expect([...container.querySelectorAll<HTMLSelectElement>("select")].find(el => el.labels?.[0]?.textContent?.startsWith("Player"))?.value).toBe("default");
});
