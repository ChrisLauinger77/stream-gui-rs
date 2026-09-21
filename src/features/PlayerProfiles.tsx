import { useEffect, useRef, useState } from "react";
import type { ProfileDraft, ProfileMutation, QualityPolicy, Settings } from "../lib/generated";
import { api } from "../lib/ipc";
import { errorCode } from "../browse/errors";
import { playbackError } from "../playback/usePlayback";
import { qualityLabels } from "../playback/QualitySelect";
import { PlayerFields } from "./PlayerFields";

type Props = { saved: Settings; saving: boolean; commit: (action: () => Promise<Settings>) => Promise<Settings> };
export function PlayerProfiles({ saved, saving, commit }: Props) {
  const [choice, setChoice] = useState<string | null>(null);
  const [editor, setEditor] = useState<{ id: string | null; draft: ProfileDraft } | null>(null);
  const [busy, setBusy] = useState(false);
  const inFlight = useRef(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const selection = useRef<HTMLSelectElement>(null);
  const focusAfter = useRef<Element | null>(null);
  useEffect(() => {
    const previous = focusAfter.current;
    if (busy || editor || !previous) return;
    focusAfter.current = null;
    if (document.activeElement === previous || document.activeElement === document.body) selection.current?.focus();
  }, [busy, editor]);
  const choiceId = choice ?? saved.selectedProfileId ?? "";
  const selected = saved.profiles.find(profile => profile.id === choiceId);
  const active = saved.profiles.find(profile => profile.id === saved.selectedProfileId);
  const mutate = async (request: ProfileMutation) => {
    if (inFlight.current || saving) return;
    focusAfter.current = document.activeElement;
    inFlight.current = true; setBusy(true); setError(null); setMessage(null);
    try {
      const accepted = await commit(() => api.modifyPlayerProfile(request));
      if (request.kind === "create") setChoice(accepted.profiles.find(profile => !saved.profiles.some(old => old.id === profile.id))?.id ?? null);
      if (request.kind === "delete") setChoice(null);
      setEditor(null); setMessage(request.kind === "delete" ? "Profile deleted. If selected, Default configuration is now active. Running streams are unchanged." : "Profile saved. New launches and explicit restarts use the selected configuration.");
    } catch (error) {
      setError(errorCode(error) === "settings" ? "The profile could not be saved. Use a unique name of 1–64 characters (128 bytes), and at most 16 profiles. Reopen the editor if the profile was deleted." : playbackError(error));
    } finally { inFlight.current = false; setBusy(false); }
  };
  const edit = (change: Partial<ProfileDraft>) => setEditor(value => value && ({ ...value, draft: { ...value.draft, ...change } }));
  return <section aria-label="Player profiles" className="player-profiles">
    <h3>Profiles</h3>
    <label>Profile<select ref={selection} value={choiceId} disabled={saving || busy || !!editor} onChange={event => setChoice(event.target.value)}>
      <option value="">Default configuration</option>{saved.profiles.map(profile => <option value={profile.id} key={profile.id}>{profile.name}</option>)}
    </select></label>
    <p className="muted">Active profile: {active?.name ?? "Default configuration"}</p>
    <p className="muted">Profile changes save separately from the default settings.</p>
    <p className="muted">Global defaults → selected profile → channel overrides → launch quality. Running sessions keep their launch configuration; Restart uses current preferences.</p>
    {!editor && <div className="settings-save">
      <button type="button" disabled={saving || busy || choiceId === (saved.selectedProfileId ?? "")} onClick={() => { void mutate({ kind: "select", id: choiceId || null }); }}>Use profile</button>
      <button type="button" disabled={saving || busy || saved.profiles.length >= 16} onClick={() => { setError(null); setEditor({ id: null, draft: { name: "", player: saved.player, quality: null, lowLatency: null } }); }}>Add profile</button>
      <button type="button" disabled={saving || busy || !selected} onClick={() => { if (selected) { setError(null); setEditor({ id: selected.id, draft: { name: selected.name, player: selected.player, quality: selected.quality, lowLatency: selected.lowLatency } }); } }}>Edit profile</button>
      <button type="button" disabled={saving || busy || !selected} onClick={() => { if (selected) void mutate({ kind: "delete", id: selected.id }); }}>Delete profile</button>
    </div>}
    {editor && <fieldset disabled={busy} className="profile-editor" onKeyDown={event => {
      if (event.key === "Enter" && !event.nativeEvent.isComposing && event.target instanceof HTMLInputElement) {
        event.preventDefault();
        if (editor.draft.name.trim()) void mutate(editor.id ? { kind: "update", id: editor.id, profile: editor.draft } : { kind: "create", profile: editor.draft });
      }
    }}>
      <legend>{editor.id ? "Edit profile" : "New profile"}</legend>
      <label>Profile name<input autoFocus value={editor.draft.name} maxLength={64} onChange={event => edit({ name: event.target.value })} /></label>
      <PlayerFields value={editor.draft.player} change={player => edit({ player: { ...editor.draft.player, ...player } })} />
      <label>Profile quality<select value={editor.draft.quality ?? "inherit"} onChange={event => edit({ quality: event.target.value === "inherit" ? null : event.target.value as QualityPolicy })}><option value="inherit">Use global quality</option>{Object.entries(qualityLabels).map(([value,label]) => <option key={value} value={value}>{label}</option>)}</select></label>
      <label>Profile low latency<select value={editor.draft.lowLatency === null ? "inherit" : editor.draft.lowLatency ? "on" : "off"} onChange={event => edit({ lowLatency: event.target.value === "inherit" ? null : event.target.value === "on" })}><option value="inherit">Use global low latency</option><option value="on">On</option><option value="off">Off</option></select></label>
      <div className="settings-save"><button type="button" disabled={saving || !editor.draft.name.trim()} onClick={() => { void mutate(editor.id ? { kind: "update", id: editor.id, profile: editor.draft } : { kind: "create", profile: editor.draft }); }}>Save profile</button><button type="button" onClick={() => { focusAfter.current = document.activeElement; setEditor(null); setError(null); }}>Cancel profile</button></div>
    </fieldset>}
    {busy && <p role="status">Saving profile…</p>}
    {error && <p role="alert" className="error">{error}</p>}
    {message && <p role="status">{message}</p>}
  </section>;
}
