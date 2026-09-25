import { useI18n, type MessageKey } from "../i18n";
import type { QualityPolicy } from "../lib/generated";
const keys: Record<QualityPolicy, MessageKey> = { source: "quality.source", high: "quality.high", medium: "quality.medium", low: "quality.low", audio: "quality.audio" };
export function useQualityLabels(): Record<QualityPolicy, string> {
  const { t } = useI18n();
  return { source: t(keys.source), high: t(keys.high), medium: t(keys.medium), low: t(keys.low), audio: t(keys.audio) };
}
export function QualitySelect({ value, change, label, disabled = false }: { value: QualityPolicy; change: (quality: QualityPolicy) => void; label: string; disabled?: boolean }) {
  const labels = useQualityLabels();
  return <label>{label}<select value={value} disabled={disabled} onChange={event => change(event.target.value as QualityPolicy)}>{Object.entries(labels).map(([key, name]) => <option key={key} value={key}>{name}</option>)}</select></label>;
}
