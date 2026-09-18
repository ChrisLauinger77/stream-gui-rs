import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import type { CategorySummary, ChannelSummary, StreamSummary } from "../lib/generated";
import { errorText } from "./errors";
import type { Watch } from "./Workspace";
import type { usePage } from "./usePage";

export function Media({ src, retryGeneration = 0, shape = "preview" }: { src: string | null; retryGeneration?: number; shape?: "preview" | "avatar" | "artwork" }) {
  const [state, setState] = useState<"loading" | "loaded" | "failed">("loading");
  useEffect(() => setState("loading"), [src]);
  useEffect(() => setState(current => current === "failed" ? "loading" : current), [retryGeneration]);
  return <span className={`media ${shape} ${state}`}>
    <span className="image-placeholder" aria-hidden="true">{!src ? "No image" : state === "failed" ? "Image unavailable" : "Loading image"}</span>
    {src && state !== "failed" && <img src={src} alt="" loading="lazy" decoding="async" referrerPolicy="no-referrer" onLoad={() => setState("loaded")} onError={() => setState("failed")} />}
  </span>;
}
const viewers = new Intl.NumberFormat(undefined, { notation: "compact", maximumFractionDigits: 1 });
export function dateLabel(value: string | null) {
  if (!value) return "";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "Date unavailable" : date.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}
