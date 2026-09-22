import { useEffect, useRef, useState } from "react";
import { useSettings } from "../settings/useSettings";
import { api } from "../lib/ipc";
import type { ChannelOverrides, ChannelSettings, QualityPolicy } from "../lib/generated";
import { qualityLabels } from "../playback/QualitySelect";
import { playbackError } from "../playback/usePlayback";
import { friendlyError } from "../browse/errors";

export function ChannelPreferences({ broadcasterId, sessionId }: { broadcasterId: string; sessionId: string }) {
  const { settings, readChannel, saveChannel } = useSettings();
  const [saved, setSaved] = useState<ChannelSettings | null>(null);
  const [edits, setEdits] = useState<Partial<ChannelOverrides>>({});
  // This component is keyed by broadcaster and UI session. Global refreshes
  // replace only the Rust snapshot; deliberate edits and save guards survive.
  const draft: ChannelOverrides = { quality: null, automaticChat: null, notifications: null, lowLatency: null, ...saved?.overrides, ...edits };
  const edit = (change: Partial<ChannelOverrides>) => setEdits(current => ({ ...current, ...change }));
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [chatBusy, setChatBusy] = useState(false);
  const [retry, setRetry] = useState(0);
  const mounted = useRef(false);
  const saving = useRef(false);
  const opening = useRef(false);
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; };
  }, []);
  useEffect(() => {
    const request = new AbortController();
    void readChannel(broadcasterId, request.signal).then(value => {
      if (!request.signal.aborted && value) setSaved(value);
    }).catch(error => { if (!request.signal.aborted) setError(playbackError(error)); });
    return () => request.abort();
  }, [broadcasterId, settings, retry, readChannel]);
  const openChat = async (browser = false) => {
    if (opening.current) return;
    opening.current = true; setChatBusy(true); setError(null); setMessage(null);
    try {
      await (browser ? api.openBrowserChat : api.openChat)({ authSessionId: sessionId, broadcasterId });
      if (mounted.current) setMessage(browser || settings?.chatProvider !== "chatterino" ? "Twitch chat opened in your default browser." : "Chatterino launch requested.");
    } catch (error) { if (mounted.current) setError(friendlyError(error)); }
    finally { if (mounted.current) { opening.current = false; setChatBusy(false); } }
  };
  return <section className="channel-preferences" aria-label="Channel preferences">
    <button type="button" disabled={chatBusy} onClick={() => { void openChat(settings?.chatProvider !== "chatterino"); }}>{settings?.chatProvider === "chatterino" ? "Open chat in Chatterino" : "Open chat in browser"}</button>
    {settings?.chatProvider === "chatterino" && <button type="button" disabled={chatBusy} onClick={() => { void openChat(true); }}>Open chat in browser</button>}
    <details><summary>Channel settings</summary>
      {!saved ? error ? <button onClick={() => setRetry(value => value + 1)}>Retry channel settings</button> : <p role="status">Loading channel settings…</p> : <form onSubmit={event => {
        event.preventDefault();
        if (saving.current) return;
        saving.current = true; setBusy(true); setError(null); setMessage(null);
        void saveChannel({ broadcasterId, overrides: draft }).then(value => {
          if (mounted.current) { setSaved(value); setEdits({}); setMessage("Channel settings saved. Running streams are unchanged."); }
        }).catch(error => { if (mounted.current) setError(playbackError(error)); })
          .finally(() => { if (mounted.current) { saving.current = false; setBusy(false); } });
      }}>
        <fieldset disabled={busy}>
          <label>Channel quality<select value={draft.quality ?? "inherit"} onChange={event => edit({ quality: event.target.value === "inherit" ? null : event.target.value as QualityPolicy })}>
            <option value="inherit">Use playback default ({qualityLabels[saved.defaultQuality]})</option>
            {Object.entries(qualityLabels).map(([value, label]) => <option key={value} value={value}>{label}</option>)}
          </select></label>
          <label>Channel low latency<select value={draft.lowLatency === null ? "inherit" : draft.lowLatency ? "on" : "off"} onChange={event => edit({ lowLatency: event.target.value === "inherit" ? null : event.target.value === "on" })}>
            <option value="inherit">Use default ({saved.defaultLowLatency ? "On" : "Off"})</option><option value="on">On</option><option value="off">Off</option>
          </select></label>
          <label>Channel automatic chat<select value={draft.automaticChat === null ? "inherit" : draft.automaticChat ? "on" : "off"} onChange={event => edit({ automaticChat: event.target.value === "inherit" ? null : event.target.value === "on" })}>
            <option value="inherit">Use global default ({saved.defaultAutomaticChat ? "Open chat" : "Keep chat closed"})</option><option value="on">Open chat with playback</option><option value="off">Keep chat closed</option>
          </select></label>
          <label>Channel notifications<select value={draft.notifications === null ? "inherit" : draft.notifications ? "on" : "off"} onChange={event => edit({ notifications: event.target.value === "inherit" ? null : event.target.value === "on" })}>
            <option value="inherit">Use global default ({saved.defaultNotifications ? "On" : "Off"})</option><option value="on">Notify when live</option><option value="off">Do not notify</option>
          </select></label>
          <p className="muted">Saved notifications: {saved.effectiveNotifications ? "On" : "Off"}. Requires monitoring and system permission.</p>
          <p className="muted">Saved effective quality: {qualityLabels[saved.effective.quality]} · Low latency: {saved.effective.lowLatency ? "On" : "Off"} · Automatic chat: {saved.effective.automaticChat ? "On" : "Off"}</p>
          <div className="settings-save"><button type="submit">Save channel settings</button><button type="button" onClick={() => setEdits({})}>Cancel changes</button></div>
          <p className="muted">These preferences follow this broadcaster across name changes and apply to this app on this device. Choose the default to remove an override. Quality and low latency can inherit from the active player profile.</p>
        </fieldset>
      </form>}
    </details>
    {error && <p role="alert" className="error">{error}</p>}
    {message && <p role="status">{message}</p>}
  </section>;
}
