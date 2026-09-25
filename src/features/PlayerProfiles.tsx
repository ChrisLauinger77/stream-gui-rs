import { useI18n, type MessageKey } from "../i18n";
import { useEffect, useRef, useState } from "react";
import type { ErrorCode, ProfileDraft, ProfileMutation, QualityPolicy, Settings } from "../lib/generated";
import { api } from "../lib/ipc";
import { errorCode } from "../browse/errors";
import { playbackError } from "../playback/usePlayback";
import { useQualityLabels } from "../playback/QualitySelect";
import { PlayerFields } from "./PlayerFields";

type Props = { saved: Settings; saving: boolean; commit: (action: () => Promise<Settings>) => Promise<Settings> };
export function PlayerProfiles({ saved, saving, commit }: Props) {
  const { t, locale } = useI18n();
  const qualityLabels = useQualityLabels();
  const [choice, setChoice] = useState<string | null>(null);
  const [editor, setEditor] = useState<{ id: string | null; draft: ProfileDraft } | null>(null);
  const [busy, setBusy] = useState(false);
  const inFlight = useRef(false);
  const [error, setError] = useState<ErrorCode | null>(null);
  const [message, setMessage] = useState<MessageKey | null>(null);
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
      setEditor(null); setMessage(request.kind === "delete" ? "profiles.deleted" : "profiles.saved");
    } catch (error) {
      setError(errorCode(error));
    } finally { inFlight.current = false; setBusy(false); }
  };
  const edit = (change: Partial<ProfileDraft>) => setEditor(value => value && ({ ...value, draft: { ...value.draft, ...change } }));
  return <section aria-label={t("playerProfiles.playerProfiles")} className="player-profiles">
    <h3>{t("playerProfiles.profiles")}</h3>
    <label>{t("playerProfiles.profile")}<select ref={selection} value={choiceId} disabled={saving || busy || !!editor} onChange={event => setChoice(event.target.value)}>
      <option value="">{t("playerProfiles.defaultConfiguration")}</option>{saved.profiles.map(profile => <option value={profile.id} key={profile.id}>{profile.name}</option>)}
    </select></label>
    <p className="muted">{t("profiles.active", { name: active?.name ?? t("playerProfiles.defaultConfiguration") })}</p>
    <p className="muted">{t("profiles.saveHelp")}</p>
    <p className="muted">{t("profiles.precedenceHelp")}</p>
    {!editor && <div className="settings-save">
      <button type="button" disabled={saving || busy || choiceId === (saved.selectedProfileId ?? "")} onClick={() => { void mutate({ kind: "select", id: choiceId || null }); }}>{t("playerProfiles.useProfile")}</button>
      <button type="button" disabled={saving || busy || saved.profiles.length >= 16} onClick={() => { setError(null); setEditor({ id: null, draft: { name: "", player: saved.player, quality: null, lowLatency: null } }); }}>{t("playerProfiles.addProfile")}</button>
      <button type="button" disabled={saving || busy || !selected} onClick={() => { if (selected) { setError(null); setEditor({ id: selected.id, draft: { name: selected.name, player: selected.player, quality: selected.quality, lowLatency: selected.lowLatency } }); } }}>{t("playerProfiles.editProfile")}</button>
      <button type="button" disabled={saving || busy || !selected} onClick={() => { if (selected) void mutate({ kind: "delete", id: selected.id }); }}>{t("playerProfiles.deleteProfile")}</button>
    </div>}
    {editor && <fieldset disabled={busy} className="profile-editor" onKeyDown={event => {
      if (event.key === "Enter" && !event.nativeEvent.isComposing && event.target instanceof HTMLInputElement) {
        event.preventDefault();
        if (editor.draft.name.trim()) void mutate(editor.id ? { kind: "update", id: editor.id, profile: editor.draft } : { kind: "create", profile: editor.draft });
      }
    }}>
      <legend>{editor.id ? t("playerProfiles.editProfile") : t("profiles.new")}</legend>
      <label>{t("playerProfiles.profileName")}<input autoFocus value={editor.draft.name} maxLength={64} onChange={event => edit({ name: event.target.value })} /></label>
      <PlayerFields value={editor.draft.player} change={player => edit({ player: { ...editor.draft.player, ...player } })} />
      <label>{t("playerProfiles.profileQuality")}<select value={editor.draft.quality ?? "inherit"} onChange={event => edit({ quality: event.target.value === "inherit" ? null : event.target.value as QualityPolicy })}><option value="inherit">{t("playerProfiles.useGlobalQuality")}</option>{Object.entries(qualityLabels).map(([value,label]) => <option key={value} value={value}>{label}</option>)}</select></label>
      <label>{t("playerProfiles.profileLowLatency")}<select value={editor.draft.lowLatency === null ? "inherit" : editor.draft.lowLatency ? "on" : "off"} onChange={event => edit({ lowLatency: event.target.value === "inherit" ? null : event.target.value === "on" })}><option value="inherit">{t("playerProfiles.useGlobalLowLatency")}</option><option value="on">{t("playerProfiles.on")}</option><option value="off">{t("playerProfiles.off")}</option></select></label>
      <div className="settings-save"><button type="button" disabled={saving || !editor.draft.name.trim()} onClick={() => { void mutate(editor.id ? { kind: "update", id: editor.id, profile: editor.draft } : { kind: "create", profile: editor.draft }); }}>{t("playerProfiles.saveProfile")}</button><button type="button" onClick={() => { focusAfter.current = document.activeElement; setEditor(null); setError(null); }}>{t("playerProfiles.cancelProfile")}</button></div>
    </fieldset>}
    {busy && <p role="status">{t("playerProfiles.savingProfile")}</p>}
    {error && <p role="alert" className="error">{error === "settings" ? t("profiles.saveFailed") : playbackError({ code: error }, locale)}</p>}
    {message && <p role="status">{t(message)}</p>}
  </section>;
}
