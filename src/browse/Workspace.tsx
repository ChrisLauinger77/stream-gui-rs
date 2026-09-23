import { memo, useImperativeHandle, useLayoutEffect, useRef, useState, type Ref } from "react";
import { api } from "../lib/ipc";
import type { NavigationIntent, Settings, StreamLanguage, BrowseRequest, CategorySummary, ChannelSummary, StreamSummary } from "../lib/generated";
import { ViewMemory, usePage } from "./usePage";
import { CategoryList, ChannelList, Media, PageFrame, StreamList } from "./components";
import { LocalItemActions, SavedItems } from "../features/DiscoveryPreferences";
import { TeamView } from "./TeamView";
import { ExactChannelLookup } from "./ExactChannelLookup";
import { LanguageFilter } from "./LanguageFilter";
import { SearchView, ChannelView, LoginChannelView } from "./details";

type Section = "following" | "live" | "categories" | "search" | "lookup" | "bookmarks";
export type BrowserActions = { intent: (intent: NavigationIntent) => void; channel: (id: string, name: string) => void; navigate: (section: Section) => void; back: () => void; forward: () => void; refresh: () => void; focus: () => void };
type Route = ({ kind: Section } | { kind: "category" | "channel" | "team" | "login"; id: string; name: string }) & { language?: StreamLanguage | null };
type Visit = { route: Route; section: Section; scroll: number; focus?: string;
  lookup: string; search: string; searchType: "channels" | "categories" | "teams"; following: "live" | "channels" };
