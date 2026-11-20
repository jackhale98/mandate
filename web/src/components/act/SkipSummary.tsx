import { useGame } from "../../state/gameStore";
import { formatDate, formatMoney, formatPct, formatWhole } from "../../i18n/format";
import { useSheetClose } from "../useSheetClose";
import { capital } from "../DecisionSheet";
import type { ChangeView } from "../../engine/protocol/ChangeView";
import { actArgs } from "./actArgs";

/** A figure as the engine gave it (unemployment comes per mille). */
function figure(c: ChangeView, v: number): string {
  if (c.figure === "money") return formatMoney(v);
  if (c.figure === "unemployment") return formatPct(v / 1000, 1);
  // Counts in the locale's style, as money is (110,766, not 110766).
  return formatWhole(v);
}

/**
 * What happened while the player was away (DEC-012; the REQ-010 homecoming
 * summary): why the skip stopped, the span, the figures before and after,
 * and the events, all facts and Fluent keys from the engine.
 */
export function SkipSummary() {
  const tr = useGame((s) => s.tr);
  const s = useGame((x) => x.skipSummary);
  const close = useGame((x) => x.closeSkipSummary);
  const openDecision = useGame((x) => x.openDecision);
  const openLegacy = useGame((x) => x.openLegacy);
  const [closing, requestClose, sheetRef] = useSheetClose(close);
  if (!s) return null;
  const changed = s.changes.filter((c) => c.before !== c.after);
  return (
    <>
      <div className={closing ? "sheet-scrim sheet-closing" : "sheet-scrim"} onClick={requestClose} aria-hidden="true" />
      <aside ref={sheetRef} className={closing ? "sheet story-sheet sheet-closing" : "sheet story-sheet"} aria-labelledby="skip-summary-title" data-testid="skip-summary">
        <div className="sheet-head">
          <div>
            <h2 id="skip-summary-title">{tr.t("ui-skip-summary")}</h2>
            <p className="muted small">{tr.t("ui-skip-summary-span", { from: formatDate(tr, s.from), to: formatDate(tr, s.to), turns: s.turns })}</p>
          </div>
          <button type="button" className="btn btn-ghost" onClick={requestClose} aria-label={tr.t("ui-close")}>
            ✕
          </button>
        </div>
        <div className="sheet-body">
          <section className="card" data-testid="skip-stop" data-stop={s.stop.kind}>
            <p>
              <strong>{tr.t(s.stop.text_key, actArgs(tr, s.stop.args))}</strong>
            </p>
            {s.stop.decision !== null && (
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => {
                  close();
                  void openDecision(s.stop.decision);
                }}
              >
                {tr.t("ui-skip-open-decision")}
              </button>
            )}
            {s.stop.kind === "act_ended" && (
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => {
                  close();
                  void openLegacy(true);
                }}
              >
                {tr.t("ui-epilogue-open")}
              </button>
            )}
          </section>
          <section className="card">
            <h3 className="sub">{tr.t("ui-skip-changes")}</h3>
            {changed.length === 0 ? (
              <p className="muted">{tr.t("ui-skip-nothing")}</p>
            ) : (
              <ul className="plain-list" data-testid="skip-changes">
                {changed.map((c) => (
                  <li key={c.figure}>
                    {tr.t(c.label_key)}: {figure(c, c.before)} → {figure(c, c.after)}
                  </li>
                ))}
              </ul>
            )}
          </section>
          <section className="card">
            <h3 className="sub">{tr.t("ui-skip-happened")}</h3>
            {s.items.length === 0 ? (
              <p className="muted">{tr.t("ui-skip-nothing")}</p>
            ) : (
              <ol className="events small" data-testid="skip-items">
                {s.items.map((l, i) => (
                  <li key={i}>
                    {l.decision !== null ? (
                      <button
                        type="button"
                        className="link-btn"
                        onClick={() => {
                          close();
                          void openDecision(l.decision);
                        }}
                      >
                        {capital(tr.event(l.text_key, l.args))}
                      </button>
                    ) : (
                      <span>{capital(tr.event(l.text_key, l.args))}</span>
                    )}
                  </li>
                ))}
              </ol>
            )}
          </section>
          <button type="button" className="btn btn-primary" onClick={requestClose} data-testid="skip-summary-close">
            {tr.t("ui-skip-continue")}
          </button>
        </div>
      </aside>
    </>
  );
}
