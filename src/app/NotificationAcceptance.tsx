import { useState } from "react";
import type { DesktopStatus, NotificationTestAction } from "../lib/generated";
import { api } from "../lib/ipc";
import type { useDesktop } from "./useDesktop";

export function isNotificationTest(status: DesktopStatus | null) {
  const action = status?.action;
  return !!status?.notificationTestAvailable && action?.kind === "channel"
    && action.authSessionId === "notification-acceptance" && action.broadcasterId === "0";
}

export function NotificationAcceptance({ desktop }: { desktop: ReturnType<typeof useDesktop> }) {
  const [message, setMessage] = useState<string | null>(null);
  if (!desktop.status?.notificationTestAvailable) return null;
  const run = (action: NotificationTestAction) => {
    setMessage(null);
    void desktop.run(async () => {
      await api.devNotificationTest(action);
      setMessage(action === "send" ? "Test notification queued. Check your desktop notification center." : "Test notification cleanup requested.");
    });
  };
  return <section className="settings-panel" aria-label="Native notification acceptance">
    <h2>Native notification acceptance · development only</h2>
    <p>Uses the native notification service. No Twitch sign-in is needed. Click the notification to open a local synthetic channel.</p>
    <p>Permission: {desktop.status.notificationPermission}. Keep the app running while testing notification history.</p>
    <div className="actions">
      {desktop.status.notificationPermission === "not_requested" && <button disabled={desktop.busy} onClick={() => { void desktop.run(api.requestNotificationPermission); }}>Allow desktop notifications</button>}
      <button disabled={desktop.busy} onClick={() => run("send")}>Send test notification</button>
      <button disabled={desktop.busy} onClick={() => run("clear")}>Clear test notifications</button>
    </div>
    {desktop.error && <p className="error" role="alert">{desktop.error}</p>}
    {message && <p role="status">{message}</p>}
  </section>;
}
