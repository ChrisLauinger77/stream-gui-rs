import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { api } from "../lib/ipc";
import type { DiscoveryMutation, ShortcutBindings, SaveChannelSettingsRequest, Settings, StreamLanguage } from "../lib/generated";

function useSettingsCoordinator() {
  const [settings, updateSettings] = useState<Settings | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [savingSettings, setSavingSettings] = useState(false);
  const settingsInFlight = useRef(false);
  const mounted = useRef(false);
  const accepted = useRef(0);
  const globalAccepted = useRef(false);
  const tails = useRef(new Map<string, Promise<void>>());
  const queued = useRef(0);
  const mutate = useCallback(<T,>(scope: string, action: () => Promise<T>, accept: (value: T) => void) => {
    // One bounded coordinator survives panel, workspace and Developer tools
    // remounts. Global snapshots serialize; disjoint channel writes may overlap.
    if (queued.current >= 8) return Promise.reject({ code: "capacity" });
    queued.current++;
    const mutation = (tails.current.get(scope) ?? Promise.resolve()).then(async () => {
      if (!mounted.current) throw { code: "cancelled" };
      const value = await action();
      accepted.current++;
      if (mounted.current) accept(value);
      return value;
    });
    const tail = mutation.then(() => {}, () => {}).finally(() => {
      queued.current--;
      if (tails.current.get(scope) === tail) tails.current.delete(scope);
    });
    tails.current.set(scope, tail);
    return mutation;
  }, []);
  const mutateSettings = useCallback((action: () => Promise<Settings>) => mutate("global", action, value => {
    globalAccepted.current = true; updateSettings(value); setError(null);
  }), [mutate]);
  const commitSettings = useCallback(async (action: () => Promise<Settings>) => {
    if (settingsInFlight.current) throw { code: "capacity" };
    settingsInFlight.current = true; setSavingSettings(true);
    try { return await mutateSettings(action); }
    finally {
      settingsInFlight.current = false;
      if (mounted.current) setSavingSettings(false);
    }
  }, [mutateSettings]);
  const modifyDiscovery = useCallback((request: DiscoveryMutation) => mutateSettings(() => api.modifyDiscovery(request)), [mutateSettings]);
  const saveShortcuts = useCallback((request: ShortcutBindings) => mutateSettings(() => api.saveShortcuts(request)), [mutateSettings]);
  const saveLanguage = useCallback((language: StreamLanguage | null) =>
    mutateSettings(() => api.saveDiscoveryLanguage(language)), [mutateSettings]);
  const saveChannel = useCallback((request: SaveChannelSettingsRequest) => {
    const scope = `channel:${request.broadcasterId}`;
    // Full override drafts cannot be queued behind a save of the same record.
    // A reopened view reads after that save settles before allowing another edit.
    if (tails.current.has(scope)) return Promise.reject({ code: "capacity" });
    return mutate(scope, () => api.saveChannelSettings(request), () => {});
  }, [mutate]);
  const readChannel = useCallback(async (broadcasterId: string, signal: AbortSignal) => {
    const scope = `channel:${broadcasterId}`;
    while (!signal.aborted) {
      const pending = [tails.current.get("global"), tails.current.get(scope)].filter(value => value !== undefined);
      if (pending.length) { await Promise.all(pending); continue; }
      const version = accepted.current;
      const current = () => version === accepted.current && !tails.current.has("global") && !tails.current.has(scope);
      try {
        const value = await api.channelSettings(broadcasterId);
        if (!signal.aborted && current()) return value;
      } catch (error) {
        if (!signal.aborted && current()) throw error;
      }
      // A read overtaken by a mutation must obtain a fresh Rust effective
      // preview. Never reconstruct inheritance from frontend snapshots.
    }
  }, []);
  useEffect(() => {
    mounted.current = true;
    if (!isTauri()) return () => { mounted.current = false; };
    let cancelled = false;
    void api.playbackSettings().then(value => {
      if (!cancelled && !globalAccepted.current) updateSettings(value);
    }).catch(error => {
      if (!cancelled && !globalAccepted.current) setError(error);
    });
    return () => { cancelled = true; mounted.current = false; };
  }, []);
  return { settings, error, dismiss: () => setError(null), savingSettings, commitSettings, modifyDiscovery, saveShortcuts, saveLanguage, saveChannel, readChannel };
}

const SettingsContext = createContext<ReturnType<typeof useSettingsCoordinator> | null>(null);
export function SettingsProvider({ children }: { children: ReactNode }) {
  const value = useSettingsCoordinator();
  return <SettingsContext.Provider value={value}>{children}</SettingsContext.Provider>;
}
export function useSettings() {
  const value = useContext(SettingsContext);
  if (!value) throw new Error("Settings provider is required");
  return value;
}
