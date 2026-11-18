import { useState } from "react";
import type { OfficeView } from "../engine/protocol/OfficeView";
import type { ParamValue } from "../engine/protocol/ParamValue";
import type { TemplateView } from "../engine/protocol/TemplateView";
import type { ActionDto } from "../engine/protocol/ActionDto";
import { useGame } from "../state/gameStore";
import { formatDate, formatMoney } from "../i18n/format";
import { StatTile } from "../components/StatTile";
import { BillCard, SupportBars } from "../components/BillCard";
import { ParamSliders } from "../components/ParamSliders";
import { TradeOffs } from "../components/TradeOffs";

const TRACKS = ["viability", "standing", "health", "legal_jeopardy", "threat"] as const;
/** Tracks where a high value is bad (shown with a "risk" bar plus the text label). */
const RISK = new Set<string>(["legal_jeopardy", "threat"]);

function PlayerCard({ o }: { o: OfficeView }) {
  const tr = useGame((s) => s.tr);
  const p = o.player;
  return (
    <section className="card" data-testid="player-card">
      <div className="card-head">
        <div>
          <h2>{p.name}</h2>
          <p className="muted small">
            {tr.t(p.role_key)} · {p.municipality} · {tr.t("ui-ward", { n: p.ward_number })}
          </p>
        </div>
        <span className="chip" data-testid="slots">
          {tr.t("ui-slots", { left: p.slots_left })}
        </span>
      </div>
      {p.slot_cap < p.slots_total && (
        <p className="note" data-testid="tired">
          {tr.t("ui-tired", { n: p.slots_total - p.slot_cap })}
        </p>
      )}
      <div className="stats stats-3">
        <StatTile label={tr.t("ui-mandate")} value={String(Math.round(p.mandate))} />
        <StatTile label={tr.t("ui-money")} value={formatMoney(p.money)} />
        <StatTile label={tr.t("ui-late-night-short")} value={tr.t(p.late_night_available ? "ui-yes" : "ui-no")} />
      </div>
      <h3 className="sub">{tr.t("ui-tracks")}</h3>
      <ul className="bars">
        {TRACKS.map((k) => {
          const v = Math.round(p.tracks[k]);
          return (
            <li key={k}>
              <span>{tr.t(`track-${k}`)}</span>
              <span className="bar" aria-hidden="true">
                <span className={`bar-fill ${RISK.has(k) ? "bar-risk" : "bar-good"}`} style={{ width: `${Math.max(0, Math.min(100, v))}%` }} />
              </span>
              <span className="num">{v}</span>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

function signed(v: number): string {
  const n = Math.round(v * 100);
  return n > 0 ? `+${n}` : String(n);
}

function TownOpinion({ o }: { o: OfficeView }) {
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

type AgendaKind = "canvass" | "town_hall" | "press" | "fundraise" | "rest";

export function Agenda({ o, campaigning = false }: { o: OfficeView; campaigning?: boolean }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const command = useGame((s) => s.command);
  const [kind, setKind] = useState<AgendaKind>("canvass");
  const [hood, setHood] = useState<number>(o.town.neighborhoods[0]?.id ?? 0);
  const [bill, setBill] = useState<number | null>(null);
  const [late, setLate] = useState(false);
  const p = o.player;
  const live = o.bills.filter((b) => b.status !== "ended" && b.status !== "draft");
  const lateAllowed = kind === "fundraise";
  const useLate = late && lateAllowed;
  const costOf = (k: string) => p.action_costs.find((c) => c.action === k) ?? { action: k, slots: 1, health: 0 };
  const cost = costOf(kind);
  const canAct = useLate ? p.late_night_available : p.slots_left >= cost.slots;

  const action = (): ActionDto => {
    switch (kind) {
      case "canvass":
        return { type: "canvass", neighborhood: hood };
      case "town_hall":
        return { type: "town_hall", neighborhood: hood, policy: bill };
      case "press":
        return { type: "press", policy: bill };
      case "fundraise":
        return { type: "fundraise" };
      case "rest":
        return { type: "rest" };
    }
  };
  const kinds: AgendaKind[] = ["canvass", "town_hall", "press", "fundraise", "rest"];
  const hoodIndex = o.town.neighborhoods.findIndex((h) => h.id === hood);
  const hallRisk = o.town.town_hall_risk[hoodIndex] ?? 0;
  const pct = (v: number) => Math.round(v * 100);

  return (
    <section className="card" data-testid="agenda">
      <h2>{tr.t("ui-agenda")}</h2>
      <p className="hint">{tr.t("ui-agenda-help")}</p>
      <div className="segmented segmented-sm agenda-kinds" role="radiogroup" aria-label={tr.t("ui-agenda")}>
        {kinds.map((k) => (
          <label key={k} className={kind === k ? "seg seg-on" : "seg"}>
            <input type="radio" name="agenda" checked={kind === k} onChange={() => setKind(k)} />
            {tr.t(`action-${k.replace("_", "-")}`)}
          </label>
        ))}
      </div>
      {(kind === "canvass" || kind === "town_hall") && (
        <label className="field">
          <span>{tr.t("ui-neighbourhood")}</span>
          <select className="input" value={hood} onChange={(e) => setHood(Number(e.target.value))}>
            {o.town.neighborhoods.map((h) => (
              <option key={h.id} value={h.id}>
                {h.name}
              </option>
            ))}
          </select>
        </label>
      )}
      {(kind === "town_hall" || kind === "press") && (
        <label className="field">
          <span>{tr.t("ui-about-bill")}</span>
          <select className="input" value={bill ?? ""} onChange={(e) => setBill(e.target.value === "" ? null : Number(e.target.value))}>
            <option value="">{tr.t("ui-none")}</option>
            {live.map((b) => (
              <option key={b.id} value={b.id}>
                {tr.t(b.name_key)}
              </option>
            ))}
          </select>
        </label>
      )}
      {/* Every option shows what it costs and what it risks before you commit (DEC-073). */}
      <p className="tradeoff" data-testid="tradeoff">
        {tr.t("ui-cost-line", { slots: cost.slots, health: Math.max(0, cost.health) })} {tr.t(`tradeoff-${kind.replace("_", "-")}`)}
        {kind === "town_hall" && <strong> {tr.t("ui-risk-hostile", { pct: pct(hallRisk) })}</strong>}
        {kind === "press" && <strong> {tr.t("ui-risk-gaffe", { pct: pct(p.press_gaffe_risk) })}</strong>}
        {kind === "fundraise" && useLate && <strong> {tr.t("ui-risk-late-money")}</strong>}
      </p>
      {lateAllowed && (
        <label className="check">
          <input type="checkbox" checked={late} disabled={!p.late_night_available} onChange={(e) => setLate(e.target.checked)} />
          {tr.t("ui-late-night-meeting")}
        </label>
      )}
      <button type="button" className="btn btn-primary btn-block" disabled={busy || !canAct} onClick={() => void command({ type: "act", action: action(), late_night: useLate })}>
        {tr.t("ui-do")}
      </button>
      {campaigning && (
        <div className="ad-buy">
          <p className="tradeoff">{tr.t("tradeoff-advertise", { cost: formatMoney(p.ad_cost) })}</p>
          <button
            type="button"
            className="btn btn-block"
            data-testid="buy-ads"
            disabled={busy || p.ads_left === 0 || p.money < p.ad_cost}
            onClick={() => void command({ type: "act", action: { type: "advertise" }, late_night: false })}
          >
            {tr.t("ui-buy-ads", { cost: formatMoney(p.ad_cost), left: p.ads_left })}
          </button>
        </div>
      )}
      {p.recent_actions.length > 0 && (
        <>
          <h3 className="sub">{tr.t("ui-recent-actions")}</h3>
          <ol className="events">
            {p.recent_actions.map((a, i) => (
              <li key={i}>
                <time className="muted">{formatDate(tr, a.date)}</time>
                <span>{tr.t(a.outcome_key, a.args)}</span>
              </li>
            ))}
          </ol>
        </>
      )}
    </section>
  );
}

function TemplateCard({ t }: { t: TemplateView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const command = useGame((s) => s.command);
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
          <button type="button" className="btn btn-primary btn-block" disabled={busy} onClick={() => void command({ type: "draft_policy", template: t.id, params }).then((id) => id !== null && setOpen(false))}>
            {tr.t("ui-draft")}
          </button>
        </div>
      )}
    </li>
  );
}

export function Office() {
  const tr = useGame((s) => s.tr);
  const o = useGame((s) => s.office);
  if (!o) {
    return (
      <main className="game-body">
        <div className="center">
          <div className="spinner" aria-hidden="true" />
        </div>
      </main>
    );
  }
  const byId = new Map(o.catalog.map((t) => [t.id, t]));
  const mine = o.bills.filter((b) => b.sponsor_is_player);
  return (
    <main className="game-body">
      <PlayerCard o={o} />
      <Agenda o={o} />
      <section className="card" data-testid="bills">
        <h2>{tr.t("ui-bills")}</h2>
        {mine.length === 0 && <p className="muted">{tr.t("ui-no-bills")}</p>}
        <div className="bill-list">
          {mine.map((b) => (
            <BillCard key={`${b.id}-${b.status}`} bill={b} template={byId.get(b.template)} town={o.town} />
          ))}
        </div>
      </section>
      {o.player.can_propose && (
        <section className="card" data-testid="catalog">
          <h2>{tr.t("ui-catalog")}</h2>
          <ul className="region-list template-list">
            {o.catalog.map((t) => (
              <TemplateCard key={t.id} t={t} />
            ))}
          </ul>
        </section>
      )}
      <TownOpinion o={o} />
    </main>
  );
}
