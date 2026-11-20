// The promise picker (DEC-097 §2; ENGAGEMENT §3.5): while campaigning, the
// engine's menu of promises built from the constituency's live state, each
// with what it commits to, when it falls due, and what keeping or breaking
// it does. Making one is free; a race allows a few.
import type { EffectLineView } from "../../engine/protocol/EffectLineView";
import { useGame } from "../../state/gameStore";
import { capital } from "../DecisionSheet";
import { useSheetClose } from "../useSheetClose";

function Lines({ title, lines }: { title: string; lines: EffectLineView[] }) {
  const tr = useGame((s) => s.tr);
  if (lines.length === 0) return null;
  return (
    <div>
      <h4 className="sub small">{title}</h4>
      <ul className="plain effect-list">
        {lines.map((l, i) => (
          <li key={i} className={`effect effect-${l.tone}`}>
            {tr.event(l.key, l.args)}
          </li>
        ))}
      </ul>
    </div>
  );
}

export function PromisePicker() {
  const tr = useGame((s) => s.tr);
  const menu = useGame((s) => s.promiseMenu);
  const open = useGame((s) => s.openPromises);
  const make = useGame((s) => s.makePromise);
  const busy = useGame((s) => s.busy);
  const [closing, requestClose, sheetRef] = useSheetClose(() => void open(false));
  if (!menu) return null;
  const voters = menu.voters;
  const votersText = voters && (voters.type === "constituency" || voters.type === "electorate") ? tr.event(voters.name_key, menu.voters_args) : null;
  return (
    <>
      <div className={closing ? "sheet-scrim sheet-closing" : "sheet-scrim"} onClick={requestClose} aria-hidden="true" />
      <aside ref={sheetRef} className={closing ? "sheet story-sheet sheet-closing" : "sheet story-sheet"} aria-labelledby="promise-picker-title" data-testid="promise-picker">
        <div className="sheet-head">
          <div>
            <h2 id="promise-picker-title">{tr.t("ui-promise-picker")}</h2>
            <p className="muted small" data-testid="promise-made">
              {tr.t("ui-promise-made", { made: menu.made, max: menu.max })}
              {votersText && <> · {tr.t("ui-promise-to", { voters: votersText })}</>}
            </p>
          </div>
          <button type="button" className="btn btn-ghost" onClick={requestClose} aria-label={tr.t("ui-close")}>
            ✕
          </button>
        </div>
        <div className="sheet-body">
          <p className="hint">{tr.t("ui-promise-picker-intro")}</p>
          {menu.options.length === 0 ? (
            <p className="muted" data-testid="promise-none">
              {tr.t("ui-promise-none")}
            </p>
          ) : (
            <ol className="options">
              {menu.options.map((o) => {
                const title = capital(tr.event(o.title_key, o.args));
                const id = `${o.template}:${o.answer ?? ""}`;
                return (
                  <li key={id} className="option" data-testid={`promise-option-${o.template}`}>
                    <p className="option-label">
                      <strong>{title}</strong>
                    </p>
                    <p className="small">{tr.event(o.detail_key, o.args)}</p>
                    <p className="small muted">
                      {tr.event(o.need_key, o.args)} · {tr.event(o.due_key, o.args)}
                    </p>
                    <div className="option-tradeoffs">
                      <Lines title={tr.t("ui-ask-if-kept")} lines={o.keep_lines} />
                      <Lines title={tr.t("ui-ask-if-broken")} lines={o.break_lines} />
                    </div>
                    <button type="button" className="btn btn-sm btn-primary" disabled={busy} aria-label={tr.t("ui-promise-choose-label", { promise: title })} onClick={() => void make(o.template, o.answer)} data-testid={`promise-make-${o.template}`}>
                      {tr.t("ui-promise-choose")}
                    </button>
                  </li>
                );
              })}
            </ol>
          )}
        </div>
      </aside>
    </>
  );
}
