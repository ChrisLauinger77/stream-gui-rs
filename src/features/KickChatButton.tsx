import { useEffect, useRef, useState } from "react";
import { api } from "../lib/ipc";
import type { ErrorCode } from "../lib/generated";
import { errorCode, errorText } from "../browse/errors";
import { useI18n } from "../i18n";

export function KickChatButton({ slug }: { slug: string }) {
  const { t, locale } = useI18n();
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<ErrorCode | null>(null);
  const generation = useRef(0);
  const busy = useRef(false);
  useEffect(() => {
    setError(null); setPending(false); busy.current = false;
    return () => { generation.current++; };
  }, [slug]);
  const open = async () => {
    if (busy.current || !slug) return;
    busy.current = true; setPending(true); setError(null);
    const version = generation.current;
    try { await api.openKickBrowserChat({ slug }); }
    catch (failure) { if (generation.current === version) setError(errorCode(failure)); }
    finally { if (generation.current === version) { busy.current = false; setPending(false); } }
  };
  return <div className="kick-chat"><button type="button" disabled={pending || !slug} onClick={() => { void open(); }}>{t("kick.browserChat")}</button>
    {error && <p className="error" role="alert">{errorText(error, locale)}</p>}
  </div>;
}
