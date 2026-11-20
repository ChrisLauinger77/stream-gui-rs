import { useEffect, useId, useRef, useState } from "react";
import type { DiscoveryList, ErrorCode, ItemKind, SavedItem } from "../lib/generated";
import { useSettings } from "../settings/useSettings";
import { errorCode, errorText } from "../browse/errors";
import { useI18n, type MessageKey } from "../i18n";
import { KickChatButton, YoutubeChatButton, youtubeVideoId } from "./BrowserChatButton";

type TwitchItemKind = Extract<ItemKind, "channel" | "category">;
type AnonymousItemKind = Extract<ItemKind, "kick_channel" | "youtube">;

const bookmarkGroupLabels = [
  ["category", "browse.categories"],
  ["channel", "discovery.twitchChannels"],
  ["kick_channel", "service.kick"],
  ["youtube", "service.youtube"],
] as const;

export function useBookmarkGroups() {
  const [collapsed, setCollapsed] = useState<ItemKind[]>([]);
  return {
    collapsed,
    toggle(kind: ItemKind) {
      setCollapsed(current => current.includes(kind) ? current.filter(value => value !== kind) : [...current, kind]);
    },
  };
}

function useDiscoveryMutation() {
  const { t, locale } = useI18n();
  const { settings, modifyDiscovery } = useSettings();
  const [pending, setPending] = useState(false);
  const [message, setMessage] = useState<MessageKey | null>(null);
  const [error, setError] = useState<ErrorCode | null>(null);
  const busy = useRef(false);
  const alive = useRef(true);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  const change = async (list: DiscoveryList, item: SavedItem, present: boolean) => {
    if (busy.current) return;
    busy.current = true; setPending(true); setError(null); setMessage(null);
    try {
      await modifyDiscovery({ list, item, present });
      if (alive.current) setMessage(list === "bookmarks" ? present ? "discovery.bookmarkSaved" : "discovery.bookmarkRemoved" : present ? "discovery.hidden" : "discovery.restored");
    } catch (failure) { if (alive.current) setError(errorCode(failure)); }
    finally { busy.current = false; if (alive.current) setPending(false); }
  };
  return { settings, pending, change, feedback: <><p role="status">{message && t(message)}</p>{error && <p role="alert" className="error">{errorText(error, locale)}</p>}</> };
}
export function LocalItemActions({ kind, id, name }: SavedItem & { kind: TwitchItemKind }) {
  const { t } = useI18n();
  const { settings, pending, change, feedback } = useDiscoveryMutation();
  const item = { kind, id, name };
  const has = (list: DiscoveryList) => settings?.discovery[list].some(entry => entry.kind === kind && entry.id === id) ?? false;
  return <div className="local-item-actions">
    <button type="button" disabled={!settings || pending} data-focus={`bookmark:${kind}:${id}`} aria-pressed={has("bookmarks")} onClick={() => { void change("bookmarks", item, !has("bookmarks")); }}>{has("bookmarks") ? t("discovery.removeBookmark") : t(kind === "channel" ? "discovery.bookmarkChannel" : "discovery.bookmarkCategory")}</button>
    <button type="button" disabled={!settings || pending} data-focus={`hide:${kind}:${id}`} aria-pressed={has("hidden")} onClick={() => { void change("hidden", item, !has("hidden")); }}>{has("hidden") ? t("discovery.restoreToDiscovery") : t(kind === "channel" ? "discovery.hideChannel" : "discovery.hideCategory")}</button>
    {feedback}
  </div>;
}
export function AnonymousBookmark({ kind, locator }: { kind: AnonymousItemKind; locator: string }) {
  const { t } = useI18n();
  const { settings, pending, change, feedback } = useDiscoveryMutation();
  // Match the Rust canonical spelling for presentation; Rust validates on save.
  const id = kind === "kick_channel" || locator.startsWith("@") ? locator.toLowerCase() : locator;
  const present = settings?.discovery.bookmarks.some(item => item.kind === kind && item.id === id) ?? false;
  return <div className="anonymous-bookmark">
    <button type="button" disabled={!settings || pending || !locator} aria-pressed={present} onClick={() => { void change("bookmarks", { kind, id, name: id }, !present); }}>{t(present ? "discovery.removeBookmark" : "discovery.bookmarkLocator")}</button>
    {feedback}
  </div>;
}

