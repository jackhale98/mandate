import { useState } from "react";
import type { OfficeView } from "../engine/protocol/OfficeView";
import type { ParamValue } from "../engine/protocol/ParamValue";
import type { TemplateView } from "../engine/protocol/TemplateView";
import { useGame } from "../state/gameStore";
import { formatDate, formatMoney } from "../i18n/format";
import { StatTile } from "./StatTile";
import { BillCard, SupportBars } from "./BillCard";
import { ParamSliders } from "./ParamSliders";
import { TradeOffs } from "./TradeOffs";

function signed(v: number): string {
  const n = Math.round(v * 100);
  return n > 0 ? `+${n}` : String(n);
}

export function TownOpinion({ o }: { o: OfficeView }) {
  const tr = useGame((s) => s.tr);
  const t = o.town;
  return (
    <section className="card">
      <h2>{tr.t("ui-town-opinion")}</h2>
      <div className="stats stats-3">
        <StatTile label={tr.t("ui-gov-approval")} value={signed(t.gov_approval)} />
        <StatTile label={tr.t("ui-you-in-ward")} value={`${Math.round(t.player_awareness_ward * 100)}%`} delta={{ text: `${tr.t("ui-awareness")} · ${signed(t.player_approval_ward)} ${tr.t("ui-approval")}`, good: null }} />
        <StatTile label={tr.t("ui-you-in-town")} value={`${Math.round(t.player_awareness_town * 100)}%`} delta={{ text: `${tr.t("ui-awareness")} · ${signed(t.player_approval_town)} ${tr.t("ui-approval")}`, good: null }} />
      </div>
      <h3 className="sub">{tr.t("ui-concerns")}</h3>
      <ul className="meters">
        {t.concerns.map((c) => (
          <li key={c.key}>
            <span>{tr.t(`concern-${c.key}`)}</span>
            <span className="meter" aria-hidden="true">
              <span className="meter-fill" style={{ width: `${Math.round(c.weight * 100)}%` }} />
            </span>
            <span className="num">{Math.round(c.weight * 100)}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}

export function TemplateCard({ t }: { t: TemplateView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const perform = useGame((s) => s.perform);
  const [open, setOpen] = useState(false);
  const [params, setParams] = useState<ParamValue[]>(t.params.map((p) => ({ id: p.id, value: p.default })));
  return (
    <li className="template">
      <button type="button" className="row-btn" aria-expanded={open} onClick={() => setOpen(!open)}>
        <span>
          {tr.t(t.name_key)} <span className="muted">· {tr.t(`category-${t.category}`)}</span>
        </span>
        <span className="num muted">{signed(t.support.overall)}</span>
      </button>
      {open && (
        <div className="template-body">
          <p className="small">{tr.t(t.desc_key)}</p>
          <p className="muted small">
            {tr.t("ui-est-cost", { amount: formatMoney(t.est_cost_default) })} · {tr.t("ui-impl-months", { n: t.implementation_months })}
          </p>
          <ParamSliders defs={t.params} values={params} onChange={setParams} />
          <h4 className="sub">{tr.t("ui-support")}</h4>
          <SupportBars s={t.support} />
          <TradeOffs effects={t.side_effects} />
          {t.secondary_keys.length > 0 && (
            <>
              <h4 className="sub">{tr.t("ui-secondary")}</h4>
              <ul className="plain">
                {t.secondary_keys.map((k) => (
                  <li key={k}>{tr.t(k)}</li>
                ))}
              </ul>
            </>
          )}
          <button type="button" className="btn btn-primary btn-block" disabled={busy} onClick={() => void perform({ type: "draft_policy", template: t.id, params }).then((id) => id !== null && setOpen(false))}>
            {tr.t("ui-draft")}
          </button>
        </div>
      )}
    </li>
  );
}

export function RecentActions() {
  const tr = useGame((s) => s.tr);
  const o = useGame((s) => s.office);
  const p = o?.player;
  if (!p || p.recent_actions.length === 0) return null;
  return (
    <section className="card">
      <h2>{tr.t("ui-recent-actions")}</h2>
      <ol className="events">
        {p.recent_actions.map((a, i) => (
          <li key={i}>
            <time className="muted">{formatDate(tr, a.date)}</time>
            <span>{tr.t(a.outcome_key, a.args)}</span>
          </li>
        ))}
      </ol>
    </section>
  );
}

/** The policy catalogue (drafting is done at the town hall). */
export function Catalog() {
  const tr = useGame((s) => s.tr);
  const o = useGame((s) => s.office);
  if (!o?.player.can_propose) return null;
  return (
    <section className="card" data-testid="catalog">
      <h2>{tr.t("ui-catalog")}</h2>
      <p className="hint">{tr.t("ui-catalog-help")}</p>
      <ul className="region-list template-list">
        {o.catalog.map((t) => (
          <TemplateCard key={t.id} t={t} />
        ))}
      </ul>
    </section>
  );
}

/** The player's bills at a stage (drafts at the town hall, bills in the chamber). */
export function MyBills({ statuses, testId }: { statuses: string[]; testId: string }) {
  const tr = useGame((s) => s.tr);
  const o = useGame((s) => s.office);
  if (!o) return null;
  const byId = new Map(o.catalog.map((t) => [t.id, t]));
  const mine = o.bills.filter((b) => b.sponsor_is_player && statuses.includes(b.status));
  return (
    <section className="card" data-testid={testId}>
      <h2>{tr.t(statuses.includes("draft") ? "ui-your-drafts" : "ui-your-bills")}</h2>
      {mine.length === 0 && <p className="muted">{tr.t(statuses.includes("draft") ? "ui-no-drafts" : "ui-no-bills")}</p>}
      <div className="bill-list">
        {mine.map((b) => (
          <BillCard key={`${b.id}-${b.status}`} bill={b} template={byId.get(b.template)} town={o.town} />
        ))}
      </div>
    </section>
  );
}
