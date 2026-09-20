import { useEffect, useRef, useState } from "react";
import type { Settings, StreamLanguage } from "../lib/generated";
import { friendlyError } from "./errors";

const languages: Record<StreamLanguage, string> = {
  ar: "Arabic", bg: "Bulgarian", cs: "Czech", da: "Danish", de: "German", el: "Greek",
  en: "English", es: "Spanish", fi: "Finnish", fr: "French", hi: "Hindi", hu: "Hungarian",
  id: "Indonesian", it: "Italian", ja: "Japanese", ko: "Korean", ms: "Malay", nl: "Dutch",
  no: "Norwegian", pl: "Polish", pt: "Portuguese", ro: "Romanian", ru: "Russian", sk: "Slovak",
  sv: "Swedish", th: "Thai", tr: "Turkish", uk: "Ukrainian", vi: "Vietnamese", zh: "Chinese", other: "Other",
};
export function LanguageFilter({ value, saved, persist }: { persist: (language: StreamLanguage | null) => Promise<Settings>; value: StreamLanguage | null; saved: (settings: Settings) => void }) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const pending = useRef(false);
  const alive = useRef(true);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  const change = async (language: StreamLanguage | null) => {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError(null);
    try {
      const result = await persist(language);
      if (alive.current) saved(result);
    } catch (failure) { if (alive.current) setError(friendlyError(failure)); }
    finally { pending.current = false; if (alive.current) setBusy(false); }
  };
  return <div className="language-filter">
    <label>Stream language<select value={value ?? ""} disabled={busy} onChange={event => { void change(event.target.value ? event.target.value as StreamLanguage : null); }}>
      <option value="">Any language</option>{Object.entries(languages).map(([code, name]) => <option key={code} value={code}>{name}</option>)}
    </select></label>
    {value && <button disabled={busy} onClick={() => { void change(null); }}>Reset language</button>}
    {busy && <span role="status">Saving language…</span>}
    {error && <p className="error" role="alert">{error}</p>}
  </div>;
}
