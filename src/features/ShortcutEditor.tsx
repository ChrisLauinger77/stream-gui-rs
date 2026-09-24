import { useI18n } from "../i18n";
import { useRef, useState } from "react";
import type { ShortcutAction, ShortcutBindings } from "../lib/generated";
import { useSettings } from "../settings/useSettings";
import { actionLabel, bindingError, bindingLabel, capturedBinding, defaultBindings, shortcutActions, shortcutErrors } from "../app/shortcuts";
import { friendlyError } from "../browse/errors";

export function ShortcutEditor() {
  const { t } = useI18n();
  const { settings, saveShortcuts, savingShortcuts: pending } = useSettings();
  const [edits, setEdits] = useState<ShortcutBindings | null>(null);
  const [capture, setCapture] = useState<ShortcutAction | null>(null);
  const [feedback, setFeedback] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const busy = useRef(false);
  const draft = edits ?? settings?.shortcuts ?? defaultBindings();
  const errors = shortcutErrors(draft);
  const save = async () => {
    if (busy.current || errors.length) return;
    busy.current = true; setError(null);
    try { await saveShortcuts(draft); setEdits(null); setFeedback(t("shortcuts.saved")); }
    catch (failure) { setError(friendlyError(failure)); }
    finally { busy.current = false; }
  };
  return <div className="shortcut-editor">
    <p className="muted">{t("shortcuts.scopeHelp")}</p>
    {shortcutActions.map(action => <div className="shortcut-row" key={action}>
      <span>{actionLabel(action)}</span><kbd aria-label={t("shortcuts.bindingLabel", { action: actionLabel(action), binding: bindingLabel(draft[action]) })}>{bindingLabel(draft[action])}</kbd>
      <button type="button" disabled={pending} aria-label={t("shortcuts.changeLabel", { action: actionLabel(action) })} aria-pressed={capture === action} onClick={event => { event.currentTarget.focus(); setCapture(action); setError(null); setFeedback(t("shortcuts.captureHelp")); }} onBlur={() => { if (capture === action) { setCapture(null); setFeedback(t("shortcuts.captureEnded")); } }} onKeyDown={event => {
        if (capture !== action) return;
        if (event.key === "Tab") { setCapture(null); setFeedback(t("shortcuts.captureEnded")); return; }
        event.preventDefault(); event.stopPropagation();
        if (event.key === "Escape") { setCapture(null); setFeedback(t("shortcuts.captureCancelled")); return; }
        if (["Control", "Alt", "Meta", "Shift"].includes(event.key) || event.repeat || event.nativeEvent.isComposing) return;
        const binding = capturedBinding(event); const invalid = bindingError(binding);
        if (invalid) { setError(invalid); return; }
        setEdits({ ...draft, [action]: binding }); setCapture(null); setError(null); setFeedback(t("shortcuts.setTo", { action: actionLabel(action), binding: bindingLabel(binding) }));
      }}>{capture === action ? t("shortcuts.listening") : t("shortcuts.change")}</button>
      <button type="button" disabled={pending || !draft[action]} aria-label={t("shortcuts.unassignLabel", { action: actionLabel(action) })} onClick={() => { setEdits({ ...draft, [action]: null }); setCapture(null); }}>{t("shortcutEditor.unassign")}</button>
    </div>)}
    {/* Keep focus on the capture control: blur would remove Cancel before its click. */}
    {capture && <button type="button" onMouseDown={event => event.preventDefault()} onClick={() => { setCapture(null); setFeedback(t("shortcuts.captureCancelled")); }}>{t("shortcutEditor.cancelCapture")}</button>}
    <div className="settings-save"><button type="button" disabled={pending || !!capture || !!errors.length || !settings} onClick={() => { void save(); }}>{t("shortcutEditor.saveShortcuts")}</button><button type="button" disabled={pending} onClick={() => { setEdits(defaultBindings()); setCapture(null); setError(null); setFeedback(t("shortcuts.defaultsRestored")); }}>{t("shortcutEditor.resetShortcutsToDefaults")}</button><button type="button" disabled={pending} onClick={() => { setEdits(null); setCapture(null); setError(null); setFeedback(t("shortcuts.changesDiscarded")); }}>{t("shortcutEditor.discardShortcutChanges")}</button></div>
    {errors.map(message => <p role="alert" className="error" key={message}>{message}</p>)}
    {error && <p role="alert" className="error">{error}</p>}<p role="status">{pending ? t("shortcuts.saving") : feedback}</p>
  </div>;
}
