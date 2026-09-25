import { useI18n } from "../i18n";
import { useEffect, useRef, useState } from "react";
import { api } from "../lib/ipc";
import type { ChannelIdentity, ErrorCode } from "../lib/generated";
import { errorCode, errorText } from "./errors";

export function ExactChannelLookup({ sessionId, login, change, open, onAuthLost }: {
  sessionId: string; login: string; change: (login: string) => void;
  open: (id: string, name: string) => void; onAuthLost: () => void;
}) {
  const { t, locale } = useI18n();
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<ErrorCode | null>(null);
  const [found, setFound] = useState<ChannelIdentity | null>(null);
  const generation = useRef(0);
  const busy = useRef(false);
  const form = useRef<HTMLFormElement>(null);
  useEffect(() => () => { generation.current++; }, [sessionId]);
  const submit = async () => {
    if (busy.current || !login) return;
    busy.current = true; setPending(true); setError(null); setFound(null);
    const version = ++generation.current;
    try {
      const result = await api.lookupChannel({ sessionId, login });
      if (version !== generation.current) return;
      if (document.activeElement === document.body || form.current?.contains(document.activeElement)) open(result.broadcasterId, result.displayName);
      else setFound(result); // A later response must not take focus from a newer interaction.
    } catch (failure) {
      if (version !== generation.current) return;
      const code = errorCode(failure);
      setError(code);
      if (code === "unauthenticated" || code === "auth_invalid") onAuthLost();
    } finally { if (version === generation.current) { busy.current = false; setPending(false); } }
  };
  return <form ref={form} className="search-form exact-lookup" onSubmit={event => { event.preventDefault(); void submit(); }}>
    <label>{t("exactChannelLookup.twitchLogin")}<input data-focus="exact-login" autoCapitalize="none" autoCorrect="off" spellCheck={false} maxLength={25} value={login} placeholder={t("lookup.loginHint")} onChange={event => {
      generation.current++; busy.current = false; setPending(false); setError(null); setFound(null); change(event.target.value);
    }} /></label>
    <p className="muted">{t("lookup.help")}</p>
    <button type="submit" data-focus="exact-submit" aria-disabled={pending || !login} onClick={event => { if (pending || !login) event.preventDefault(); }}>{t("exactChannelLookup.openChannel")}</button>
    {pending && <p role="status">{t("exactChannelLookup.lookingUpChannel")}</p>}
    {error && <p className="error" role="alert">{error === "invalid_input" ? t("lookup.invalidLogin") : error === "not_found" ? t("lookup.notFound") : errorText(error, locale)}</p>}
    {found && <div><p role="status">{t("lookup.ready")}</p><button onClick={() => open(found.broadcasterId, found.displayName)} type="button">{t("exactChannelLookup.openResolvedChannel")}</button></div>}
  </form>;
}
