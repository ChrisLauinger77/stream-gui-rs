import type { Account, AuthStatus } from "../lib/generated";
import { api } from "../lib/ipc";
import { Panel } from "../components/Panel";

export function Authentication({ status, account, busy, cancelling, cancel, run }: {
  status: AuthStatus | null;
  account: Account | null;
  cancelling: boolean;
  cancel: () => void;
  busy: boolean;
  run: (action: () => Promise<unknown>) => void;
}) {
  const configured = !!status && status.phase !== "not_configured";
  return <Panel title="Twitch">
    <p>Status: <strong>{status?.phase.replaceAll("_", " ") ?? "Connecting…"}</strong>
      {status?.user && <> · {status.user.login} · valid for {status.user.expiresIn}s</>}</p>
    {status?.phase === "not_configured" && <p>Register a new public Twitch application, set <code>TWITCH_CLIENT_ID</code> in the backend environment, and restart. See <code>docs/authentication.md</code>.</p>}
    <div className="actions">
      <button disabled={busy || !configured || status?.phase === "restoring" || status?.phase === "authenticated" || status?.phase === "authorizing"} onClick={() => run(api.login)}>Log in</button>
      <button disabled={busy || !configured} onClick={() => run(api.logout)}>Log out</button>
      <button disabled={busy || status?.phase !== "authenticated"} onClick={() => run(api.validate)}>Validate</button>
      <button disabled={busy || status?.phase !== "authenticated"} onClick={() => run(api.refresh)}>Refresh token</button>
    </div>
    {status?.phase === "authorizing" && <button disabled={cancelling} onClick={cancel}>Cancel authorization</button>}
    {account && <div className="account">
      {account.profileImageUrl && <img src={account.profileImageUrl} alt="" width="48" height="48" referrerPolicy="no-referrer" />}
      <p>{account.displayName} · {account.login} · Twitch ID {account.id}</p>
    </div>}
    {status?.user && <p>Granted scopes: {status.user.scopes.join(", ") || "none"}</p>}
    {status?.authorization && <div className="authorization">
      <p>Enter <strong className="user-code">{status.authorization.userCode}</strong> at Twitch (expires in {status.authorization.expiresIn}s).</p>
      <button disabled={busy} onClick={() => run(api.openVerification)}>Open Twitch in system browser</button>
      <p className="path">{status.authorization.verificationUri}</p>
      <p>Waiting for authorization…</p>
    </div>}
    {status?.error && <p className="error" role="status">{status.error.message}</p>}
    <p className="muted">Credential storage: {status?.credentialStorage ?? "Rust only"}. Credentials stay in Rust and the OS credential store.</p>
  </Panel>;
}
