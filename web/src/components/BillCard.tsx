import { useState } from "react";
import type { BillView } from "../engine/protocol/BillView";
import type { ClauseDto } from "../engine/protocol/ClauseDto";
import type { ParamValue } from "../engine/protocol/ParamValue";
import type { SupportView } from "../engine/protocol/SupportView";
import type { TemplateView } from "../engine/protocol/TemplateView";
import type { TownView } from "../engine/protocol/TownView";
import { useGame } from "../state/gameStore";
import { formatDate, formatMoney } from "../i18n/format";
import { ParamSliders } from "./ParamSliders";
import { TradeOffs } from "./TradeOffs";
import { personName } from "./Knowing";
import { signerText } from "./posts";

/** Signed −1..1 score shown as −100..+100 with a text label (never colour alone, DEC-051). */
export function SupportBars({ s }: { s: SupportView }) {
  const tr = useGame((st) => st.tr);
  // Who supports it is learned (DEC-091): below "known" only its band.
  if (s.overall === null) {
    return (
      <p className="small" data-testid="support-band">
        {tr.t(`support-band-${s.band}`)} · <span className="muted">{tr.t("knowledge-support-unknown")}</span>
      </p>
    );
  }
  const rows: [string, number][] = (
    [
      ["ui-support-overall", s.overall],
      ["ui-support-low", s.low],
      ["ui-support-middle", s.middle],
      ["ui-support-high", s.high],
    ] as [string, number | null][]
  ).filter((r): r is [string, number] => r[1] !== null);
  return (
    <ul className="diverge">
      {rows.map(([k, v]) => {
        const pct = Math.round(v * 100);
        return (
          <li key={k}>
            <span>{tr.t(k)}</span>
            <span className="dv-track" aria-hidden="true">
              <span className={pct >= 0 ? "dv-fill dv-pos" : "dv-fill dv-neg"} style={pct >= 0 ? { left: "50%", width: `${pct / 2}%` } : { right: "50%", width: `${-pct / 2}%` }} />
            </span>
            <span className="num">{pct > 0 ? `+${pct}` : pct}</span>
          </li>
        );
      })}
    </ul>
  );
}

function clauseLabel(tr: ReturnType<typeof useGame.getState>["tr"], c: ClauseDto, town: TownView): string {
  switch (c.type) {
    case "earmark":
      return tr.t("ui-earmark", { n: town.wards.find((w) => w.id === c.ward)?.name ?? "?" });
    case "sunset":
      return tr.t("ui-sunset", { years: c.years });
    case "pilot":
      return tr.t("ui-pilot", { place: town.neighborhoods.find((h) => h.id === c.neighborhood)?.name ?? "?" });
  }
}

