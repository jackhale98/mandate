// Faults (DEC-114, upkeep made visible): what is wrong and where, how it
// stands in the repair queue, and "See it" (walk to the streets it is on).
// Minimal wiring for B2: the ask's fault in Today and the street's panel;
// B4 draws the cue on its spot. Everything shown is the engine's (the UI
// words nothing itself and counts nothing).
import type { FaultView } from "../../engine/protocol/FaultView";
import { formatDate } from "../../i18n/format";
import { useGame } from "../../state/gameStore";
import { capital } from "../DecisionSheet";
import { WhyButton } from "../why/WhyButton";

/** One fault: what is wrong, its standing in the queue, "See it", and "Why?". */
export function FaultLine({ f, see = true }: { f: FaultView; see?: boolean }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const visit = useGame((s) => s.visit);
  const what = tr.event(f.what_key, f.args);
  const name = tr.event(f.name_key, f.args);
  const status = tr.t(`ui-fault-status-${f.status}`);
  const when =
    f.status === "fixed" && f.fixed_on
      ? formatDate(tr, f.fixed_on)
      : f.status === "repairing" && f.done_by
        ? formatDate(tr, f.done_by)
        : tr.t("ui-fault-since", { date: formatDate(tr, f.opened) });
  return (
    <div className="fault" data-testid={`fault-${f.id}`} data-status={f.status} data-cue={f.cue} data-spot={`${f.spot.kind}:${f.spot.slot}`}>
      <p className="small">{what}</p>
      <p className="muted small">
        <span className="chip chip-muted" data-testid="fault-status">
          {status}
        </span>{" "}
        · {when}
        {f.moved_up && f.status === "open" && <> · {tr.t("ui-fault-moved-up")}</>}
        {f.next_in_queue && <> · {tr.t("ui-fault-next")}</>}
        {f.note_key && (
          <>
            {" "}
            · <span data-testid="fault-note">{tr.event(f.note_key, f.args)}</span>
          </>
        )}{" "}
        <WhyButton event={f.event} about={capital(name)} inline />
      </p>
      {see && !f.here && (
        <button
          type="button"
          className="btn btn-sm btn-ghost"
          disabled={busy}
          aria-label={tr.t("ui-fault-see-label", { fault: name, hood: f.args.hood ?? "" })}
          onClick={() => void visit(f.see.location)}
          data-testid={`fault-see-${f.id}`}
        >
          <span aria-hidden="true">👀 </span>
          {tr.t("ui-fault-see")}
        </button>
      )}
    </div>
  );
}

/** The street's faults (the place panel of a neighbourhood's streets). */
export function FaultsHere({ faults }: { faults: FaultView[] }) {
  const tr = useGame((s) => s.tr);
  if (faults.length === 0) return null;
  return (
    <section className="card" data-testid="faults-here" aria-labelledby="faults-here-title">
      <h2 id="faults-here-title">{tr.t("ui-faults-here")}</h2>
      <ul className="plain-list">
        {faults.map((f) => (
          <li key={f.id}>
            <FaultLine f={f} see={false} />
          </li>
        ))}
      </ul>
    </section>
  );
}
