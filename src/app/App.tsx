import { useCallback, useEffect, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { Panel } from "../components/Panel";
import { Authentication } from "../features/Authentication";
import { Playback } from "../features/Playback";
import type { AuthStatus, BackendDiagnostics, ProbeResult, SessionSnapshot } from "../lib/generated";
import { api, errorMessage } from "../lib/ipc";

export function App() {
  const [backend, setBackend] = useState<BackendDiagnostics | null>(null);
  const [auth, setAuth] = useState<AuthStatus | null>(null);
  const [sessions, setSessions] = useState<SessionSnapshot[]>([]);
  const [customPath, setCustomPath] = useState("");
  const [probe, setProbe] = useState<ProbeResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const actionPending = useRef(false);
  const refresh = useCallback(async () => {
    const [nextAuth, nextSessions] = await Promise.all([api.authStatus(), api.sessions()]);
    setAuth(nextAuth); setSessions(nextSessions);
  }, []);

  useEffect(() => {
    if (!isTauri()) return;
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    void api.diagnostics().then((diagnostics) => {
      if (!cancelled) { setBackend(diagnostics); setCustomPath(diagnostics.settings.streamlinkPath ?? ""); }
    }).catch((error: unknown) => { if (!cancelled) setError(errorMessage(error)); });
    const poll = async () => {
      try {
        // Sequential polling avoids a growing queue behind an OAuth request.
        const [nextAuth, nextSessions] = await Promise.all([api.authStatus(), api.sessions()]);
        if (!cancelled) { setAuth(nextAuth); setSessions(nextSessions); }
      } catch (error) { if (!cancelled) setError(errorMessage(error)); }
      if (!cancelled) timer = setTimeout(() => { void poll(); }, 1000);
    };
    void poll();
    return () => { cancelled = true; clearTimeout(timer); };
  }, []);

  const run = (action: () => Promise<unknown>) => {
    if (actionPending.current) return;
    actionPending.current = true;
    setBusy(true); setError(null);
    void (async () => {
      try { await action(); }
      catch (error) { setError(errorMessage(error)); }
      finally {
        try { await refresh(); } catch (error) { setError(errorMessage(error)); }
        setBusy(false); actionPending.current = false;
      }
    })();
  };

  return <main>
    <header><h1>Twitch GUI RS</h1><p>Phase 0 · developer prototype</p></header>
    {!isTauri() && <p className="notice">Browser preview only. Backend controls require the desktop app: <code>npm run tauri dev</code>.</p>}
    {error && <p role="alert" className="error">{error}</p>}
    <Panel title="Backend">
      <p>{backend ? `Ready · ${backend.version} · ${backend.platform}` : "Not connected"}</p>
      {backend && <p className="muted path">Settings v{backend.settings.version}: {backend.settingsPath}</p>}
    </Panel>
    <Authentication status={auth} busy={busy} run={run} />
    <Panel title="Streamlink">
      <form onSubmit={(event) => {
        event.preventDefault();
        run(async () => {
          setProbe(null);
          const result = await api.probe({ customPath: customPath || null });
          setProbe(result); setBackend(await api.diagnostics());
        });
      }}>
        <label>Custom executable path<input value={customPath} placeholder="Leave empty to discover on PATH" onChange={(event) => { setCustomPath(event.target.value); setProbe(null); }} /></label>
        <button type="submit" disabled={busy || !backend}>Probe and save</button>
      </form>
      <p>Detected executable: <span className="path">{probe?.executable ?? "Not probed"}</span></p>
      <p>Version: {probe?.version ?? "—"}</p>
      <p className="muted">A successful probe saves this choice. Launch checks the saved executable again.</p>
    </Panel>
    <Playback sessions={sessions} busy={busy} ready={!!backend && !!probe} run={run} />
  </main>;
}
