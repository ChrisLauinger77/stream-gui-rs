import { useEffect, useRef, useState } from "react";
import type { DiscoveryList, ItemKind, SavedItem } from "../lib/generated";
import { useSettings } from "../settings/useSettings";
import { friendlyError } from "../browse/errors";

function useDiscoveryMutation() {
  const { settings, modifyDiscovery } = useSettings();
  const [pending, setPending] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const busy = useRef(false);
  const alive = useRef(true);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  const change = async (list: DiscoveryList, item: SavedItem, present: boolean) => {
    if (busy.current) return;
    busy.current = true; setPending(true); setError(null); setMessage(null);
    try {
      await modifyDiscovery({ list, item, present });
      if (alive.current) setMessage(list === "bookmarks" ? present ? "Bookmark saved locally." : "Bookmark removed." : present ? "Hidden from discovery. Direct navigation remains available." : "Restored to discovery.");
    } catch (failure) { if (alive.current) setError(friendlyError(failure)); }
    finally { busy.current = false; if (alive.current) setPending(false); }
  };
  return { settings, pending, change, feedback: <><p role="status">{message}</p>{error && <p role="alert" className="error">{error}</p>}</> };
}
export function LocalItemActions({ kind, id, name }: SavedItem) {
  const { settings, pending, change, feedback } = useDiscoveryMutation();
  const item = { kind, id, name };
  const has = (list: DiscoveryList) => settings?.discovery[list].some(entry => entry.kind === kind && entry.id === id) ?? false;
  return <div className="local-item-actions">
    <button type="button" disabled={!settings || pending} data-focus={`bookmark:${kind}:${id}`} aria-pressed={has("bookmarks")} onClick={() => { void change("bookmarks", item, !has("bookmarks")); }}>{has("bookmarks") ? "Remove bookmark" : `Bookmark ${kind}`}</button>
    <button type="button" disabled={!settings || pending} data-focus={`hide:${kind}:${id}`} aria-pressed={has("hidden")} onClick={() => { void change("hidden", item, !has("hidden")); }}>{has("hidden") ? "Restore to discovery" : `Hide ${kind} from discovery`}</button>
    {feedback}
  </div>;
}
export function SavedItems({ list, open }: { list: DiscoveryList; open?: (kind: ItemKind, id: string, name: string) => void }) {
  const { settings, pending, change, feedback } = useDiscoveryMutation();
  const region = useRef<HTMLDivElement>(null);
  const items = settings?.discovery[list] ?? [];
  return <div ref={region} tabIndex={-1} className="saved-items" role="region" aria-label={list === "bookmarks" ? "Local bookmarks" : "Hidden items"}>
    <p className="muted">{list === "bookmarks" ? "Local bookmarks are independent of Twitch follows. Saved names may be outdated; opening an item requests current Twitch data. Hidden bookmarks remain reachable." : "Hidden channels and categories are filtered from Live and category discovery. Following, Search, Teams, bookmarks and direct navigation remain available. Restore stale items here without contacting Twitch."}</p>
    {!items.length && <p>{list === "bookmarks" ? "No bookmarks yet. Open a channel or category to bookmark it." : "No hidden items."}</p>}
    {items.map(item => <div className="saved-item" key={`${item.kind}:${item.id}`}>
      {open ? <button type="button" className="text-button" data-focus={`saved:${item.kind}:${item.id}`} onClick={() => open(item.kind, item.id, item.name)}>{item.name}</button> : <span>{item.name}</span>}
      <span className="muted">{item.kind} · {item.id}</span>
      <button type="button" disabled={pending} aria-label={`${list === "bookmarks" ? "Remove bookmark" : "Restore"} ${item.name}`} onClick={event => {
        const button = event.currentTarget;
        void change(list, item, false).then(() => {
          if (!button.isConnected && document.activeElement === document.body) region.current?.focus();
        });
      }}>{list === "bookmarks" ? "Remove" : "Restore"}</button>
    </div>)}
    {feedback}
  </div>;
}
