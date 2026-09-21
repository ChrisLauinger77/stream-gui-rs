import { useEffect, useRef, useState } from "react";
import { api } from "../lib/ipc";
import type { UpdateStatus } from "../lib/generated";

export function UpdateAwareness() {
  const [status, setStatus] = useState<UpdateStatus>({ phase: "not_checked", latestVersion: null });
  const [busy, setBusy] = useState<"check" | "release" | null>(null);
  const [error, setError] = useState<string | null>(null);
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
        else setError("The update action could not be completed. Try again later.");
      }
    }
    finally { inFlight.current = false; if (mounted.current) setBusy(null); }
  };
  // Request progress is local UI state; opening a page leaves the accepted result visible.
  const message = busy === "check" ? "Checking for updates…" : {
    not_checked: "Updates have not been checked.", checking: "Checking for updates…",
    current: "Stream GUI RS is up to date", available: `Version ${status.latestVersion} is available`,
    development: `This build is newer than stable version ${status.latestVersion}.`,
    no_stable_release: "No stable release was returned.", unavailable: "Unable to check for updates",
  }[status.phase];
  return <section className="setting-group update-awareness" aria-label="Update awareness">
    <p>Checks the official GitHub repository for stable releases. Checks are manual; nothing is installed or downloaded automatically.</p>
    <p role="status">{message}</p>
    <div className="settings-save">
      <button type="button" disabled={!!busy || status.phase === "checking"} onClick={() => { void run("check", async () => { const value = await api.checkUpdates(); if (mounted.current) setStatus(value); }); }}>Check for updates</button>
      {status.phase !== "not_checked" && <button type="button" disabled={!!busy || status.phase === "checking"} onClick={() => { void run("check", async () => { const value = await api.refreshUpdates(); if (mounted.current) setStatus(value); }); }}>Refresh update check</button>}
      {status.latestVersion && <button type="button" disabled={!!busy} onClick={() => { void run("release", async () => { await api.openUpdateRelease(); }); }}>View release</button>}
    </div>
    <p className="muted">Results, including failures, are kept for 24 hours while this app runs. Explicit refresh is limited to once per minute. No background checks or notifications.</p>
    {error && <p role="alert" className="error">{error}</p>}
  </section>;
}
