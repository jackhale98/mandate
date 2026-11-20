import { useGame } from "../state/gameStore";
import { useReport } from "../state/reportStore";

export function ErrorBanner() {
  const error = useGame((s) => s.error);
  const clear = useGame((s) => s.clearError);
  const tr = useGame((s) => s.tr);
  const openReport = useReport((s) => s.openReport);
  if (!error) return null;
  return (
    <div className="banner banner-error" role="alert">
      <span>{error}</span>
      <span className="banner-actions">
        {/* The error is kept for the report (state/reportStore). */}
        <button type="button" className="btn btn-ghost" onClick={() => openReport("error")} data-testid="report-from-error">
          {tr.t("ui-report")}
        </button>
        <button type="button" className="btn btn-ghost" onClick={clear}>
          {tr.t("ui-dismiss")}
        </button>
      </span>
    </div>
  );
}
