// A street's doors as rows (DEC-114 §2, quick mode, DEC-051): the doors the
// engine would knock first and the households the player remembers, then
// every door on request, each with "Knock" (the same doorstep the street's
// houses open). The canvass under way shows its progress and "Finish the
// street"; "Canvass the street" in one tap stays in the place's actions.
// Rows name the household on the door ("The Harrow family, number 12") and
// what they remember of the player in words; nothing counts doors (DEC-113).
import { useState } from "react";
import type { DoorRowView } from "../../engine/protocol/DoorRowView";
import type { StreetDoorsView } from "../../engine/protocol/StreetDoorsView";
import { useGame } from "../../state/gameStore";
import { useDoor } from "./doorStore";
import "./doors.css";

function DoorRow({ d, hood, can }: { d: DoorRowView; hood: number; can: boolean }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const open = useDoor((s) => s.open);
  const last = d.last;
  return (
    <li className="door-row" data-testid={`door-row-${d.door}`} data-door={d.door} data-knocked={d.knocked ? "1" : "0"}>
      <span>
        <strong>{d.vacant ? tr.t("ui-door-row-vacant", { number: d.number }) : tr.t("ui-door-row", { family: d.family, number: d.number })}</strong>
        {last && (
          <span className="muted small">
            {" "}
            · {tr.t(`ui-door-last-${last.went}`, { month: tr.t(`month-${last.on.month}`) })}
            {last.worry_key && <> {tr.t("ui-door-last-worry", { worry: tr.t(last.worry_key) })}</>}
          </span>
        )}
        {d.poster && <span className="chip chip-muted door-poster">{tr.t("ui-door-poster")}</span>}
      </span>
      {d.knocked ? (
        <span className="chip chip-muted">{tr.t("ui-door-knocked")}</span>
      ) : (
        <button type="button" className="btn btn-sm" disabled={busy || !can} onClick={() => void open(hood, d.door)} data-testid={`door-knock-${d.door}`}>
          {tr.t("ui-door-knock")}
        </button>
      )}
    </li>
  );
}

export function DoorsHere({ v }: { v: StreetDoorsView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const command = useGame((s) => s.command);
  const [all, setAll] = useState(false);
  // The engine's next doors first (in its order), then the households who remember a call.
  const suggested = v.suggested.map((k) => v.doors.find((d) => d.door === k)).filter((d): d is DoorRowView => d !== undefined);
  const rows = all ? v.doors : [...suggested, ...v.doors.filter((d) => !v.suggested.includes(d.door) && (d.last !== null || d.knocked))];
  const under = v.canvass;
  return (
    <section className="card doors-here" data-testid="doors-here">
      <h2>{tr.t("ui-doors")}</h2>
      <p className="small muted" data-testid="doors-cost">
        {under ? tr.t("ui-door-under-way", { left: under.left }) : tr.t("ui-door-starts", { slots: v.slots, doors: v.street_doors })}
      </p>
      {!v.can_knock && v.blocked_key && <p className="note small">{tr.t(v.blocked_key)}</p>}
      <ul className="plain-list door-rows">
        {rows.map((d) => (
          <DoorRow key={d.door} d={d} hood={v.neighborhood} can={v.can_knock} />
        ))}
      </ul>
      <div className="door-next">
        {!all && rows.length < v.doors.length && (
          <button type="button" className="btn btn-sm btn-ghost" onClick={() => setAll(true)} data-testid="doors-all">
            {tr.t("ui-door-more")}
          </button>
        )}
        {under && (
          <button type="button" className="btn btn-sm" disabled={busy} onClick={() => void command({ type: "act", action: { type: "canvass", neighborhood: v.neighborhood }, late_night: false })} data-testid="doors-finish">
            {tr.t("ui-door-finish")}
          </button>
        )}
      </div>
    </section>
  );
}
