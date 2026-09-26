import { useEffect } from "react";
import type { CustomTheme, TextScale, Theme, ThemePalette } from "../lib/generated";
import { themeDefinition, themeTokenNames, themeTokens } from "../styles/themeTokens";

export function useAppearance(theme: Theme, palette: ThemePalette, custom: CustomTheme | null, textScale: TextScale = "100") {
  useEffect(() => { document.documentElement.dataset.textScale = textScale; }, [textScale]);
  useEffect(() => {
    const media = window.matchMedia?.("(prefers-color-scheme: dark)");
    const apply = () => {
      const resolved = palette === "dracula" ? "dark" : theme === "system" ? (media?.matches ? "dark" : "light") : theme;
      const definition = themeDefinition(palette, custom);
      for (const name of themeTokenNames) document.documentElement.style.removeProperty(name);
      if (definition) {
        const tokens = themeTokens(definition[resolved], resolved, palette !== "custom");
        for (const name of themeTokenNames) document.documentElement.style.setProperty(name, tokens[name]);
      }
      document.documentElement.dataset.theme = theme;
      document.documentElement.dataset.themePalette = palette;
      document.documentElement.dataset.resolvedPalette = definition ? palette : "default";
      document.documentElement.dataset.resolvedTheme = resolved;
      document.documentElement.style.colorScheme = resolved;
    };
    apply();
    media?.addEventListener("change", apply);
    return () => media?.removeEventListener("change", apply);
  }, [theme, palette, custom]);
}
