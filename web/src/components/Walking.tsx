import { useGame } from "../state/gameStore";
import { placeIcon, placeName } from "../places/labels";

/** The short, skippable walk shown while an auto-routed action runs (DEC-079 rule 2). */
export function Walking() {
  const tr = useGame((s) => s.tr);
  const to = useGame((s) => s.walking);
  const skip = useGame((s) => s.skipWalk);
  if (!to) return null;
  return (
    <div className="walking" role="status" data-testid="walking">
      <button type="button" className="walking-card" onClick={skip} aria-label={tr.t("ui-skip-walk")}>
        <span className="walking-icon" aria-hidden="true">
          {placeIcon(to.kind_key)}
        </span>
        <span className="walking-text">{tr.t("ui-walking-to", { place: placeName(tr, to) })}</span>
        <span className="walking-track" aria-hidden="true">
          <span className="walking-dot" />
        </span>
        <span className="muted small">{tr.t("ui-tap-to-skip")}</span>
      </button>
    </div>
  );
}
