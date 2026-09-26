import type { CustomTheme, SchemeColours, ThemePalette } from "../lib/generated";
import { bundledTheme } from "./palettes";

export const themeTokenNames = ["--bg", "--surface", "--raised", "--text", "--muted", "--line", "--accent", "--accent-text", "--live", "--focus", "--danger-bg"] as const;
type ThemeToken = typeof themeTokenNames[number];

export function themeDefinition(name: ThemePalette, custom: CustomTheme | null) {
  if (name === "default") return null;
  if (name === "custom") return custom;
  return bundledTheme(name) ?? null;
}

function rgb(hex: string): number[] {
  return [1, 3, 5].map(start => parseInt(hex.slice(start, start + 2), 16));
}
function luminance(hex: string): number {
  const channels = rgb(hex).map(channel => {
    const value = channel / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
}
export function contrast(a: string, b: string): number {
  const first = luminance(a); const second = luminance(b);
  return (Math.max(first, second) + 0.05) / (Math.min(first, second) + 0.05);
}
function mix(from: string, to: string, factor: number): string {
  const start = rgb(from); const end = rgb(to);
  return `#${start.map((value, index) => Math.round(value + (end[index] - value) * factor).toString(16).padStart(2, "0")).join("")}`;
}
function readable(color: string, backgrounds: string[], floor: number, dark: boolean): string {
  let best = color;
  let bestContrast = minimumContrast(color, backgrounds);
  if (bestContrast >= floor) return color;
  for (const target of dark ? ["#ffffff", "#000000"] : ["#000000", "#ffffff"]) {
    for (let step = 1; step <= 255; step++) {
      const candidate = mix(color, target, step / 255);
      const candidateContrast = minimumContrast(candidate, backgrounds);
      if (candidateContrast >= floor) return candidate;
      if (candidateContrast > bestContrast) { best = candidate; bestContrast = candidateContrast; }
    }
  }
  return best;
}
function minimumContrast(color: string, backgrounds: string[]): number {
  return Math.min(...backgrounds.map(background => contrast(color, background)));
}

/** Map Sidra's colour ladder to this app's semantic tokens; custom source colors remain unchanged. */
export function themeTokens(colors: SchemeColours, mode: "dark" | "light", bundled: boolean): Record<ThemeToken, string> {
  const dark = mode === "dark";
  const danger = bundled ? (dark ? "#302126" : "#fff0f0")
    : [colors.base, colors.surface0, colors.surface1].reduce((best, candidate) => contrast(colors.text, candidate) > contrast(colors.text, best) ? candidate : best);
  const backgrounds = [colors.base, colors.surface0, colors.surface1, danger];
  const text = bundled ? readable(colors.text, backgrounds, 4.5, dark) : colors.text;
  const muted = bundled ? readable(colors.subtext0, backgrounds, 4.5, dark) : colors.subtext0;
  const accent = bundled ? readable(colors.accent, backgrounds, 4.5, dark) : colors.accent;
  const liveOnDark = bundled ? dark : minimumContrast("#ffffff", backgrounds) >= minimumContrast("#000000", backgrounds);
  const live = readable(liveOnDark ? "#ff8e91" : "#b7293b", backgrounds, 4.5, liveOnDark);
  const focus = bundled ? readable(colors.accent, backgrounds, 3, dark) : colors.accent;
  const line = bundled ? readable(colors.surface2, [colors.base, colors.surface0, colors.surface1], 3, dark) : colors.surface2;
  const accentText = contrast(accent, "#ffffff") >= contrast(accent, "#000000") ? "#ffffff" : "#000000";
  return {
    "--bg": colors.base, "--surface": colors.surface0, "--raised": colors.surface1,
    "--text": text, "--muted": muted, "--line": line,
    "--accent": accent, "--accent-text": accentText, "--live": live,
    "--focus": focus, "--danger-bg": danger,
  };
}
