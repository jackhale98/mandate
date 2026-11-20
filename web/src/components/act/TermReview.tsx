import { useState } from "react";
import { useGame } from "../../state/gameStore";
import { formatDate } from "../../i18n/format";
import { useSheetClose } from "../useSheetClose";
import type { SkipTargetDto } from "../../engine/protocol/SkipTargetDto";

/**
 * Term in review (DEC-012): the standing directives the player-proxy follows
 * while the player is away, the points the engine offers to skip to, and
 * Skip. While a skip runs, its progress and Cancel (taken between turns).
 * Every option, target and date is the engine's; this only sends commands.
 */
export function TermReview() {
  const tr = useGame((s) => s.tr);
  const review = useGame((s) => s.termReview);
  const open = useGame((s) => s.openTermReview);
  const setDirective = useGame((s) => s.setDirective);
  const skip = useGame((s) => s.skip);
  const progress = useGame((s) => s.skipProgress);
  const cancel = useGame((s) => s.cancelSkip);
  const cancelling = useGame((s) => s.skipCancel);
  const busy = useGame((s) => s.busy);
  const [target, setTarget] = useState<SkipTargetDto | null>(null);
  const [closing, requestClose, sheetRef] = useSheetClose(() => void open(false));
  if (!review) return null;
  const chosen = target ?? review.targets[0]?.target ?? null;
  const skipping = progress !== null;
  return (
    <>
      <div className={closing ? "sheet-scrim sheet-closing" : "sheet-scrim"} onClick={skipping ? undefined : requestClose} aria-hidden="true" />
      <aside ref={sheetRef} className={closing ? "sheet story-sheet sheet-closing" : "sheet story-sheet"} aria-labelledby="term-review-title" data-testid="term-review">
        <div className="sheet-head">
          <h2 id="term-review-title">{tr.t("ui-term-review")}</h2>
          {!skipping && (
            <button type="button" className="btn btn-ghost" onClick={requestClose} aria-label={tr.t("ui-close")}>
              ✕
            </button>
          )}
        </div>
        <div className="sheet-body">
          {skipping ? (
            <section className="card" aria-live="polite" data-testid="skip-progress">
              <h3 className="sub">{tr.t("ui-skipping")}</h3>
              <div className="bar" role="progressbar" aria-valuemin={0} aria-valuemax={1000} aria-valuenow={progress.progress_pm} aria-label={tr.t("ui-skipping")}>
                <span className="bar-fill" style={{ width: `${progress.progress_pm / 10}%` }} />
              </div>
              <p className="muted small">{tr.t("ui-skipping-progress", { date: formatDate(tr, progress.date), turns: progress.turns })}</p>
              <button type="button" className="btn" onClick={cancel} disabled={cancelling} data-testid="skip-cancel">
                {tr.t("ui-skip-cancel")}
              </button>
            </section>
          ) : (
            <>
              <p className="muted small">{tr.t("ui-term-review-intro")}</p>
              <section className="card">
                <h3 className="sub">{tr.t("ui-term-review-directives")}</h3>
                {review.directives.map((d) => (
                  <label key={d.id} className="field" data-testid={`directive-${d.id}`}>
                    <span>{tr.t(d.name_key)}</span>
                    <select value={d.chosen} disabled={busy} onChange={(e) => void setDirective(d.id, e.target.value)} aria-describedby={`help-${d.id}`}>
                      {d.options.map((o) => (
                        <option key={o.id} value={o.id}>
                          {tr.t(o.name_key)}
                        </option>
                      ))}
                    </select>
                    <span id={`help-${d.id}`} className="muted tiny">
                      {tr.t(d.help_key)}
                    </span>
                  </label>
                ))}
              </section>
              <section className="card">
                <h3 className="sub">{tr.t("ui-term-review-target")}</h3>
                {review.targets.length === 0 ? (
                  <p className="muted">{tr.t("ui-term-review-none")}</p>
                ) : (
                  <fieldset className="plain-list">
                    <legend className="visually-hidden">{tr.t("ui-term-review-target")}</legend>
                    {review.targets.map((t) => (
                      <label key={t.target} className="row-radio">
                        <input type="radio" name="skip-target" value={t.target} checked={chosen === t.target} onChange={() => setTarget(t.target)} data-testid={`skip-target-${t.target}`} />
                        <span>{tr.t("ui-skip-target-date", { name: tr.t(t.name_key), date: formatDate(tr, t.date) })}</span>
                      </label>
                    ))}
                  </fieldset>
                )}
                <button type="button" className="btn btn-primary" disabled={busy || chosen === null || review.act_over} onClick={() => chosen && void skip(chosen)} data-testid="skip-go">
                  {tr.t("ui-term-review-skip")}
                </button>
              </section>
            </>
          )}
        </div>
      </aside>
    </>
  );
}
