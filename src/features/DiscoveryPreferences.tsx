import { useEffect, useRef, useState } from "react";
import type { DiscoveryList, ErrorCode, ItemKind, SavedItem } from "../lib/generated";
import { useSettings } from "../settings/useSettings";
import { errorCode, errorText } from "../browse/errors";
import { useI18n, type MessageKey } from "../i18n";
import { KickChatButton, YoutubeChatButton, youtubeVideoId } from "./BrowserChatButton";

type TwitchItemKind = Extract<ItemKind, "channel" | "category">;
type AnonymousItemKind = Extract<ItemKind, "kick_channel" | "youtube">;

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

export function SavedItems({ list, open, watchKick, watchYoutube }: { list: DiscoveryList; open?: (kind: TwitchItemKind, id: string, name: string) => void; watchKick?: (slug: string) => Promise<void>; watchYoutube?: (locator: string) => Promise<void> }) {
  const { t } = useI18n();
  const { settings, pending, change, feedback } = useDiscoveryMutation();
  const region = useRef<HTMLDivElement>(null);
  const items = settings?.discovery[list] ?? [];
  return <div ref={region} tabIndex={-1} className="saved-items" role="region" aria-label={list === "bookmarks" ? t("discovery.localBookmarks") : t("settings.sectionHidden")}>
    <p className="muted">{list === "bookmarks" ? t("discovery.bookmarksHelp") : t("discovery.hiddenHelp")}</p>
    {list === "bookmarks" && <p className="muted">{t("discovery.anonymousBookmarksHelp")}</p>}
    {!items.length && <p>{list === "bookmarks" ? t("discovery.noBookmarks") : t("discovery.noHidden")}</p>}
    {items.map(item => <div className="saved-item" key={`${item.kind}:${item.id}`}>
      {open && (item.kind === "channel" || item.kind === "category") ? <button type="button" className="text-button" data-focus={`saved:${item.kind}:${item.id}`} onClick={() => { if (item.kind === "channel" || item.kind === "category") open(item.kind, item.id, item.name); }}>{item.name}</button> : <span>{item.name}</span>}
      <span className="muted">{t(({ channel: "discovery.channel", category: "discovery.category", kick_channel: "service.kick", youtube: "service.youtube" } as const)[item.kind])} · {item.id}</span>
      {item.kind === "kick_channel" && <>{watchKick && <SavedPlayback item={item} watch={watchKick} />}<KickChatButton slug={item.id} /></>}
      {item.kind === "youtube" && <>{watchYoutube && <SavedPlayback item={item} watch={watchYoutube} />}{youtubeVideoId(item.id) && <YoutubeChatButton locator={item.id} />}</>}
      {list === "bookmarks" && !open && (item.kind === "channel" || item.kind === "category") && <span className="muted">{t("discovery.twitchSignIn")}</span>}
      <button type="button" disabled={pending} aria-label={t(list === "bookmarks" ? "discovery.removeNamed" : "discovery.restoreNamed", { name: item.name })} onClick={event => {
        const button = event.currentTarget;
        void change(list, item, false).then(() => {
          if (!button.isConnected && document.activeElement === document.body) region.current?.focus();
        });
      }}>{list === "bookmarks" ? t("discovery.remove") : t("discovery.restore")}</button>
    </div>)}
    {feedback}
  </div>;
}
