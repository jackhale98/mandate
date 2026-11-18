import { useGame } from "../state/gameStore";

export function ErrorBanner() {
  const error = useGame((s) => s.error);
  const clear = useGame((s) => s.clearError);
  const tr = useGame((s) => s.tr);
  if (!error) return null;
  return (
    <div className="banner banner-error" role="alert">
      <span>{error}</span>
      <button type="button" className="btn btn-ghost" onClick={clear}>
        {tr.t("ui-dismiss")}
      </button>
    </div>
  );
}
