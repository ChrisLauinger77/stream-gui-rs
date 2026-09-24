import { useI18n, type MessageKey } from "../i18n";
import { api } from "../lib/ipc";
import type { BackgroundSettings as Preferences, MonitorPhase, NotificationPermission } from "../lib/generated";
import type { useDesktop } from "../app/useDesktop";
import { errorText } from "../browse/errors";

const phases: Record<MonitorPhase, MessageKey> = {
  disabled: "background.phaseDisabled", signed_out: "background.phaseSignedOut", paused: "background.phasePaused",
  baseline: "background.phaseBaseline", running: "background.phaseRunning", recovering: "background.phaseRecovering", stopped: "background.phaseStopped",
};
export const permissionMessages: Record<NotificationPermission, MessageKey> = {
  unknown: "background.permissionUnknown", not_requested: "background.permissionNotRequested", granted: "background.permissionGranted",
  denied: "background.permissionDenied", unavailable: "background.permissionUnavailable", os_managed: "background.permissionOsManaged",
};
export function BackgroundSettings({ value, change, desktop }: { value: Preferences; change: (value: Partial<Preferences>) => void; desktop: ReturnType<typeof useDesktop> }) {
  const { t, count, number } = useI18n();
  const status = desktop.status;
  return <div>
    <label className="checkbox-label"><input type="checkbox" checked={value.monitoringEnabled} onChange={event => change({ monitoringEnabled: event.target.checked })} />{t("backgroundSettings.monitorFollowedLiveStreams")}</label>
    <label>{t("backgroundSettings.checkForLiveStreams")}<select value={value.intervalSeconds} onChange={event => change({ intervalSeconds: Number(event.target.value) })}>
      <option value={60}>{t("backgroundSettings.everyMinute")}</option><option value={120}>{t("backgroundSettings.every2Minutes")}</option><option value={300}>{t("backgroundSettings.every5Minutes")}</option>
    </select></label>
    <label className="checkbox-label"><input type="checkbox" checked={value.notificationsEnabled} onChange={event => change({ notificationsEnabled: event.target.checked })} />{t("background.notifyToggle")}</label>
    <p className="muted">{t("background.channelOverrideHelp")}</p>
    <label className="checkbox-label"><input type="checkbox" checked={value.closeToBackground} onChange={event => change({ closeToBackground: event.target.checked })} />{t("background.closeToggle")}</label>
    <p className="muted">{t("background.closeHelp")}</p>
    {status && <>
      <p role="status">{t(phases[status.monitor.phase])}{status.monitor.liveCount !== null && <> · {count("background.followedLive", status.monitor.liveCount)}{status.monitor.stale && ` ${t("background.previousCount")}`}</>}</p>
      {status.monitor.error && <p className="muted">{t("background.retryIn", { error: errorText(status.monitor.error), seconds: number(status.monitor.retryInSeconds) })}</p>}
      <button type="button" disabled={desktop.busy || status.monitor.phase === "disabled"} onClick={() => { void desktop.run(status.monitor.paused ? api.resumeMonitor : api.pauseMonitor); }}>{status.monitor.paused ? t("background.resumeMonitoring") : t("background.pauseMonitoring")}</button>
      <p className="muted">{t("background.pauseHelp")}</p>
      <p>{t(permissionMessages[status.notificationPermission])}</p>
      {status.notificationPermission === "not_requested" && <button type="button" disabled={desktop.busy} onClick={() => { void desktop.run(api.requestNotificationPermission); }}>{t("backgroundSettings.allowDesktopNotifications")}</button>}
      {status.notificationPermission === "unavailable" && <button type="button" disabled={desktop.busy} onClick={() => { void desktop.run(api.requestNotificationPermission); }}>{t("backgroundSettings.retryDesktopNotifications")}</button>}
      {!status.notificationClickSupported && <p className="muted">{t("background.clickUnavailable")}</p>}
      {!status.trayAvailable && <p className="muted">{t("background.trayUnavailable")}</p>}
      {status.monitor.notificationError && <p role="status">{t("background.deliveryFailed")}</p>}
    </>}
    {desktop.error && <p className="error" role="alert">{desktop.error}</p>}
  </div>;
}
