import { useEffect, useState } from "react";
import { useGame } from "../state/gameStore";

// Service-worker registration with an explicit reload prompt, so an update
// never swaps code under a running turn. This needs `registerType: "prompt"`
// in vite.config.ts: under "autoUpdate" the plugin reloads the page by itself
// and `onNeedRefresh` is never called.
//
// The bar sits above every sheet (styles.css `.banner-info`), so the pocket or a decision never hides it.
//
// A browser checks for a new worker only when the page loads, so an installed
// app that stays open (or sits in the background) would run an old build for
// days. The registration is asked to check every hour and whenever the app
// comes back to the foreground.
const CHECK_EVERY_MS = 60 * 60 * 1000;

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
          setUpdate(() => () => {
            // The plugin reloads when the new worker takes over only for a page that already had a worker
            // when it loaded (`isUpdate`): one that installed the first worker itself would swap the code
            // silently under the running page and never reload. Reload on the takeover here, whichever it was.
            navigator.serviceWorker.addEventListener("controllerchange", () => window.location.reload(), { once: true });
            void reload(true);
          });
        },
        onRegisteredSW(_url, registration) {
          if (!registration) return;
          const check = () => {
            if (navigator.onLine) void registration.update().catch(() => undefined);
          };
          window.setInterval(check, CHECK_EVERY_MS);
          document.addEventListener("visibilitychange", () => {
            if (document.visibilityState === "visible") check();
          });
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
