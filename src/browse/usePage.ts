import { useEffect, useRef, useState } from "react";
import type { ErrorCode, PagedResult } from "../lib/generated";
import { errorCode } from "./errors";

export type ViewPage<T> = PagedResult<T> & { label?: string; imageUrl?: string | null };
type Snapshot<T> = { page: ViewPage<T>; cursors: string[]; pages: number; limited: boolean };
// Navigation snapshots only: no TTL or fresh-cache decisions in React. Every
// new query/refresh/load-more goes through Rust. Session unmount drops all data.
export class ViewMemory {
  private entries = new Map<string, unknown>();
  read<T>(key: string) { return this.entries.get(key) as Snapshot<T> | undefined; }
  write<T>(key: string, value: Snapshot<T>) {
    this.entries.delete(key); this.entries.set(key, value);
    while (this.entries.size > 12) this.entries.delete(this.entries.keys().next().value!);
  }
}
export function usePage<T>({ viewKey, memory, load, identify, onAuthLost }: {
  viewKey: string; memory: ViewMemory;
  load: (cursor: string | null, refresh: boolean) => Promise<ViewPage<T>>;
  identify: (item: T) => string; onAuthLost: () => void;
}) {
  const [snapshot, setSnapshot] = useState(() => memory.read<T>(viewKey));
  const [pending, setPending] = useState(false);
  const [imageRetryGeneration, setImageRetryGeneration] = useState(0);
  const [error, setError] = useState<ErrorCode | null>(null);
  const [retained, setRetained] = useState(() => !!memory.read<T>(viewKey));
  const current = useRef(snapshot); current.current = snapshot;
  const loader = useRef(load); loader.current = load;
  const authLost = useRef(onAuthLost); authLost.current = onAuthLost;
  const identifyRef = useRef(identify); identifyRef.current = identify;
  const alive = useRef(true);
  const busy = useRef(false);
  const generation = useRef(0);
  const lastMore = useRef(false);
  const request = async (more: boolean, refresh: boolean) => {
    if (busy.current) return;
    const previous = current.current;
    if (more && (!previous?.page.cursor || previous.limited)) return;
    busy.current = true; setPending(true); setError(null); lastMore.current = more;
    const version = ++generation.current;
    const cursor = more ? previous!.page.cursor : null;
    try {
      const response = await loader.current(cursor, refresh);
      if (!alive.current || version !== generation.current) return;
      const cursors = more ? [...previous!.cursors, cursor!] : [];
      const pages = more ? previous!.pages + 1 : 1;
      const repeated = !!response.cursor && (response.cursor === cursor || cursors.includes(response.cursor));
      const items = [...new Map((more ? [...previous!.page.items, ...response.items] : response.items).map(item => [identifyRef.current(item), item])).values()].slice(0, 300);
      const next: Snapshot<T> = { page: { ...response, items,
        cursor: repeated || pages >= 10 ? null : response.cursor,
        warnings: [...new Set([...(more ? previous!.page.warnings : []), ...response.warnings, ...(repeated ? ["invalid_response" as const] : [])])],
      }, cursors, pages, limited: pages >= 10 && !!response.cursor };
      current.current = next; setSnapshot(next); memory.write(viewKey, next);
      setRetained(more);
      if (refresh) setImageRetryGeneration(value => value + 1);
    } catch (failure) {
      if (!alive.current || version !== generation.current) return;
      const code = errorCode(failure); setError(code);
      if (code === "unauthenticated" || code === "auth_invalid") authLost.current();
    } finally {
      if (alive.current && version === generation.current) { busy.current = false; setPending(false); }
    }
  };
  useEffect(() => {
    alive.current = true;
    let disposed = false;
    // Defer dispatch so StrictMode's mount/cleanup probe does not send twice.
    if (!current.current) queueMicrotask(() => { if (!disposed) void request(false, false); });
    return () => { disposed = true; alive.current = false; generation.current++; busy.current = false; };
    // A keyed view owns exactly one query; loader changes do not refetch it.
  }, [viewKey]);
  return { page: snapshot?.page, pending, error, retained, imageRetryGeneration, limited: snapshot?.limited ?? false,
    refresh: () => { void request(false, true); }, more: () => { void request(true, false); },
    retry: () => { void request(lastMore.current, !lastMore.current); } };
}
