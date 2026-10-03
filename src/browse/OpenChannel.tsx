import { useEffect, useRef, useState } from "react";
import { useI18n } from "../i18n";
import type { StreamingService } from "../lib/generated";
import { ExactChannelLookup } from "./ExactChannelLookup";
import { KickChatButton } from "../features/KickChatButton";

export function OpenChannel({ sessionId, login, change, open, onAuthLost, watchKick }: {
  sessionId: string | null; login: string; change: (login: string) => void;
  open: (id: string, name: string) => void; onAuthLost: () => void;
  watchKick: (slug: string) => Promise<void>;
}) {
  const { t } = useI18n();
  const [service, setService] = useState<StreamingService>(sessionId ? "twitch" : "kick");
  return <div className="open-channel">
    <label>{t("openChannel.service")}<select data-focus="channel-service" value={service} onChange={event => setService(event.target.value as StreamingService)}>
      <option value="twitch">{t("service.twitch")}</option><option value="kick">{t("service.kick")}</option>
    </select></label>
    {service === "kick" ? <KickChannel watch={watchKick} /> : sessionId ?
      <ExactChannelLookup sessionId={sessionId} login={login} change={change} open={open} onAuthLost={onAuthLost} /> :
      <p role="status">{t("openChannel.twitchSignIn")}</p>}
  </div>;
}

function KickChannel({ watch }: { watch: (slug: string) => Promise<void> }) {
  const { t } = useI18n();
  const [slug, setSlug] = useState("");
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
    <label>{t("kick.channel")}<input data-focus="kick-slug" autoCapitalize="none" autoCorrect="off" spellCheck={false} maxLength={100} value={slug} disabled={pending} onChange={event => setSlug(event.target.value)} /></label>
    <p className="muted">{t("kick.locatorHelp")}</p>
    <div className="session-actions"><button type="submit" disabled={pending || !slug}>{t("kick.play")}</button><KickChatButton slug={slug} /></div>
    {pending && <p role="status">{t("kick.starting")}</p>}
    <p className="muted">{t("kick.playbackHelp")}</p>
  </form>;
}
