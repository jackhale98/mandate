// A lead (DEC-091: finding things is guided, not hunted): what to ask and
// where, and a way there (auto-routed, DEC-079). An arc only heard of is a
// lead too (A3): that something is going on, and where, never who or what.
import type { ArcLeadView } from "../../engine/protocol/ArcLeadView";
import type { LeadView } from "../../engine/protocol/LeadView";
import type { LocationView } from "../../engine/protocol/LocationView";
import { placeName } from "../../places/labels";
import { useGame } from "../../state/gameStore";
import { leadText } from "./notebookModel";

function GoThere({ to, here }: { to: LocationView | null; here: boolean }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const visit = useGame((s) => s.visit);
  if (!to) return null;
  if (here) return <span className="muted small">{tr.t("ui-notebook-here")}</span>;
  return (
    <button type="button" className="btn btn-sm" disabled={busy} onClick={() => void visit(to.location)} data-testid="notebook-lead-go">
      {tr.t("ui-notebook-go")}: {placeName(tr, to)}
    </button>
  );
}

export function Lead({ lead }: { lead: LeadView }) {
  const tr = useGame((s) => s.tr);
  return (
    <div className="nb-lead" data-testid="notebook-lead">
      <p>
        <span className="nb-lead-label">{tr.t("ui-notebook-lead")}: </span>
        {leadText(tr, lead)}
      </p>
      <GoThere to={lead.route?.to ?? lead.to} here={lead.route?.here === true} />
    </div>
  );
}

export function ArcLead({ lead }: { lead: ArcLeadView }) {
  const tr = useGame((s) => s.tr);
  return (
    <div className="nb-lead" data-testid="arc-lead">
      <p>{tr.event(lead.text.key, lead.text.args)}</p>
      <GoThere to={lead.route?.to ?? lead.to} here={lead.route?.here === true} />
    </div>
  );
}
