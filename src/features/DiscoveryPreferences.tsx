import { useEffect, useRef, useState } from "react";
import type { DiscoveryList, ErrorCode, ItemKind, SavedItem } from "../lib/generated";
import { useSettings } from "../settings/useSettings";
import { errorCode, errorText } from "../browse/errors";
import { useI18n, type MessageKey } from "../i18n";

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
export function LocalItemActions({ kind, id, name }: SavedItem) {
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
export function SavedItems({ list, open }: { list: DiscoveryList; open?: (kind: ItemKind, id: string, name: string) => void }) {
  const { t } = useI18n();
  const { settings, pending, change, feedback } = useDiscoveryMutation();
  const region = useRef<HTMLDivElement>(null);
  const items = settings?.discovery[list] ?? [];
  return <div ref={region} tabIndex={-1} className="saved-items" role="region" aria-label={list === "bookmarks" ? t("discovery.localBookmarks") : t("settings.sectionHidden")}>
    <p className="muted">{list === "bookmarks" ? t("discovery.bookmarksHelp") : t("discovery.hiddenHelp")}</p>
    {!items.length && <p>{list === "bookmarks" ? t("discovery.noBookmarks") : t("discovery.noHidden")}</p>}
    {items.map(item => <div className="saved-item" key={`${item.kind}:${item.id}`}>
      {open ? <button type="button" className="text-button" data-focus={`saved:${item.kind}:${item.id}`} onClick={() => open(item.kind, item.id, item.name)}>{item.name}</button> : <span>{item.name}</span>}
      <span className="muted">{t(item.kind === "channel" ? "discovery.channel" : "discovery.category")} · {item.id}</span>
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
