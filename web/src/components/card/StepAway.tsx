// "Step away from politics" (M6 note 23): one confirmation, then the run
// closes, unfinished, at any time: mid-turn, in a campaign, in a scene. The
// engine closes the record (`CommandDto::StepAway`); the epilogue then opens
// with the career card.
import { useEffect, useId, useRef } from "react";
import { useGame } from "../../state/gameStore";
import "./card.css";

export function StepAway() {
  const tr = useGame((s) => s.tr);
  const open = useGame((s) => s.stepAwayOpen);
  const setOpen = useGame((s) => s.openStepAway);
  const stepAway = useGame((s) => s.stepAway);
  const busy = useGame((s) => s.busy);
  // A race is under way that the player stands in: the campaign view says so.
  const racing = useGame((s) => s.campaign?.races.some((r) => r.player_awareness !== null) ?? false);
  const id = useId();
  const keep = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (open) keep.current?.focus();
  }, [open]);
  if (!open) return null;
  return (
    <div
      className="modal-backdrop step-away-backdrop"
      onKeyDown={(e) => {
        if (e.key === "Escape") setOpen(false);
      }}
    >
      <div className="modal step-away" role="alertdialog" aria-modal="true" aria-labelledby={`${id}-title`} aria-describedby={`${id}-body`} data-testid="step-away-dialog">
        <h2 id={`${id}-title`}>{tr.t("ui-step-away-title")}</h2>
        <p id={`${id}-body`}>{tr.t("ui-step-away-body")}</p>
        {racing && <p className="hint">{tr.t("ui-step-away-race")}</p>}
        <div className="step-away-actions">
          <button ref={keep} type="button" className="btn" onClick={() => setOpen(false)} data-testid="step-away-no">
            {tr.t("ui-step-away-no")}
          </button>
          <button type="button" className="btn btn-danger" disabled={busy} onClick={() => void stepAway()} data-testid="step-away-yes">
            {tr.t("ui-step-away-yes")}
          </button>
        </div>
      </div>
    </div>
  );
}
