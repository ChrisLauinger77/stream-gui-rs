import { useCallback, useEffect, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { DeveloperTools } from "./DeveloperTools";
import { useAuthentication } from "./useAuthentication";
import { BrowserWorkspace, type BrowserActions } from "../browse/Workspace";
import { friendlyError } from "../browse/errors";
import type { Account, AuthStatus } from "../lib/generated";

import { SettingsProvider } from "../settings/useSettings";
import { usePlayback } from "../playback/usePlayback";
import { About } from "../features/About";
import { SupportReport } from "../features/SupportReport";
import { Playback } from "../features/Playback";
import { PlaybackSettings } from "../features/PlaybackSettings";
import { api } from "../lib/ipc";
import { useDesktop } from "./useDesktop";
import { isNotificationTest, NotificationAcceptance } from "./NotificationAcceptance";
import { usePanelFocus } from "./usePanelFocus";
import { useAppearance } from "./useAppearance";
import { useShortcuts, shortcutLabels } from "./shortcuts";

export function App() {
  return <SettingsProvider><AppContent /></SettingsProvider>;
}
function AppContent() {
  const [developer, setDeveloper] = useState(false);
  const desktop = useDesktop();
  useEffect(() => { if (desktop.status?.action || desktop.status?.navigation) setDeveloper(false); }, [desktop.status?.action?.id, desktop.status?.navigation?.id]);
  if (developer) return <><div className="developer-banner"><button onClick={() => setDeveloper(false)}>← Back to browsing</button><span>Developer tools · backend diagnostics</span></div><NotificationAcceptance desktop={desktop} /><DeveloperTools /></>;
  return <Application desktop={desktop} developer={() => setDeveloper(true)} />;
}
function Application({ desktop, developer }: { desktop: ReturnType<typeof useDesktop>; developer: () => void }) {
  const auth = useAuthentication();
  const actionState = desktop.actions.current;
  const [notificationTest, setNotificationTest] = useState(() => isNotificationTest(desktop.status) && actionState.handled !== desktop.status?.action?.id);
  const testHeading = useRef<HTMLHeadingElement>(null);
  const [about, setAbout] = useState<string | null>(null);
  const [support, setSupport] = useState(false);
  const [settings, setSettings] = useState(false);
  const [watching, setWatching] = useState(false);
  const [navigationFocus, setNavigationFocus] = useState<{ id: string; target: "channel" | "watching" | "test" } | null>(null);
  const playback = usePlayback();
  const { run } = playback;
  useAppearance(playback.settings?.theme ?? "system", playback.settings?.textScale ?? "100");
  const workspace = useRef<BrowserActions>(null);
  const { capture, restore } = usePanelFocus();
  const settingsButton = useRef<HTMLButtonElement>(null);
  const watchingButton = useRef<HTMLButtonElement>(null);
  const settingsPanel = useRef<HTMLElement>(null);
  const settingsHeading = useRef<HTMLHeadingElement>(null);
  const watchingPanel = useRef<HTMLDivElement>(null);
  const shortcuts = shortcutLabels(undefined, playback.settings?.shortcuts);
  const closeSettings = () => { restore("settings", settingsPanel.current, settingsButton.current); setSettings(false); };
  const closeWatching = () => { restore("watching", watchingPanel.current, watchingButton.current); setWatching(false); };
  useEffect(() => {
    const action = desktop.status?.action;
    if (!action) return;
    if (actionState.handled !== action.id) {
      if (action.kind === "channel") {
        if (isNotificationTest(desktop.status)) {
          setNotificationTest(true); setSettings(false); setWatching(false);
          setNavigationFocus({ id: action.id, target: "test" });
        } else {
          if (action.authSessionId !== auth.sessionId) return;
          if (!workspace.current) return;
          setNotificationTest(false);
          setSettings(false); setWatching(false); workspace.current.channel(action.broadcasterId, action.displayName);
          setNavigationFocus({ id: action.id, target: "channel" });
        }
      } else if (action.kind === "about") { setSupport(false); setAbout(action.id); }
      else {
        setNotificationTest(false); setSettings(false); setWatching(true);
        setNavigationFocus({ id: action.id, target: "watching" });
      }
      if (action.kind !== "about") { setAbout(null); setSupport(false); }
      actionState.handled = action.id;
    }
    if (actionState.acknowledged === action.id || actionState.acknowledging) return;
    actionState.acknowledging = true;
    void api.acknowledgeDesktopAction(action.id)
      .then(() => { actionState.acknowledged = action.id; })
      .catch(() => { /* Retry acknowledgement on the next snapshot without navigating again. */ })
      .finally(() => { actionState.acknowledging = false; });
  }, [desktop.status, auth.sessionId, notificationTest, actionState]);
  useEffect(() => {
    const pending = desktop.status?.navigation;
    const state = desktop.navigationActions.current;
    if (!pending || !playback.settings || !auth.status || auth.status.phase === "restoring") return;
    if (state.handled !== pending.id) {
      if (pending.intent.kind !== "show" && (!auth.sessionId || !workspace.current)) return;
      setSettings(false); setWatching(false); setNotificationTest(false); setAbout(null); setSupport(false);
      if (pending.intent.kind !== "show") workspace.current!.intent(pending.intent);
      setNavigationFocus({ id: pending.id, target: "channel" });
      state.handled = pending.id;
    }
    if (state.acknowledged === pending.id || state.acknowledging) return;
    state.acknowledging = true;
    void api.acknowledgeNavigationIntent(pending.id)
      .then(() => { state.acknowledged = pending.id; })
      .catch(() => { /* Retry only the acknowledgement on the next native snapshot. */ })
      .finally(() => { state.acknowledging = false; });
  }, [desktop.status?.navigation, auth.sessionId, auth.status, playback.settings, desktop.navigationActions]);
  useEffect(() => {
    // Passive unmount cleanup closes the modal (including native opener focus)
    // before this effect focuses the accepted destination, even on the same route.
    if (navigationFocus?.target === "channel") workspace.current?.focus();
    else if (navigationFocus?.target === "watching") watchingPanel.current?.querySelector<HTMLElement>("h2")?.focus();
    else if (navigationFocus?.target === "test") testHeading.current?.focus();
  }, [navigationFocus]);
  useEffect(() => { if (notificationTest) testHeading.current?.focus(); }, [notificationTest]);
  useEffect(() => { if (settings) settingsHeading.current?.focus(); }, [settings]);
  useEffect(() => { if (watching) watchingPanel.current?.querySelector<HTMLElement>("h2")?.focus(); }, [watching]);
  const navigate = (section: "search" | "following" | "live" | "categories") => {
    setSettings(false); setWatching(false); workspace.current?.navigate(section);
  };
  useShortcuts({
    home: () => navigate("following"), forward: () => { setSettings(false); setWatching(false); workspace.current?.forward(); },
    search: () => navigate("search"), following: () => navigate("following"), live: () => navigate("live"), categories: () => navigate("categories"),
    watching: () => { if (!watching) capture("watching"); setWatching(true); setSettings(false); }, settings: () => { if (!settings) capture("settings"); setSettings(true); setWatching(false); },
    back: () => { if (settings) closeSettings(); else if (watching) closeWatching(); else if (notificationTest) setNotificationTest(false); else workspace.current?.back(); },
    refresh: () => { if (!settings) workspace.current?.refresh(); },
  }, playback.settings?.shortcuts);
  const watch = useCallback((broadcasterId: string) => {
    if (!auth.sessionId) return;
    capture("watching"); setWatching(true); setSettings(false);
    void run(`launch:${broadcasterId}`, () => api.launch({ authSessionId: auth.sessionId!, broadcasterId, quality: null }), "Streamlink process started. Check Watching for status.");
  }, [auth.sessionId, run, capture]);
  const activeCount = playback.sessions.filter(session => session.restarting || ["starting", "running", "stopping"].includes(session.phase)).length;
  const controls = <div className="account-controls">
    <span className="connection-dot" aria-hidden="true" />
    <span>{auth.account?.displayName ?? auth.status?.user?.login ?? "Not connected"}</span>
    {auth.sessionId && <button className="quiet" disabled={auth.busy === "logout"} onClick={() => { void auth.run("logout"); }}>Sign out</button>}
    <button ref={watchingButton} className="quiet" aria-label="Watching" title={`Watching (${shortcuts.watching})`} aria-expanded={watching} onClick={event => { if (watching) closeWatching(); else { capture("watching", event.currentTarget); setWatching(true); setSettings(false); } }}>Watching{activeCount > 0 && ` (${activeCount})`}</button>
    <button ref={settingsButton} className="quiet" title={`Settings (${shortcuts.settings})`} aria-expanded={settings} onClick={event => { if (settings) closeSettings(); else { capture("settings", event.currentTarget); setSettings(true); setWatching(false); } }}>Settings</button>
  </div>;
  return <div className="application">
    <header className="app-bar"><div className="brand"><span aria-hidden="true">▶</span> Stream GUI RS</div>{controls}</header>
    {settings && <section ref={settingsPanel} className="settings-panel" aria-label="Settings">
      <div className="settings-header"><h2 tabIndex={-1} ref={settingsHeading}>Settings</h2><button className="quiet" onClick={closeSettings}>Close Settings</button><button className="quiet" onClick={() => setSupport(true)}>Prepare support report</button><button className="quiet" onClick={developer}>Developer tools</button><button className="quiet" disabled={desktop.busy} onClick={() => { void desktop.run(api.quit); }}>{activeCount ? `Quit (stops ${activeCount} streams)` : "Quit"}</button></div>
      <PlaybackSettings desktop={desktop} saved={playback.settings} saving={playback.savingSettings} commit={playback.commitSettings} />
      {auth.status?.phase === "not_configured" && <p>This build does not include a Twitch application ID. If you built it from source, follow the authentication setup in the project documentation.</p>}
    </section>}
    <div className={playback.error ? "error playback-feedback" : playback.message ? "notice playback-feedback" : "playback-feedback empty-feedback"} role={playback.error ? "alert" : "status"} aria-atomic="true">{playback.error ?? playback.message}{(playback.error || playback.message) && <button className="quiet" onClick={playback.dismiss}>Dismiss</button>}</div>
    {watching && <div className="watching-panel" ref={watchingPanel}><button className="quiet close-panel" onClick={closeWatching}>Close Watching</button><Playback sessions={playback.sessions}
      isStopping={id => playback.pending.has(`stop:${id}`)} isRestarting={id => playback.pending.has(`restart:${id}`)}
      stop={id => { void playback.run(`stop:${id}`, () => api.stop(id), "Playback stopped."); }}
      restart={(session, quality) => { void playback.run(`restart:${session.id}`, () => api.restart({ sessionId: session.id, generation: session.generation, quality }), "Streamlink process restarted."); }} /></div>}
    {desktop.status?.navigation && !auth.sessionId && desktop.status.navigation.intent.kind !== "show" && <p className="notice" role="status">Connect to Twitch to open the requested destination.</p>}
    {auth.error && <p className="error" role="alert">{auth.error}</p>}
    {notificationTest ? <main className="settings-panel">
      <h1 tabIndex={-1} ref={testHeading}>TEST notification · Synthetic channel</h1>
      <p>The native notification activated this local target. No Twitch channel data or playback is involved.</p>
      <button onClick={() => setNotificationTest(false)}>Back to browsing</button>
      <button onClick={developer}>Developer tools</button>
    </main> : auth.sessionId ? <BrowserWorkspace preferences={playback.settings} saveLanguage={playback.saveLanguage} actionsRef={workspace} key={auth.sessionId} sessionId={auth.sessionId} onAuthLost={auth.lost} watch={watch} pending={playback.pending} /> :
      <SignIn status={auth.status} account={auth.account} busy={auth.busy} run={auth.run} />}
    {about && <About activation={about} close={() => setAbout(null)} />}
    {support && <SupportReport close={() => setSupport(false)} />}
    <footer className="app-footer"><span>Twitch browsing · Streamlink desktop</span><button className="quiet" disabled={desktop.busy} onClick={() => { void desktop.run(api.showAbout); }}>About Stream GUI RS</button><span>{auth.sessionId ? "Connected to Twitch" : "Connect your Twitch account"}</span></footer>
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
