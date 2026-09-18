import { useState } from "react";
import type { SessionSnapshot } from "../lib/generated";
import { api } from "../lib/ipc";
import { Panel } from "../components/Panel";

export function Playback({ sessions, busy, ready, run }: {
  sessions: SessionSnapshot[];
  busy: boolean;
  ready: boolean;
  run: (action: () => Promise<unknown>) => void;
}) {
  const [url, setUrl] = useState("");
  const [quality, setQuality] = useState("best");
  return <Panel title="Playback prototype">
    <form onSubmit={(event) => { event.preventDefault(); run(() => api.launch({ url, quality })); }}>
      <label>Twitch channel URL<input type="url" required value={url} placeholder="https://www.twitch.tv/channel" onChange={(event) => setUrl(event.target.value)} /></label>
      <label>Quality<select value={quality} onChange={(event) => setQuality(event.target.value)}>
        {["best", "worst", "1080p60", "720p60", "720p", "480p", "audio_only"].map((value) => <option key={value}>{value}</option>)}
      </select></label>
      <button type="submit" disabled={busy || !ready}>Launch Streamlink</button>
    </form>
    <p className="muted">Streamlink opens an external player. “Running” reports the process state; it does not confirm video playback.</p>
    {sessions.length === 0 && <p>No sessions yet.</p>}
    {[...sessions].reverse().map((session) => <article key={session.id} className="session">
      <div className="session-heading"><strong>{session.phase}</strong>
        <button disabled={busy || !["running", "stopping"].includes(session.phase)} onClick={() => run(() => api.stop(session.id))}>Stop</button></div>
      <p className="path">{session.url} · {session.quality}</p>
      <p className="muted">Session {session.id} · PID {session.pid} · exit {session.exitCode ?? "—"}{session.stopRequested && " · stop requested"}</p>
      <pre aria-label={`Diagnostics for session ${session.id}`}>{session.logs.map((entry) => `[${entry.source}] ${entry.text}`).join("\n") || "Waiting for output…"}</pre>
      {session.droppedLogEntries > 0 && <p className="muted">{session.droppedLogEntries} older log entries discarded.</p>}
    </article>)}
  </Panel>;
}
