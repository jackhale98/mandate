import { useEffect, useMemo, useRef } from "react";
import { useGame } from "../../state/gameStore";
import { useSheetClose } from "../useSheetClose";
import { WhyBars } from "../ElectionCharts";
import type { FactorView } from "../../engine/protocol/FactorView";
import { layoutWhy, type RootKind } from "./whyModel";
import { useWhy, type WhyFactors } from "./whyStore";
import "./why.css";

/** A mark before each kind of start (decoration: the line says what it is). */
const ROOT_MARK: Record<RootKind, string> = {
  player: "⚑",
  actor: "☺",
  process: "⚙",
  pressure: "⚖",
  exogenous: "☁",
  history: "⌛",
  forgotten: "…",
  unknown: "?",
};

/**
 * "Why?" (the causal spine, DEC-108): what led to an outcome, as the engine
 * recorded it. The event asked about is at the top; under it each earlier
 * step in one sentence with its date, joined by how it led on, and under a
 * step what it names as a start (the player's own act, someone's choice, a
 * standing rule, a pressure, an outside shock). A line at the top says when
 * something the player did is behind it. For a race the engine's factor
 * decomposition is shown beside the chain, as the result gave it. The panel
 * lays out the engine's answer and computes nothing.
 */
export function WhyPanel() {
  const tr = useGame((s) => s.tr);
  const w = useWhy((s) => s.open);
  const close = useWhy((s) => s.close);
  const further = useWhy((s) => s.further);
  const [closing, requestClose, sheetRef] = useSheetClose(close);
  const title = useRef<HTMLHeadingElement>(null);
  const event = w?.event ?? null;
  // The keyboard goes to the panel when it opens (and comes back to the control on close); nothing holds it there.
  useEffect(() => {
    if (event !== null) title.current?.focus({ preventScroll: true });
  }, [event]);
  // The panel goes with the game screen.
  useEffect(() => () => useWhy.setState({ open: null }), []);
  const view = w?.view ?? null;
  const layout = useMemo(() => (view ? layoutWhy(tr, view) : null), [tr, view]);
  if (!w) return null;
  return (
    <>
      <div className={closing ? "sheet-scrim why-scrim sheet-closing" : "sheet-scrim why-scrim"} onClick={requestClose} aria-hidden="true" />
      <aside ref={sheetRef} className={closing ? "sheet why-sheet sheet-closing" : "sheet why-sheet"} aria-labelledby="why-title" data-testid="why-panel" data-event={w.event}>
        <div className="sheet-head">
          <h2 id="why-title" ref={title} tabIndex={-1} className="why-title">
            {tr.t("ui-why-title")}
          </h2>
          <button type="button" className="btn btn-ghost" onClick={requestClose} aria-label={tr.t("ui-close")}>
            ✕
          </button>
        </div>
        <div className="sheet-body" aria-busy={w.busy}>
          {!layout && w.busy && (
            <p className="muted" role="status">
              {tr.t("ui-why-loading")}
            </p>
          )}
          {w.gone && (
            <p className="note" role="status" data-testid="why-gone">
              {tr.t("ui-why-gone")}
            </p>
          )}
          {layout?.player && (
            <p className="why-player" data-testid="why-player">
              <strong>{tr.t("ui-why-player")}</strong> {layout.player.text}
              {layout.player.date && <span className="muted small"> · {layout.player.date}</span>}
            </p>
          )}
          {layout && (
            <ol className="why-chain" aria-label={tr.t("ui-why-chain")} data-testid="why-chain">
              {layout.rows.map((r) => (
                <li key={r.id} className="why-step" data-testid="why-step" data-depth={r.depth}>
                  {r.join !== null && (
                    <p className="why-join" data-testid="why-join">
                      {r.from !== null && <span className="why-from">{tr.t("ui-why-branch", { step: r.from })} </span>}
                      {r.join}
                    </p>
                  )}
                  <div className="why-card">
                    <time className="muted small">{r.date}</time>
                    <p className="why-text">{r.text}</p>
                    {r.roots.length > 0 && (
                      <ul className="why-roots">
                        {r.roots.map((x) => (
                          <li key={x.text} className={`why-root why-root-${x.kind}`} data-testid="why-root" data-kind={x.kind}>
                            <span className="why-mark" aria-hidden="true">
                              {ROOT_MARK[x.kind]}
                            </span>
                            <span>{x.text}</span>
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                </li>
              ))}
            </ol>
          )}
          {layout?.more && (
            <div className="why-more" data-testid="why-more">
              <p className="muted small">{tr.t("ui-why-more")}</p>
              {layout.further && (
                <button type="button" className="btn btn-sm" disabled={w.busy} onClick={() => void further()} data-testid="why-further">
                  {tr.t("ui-why-further")}
                </button>
              )}
            </div>
          )}
          {layout && w.factors && w.factors.factors.length > 0 && <Factors f={w.factors} />}
        </div>
      </aside>
    </>
  );
}

/** What moved the votes: the engine's decomposition of the race, as its result card shows it. */
function Factors({ f }: { f: WhyFactors }) {
  const tr = useGame((s) => s.tr);
  const factors: FactorView[] = f.factors;
  return (
    <section className="card" data-testid="why-factors">
      <h3 className="sub">{tr.t("ui-why-result")}</h3>
      <p className="hint">{tr.t(f.sign < 0 ? "ui-why-lost-help" : "ui-why-result-help")}</p>
      <WhyBars factors={factors} sign={f.sign} />
    </section>
  );
}
