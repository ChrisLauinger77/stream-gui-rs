import { useI18n, type MessageKey } from "../i18n";
import { useEffect, useRef, useState } from "react";
import { useSettings } from "../settings/useSettings";
import { api } from "../lib/ipc";
import type { ChannelOverrides, ChannelSettings, ErrorCode, QualityPolicy } from "../lib/generated";
import { useQualityLabels } from "../playback/QualitySelect";
import { playbackError } from "../playback/usePlayback";
import { errorCode, errorText } from "../browse/errors";

type PreferenceError = { code: ErrorCode; context: "playback" | "general" };

export function ChannelPreferences({ broadcasterId, sessionId }: { broadcasterId: string; sessionId: string }) {
  const { t, locale } = useI18n();
  const qualityLabels = useQualityLabels();
  const { settings, readChannel, saveChannel } = useSettings();
  const [saved, setSaved] = useState<ChannelSettings | null>(null);
  const [edits, setEdits] = useState<Partial<ChannelOverrides>>({});
  // This component is keyed by broadcaster and UI session. Global refreshes
  // replace only the Rust snapshot; deliberate edits and save guards survive.
  const draft: ChannelOverrides = { quality: null, automaticChat: null, notifications: null, lowLatency: null, ...saved?.overrides, ...edits };
  const edit = (change: Partial<ChannelOverrides>) => setEdits(current => ({ ...current, ...change }));
  const [error, setError] = useState<PreferenceError | null>(null);
  const [message, setMessage] = useState<MessageKey | null>(null);
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
    }).catch(error => { if (!request.signal.aborted) setError({ code: errorCode(error), context: "playback" }); });
    return () => request.abort();
  }, [broadcasterId, settings, retry, readChannel]);
  const openChat = async (browser = false) => {
    if (opening.current) return;
    opening.current = true; setChatBusy(true); setError(null); setMessage(null);
    try {
      await (browser ? api.openBrowserChat : api.openChat)({ authSessionId: sessionId, broadcasterId });
      if (mounted.current) setMessage(browser || settings?.chatProvider !== "chatterino" ? "channel.chatBrowserOpened" : "channel.chatterinoRequested");
    } catch (error) { if (mounted.current) setError({ code: errorCode(error), context: "general" }); }
    finally { if (mounted.current) { opening.current = false; setChatBusy(false); } }
  };
  return <section className="channel-preferences" aria-label={t("channelPreferences.channelPreferences")}>
    <button type="button" disabled={chatBusy} onClick={() => { void openChat(settings?.chatProvider !== "chatterino"); }}>{settings?.chatProvider === "chatterino" ? t("channel.openChatterino") : t("channelPreferences.openChatInBrowser")}</button>
    {settings?.chatProvider === "chatterino" && <button type="button" disabled={chatBusy} onClick={() => { void openChat(true); }}>{t("channelPreferences.openChatInBrowser")}</button>}
    <details><summary>{t("channelPreferences.channelSettings")}</summary>
      {!saved ? error ? <button onClick={() => setRetry(value => value + 1)}>{t("channelPreferences.retryChannelSettings")}</button> : <p role="status">{t("channelPreferences.loadingChannelSettings")}</p> : <form onSubmit={event => {
        event.preventDefault();
        if (saving.current) return;
        saving.current = true; setBusy(true); setError(null); setMessage(null);
        void saveChannel({ broadcasterId, overrides: draft }).then(value => {
          if (mounted.current) { setSaved(value); setEdits({}); setMessage("channel.saved"); }
        }).catch(error => { if (mounted.current) setError({ code: errorCode(error), context: "playback" }); })
          .finally(() => { if (mounted.current) { saving.current = false; setBusy(false); } });
      }}>
        <fieldset disabled={busy}>
          <label>{t("channelPreferences.channelQuality")}<select value={draft.quality ?? "inherit"} onChange={event => edit({ quality: event.target.value === "inherit" ? null : event.target.value as QualityPolicy })}>
            <option value="inherit">{t("channel.inheritQuality", { quality: qualityLabels[saved.defaultQuality] })}</option>
            {Object.entries(qualityLabels).map(([value, label]) => <option key={value} value={value}>{label}</option>)}
          </select></label>
          <label>{t("channelPreferences.channelLowLatency")}<select value={draft.lowLatency === null ? "inherit" : draft.lowLatency ? "on" : "off"} onChange={event => edit({ lowLatency: event.target.value === "inherit" ? null : event.target.value === "on" })}>
            <option value="inherit">{t("channel.inheritLowLatency", { state: t(saved.defaultLowLatency ? "channelPreferences.on" : "channelPreferences.off") })}</option><option value="on">{t("channelPreferences.on")}</option><option value="off">{t("channelPreferences.off")}</option>
          </select></label>
          <label>{t("channelPreferences.channelAutomaticChat")}<select value={draft.automaticChat === null ? "inherit" : draft.automaticChat ? "on" : "off"} onChange={event => edit({ automaticChat: event.target.value === "inherit" ? null : event.target.value === "on" })}>
            <option value="inherit">{t("channel.inheritChat", { state: t(saved.defaultAutomaticChat ? "channel.openChat" : "channelPreferences.keepChatClosed") })}</option><option value="on">{t("channelPreferences.openChatWithPlayback")}</option><option value="off">{t("channelPreferences.keepChatClosed")}</option>
          </select></label>
          <label>{t("channelPreferences.channelNotifications")}<select value={draft.notifications === null ? "inherit" : draft.notifications ? "on" : "off"} onChange={event => edit({ notifications: event.target.value === "inherit" ? null : event.target.value === "on" })}>
            <option value="inherit">{t("channel.inheritNotifications", { state: t(saved.defaultNotifications ? "channelPreferences.on" : "channelPreferences.off") })}</option><option value="on">{t("channelPreferences.notifyWhenLive")}</option><option value="off">{t("channelPreferences.doNotNotify")}</option>
          </select></label>
          <p className="muted">{t("channel.notificationsStatus", { state: t(saved.effectiveNotifications ? "channelPreferences.on" : "channelPreferences.off") })}</p>
          <p className="muted">{t("channel.effectiveSummary", { quality: qualityLabels[saved.effective.quality], latency: t(saved.effective.lowLatency ? "channelPreferences.on" : "channelPreferences.off"), chat: t(saved.effective.automaticChat ? "channelPreferences.on" : "channelPreferences.off") })}</p>
          <div className="settings-save"><button type="submit">{t("channelPreferences.saveChannelSettings")}</button><button type="button" onClick={() => setEdits({})}>{t("channelPreferences.cancelChanges")}</button></div>
          <p className="muted">{t("channel.scopeHelp")}</p>
        </fieldset>
      </form>}
    </details>
    {error && <p role="alert" className="error">{error.context === "playback" ? playbackError({ code: error.code }, locale) : errorText(error.code, locale)}</p>}
    {message && <p role="status">{t(message)}</p>}
  </section>;
}
