import { useEffect, useState } from "react";
import type { QualityPolicy, SessionSnapshot } from "../lib/generated";
import { Panel } from "../components/Panel";
import { qualityLabels } from "../playback/QualitySelect";
import { playbackError } from "../playback/usePlayback";

type Props = {
  sessions: SessionSnapshot[];
  isStopping: (id: string) => boolean;
  isRestarting: (id: string) => boolean;
  stop: (id: string) => void;
  restart: (session: SessionSnapshot, quality: QualityPolicy | null) => void;
};
export function Playback(props: Props) {
  return <Panel title="Watching">
    <p className="muted">Running means Streamlink is active; it does not confirm video playback. Streams continue when you sign out. Closing the app stops them.</p>
    {props.sessions.length === 0 && <p>No playback sessions. Choose Watch on a live stream.</p>}
    {[...props.sessions].reverse().map(session => <Session key={session.id} session={session} {...props} />)}
  </Panel>;
}
function Session({ session, isStopping, isRestarting, stop, restart }: Props & { session: SessionSnapshot }) {
  const [quality, setQuality] = useState<QualityPolicy | null>(null);
  useEffect(() => setQuality(null), [session.generation, session.qualityPolicy]);
  const active = session.restarting || ["starting", "running", "stopping"].includes(session.phase);
  const busy = isRestarting(session.id) || session.restarting;
  const name = session.stream?.displayName ?? session.url;
  const end = session.endedAt ?? Math.floor(Date.now() / 1000);
  const elapsed = Math.max(0, end - session.startedAt);
  return <article className="session" aria-label={`Playback ${name}`}>
    <div className="session-heading"><h3>{name}</h3><strong>{session.phase}</strong></div>
    <p className="session-title" title={session.stream?.title ?? undefined}>{session.stream?.title}</p>
    <p className="muted">{session.stream?.category && `${session.stream.category} · `}{session.qualityPolicy ? qualityLabels[session.qualityPolicy] : session.quality}{session.effectiveSettings && ` · Low latency: ${session.effectiveSettings.lowLatency ? "On" : "Off"}`} · Started {new Date(session.startedAt * 1000).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })} · {Math.floor(elapsed / 60)}m {elapsed % 60}s{busy && " · Restarting…"}</p>
    {session.chatError && <p className="notice">Browser chat did not open. Playback is unaffected; you can open chat from channel details.</p>}
    {session.failure && <p className="error">{playbackError({ code: session.failure })}</p>}
    <div className="session-actions">
      <button disabled={isStopping(session.id) || !active} onClick={() => stop(session.id)}>Stop</button>
      <label>Restart quality<select value={quality ?? "inherit"} onChange={event => setQuality(event.target.value === "inherit" ? null : event.target.value as QualityPolicy)} disabled={busy}>
        <option value="inherit">Use current channel/default</option>{Object.entries(qualityLabels).map(([value, label]) => <option value={value} key={value}>{label}</option>)}
      </select></label>
      <button disabled={busy || isStopping(session.id) || !session.stream} onClick={() => restart(session, quality)}>Restart</button>
      <span className="muted">Quality changes apply when you restart.</span>
    </div>
    <details className="session-diagnostics"><summary>Diagnostics</summary>
      <p className="muted path">Session {session.id} · Generation {session.generation} · PID {session.pid || "—"} · Exit {session.exitCode ?? "—"}</p>
      <pre tabIndex={0} aria-label={`Diagnostics for session ${session.id}`}>{session.logs.map(entry => `[${entry.source}] ${entry.text}`).join("\n") || "No diagnostic output yet."}</pre>
      {session.droppedLogEntries > 0 && <p className="muted">{session.droppedLogEntries} older log entries discarded.</p>}
    </details>
  </article>;
}
