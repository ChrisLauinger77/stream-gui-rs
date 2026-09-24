import { useCallback, useEffect, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { api } from "../lib/ipc";
import { errorCode, friendlyError } from "../browse/errors";
import { currentLocale, translate } from "../i18n";
import { useSettings } from "../settings/useSettings";
import type { SessionSnapshot } from "../lib/generated";

export function playbackError(error: unknown) {
  const code = errorCode(error);
  if (code === "capacity") return translate(currentLocale(), "playbackErrors.capacity");
  if (code === "timeout") return translate(currentLocale(), "playbackErrors.timeout");
  if (code === "not_found") return translate(currentLocale(), "playbackErrors.notFound");
  if (code === "settings") return translate(currentLocale(), "playbackErrors.settings");
  if (code === "invalid_input") return translate(currentLocale(), "playbackErrors.invalidInput");
  return friendlyError(error);
}

// Only Rust snapshots describe processes. Pending keys describe requests, never
// a competing process state. One poll at a time; old polls cannot undo actions.
export function usePlayback() {
  const [sessions, setSessions] = useState<SessionSnapshot[]>([]);
  const preferences = useSettings();
  const [pending, setPending] = useState<ReadonlySet<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const revision = useRef(0);
  const inFlight = useRef(new Set<string>());
  const mounted = useRef(false);
  useEffect(() => {
    mounted.current = true;
    if (!isTauri()) return () => { mounted.current = false; };
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      const version = revision.current;
      try {
        const values = await api.sessions();
        if (!cancelled && version === revision.current) setSessions(values);
      } catch (error) { if (!cancelled && version === revision.current) setError(playbackError(error)); }
      if (!cancelled) timer = setTimeout(() => { void poll(); }, 1000);
    };
    void poll();
    return () => { cancelled = true; mounted.current = false; clearTimeout(timer); };
  }, []);
  const run = useCallback(async (key: string, action: () => Promise<SessionSnapshot>, success: string) => {
    if (inFlight.current.has(key)) return;
    inFlight.current.add(key); revision.current++;
    setPending(new Set(inFlight.current)); setError(null); setMessage(null);
    try {
      const snapshot = await action();
      if (mounted.current) {
        setSessions(items => {
          const old = items.find(item => item.id === snapshot.id);
          if (old && old.generation > snapshot.generation) return items;
          return old ? items.map(item => item.id === snapshot.id ? snapshot : item) : [...items, snapshot].slice(-16);
        });
        if (snapshot.failure || snapshot.phase === "failed") setError(playbackError({ code: snapshot.failure ?? "process_failed" }));
        else if (!key.startsWith("stop:") && ["stopping", "exited"].includes(snapshot.phase)) setMessage(translate(currentLocale(), "playback.alreadyStopped"));
        else if (key.startsWith("stop:") && !["exited", "failed"].includes(snapshot.phase)) setMessage(translate(currentLocale(), "playback.stopping"));
        else setMessage(success);
      }
    } catch (error) { if (mounted.current) setError(playbackError(error)); }
    finally {
      revision.current++; inFlight.current.delete(key);
      if (mounted.current) setPending(new Set(inFlight.current));
    }
  }, []);
  return { ...preferences, sessions, pending, run, error: error ?? (preferences.error ? playbackError(preferences.error) : null), message, dismiss: () => { setError(null); setMessage(null); preferences.dismiss(); } };
}
