import type { ReactNode } from "react";

export function Panel({ title, children }: { title: string; children: ReactNode }) {
  return <section><h2 tabIndex={-1}>{title}</h2>{children}</section>;
}
