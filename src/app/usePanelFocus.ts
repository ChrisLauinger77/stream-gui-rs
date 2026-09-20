import { useCallback, useRef } from "react";

type Panel = "settings" | "watching";
export function usePanelFocus() {
  const origins = useRef<Partial<Record<Panel, HTMLElement | null>>>({});
  const capture = useCallback((panel: Panel, opener?: HTMLElement) => {
    origins.current[panel] = opener ?? (document.activeElement instanceof HTMLElement ? document.activeElement : null);
  }, []);
  const restore = useCallback((panel: Panel, surface: HTMLElement | null, fallback: HTMLElement | null) => {
    const active = document.activeElement;
    if (active !== document.body && active !== fallback && !surface?.contains(active)) return;
    const origin = origins.current[panel];
    const usable = (element: HTMLElement | null | undefined): element is HTMLElement => !!element?.isConnected
      && !element.matches(":disabled") && !element.closest("[hidden]") && !surface?.contains(element);
    queueMicrotask(() => {
      // A close/save result must not undo a deliberate subsequent focus move.
      if (document.activeElement !== active && document.activeElement !== document.body) return;
      (usable(origin) ? origin : usable(fallback) ? fallback : null)?.focus({ preventScroll: true });
    });
  }, []);
  return { capture, restore };
}
