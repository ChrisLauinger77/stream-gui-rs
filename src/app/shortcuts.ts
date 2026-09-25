import { useEffect } from "react";
import type { ShortcutAction, ShortcutBinding, ShortcutBindings } from "../lib/generated";
import { currentLocale, translate, type MessageKey } from "../i18n";
export type { ShortcutAction } from "../lib/generated";
export const isMac = () => /Mac|iPhone|iPad/.test(navigator.platform);
export const shortcutActions: ShortcutAction[] = ["home", "following", "live", "categories", "search", "watching", "open_channel", "bookmarks", "settings", "back", "forward", "refresh"];
const actionKeys: Record<ShortcutAction, MessageKey> = { home: "shortcuts.home", following: "shortcuts.following", live: "shortcuts.live", categories: "shortcuts.categories", search: "shortcuts.search", watching: "shortcuts.watching", open_channel: "browse.openChannel", bookmarks: "browse.bookmarks", settings: "shortcuts.settings", back: "shortcuts.back", forward: "shortcuts.forward", refresh: "shortcuts.refresh" };
export const actionLabel = (action: ShortcutAction) => translate(currentLocale(), actionKeys[action]);
export function defaultBindings(mac = isMac()): ShortcutBindings {
  const make = (key: string, primary = true, alt = false): ShortcutBinding => ({ key, primary, alt, control: false, meta: false, shift: false });
  return { home: make("Home", false, true), following: make("1"), live: make("2"), categories: make("3"), search: make("k"), watching: make("4"), open_channel: make("5"), bookmarks: make("6"), settings: make(","), back: make(mac ? "[" : "ArrowLeft", mac, !mac), forward: make(mac ? "]" : "ArrowRight", mac, !mac), refresh: make("r") };
}
export function bindingLabel(binding: ShortcutBinding | null | undefined, mac = isMac()) {
  if (!binding) return translate(currentLocale(), "shortcuts.unassigned");
  const modifiers = [binding.primary && (mac ? "⌘" : "Ctrl"), binding.control && (mac ? "Control" : "Ctrl"), binding.alt && (mac ? "Option" : "Alt"), binding.shift && "Shift", binding.meta && (mac ? "⌘" : "Meta")].filter(Boolean);
  const key = ({ ArrowLeft: "←", ArrowRight: "→" } as Record<string, string>)[binding.key] ?? (binding.key.length === 1 ? binding.key.toUpperCase() : binding.key);
  return mac && modifiers.length === 1 && modifiers[0] === "⌘" ? `⌘${key}` : [...modifiers, key].join("+");
}
export function shortcutLabels(mac = isMac(), bindings = defaultBindings(mac)): Record<ShortcutAction, string> {
  return Object.fromEntries(shortcutActions.map(action => [action, bindingLabel(bindings[action], mac)])) as Record<ShortcutAction, string>;
}
function signature(binding: ShortcutBinding, mac: boolean) {
  return [binding.key, binding.control || binding.primary && !mac, binding.meta || binding.primary && mac, binding.alt, binding.shift].join(":");
}
export function bindingError(binding: ShortcutBinding): string | null {
  if (!/^(?:[a-z0-9,\[\]]|ArrowLeft|ArrowRight|Home|F[6-9]|F1[0-2])$/.test(binding.key)
    || !(binding.primary || binding.control || binding.meta || binding.alt)
    || binding.primary && (binding.control || binding.meta) || binding.control && binding.meta
    || ["q", "w", "n", "t", "l"].includes(binding.key)) return translate(currentLocale(), "shortcuts.invalidBinding");
  return null;
}
export function shortcutErrors(bindings: ShortcutBindings): string[] {
  const errors = new Set<string>();
  for (const action of shortcutActions) {
    const binding = bindings[action];
    if (binding === undefined) errors.add(translate(currentLocale(), "shortcuts.missingBinding", { action: actionLabel(action) }));
    if (binding) { const error = bindingError(binding); if (error) errors.add(translate(currentLocale(), "shortcuts.bindingError", { action: actionLabel(action), error })); }
  }
  for (const mac of [false, true]) {
    const seen = new Map<string, ShortcutAction>();
    for (const action of shortcutActions) {
      const binding = bindings[action]; if (!binding) continue;
      const key = signature(binding, mac); const other = seen.get(key);
      if (other) errors.add(translate(currentLocale(), "shortcuts.conflict", { action: actionLabel(action), other: actionLabel(other) }));
      seen.set(key, action);
    }
  }
  return [...errors];
}
export function capturedBinding(event: Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "altKey" | "shiftKey">, mac = isMac()): ShortcutBinding {
  return { key: event.key.length === 1 ? event.key.toLowerCase() : event.key, primary: mac ? event.metaKey : event.ctrlKey, control: mac && event.ctrlKey, meta: !mac && event.metaKey, alt: event.altKey, shift: event.shiftKey };
}
export function shortcutAction(event: KeyboardEvent, mac: boolean, bindings = defaultBindings(mac)): ShortcutAction | null {
  if (event.defaultPrevented || event.repeat || event.isComposing) return null;
  const target = event.target;
  if (target instanceof Element && target.closest("input, textarea, select, [contenteditable]:not([contenteditable='false'])")) return null;
  if ([...document.querySelectorAll<HTMLElement>('dialog[open], [role="dialog"][aria-modal="true"]')].some(dialog => !dialog.hidden)) return null;
  const observed = signature({ ...capturedBinding(event, mac), primary: false, control: event.ctrlKey, meta: event.metaKey }, mac);
  return shortcutActions.find(action => bindings[action] && signature(bindings[action]!, mac) === observed) ?? null;
}
export function useShortcuts(actions: Record<ShortcutAction, () => void>, bindings?: ShortcutBindings) {
  useEffect(() => {
    const handle = (event: KeyboardEvent) => {
      const action = shortcutAction(event, isMac(), bindings);
      if (!action) return;
      event.preventDefault(); actions[action]();
    };
    window.addEventListener("keydown", handle);
    return () => window.removeEventListener("keydown", handle);
  }, [actions, bindings]);
}
