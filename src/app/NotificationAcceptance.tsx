import { useI18n } from "../i18n";
import { useState } from "react";
import type { DesktopStatus, NotificationTestAction } from "../lib/generated";
import { api } from "../lib/ipc";
import type { useDesktop } from "./useDesktop";
import { permissionMessages } from "../features/BackgroundSettings";

export function isNotificationTest(status: DesktopStatus | null) {
  const action = status?.action;
  return !!status?.notificationTestAvailable && action?.kind === "channel"
    && action.authSessionId === "notification-acceptance" && action.broadcasterId === "0";
}

export function NotificationAcceptance({ desktop }: { desktop: ReturnType<typeof useDesktop> }) {
  const { t } = useI18n();
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
      setMessage(action === "send" ? t("notificationAcceptance.testQueued") : t("notificationAcceptance.cleanupRequested"));
    });
  };
  return <section className="settings-panel" aria-label={t("notificationAcceptance.nativeNotificationAcceptance")}>
    <h2>{t("notificationTest.title")}</h2>
    <p>{t("notificationTest.intro")}</p>
    <p>{t(permissionMessages[permission])}{" "}{t("notificationTest.historyHelp")}</p>
    {permission === "not_requested" && <p>{t("notificationTest.permissionHelp")}</p>}
    {permission === "unavailable" && <p>{t("notificationTest.macosHelp")}</p>}
    <div className="actions">
      {permission === "not_requested" && <button disabled={desktop.busy} onClick={requestPermission}>{t("notificationAcceptance.allowDesktopNotifications")}</button>}
      {permission === "unavailable" && <button disabled={desktop.busy} onClick={requestPermission}>{t("notificationAcceptance.retryDesktopNotifications")}</button>}
      <button disabled={desktop.busy || !canSend} onClick={() => run("send")}>{t("notificationAcceptance.sendTestNotification")}</button>
      <button disabled={desktop.busy} onClick={() => run("clear")}>{t("notificationAcceptance.clearTestNotifications")}</button>
    </div>
    {desktop.error && <p className="error" role="alert">{desktop.error}</p>}
    {message && <p role="status">{message}</p>}
  </section>;
}
