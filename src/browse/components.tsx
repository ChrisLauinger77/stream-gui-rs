import { currentLocale, formatDate, translate, useI18n } from "../i18n";
import { useEffect, useId, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import type { CategorySummary, ChannelSummary, StreamSummary } from "../lib/generated";
import { errorText } from "./errors";
import type { Watch } from "./Workspace";
import type { usePage } from "./usePage";

export function Media({ src, retryGeneration = 0, shape = "preview" }: { src: string | null; retryGeneration?: number; shape?: "preview" | "avatar" | "artwork" }) {
  const { t } = useI18n();
  const [state, setState] = useState<"loading" | "loaded" | "failed">("loading");
  useEffect(() => setState("loading"), [src]);
  useEffect(() => setState(current => current === "failed" ? "loading" : current), [retryGeneration]);
  return <span className={`media ${shape} ${state}`}>
    <span className="image-placeholder" aria-hidden="true">{!src ? t("streams.noImage") : state === "failed" ? t("streams.imageUnavailable") : t("streams.loadingImage")}</span>
    {src && state !== "failed" && <img src={src} alt="" loading="lazy" decoding="async" referrerPolicy="no-referrer" onLoad={() => setState("loaded")} onError={() => setState("failed")} />}
  </span>;
}
export function dateLabel(value: string | null) {
  if (!value) return "";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? translate(currentLocale(), "streams.dateUnavailable") : formatDate(currentLocale(), date, { dateStyle: "medium", timeStyle: "short" });
}
export type OpenChannel = (id: string, name: string) => void;
export type OpenCategory = (id: string, name: string) => void;
export function StreamPreview({ stream, retryGeneration }: { stream: StreamSummary; retryGeneration: number }) {
  const { t, count, compact } = useI18n();
  const viewers = compact(stream.viewerCount);
  return <div className="stream-preview"><Media retryGeneration={retryGeneration} src={stream.previewUrl} /><span className="live-tag">{t("components.live")}</span><span className="viewer-count">{count("streams.viewers", stream.viewerCount, { count: viewers })}</span></div>;
}
export function StreamCard({ stream, channel, category, retryGeneration, ...watch }: { stream: StreamSummary; retryGeneration: number; channel: OpenChannel; category: OpenCategory } & Watch) {
  const { t } = useI18n();
  const description = useId();
  return <article className="stream-card">
    <button className="item-link" data-focus={`stream:${stream.streamId}`} onClick={() => channel(stream.broadcasterId, stream.displayName)} aria-label={t("streams.openChannel", { name: stream.displayName })} aria-describedby={description}>
      <StreamPreview retryGeneration={retryGeneration} stream={stream} />
      <h3>{stream.displayName}</h3><p className="stream-title" title={stream.title}>{stream.title || t("streams.untitled")}</p>
    </button>
    <span className="sr-only" id={description}>{t("streams.liveDescription", { category: stream.categoryName ?? t("components.uncategorized"), title: stream.title || t("streams.untitled") })}</span>
    <div className="stream-meta">{stream.categoryId ? <button className="text-button" data-focus={`category:${stream.categoryId}:stream:${stream.streamId}`} onClick={() => category(stream.categoryId!, stream.categoryName ?? t("discovery.category"))}>{stream.categoryName ?? t("discovery.category")}</button> : <span>{t("components.uncategorized")}</span>}{stream.language && <span>{stream.language.toUpperCase()}</span>}</div>
    <WatchButton id={stream.broadcasterId} name={stream.displayName} {...watch} />
  </article>;
}
export function StreamList({ items, channel, category, retryGeneration, ...watch }: { items: StreamSummary[]; retryGeneration: number; channel: OpenChannel; category: OpenCategory } & Watch) {
  return <div className="stream-grid">{items.map(stream => <StreamCard retryGeneration={retryGeneration} key={stream.streamId} stream={stream} channel={channel} category={category} {...watch} />)}</div>;
}
export function WatchButton({ id, name, watch, pending }: Watch & { id: string; name: string }) {
  const { t } = useI18n();
  const busy = pending.has(`launch:${id}`);
  return <button className="watch-button" aria-label={t("streams.watchNamed", { name })} disabled={busy} onClick={() => watch(id)}>{busy ? t("streams.starting") : t("streams.watch")}</button>;
}
export function CategoryList({ items, open, retryGeneration }: { items: CategorySummary[]; retryGeneration: number; open: OpenCategory }) {
  const { t } = useI18n();
  return <div className="category-grid">{items.map(category => <button key={category.id} className="item-link category-card" data-focus={`category:${category.id}`} onClick={() => open(category.id, category.name)} aria-label={t("streams.openCategory", { name: category.name })}><Media retryGeneration={retryGeneration} src={category.imageUrl} shape="artwork" /><h3>{category.name}</h3></button>)}</div>;
}
export function ChannelList({ items, open, retryGeneration, ...watch }: { items: ChannelSummary[]; retryGeneration: number; open: OpenChannel } & Watch) {
  const { t } = useI18n();
  const description = useId();
  return <div className="channel-list">{items.map(channel => <div className="channel-entry" key={channel.broadcasterId}><button className="item-link channel-row" data-focus={`channel:${channel.broadcasterId}`} onClick={() => open(channel.broadcasterId, channel.displayName)} aria-label={t("streams.openChannel", { name: channel.displayName })} aria-describedby={`${description}-${channel.broadcasterId}`}>
    <Media retryGeneration={retryGeneration} src={channel.imageUrl} shape="avatar" /><span><h3>{channel.displayName}<span className={channel.liveState === "live" ? "live-tag" : "muted"}> · {channel.liveState === "unknown" ? t("streams.statusUnavailable") : channel.liveState === "live" ? t("components.live") : t("streams.offline")}</span></h3>
    <p>{channel.title ?? `@${channel.login}`}</p><p>{channel.categoryName}{channel.language && ` · ${channel.language.toUpperCase()}`}</p>
    {channel.followedAt && <p>{t("components.followed")}{" "}{dateLabel(channel.followedAt)}</p>}</span>
  </button><span className="sr-only" id={`${description}-${channel.broadcasterId}`}>{channel.liveState === "unknown" ? t("search.liveUnknown") : channel.liveState === "live" ? t("browse.live") : t("streams.offline")}{channel.categoryName && ` · ${channel.categoryName}`}</span>{channel.liveState === "live" && <WatchButton id={channel.broadcasterId} name={channel.displayName} {...watch} />}</div>)}</div>;
}
export function PageFrame<T>({ query, children, empty, detail = false }: { query: ReturnType<typeof usePage<T>>; children: ReactNode; empty: string; detail?: boolean }) {
  const { t, count } = useI18n();
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
    <div className="query-toolbar"><span role="status">{(query.retained || (query.error && query.page)) ? t("streams.previousResults") : query.page?.freshness === "stale" ? t("streams.staleData") : query.page?.freshness === "cached" ? count("streams.cachedAge", query.page.ageSeconds) : query.page ? t("streams.updatedFromTwitch") : ""}{query.error && query.page ? t("streams.updateFailed") : ""}</span><button data-browse-refresh disabled={query.pending} onClick={query.refresh}>{t("components.refresh")}</button></div>
    {query.error && <div className="error" role="alert">{errorText(query.error)}<button onClick={query.retry} disabled={query.pending}>{t("components.retry")}</button></div>}
    {query.page?.warnings.map(code => <p className="notice" role="status" key={code}>{t("components.someDetailsAreUnavailable")}{" "}{errorText(code)}</p>)}
    {query.pending && <p className="loading" role="status">{query.page ? t("streams.loadingMore") : t("streams.loading")}</p>}
    <div ref={results} role="region" aria-label={t("components.results")} tabIndex={-1}>{query.page?.items.length ? children : !query.pending && !query.error && query.page && <div className="empty-state"><h2>{empty}</h2><p>{t("streams.emptyHelp")}</p></div>}</div>
    {!detail && query.page?.cursor && <button className="load-more" aria-disabled={query.pending} onBlur={() => { paginationFocus.current = null; }} onClick={event => {
      if (query.pending) return;
      // Keep the pending control focusable; a native disabled button can blur
      // before the request finishes. Any deliberate blur cancels restoration.
      paginationFocus.current = document.activeElement === event.currentTarget
        ? new Set([...results.current!.querySelectorAll<HTMLElement>("[data-focus]")].map(item => item.dataset.focus!)) : null;
      query.more();
    }}>{query.pending ? t("streams.loading") : t("streams.loadMore")}</button>}
    {query.limited && <p className="notice">{t("streams.itemLimit")}</p>}
  </div>;
}
