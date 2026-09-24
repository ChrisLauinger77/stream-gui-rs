import { test } from "node:test";
import assert from "node:assert/strict";
import { parseCatalog, validateCatalogs } from "./validate-translations.mjs";

const complete = () => Object.fromEntries(["en", "de", "es", "fr"].map(locale =>
  [locale, { "test.message": "Hello {name}", "test.count.one": "{count} item", "test.count.other": "{count} items" }]));
const usage = 't("test.message"); count("test.count", value)';

test("valid catalogs and plural references", () => assert.doesNotThrow(() => validateCatalogs(complete(), usage)));
test("malformed and duplicate resource keys", () => {
  assert.throws(() => parseCatalog('{"test.message":', "de"), /malformed JSON/);
  assert.throws(() => parseCatalog('{"test.message":"one","test.message":"two"}', "de"), /duplicate keys/);
});
test("missing, extra and unused keys", () => {
  const missing = complete(); delete missing.de["test.message"];
  assert.throws(() => validateCatalogs(missing), /missing test.message/);
  const extra = complete(); extra.fr["test.extra"] = "Extra";
  assert.throws(() => validateCatalogs(extra), /extra test.extra/);
  assert.throws(() => validateCatalogs(complete(), ""), /Unused translation keys/);
});
test("invalid placeholders, HTML and malformed values", () => {
  const mismatch = complete(); mismatch.es["test.message"] = "Hola {other}";
  assert.throws(() => validateCatalogs(mismatch), /placeholders differ/);
  const broken = complete(); broken.en["test.message"] = "Hello {name";
  assert.throws(() => validateCatalogs(broken), /invalid placeholder/);
  const markup = complete(); markup.de["test.message"] = "<b>Hello</b> {name}";
  assert.throws(() => validateCatalogs(markup), /HTML is not allowed/);
  const empty = complete(); empty.fr["test.message"] = "";
  assert.throws(() => validateCatalogs(empty), /empty\/non-text/);
});