export type OpenChannel = (id: string, name: string) => void;
export type OpenCategory = (id: string, name: string) => void;
export function StreamPreview({ stream, retryGeneration }: { stream: StreamSummary; retryGeneration: number }) {
  return <div className="stream-preview"><Media retryGeneration={retryGeneration} src={stream.previewUrl} /><span className="live-tag">LIVE</span><span className="viewer-count">{viewers.format(stream.viewerCount)} viewers</span></div>;
}
export function StreamCard({ stream, channel, category, retryGeneration, ...watch }: { stream: StreamSummary; retryGeneration: number; channel: OpenChannel; category: OpenCategory } & Watch) {
  return <article className="stream-card">
    <button className="item-link" data-focus={`stream:${stream.streamId}`} onClick={() => channel(stream.broadcasterId, stream.displayName)} aria-label={`Open channel ${stream.displayName}`}>
      <StreamPreview retryGeneration={retryGeneration} stream={stream} />
      <h3>{stream.displayName}</h3><p className="stream-title" title={stream.title}>{stream.title || "Untitled stream"}</p>
    </button>
    <div className="stream-meta">{stream.categoryId ? <button className="text-button" onClick={() => category(stream.categoryId!, stream.categoryName ?? "Category")}>{stream.categoryName ?? "Category"}</button> : <span>Uncategorized</span>}{stream.language && <span>{stream.language.toUpperCase()}</span>}</div>
    <WatchButton id={stream.broadcasterId} name={stream.displayName} {...watch} />
  </article>;
}
export function StreamList({ items, channel, category, retryGeneration, ...watch }: { items: StreamSummary[]; retryGeneration: number; channel: OpenChannel; category: OpenCategory } & Watch) {
  return <div className="stream-grid">{items.map(stream => <StreamCard retryGeneration={retryGeneration} key={stream.streamId} stream={stream} channel={channel} category={category} {...watch} />)}</div>;
}
export function WatchButton({ id, name, watch, pending }: Watch & { id: string; name: string }) {
  const busy = pending.has(`launch:${id}`);
  return <button className="watch-button" aria-label={`Watch ${name}`} disabled={busy} onClick={() => watch(id)}>{busy ? "Starting…" : "▶ Watch"}</button>;
}
export function CategoryList({ items, open, retryGeneration }: { items: CategorySummary[]; retryGeneration: number; open: OpenCategory }) {
  return <div className="category-grid">{items.map(category => <button key={category.id} className="item-link category-card" data-focus={`category:${category.id}`} onClick={() => open(category.id, category.name)} aria-label={`Open category ${category.name}`}><Media retryGeneration={retryGeneration} src={category.imageUrl} shape="artwork" /><h3>{category.name}</h3></button>)}</div>;
}
export function ChannelList({ items, open, retryGeneration, ...watch }: { items: ChannelSummary[]; retryGeneration: number; open: OpenChannel } & Watch) {
  return <div className="channel-list">{items.map(channel => <div className="channel-entry" key={channel.broadcasterId}><button className="item-link channel-row" data-focus={`channel:${channel.broadcasterId}`} onClick={() => open(channel.broadcasterId, channel.displayName)} aria-label={`Open channel ${channel.displayName}`}>
    <Media retryGeneration={retryGeneration} src={channel.imageUrl} shape="avatar" /><span><h3>{channel.displayName}<span className={channel.liveState === "live" ? "live-tag" : "muted"}> · {channel.liveState === "unknown" ? "Status unavailable" : channel.liveState === "live" ? "LIVE" : "Offline"}</span></h3>
    <p>{channel.title ?? `@${channel.login}`}</p><p>{channel.categoryName}{channel.language && ` · ${channel.language.toUpperCase()}`}</p>
    {channel.followedAt && <p>Followed {dateLabel(channel.followedAt)}</p>}</span>
  </button>{channel.liveState === "live" && <WatchButton id={channel.broadcasterId} name={channel.displayName} {...watch} />}</div>)}</div>;
}
export function PageFrame<T>({ query, children, empty, detail = false }: { query: ReturnType<typeof usePage<T>>; children: ReactNode; empty: string; detail?: boolean }) {
  const results = useRef<HTMLDivElement>(null);
  const paginationFocus = useRef<Set<string> | null>(null);
  useLayoutEffect(() => {
    if (query.pending) return;
    const previous = paginationFocus.current;
    paginationFocus.current = null;
    if (!previous || query.error || query.page?.cursor || document.activeElement !== document.body) return;
    const appended = [...(results.current?.querySelectorAll<HTMLElement>("[data-focus]") ?? [])].find(item => !previous.has(item.dataset.focus!));
    (appended ?? results.current)?.focus();
  }, [query.page, query.pending, query.error]);
  return <div aria-busy={query.pending}>
    <div className="query-toolbar"><span role="status">{(query.retained || (query.error && query.page)) ? "Previous results · refresh to check for updates" : query.page?.freshness === "stale" ? "Stale data" : query.page?.freshness === "cached" ? `Cached · ${query.page.ageSeconds}s old` : query.page ? "Updated from Twitch" : ""}{query.error && query.page ? " · update failed" : ""}</span><button data-browse-refresh disabled={query.pending} onClick={query.refresh}>Refresh</button></div>
    {query.error && <div className="error" role="alert">{errorText(query.error)}<button onClick={query.retry} disabled={query.pending}>Retry</button></div>}
    {query.page?.warnings.map(code => <p className="notice" role="status" key={code}>Some details are unavailable. {errorText(code)}</p>)}
    {query.pending && <p className="loading" role="status">{query.page ? "Loading more information…" : "Loading…"}</p>}
    <div ref={results} role="region" aria-label="Results" tabIndex={-1}>{query.page?.items.length ? children : !query.pending && !query.error && query.page && <div className="empty-state"><h2>{empty}</h2><p>Try refreshing, or explore another view.</p></div>}</div>
    {!detail && query.page?.cursor && <button className="load-more" aria-disabled={query.pending} onBlur={() => { paginationFocus.current = null; }} onClick={event => {
      if (query.pending) return;
      // Keep the pending control focusable; a native disabled button can blur
      // before the request finishes. Any deliberate blur cancels restoration.
      paginationFocus.current = document.activeElement === event.currentTarget
        ? new Set([...results.current!.querySelectorAll<HTMLElement>("[data-focus]")].map(item => item.dataset.focus!)) : null;
      query.more();
    }}>{query.pending ? "Loading…" : "Load more"}</button>}
    {query.limited && <p className="notice">Showing up to 300 items. Refresh to start a new browsing session.</p>}
  </div>;
}
