import { useI18n } from "../i18n";
import { useEffect, useState } from "react";
import { Modal } from "../components/Modal";
import { api } from "../lib/ipc";
import { friendlyError } from "../browse/errors";

export function SupportReport({ close }: { close: () => void }) {
  const { t } = useI18n();
  const [report, setReport] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let current = true;
    void api.supportReport().then(value => { if (current) setReport(value.text); })
      .catch(error => { if (current) setError(friendlyError(error)); });
    return () => { current = false; };
  }, []);
  return <Modal title={t("supportReport.supportReport")} close={close} className="support-report">
    <p>{t("supportReport.privacyHelp")}</p>
    <p className="muted">{t("supportReport.contents")}</p>
    {error ? <p role="alert" className="error">{error}</p> : report === null ? <p role="status">{t("supportReport.preparingReport")}</p> : <>
      <p role="status">{t("supportReport.reportReadyToReview")}</p>
      <label>{t("supportReport.reportPreview")}<textarea readOnly value={report} spellCheck={false} /></label>
    </>}
  </Modal>;
}
