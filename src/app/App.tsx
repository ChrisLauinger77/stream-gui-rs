import { useEffect, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { DeveloperTools } from "./DeveloperTools";
import { useAuthentication } from "./useAuthentication";
import { BrowserWorkspace } from "../browse/Workspace";
import { friendlyError } from "../browse/errors";
import type { Account, AuthStatus } from "../lib/generated";

type Theme = "system" | "dark" | "light";
function initialTheme(): Theme {
  try { const value = localStorage.getItem("stream-gui-theme"); if (value === "light" || value === "dark") return value; } catch { /* optional visual preference */ }
  return "system";
}
export function App() {
  const [developer, setDeveloper] = useState(false);
  const [theme, setTheme] = useState<Theme>(initialTheme);
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    try { localStorage.setItem("stream-gui-theme", theme); } catch { /* no persistence required */ }
  }, [theme]);
  if (developer) return <><div className="developer-banner"><button onClick={() => setDeveloper(false)}>← Back to browsing</button><span>Developer tools · isolated playback prototype</span></div><DeveloperTools /></>;
  return <Application theme={theme} setTheme={setTheme} developer={() => setDeveloper(true)} />;
}
function Application({ theme, setTheme, developer }: { theme: Theme; setTheme: (value: Theme) => void; developer: () => void }) {
  const auth = useAuthentication();
  const [settings, setSettings] = useState(false);
  const controls = <div className="account-controls">
    <span className="connection-dot" aria-hidden="true" />
    <span>{auth.account?.displayName ?? auth.status?.user?.login ?? "Not connected"}</span>
    {auth.sessionId && <button className="quiet" disabled={auth.busy === "logout"} onClick={() => { void auth.run("logout"); }}>Sign out</button>}
    <button className="quiet" aria-expanded={settings} onClick={() => setSettings(!settings)}>Settings</button>
  </div>;
  return <div className="application">
    <header className="app-bar"><div className="brand"><span aria-hidden="true">▶</span> Stream GUI RS</div>{controls}</header>
    {settings && <section className="settings-panel" aria-label="Settings">
      <label>Appearance<select value={theme} onChange={e => setTheme(e.target.value as Theme)}><option value="system">System</option><option value="dark">Dark</option><option value="light">Light</option></select></label>
      <p>Streamlink is installed separately. Its existing path and playback controls are available in developer tools.</p>
      <button onClick={developer}>Developer tools</button>
      {auth.status?.phase === "not_configured" && <p>Set the public <code>TWITCH_CLIENT_ID</code> in the backend environment and restart. No client secret is needed.</p>}
    </section>}
    {auth.error && <p className="error" role="alert">{auth.error}</p>}
    {auth.sessionId ? <BrowserWorkspace key={auth.sessionId} sessionId={auth.sessionId} onAuthLost={auth.lost} /> :
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
