import { memo, useImperativeHandle, useLayoutEffect, useRef, useState, type Ref } from "react";
import { api } from "../lib/ipc";
import type { Settings, StreamLanguage, BrowseRequest, CategorySummary, ChannelSummary, StreamSummary } from "../lib/generated";
import { ViewMemory, usePage } from "./usePage";
import { CategoryList, ChannelList, Media, PageFrame, StreamList } from "./components";
import { LanguageFilter } from "./LanguageFilter";
import { SearchView, ChannelView } from "./details";

type Section = "following" | "live" | "categories" | "search";
export type BrowserActions = { channel: (id: string, name: string) => void; navigate: (section: Section) => void; back: () => void; refresh: () => void };
type Route = ({ kind: Section } | { kind: "category" | "channel"; id: string; name: string }) & { language?: StreamLanguage | null };
type Visit = { route: Route; section: Section; scroll: number; focus?: string;
  search: string; searchType: "channels" | "categories"; following: "live" | "channels" };
const routeKey = (route: Route) => route.kind + ("id" in route ? `:${route.id}` : "") + (route.kind === "live" || route.kind === "category" ? `:${route.language ?? "any"}` : "");
export type QueryContext = { sessionId: string; memory: ViewMemory; onAuthLost: () => void; settingsRevision: number };
export type Watch = { watch: (id: string) => void; pending: ReadonlySet<string> };
export type Links = Watch & { channel: (id: string, name: string) => void; category: (id: string, name: string) => void };
export const pageRequest = (sessionId: string, cursor: string | null, refresh: boolean): BrowseRequest => ({ sessionId, cursor, refresh });
export const BrowserWorkspace = memo(function BrowserWorkspace({ sessionId, onAuthLost, watch, pending, actionsRef, settingsRevision, preferences, saveLanguage }: { preferences: Settings | null; saveLanguage: (language: StreamLanguage | null) => Promise<Settings>; sessionId: string; onAuthLost: () => void; actionsRef: Ref<BrowserActions>; settingsRevision: number } & Watch) {
  const memory = useRef(new ViewMemory()).current;
  const [route, setRoute] = useState<Route>({ kind: "following" });
  const [section, setSection] = useState<Section>("following");
  const [history, setHistory] = useState<Visit[]>([]);
  const [following, setFollowing] = useState<"live" | "channels">("live");
  const [search, setSearch] = useState("");
  const [searchType, setSearchType] = useState<"channels" | "categories">("channels");
  const content = useRef<HTMLElement>(null);
  const focusSearch = useRef(false);
  const filterChange = useRef(false);
  const restore = useRef<{ scroll: number; focus?: string } | null>(null);
  const navigate = (next: Route) => {
    if (next.kind === "live" || next.kind === "category") next = { ...next, language: preferences?.discoveryLanguage ?? null };
    if (routeKey(next) === routeKey(route)) return;
    const active = document.activeElement as HTMLElement | null;
    setHistory(items => [...items, { route, section, search, searchType, following, scroll: content.current?.scrollTop ?? 0, focus: active?.dataset.focus }].slice(-12));
    if (next.kind !== "category" && next.kind !== "channel") setSection(next.kind);
    restore.current = null; setRoute(next);
  };
  const back = () => {
    const previous = history.at(-1);
    if (!previous) return;
    restore.current = previous; setHistory(history.slice(0, -1)); setSection(previous.section); setRoute(previous.route);
    setSearch(previous.search); setSearchType(previous.searchType); setFollowing(previous.following);
  };
  useImperativeHandle(actionsRef, () => ({
    channel: (id, name) => navigate({ kind: "channel", id, name }),
    navigate: next => {
      focusSearch.current = next === "search";
      if (next === "search" && route.kind === "search") { content.current?.querySelector<HTMLInputElement>('input[type="search"]')?.focus(); focusSearch.current = false; }
      else navigate({ kind: next });
    },
    back,
    refresh: () => content.current?.querySelector<HTMLButtonElement>("[data-browse-refresh]")?.click(),
  }));
  useLayoutEffect(() => {
    if (filterChange.current) { filterChange.current = false; if (content.current) content.current.scrollTop = 0; return; }
    const target = restore.current;
    const focus = target?.focus ? [...(content.current?.querySelectorAll<HTMLElement>("[data-focus]") ?? [])].find(el => el.dataset.focus === target.focus) : undefined;
    (focus ?? content.current?.querySelector<HTMLElement>("h1"))?.focus({ preventScroll: true });
    if (content.current) content.current.scrollTop = target?.scroll ?? 0;
    if (focusSearch.current && route.kind === "search") { content.current?.querySelector<HTMLInputElement>('input[type="search"]')?.focus(); focusSearch.current = false; }
  }, [route]);
  const context = { sessionId, memory, onAuthLost, settingsRevision };
  const language = route.language ?? null;
  const links: Links = { watch, pending, channel: (id, name) => navigate({ kind: "channel", id, name }), category: (id, name) => navigate({ kind: "category", id, name }) };
  const title = "name" in route ? route.name : ({ following: "Following", live: "Live now", categories: "Categories", search: "Search" }[route.kind]);
  const subtitle = { following: "The channels you choose to keep up with.", live: "Popular streams, happening right now.", categories: "Find a game. Find your community.", search: "Discover channels and categories on Twitch.", category: "Live streams in this category.", channel: "Channel details" }[route.kind];
  return <div className="workspace">
    <nav className="side-nav" aria-label="Main navigation"><p className="nav-label">BROWSE</p>{([['following','Following','♡'],['live','Live','◉'],['categories','Categories','▦'],['search','Search','⌕']] as const).map(([kind, label, symbol]) => <button key={kind} aria-label={label} aria-current={section === kind ? "page" : undefined} onClick={() => navigate({ kind })}><span className="nav-symbol" aria-hidden="true">{symbol}</span>{label}</button>)}</nav>
    <main className="browse-content" ref={content}>
      <div className="view-top"><button className="quiet" disabled={!history.length} onClick={back} aria-label="Go back">← Back</button><span className="muted">{section === "following" ? "Your Twitch" : "Explore Twitch"}</span></div>
      <div className="view-heading"><div><h1 tabIndex={-1}>{title}</h1><p>{subtitle}</p></div></div>
      {(route.kind === "live" || route.kind === "category") && <LanguageFilter value={language} persist={saveLanguage} saved={value => {
        setRoute(current => {
          if (current !== route) return current;
          memory.forget(routeKey({ ...current, language: value.discoveryLanguage }));
          filterChange.current = true;
          return { ...current, language: value.discoveryLanguage };
        });
      }} />}
      {route.kind === "following" && <><div className="tabs" aria-label="Following views"><button aria-pressed={following === "live"} onClick={() => setFollowing("live")}>Live streams</button><button aria-pressed={following === "channels"} onClick={() => setFollowing("channels")}>All channels</button></div>{following === "live" ? <Streams key="followed" mode="followed" context={context} links={links} /> : <Follows context={context} links={links} />}</>}
      {route.kind === "live" && <Streams key={`live:${language}`} mode="live" language={language} context={context} links={links} />}
      {route.kind === "categories" && <Categories context={context} links={links} />}
      {route.kind === "category" && <Category key={`${route.id}:${language}`} id={route.id} language={language} context={context} links={links} />}
      {route.kind === "search" && <SearchView context={context} links={links} draft={search} setDraft={setSearch} type={searchType} setType={setSearchType} />}
      {route.kind === "channel" && <ChannelView key={route.id} id={route.id} context={context} links={links} />}
    </main>
  </div>;
});
function Streams({ mode, context, links, language = null }: { language?: StreamLanguage | null; mode: "live" | "followed"; context: QueryContext; links: Links }) {
  const query = usePage<StreamSummary>({ ...context, viewKey: mode === "live" ? `live:${language ?? "any"}` : mode,
    identify: stream => stream.streamId,
    load: (cursor, refresh) => mode === "followed" ? api.followedStreams(pageRequest(context.sessionId, cursor, refresh)) : api.streams({ page: pageRequest(context.sessionId, cursor, refresh), language }) });
  return <PageFrame query={query} empty={mode === "followed" ? "No followed channels are live right now" : language ? "No streams in this language" : "No live streams found"}><StreamList retryGeneration={query.imageRetryGeneration} items={query.page?.items ?? []} {...links} /></PageFrame>;
}
function Follows({ context, links }: { context: QueryContext; links: Links }) {
  const query = usePage<ChannelSummary>({ ...context, viewKey: "followed-channels", identify: channel => channel.broadcasterId,
    load: (cursor, refresh) => api.followedChannels(pageRequest(context.sessionId, cursor, refresh)) });
  return <PageFrame query={query} empty="You aren’t following any channels yet"><ChannelList retryGeneration={query.imageRetryGeneration} items={query.page?.items ?? []} open={links.channel} watch={links.watch} pending={links.pending} /></PageFrame>;
}
function Categories({ context, links }: { context: QueryContext; links: Links }) {
  const query = usePage<CategorySummary>({ ...context, viewKey: "categories", identify: category => category.id,
    load: (cursor, refresh) => api.categories(pageRequest(context.sessionId, cursor, refresh)) });
  return <PageFrame query={query} empty="No categories found"><CategoryList retryGeneration={query.imageRetryGeneration} items={query.page?.items ?? []} open={links.category} /></PageFrame>;
}
function Category({ id, context, links, language }: { language: StreamLanguage | null; id: string; context: QueryContext; links: Links }) {
  const query = usePage<StreamSummary>({ ...context, viewKey: `category:${id}:${language ?? "any"}`, identify: stream => stream.streamId,
    load: async (cursor, refresh) => { const result = await api.category({ id, page: { page: pageRequest(context.sessionId, cursor, refresh), language } }); return { ...result.streams, label: result.category.name, imageUrl: result.category.imageUrl }; } });
  return <><div className="category-identity">{query.page && <><Media retryGeneration={query.imageRetryGeneration} src={query.page.imageUrl ?? null} shape="artwork" /><span>{query.page.label}</span></>}</div><PageFrame query={query} empty={language ? "No streams in this language" : "No streams are live in this category"}><StreamList retryGeneration={query.imageRetryGeneration} items={query.page?.items ?? []} {...links} /></PageFrame></>;
}
