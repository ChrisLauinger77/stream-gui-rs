import { useState } from "react";
import type { DesktopStatus, NotificationTestAction } from "../lib/generated";
import { api } from "../lib/ipc";
import type { useDesktop } from "./useDesktop";
import { notificationPermissionMessages } from "../features/BackgroundSettings";

export function isNotificationTest(status: DesktopStatus | null) {
  const action = status?.action;
  return !!status?.notificationTestAvailable && action?.kind === "channel"
    && action.authSessionId === "notification-acceptance" && action.broadcasterId === "0";
}

export function NotificationAcceptance({ desktop }: { desktop: ReturnType<typeof useDesktop> }) {
  const [message, setMessage] = useState<string | null>(null);
  if (!desktop.status?.notificationTestAvailable) return null;
  const permission = desktop.status.notificationPermission;
  const canSend = permission === "granted" || permission === "os_managed";
  const requestPermission = () => {
    setMessage(null);
    void desktop.run(api.requestNotificationPermission);
  };
  const run = (action: NotificationTestAction) => {
    setMessage(null);
    void desktop.run(async () => {
      await api.devNotificationTest(action);
      setMessage(action === "send" ? "Test notification queued. OS delivery is not confirmed; check your desktop notification center." : "Test notification cleanup requested.");
    });
  };
  return <section className="settings-panel" aria-label="Native notification acceptance">
    <h2>Native notification acceptance · development only</h2>
    <p>Uses the native notification service. No Twitch sign-in is needed. Click the notification to open a local synthetic channel.</p>
    <p>{notificationPermissionMessages[permission]} Keep the app running while testing notification history.</p>
    {permission === "not_requested" && <p>Allow desktop notifications before sending a test.</p>}
    {permission === "unavailable" && <p>On macOS, verify the installed app bundle’s code signature. A rejected authorization request may leave the app absent from System Settings → Notifications.</p>}
    <div className="actions">
      {permission === "not_requested" && <button disabled={desktop.busy} onClick={requestPermission}>Allow desktop notifications</button>}
      {permission === "unavailable" && <button disabled={desktop.busy} onClick={requestPermission}>Retry desktop notifications</button>}
      <button disabled={desktop.busy || !canSend} onClick={() => run("send")}>Send test notification</button>
      <button disabled={desktop.busy} onClick={() => run("clear")}>Clear test notifications</button>
    </div>
    {desktop.error && <p className="error" role="alert">{desktop.error}</p>}
    {message && <p role="status">{message}</p>}
  </section>;
}
