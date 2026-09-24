import { useI18n } from "../i18n";
import { useEffect, useState } from "react";
import type { QualityPolicy, SessionSnapshot } from "../lib/generated";
import { Panel } from "../components/Panel";
import { useQualityLabels } from "../playback/QualitySelect";
import { playbackError } from "../playback/usePlayback";

type Props = {
  sessions: SessionSnapshot[];
  isStopping: (id: string) => boolean;
  isRestarting: (id: string) => boolean;
  stop: (id: string) => void;
  restart: (session: SessionSnapshot, quality: QualityPolicy | null) => void;
};
export function Playback(props: Props) {
  const { t } = useI18n();
  return <Panel title={t("playback.watching")}>
    <p className="muted">{t("playback.runningHelp")}</p>
    {props.sessions.length === 0 && <p>{t("playback.empty")}</p>}
    {[...props.sessions].reverse().map(session => <Session key={session.id} session={session} {...props} />)}
  </Panel>;
}
function Session({ session, isStopping, isRestarting, stop, restart }: Props & { session: SessionSnapshot }) {
  const { t, count, number, date } = useI18n();
  const qualityLabels = useQualityLabels();
  const [quality, setQuality] = useState<QualityPolicy | null>(null);
  useEffect(() => setQuality(null), [session.generation, session.qualityPolicy]);
  const active = session.restarting || ["starting", "running", "stopping"].includes(session.phase);
  const busy = isRestarting(session.id) || session.restarting;
  const name = session.stream?.displayName ?? session.url;
  const end = session.endedAt ?? Math.floor(Date.now() / 1000);
  const elapsed = Math.max(0, end - session.startedAt);
  const phase = t(({ starting: "playback.phaseStarting", running: "playback.phaseRunning", stopping: "playback.phaseStopping", exited: "playback.phaseExited", failed: "playback.phaseFailed" } as const)[session.phase]);
  return <article className="session" aria-label={t("playback.namedSession", { name })}>
    <div className="session-heading"><h3>{name}</h3><strong>{phase}</strong></div>
    <p className="session-title" title={session.stream?.title ?? undefined}>{session.stream?.title}</p>
    <p className="muted">{session.stream?.category && <>{session.stream.category} · </>}{session.qualityPolicy ? qualityLabels[session.qualityPolicy] : session.quality}{session.effectiveSettings && <> · {t("playback.lowLatency", { state: t(session.effectiveSettings.lowLatency ? "channelPreferences.on" : "channelPreferences.off") })}</>} · {t("playback.startedAt", { time: date(session.startedAt * 1000, { hour: "2-digit", minute: "2-digit" }) })} · {t("playback.duration", { minutes: number(Math.floor(elapsed / 60)), seconds: number(elapsed % 60) })}{busy && <> · {t("playback.restarting")}</>}</p>
    {session.chatError && <p className="notice">{t("playback.chatFailure")}</p>}
    {session.failure && <p className="error">{playbackError({ code: session.failure })}</p>}
    <div className="session-actions">
      <button disabled={isStopping(session.id) || !active} onClick={() => stop(session.id)}>{t("playback.stop")}</button>
      <label>{t("playback.restartQuality")}<select value={quality ?? "inherit"} onChange={event => setQuality(event.target.value === "inherit" ? null : event.target.value as QualityPolicy)} disabled={busy}>
        <option value="inherit">{t("playback.useCurrentChannelDefault")}</option>{Object.entries(qualityLabels).map(([value, label]) => <option value={value} key={value}>{label}</option>)}
      </select></label>
      <button disabled={busy || isStopping(session.id) || !session.stream} onClick={() => restart(session, quality)}>{t("playback.restart")}</button>
      <span className="muted">{t("playback.qualityRestartHelp")}</span>
    </div>
    <details className="session-diagnostics"><summary>{t("playback.diagnostics")}</summary>
      <p className="muted path">{t("playback.session")}{" "}{session.id}{" "}{t("playback.generation")}{" "}{session.generation}{" "}{t("playback.pid")}{" "}{session.pid || "—"}{" "}{t("playback.exit")}{" "}{session.exitCode ?? "—"}</p>
      <pre tabIndex={0} aria-label={t("playback.diagnosticsFor", { id: session.id })}>{session.logs.map(entry => `[${entry.source}] ${entry.text}`).join("\n") || t("playback.noDiagnostics")}</pre>
      {session.droppedLogEntries > 0 && <p className="muted">{count("playback.droppedLogs", session.droppedLogEntries)}</p>}
    </details>
  </article>;
}
