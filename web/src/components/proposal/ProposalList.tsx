// Proposals as rows (DEC-092): on the desk (the office's open proposals, at
// the town hall where drafting is done) and at a place (`proposals_here`).
// Each row is the proposal's title and what the engine says of it in a line;
// it opens the proposal sheet. Presentation only.
import type { ProposalView } from "../../engine/protocol/ProposalView";
import { formatMoney } from "../../i18n/format";
import { useGame } from "../../state/gameStore";
import { personName } from "../Knowing";
import { Row, RowList } from "../notebook/rows";
import { proposalTitle } from "./ProposalSheet";
import { useProposal } from "./proposalStore";

function ProposalRow({ p }: { p: ProposalView }) {
  const tr = useGame((s) => s.tr);
  const openProposal = useProposal((s) => s.openProposal);
  const name = proposalTitle(tr, p);
  const who = p.sponsors.map((s) => personName(tr, s.person.name) || tr.t(s.person.role_key)).filter(Boolean);
  const sub = [
    p.level === "heard" ? tr.t("ui-proposal-heard") : p.est_annual_cost !== null ? tr.t("ui-proposal-cost", { amount: formatMoney(p.est_annual_cost) }) : null,
    p.bills.length > 0 ? tr.t("ui-proposal-yours", { n: p.bills.length }) : null,
    who.join(", ") || null,
  ]
    .filter(Boolean)
    .join(" · ");
  const open = (e?: { currentTarget: HTMLElement }) => void openProposal({ proposal: p.id, family: p.family }, { opener: e?.currentTarget ?? null });
  return (
    <Row
      mark="📋"
      title={name}
      sub={sub}
      reason={p.reason_key ? tr.event(p.reason_key, p.args) : null}
      label={`${name}. ${sub}`}
      onActivate={() => void openProposal({ proposal: p.id, family: p.family }, { opener: document.activeElement instanceof HTMLElement ? document.activeElement : null })}
      end={
        <button type="button" className="btn btn-sm rw-go" tabIndex={-1} aria-label={tr.t("ui-proposal-open-label", { name })} onClick={(e) => open(e)} data-testid={`open-proposal-${p.id}`}>
          {tr.t("ui-proposal-open")}
        </button>
      }
      li={{ "data-testid": `proposal-row-${p.id}`, "data-level": p.level, "data-family": p.family }}
    />
  );
}

export function ProposalRows({ proposals, testId }: { proposals: ProposalView[]; testId?: string }) {
  return (
    <RowList testId={testId}>
      {proposals.map((p) => (
        <ProposalRow key={p.id} p={p} />
      ))}
    </RowList>
  );
}

/** The desk's proposals: open in the player's jurisdictions, heard of (where drafting is done). */
export function DeskProposals() {
  const tr = useGame((s) => s.tr);
  const o = useGame((s) => s.office);
  if (!o || o.proposals.length === 0) return null;
  return (
    <section className="card" data-testid="desk-proposals">
      <h2>{tr.t("ui-proposals")}</h2>
      <p className="hint">{tr.t("ui-proposals-help")}</p>
      <ProposalRows proposals={o.proposals} testId="desk-proposal-list" />
    </section>
  );
}

/** Proposals about this place (bound to it, its streets, or a sponsor who works here). */
export function ProposalsHere({ proposals }: { proposals: ProposalView[] }) {
  const tr = useGame((s) => s.tr);
  if (proposals.length === 0) return null;
  return (
    <section className="card" data-testid="proposals-here">
      <h2>{tr.t("ui-proposals-here")}</h2>
      <ProposalRows proposals={proposals} testId="proposals-here-list" />
    </section>
  );
}
