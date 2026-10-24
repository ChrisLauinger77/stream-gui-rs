import { useEffect, useRef, useState, type ReactNode } from "react";
import { useI18n, type MessageKey } from "../i18n";
import type { StreamingService } from "../lib/generated";
import { ExactChannelLookup } from "./ExactChannelLookup";
import { AnonymousBookmark } from "../features/DiscoveryPreferences";
import { KickChatButton, YoutubeChatButton } from "../features/BrowserChatButton";

export type OpenChannelDraft = { service: StreamingService; twitchLogin: string; kickSlug: string; youtubeLocator: string };

export function OpenChannel({ sessionId, draft, change, open, onAuthLost, watchKick, watchYoutube }: {
  sessionId: string | null; draft: OpenChannelDraft; change: (draft: OpenChannelDraft) => void;
  open: (id: string, name: string) => void; onAuthLost: () => void;
  watchKick: (slug: string) => Promise<void>; watchYoutube: (locator: string) => Promise<void>;
}) {
  const { t } = useI18n();
  return <div className="open-channel">
    <label>{t("openChannel.service")}<select data-focus="channel-service" value={draft.service} onChange={event => change({ ...draft, service: event.target.value as StreamingService })}>
      <option value="twitch">{t("service.twitch")}</option><option value="kick">{t("service.kick")}</option><option value="youtube">{t("service.youtube")}</option>
    </select></label>
    {draft.service === "kick" ? <AnonymousChannel kind="kick_channel" watch={watchKick} value={draft.kickSlug} change={kickSlug => change({ ...draft, kickSlug })} focus="kick-slug" maxLength={100} keys={kickKeys} chat={<KickChatButton slug={draft.kickSlug} />} /> :
      draft.service === "youtube" ? <AnonymousChannel kind="youtube" watch={watchYoutube} value={draft.youtubeLocator} change={youtubeLocator => change({ ...draft, youtubeLocator })} focus="youtube-locator" maxLength={31} keys={youtubeKeys} chat={<YoutubeChatButton locator={draft.youtubeLocator} />} /> :
      sessionId ? <ExactChannelLookup sessionId={sessionId} login={draft.twitchLogin} change={twitchLogin => change({ ...draft, twitchLogin })} open={open} onAuthLost={onAuthLost} /> :
      <p role="status">{t("openChannel.twitchSignIn")}</p>}
  </div>;
}

type Keys = { label: MessageKey; help: MessageKey; play: MessageKey; starting: MessageKey; playback: MessageKey };
const kickKeys: Keys = { label: "kick.channel", help: "kick.locatorHelp", play: "kick.play", starting: "kick.starting", playback: "kick.playbackHelp" };
const youtubeKeys: Keys = { label: "youtube.locator", help: "youtube.locatorHelp", play: "youtube.play", starting: "kick.starting", playback: "youtube.playbackHelp" };

// Kick and YouTube take a bare locator without a Twitch session or lookup.
function AnonymousChannel({ kind, watch, value, change, focus, maxLength, keys, chat }: {
  kind: "kick_channel" | "youtube"; watch: (value: string) => Promise<void>; value: string; change: (value: string) => void;
  focus: string; maxLength: number; keys: Keys; chat: ReactNode;
}) {
  const { t } = useI18n();
  const [pending, setPending] = useState(false);
  const busy = useRef(false);
  const mounted = useRef(false);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  const submit = async () => {
    if (busy.current || !value) return;
    busy.current = true; setPending(true);
    try { await watch(value); }
    finally { busy.current = false; if (mounted.current) setPending(false); }
  };
  return <form className="search-form exact-lookup" onSubmit={event => { event.preventDefault(); void submit(); }}>
    <label>{t(keys.label)}<input data-focus={focus} autoCapitalize="none" autoCorrect="off" spellCheck={false} maxLength={maxLength} value={value} disabled={pending} onChange={event => change(event.target.value)} /></label>
    <p className="muted">{t(keys.help)}</p>
    <div className="session-actions"><button type="submit" disabled={pending || !value}>{t(keys.play)}</button>{chat}<AnonymousBookmark key={kind} kind={kind} locator={value} /></div>
    {pending && <p role="status">{t(keys.starting)}</p>}
    <p className="muted">{t(keys.playback)}</p>
  </form>;
}
