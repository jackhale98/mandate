import { useGame } from "../../state/gameStore";
import { formatDate, formatMoney } from "../../i18n/format";
import { useSheetClose } from "../useSheetClose";
import { capital } from "../DecisionSheet";
import { actArgs } from "./actArgs";
import { WhyButton } from "../why/WhyButton";
import { CareerCard } from "../card/CareerCard";
import { useEffect, useRef } from "react";

/**
 * The act epilogue and its Legacy record (DEC-006, DEC-100: a record, no
 * unlocks): how the act ended, the facts the ending reports, the final
 * standing, and the record, newest first. Facts and keys from the engine.
 * Before the act is over it shows the record so far.
 */
export function Epilogue() {
  const tr = useGame((s) => s.tr);
  const l = useGame((s) => s.legacy);
  const open = useGame((s) => s.openLegacy);
  const quit = useGame((s) => s.saveAndQuit);
  const openDecision = useGame((s) => s.openDecision);
  const [closing, requestClose, sheetRef] = useSheetClose(() => void open(false));
  const card = useGame((s) => s.careerCard);
  const cardLoading = useGame((s) => s.cardLoading);
  const loadCard = useGame((s) => s.loadCard);
  const over = l?.ending != null;
  const asked = useRef(false);
  // Once the act is over, the epilogue carries the career card (M6 note 23): asked for once.
  useEffect(() => {
    if (over && !card && !cardLoading && !asked.current) {
      asked.current = true;
      void loadCard();
    }
  }, [over, card, cardLoading, loadCard]);
  if (!l) return null;
  const e = l.ending;
  return (
    <>
      <div className={closing ? "sheet-scrim sheet-closing" : "sheet-scrim"} onClick={requestClose} aria-hidden="true" />
      <aside ref={sheetRef} className={closing ? "sheet story-sheet sheet-closing" : "sheet story-sheet"} aria-labelledby="epilogue-title" data-testid="epilogue">
        <div className="sheet-head">
          <div>
            <h2 id="epilogue-title">{e ? tr.t(e.title_key) : tr.t("ui-epilogue-legacy")}</h2>
            <p className="muted small">
              {formatDate(tr, l.began)}
              {l.closed && <> – {formatDate(tr, l.closed)}</>}
            </p>
          </div>
          <button type="button" className="btn btn-ghost" onClick={requestClose} aria-label={tr.t("ui-close")}>
            ✕
          </button>
        </div>
        <div className="sheet-body">
          {e && (
            <section className={`card ending-${e.tone}`} data-testid="epilogue-ending" data-ending={e.id}>
              <h3 className="sub">{tr.t("ui-epilogue")}</h3>
              <p>{tr.t(e.text_key, actArgs(tr, e.args))}</p>
              <h4 className="sub">{tr.t("ui-epilogue-facts")}</h4>
              <dl className="facts">
                {e.facts.map((f) => (
                  <div key={f.fact}>
                    <dt>{tr.t(f.label_key)}</dt>
                    <dd className="num">{f.value}</dd>
                  </div>
                ))}
              </dl>
            </section>
          )}
          {e && <CareerCard />}
          {l.final_standing && (
            <section className="card">
              <h3 className="sub">{tr.t("ui-epilogue-final")}</h3>
              <ul className="plain-list">
                <li>
                  {tr.t("ui-skip-change-mandate")}: {l.final_standing.mandate}
                </li>
                <li>
                  {tr.t("ui-skip-change-standing")}: {l.final_standing.standing}
                </li>
                <li>
                  {tr.t("ui-skip-change-viability")}: {l.final_standing.viability}
                </li>
                <li>
                  {tr.t("ui-skip-change-money")}: {formatMoney(l.final_standing.money)}
                </li>
                {l.final_standing.post_keys.map((k) => (
                  <li key={k}>{tr.t(k)}</li>
                ))}
              </ul>
            </section>
          )}
          <section className="card">
            <h3 className="sub">{tr.t("ui-epilogue-legacy")}</h3>
            {l.entries.length === 0 ? (
              <p className="muted">{tr.t("ui-skip-nothing")}</p>
            ) : (
              <ol className="events small" data-testid="legacy-entries">
                {l.entries.map((x, i) => (
                  <li key={i}>
                    {x.date && <time className="muted">{formatDate(tr, x.date)}</time>}
                    {x.decision !== null ? (
                      <button type="button" className="link-btn" onClick={() => void openDecision(x.decision)}>
                        {capital(tr.event(x.text_key, x.args))}
                      </button>
                    ) : (
                      <span>{capital(tr.event(x.text_key, x.args))}</span>
                    )}
                    <WhyButton event={x.event} about={capital(tr.event(x.text_key, x.args))} inline />
                  </li>
                ))}
              </ol>
            )}
          </section>
          {e && (
            <button type="button" className="btn btn-primary" onClick={() => void quit()} data-testid="epilogue-new-game">
              {tr.t("ui-epilogue-new-game")}
            </button>
          )}
        </div>
      </aside>
    </>
  );
}
