import { useI18n } from "../i18n";
import type { PlayerMode, PlayerSettings } from "../lib/generated";

export function PlayerFields({ value, change }: { value: PlayerSettings; change: (value: Partial<PlayerSettings>) => void }) {
  const { t, count } = useI18n();
  const mode = value.mode;
  return <>
          <label>{t("playerFields.player")}<select value={mode} onChange={event => change({ mode: event.target.value as PlayerMode, executable: null })}>
            <option value="default">{t("playerFields.streamlinkDefault")}</option><option value="mpv">mpv</option><option value="vlc">VLC</option><option value="custom">{t("playerFields.customExecutable")}</option>
          </select></label>
          {mode !== "default" && <label>{mode === "custom" ? t("playerFields.playerExecutable") : t("player.executableOverride")}<input aria-label={t("playerFields.playerExecutable")} value={value.executable ?? ""} placeholder={mode === "custom" ? t("player.fullPath") : t("playbackSettings.automaticDiscovery")} maxLength={4096} required={mode === "custom"} onChange={event => change({ executable: event.target.value || null })} /></label>}
          <p className="muted">{mode === "default" ? t("player.streamlinkSelects") : value.executable ? t("player.explicitOverride") : t("player.automaticDiscovery")}</p>
          <details className="player-arguments"><summary>{count("player.arguments", value.arguments.length)}</summary>
            <p className="muted">{t("player.argumentsHelp")}</p>
            {value.arguments.map((argument, index) => <div className="argument-row" key={index}>
              <label>{t("player.argumentNumber", { count: index + 1 })}<input value={argument} maxLength={4096} onChange={event => change({ arguments: value.arguments.map((value, i) => i === index ? event.target.value : value) })} /></label>
              <button type="button" aria-label={t("player.removeArgument", { count: index + 1 })} onClick={() => change({ arguments: value.arguments.filter((_, i) => i !== index) })}>{t("playerFields.remove")}</button>
            </div>)}
            <button type="button" disabled={value.arguments.length >= 32} onClick={() => change({ arguments: [...value.arguments, ""] })}>{t("playerFields.addArgument")}</button>
          </details>
  </>;
}
