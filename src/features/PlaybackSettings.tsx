import { useState } from "react";
import { api } from "../lib/ipc";
import type { PlayerDiscovery, PlayerMode, ProbeResult, Settings } from "../lib/generated";
import { QualitySelect } from "../playback/QualitySelect";
import { playbackError } from "../playback/usePlayback";

export function PlaybackSettings({ saved, onSaved }: { saved: Settings | null; onSaved: (value: Settings) => void }) {
  // Mount the editable form only after Rust's persisted snapshot is available.
  if (!saved) return <p role="status">Loading playback settings…</p>;
  return <SettingsForm initial={saved} onSaved={onSaved} />;
}
function SettingsForm({ initial, onSaved }: { initial: Settings; onSaved: (value: Settings) => void }) {
  const [draft, setDraft] = useState(initial);
  const [probe, setProbe] = useState<ProbeResult | null>(null);
  const [players, setPlayers] = useState<PlayerDiscovery | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const run = async (key: string, action: () => Promise<void>) => {
    if (busy) return;
    setBusy(key); setError(null); setMessage(null);
    try { await action(); } catch (error) { setError(playbackError(error)); }
    finally { setBusy(null); }
  };
  const mode = draft.player.mode;
  return <form className="playback-settings" aria-label="Playback settings" onSubmit={event => {
    event.preventDefault();
    void run("save", async () => {
      const saved = await api.savePlaybackSettings(draft);
      setDraft(saved); onSaved(saved); setMessage("Playback settings saved. New launches and restarts use these settings.");
    });
  }}>
    <fieldset disabled={!!busy}>
      <legend>Playback</legend>
      <div className="playback-settings-grid">
        <div className="setting-group">
          <label>Streamlink executable<input value={draft.streamlinkPath ?? ""} placeholder="Automatic discovery" maxLength={4096} onChange={event => { setDraft({ ...draft, streamlinkPath: event.target.value || null }); setProbe(null); }} /></label>
          <button type="button" onClick={() => { void run("probe", async () => {
            setProbe(null);
            const result = await api.probe({ customPath: draft.streamlinkPath });
            setProbe(result);
            // Probe saves only this path. Preserve other unsaved form edits.
            onSaved(await api.playbackSettings());
          }); }}>Test and save Streamlink path</button>
          <p className="muted path">Detected: {probe?.executable ?? "Not tested"}<br />Version: {probe?.version ?? "—"}</p>
        </div>
        <div className="setting-group">
          <label>Player<select value={mode} onChange={event => setDraft({ ...draft, player: { ...draft.player, mode: event.target.value as PlayerMode, executable: null } })}>
            <option value="default">Streamlink default</option><option value="mpv">mpv</option><option value="vlc">VLC</option><option value="custom">Custom executable</option>
          </select></label>
          {mode !== "default" && <label>Player executable{mode !== "custom" && " (optional override)"}<input aria-label="Player executable" value={draft.player.executable ?? ""} placeholder={mode === "custom" ? "Full executable path" : "Automatic discovery"} maxLength={4096} required={mode === "custom"} onChange={event => setDraft({ ...draft, player: { ...draft.player, executable: event.target.value || null } })} /></label>}
          <button type="button" onClick={() => { void run("players", async () => setPlayers(await api.discoverPlayers())); }}>Find installed players</button>
          {players && <p className="muted path">mpv: {players.mpv ?? "Not found"}<br />VLC: {players.vlc ?? "Not found"}</p>}
        </div>
        <div className="setting-group">
          <QualitySelect label="Default quality" value={draft.defaultQuality} change={quality => setDraft({ ...draft, defaultQuality: quality })} />
          <p className="muted">High, Medium and Low prefer the indicated cap, falling back to Source if no suitable rendition exists. Audio depends on the stream.</p>
        </div>
      </div>
      <details className="player-arguments"><summary>Player arguments ({draft.player.arguments.length})</summary>
        <p className="muted">One literal argument per row. Spaces, quotes, braces and empty values are preserved. Do not add quotes around a path. No shell or substitutions are used.</p>
        {draft.player.arguments.map((argument, index) => <div className="argument-row" key={index}>
          <label>Player argument {index + 1}<input value={argument} maxLength={4096} onChange={event => setDraft({ ...draft, player: { ...draft.player, arguments: draft.player.arguments.map((value, i) => i === index ? event.target.value : value) } })} /></label>
          <button type="button" aria-label={`Remove argument ${index + 1}`} onClick={() => setDraft({ ...draft, player: { ...draft.player, arguments: draft.player.arguments.filter((_, i) => i !== index) } })}>Remove</button>
        </div>)}
        <button type="button" disabled={draft.player.arguments.length >= 32} onClick={() => setDraft({ ...draft, player: { ...draft.player, arguments: [...draft.player.arguments, ""] } })}>Add argument</button>
      </details>
      <button type="submit">Save playback settings</button>
    </fieldset>
    {busy && <p role="status">{busy === "probe" ? "Testing Streamlink…" : "Working…"}</p>}
    {error && <p className="error" role="alert">{error}</p>}
    {message && <p role="status">{message}</p>}
    <p className="muted">Install Streamlink 8 or newer and an external player separately. Streamlink default uses its own player discovery. User Streamlink configuration and sideloaded plugins are disabled for predictable launches.</p>
  </form>;
}
