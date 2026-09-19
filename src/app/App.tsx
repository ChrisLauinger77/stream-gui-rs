import { useCallback, useEffect, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { DeveloperTools } from "./DeveloperTools";
import { useAuthentication } from "./useAuthentication";
import { BrowserWorkspace, type BrowserActions } from "../browse/Workspace";
import { friendlyError } from "../browse/errors";
import type { Account, AuthStatus } from "../lib/generated";

import { usePlayback } from "../playback/usePlayback";
import { Playback } from "../features/Playback";
import { PlaybackSettings } from "../features/PlaybackSettings";
import { api } from "../lib/ipc";
import { useDesktop } from "./useDesktop";
import { useAppearance } from "./useAppearance";
import { useShortcuts, shortcutLabels } from "./shortcuts";

export function App() {
  const [developer, setDeveloper] = useState(false);
  if (developer) return <><div className="developer-banner"><button onClick={() => setDeveloper(false)}>← Back to browsing</button><span>Developer tools · backend diagnostics</span></div><DeveloperTools /></>;
  return <Application developer={() => setDeveloper(true)} />;
}
function Application({ developer }: { developer: () => void }) {
  const auth = useAuthentication();
  const desktop = useDesktop();
  const handledAction = useRef<string | null>(null);
  const acknowledgedAction = useRef<string | null>(null);
  const acknowledgingAction = useRef(false);
  const [settings, setSettings] = useState(false);
  const [watching, setWatching] = useState(false);
  const playback = usePlayback();
  const { run } = playback;
  useAppearance(playback.settings?.theme ?? "system");
  const workspace = useRef<BrowserActions>(null);
  const settingsHeading = useRef<HTMLHeadingElement>(null);
  const watchingPanel = useRef<HTMLDivElement>(null);
  const [settingsRevision, setSettingsRevision] = useState(0);
  const shortcuts = shortcutLabels();
  useEffect(() => {
    const action = desktop.status?.action;
    if (!action) return;
    if (handledAction.current !== action.id) {
      if (action.kind === "channel") {
        if (action.authSessionId !== auth.sessionId || !workspace.current) return;
        setSettings(false); setWatching(false); workspace.current.channel(action.broadcasterId, action.displayName);
      } else { setSettings(false); setWatching(true); }
      handledAction.current = action.id;
    }
    if (acknowledgedAction.current === action.id || acknowledgingAction.current) return;
    acknowledgingAction.current = true;
    void api.acknowledgeDesktopAction(action.id)
      .then(() => { acknowledgedAction.current = action.id; })
      .catch(() => { /* Retry acknowledgement on the next snapshot without navigating again. */ })
      .finally(() => { acknowledgingAction.current = false; });
  }, [desktop.status?.action, auth.sessionId]);
  useEffect(() => { if (settings) settingsHeading.current?.focus(); }, [settings]);
  useEffect(() => { if (watching) watchingPanel.current?.querySelector<HTMLElement>("h2")?.focus(); }, [watching]);
  const navigate = (section: "search" | "following" | "live" | "categories") => {
    setSettings(false); setWatching(false); workspace.current?.navigate(section);
  };
  useShortcuts({
    search: () => navigate("search"), following: () => navigate("following"), live: () => navigate("live"), categories: () => navigate("categories"),
    watching: () => { setWatching(true); setSettings(false); }, settings: () => { setSettings(true); setWatching(false); },
    back: () => { if (settings) setSettings(false); else if (watching) setWatching(false); else workspace.current?.back(); },
    refresh: () => { if (!settings) workspace.current?.refresh(); },
  });
  const watch = useCallback((broadcasterId: string) => {
    if (!auth.sessionId) return;
    setWatching(true); setSettings(false);
    void run(`launch:${broadcasterId}`, () => api.launch({ authSessionId: auth.sessionId!, broadcasterId, quality: null }), "Streamlink started. Check Watching for status.");
  }, [auth.sessionId, run]);
  const activeCount = playback.sessions.filter(session => session.restarting || ["starting", "running", "stopping"].includes(session.phase)).length;
  const controls = <div className="account-controls">
    <span className="connection-dot" aria-hidden="true" />
    <span>{auth.account?.displayName ?? auth.status?.user?.login ?? "Not connected"}</span>
    {auth.sessionId && <button className="quiet" disabled={auth.busy === "logout"} onClick={() => { void auth.run("logout"); }}>Sign out</button>}
    <button className="quiet" aria-label="Watching" title={`Watching (${shortcuts.watching})`} aria-expanded={watching} onClick={() => { setWatching(!watching); if (!watching) setSettings(false); }}>Watching{activeCount > 0 && ` (${activeCount})`}</button>
    <button className="quiet" title={`Settings (${shortcuts.settings})`} aria-expanded={settings} onClick={() => { setSettings(!settings); if (!settings) setWatching(false); }}>Settings</button>
  </div>;
  return <div className="application">
    <header className="app-bar"><div className="brand"><span aria-hidden="true">▶</span> Stream GUI RS</div>{controls}</header>
    {settings && <section className="settings-panel" aria-label="Settings">
      <div className="settings-header"><h2 tabIndex={-1} ref={settingsHeading}>Settings</h2><button className="quiet" onClick={developer}>Developer tools</button><button className="quiet" disabled={desktop.busy} onClick={() => { void desktop.run(api.quit); }}>{activeCount ? `Quit (stops ${activeCount} streams)` : "Quit"}</button></div>
      <PlaybackSettings desktop={desktop} saved={playback.settings} onSaved={value => { playback.setSettings(value); setSettingsRevision(revision => revision + 1); }} />
      {auth.status?.phase === "not_configured" && <p>This build does not include a Twitch application ID. If you built it from source, follow the authentication setup in the project documentation.</p>}
    </section>}
    {(playback.error || playback.message) && <div className={playback.error ? "error playback-feedback" : "notice playback-feedback"} role={playback.error ? "alert" : "status"}>{playback.error ?? playback.message}<button className="quiet" onClick={playback.dismiss}>Dismiss</button></div>}
    {watching && <div className="watching-panel" ref={watchingPanel}><Playback sessions={playback.sessions}
      isStopping={id => playback.pending.has(`stop:${id}`)} isRestarting={id => playback.pending.has(`restart:${id}`)}
      stop={id => { void playback.run(`stop:${id}`, () => api.stop(id), "Playback stopped."); }}
      restart={(session, quality) => { void playback.run(`restart:${session.id}`, () => api.restart({ sessionId: session.id, generation: session.generation, quality }), "Streamlink restarted."); }} /></div>}
    {auth.error && <p className="error" role="alert">{auth.error}</p>}
    {auth.sessionId ? <BrowserWorkspace actionsRef={workspace} settingsRevision={settingsRevision} key={auth.sessionId} sessionId={auth.sessionId} onAuthLost={auth.lost} watch={watch} pending={playback.pending} /> :
      <SignIn status={auth.status} account={auth.account} busy={auth.busy} run={auth.run} />}
    <footer className="app-footer"><span>Twitch browsing · Streamlink desktop</span><span>{auth.sessionId ? "Connected to Twitch" : "Connect your Twitch account"}</span></footer>
  </div>;
}
function SignIn({ status, busy, run }: { status: AuthStatus | null; account: Account | null; busy: string | null; run: ReturnType<typeof useAuthentication>["run"] }) {
  const pending = status?.phase === "authorizing" || busy === "login";
  return <main className="sign-in">
    <div className="eyebrow">YOUR STREAMS, IN ONE PLACE</div>
    <h1>Find what’s live.</h1>
    <p>Browse the channels you follow, discover categories,<br />and find your next stream.</p>
    {!isTauri() ? <p className="notice">This is a browser preview. Open the desktop application to connect to Twitch.</p> : !status || status.phase === "restoring" ? <p role="status">Restoring your Twitch session…</p> : <>
      {pending ? <div className="sign-in-flow">
        {status.authorization ? <><p>Enter this code on Twitch:</p><strong className="user-code">{status.authorization.userCode}</strong><p className="muted">Code expires in {Math.ceil(status.authorization.expiresIn / 60)} minutes.</p><button disabled={busy === "openVerification"} onClick={() => { void run("openVerification"); }}>Open Twitch sign-in</button><p className="path">{status.authorization.verificationUri}</p><p role="status">Waiting for authorization…</p></> : <p role="status">Starting secure sign-in…</p>}
        <button className="quiet" disabled={busy === "cancel"} onClick={() => { void run("cancel"); }}>Cancel sign-in</button>
      </div> : <button className="primary" disabled={!!busy || status.phase === "not_configured"} onClick={() => { void run("login"); }}>Connect to Twitch</button>}
      {status.error && <p role="alert" className="error">{friendlyError(status.error)}</p>}
      {status.phase === "not_configured" && <p className="notice">Twitch sign-in needs to be configured for this build. Open Settings for instructions.</p>}
    </>}
    <p className="muted">Sign-in opens in your browser. Credentials stay in secure system storage.</p>
  </main>;
}
