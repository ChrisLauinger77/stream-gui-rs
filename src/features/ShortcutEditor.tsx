import { useRef, useState } from "react";
import type { ShortcutAction, ShortcutBindings } from "../lib/generated";
import { useSettings } from "../settings/useSettings";
import { actionLabel, bindingError, bindingLabel, capturedBinding, defaultBindings, shortcutActions, shortcutErrors } from "../app/shortcuts";
import { friendlyError } from "../browse/errors";

export function ShortcutEditor() {
  const { settings, saveShortcuts } = useSettings();
  const [edits, setEdits] = useState<ShortcutBindings | null>(null);
  const [capture, setCapture] = useState<ShortcutAction | null>(null);
  const [feedback, setFeedback] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const busy = useRef(false);
  const draft = edits ?? settings?.shortcuts ?? defaultBindings();
  const errors = shortcutErrors(draft);
  const save = async () => {
    if (busy.current || errors.length) return;
    busy.current = true; setPending(true); setError(null);
    try { await saveShortcuts(draft); setEdits(null); setFeedback("Shortcuts saved."); }
    catch (failure) { setError(friendlyError(failure)); }
    finally { busy.current = false; setPending(false); }
  };
  return <div className="shortcut-editor">
    <p className="muted">Application-local shortcuts pause in text inputs and modal dialogs. Primary means Command on macOS and Ctrl on Windows/Linux. Changes apply after Save shortcuts.</p>
    {shortcutActions.map(action => <div className="shortcut-row" key={action}>
      <span>{actionLabel(action)}</span><kbd aria-label={`${actionLabel(action)} binding: ${bindingLabel(draft[action])}`}>{bindingLabel(draft[action])}</kbd>
      <button type="button" disabled={pending} aria-label={`Change ${actionLabel(action)} shortcut`} aria-pressed={capture === action} onClick={() => { setCapture(action); setError(null); setFeedback("Press a shortcut. Escape cancels; Tab leaves capture."); }} onBlur={() => setCapture(null)} onKeyDown={event => {
        if (capture !== action) return;
        if (event.key === "Tab") { setCapture(null); setFeedback("Capture ended."); return; }
        event.preventDefault(); event.stopPropagation();
        if (event.key === "Escape") { setCapture(null); setFeedback("Capture cancelled."); return; }
        if (["Control", "Alt", "Meta", "Shift"].includes(event.key) || event.repeat || event.nativeEvent.isComposing) return;
        const binding = capturedBinding(event); const invalid = bindingError(binding);
        if (invalid) { setError(invalid); return; }
        setEdits({ ...draft, [action]: binding }); setCapture(null); setError(null); setFeedback(`${actionLabel(action)} set to ${bindingLabel(binding)}. Save to apply.`);
      }}>{capture === action ? "Listening…" : "Change"}</button>
      <button type="button" disabled={pending || !draft[action]} aria-label={`Unassign ${actionLabel(action)} shortcut`} onClick={() => { setEdits({ ...draft, [action]: null }); setCapture(null); }}>Unassign</button>
    </div>)}
    {capture && <button type="button" onClick={() => { setCapture(null); setFeedback("Capture cancelled."); }}>Cancel capture</button>}
    <div className="settings-save"><button type="button" disabled={pending || !!capture || !!errors.length || !settings} onClick={() => { void save(); }}>Save shortcuts</button><button type="button" disabled={pending} onClick={() => { setEdits(defaultBindings()); setCapture(null); setError(null); setFeedback("Default shortcuts restored in the draft. Save to apply."); }}>Reset shortcuts to defaults</button><button type="button" disabled={pending} onClick={() => { setEdits(null); setCapture(null); setError(null); setFeedback("Shortcut changes discarded."); }}>Discard shortcut changes</button></div>
    {errors.map(message => <p role="alert" className="error" key={message}>{message}</p>)}
    {error && <p role="alert" className="error">{error}</p>}<p role="status">{pending ? "Saving shortcuts…" : feedback}</p>
  </div>;
}
