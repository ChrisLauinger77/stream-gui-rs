import { useEffect, useRef, useState } from "react";
import { api } from "../lib/ipc";
import type { ChannelOverrides, ChannelSettings, QualityPolicy } from "../lib/generated";
import { qualityLabels } from "../playback/QualitySelect";
import { playbackError } from "../playback/usePlayback";
import { friendlyError } from "../browse/errors";

export function ChannelPreferences({ broadcasterId, sessionId, settingsRevision }: { broadcasterId: string; sessionId: string; settingsRevision: number }) {
  const [saved, setSaved] = useState<ChannelSettings | null>(null);
  const [draft, setDraft] = useState<ChannelOverrides>({ quality: null, automaticChat: null, notifications: null, lowLatency: null });
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [chatBusy, setChatBusy] = useState(false);
  const [retry, setRetry] = useState(0);
  const revision = useRef(0);
  const saving = useRef(false);
  const opening = useRef(false);
  useEffect(() => {
    const version = ++revision.current;
    setSaved(null); setError(null); setMessage(null); setBusy(false); setChatBusy(false); saving.current = false; opening.current = false;
    void api.channelSettings(broadcasterId).then(value => {
      if (version === revision.current) { setSaved(value); setDraft(value.overrides); }
    }).catch(error => { if (version === revision.current) setError(playbackError(error)); });
    return () => { revision.current++; };
  }, [broadcasterId, sessionId, settingsRevision, retry]);
  const openChat = async () => {
    if (opening.current) return;
    opening.current = true; setChatBusy(true); setError(null); setMessage(null);
    const version = revision.current;
    try {
      await api.openChat({ authSessionId: sessionId, broadcasterId });
      if (version === revision.current) setMessage("Twitch chat opened in your default browser.");
    } catch (error) { if (version === revision.current) setError(friendlyError(error)); }
    finally { if (version === revision.current) { opening.current = false; setChatBusy(false); } }
  };
  return <section className="channel-preferences" aria-label="Channel preferences">
    <button type="button" disabled={chatBusy} onClick={() => { void openChat(); }}>Open chat in browser</button>
    <details><summary>Channel settings</summary>
      {!saved ? error ? <button onClick={() => setRetry(value => value + 1)}>Retry channel settings</button> : <p role="status">Loading channel settings…</p> : <form onSubmit={event => {
        event.preventDefault();
        if (saving.current) return;
        saving.current = true; setBusy(true); setError(null); setMessage(null);
        const version = revision.current;
        void api.saveChannelSettings({ broadcasterId, overrides: draft }).then(value => {
          if (version === revision.current) { setSaved(value); setDraft(value.overrides); setMessage("Channel settings saved. Running streams are unchanged."); }
        }).catch(error => { if (version === revision.current) setError(playbackError(error)); })
          .finally(() => { if (version === revision.current) { saving.current = false; setBusy(false); } });
      }}>
        <fieldset disabled={busy}>
          <label>Channel quality<select value={draft.quality ?? "inherit"} onChange={event => setDraft({ ...draft, quality: event.target.value === "inherit" ? null : event.target.value as QualityPolicy })}>
            <option value="inherit">Use global default ({qualityLabels[saved.defaultQuality]})</option>
            {Object.entries(qualityLabels).map(([value, label]) => <option key={value} value={value}>{label}</option>)}
          </select></label>
          <label>Channel browser chat<select value={draft.automaticChat === null ? "inherit" : draft.automaticChat ? "on" : "off"} onChange={event => setDraft({ ...draft, automaticChat: event.target.value === "inherit" ? null : event.target.value === "on" })}>
            <option value="inherit">Use global default ({saved.defaultAutomaticChat ? "Open chat" : "Keep chat closed"})</option><option value="on">Open chat with playback</option><option value="off">Keep chat closed</option>
          </select></label>
          <label>Channel notifications<select value={draft.notifications === null ? "inherit" : draft.notifications ? "on" : "off"} onChange={event => setDraft({ ...draft, notifications: event.target.value === "inherit" ? null : event.target.value === "on" })}>
            <option value="inherit">Use global default ({saved.defaultNotifications ? "On" : "Off"})</option><option value="on">Notify when live</option><option value="off">Do not notify</option>
          </select></label>
          <p className="muted">Saved notifications: {saved.effectiveNotifications ? "On" : "Off"}. Requires monitoring and system permission.</p>
          <p className="muted">Saved effective quality: {qualityLabels[saved.effective.quality]} · Browser chat: {saved.effective.automaticChat ? "On" : "Off"}</p>
          <div className="settings-save"><button type="submit">Save channel settings</button><button type="button" onClick={() => setDraft(saved.overrides)}>Cancel changes</button></div>
          <p className="muted">These preferences follow this broadcaster across name changes and apply to this app on this device. Choose the global default to remove an override.</p>
        </fieldset>
      </form>}
    </details>
    {error && <p role="alert" className="error">{error}</p>}
    {message && <p role="status">{message}</p>}
  </section>;
}
