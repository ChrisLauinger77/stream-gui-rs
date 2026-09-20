import { useCallback, useEffect, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { api } from "../lib/ipc";
import { errorCode, friendlyError } from "../browse/errors";
import type { SessionSnapshot, Settings, StreamLanguage } from "../lib/generated";

export function playbackError(error: unknown) {
  const code = errorCode(error);
  if (code === "capacity") return "Playback is busy or eight streams are already active. Stop a stream or try again shortly.";
  if (code === "timeout") return "The playback operation took too long. Check Watching and test Streamlink in Settings.";
  if (code === "not_found") return "This session is no longer in the playback history.";
  if (code === "settings") return "Playback settings could not be saved or read. Check the settings file and try again.";
  if (code === "invalid_input") return "Check the executable path and playback selection, then try again.";
  return friendlyError(error);
}

// Only Rust snapshots describe processes. Pending keys describe requests, never
// a competing process state. One poll at a time; old polls cannot undo actions.
export function usePlayback() {
  const [sessions, setSessions] = useState<SessionSnapshot[]>([]);
  const [settings, updateSettings] = useState<Settings | null>(null);
  const [savingSettings, setSavingSettings] = useState(false);
  const settingsInFlight = useRef(false);
  const [pending, setPending] = useState<ReadonlySet<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const acceptedSettingsRevision = useRef(0);
  const settingsTail = useRef(Promise.resolve());
  const settingsQueued = useRef(0);
  const mutateSettings = useCallback((action: () => Promise<Settings>) => {
    // Serialize this window's mutations so complete Rust snapshots can be accepted
    // in order. Navigation cannot reset the queue; retain at most eight intents.
    if (settingsQueued.current >= 8) return Promise.reject({ code: "capacity" });
    settingsQueued.current++;
    const mutation = settingsTail.current.then(async () => {
      if (!mounted.current) throw { code: "cancelled" };
      const value = await action();
      if (mounted.current) { acceptedSettingsRevision.current++; updateSettings(value); }
      return value;
    });
    settingsTail.current = mutation.then(() => {}, () => {}).finally(() => { settingsQueued.current--; });
    return mutation;
  }, []);
  const commitSettings = useCallback(async (action: () => Promise<Settings>) => {
    if (settingsInFlight.current) throw { code: "capacity" };
    settingsInFlight.current = true; setSavingSettings(true);
    try {
      return await mutateSettings(action);
    } finally {
      settingsInFlight.current = false;
      if (mounted.current) setSavingSettings(false);
    }
  }, [mutateSettings]);
  const saveLanguage = useCallback((language: StreamLanguage | null) =>
    mutateSettings(() => api.saveDiscoveryLanguage(language)), [mutateSettings]);
  const revision = useRef(0);
  const inFlight = useRef(new Set<string>());
  const mounted = useRef(false);
  useEffect(() => {
    mounted.current = true;
    if (!isTauri()) return () => { mounted.current = false; };
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout>;
    const initialRevision = acceptedSettingsRevision.current;
    void api.playbackSettings().then(value => {
      if (!cancelled && acceptedSettingsRevision.current === initialRevision) updateSettings(value);
    }).catch(error => {
      if (!cancelled && acceptedSettingsRevision.current === initialRevision) setError(playbackError(error));
    });
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
        else if (!key.startsWith("stop:") && ["stopping", "exited"].includes(snapshot.phase)) setMessage("The Streamlink process has already stopped or is stopping.");
        else if (key.startsWith("stop:") && !["exited", "failed"].includes(snapshot.phase)) setMessage("Stopping the Streamlink process…");
        else setMessage(success);
      }
    } catch (error) { if (mounted.current) setError(playbackError(error)); }
    finally {
      revision.current++; inFlight.current.delete(key);
      if (mounted.current) setPending(new Set(inFlight.current));
    }
  }, []);
  return { sessions, settings, commitSettings, savingSettings, saveLanguage, pending, run, error, message, dismiss: () => { setError(null); setMessage(null); } };
}
