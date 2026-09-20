import { useEffect } from "react";
import type { TextScale, Theme } from "../lib/generated";

export function useAppearance(theme: Theme, textScale: TextScale = "100") {
  useEffect(() => { document.documentElement.dataset.textScale = textScale; }, [textScale]);
  useEffect(() => {
    const media = window.matchMedia?.("(prefers-color-scheme: dark)");
    const apply = () => {
      const resolved = theme === "system" ? (media?.matches ? "dark" : "light") : theme;
      document.documentElement.dataset.theme = theme;
      document.documentElement.dataset.resolvedTheme = resolved;
      document.documentElement.style.colorScheme = resolved;
    };
    apply();
    media?.addEventListener("change", apply);
    return () => media?.removeEventListener("change", apply);
  }, [theme]);
}