const routeKey = (route: Route) => route.kind + ("id" in route ? `:${route.id}` : "") + (route.kind === "live" || route.kind === "category" ? `:${route.language ?? "any"}` : "");
export type QueryContext = { sessionId: string; memory: ViewMemory; hidden?: Settings["discovery"]["hidden"]; onAuthLost: () => void };
export type Watch = { watch: (id: string) => void; pending: ReadonlySet<string> };
export type Links = Watch & { team: (name: string) => void; channel: (id: string, name: string) => void; category: (id: string, name: string) => void };
export const pageRequest = (sessionId: string, cursor: string | null, refresh: boolean): BrowseRequest => ({ sessionId, cursor, refresh });
export const BrowserWorkspace = memo(function BrowserWorkspace({ sessionId, onAuthLost, watch, pending, actionsRef, preferences, saveLanguage }: { preferences: Settings | null; saveLanguage: (language: StreamLanguage | null) => Promise<Settings>; sessionId: string; onAuthLost: () => void; actionsRef: Ref<BrowserActions> } & Watch) {
  const memory = useRef(new ViewMemory()).current;
  const [route, setRoute] = useState<Route>({ kind: "following" });
  const [section, setSection] = useState<Section>("following");
  const [history, setHistory] = useState<Visit[]>([]);
  const [future, setFuture] = useState<Visit[]>([]);
  const [following, setFollowing] = useState<"live" | "channels">("live");
  const [lookup, setLookup] = useState("");
  const [search, setSearch] = useState("");
  const [searchType, setSearchType] = useState<"channels" | "categories" | "teams">("channels");
  const content = useRef<HTMLElement>(null);
  const focusSearch = useRef(false);
  const filterChange = useRef(false);
  const restore = useRef<{ scroll: number; focus?: string } | null>(null);
  const visit = (): Visit => ({ route, section, lookup, search, searchType, following, scroll: content.current?.scrollTop ?? 0, focus: (document.activeElement as HTMLElement | null)?.dataset.focus });
  const acceptVisit = (previous: Visit) => {
    restore.current = previous; setSection(previous.section); setRoute(previous.route);
    setLookup(previous.lookup); setSearch(previous.search); setSearchType(previous.searchType); setFollowing(previous.following);
  };
  const navigate = (next: Route) => {
    if (next.kind === "live" || next.kind === "category") next = { ...next, language: preferences?.discoveryLanguage ?? null };
    if (routeKey(next) === routeKey(route)) return;
    setHistory(items => [...items, visit()].slice(-12)); setFuture([]);
    if (next.kind !== "category" && next.kind !== "channel" && next.kind !== "team" && next.kind !== "login") setSection(next.kind);
    restore.current = null; setRoute(next);
  };
  const back = () => {
    const previous = history.at(-1);
    if (!previous) return;
    setFuture(items => [...items, visit()].slice(-12)); setHistory(history.slice(0, -1)); acceptVisit(previous);
  };
  const forward = () => {
    const next = future.at(-1);
    if (!next) return;
    setHistory(items => [...items, visit()].slice(-12)); setFuture(future.slice(0, -1)); acceptVisit(next);
  };
  useImperativeHandle(actionsRef, () => ({
    intent: intent => {
      if (intent.kind === "channel") navigate({ kind: "login", id: intent.login, name: intent.login });
      else if (intent.kind === "category") navigate({ kind: "category", id: intent.id, name: "Category" });
      else if (intent.kind === "team") navigate({ kind: "team", id: intent.name, name: intent.name });
    },
    channel: (id, name) => navigate({ kind: "channel", id, name }),
    navigate: next => {
      focusSearch.current = next === "search";
      if (next === "search" && route.kind === "search") { content.current?.querySelector<HTMLInputElement>('input[type="search"]')?.focus(); focusSearch.current = false; }
      else navigate({ kind: next });
    },
    back, forward,
    refresh: () => content.current?.querySelector<HTMLButtonElement>("[data-browse-refresh]")?.click(),
    focus: () => content.current?.querySelector<HTMLElement>("h1")?.focus({ preventScroll: true }),
  }));
  useLayoutEffect(() => {
    if (filterChange.current) { filterChange.current = false; if (content.current) content.current.scrollTop = 0; return; }
    const target = restore.current;
    const focus = target?.focus ? [...(content.current?.querySelectorAll<HTMLElement>("[data-focus]") ?? [])].find(el => el.dataset.focus === target.focus && !el.matches(":disabled") && !el.closest("[hidden]")) : undefined;
    (focus ?? content.current?.querySelector<HTMLElement>("h1"))?.focus({ preventScroll: true });
    if (content.current) content.current.scrollTop = target?.scroll ?? 0;
    if (focusSearch.current && route.kind === "search") { content.current?.querySelector<HTMLInputElement>('input[type="search"]')?.focus(); focusSearch.current = false; }
  }, [route]);
  const context = { sessionId, memory, onAuthLost, hidden: preferences?.discovery.hidden };
  const language = route.language ?? null;
  const links: Links = { watch, pending, team: name => navigate({ kind: "team", id: name.toLowerCase(), name }), channel: (id, name) => navigate({ kind: "channel", id, name }), category: (id, name) => navigate({ kind: "category", id, name }) };
  const title = "name" in route ? route.name : ({ following: "Following", live: "Live now", categories: "Categories", search: "Search", lookup: "Open channel", bookmarks: "Bookmarks" }[route.kind]);
  const subtitle = { following: "The channels you choose to keep up with.", live: "Popular streams, happening right now.", categories: "Find a game. Find your community.", search: "Discover channels and categories on Twitch.", category: "Live streams in this category.", lookup: "Go directly to a known Twitch login.", login: "Channel details", team: "Twitch team members", channel: "Channel details", bookmarks: "Your saved channels and categories, on this device." }[route.kind];
  return <div className="workspace">
    <nav className="side-nav" aria-label="Main navigation"><p className="nav-label">BROWSE</p>{([['following','Following','♡'],['live','Live','◉'],['categories','Categories','▦'],['search','Search','⌕'],['lookup','Open channel','→'],['bookmarks','Bookmarks','☆']] as const).map(([kind, label, symbol]) => <button key={kind} aria-label={label} aria-current={section === kind ? "page" : undefined} onClick={() => navigate({ kind })}><span className="nav-symbol" aria-hidden="true">{symbol}</span>{label}</button>)}</nav>
    <main className="browse-content" ref={content}>
      <div className="view-top"><button className="quiet" disabled={!history.length} onClick={back} aria-label="Go back" data-focus="navigation:back">← Back</button><button className="quiet" disabled={!future.length} onClick={forward} aria-label="Go forward" data-focus="navigation:forward">Forward →</button><button className="quiet" onClick={() => navigate({ kind: "following" })} aria-label="Go home" data-focus="navigation:home">Home</button><span className="muted">{section === "following" ? "Your Twitch" : "Explore Twitch"}</span></div>
      <div className="view-heading"><div><h1 tabIndex={-1}>{title}</h1><p>{subtitle}</p></div></div>
      {(route.kind === "live" || route.kind === "category") && <LanguageFilter value={language} persist={saveLanguage} saved={value => {
        setFuture([]);
        setRoute(current => {
          if (current !== route) return current;
          memory.forget(routeKey({ ...current, language: value.discoveryLanguage }));
          filterChange.current = true;
          return { ...current, language: value.discoveryLanguage };
        });
      }} />}
      {route.kind === "following" && <><div className="tabs" aria-label="Following views"><button aria-pressed={following === "live"} onClick={() => { setFuture([]); setFollowing("live"); }}>Live streams</button><button aria-pressed={following === "channels"} onClick={() => { setFuture([]); setFollowing("channels"); }}>All channels</button></div>{following === "live" ? <Streams key="followed" mode="followed" context={context} links={links} /> : <Follows context={context} links={links} />}</>}
      {route.kind === "live" && <Streams key={`live:${language}`} mode="live" language={language} context={context} links={links} />}
      {route.kind === "categories" && <Categories context={context} links={links} />}
      {route.kind === "category" && <Category key={`${route.id}:${language}`} id={route.id} language={language} context={context} links={links} />}
      {route.kind === "bookmarks" && <SavedItems list="bookmarks" open={(kind, id, name) => navigate({ kind, id, name })} />}
      {route.kind === "lookup" && <ExactChannelLookup sessionId={sessionId} login={lookup} change={value => { setFuture([]); setLookup(value); }} open={links.channel} onAuthLost={onAuthLost} />}
      {route.kind === "search" && <SearchView context={context} links={links} draft={search} setDraft={value => { setFuture([]); setSearch(value); }} type={searchType} setType={value => { setFuture([]); setSearchType(value); }} />}
      {route.kind === "team" && <TeamView key={route.id} name={route.id} context={context} links={links} />}
      {route.kind === "login" && <LoginChannelView key={route.id} login={route.id} context={context} links={links} />}
      {route.kind === "channel" && <ChannelView key={route.id} id={route.id} context={context} links={links} />}
    </main>
  </div>;
});
function Streams({ mode, context, links, language = null }: { language?: StreamLanguage | null; mode: "live" | "followed"; context: QueryContext; links: Links }) {
  const query = usePage<StreamSummary>({ ...context, viewKey: mode === "live" ? `live:${language ?? "any"}` : mode,
    identify: stream => stream.streamId,
    load: (cursor, refresh) => mode === "followed" ? api.followedStreams(pageRequest(context.sessionId, cursor, refresh)) : api.streams({ page: pageRequest(context.sessionId, cursor, refresh), language }) });
  const visible = mode === "live" ? filtered(query, stream => !context.hidden?.some(item => item.kind === "channel" ? item.id === stream.broadcasterId : item.id === stream.categoryId)) : query;
  return <PageFrame query={visible} empty={mode === "followed" ? "No followed channels are live right now" : language ? "No streams in this language" : "No live streams found"}><StreamList retryGeneration={query.imageRetryGeneration} items={visible.page?.items ?? []} {...links} /></PageFrame>;
}
function Follows({ context, links }: { context: QueryContext; links: Links }) {
  const query = usePage<ChannelSummary>({ ...context, viewKey: "followed-channels", identify: channel => channel.broadcasterId,
    load: (cursor, refresh) => api.followedChannels(pageRequest(context.sessionId, cursor, refresh)) });
  return <PageFrame query={query} empty="You aren’t following any channels yet"><ChannelList retryGeneration={query.imageRetryGeneration} items={query.page?.items ?? []} open={links.channel} watch={links.watch} pending={links.pending} /></PageFrame>;
}
function Categories({ context, links }: { context: QueryContext; links: Links }) {
  const query = usePage<CategorySummary>({ ...context, viewKey: "categories", identify: category => category.id,
    load: (cursor, refresh) => api.categories(pageRequest(context.sessionId, cursor, refresh)) });
  const visible = filtered(query, category => !context.hidden?.some(item => item.kind === "category" && item.id === category.id));
  return <PageFrame query={visible} empty="No visible categories found"><CategoryList retryGeneration={query.imageRetryGeneration} items={visible.page?.items ?? []} open={links.category} /></PageFrame>;
}
function Category({ id, context, links, language }: { language: StreamLanguage | null; id: string; context: QueryContext; links: Links }) {
  const query = usePage<StreamSummary>({ ...context, viewKey: `category:${id}:${language ?? "any"}`, identify: stream => stream.streamId,
    load: async (cursor, refresh) => { const result = await api.category({ id, page: { page: pageRequest(context.sessionId, cursor, refresh), language } }); return { ...result.streams, label: result.category.name, imageUrl: result.category.imageUrl }; } });
  const visible = filtered(query, stream => !context.hidden?.some(item => item.kind === "channel" && item.id === stream.broadcasterId));
  return <><div className="category-identity">{query.page && <><Media retryGeneration={query.imageRetryGeneration} src={query.page.imageUrl ?? null} shape="artwork" /><span>{query.page.label}</span></>}</div>{query.page?.label && <LocalItemActions kind="category" id={id} name={query.page.label} />}<PageFrame query={visible} empty={language ? "No streams in this language" : "No streams are live in this category"}><StreamList retryGeneration={query.imageRetryGeneration} items={visible.page?.items ?? []} {...links} /></PageFrame></>;
}

function filtered<T>(query: ReturnType<typeof usePage<T>>, keep: (item: T) => boolean) {
  return { ...query, page: query.page ? { ...query.page, items: query.page.items.filter(keep) } : undefined };
}
