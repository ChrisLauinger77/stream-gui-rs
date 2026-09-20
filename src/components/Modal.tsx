import { useEffect, useId, useRef, type ReactNode } from "react";

// The native dialog keeps focus inside the modal and makes the rest of the
// application inert. It remains part of the main window's existing lifecycle.
export function Modal({ title, children, close, activation = "", className = "" }: { title: string; children: ReactNode; close: () => void; activation?: string; className?: string }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const titleId = useId();
  useEffect(() => {
    const element = dialog.current!;
    const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    element.showModal(); heading.current?.focus();
    return () => {
      const inside = element.contains(document.activeElement) || document.activeElement === document.body;
      element.close();
      if (inside && opener?.isConnected && !opener.matches(":disabled") && !opener.closest("[hidden]")) opener.focus({ preventScroll: true });
    };
  }, []);
  useEffect(() => { heading.current?.focus(); }, [activation]);
  return <dialog ref={dialog} className={`app-dialog ${className}`} aria-labelledby={titleId} onCancel={event => { event.preventDefault(); close(); }}>
    <h2 id={titleId} ref={heading} tabIndex={-1}>{title}</h2>
    {children}
    <div className="dialog-actions"><button onClick={close}>Close</button></div>
  </dialog>;
}
