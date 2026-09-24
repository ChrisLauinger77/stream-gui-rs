import { useI18n } from "../i18n";
import { useEffect, useState } from "react";
import { Modal } from "../components/Modal";
import { api } from "../lib/ipc";
import { friendlyError } from "../browse/errors";
import type { AppInfo } from "../lib/generated";
import icon from "../assets/app-icon.png";

export function About({ activation, close }: { activation: string; close: () => void }) {
  const { t } = useI18n();
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [opening, setOpening] = useState(false);
  useEffect(() => {
    let current = true;
    void api.appInfo().then(value => { if (current) setInfo(value); })
      .catch(error => { if (current) setError(friendlyError(error)); });
    return () => { current = false; };
  }, []);
  return <Modal title={info?.name ?? "Stream GUI RS"} activation={activation} close={close} className="about-dialog">
    <img src={icon} alt="" width={80} height={80} />
    {info ? <>
      <p>{t("about.version")}{" "}{info.version} ({info.commit})</p>
      <a href={info.repository} aria-disabled={opening} onClick={event => {
        event.preventDefault(); if (opening) return;
        setOpening(true); setError(null);
        void api.openRepository().catch(error => setError(friendlyError(error))).finally(() => setOpening(false));
      }}>{t("about.githubRepository")}</a>
    </> : <p role="status">{t("about.loadingApplicationInformation")}</p>}
    {error && <p role="alert" className="error">{error}</p>}
  </Modal>;
}
