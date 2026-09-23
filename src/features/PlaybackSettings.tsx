import { useRef, useState } from "react";
import { api } from "../lib/ipc";
import type { ChatProvider, PlayerDiscovery, ProbeResult, Settings, TextScale, Theme } from "../lib/generated";
import { QualitySelect } from "../playback/QualitySelect";
import { playbackError } from "../playback/usePlayback";
import { ShortcutEditor } from "./ShortcutEditor";

import { SavedItems } from "./DiscoveryPreferences";
import { PlayerFields } from "./PlayerFields";
import { PlayerProfiles } from "./PlayerProfiles";
import { UpdateAwareness } from "./UpdateAwareness";
import { BackgroundSettings } from "./BackgroundSettings";
import type { useDesktop } from "../app/useDesktop";

const sections = ["Playback", "Streamlink", "Player", "Appearance", "Background", "Updates", "Shortcuts", "Hidden items"] as const;
type Section = typeof sections[number];
type SettingsProps = { desktop: ReturnType<typeof useDesktop>; saved: Settings | null; saving: boolean; commit: (action: () => Promise<Settings>) => Promise<Settings> };
type DraftEdits = Omit<Partial<Settings>, "player" | "background" | "profiles" | "selectedProfileId"> & {
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
  const [chatterino, setChatterino] = useState<string | null | undefined>(undefined);
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
          <label className="checkbox-label"><input type="checkbox" checked={draft.automaticChat} onChange={event => edit({ automaticChat: event.target.checked })} />Open chat when playback starts</label>
          <p className="muted">Opens the selected chat application for each launch or explicit restart. Channel details can override quality, low latency and chat independently.</p>
          <label>Chat application<select value={draft.chatProvider} onChange={event => edit({ chatProvider: event.target.value as ChatProvider })}><option value="browser">Browser</option><option value="chatterino">Chatterino</option></select></label>
          {draft.chatProvider === "chatterino" && <>
            <label>Chatterino executable (optional override)<input value={draft.chatterinoPath ?? ""} maxLength={4096} placeholder="Automatic discovery" onChange={event => edit({ chatterinoPath: event.target.value || null })} /></label>
            <button type="button" disabled={saving} onClick={() => { void run("chatterino", async () => setChatterino(await api.discoverChatterino())); }}>Find Chatterino</button>
            {chatterino !== undefined && <p className="muted path">Chatterino: {chatterino ?? "Not found; install it or set an executable path."}</p>}
            <p className="muted">Chatterino uses its own login. Each opening may start an independent window, which stays open after playback stops or this app quits. Missing Chatterino shows an error; channel details always offer browser chat.</p>
          </>}
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
          <PlayerProfiles saved={saved} saving={saving || !!busy} commit={commit} />
          <h3>Default configuration</h3>
          <p className="muted">Used when no profile is selected. Profiles replace the player; optional profile quality and low latency override these defaults.</p>
          <PlayerFields value={draft.player} change={player => edit({ player })} />
          <button type="button" disabled={saving} onClick={() => { void run("players", async () => setPlayers(await api.discoverPlayers())); }}>Find installed players</button>
          {players && <p className="muted path">mpv: {players.mpv ?? "Not found"}<br />VLC: {players.vlc ?? "Not found"}</p>}
        </div>
        <div hidden={section !== "Appearance"} className="setting-group">
          <label>Text size<select value={draft.textScale} onChange={event => edit({ textScale: event.target.value as TextScale })}><option value="100">100%</option><option value="125">125%</option><option value="150">150%</option></select></label>
          <label>Appearance<select value={draft.theme} onChange={event => edit({ theme: event.target.value as Theme })}><option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option></select></label>
          <p className="muted">System follows the color preference provided by your desktop. Save to apply your selection.</p>
        </div>
        <div hidden={section !== "Background"} className="setting-group">
          <BackgroundSettings value={draft.background} change={background => edit({ background })} desktop={desktop} />
        </div>
        {section === "Hidden items" && <SavedItems list="hidden" />}
        {section === "Updates" && <UpdateAwareness />}
        {section === "Shortcuts" && <ShortcutEditor />}
        {section !== "Shortcuts" && section !== "Updates" && section !== "Hidden items" && <div className="settings-save"><button type="submit" disabled={saving}>Save settings</button><button type="button" onClick={() => { setEdits({}); setProbe(null); setError(null); setMessage("Unsaved changes discarded."); }}>Cancel changes</button><span className="muted">{JSON.stringify(draft) !== JSON.stringify(saved) ? "Unsaved changes" : "Saved preferences"}</span></div>}
      </fieldset>
      {(busy || saving) && <p role="status">{busy === "probe" ? "Testing Streamlink…" : "Working…"}</p>}
      {error && <p className="error" role="alert">{error}</p>}
      {message && <p role="status">{message}</p>}
    </form>
  </div>;
}
