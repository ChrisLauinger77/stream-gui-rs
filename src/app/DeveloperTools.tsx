import { useI18n } from "../i18n";
import { useCallback, useEffect, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { useSettings } from "../settings/useSettings";
import { Panel } from "../components/Panel";
import { Authentication } from "../features/Authentication";
import { Playback } from "../features/Playback";
import type { Account, AuthStatus, BackendDiagnostics, ErrorCode, ProbeResult, SessionSnapshot } from "../lib/generated";
import { api } from "../lib/ipc";
import { errorCode, errorText } from "../browse/errors";

type ActionKey = "auth" | "cancel" | "probe" | `restart:${string}` | `stop:${string}`;

export function DeveloperTools() {
  const { t, locale } = useI18n();
  const { settings, savingSettings, commitSettings } = useSettings();
  const [backend, setBackend] = useState<BackendDiagnostics | null>(null);
  const [auth, setAuth] = useState<AuthStatus | null>(null);
  const [account, setAccount] = useState<Account | null>(null);
  const [sessions, setSessions] = useState<SessionSnapshot[]>([]);
  const [pathEdit, setPathEdit] = useState<string | null>(null);
  const customPath = pathEdit ?? settings?.streamlinkPath ?? "";
  const [probe, setProbe] = useState<ProbeResult | null>(null);
  const [pending, setPending] = useState<ReadonlySet<ActionKey>>(new Set());
  const [error, setError] = useState<ErrorCode | null>(null);
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
      if (!cancelled) setBackend(diagnostics);
    }).catch((error: unknown) => { if (!cancelled) setError(errorCode(error)); });
    const startPolling = <T,>(read: () => Promise<T>, apply: (value: T) => void) => {
      const poll = async () => {
        try {
          const value = await read();
          if (!cancelled) apply(value);
        } catch (error) { if (!cancelled) setError(errorCode(error)); }
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
            setError(errorCode(error));
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
      catch (error) { setError(errorCode(error)); }
      finally {
        try { await refresh?.(); } catch (error) { setError(errorCode(error)); }
        actionPending.current.delete(key);
        setPending(new Set(actionPending.current));
      }
    })();
  };

  return <main className="developer-tools">
    <header><h1>Stream GUI RS</h1><p>{t("developer.intro")}</p></header>
    {!isTauri() && <p className="notice">{t("developer.browserPreview")}{" "}<code>{t("developerTools.npmRunTauriDev")}</code>.</p>}
    {error && <p role="alert" className="error">{errorText(error, locale)}</p>}
    <Panel title={t("developerTools.backend")}>
      <p>{backend ? t("developer.ready", { version: backend.version, commit: backend.commit, platform: backend.platform }) : t("app.notConnected")}</p>
      {backend && <p className="muted path">{t("developerTools.settings")}{" "}{backend.settingsPath}</p>}
    </Panel>
    <Authentication status={auth} account={account} busy={pending.has("auth")}
      cancelling={pending.has("cancel")} cancel={() => run("cancel", api.cancel, refreshAuth)}
      run={(action) => run("auth", action, refreshAuth)} />
    <Panel title="Streamlink">
      <form onSubmit={(event) => {
        event.preventDefault();
        run("probe", async () => {
          setProbe(null);
          await commitSettings(async () => {
            const result = await api.probe({ customPath: customPath || null });
            setProbe(result); return api.playbackSettings();
          });
          setBackend(await api.diagnostics());
        });
      }}>
        <label>{t("developerTools.customExecutablePath")}<input value={customPath} placeholder={t("developer.pathHint")} onChange={(event) => { setPathEdit(event.target.value); setProbe(null); }} /></label>
        <button type="submit" disabled={pending.has("probe") || savingSettings || !settings || !backend}>{t("developerTools.probeAndSave")}</button>
      </form>
      <p>{t("developerTools.detectedExecutable")}{" "}<span className="path">{probe?.executable ?? t("developer.notProbed")}</span></p>
      <p>{t("developerTools.version")}{" "}{probe?.version ?? "—"}</p>
      <p className="muted">{t("developer.probeHelp")}</p>
    </Panel>
    <Playback sessions={sessions} isStopping={id => pending.has(`stop:${id}`)} isRestarting={id => pending.has(`restart:${id}`)}
      restart={(session, quality) => run(`restart:${session.id}`, () => api.restart({ sessionId: session.id, generation: session.generation, quality }), refreshSessions)}
      stop={id => run(`stop:${id}`, () => api.stop(id), refreshSessions)} />
  </main>;
}
