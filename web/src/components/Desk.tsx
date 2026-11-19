import { useEffect, useState } from "react";
import type { OfficeView } from "../engine/protocol/OfficeView";
import type { ParamValue } from "../engine/protocol/ParamValue";
import type { PolicyPreviewView } from "../engine/protocol/PolicyPreviewView";
import type { TemplateView } from "../engine/protocol/TemplateView";
import { useGame } from "../state/gameStore";
import { formatDate, formatMoney } from "../i18n/format";
import { StatTile } from "./StatTile";
import { BillCard, SupportBars } from "./BillCard";
import { ParamSliders } from "./ParamSliders";
import { TradeOffs } from "./TradeOffs";
import { LeadLine, Unknown } from "./Knowing";

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
      {t.concerns_knowledge.level === "unknown" && (
        <p className="small">
          <Unknown k={t.concerns_knowledge} text="knowledge-concerns-unknown" />
        </p>
      )}
      <ul className="meters">
        {t.concerns.map((c) => (
          <li key={c.id}>
            <span>{tr.t(c.name_key)}</span>
            <span className="meter" aria-hidden="true">
              <span className="meter-fill" style={{ width: `${Math.round(Math.min(c.weight, 1) * 100)}%` }} />
            </span>
            <span className="num">{Math.round(c.weight * 100)}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}

/** How long the sliders must rest before the engine is asked for an estimate. */
export const PREVIEW_DEBOUNCE_MS = 150;

const paramsKey = (params: ParamValue[]) =>
  [...params]
    .sort((a, b) => a.id.localeCompare(b.id))
    .map((p) => `${p.id}=${p.value}`)
    .join("&");

/** The engine's estimate for the current slider values (debounced). The UI
 * never computes costs or side effects itself; until the first answer it
 * shows the catalogue's default estimate. */
function usePolicyPreview(template: string, params: ParamValue[], enabled: boolean) {
  const previewPolicy = useGame((s) => s.previewPolicy);
  const [answer, setAnswer] = useState<{ key: string; view: PolicyPreviewView } | null>(null);
  const key = paramsKey(params);
  useEffect(() => {
    if (!enabled) return;
    let live = true;
    const h = window.setTimeout(() => {
      void previewPolicy(template, params).then((view) => {
        if (live && view) setAnswer({ key, view });
      });
    }, PREVIEW_DEBOUNCE_MS);
    return () => {
      live = false;
      window.clearTimeout(h);
    };
  }, [enabled, key, params, previewPolicy, template]);
  return { view: answer?.view ?? null, stale: enabled && answer?.key !== key };
}

export function TemplateCard({ t }: { t: TemplateView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const perform = useGame((s) => s.perform);
  const [open, setOpen] = useState(false);
  const [params, setParams] = useState<ParamValue[]>(t.params.map((p) => ({ id: p.id, value: p.default })));
  const touched = params.some((p) => p.value !== t.params.find((d) => d.id === p.id)?.default);
  const preview = usePolicyPreview(t.id, params, open && touched);
  const cost = touched && preview.view ? preview.view.est_annual_cost : t.est_cost_default;
  const effects = touched && preview.view ? preview.view.side_effects : t.side_effects;
  // A situational policy only heard of is a stub: it can't be drafted yet (DEC-091).
  const stub = t.knowledge?.level === "heard";
  return (
    <li className="template">
      <button type="button" className="row-btn" aria-expanded={open} onClick={() => setOpen(!open)}>
        <span>
          {tr.t(t.name_key)} <span className="muted">· {tr.t(`category-${t.category}`)}</span>
        </span>
        <span className="num muted">{t.support.overall === null ? tr.t(`support-band-${t.support.band}`) : signed(t.support.overall)}</span>
      </button>
      {open && stub && (
        <div className="template-body" data-testid="policy-stub">
          {t.unlock_reason_key && <p className="hint">{tr.t("ui-policy-situational", { reason: tr.t(t.unlock_reason_key) })}</p>}
          <p className="small muted">
            {tr.t("knowledge-proposal-stub")}
            <LeadLine lead={t.knowledge?.lead ?? null} />
          </p>
        </div>
      )}
      {open && !stub && (
        <div className="template-body">
          <p className="small">{tr.t(t.desc_key)}</p>
          {t.situational && t.unlock_reason_key && <p className="hint">{tr.t("ui-policy-situational", { reason: tr.t(t.unlock_reason_key) })}</p>}
          <p className="muted small" data-testid="policy-estimate" aria-live="polite" aria-busy={preview.stale}>
            <span data-testid="policy-cost">{tr.t("ui-est-cost", { amount: formatMoney(cost) })}</span>
            {touched && preview.view && preview.view.budget_share > 0 && <> · {tr.t("ui-budget-share", { pct: (preview.view.budget_share * 100).toFixed(1) })}</>} · {tr.t("ui-impl-months", { n: t.implementation_months })}
            {preview.stale && <> · {tr.t("ui-preview-updating")}</>}
          </p>
          <ParamSliders defs={t.params} values={params} onChange={setParams} />
          <h4 className="sub">{tr.t("ui-support")}</h4>
          <SupportBars s={t.support} />
          <TradeOffs effects={effects} />
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
        {o.catalog
          .filter((t) => t.available)
          .map((t) => (
            <TemplateCard key={t.id} t={t} />
          ))}
      </ul>
      <LockedPolicies o={o} />
    </section>
  );
}

/** Situational policies not on the table yet (DEC-077 §3), with what opens them. */
function LockedPolicies({ o }: { o: OfficeView }) {
  const tr = useGame((s) => s.tr);
  const locked = o.catalog.filter((t) => !t.available);
  if (locked.length === 0) return null;
  return (
    <>
      <h3 className="sub">{tr.t("ui-catalog-situational")}</h3>
      <p className="hint">{tr.t("ui-catalog-situational-help")}</p>
      <ul className="plain" data-testid="catalog-locked">
        {locked.map((t) => (
          <li key={t.id} className="muted small">
            <strong>{tr.t(t.name_key)}</strong> · {t.unlock_reason_key ? tr.t("ui-policy-locked", { reason: tr.t(t.unlock_reason_key) }) : null}
          </li>
        ))}
      </ul>
    </>
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
