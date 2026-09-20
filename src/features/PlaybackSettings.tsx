import { useRef, useState } from "react";
import { api } from "../lib/ipc";
import type { PlayerDiscovery, PlayerMode, ProbeResult, Settings, TextScale, Theme } from "../lib/generated";
import { QualitySelect } from "../playback/QualitySelect";
import { playbackError } from "../playback/usePlayback";
import { shortcutLabels } from "../app/shortcuts";

import { BackgroundSettings } from "./BackgroundSettings";
import type { useDesktop } from "../app/useDesktop";

const sections = ["Playback", "Streamlink", "Player", "Appearance", "Background", "Shortcuts"] as const;
type Section = typeof sections[number];
type SettingsProps = { desktop: ReturnType<typeof useDesktop>; saved: Settings | null; saving: boolean; commit: (action: () => Promise<Settings>) => Promise<Settings> };
type DraftEdits = Omit<Partial<Settings>, "player" | "background"> & {
  player?: Partial<Settings["player"]>;
  background?: Partial<Settings["background"]>;
};
export function PlaybackSettings(props: SettingsProps) {
  if (!props.saved) return <p role="status">Loading settings…</p>;
  return <SettingsForm {...props} saved={props.saved} />;
}
function SettingsForm({ saved, desktop, saving, commit }: SettingsProps & { saved: Settings }) {
  const [section, setSection] = useState<Section>("Playback");
  // Only deliberate field edits overlay the accepted snapshot. Reopened forms
  // adopt completed saves without discarding edits, even an edit back to default.
  const [edits, setEdits] = useState<DraftEdits>({});
  const draft: Settings = { ...saved, ...edits,
    player: { ...saved.player, ...edits.player }, background: { ...saved.background, ...edits.background } };
  const edit = (change: DraftEdits) => setEdits(current => ({ ...current, ...change,
    ...(change.player && { player: { ...current.player, ...change.player } }),
    ...(change.background && { background: { ...current.background, ...change.background } }),
  }));
  const [probe, setProbe] = useState<ProbeResult | null>(null);
  const [players, setPlayers] = useState<PlayerDiscovery | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const inFlight = useRef(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const run = async (key: string, action: () => Promise<void>) => {
    if (inFlight.current || saving) return;
    inFlight.current = true; setBusy(key); setError(null); setMessage(null);
    try { await action(); } catch (error) { setError(playbackError(error)); }
    finally { inFlight.current = false; setBusy(null); }
  };
  const mode = draft.player.mode;
  const shortcuts = shortcutLabels();
  return <div className="settings-content">
    <nav className="settings-nav" aria-label="Settings sections">{sections.map(name => <button key={name} aria-current={section === name ? "page" : undefined} onClick={() => setSection(name)}>{name}</button>)}</nav>
    <form noValidate className="playback-settings" aria-label="Application preferences" onSubmit={event => {
      event.preventDefault();
      void run("save", async () => {
        await commit(() => api.savePlaybackSettings(draft));
        setEdits({}); setMessage("Settings saved. Background preferences apply now; playback preferences apply to new launches and restarts.");
      });
    }}>
      <fieldset disabled={!!busy}>
        <legend>{section}</legend>
        <div hidden={section !== "Playback"} className="setting-group">
          <QualitySelect label="Default quality" value={draft.defaultQuality} change={defaultQuality => edit({ defaultQuality })} />
          <p className="muted">High, Medium and Low prefer their indicated resolution and fall back to Source when needed.</p>
          <label className="checkbox-label"><input type="checkbox" checked={draft.lowLatency} onChange={event => edit({ lowLatency: event.target.checked })} />Prefer low latency</label>
          <p className="muted">Can reduce stream delay with less buffering resilience. Actual delay depends on the stream, network and player; no specific latency is guaranteed.</p>
          <label className="checkbox-label"><input type="checkbox" checked={draft.automaticChat} onChange={event => edit({ automaticChat: event.target.checked })} />Open browser chat when playback starts</label>
          <p className="muted">Opens Twitch chat in your default browser for each launch or explicit restart. Channel details can override quality, low latency and chat independently.</p>
          <p className="muted">Changing preferences leaves running streams unchanged. Explicit Quit stops playback.</p>
        </div>
        <div hidden={section !== "Streamlink"} className="setting-group">
          <label>Streamlink executable<input value={draft.streamlinkPath ?? ""} placeholder="Automatic discovery" maxLength={4096} onChange={event => { edit({ streamlinkPath: event.target.value || null }); setProbe(null); }} /></label>
          <p className="muted">{draft.streamlinkPath ? "Explicit executable override" : "Automatic discovery from PATH and standard installation locations"}</p>
          <button type="button" disabled={saving} onClick={() => { void run("probe", async () => {
            setProbe(null);
            await commit(async () => {
              const result = await api.probe({ customPath: draft.streamlinkPath });
              setProbe(result); return api.playbackSettings();
            });
          }); }}>Test and save Streamlink path</button>
          <p className="muted path">Detected: {probe?.executable ?? "Not tested"}<br />Version: {probe?.version ?? "—"}</p>
          <p className="muted">Install Streamlink 8 or newer separately. A failed test preserves the saved path. Streamlink configuration files and sideloaded plugins are disabled.</p>
        </div>
        <div hidden={section !== "Player"} className="setting-group">
          <label>Player<select value={mode} onChange={event => edit({ player: { mode: event.target.value as PlayerMode, executable: null } })}>
            <option value="default">Streamlink default</option><option value="mpv">mpv</option><option value="vlc">VLC</option><option value="custom">Custom executable</option>
          </select></label>
          {mode !== "default" && <label>Player executable{mode !== "custom" && " (optional override)"}<input aria-label="Player executable" value={draft.player.executable ?? ""} placeholder={mode === "custom" ? "Full executable path" : "Automatic discovery"} maxLength={4096} required={mode === "custom"} onChange={event => edit({ player: { executable: event.target.value || null } })} /></label>}
          <p className="muted">{mode === "default" ? "Streamlink selects its default installed player." : draft.player.executable ? "Explicit player executable override" : "Automatic discovery of the selected player"}</p>
          <button type="button" disabled={saving} onClick={() => { void run("players", async () => setPlayers(await api.discoverPlayers())); }}>Find installed players</button>
          {players && <p className="muted path">mpv: {players.mpv ?? "Not found"}<br />VLC: {players.vlc ?? "Not found"}</p>}
          <details className="player-arguments"><summary>Player arguments ({draft.player.arguments.length})</summary>
            <p className="muted">One literal argument per row. Spaces, quotes, braces and empty arguments are preserved. No shell expansion.</p>
            {draft.player.arguments.map((argument, index) => <div className="argument-row" key={index}>
              <label>Player argument {index + 1}<input value={argument} maxLength={4096} onChange={event => edit({ player: { arguments: draft.player.arguments.map((value, i) => i === index ? event.target.value : value) } })} /></label>
              <button type="button" aria-label={`Remove argument ${index + 1}`} onClick={() => edit({ player: { arguments: draft.player.arguments.filter((_, i) => i !== index) } })}>Remove</button>
            </div>)}
            <button type="button" disabled={draft.player.arguments.length >= 32} onClick={() => edit({ player: { arguments: [...draft.player.arguments, ""] } })}>Add argument</button>
          </details>
        </div>
        <div hidden={section !== "Appearance"} className="setting-group">
          <label>Text size<select value={draft.textScale} onChange={event => edit({ textScale: event.target.value as TextScale })}><option value="100">100%</option><option value="125">125%</option><option value="150">150%</option></select></label>
          <label>Appearance<select value={draft.theme} onChange={event => edit({ theme: event.target.value as Theme })}><option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option></select></label>
          <p className="muted">System follows the color preference provided by your desktop. Save to apply your selection.</p>
        </div>
        <div hidden={section !== "Background"} className="setting-group">
          <BackgroundSettings value={draft.background} change={background => edit({ background })} desktop={desktop} />
        </div>
        <div hidden={section !== "Shortcuts"}>
          <p className="muted">Available while this app is focused. Shortcuts pause while typing, choosing an input value, or using a modal dialog.</p>
          <dl className="shortcut-reference">{Object.entries(shortcuts).map(([action, label]) => <div key={action}><dt>{action === "search" ? "Focus Search" : action[0].toUpperCase() + action.slice(1)}</dt><dd><kbd>{label}</kbd></dd></div>)}</dl>
        </div>
        {section !== "Shortcuts" && <div className="settings-save"><button type="submit" disabled={saving}>Save settings</button><button type="button" onClick={() => { setEdits({}); setProbe(null); setError(null); setMessage("Unsaved changes discarded."); }}>Cancel changes</button><span className="muted">{JSON.stringify(draft) !== JSON.stringify(saved) ? "Unsaved changes" : "Saved preferences"}</span></div>}
      </fieldset>
      {(busy || saving) && <p role="status">{busy === "probe" ? "Testing Streamlink…" : "Working…"}</p>}
      {error && <p className="error" role="alert">{error}</p>}
      {message && <p role="status">{message}</p>}
    </form>
  </div>;
}
