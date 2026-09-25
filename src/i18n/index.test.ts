import { describe, expect, test } from "vitest";
import { message, resolveLocale, translate, translateCount } from "./index";

describe("UI language selection", () => {
  test.each([
    ["en-US", "en"], ["en-GB", "en"], ["de-DE", "de"], ["de-AT", "de"],
    ["es-ES", "es"], ["es-MX", "es"], ["fr-FR", "fr"], ["fr-CA", "fr"],
    ["ja-JP", "en"],
  ] as const)("system language %s resolves to %s", (system, expected) => {
    expect(resolveLocale("system", [system])).toBe(expected);
  });
  test("the explicit preference overrides the system and unsupported languages fall back to English", () => {
    expect(resolveLocale("fr", ["de-DE"])).toBe("fr");
    expect(resolveLocale("system", ["ja-JP", "de-DE"])).toBe("en");
    expect(resolveLocale("system", [])).toBe("en");
  });
  test("missing keys and arguments have visible deterministic fallbacks", () => {
    expect(message({}, { "test.key": "Source" }, "test.key")).toBe("Source");
    expect(message({}, {}, "test.missing")).toBe("⟦test.missing⟧");
    expect(translate("en", "settings.languageHint", { unused: 1 })).toContain("System follows");
    expect(translate("en", "native.live")).toBe("⟦name⟧ is live");
    expect(translate("en", "native.live", { name: "Alice", extra: "ignored" })).toBe("Alice is live");
    const long = "A".repeat(10_000);
    expect(message({ "test.long": long }, {}, "test.long")).toBe(long);
  });
  test("plural forms and visible counts follow the selected locale", () => {
    expect(translateCount("en", "streams.viewers", 1)).toBe("1 viewer");
    expect(translateCount("en", "streams.viewers", 2)).toBe("2 viewers");
    expect(translateCount("de", "streams.viewers", 1000)).toContain("1.000");
  });
});
