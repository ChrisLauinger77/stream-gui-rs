import { useEffect, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { api } from "../lib/ipc";
import type { CustomTheme } from "../lib/generated";

/** Poll only while Settings is open or Custom is selected; Rust owns file access and validation. */
export function useCustomTheme(active: boolean): CustomTheme | null {
  const [custom, setCustom] = useState<CustomTheme | null>(null);
  useEffect(() => {
    if (!active || !isTauri()) { setCustom(null); return; }
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout>;
    const refresh = async () => {
      try {
        const value = await api.customTheme();
        if (!cancelled) setCustom(current => JSON.stringify(current) === JSON.stringify(value) ? current : value);
      } catch {
        if (!cancelled) setCustom(null);
      } finally {
        if (!cancelled) timer = setTimeout(() => { void refresh(); }, 1000);
      }
    };
    void refresh();
    return () => { cancelled = true; clearTimeout(timer); };
  }, [active]);
  return custom;
}
