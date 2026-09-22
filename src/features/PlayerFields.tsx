import type { PlayerMode, PlayerSettings } from "../lib/generated";

export function PlayerFields({ value, change }: { value: PlayerSettings; change: (value: Partial<PlayerSettings>) => void }) {
  const mode = value.mode;
  return <>
          <label>Player<select value={mode} onChange={event => change({ mode: event.target.value as PlayerMode, executable: null })}>
            <option value="default">Streamlink default</option><option value="mpv">mpv</option><option value="vlc">VLC</option><option value="custom">Custom executable</option>
          </select></label>
          {mode !== "default" && <label>Player executable{mode !== "custom" && " (optional override)"}<input aria-label="Player executable" value={value.executable ?? ""} placeholder={mode === "custom" ? "Full executable path" : "Automatic discovery"} maxLength={4096} required={mode === "custom"} onChange={event => change({ executable: event.target.value || null })} /></label>}
          <p className="muted">{mode === "default" ? "Streamlink selects its default installed player." : value.executable ? "Explicit player executable override" : "Automatic discovery of the selected player"}</p>
          <details className="player-arguments"><summary>Player arguments ({value.arguments.length})</summary>
            <p className="muted">One literal argument per row. Spaces, quotes, braces and empty arguments are preserved. No shell expansion.</p>
            {value.arguments.map((argument, index) => <div className="argument-row" key={index}>
              <label>Player argument {index + 1}<input value={argument} maxLength={4096} onChange={event => change({ arguments: value.arguments.map((value, i) => i === index ? event.target.value : value) })} /></label>
              <button type="button" aria-label={`Remove argument ${index + 1}`} onClick={() => change({ arguments: value.arguments.filter((_, i) => i !== index) })}>Remove</button>
            </div>)}
            <button type="button" disabled={value.arguments.length >= 32} onClick={() => change({ arguments: [...value.arguments, ""] })}>Add argument</button>
          </details>
  </>;
}
