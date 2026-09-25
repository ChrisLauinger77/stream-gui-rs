import { useCallback, useEffect, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { api } from "../lib/ipc";
import type { Account, AuthStatus, ErrorCode } from "../lib/generated";
import { errorCode } from "../browse/errors";

export function useAuthentication() {
  const [status, setStatus] = useState<AuthStatus | null>(null);
  const [account, setAccount] = useState<Account | null>(null);
  const [error, setError] = useState<ErrorCode | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const revision = useRef(0);
  const loggingOut = useRef(false);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    if (!isTauri()) return;
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      const version = revision.current;
      try {
        const next = await api.authStatus();
        if (!stopped && version === revision.current && !loggingOut.current) setStatus(next);
      } catch (e) { if (!stopped) setError(errorCode(e)); }
      if (!stopped) timer = setTimeout(() => { void poll(); }, 1000);
    };
    void poll();
    return () => { stopped = true; alive.current = false; revision.current++; clearTimeout(timer); };
  }, []);
  const sessionId = status?.phase === "authenticated" ? status.sessionId : null;
  const userId = status?.user?.id;
  useEffect(() => {
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    setAccount(null);
    if (!sessionId) return;
    const load = async () => {
      try {
        const result = await api.account();
        if (!stopped && result.id === userId) setAccount(result);
      } catch { if (!stopped) timer = setTimeout(() => { void load(); }, 60_000); }
    };
    void load();
    return () => { stopped = true; clearTimeout(timer); };
  }, [sessionId, userId]);
  const run = async (name: "login" | "cancel" | "logout" | "openVerification") => {
    const version = ++revision.current;
    loggingOut.current = name === "logout";
    setBusy(name); setError(null);
    if (name === "logout") { setAccount(null); setStatus(null); }
    try {
      const next = await api[name]();
      if (alive.current && version === revision.current && next) setStatus(next);
    } catch (e) { if (alive.current && version === revision.current) setError(errorCode(e)); }
    finally { if (alive.current && version === revision.current) { loggingOut.current = false; setBusy(null); } }
  };
  const lost = useCallback(() => { revision.current++; setStatus(null); setAccount(null); }, []);
  return { status, sessionId, account, error, busy, run, lost };
}
