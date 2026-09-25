import { readFileSync, readdirSync, statSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const locales = ["en", "de", "es", "fr"];
const root = dirname(fileURLToPath(import.meta.url));
const catalogRoot = join(root, "../src/i18n");

export function parseCatalog(source, locale) {
  const keys = [...source.matchAll(/"([^"\\]+)"\s*:/g)].map(match => match[1]);
  if (new Set(keys).size !== keys.length) throw new Error(`${locale}: duplicate keys`);
  let catalog;
  try { catalog = JSON.parse(source); }
  catch { throw new Error(`${locale}: malformed JSON`); }
  if (catalog === null || Array.isArray(catalog) || typeof catalog !== "object") {
    throw new Error(`${locale}: expected an object`);
  }
  return catalog;
}

function parameters(value, locale, key) {
  if (typeof value !== "string" || !value.trim()) throw new Error(`${locale}: empty/non-text value for ${key}`);
  if (/<\/?[a-z][^>]*>/i.test(value)) throw new Error(`${locale}: HTML is not allowed in ${key}`);
  const remaining = value.replace(/\{[a-zA-Z][a-zA-Z0-9]*\}/g, "");
  if (remaining.includes("{") || remaining.includes("}")) throw new Error(`${locale}: invalid placeholder in ${key}`);
  return [...value.matchAll(/\{([a-zA-Z][a-zA-Z0-9]*)\}/g)].map(match => match[1]).sort().join(",");
}

export function validateCatalogs(catalogs, usage = null) {
  const english = catalogs.en;
  if (!english || typeof english !== "object") throw new Error("Missing English catalog");
  for (const [locale, catalog] of Object.entries(catalogs)) {
    for (const [key, value] of Object.entries(catalog)) {
      if (!/^[a-z][a-zA-Z0-9]*(?:\.[a-z][a-zA-Z0-9]*)+$/.test(key)) throw new Error(`${locale}: invalid key ${key}`);
      parameters(value, locale, key);
    }
  }
  for (const locale of locales.slice(1)) {
    const current = catalogs[locale];
    if (!current) throw new Error(`${locale}: missing catalog`);
    const missing = Object.keys(english).filter(key => !(key in current));
    const extra = Object.keys(current).filter(key => !(key in english));
    if (missing.length || extra.length) throw new Error(`${locale}: missing ${missing.join(", ") || "none"}; extra ${extra.join(", ") || "none"}`);
    for (const key of Object.keys(english)) {
      if (parameters(current[key], locale, key) !== parameters(english[key], "en", key)) {
        throw new Error(`${locale}: placeholders differ for ${key}`);
      }
    }
  }
  if (usage !== null) {
    const orphans = Object.keys(english).filter(key => {
      const direct = usage.includes(`"${key}"`) || usage.includes(`'${key}'`);
      const base = key.endsWith(".one") || key.endsWith(".other") ? key.slice(0, key.lastIndexOf(".")) : null;
      return !direct && (!base || !usage.includes(`"${base}"`));
    });
    if (orphans.length) throw new Error(`Unused translation keys: ${orphans.join(", ")}`);
  }
}

function sourceFiles(directory) {
  return readdirSync(directory).flatMap(name => {
    const path = join(directory, name);
    if (statSync(path).isDirectory()) return sourceFiles(path);
    return /\.(rs|ts|tsx)$/.test(name) && !name.includes(".test.") && name !== "generated.ts" ? [path] : [];
  });
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const catalogs = Object.fromEntries(locales.map(locale => [locale, parseCatalog(readFileSync(join(catalogRoot, `${locale}.json`), "utf8"), locale)]));
  const usage = [...sourceFiles(join(root, "../src")), ...sourceFiles(join(root, "../src-tauri/src"))]
    .map(path => readFileSync(path, "utf8")).join("\n");
  validateCatalogs(catalogs, usage);
  console.log(`Translation catalogs valid: ${Object.keys(catalogs.en).length} keys in ${locales.join(", ")}`);
}
