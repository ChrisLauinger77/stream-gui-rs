import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { useSettings } from "../settings/useSettings";
import { api } from "../lib/ipc";
import type { UiLanguage } from "../lib/generated";
import en from "./en.json";
import de from "./de.json";
import es from "./es.json";
import fr from "./fr.json";

export type Locale = "en" | "de" | "es" | "fr";
export type MessageKey = keyof typeof en;
type Parameters = Record<string, string | number>;
type Catalog = Record<string, string>;
const catalogs: Record<Locale, Catalog> = { en, de, es, fr };
const numbers: Record<Locale, Intl.NumberFormat> = {
  en: new Intl.NumberFormat("en"), de: new Intl.NumberFormat("de"),
  es: new Intl.NumberFormat("es"), fr: new Intl.NumberFormat("fr"),
};
const compactNumbers: Record<Locale, Intl.NumberFormat> = {
  en: new Intl.NumberFormat("en", { notation: "compact", maximumFractionDigits: 1 }),
  de: new Intl.NumberFormat("de", { notation: "compact", maximumFractionDigits: 1 }),
  es: new Intl.NumberFormat("es", { notation: "compact", maximumFractionDigits: 1 }),
  fr: new Intl.NumberFormat("fr", { notation: "compact", maximumFractionDigits: 1 }),
};
const plurals: Record<Locale, Intl.PluralRules> = {
  en: new Intl.PluralRules("en"), de: new Intl.PluralRules("de"),
  es: new Intl.PluralRules("es"), fr: new Intl.PluralRules("fr"),
};
const dates = new Map<string, Intl.DateTimeFormat>();
export function formatDate(locale: Locale, value: string | number | Date, options?: Intl.DateTimeFormatOptions) {
  const key = `${locale}:${JSON.stringify(options ?? {})}`;
  let formatter = dates.get(key);
  if (!formatter) { formatter = new Intl.DateTimeFormat(locale, options); dates.set(key, formatter); }
  return formatter.format(new Date(value));
}
let activeLocale: Locale = "en";
export const currentLocale = () => activeLocale;

export function resolveLocale(preference: UiLanguage, languages: readonly string[]): Locale {
  if (preference !== "system") return preference;
  const code = languages[0]?.trim().split(/[-_]/, 1)[0]?.toLowerCase();
  if (code === "en" || code === "de" || code === "es" || code === "fr") return code;
  return "en";
}

export function translate(locale: Locale, key: MessageKey, values: Parameters = {}): string {
  const template = message(catalogs[locale], en, key);
  return template.replace(/\{([a-zA-Z][a-zA-Z0-9]*)\}/g, (_match, name: string) =>
    Object.prototype.hasOwnProperty.call(values, name) ? String(values[name]) : `⟦${name}⟧`);
}

export function message(catalog: Catalog, source: Catalog, key: string): string {
  return catalog[key] ?? source[key] ?? `⟦${key}⟧`;
}

export function translateCount(locale: Locale, base: string, count: number, values: Parameters = {}): string {
  const form = plurals[locale].select(count);
  const key = `${base}.${form}` as MessageKey;
  const fallback = `${base}.other` as MessageKey;
  return translate(locale, Object.prototype.hasOwnProperty.call(en, key) ? key : fallback, { count: numbers[locale].format(count), ...values });
}

type Translator = {
  locale: Locale;
  t: (key: MessageKey, values?: Parameters) => string;
  count: (base: string, value: number, values?: Parameters) => string;
  number: (value: number) => string;
  compact: (value: number) => string;
  date: (value: string | number | Date, options?: Intl.DateTimeFormatOptions) => string;
};
const fallback: Translator = {
  locale: "en", t: (key, values) => translate("en", key, values),
  count: (base, value, values) => translateCount("en", base, value, values),
  number: value => numbers.en.format(value),
  compact: value => compactNumbers.en.format(value),
  date: (value, options) => formatDate("en", value, options),
};
const Context = createContext<Translator>(fallback);

export function I18nProvider({ children }: { children: ReactNode }) {
  const { settings } = useSettings();
  const browserLocale = resolveLocale("system", navigator.languages?.length ? navigator.languages : [navigator.language]);
  const [nativeLocale, setNativeLocale] = useState<Locale | null>(null);
  useEffect(() => {
    if (!isTauri()) return;
    let active = true;
    void api.systemUiLanguage().then(language => {
      if (active && language && language !== "system") setNativeLocale(language);
    }).catch(() => {});
    return () => { active = false; };
  }, []);
  const locale = settings?.uiLanguage && settings.uiLanguage !== "system"
    ? settings.uiLanguage : nativeLocale ?? browserLocale;
  activeLocale = locale;
  const value = useMemo<Translator>(() => ({
    locale,
    t: (key, values) => translate(locale, key, values),
    count: (base, count, values) => translateCount(locale, base, count, values),
    number: number => numbers[locale].format(number),
    compact: number => compactNumbers[locale].format(number),
    date: (date, options) => formatDate(locale, date, options),
  }), [locale]);
  useEffect(() => { activeLocale = locale; document.documentElement.lang = locale; return () => { activeLocale = "en"; }; }, [locale]);
  return <Context.Provider value={value}>{children}</Context.Provider>;
}

export function useI18n() { return useContext(Context); }
