import { useI18n, type MessageKey } from "../i18n";
import { useEffect, useRef, useState } from "react";
import { api } from "../lib/ipc";
import type { UpdateStatus } from "../lib/generated";

export function UpdateAwareness() {
  const { t } = useI18n();
  const [status, setStatus] = useState<UpdateStatus>({ phase: "not_checked", latestVersion: null });
  const [busy, setBusy] = useState<"check" | "release" | null>(null);
  const [error, setError] = useState<MessageKey | null>(null);
  const mounted = useRef(false);
  const inFlight = useRef(false);
  const revision = useRef(0);
  useEffect(() => {
    mounted.current = true;
    const version = revision.current;
    void api.updateStatus().then(value => { if (mounted.current && revision.current === version) setStatus(value); }).catch(() => {});
    return () => { mounted.current = false; };
  }, []);
  useEffect(() => {
    if (status.phase !== "checking") return;
    let cancelled = false;
    const timer = setTimeout(() => { void api.updateStatus().then(value => { if (!cancelled) setStatus(value); }).catch(() => { if (!cancelled) setStatus({ phase: "unavailable", latestVersion: null }); }); }, 500);
    return () => { cancelled = true; clearTimeout(timer); };
  }, [status]);
  const run = async (kind: "check" | "release", action: () => Promise<void>) => {
    if (inFlight.current) return;
    inFlight.current = true; revision.current++; setBusy(kind); setError(null);
    try { await action(); } catch {
      if (mounted.current) {
        if (kind === "check") setStatus({ phase: "unavailable", latestVersion: null });
        else setError("updates.actionFailed");
      }
    }
    finally { inFlight.current = false; if (mounted.current) setBusy(null); }
  };
  // Request progress is local UI state; opening a page leaves the accepted result visible.
  const message = busy === "check" ? t("updates.checking") : {
    not_checked: t("updates.notChecked"), checking: t("updates.checking"),
    current: t("updates.current"), available: t("updates.available", { version: status.latestVersion ?? "" }),
    development: t("updates.development", { version: status.latestVersion ?? "" }),
    no_stable_release: t("updates.noStableRelease"), unavailable: t("updates.unavailable"),
  }[status.phase];
  return <section className="setting-group update-awareness" aria-label={t("updateAwareness.updateAwareness")}>
    <p>{t("updates.sourceHelp")}</p>
    <p role="status">{message}</p>
    <div className="settings-save">
      <button type="button" disabled={!!busy || status.phase === "checking"} onClick={() => { void run("check", async () => { const value = await api.checkUpdates(); if (mounted.current) setStatus(value); }); }}>{t("updateAwareness.checkForUpdates")}</button>
      {status.phase !== "not_checked" && <button type="button" disabled={!!busy || status.phase === "checking"} onClick={() => { void run("check", async () => { const value = await api.refreshUpdates(); if (mounted.current) setStatus(value); }); }}>{t("updateAwareness.refreshUpdateCheck")}</button>}
      {status.latestVersion && <button type="button" disabled={!!busy} onClick={() => { void run("release", async () => { await api.openUpdateRelease(); }); }}>{t("updateAwareness.viewRelease")}</button>}
    </div>
    <p className="muted">{t("updates.cacheHelp")}</p>
    {error && <p role="alert" className="error">{t(error)}</p>}
  </section>;
}
