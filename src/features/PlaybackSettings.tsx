import { useI18n, type MessageKey } from "../i18n";
import { useRef, useState } from "react";
import { api } from "../lib/ipc";
import type { ChatProvider, ErrorCode, PlayerDiscovery, ProbeResult, Settings, TextScale, Theme, ThemePalette, UiLanguage } from "../lib/generated";
import { BUNDLED_THEMES } from "../styles/palettes";
import { QualitySelect } from "../playback/QualitySelect";
import { playbackError } from "../playback/usePlayback";
import { errorCode } from "../browse/errors";
import { ShortcutEditor } from "./ShortcutEditor";

import { SavedItems } from "./DiscoveryPreferences";
import { PlayerFields } from "./PlayerFields";
import { PlayerProfiles } from "./PlayerProfiles";
import { UpdateAwareness } from "./UpdateAwareness";
import { BackgroundSettings } from "./BackgroundSettings";
import type { useDesktop } from "../app/useDesktop";
import { useSettings } from "../settings/useSettings";

const sections = ["Playback", "Streamlink", "Player", "Appearance", "Background", "Updates", "Shortcuts", "Hidden items"] as const;
type Section = typeof sections[number];
const sectionKeys = { Playback: "settings.sectionPlayback", Streamlink: "settings.sectionStreamlink", Player: "settings.sectionPlayer", Appearance: "settings.sectionAppearance", Background: "settings.sectionBackground", Updates: "settings.sectionUpdates", Shortcuts: "settings.sectionShortcuts", "Hidden items": "settings.sectionHidden" } as const;
type SettingsProps = { desktop: ReturnType<typeof useDesktop>; saved: Settings | null; saving: boolean; commit: (action: () => Promise<Settings>) => Promise<Settings>; customThemeAvailable: boolean };
type DraftEdits = Omit<Partial<Settings>, "player" | "background" | "profiles" | "selectedProfileId"> & {
  player?: Partial<Settings["player"]>;
  background?: Partial<Settings["background"]>;
};
export function PlaybackSettings(props: SettingsProps) {
  const { t } = useI18n();
  if (!props.saved) return <p role="status">{t("playbackSettings.loadingSettings")}</p>;
  return <SettingsForm {...props} saved={props.saved} />;
}
function SettingsForm({ saved, desktop, saving, commit, customThemeAvailable }: SettingsProps & { saved: Settings }) {
  const { t, locale } = useI18n();
  const { saveUiLanguage } = useSettings();
  const [languageDraft, setLanguageDraft] = useState<UiLanguage | null>(null);
  const [section, setSection] = useState<Section>("Playback");
  // Only deliberate field edits overlay the accepted snapshot. Reopened forms
  // adopt completed saves without discarding edits, even an edit back to default.
  const [edits, setEdits] = useState<DraftEdits>({});
  const draft: Settings = { ...saved, ...edits,
    player: { ...saved.player, ...edits.player }, background: { ...saved.background, ...edits.background } };
  const edit = (change: DraftEdits) => setEdits(current => ({ ...current, ...change,
    ...(change.player && { player: { ...current.player, ...change.player } }),
    ...(change.background && { background: { ...current.background, ...change.background } }),
  }));
  const [probe, setProbe] = useState<ProbeResult | null>(null);
  const [chatterino, setChatterino] = useState<string | null | undefined>(undefined);
  const [players, setPlayers] = useState<PlayerDiscovery | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const inFlight = useRef(false);
  const [error, setError] = useState<ErrorCode | null>(null);
  const [message, setMessage] = useState<MessageKey | null>(null);
  const run = async (key: string, action: () => Promise<void>) => {
    if (inFlight.current || saving) return;
    inFlight.current = true; setBusy(key); setError(null); setMessage(null);
    try { await action(); } catch (error) { setError(errorCode(error)); }
    finally { inFlight.current = false; setBusy(null); }
  };
  return <div className="settings-content">
    <nav className="settings-nav" aria-label={t("playbackSettings.settingsSections")}>{sections.map(name => <button key={name} aria-current={section === name ? "page" : undefined} onClick={() => setSection(name)}>{t(sectionKeys[name])}</button>)}</nav>
    <form noValidate className="playback-settings" aria-label={t("playbackSettings.applicationPreferences")} onSubmit={event => {
      event.preventDefault();
      void run("save", async () => {
        await commit(() => api.savePlaybackSettings(draft));
        setEdits({}); setMessage("settings.saved");
      });
    }}>
      <fieldset disabled={!!busy}>
        <legend>{t(sectionKeys[section])}</legend>
        <div hidden={section !== "Playback"} className="setting-group">
          <QualitySelect label={t("settings.defaultQuality")} value={draft.defaultQuality} change={defaultQuality => edit({ defaultQuality })} />
          <p className="muted">{t("settings.qualityHelp")}</p>
          <label className="checkbox-label"><input type="checkbox" checked={draft.lowLatency} onChange={event => edit({ lowLatency: event.target.checked })} />{t("playbackSettings.preferLowLatency")}</label>
          <p className="muted">{t("settings.lowLatencyHelp")}</p>
          <label className="checkbox-label"><input type="checkbox" checked={draft.automaticChat} onChange={event => edit({ automaticChat: event.target.checked })} />{t("settings.automaticChat")}</label>
          <p className="muted">{t("settings.chatHelp")}</p>
          <label>{t("playbackSettings.chatApplication")}<select value={draft.chatProvider} onChange={event => edit({ chatProvider: event.target.value as ChatProvider })}><option value="browser">{t("playbackSettings.browser")}</option><option value="chatterino">Chatterino</option></select></label>
          {draft.chatProvider === "chatterino" && <>
            <label>{t("settings.chatterinoPath")}<input value={draft.chatterinoPath ?? ""} maxLength={4096} placeholder={t("playbackSettings.automaticDiscovery")} onChange={event => edit({ chatterinoPath: event.target.value || null })} /></label>
            <button type="button" disabled={saving} onClick={() => { void run("chatterino", async () => setChatterino(await api.discoverChatterino())); }}>{t("playbackSettings.findChatterino")}</button>
            {chatterino !== undefined && <p className="muted path">{t("playbackSettings.chatterino")}{" "}{chatterino ?? t("settings.chatterinoMissing")}</p>}
            <p className="muted">{t("settings.chatterinoHelp")}</p>
          </>}
          <p className="muted">{t("settings.runningPreferencesHelp")}</p>
        </div>
        <div hidden={section !== "Streamlink"} className="setting-group">
          <label>{t("playbackSettings.streamlinkExecutable")}<input value={draft.streamlinkPath ?? ""} placeholder={t("playbackSettings.automaticDiscovery")} maxLength={4096} onChange={event => { edit({ streamlinkPath: event.target.value || null }); setProbe(null); }} /></label>
          <p className="muted">{draft.streamlinkPath ? t("settings.explicitExecutable") : t("settings.autoExecutable")}</p>
          <button type="button" disabled={saving} onClick={() => { void run("probe", async () => {
            setProbe(null);
            await commit(async () => {
              const result = await api.probe({ customPath: draft.streamlinkPath });
              setProbe(result); return api.playbackSettings();
            });
          }); }}>{t("settings.testStreamlink")}</button>
          <p className="muted path">{t("playbackSettings.detected")}{" "}{probe?.executable ?? t("settings.notTested")}<br />{t("playbackSettings.version")}{" "}{probe?.version ?? "—"}</p>
          <p className="muted">{t("settings.streamlinkHelp")}</p>
        </div>
        <div hidden={section !== "Player"} className="setting-group">
          <PlayerProfiles saved={saved} saving={saving || !!busy} commit={commit} />
          <h3>{t("playbackSettings.defaultConfiguration")}</h3>
          <p className="muted">{t("profiles.defaultHelp")}</p>
          <PlayerFields value={draft.player} change={player => edit({ player })} />
          <button type="button" disabled={saving} onClick={() => { void run("players", async () => setPlayers(await api.discoverPlayers())); }}>{t("playbackSettings.findInstalledPlayers")}</button>
          {players && <p className="muted path">{t("playbackSettings.mpv")}{" "}{players.mpv ?? t("settings.notFound")}<br />{t("playbackSettings.vlc")}{" "}{players.vlc ?? t("settings.notFound")}</p>}
        </div>
        <div hidden={section !== "Appearance"} className="setting-group">
          <label>{t("settings.language")}<select value={languageDraft ?? saved.uiLanguage} disabled={!!busy || saving || languageDraft !== null} onChange={event => {
            const language = event.target.value as UiLanguage;
            setLanguageDraft(language); setError(null);
            void saveUiLanguage(language).catch(failure => setError(errorCode(failure))).finally(() => setLanguageDraft(null));
          }}><option value="system">{t("settings.languageSystem")}</option><option value="en">English</option><option value="de">Deutsch</option><option value="es">Español</option><option value="fr">Français</option></select></label>
          <p className="muted">{t("settings.languageHint")}</p>
          <label>{t("playbackSettings.textSize")}<select value={draft.textScale} onChange={event => edit({ textScale: event.target.value as TextScale })}><option value="100">100%</option><option value="125">125%</option><option value="150">150%</option></select></label>
          <label>{t("settings.themeStyle")}<select value={draft.themePalette === "custom" && !customThemeAvailable ? "default" : draft.themePalette} onChange={event => edit({ themePalette: event.target.value as ThemePalette })}><option value="default">{t("settings.themeDefault")}</option>{BUNDLED_THEMES.map(theme => <option value={theme.name} key={theme.name}>{theme.label}</option>)}{customThemeAvailable && <option value="custom">{t("settings.themeCustom")}</option>}</select></label>
          {draft.themePalette === "custom" && !customThemeAvailable && <><p className="muted" role="status">{t("settings.themeUnavailable")}</p><button type="button" onClick={() => edit({ themePalette: "default" })}>{t("settings.themeUseDefault")}</button></>}
          <label>{t("settings.colorMode")}<select value={draft.themePalette === "dracula" ? "dark" : draft.theme} disabled={draft.themePalette === "dracula"} aria-describedby={draft.themePalette === "dracula" ? "dracula-mode-note" : undefined} onChange={event => edit({ theme: event.target.value as Theme })}><option value="system">{t("playbackSettings.system")}</option><option value="light">{t("playbackSettings.light")}</option><option value="dark">{t("playbackSettings.dark")}</option></select></label>
          {draft.themePalette === "dracula" && <p className="muted" id="dracula-mode-note">{t("settings.draculaDarkOnly")}</p>}
          <p className="muted">{t("settings.themeHelp")}</p>
        </div>
        <div hidden={section !== "Background"} className="setting-group">
          <BackgroundSettings value={draft.background} change={background => edit({ background })} desktop={desktop} />
        </div>
        {section === "Hidden items" && <SavedItems list="hidden" />}
        {section === "Updates" && <UpdateAwareness />}
        {section === "Shortcuts" && <ShortcutEditor />}
        {section !== "Shortcuts" && section !== "Updates" && section !== "Hidden items" && <div className="settings-save"><button type="submit" disabled={saving}>{t("playbackSettings.saveSettings")}</button><button type="button" onClick={() => { setEdits({}); setProbe(null); setError(null); setMessage("settings.discarded"); }}>{t("playbackSettings.cancelChanges")}</button><span className="muted">{JSON.stringify(draft) !== JSON.stringify(saved) ? t("settings.unsaved") : t("settings.savedPreferences")}</span></div>}
      </fieldset>
      {(busy || saving) && <p role="status">{busy === "probe" ? t("settings.testingStreamlink") : t("settings.working")}</p>}
      {error && <p className="error" role="alert">{playbackError({ code: error }, locale)}</p>}
      {message && <p role="status">{t(message)}</p>}
    </form>
  </div>;
}
