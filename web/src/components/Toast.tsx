import { useEffect } from "react";
import { useGame } from "../state/gameStore";

/** Result of the last player action (the engine supplies the key and args). */
export function Toast() {
  const tr = useGame((s) => s.tr);
  const toast = useGame((s) => s.toast);
  const dismiss = useGame((s) => s.dismissToast);
  useEffect(() => {
    if (!toast) return;
    const id = window.setTimeout(dismiss, 5000);
    return () => window.clearTimeout(id);
  }, [toast, dismiss]);
  if (!toast) return null;
  return (
    <div className="toast" role="status" data-testid="toast">
      <span>{tr.t(toast.key, toast.args)}</span>
      <button type="button" className="btn btn-sm btn-ghost" onClick={dismiss} aria-label={tr.t("ui-dismiss")}>
        ×
      </button>
    </div>
  );
}
