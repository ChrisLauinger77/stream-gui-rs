import { useI18n } from "../i18n";
import { useEffect, useState } from "react";
import { Modal } from "../components/Modal";
import { api } from "../lib/ipc";
import { errorCode, errorText } from "../browse/errors";
import type { ErrorCode } from "../lib/generated";

export function SupportReport({ close }: { close: () => void }) {
  const { t, locale } = useI18n();
  const [report, setReport] = useState<string | null>(null);
  const [error, setError] = useState<ErrorCode | null>(null);
  useEffect(() => {
    let current = true;
    void api.supportReport().then(value => { if (current) setReport(value.text); })
      .catch(error => { if (current) setError(errorCode(error)); });
    return () => { current = false; };
  }, []);
  return <Modal title={t("supportReport.supportReport")} close={close} className="support-report">
    <p>{t("supportReport.privacyHelp")}</p>
    <p className="muted">{t("supportReport.contents")}</p>
    {error ? <p role="alert" className="error">{errorText(error, locale)}</p> : report === null ? <p role="status">{t("supportReport.preparingReport")}</p> : <>
      <p role="status">{t("supportReport.reportReadyToReview")}</p>
      <label>{t("supportReport.reportPreview")}<textarea readOnly value={report} spellCheck={false} /></label>
    </>}
  </Modal>;
}
