import { useEffect, useRef, useState } from "react";
import { useI18n } from "../i18n";
import type { StreamingService } from "../lib/generated";
import { ExactChannelLookup } from "./ExactChannelLookup";
import { KickChatButton } from "../features/KickChatButton";

export type OpenChannelDraft = { service: StreamingService; twitchLogin: string; kickSlug: string };

export function OpenChannel({ sessionId, draft, change, open, onAuthLost, watchKick }: {
  sessionId: string | null; draft: OpenChannelDraft; change: (draft: OpenChannelDraft) => void;
  open: (id: string, name: string) => void; onAuthLost: () => void;
  watchKick: (slug: string) => Promise<void>;
}) {
  const { t } = useI18n();
  return <div className="open-channel">
    <label>{t("openChannel.service")}<select data-focus="channel-service" value={draft.service} onChange={event => change({ ...draft, service: event.target.value as StreamingService })}>
      <option value="twitch">{t("service.twitch")}</option><option value="kick">{t("service.kick")}</option>
    </select></label>
    {draft.service === "kick" ? <KickChannel watch={watchKick} slug={draft.kickSlug} change={kickSlug => change({ ...draft, kickSlug })} /> : sessionId ?
      <ExactChannelLookup sessionId={sessionId} login={draft.twitchLogin} change={twitchLogin => change({ ...draft, twitchLogin })} open={open} onAuthLost={onAuthLost} /> :
      <p role="status">{t("openChannel.twitchSignIn")}</p>}
  </div>;
}

function KickChannel({ watch, slug, change }: { watch: (slug: string) => Promise<void>; slug: string; change: (slug: string) => void }) {
  const { t } = useI18n();
  const [pending, setPending] = useState(false);
  const busy = useRef(false);
  const mounted = useRef(false);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  const submit = async () => {
    if (busy.current || !slug) return;
    busy.current = true; setPending(true);
    try { await watch(slug); }
    finally { busy.current = false; if (mounted.current) setPending(false); }
  };
  return <form className="search-form exact-lookup" onSubmit={event => { event.preventDefault(); void submit(); }}>
    <label>{t("kick.channel")}<input data-focus="kick-slug" autoCapitalize="none" autoCorrect="off" spellCheck={false} maxLength={100} value={slug} disabled={pending} onChange={event => change(event.target.value)} /></label>
    <p className="muted">{t("kick.locatorHelp")}</p>
    <div className="session-actions"><button type="submit" disabled={pending || !slug}>{t("kick.play")}</button><KickChatButton slug={slug} /></div>
    {pending && <p role="status">{t("kick.starting")}</p>}
    <p className="muted">{t("kick.playbackHelp")}</p>
  </form>;
}
