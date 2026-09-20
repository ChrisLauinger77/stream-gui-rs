import { useEffect, useState } from "react";
import { Modal } from "../components/Modal";
import { api } from "../lib/ipc";
import { friendlyError } from "../browse/errors";

export function SupportReport({ close }: { close: () => void }) {
  const [report, setReport] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let current = true;
    void api.supportReport().then(value => { if (current) setReport(value.text); })
      .catch(error => { if (current) setError(friendlyError(error)); });
    return () => { current = false; };
  }, []);
  return <Modal title="Support report" close={close} className="support-report">
    <p>Preview this local snapshot before sharing. Select the text and use your normal Copy action. Nothing is uploaded automatically.</p>
    <p className="muted">Includes build, platform, player mode and anonymous process status. Excludes identities, paths, arguments, credentials and raw logs.</p>
    {error ? <p role="alert" className="error">{error}</p> : report === null ? <p role="status">Preparing report…</p> : <>
      <p role="status">Report ready to review.</p>
      <label>Report preview<textarea readOnly value={report} spellCheck={false} /></label>
    </>}
  </Modal>;
}
