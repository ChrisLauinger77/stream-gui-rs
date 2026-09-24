import { useI18n } from "../i18n";
import type { Account, AuthStatus } from "../lib/generated";
import { api } from "../lib/ipc";
import { Panel } from "../components/Panel";
import { friendlyError } from "../browse/errors";

export function Authentication({ status, account, busy, cancelling, cancel, run }: {
  status: AuthStatus | null;
  account: Account | null;
  cancelling: boolean;
  cancel: () => void;
  busy: boolean;
  run: (action: () => Promise<unknown>) => void;
}) {
  const { t, count } = useI18n();
  const configured = !!status && status.phase !== "not_configured";
  return <Panel title="Twitch">
    <p>{t("authentication.status")}{" "}<strong>{status ? t(({ not_configured: "auth.phaseNotConfigured", restoring: "auth.phaseRestoring", signed_out: "auth.phaseSignedOut", authorizing: "auth.phaseAuthorizing", authenticated: "auth.phaseAuthenticated", cancelled: "auth.phaseCancelled", expired: "auth.phaseExpired", error: "auth.phaseError" } as const)[status.phase]) : t("auth.connecting")}</strong>
      {status?.user && <> · {status.user.login} · {count("auth.validFor", status.user.expiresIn)}</>}</p>
    {status?.phase === "not_configured" && <p>{t("authentication.configurationHelp", { variable: "TWITCH_CLIENT_ID", docs: "docs/authentication.md" })}</p>}
    <div className="actions">
      <button disabled={busy || !configured || status?.phase === "restoring" || status?.phase === "authenticated" || status?.phase === "authorizing"} onClick={() => run(api.login)}>{t("authentication.logIn")}</button>
      <button disabled={busy || !configured} onClick={() => run(api.logout)}>{t("authentication.logOut")}</button>
      <button disabled={busy || status?.phase !== "authenticated"} onClick={() => run(api.validate)}>{t("authentication.validate")}</button>
      <button disabled={busy || status?.phase !== "authenticated"} onClick={() => run(api.refresh)}>{t("authentication.refreshToken")}</button>
    </div>
    {status?.phase === "authorizing" && <button disabled={cancelling} onClick={cancel}>{t("authentication.cancelAuthorization")}</button>}
    {account && <div className="account">
      {account.profileImageUrl && <img src={account.profileImageUrl} alt="" width="48" height="48" referrerPolicy="no-referrer" />}
      <p>{account.displayName} · {account.login}{" "}{t("authentication.twitchId")}{" "}{account.id}</p>
    </div>}
    {status?.user && <p>{t("authentication.grantedScopes")}{" "}{status.user.scopes.join(", ") || t("auth.none")}</p>}
    {status?.authorization && <div className="authorization">
      <p>{t("auth.enterCode")} <strong className="user-code">{status.authorization.userCode}</strong></p><p>{count("auth.codeExpires", status.authorization.expiresIn)}</p>
      <button disabled={busy} onClick={() => run(api.openVerification)}>{t("auth.openVerification")}</button>
      <p className="path">{status.authorization.verificationUri}</p>
      <p>{t("authentication.waitingForAuthorization")}</p>
    </div>}
    {status?.error && <p className="error" role="alert">{friendlyError(status.error)}</p>}
    <p className="muted">{t("auth.storageInfo")}</p>
  </Panel>;
}