export function BillCard({ bill, template, town }: { bill: BillView; template: TemplateView | undefined; town: TownView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const perform = useGame((s) => s.perform);
  const [why, setWhy] = useState(false);
  const [editing, setEditing] = useState(false);
  const [params, setParams] = useState<ParamValue[]>(bill.params);
  const [clauses, setClauses] = useState<ClauseDto[]>(bill.clauses);
  const mine = bill.sponsor_is_player;
  const editable = mine && (bill.status === "draft" || bill.status === "under_consideration");
  const pending = bill.status === "under_consideration";

  const status = bill.status === "ended" && bill.end_reason ? tr.t(`end-${bill.end_reason}`) : tr.t(`status-${bill.status}`);
  const hasClause = (t: ClauseDto["type"]) => clauses.some((c) => c.type === t);

  return (
    <article className="bill" data-testid={`bill-${bill.id}`}>
      <div className="card-head">
        <h3 className="bill-title">{tr.t(bill.name_key)}</h3>
        <span className={bill.status === "ended" ? "chip chip-muted" : "chip"}>{status}</span>
      </div>
      <p className="muted small">
        {tr.t("ui-est-cost", { amount: formatMoney(bill.est_annual_cost) })}
        {!mine && <> · {personName(tr, bill.sponsor_name)}</>}
      </p>
      {pending && bill.next_vote && bill.next_stage_key && (
        <p className="small">
          {tr.t("ui-next-vote", { stage: tr.t(bill.next_stage_key), date: formatDate(tr, bill.next_vote) })}
          <br />
          <strong>{bill.whip_exact ? tr.t("ui-whip", { yes: bill.expected_yes.toFixed(1), seats: bill.seats, needed: bill.needed }) : tr.t("ui-whip-about", { yes: bill.expected_yes, seats: bill.seats, needed: bill.needed })}</strong>
          {bill.signer && <span data-testid="signer-lean"> · {signerText(tr, bill.signer)}</span>}
        </p>
      )}
      {(bill.status === "enacted" || bill.status === "in_effect") && (
        <ul className="meters">
          <li>
            <span>{tr.t("ui-funded", { pct: Math.round(bill.funded_pct) })}</span>
            <span className="meter" aria-hidden="true">
              <span className="meter-fill" style={{ width: `${Math.min(100, bill.funded_pct)}%` }} />
            </span>
            <span className="num">{Math.round(bill.funded_pct)}%</span>
          </li>
          <li>
            <span>{tr.t("ui-progress", { pct: Math.round(bill.progress * 100) })}</span>
            <span className="meter" aria-hidden="true">
              <span className="meter-fill" style={{ width: `${Math.round(bill.progress * 100)}%` }} />
            </span>
            <span className="num">{Math.round(bill.progress * 100)}%</span>
          </li>
        </ul>
      )}
      {clauses.length > 0 && !editing && (
        <div className="tags">
          {clauses.map((c, i) => (
            <span key={i} className="chip chip-muted">
              {clauseLabel(tr, c, town)}
            </span>
          ))}
        </div>
      )}

      <div className="bill-actions">
        <button type="button" className="btn btn-sm btn-ghost" aria-expanded={why} onClick={() => setWhy(!why)}>
          {tr.t("ui-why")}
        </button>
        {editable && (
          <button type="button" className="btn btn-sm btn-ghost" aria-expanded={editing} onClick={() => setEditing(!editing)}>
            {tr.t("ui-amend-open")}
          </button>
        )}
        {mine && bill.status === "draft" && (
          <button type="button" className="btn btn-sm btn-primary" disabled={busy} onClick={() => void perform({ type: "introduce_policy", policy: bill.id })}>
            {tr.t("ui-introduce")}
          </button>
        )}
        {editable && (
          <button type="button" className="btn btn-sm btn-ghost" disabled={busy} onClick={() => void perform({ type: "withdraw_policy", policy: bill.id })}>
            {tr.t("ui-withdraw")}
          </button>
        )}
      </div>

      {editing && template && (
        <div className="bill-edit">
          <ParamSliders defs={template.params} values={params} onChange={setParams} />
          <h4 className="sub">{tr.t("ui-clauses")}</h4>
          <ul className="clause-list">
            {clauses.map((c, i) => (
              <li key={i}>
                {c.type === "earmark" ? (
                  <select className="input" aria-label={tr.t("ui-add-earmark")} value={c.ward} onChange={(e) => setClauses(clauses.map((x, j) => (j === i ? { type: "earmark", ward: Number(e.target.value) } : x)))}>
                    {town.wards.map((w) => (
                      <option key={w.id} value={w.id}>
                        {tr.t("ui-ward", { n: w.name })}
                      </option>
                    ))}
                  </select>
                ) : c.type === "pilot" ? (
                  <select className="input" aria-label={tr.t("ui-add-pilot")} value={c.neighborhood} onChange={(e) => setClauses(clauses.map((x, j) => (j === i ? { type: "pilot", neighborhood: Number(e.target.value) } : x)))}>
                    {town.neighborhoods.map((h) => (
                      <option key={h.id} value={h.id}>
                        {h.name}
                      </option>
                    ))}
                  </select>
                ) : (
                  <select className="input" aria-label={tr.t("ui-add-sunset")} value={c.years} onChange={(e) => setClauses(clauses.map((x, j) => (j === i ? { type: "sunset", years: Number(e.target.value) } : x)))}>
                    {[2, 3, 4, 5, 8, 10].map((y) => (
                      <option key={y} value={y}>
                        {tr.t("ui-sunset", { years: y })}
                      </option>
                    ))}
                  </select>
                )}
                <button type="button" className="btn btn-sm btn-ghost" onClick={() => setClauses(clauses.filter((_, j) => j !== i))}>
                  {tr.t("ui-remove")}
                </button>
              </li>
            ))}
          </ul>
          <div className="bill-actions">
            {town.wards.length > 0 && !hasClause("earmark") && (
              <button type="button" className="btn btn-sm btn-ghost" onClick={() => setClauses([...clauses, { type: "earmark", ward: town.wards[0]?.id ?? 0 }])}>
                {tr.t("ui-add-earmark")}
              </button>
            )}
            {!hasClause("sunset") && (
              <button type="button" className="btn btn-sm btn-ghost" onClick={() => setClauses([...clauses, { type: "sunset", years: 4 }])}>
                {tr.t("ui-add-sunset")}
              </button>
            )}
            {town.neighborhoods.length > 0 && !hasClause("pilot") && (
              <button type="button" className="btn btn-sm btn-ghost" onClick={() => setClauses([...clauses, { type: "pilot", neighborhood: town.neighborhoods[0]?.id ?? 0 }])}>
                {tr.t("ui-add-pilot")}
              </button>
            )}
          </div>
          {pending && <p className="hint">{tr.t("ui-amend-cost")}</p>}
          <button
            type="button"
            className="btn btn-primary btn-block"
            disabled={busy}
            onClick={() => {
              void perform({ type: "amend_policy", policy: bill.id, params, clauses }).then(() => setEditing(false));
            }}
          >
            {tr.t("ui-amend")}
          </button>
        </div>
      )}

      {why && (
        <div className="why">
          <h4 className="sub">{tr.t("ui-support")}</h4>
          <SupportBars s={bill.support} />
          <TradeOffs effects={bill.side_effects} />
          {template && template.secondary_keys.length > 0 && (
            <>
              <h4 className="sub">{tr.t("ui-secondary")}</h4>
              <ul className="plain">
                {template.secondary_keys.map((k) => (
                  <li key={k}>{tr.t(k)}</li>
                ))}
              </ul>
            </>
          )}
          {bill.ledger.length > 0 && (
            <>
              <h4 className="sub">{tr.t("ui-ledger")}</h4>
              <dl className="ledger">
                {bill.ledger.map((l) => (
                  <div key={l.key}>
                    <dt>{tr.t(`ledger-${l.key}`)}</dt>
                    <dd className="num">{Math.abs(l.value) >= 100 ? formatMoney(l.value) : l.value.toFixed(1)}</dd>
                  </div>
                ))}
              </dl>
            </>
          )}
          {bill.votes.length > 0 && (
            <>
              <h4 className="sub">{tr.t("ui-votes")}</h4>
              <ul className="plain">
                {bill.votes.map((v, i) => (
                  <li key={i}>
                    {formatDate(tr, v.date)}:{" "}
                    {tr.t("ui-vote-record", { yes: v.yes, no: v.no, result: tr.t(v.veto ? "ui-vetoed" : v.passed ? "ui-passed" : "ui-failed") })}
                  </li>
                ))}
              </ul>
            </>
          )}
        </div>
      )}
    </article>
  );
}
