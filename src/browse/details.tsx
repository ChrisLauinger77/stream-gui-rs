import { useEffect, useState } from "react";
import { LocalItemActions } from "../features/DiscoveryPreferences";
import { ChannelPreferences } from "../features/ChannelPreferences";
import { api } from "../lib/ipc";
import type { CategorySummary, ChannelDetails, ChannelSummary } from "../lib/generated";
import { usePage } from "./usePage";
import { CategoryList, ChannelList, dateLabel, Media, PageFrame, StreamPreview, WatchButton } from "./components";
import { pageRequest, type Links, type QueryContext } from "./Workspace";
export function SearchView({ context, links, draft, setDraft, type, setType }: {
  context: QueryContext; links: Links; draft: string; setDraft: (value: string) => void;
  type: "channels" | "categories"; setType: (value: "channels" | "categories") => void;
}) {
  const [settled, setSettled] = useState(draft.trim());
  useEffect(() => { const timer = setTimeout(() => setSettled(draft.trim()), 350); return () => clearTimeout(timer); }, [draft]);
  return <><form className="search-form" role="search" onSubmit={e => { e.preventDefault(); setSettled(draft.trim()); }}>
    <label>Search Twitch<input type="search" name="query" maxLength={100} autoComplete="off" placeholder="Channel or category name" value={draft} onChange={e => setDraft(e.target.value)} /></label></form>
    <div className="tabs" aria-label="Search result type"><button aria-pressed={type === "channels"} onClick={() => setType("channels")}>Channels</button><button aria-pressed={type === "categories"} onClick={() => setType("categories")}>Categories</button></div>
    {!draft.trim() ? <div className="empty-state"><h2>What are you looking for?</h2><p>Enter a channel or category name to get started.</p></div> : settled !== draft.trim() ? <p role="status" className="loading">Waiting for your search…</p> : type === "channels" ? <ChannelSearch key={`channels:${settled}`} search={settled} context={context} links={links} /> : <CategorySearch key={`categories:${settled}`} search={settled} context={context} links={links} />}
  </>;
}
function ChannelSearch({ search, context, links }: { search: string; context: QueryContext; links: Links }) {
  const query = usePage<ChannelSummary>({ ...context, viewKey: `search:channels:${search}`, identify: c => c.broadcasterId,
    load: (cursor, refresh) => api.searchChannels({ query: search, page: pageRequest(context.sessionId, cursor, refresh) }) });
  return <PageFrame query={query} empty={`No channels found for “${search}”`}><ChannelList retryGeneration={query.imageRetryGeneration} items={query.page?.items ?? []} open={links.channel} watch={links.watch} pending={links.pending} /></PageFrame>;
}
function CategorySearch({ search, context, links }: { search: string; context: QueryContext; links: Links }) {
  const query = usePage<CategorySummary>({ ...context, viewKey: `search:categories:${search}`, identify: c => c.id,
    load: (cursor, refresh) => api.searchCategories({ query: search, page: pageRequest(context.sessionId, cursor, refresh) }) });
  return <PageFrame query={query} empty={`No categories found for “${search}”`}><CategoryList retryGeneration={query.imageRetryGeneration} items={query.page?.items ?? []} open={links.category} /></PageFrame>;
}
export function ChannelView({ id, context, links }: { id: string; context: QueryContext; links: Links }) {
  const query = usePage<ChannelDetails>({ ...context, viewKey: `channel:${id}`, identify: d => d.channel.broadcasterId,
    load: async (_cursor, refresh) => {
      const details = await api.channel({ id, page: pageRequest(context.sessionId, null, refresh) });
      return { items: [details], cursor: null, freshness: details.freshness, ageSeconds: details.ageSeconds, warnings: details.warnings };
    } });
  const details = query.page?.items[0];
  return <PageFrame query={query} detail empty="Channel unavailable">{details && <article className="channel-detail">
    <div className="channel-identity"><Media retryGeneration={query.imageRetryGeneration} src={details.channel.imageUrl} shape="avatar" /><div><h2>{details.channel.displayName}</h2><p className="muted">@{details.channel.login}</p><p className={details.channel.liveState === "live" ? "live-tag" : "muted"}>{details.channel.liveState === "live" ? "LIVE NOW" : details.channel.liveState === "offline" ? "Offline — no current live stream" : "Live status unavailable"}</p></div></div>
    {details.channel.liveState === "live" && details.stream && <WatchButton id={details.channel.broadcasterId} name={details.channel.displayName} watch={links.watch} pending={links.pending} />}
    <LocalItemActions kind="channel" id={id} name={details.channel.displayName} />
    <ChannelPreferences key={`${context.sessionId}:${id}`} broadcasterId={id} sessionId={context.sessionId} />
    {details.description && <p className="channel-description">{details.description}</p>}
    {details.stream ? <div className="channel-stream"><StreamPreview retryGeneration={query.imageRetryGeneration} stream={details.stream} /><h3>{details.stream.title}</h3><p className="stream-meta">{details.stream.categoryId && <button className="text-button" onClick={() => links.category(details.stream!.categoryId!, details.stream!.categoryName ?? "Category")}>{details.stream.categoryName ?? "Category"}</button>}{details.stream.language?.toUpperCase()}</p>{details.stream.startedAt && <p className="muted">Started <time dateTime={details.stream.startedAt}>{dateLabel(details.stream.startedAt)}</time></p>}</div> : <div><h3>{details.channel.title ?? "No stream information available"}</h3><p className="muted">{details.channel.categoryName}{details.channel.language && ` · ${details.channel.language.toUpperCase()}`}</p></div>}
  </article>}</PageFrame>;
}
