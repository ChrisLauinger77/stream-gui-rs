import { useEffect, useRef, useState } from "react";
import { api } from "../lib/ipc";
import type { ErrorCode } from "../lib/generated";
import { errorCode, errorText } from "../browse/errors";
import { useI18n, type MessageKey } from "../i18n";

// Only an 11-character video ID names a YouTube chat; Rust validates again.
export const youtubeVideoId = (locator: string) => /^[A-Za-z0-9_-]{11}$/.test(locator);

export function KickChatButton({ slug }: { slug: string }) {
  return <BrowserChatButton target={slug} label="kick.browserChat" open={() => api.openKickBrowserChat({ slug })} />;
}

export function YoutubeChatButton({ locator }: { locator: string }) {
  return <BrowserChatButton target={youtubeVideoId(locator) ? locator : ""} label="youtube.browserChat" open={() => api.openYoutubeBrowserChat({ locator })} />;
}

function BrowserChatButton({ target, label, open }: { target: string; label: MessageKey; open: () => Promise<null> }) {
  const { t, locale } = useI18n();
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<ErrorCode | null>(null);
  const generation = useRef(0);
  const busy = useRef(false);
  useEffect(() => {
    setError(null); setPending(false); busy.current = false;
    return () => { generation.current++; };
  }, [target]);
  const run = async () => {
    if (busy.current || !target) return;
    busy.current = true; setPending(true); setError(null);
    const version = generation.current;
    try { await open(); }
    catch (failure) { if (generation.current === version) setError(errorCode(failure)); }
    finally { if (generation.current === version) { busy.current = false; setPending(false); } }
  };
  return <div className="kick-chat"><button type="button" disabled={pending || !target} onClick={() => { void run(); }}>{t(label)}</button>
    {error && <p className="error" role="alert">{errorText(error, locale)}</p>}
  </div>;
}
