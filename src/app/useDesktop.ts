import { useEffect, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { api } from "../lib/ipc";
import type { DesktopStatus } from "../lib/generated";
import { friendlyError } from "../browse/errors";

// Snapshot presentation only: this does not request Twitch data or own monitoring.
export function useDesktop() {
  const [status, setStatus] = useState<DesktopStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const pending = useRef(false);
  const mounted = useRef(false);
  const revision = useRef(0);
  useEffect(() => {
    mounted.current = true;
    if (!isTauri()) return () => { mounted.current = false; };
    let live = true;
    let timer: ReturnType<typeof setTimeout>;
    const refresh = async () => {
      const request = revision.current;
      try {
        const value = await api.desktopStatus();
        if (live && !pending.current && request === revision.current && value) setStatus(value);
      }
      catch { /* An unavailable snapshot must not start a replacement monitor. */ }
      if (live) timer = setTimeout(() => { void refresh(); }, 2000);
    };
    void refresh();
    return () => { live = false; mounted.current = false; clearTimeout(timer); };
  }, []);
  const run = async (action: () => Promise<unknown>) => {
    if (pending.current) return;
    pending.current = true; revision.current += 1; setBusy(true); setError(null);
    try {
      await action();
      const value = await api.desktopStatus();
      if (mounted.current && value) setStatus(value);
    } catch (error) { if (mounted.current) setError(friendlyError(error)); }
    finally { pending.current = false; if (mounted.current) setBusy(false); }
  };
  return { status, busy, error, run };
}
