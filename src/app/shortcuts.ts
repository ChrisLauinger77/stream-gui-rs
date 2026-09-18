import { useEffect } from "react";

export type ShortcutAction = "search" | "following" | "live" | "categories" | "watching" | "settings" | "back" | "refresh";
export const isMac = () => /Mac|iPhone|iPad/.test(navigator.platform);
export function shortcutLabels(mac = isMac()): Record<ShortcutAction, string> {
  const mod = mac ? "⌘" : "Ctrl+";
  return { search: `${mod}K`, following: `${mod}1`, live: `${mod}2`, categories: `${mod}3`, watching: `${mod}4`, settings: `${mod},`, back: mac ? "⌘[" : "Alt+←", refresh: `${mod}R` };
}
export function shortcutAction(event: KeyboardEvent, mac: boolean): ShortcutAction | null {
  if (event.defaultPrevented || event.repeat || event.isComposing || event.shiftKey) return null;
  const target = event.target;
  if (target instanceof Element && target.closest("input, textarea, select, [contenteditable]:not([contenteditable='false'])")) return null;
  if ([...document.querySelectorAll<HTMLElement>('dialog[open], [role="dialog"][aria-modal="true"]')].some(dialog => !dialog.hidden)) return null;
  if (!event.ctrlKey && !event.metaKey && event.altKey && event.key === "ArrowLeft") return "back";
  if (event.altKey || (mac ? !event.metaKey || event.ctrlKey : !event.ctrlKey || event.metaKey)) return null;
  const bindings: Partial<Record<string, ShortcutAction>> = { k: "search", "1": "following", "2": "live", "3": "categories", "4": "watching", ",": "settings", "[": "back", r: "refresh" };
  return bindings[event.key.toLowerCase()] ?? null;
}
export function useShortcuts(actions: Record<ShortcutAction, () => void>) {
  useEffect(() => {
    const handle = (event: KeyboardEvent) => {
      const action = shortcutAction(event, isMac());
      if (!action) return;
      event.preventDefault(); actions[action]();
    };
    window.addEventListener("keydown", handle);
    return () => window.removeEventListener("keydown", handle);
  }, [actions]);
}
