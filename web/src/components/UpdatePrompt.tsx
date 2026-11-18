import { useEffect, useState } from "react";
import { useGame } from "../state/gameStore";

// Service-worker registration with an explicit reload prompt, so an update
// never swaps code under a running turn.
export function UpdatePrompt() {
  const tr = useGame((s) => s.tr);
  const [update, setUpdate] = useState<null | (() => void)>(null);

  useEffect(() => {
    if (import.meta.env.DEV || !("serviceWorker" in navigator)) return;
    let cancelled = false;
    void import("virtual:pwa-register").then(({ registerSW }) => {
      if (cancelled) return;
      const reload = registerSW({
        onNeedRefresh() {
          setUpdate(() => () => void reload(true));
        },
      });
    });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!update) return null;
  return (
    <div className="banner banner-info" role="status">
      <span>{tr.t("ui-update-ready")}</span>
      <button type="button" className="btn" onClick={update}>
        {tr.t("ui-reload")}
      </button>
    </div>
  );
}
