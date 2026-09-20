import { useEffect, useId, useRef, type ReactNode } from "react";

// The native dialog keeps focus inside the modal and makes the rest of the
// application inert. It remains part of the main window's existing lifecycle.
export function Modal({ title, children, close, activation = "", className = "" }: { title: string; children: ReactNode; close: () => void; activation?: string; className?: string }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const titleId = useId();
  const closeRef = useRef(close); closeRef.current = close;
  useEffect(() => {
    const element = dialog.current!;
    const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const native = typeof element.showModal === "function";
    // Older system WebKit builds lack dialog.showModal. Keep a small keyboard
    // fallback so support reporting remains usable on supported older macOS.
    const siblings = native ? [] : [...element.parentElement!.children].filter(node => node !== element).map(node => ({
      node: node as HTMLElement, hidden: node.getAttribute("aria-hidden"), pointer: (node as HTMLElement).style.pointerEvents,
    }));
    const contain = () => { if (!element.contains(document.activeElement)) heading.current?.focus(); };
    const keys = (event: KeyboardEvent) => {
      if (event.key === "Escape") { event.preventDefault(); closeRef.current(); }
      if (event.key !== "Tab") return;
      const controls = [...element.querySelectorAll<HTMLElement>('button:enabled, a[href], textarea:enabled, [tabindex="0"]')];
      const first = controls[0], last = controls.at(-1);
      if (event.shiftKey && (document.activeElement === first || document.activeElement === heading.current)) { event.preventDefault(); last?.focus(); }
      else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
    };
    if (native) element.showModal();
    else {
      element.setAttribute("open", ""); element.classList.add("dialog-fallback");
      for (const { node } of siblings) { node.setAttribute("aria-hidden", "true"); node.style.pointerEvents = "none"; }
      document.addEventListener("focusin", contain); element.addEventListener("keydown", keys);
    }
    heading.current?.focus();
    return () => {
      const inside = element.contains(document.activeElement) || document.activeElement === document.body;
      if (native) element.close();
      else {
        document.removeEventListener("focusin", contain); element.removeEventListener("keydown", keys);
        for (const { node, hidden, pointer } of siblings) {
          if (hidden === null) node.removeAttribute("aria-hidden"); else node.setAttribute("aria-hidden", hidden);
          node.style.pointerEvents = pointer;
        }
        element.removeAttribute("open");
      }
      if (inside && opener?.isConnected && !opener.matches(":disabled") && !opener.closest("[hidden]")) opener.focus({ preventScroll: true });
    };
  }, []);
  useEffect(() => { heading.current?.focus(); }, [activation]);
  return <dialog ref={dialog} className={`app-dialog ${className}`} aria-labelledby={titleId} aria-modal="true" role="dialog" onCancel={event => { event.preventDefault(); close(); }}>
    <h2 id={titleId} ref={heading} tabIndex={-1}>{title}</h2>
    {children}
    <div className="dialog-actions"><button onClick={close}>Close</button></div>
  </dialog>;
}
