import { useEffect, useRef, useState } from "react";
import { api } from "../lib/ipc";
import type { ChannelIdentity } from "../lib/generated";
import { errorCode, friendlyError } from "./errors";

export function ExactChannelLookup({ sessionId, login, change, open, onAuthLost }: {
  sessionId: string; login: string; change: (login: string) => void;
  open: (id: string, name: string) => void; onAuthLost: () => void;
}) {
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
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
      setError(code === "invalid_input" ? "Enter a Twitch login using up to 25 letters, numbers or underscores. Do not enter a URL." : code === "not_found" ? "No channel has that Twitch login." : friendlyError(failure));
      if (code === "unauthenticated" || code === "auth_invalid") onAuthLost();
    } finally { if (version === generation.current) { busy.current = false; setPending(false); } }
  };
  return <form ref={form} className="search-form exact-lookup" onSubmit={event => { event.preventDefault(); void submit(); }}>
    <label>Twitch login<input data-focus="exact-login" autoCapitalize="none" autoCorrect="off" spellCheck={false} maxLength={25} value={login} placeholder="Channel login, not a URL" onChange={event => {
      generation.current++; busy.current = false; setPending(false); setError(null); setFound(null); change(event.target.value);
    }} /></label>
    <p className="muted">Opens the exact channel’s details, including offline channels. Use Search to discover channels and categories.</p>
    <button type="submit" data-focus="exact-submit" aria-disabled={pending || !login} onClick={event => { if (pending || !login) event.preventDefault(); }}>Open channel</button>
    {pending && <p role="status">Looking up channel…</p>}
    {error && <p className="error" role="alert">{error}</p>}
    {found && <div><p role="status">Channel found. Open its details when ready.</p><button onClick={() => open(found.broadcasterId, found.displayName)} type="button">Open resolved channel</button></div>}
  </form>;
}
