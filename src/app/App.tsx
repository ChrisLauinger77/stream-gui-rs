import { useCallback, useEffect, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { Panel } from "../components/Panel";
import { Authentication } from "../features/Authentication";
import { Playback } from "../features/Playback";
import type { Account, AuthStatus, BackendDiagnostics, ProbeResult, SessionSnapshot } from "../lib/generated";
import { api, errorMessage } from "../lib/ipc";

type ActionKey = "auth" | "cancel" | "probe" | "launch" | `stop:${string}`;

export function App() {
  const [backend, setBackend] = useState<BackendDiagnostics | null>(null);
  const [auth, setAuth] = useState<AuthStatus | null>(null);
  const [account, setAccount] = useState<Account | null>(null);
  const [sessions, setSessions] = useState<SessionSnapshot[]>([]);
  const [customPath, setCustomPath] = useState("");
  const [probe, setProbe] = useState<ProbeResult | null>(null);
  const [pending, setPending] = useState<ReadonlySet<ActionKey>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const actionPending = useRef(new Set<ActionKey>());
  const refreshAuth = useCallback(async () => {
    setAuth(await api.authStatus());
  }, []);
  const refreshSessions = useCallback(async () => {
    setSessions(await api.sessions());
  }, []);

  useEffect(() => {
    if (!isTauri()) return;
    let cancelled = false;
    const timers = new Set<ReturnType<typeof setTimeout>>();
    void api.diagnostics().then((diagnostics) => {
      if (!cancelled) { setBackend(diagnostics); setCustomPath(diagnostics.settings.streamlinkPath ?? ""); }
    }).catch((error: unknown) => { if (!cancelled) setError(errorMessage(error)); });
    const startPolling = <T,>(read: () => Promise<T>, apply: (value: T) => void) => {
      const poll = async () => {
        try {
          const value = await read();
          if (!cancelled) apply(value);
        } catch (error) { if (!cancelled) setError(errorMessage(error)); }
        if (!cancelled) {
          const timer = setTimeout(() => { timers.delete(timer); void poll(); }, 1000);
          timers.add(timer);
        }
      };
      void poll();
    };
    // Each source has at most one poll in flight. Slow OAuth cannot hold back
    // session snapshots, and neither source accumulates a polling queue.
    startPolling(api.authStatus, setAuth);
    startPolling(api.sessions, setSessions);
    return () => { cancelled = true; timers.forEach(clearTimeout); };
  }, []);

  const accountId = auth?.phase === "authenticated" ? auth.user?.id : undefined;
  useEffect(() => {
    let cancelled = false;
    let retry: ReturnType<typeof setTimeout> | undefined;
    setAccount(null);
    if (accountId) {
      const load = async () => {
        try {
          const account = await api.account();
          if (!cancelled && account.id === accountId) setAccount(account);
        } catch (error) {
          if (!cancelled) {
            setError(errorMessage(error));
            retry = setTimeout(() => { void load(); }, 60_000);
          }
        }
      };
      void load();
    }
    return () => { cancelled = true; clearTimeout(retry); };
  }, [accountId]);

  const run = (key: ActionKey, action: () => Promise<unknown>, refresh?: () => Promise<void>) => {
    if (actionPending.current.has(key)) return;
    actionPending.current.add(key);
    setPending(new Set(actionPending.current)); setError(null);
    void (async () => {
      try { await action(); }
      catch (error) { setError(errorMessage(error)); }
      finally {
        try { await refresh?.(); } catch (error) { setError(errorMessage(error)); }
        actionPending.current.delete(key);
        setPending(new Set(actionPending.current));
      }
    })();
  };

  return <main>
    <header><h1>Stream GUI RS</h1><p>Phase 1 · authentication developer screen</p></header>
    {!isTauri() && <p className="notice">Browser preview only. Backend controls require the desktop app: <code>npm run tauri dev</code>.</p>}
    {error && <p role="alert" className="error">{error}</p>}
    <Panel title="Backend">
      <p>{backend ? `Ready · ${backend.version} · ${backend.platform}` : "Not connected"}</p>
      {backend && <p className="muted path">Settings v{backend.settings.version}: {backend.settingsPath}</p>}
    </Panel>
    <Authentication status={auth} account={account} busy={pending.has("auth")}
      cancelling={pending.has("cancel")} cancel={() => run("cancel", api.cancel, refreshAuth)}
      run={(action) => run("auth", action, refreshAuth)} />
    <Panel title="Streamlink">
      <form onSubmit={(event) => {
        event.preventDefault();
        run("probe", async () => {
          setProbe(null);
          const result = await api.probe({ customPath: customPath || null });
          setProbe(result); setBackend(await api.diagnostics());
        });
      }}>
        <label>Custom executable path<input value={customPath} placeholder="Leave empty to discover on PATH" onChange={(event) => { setCustomPath(event.target.value); setProbe(null); }} /></label>
        <button type="submit" disabled={pending.has("probe") || pending.has("launch") || !backend}>Probe and save</button>
      </form>
      <p>Detected executable: <span className="path">{probe?.executable ?? "Not probed"}</span></p>
      <p>Version: {probe?.version ?? "—"}</p>
      <p className="muted">A successful probe saves this choice. Launch checks the saved executable again.</p>
    </Panel>
    <Playback sessions={sessions} launchBusy={pending.has("launch") || pending.has("probe")}
      ready={!!backend && !!probe} isStopping={(id) => pending.has(`stop:${id}`)}
      launch={(request) => run("launch", () => api.launch(request), refreshSessions)}
      stop={(id) => run(`stop:${id}`, () => api.stop(id), refreshSessions)} />
  </main>;
}
