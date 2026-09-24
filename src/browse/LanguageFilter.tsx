import { useI18n, type MessageKey } from "../i18n";
import { useEffect, useRef, useState } from "react";
import type { Settings, StreamLanguage } from "../lib/generated";
import { friendlyError } from "./errors";

const languages: Record<StreamLanguage, MessageKey> = {
  ar: "streamLanguage.ar", bg: "streamLanguage.bg", cs: "streamLanguage.cs", da: "streamLanguage.da", de: "streamLanguage.de", el: "streamLanguage.el",
  en: "streamLanguage.en", es: "streamLanguage.es", fi: "streamLanguage.fi", fr: "streamLanguage.fr", hi: "streamLanguage.hi", hu: "streamLanguage.hu",
  id: "streamLanguage.id", it: "streamLanguage.it", ja: "streamLanguage.ja", ko: "streamLanguage.ko", ms: "streamLanguage.ms", nl: "streamLanguage.nl",
  no: "streamLanguage.no", pl: "streamLanguage.pl", pt: "streamLanguage.pt", ro: "streamLanguage.ro", ru: "streamLanguage.ru", sk: "streamLanguage.sk",
  sv: "streamLanguage.sv", th: "streamLanguage.th", tr: "streamLanguage.tr", uk: "streamLanguage.uk", vi: "streamLanguage.vi", zh: "streamLanguage.zh", other: "streamLanguage.other",
};
export function LanguageFilter({ value, saved, persist }: { persist: (language: StreamLanguage | null) => Promise<Settings>; value: StreamLanguage | null; saved: (settings: Settings) => void }) {
  const { t } = useI18n();
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
    <label>{t("languageFilter.streamLanguage")}<select value={value ?? ""} disabled={busy} onChange={event => { void change(event.target.value ? event.target.value as StreamLanguage : null); }}>
      <option value="">{t("languageFilter.anyLanguage")}</option>{Object.entries(languages).map(([code, key]) => <option key={code} value={code}>{t(key)}</option>)}
    </select></label>
    {value && <button disabled={busy} onClick={() => { void change(null); }}>{t("languageFilter.resetLanguage")}</button>}
    {busy && <span role="status">{t("languageFilter.savingLanguage")}</span>}
    {error && <p className="error" role="alert">{error}</p>}
  </div>;
}
