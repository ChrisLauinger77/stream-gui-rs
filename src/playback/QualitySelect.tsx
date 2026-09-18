import type { QualityPolicy } from "../lib/generated";
export const qualityLabels: Record<QualityPolicy, string> = { source: "Source", high: "High · 720p30", medium: "Medium · 540p30", low: "Low · 360p30", audio: "Audio" };
export function QualitySelect({ value, change, label, disabled = false }: { value: QualityPolicy; change: (quality: QualityPolicy) => void; label: string; disabled?: boolean }) {
  return <label>{label}<select value={value} disabled={disabled} onChange={event => change(event.target.value as QualityPolicy)}>{Object.entries(qualityLabels).map(([key, name]) => <option key={key} value={key}>{name}</option>)}</select></label>;
}