function SavedPlayback({ item, watch }: { item: SavedItem; watch: (locator: string) => Promise<void> }) {
  const { t } = useI18n();
  const [pending, setPending] = useState(false);
  const busy = useRef(false);
  const alive = useRef(true);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  const play = async () => {
    if (busy.current) return;
    busy.current = true; setPending(true);
    try { await watch(item.id); }
    finally { busy.current = false; if (alive.current) setPending(false); }
  };
  return <button type="button" disabled={pending} data-focus={`saved:${item.kind}:${item.id}`} onClick={() => { void play(); }}>{t(item.kind === "kick_channel" ? "kick.play" : "youtube.play")}</button>;
}

export function SavedItems({ list, open, watchKick, watchYoutube, bookmarkGroups }: {
  list: DiscoveryList;
  open?: (kind: TwitchItemKind, id: string, name: string) => void;
  watchKick?: (slug: string) => Promise<void>;
  watchYoutube?: (locator: string) => Promise<void>;
  bookmarkGroups?: ReturnType<typeof useBookmarkGroups>;
}) {
  const { t, locale, number } = useI18n();
  const { settings, pending, change, feedback } = useDiscoveryMutation();
  const region = useRef<HTMLDivElement>(null);
  const groupId = useId();
  const localGroups = useBookmarkGroups();
  const groups = bookmarkGroups ?? localGroups;
  const items = settings?.discovery[list] ?? [];
  const bookmarks = list === "bookmarks";
  const { compare } = new Intl.Collator(locale, { sensitivity: "base", numeric: true });
  const renderItem = (item: SavedItem) => {
    const { kind, id, name } = item;
    const twitch = kind === "channel" || kind === "category";
    return <div className="saved-item" key={`${kind}:${id}`}>
      {open && twitch ? <button type="button" className="text-button" data-focus={`saved:${kind}:${id}`} onClick={() => open(kind, id, name)}>{name}</button> : <span>{name}</span>}
      {!bookmarks && twitch && <span className="muted">{t(kind === "channel" ? "discovery.channel" : "discovery.category")} · {id}</span>}
      {bookmarks && (twitch || id !== name) && <span className="muted">{id}</span>}
      {item.kind === "kick_channel" && <>{watchKick && <SavedPlayback item={item} watch={watchKick} />}<KickChatButton slug={item.id} /></>}
      {item.kind === "youtube" && <>{watchYoutube && <SavedPlayback item={item} watch={watchYoutube} />}{youtubeVideoId(item.id) && <YoutubeChatButton locator={item.id} />}</>}
      {bookmarks && !open && twitch && <span className="muted">{t("discovery.twitchSignIn")}</span>}
      <button type="button" disabled={pending} aria-label={t(bookmarks ? "discovery.removeNamed" : "discovery.restoreNamed", { name })} onClick={event => {
        const button = event.currentTarget;
        void change(list, item, false).then(() => {
          if (!button.isConnected && document.activeElement === document.body) region.current?.focus();
        });
      }}>{t(bookmarks ? "discovery.remove" : "discovery.restore")}</button>
    </div>;
  };
  return <div ref={region} tabIndex={-1} className="saved-items" role="region" aria-label={t(bookmarks ? "discovery.localBookmarks" : "settings.sectionHidden")}>
    <p className="muted">{t(bookmarks ? "discovery.bookmarksHelp" : "discovery.hiddenHelp")}</p>
    {bookmarks && <p className="muted">{t("discovery.anonymousBookmarksHelp")}</p>}
    {!items.length && <p>{t(bookmarks ? "discovery.noBookmarks" : "discovery.noHidden")}</p>}
    {bookmarks ? bookmarkGroupLabels.map(([kind, label]) => {
      const entries = items.filter(item => item.kind === kind).sort((a, b) => compare(a.name, b.name));
      if (!entries.length) return null;
      const expanded = !groups.collapsed.includes(kind);
      const id = `${groupId}-${kind}`;
      return <div className="bookmark-group" key={kind}>
        <button type="button" className="bookmark-group-toggle" aria-expanded={expanded} aria-controls={id} data-focus={`bookmark-group:${kind}`} onClick={() => groups.toggle(kind)}>
          {t(label)} <span className="muted">({number(entries.length)})</span>
        </button>
        <div className="bookmark-group-items" id={id} hidden={!expanded}>{entries.map(renderItem)}</div>
      </div>;
    }) : items.map(renderItem)}
    {feedback}
  </div>;
}
