import { api } from "../lib/ipc";
import type { BackgroundSettings as Preferences, MonitorPhase, NotificationPermission } from "../lib/generated";
import type { useDesktop } from "../app/useDesktop";
import { errorText } from "../browse/errors";

const phases: Record<MonitorPhase, string> = {
  disabled: "Monitoring is disabled", signed_out: "Sign in to start monitoring", paused: "Monitoring is paused",
  baseline: "Establishing a quiet baseline…", running: "Monitoring followed streams", recovering: "Waiting to recover", stopped: "Monitoring stopped",
};
export const notificationPermissionMessages: Record<NotificationPermission, string> = {
  unknown: "Checking system permission…", not_requested: "Permission has not been requested", granted: "Notifications allowed",
  denied: "Notifications denied — change this in system settings", unavailable: "Native notifications unavailable. Check the app installation and OS permissions, then retry.",
  os_managed: "Delivery is controlled by your desktop notification settings",
};
export function BackgroundSettings({ value, change, desktop }: { value: Preferences; change: (value: Partial<Preferences>) => void; desktop: ReturnType<typeof useDesktop> }) {
  const status = desktop.status;
  return <div>
    <label className="checkbox-label"><input type="checkbox" checked={value.monitoringEnabled} onChange={event => change({ monitoringEnabled: event.target.checked })} />Monitor followed live streams</label>
    <label>Check for live streams<select value={value.intervalSeconds} onChange={event => change({ intervalSeconds: Number(event.target.value) })}>
      <option value={60}>Every minute</option><option value={120}>Every 2 minutes</option><option value={300}>Every 5 minutes</option>
    </select></label>
    <label className="checkbox-label"><input type="checkbox" checked={value.notificationsEnabled} onChange={event => change({ notificationsEnabled: event.target.checked })} />Notify when followed channels go live</label>
    <p className="muted">Channel settings can override this notification default. Startup, Resume and recovery establish a quiet baseline; already-live streams do not trigger alerts.</p>
    <label className="checkbox-label"><input type="checkbox" checked={value.closeToBackground} onChange={event => change({ closeToBackground: event.target.checked })} />Keep Stream GUI RS running in the background when the window is closed</label>
    <p className="muted">Monitoring and playback continue while hidden. Restore from the tray menu. If no tray is available, closing minimizes instead. Normal minimize is unchanged. Quit stops all owned playback.</p>
    {status && <>
      <p role="status">{phases[status.monitor.phase]}{status.monitor.liveCount !== null && ` · ${status.monitor.liveCount} followed live${status.monitor.stale ? " (previous count)" : ""}`}</p>
      {status.monitor.error && <p className="muted">{errorText(status.monitor.error)} Retrying in about {status.monitor.retryInSeconds} seconds.</p>}
      <button type="button" disabled={desktop.busy || status.monitor.phase === "disabled"} onClick={() => { void desktop.run(status.monitor.paused ? api.resumeMonitor : api.pauseMonitor); }}>{status.monitor.paused ? "Resume monitoring" : "Pause monitoring"}</button>
      <p className="muted">Pause lasts until Resume or application restart.</p>
      <p>{notificationPermissionMessages[status.notificationPermission]}</p>
      {status.notificationPermission === "not_requested" && <button type="button" disabled={desktop.busy} onClick={() => { void desktop.run(api.requestNotificationPermission); }}>Allow desktop notifications</button>}
      {status.notificationPermission === "unavailable" && <button type="button" disabled={desktop.busy} onClick={() => { void desktop.run(api.requestNotificationPermission); }}>Retry desktop notifications</button>}
      {!status.notificationClickSupported && <p className="muted">This notification provider does not support channel click actions. Restore the app from its tray or taskbar.</p>}
      {!status.trayAvailable && <p className="muted">A tray is not available. Background close will minimize the window.</p>}
      {status.monitor.notificationError && <p role="status">A notification could not be delivered. Monitoring continues.</p>}
    </>}
    {desktop.error && <p className="error" role="alert">{desktop.error}</p>}
  </div>;
}
