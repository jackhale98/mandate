// The doorstep (DEC-114 §3.3): one line from them, one choice, one line
// back. Who answers and what they say first, the three things to say (make
// the case on a worry the player knows of), their reply, and the street's
// progress: the first knock starts a canvass (its time shown before), the
// last finishes it, "Finish the street" does sooner. Opened from a house's
// door on the street or from the street's rows; everything shown is the
// engine's words (`DoorView`), and nothing here counts doors.
import { useRef, useState } from "react";
import { useGame } from "../../state/gameStore";
import { useSheetClose } from "../useSheetClose";
import { useDoor } from "./doorStore";
import "./doors.css";

export function DoorSheet() {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const command = useGame((s) => s.command);
  const v = useDoor((s) => s.view);
  const reply = useDoor((s) => s.reply);
  const error = useDoor((s) => s.error);
  const say = useDoor((s) => s.say);
  const close = useDoor((s) => s.close);
  const [casing, setCasing] = useState(false);
  const sheet = useRef<HTMLElement>(null);
  const [closing, requestClose, sheetRef] = useSheetClose(() => {
    setCasing(false);
    close();
  }, sheet);
  if (!v) return null;
  const street = v.street;
  const under = street.canvass;
  const canSay = street.can_knock && !v.knocked && !busy;
  const done = reply?.args.street === "done";
  const finish = async () => {
    await command({ type: "act", action: { type: "canvass", neighborhood: v.neighborhood }, late_night: false });
    requestClose();
  };
  return (
    <>
      <div className={closing ? "sheet-scrim door-scrim sheet-closing" : "sheet-scrim door-scrim"} onClick={requestClose} aria-hidden="true" />
      <aside ref={sheetRef} className={closing ? "sheet door-sheet sheet-closing" : "sheet door-sheet"} aria-labelledby="door-title" data-testid="door-sheet" data-door={v.door} data-hood={v.neighborhood}>
        <div className="sheet-head">
          <div>
            <h2 id="door-title">
              <span aria-hidden="true">🚪 </span>
              {tr.t("ui-door-title", { number: v.number })}
            </h2>
            <p className="muted small">{tr.t("ui-door-row", { family: v.family, number: v.number })}</p>
          </div>
          <div className="sheet-tools">
            <button type="button" className="btn btn-ghost" onClick={requestClose} aria-label={tr.t("ui-close")}>
              ✕
            </button>
          </div>
        </div>
        <div className="sheet-body">
          <section className="card door-step" data-testid="door-opener" data-answers={v.answers ? "1" : "0"}>
            {v.opener.map((l, i) => (
              <p key={i} className={i === 0 ? "door-who" : "door-line"}>
                {tr.event(l.key, l.args)}
              </p>
            ))}
          </section>
          {!reply && (
            <section className="card door-choices" data-testid="door-choices">
              <p className="small muted" data-testid="door-cost">
                {under ? tr.t("ui-door-under-way", { left: Math.max(0, under.left - 1) }) : tr.t("ui-door-starts", { slots: street.slots, doors: street.street_doors })}
              </p>
              {!street.can_knock && street.blocked_key && <p className="note small">{tr.t(street.blocked_key)}</p>}
              {casing ? (
                <div className="door-worries" role="group" aria-label={tr.t("ui-door-case")}>
                  {street.worries.length === 0 && <p className="small muted">{tr.t("ui-canvass-no-worries")}</p>}
                  {street.worries.map((w) => (
                    <button key={w.id} type="button" className="btn btn-block" disabled={!canSay} onClick={() => void say({ type: "case", issue: w.id })} data-testid={`door-worry-${w.id}`}>
                      {tr.t(w.name_key)}
                      {w.fit_band && <span className="muted small"> · {tr.t(`ui-canvass-fit-${w.fit_band}`)}</span>}
                    </button>
                  ))}
                  <button type="button" className="btn btn-ghost btn-sm" onClick={() => setCasing(false)}>
                    {tr.t("ui-back")}
                  </button>
                </div>
              ) : (
                <div className="door-choice-list">
                  {v.choices.map((c) => (
                    <button
                      key={c.kind}
                      type="button"
                      className={c.kind === "listen" ? "btn btn-primary btn-block" : "btn btn-block"}
                      disabled={!canSay || (c.kind === "case" && street.worries.length === 0)}
                      onClick={() => (c.kind === "case" ? setCasing(true) : void say(c.kind === "ask" ? { type: "ask" } : { type: "listen" }))}
                      data-testid={`door-choice-${c.kind}`}
                    >
                      {tr.t(c.label_key)}
                    </button>
                  ))}
                </div>
              )}
            </section>
          )}
          {reply && (
            <section className="card door-replied" data-testid="door-reply-card">
              <p className="door-reply" data-testid="door-reply" data-went={reply.args.went ?? ""}>
                {tr.event(reply.key, reply.args)}
              </p>
              {reply.args.agreed && <p className="small">{tr.t("door-agreed")}</p>}
              {done && (
                <p className="small" data-testid="door-street-done">
                  {tr.t("ui-door-street-done")} {tr.event("outcome-canvass", { place: reply.args.street_place ?? "", fresh: reply.args.street_fresh ?? "new" })}
                </p>
              )}
              <div className="door-next">
                <button type="button" className="btn btn-primary" onClick={requestClose} data-testid="door-next">
                  {tr.t(done ? "ui-door-close" : "ui-door-next")}
                </button>
                {!done && under && (
                  <button type="button" className="btn" disabled={busy} onClick={() => void finish()} data-testid="door-finish">
                    {tr.t("ui-door-finish")}
                  </button>
                )}
              </div>
            </section>
          )}
          {error && (
            <p className="error small" role="alert">
              {error}
            </p>
          )}
        </div>
      </aside>
    </>
  );
}
