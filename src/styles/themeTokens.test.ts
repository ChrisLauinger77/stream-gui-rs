import { expect, test } from "vitest";
import { BUNDLED_THEMES } from "./palettes";
import { contrast, themeDefinition, themeTokenNames, themeTokens } from "./themeTokens";

test("theme registry offers Sidra's eight bundled palettes and Default has no override", () => {
  expect(BUNDLED_THEMES.map(theme => theme.name)).toEqual([
    "catppuccin", "dracula", "everforest", "gruvbox", "nord", "rose-pine", "solarized", "tokyo-night",
  ]);
  expect(themeDefinition("default", null)).toBeNull();
  expect(themeDefinition("custom", null)).toBeNull();
});

test("bundled palettes keep readable semantic text, accents and focus in both modes", () => {
  for (const theme of BUNDLED_THEMES) {
    for (const mode of (theme.name === "dracula" ? ["dark"] : ["dark", "light"]) as ("dark" | "light")[]) {
      const source = theme[mode];
      expect(Object.values(source).every(value => /^#[0-9a-f]{6}$/.test(value)), `${theme.name}/${mode} source`).toBe(true);
      expect(source.overlay).not.toBe(source.subtext0);
      expect(source.crust).not.toBe(source.surface0);
      const tokens = themeTokens(source, mode, true);
      expect(Object.keys(tokens)).toEqual([...themeTokenNames]);
      for (const background of ["--bg", "--surface", "--raised", "--danger-bg"] as const) {
        for (const foreground of ["--text", "--muted", "--accent", "--live"] as const) {
          expect(contrast(tokens[foreground], tokens[background]), `${theme.name}/${mode} ${foreground} on ${background}`).toBeGreaterThanOrEqual(4.5);
        }
        expect(contrast(tokens["--focus"], tokens[background]), `${theme.name}/${mode} focus on ${background}`).toBeGreaterThanOrEqual(3);
        if (background !== "--danger-bg") expect(contrast(tokens["--line"], tokens[background]), `${theme.name}/${mode} line on ${background}`).toBeGreaterThanOrEqual(3);
      }
      expect(contrast(tokens["--accent-text"], tokens["--accent"])).toBeGreaterThanOrEqual(4.5);
    }
  }
});

test("dark-only custom colors keep error text readable when Light is selected", () => {
  const source = BUNDLED_THEMES[0].dark;
  const tokens = themeTokens(source, "light", false);
  expect([source.base, source.surface0, source.surface1]).toContain(tokens["--danger-bg"]);
  expect(contrast(tokens["--text"], tokens["--danger-bg"])).toBeGreaterThanOrEqual(4.5);
});
