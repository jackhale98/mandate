import { useState } from "react";
import type { ActionDto } from "../engine/protocol/ActionDto";
import type { CommandDto } from "../engine/protocol/CommandDto";
import { useGame } from "../state/gameStore";
import { routeFor } from "../state/routing";
import { formatMoney } from "../i18n/format";
import { placeName } from "../places/labels";
import { WorryPicker, useCanvassPreview } from "./talk/WorryPicker";

export type FormAction = "canvass" | "town_hall" | "press" | "fundraise" | "rest" | "advertise" | "lobby" | "trade_favor" | "take_bribe" | "dip_funds" | "leak_file";

const LATE: ReadonlySet<FormAction> = new Set(["lobby", "trade_favor", "fundraise", "take_bribe"]);
/** Only late at night (an envelope under the table). */
const LATE_ONLY: ReadonlySet<FormAction> = new Set(["take_bribe"]);
/** Borderline and illegal actions (DEC-034): a payoff now, a risk later. */
export const SHADY: ReadonlySet<FormAction> = new Set(["take_bribe", "dip_funds", "leak_file"]);

/**
 * One action with its choices, cost, and risks (DEC-073). Choosing it
 * auto-routes the player to where it is done (DEC-079); `hood` fixes the
 * neighbourhood when the form sits in a place panel.
 */
export function ActionForm({ action, hood: fixedHood, lateOnly = false, allowLate = true, showRoute = false }: { action: FormAction; hood?: number | null; lateOnly?: boolean; allowLate?: boolean; showRoute?: boolean }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const o = useGame((s) => s.office);
  const pocket = useGame((s) => s.pocket);
  const perform = useGame((s) => s.perform);
  const [hoodSel, setHood] = useState<number | null>(null);
  const [bill, setBill] = useState<number | null>(null);
  const [member, setMember] = useState<number | null>(null);
  const [late, setLate] = useState(lateOnly || LATE_ONLY.has(action));
  // Canvassing on a worry (DEC-091): the worries and their match come from the engine's preview.
  const [issue, setIssue] = useState<string | null>(null);
  const [landed, setLanded] = useState<{ fit: string; nameKey: string } | null>(null);
  const canvassHood = action === "canvass" ? (fixedHood ?? hoodSel ?? o?.town.neighborhoods[0]?.id ?? null) : null;
  const canvass = useCanvassPreview(canvassHood);
  if (!o || !pocket) return null;
  const shady = o.player.shady.find((x) => x.action === action);
  const p = o.player;
  const hood = fixedHood ?? hoodSel ?? o.town.neighborhoods[0]?.id ?? 0;
  const live = o.bills.filter((b) => b.status === "under_consideration" || b.status === "enacted" || b.status === "in_effect");
  const pending = o.bills.filter((b) => b.status === "under_consideration");
  const lobbyBill = action === "lobby" || action === "trade_favor" ? (pending.find((b) => b.id === bill) ?? pending[0]) : undefined;
  // The bodies' members, then the executive (DEC-098: whoever the constitution seats).
  const colleagues = o.bodies
    .flatMap((b) => b.members)
    .concat(o.executive ? [o.executive] : [])
    .filter((m) => !m.is_player);
  const target = colleagues.find((m) => m.id === member) ?? colleagues[0];
  const lateAllowed = allowLate && LATE.has(action);
  const useLate = (late || lateOnly || LATE_ONLY.has(action)) && lateAllowed;
  const cost = p.action_costs.find((c) => c.action === action) ?? { action, slots: 1, health: 0 };
  const hoodScoped = action === "canvass" || action === "town_hall";
  const route = routeFor(pocket, { action, lateNight: useLate, neighborhood: hoodScoped ? hood : null });
  const hoodIndex = o.town.neighborhoods.findIndex((h) => h.id === hood);
  const pct = (v: number) => Math.round(v * 100);

  let act: ActionDto | null;
  switch (action) {
    case "canvass":
      act = issue ? { type: "canvass", neighborhood: hood, issue } : { type: "canvass", neighborhood: hood };
      break;
    case "town_hall":
      act = { type: "town_hall", neighborhood: hood, policy: bill };
      break;
    case "press":
      act = { type: "press", policy: bill };
      break;
    case "fundraise":
      act = { type: "fundraise" };
      break;
    case "rest":
      act = { type: "rest" };
      break;
    case "advertise":
      act = { type: "advertise" };
      break;
    case "lobby":
    case "trade_favor":
      act = lobbyBill && target ? { type: action, politician: target.id, policy: lobbyBill.id } : null;
      break;
    case "take_bribe":
    case "dip_funds":
    case "leak_file":
      act = { type: action };
      break;
  }
  const cmd: CommandDto | null = act ? { type: "act", action: act, late_night: useLate } : null;
  const enough = (action === "advertise" ? p.ads_left > 0 && p.money >= p.ad_cost : useLate ? p.late_night_available : p.slots_left >= cost.slots) && !shady?.unavailable_key;
  const exactOdds = lobbyBill?.whip_exact ? lobbyBill.whip.find((w) => w.politician === target?.id) : undefined;

  return (
    <div className="action-form" data-testid={`form-${action}`}>
      {hoodScoped && fixedHood == null && (
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
      {action === "canvass" && <WorryPicker view={canvass} issue={issue} onChange={setIssue} />}
      {(action === "town_hall" || action === "press") && (
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
      {(action === "lobby" || action === "trade_favor") &&
        (pending.length === 0 ? (
          <p className="muted small">{tr.t("ui-no-pending")}</p>
        ) : (
          <>
            <label className="field">
              <span>{tr.t("ui-select-bill")}</span>
              <select className="input" value={lobbyBill?.id ?? ""} onChange={(e) => setBill(Number(e.target.value))}>
                {pending.map((b) => (
                  <option key={b.id} value={b.id}>
                    {tr.t(b.name_key)}
                  </option>
                ))}
              </select>
            </label>
            <label className="field">
              <span>{tr.t("ui-colleague")}</span>
              <select className="input" value={target?.id ?? ""} onChange={(e) => setMember(Number(e.target.value))}>
                {colleagues.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.name === "" ? `${tr.t(m.role_key)}${m.ward_number !== null ? ` · ${tr.t("ui-ward", { n: m.ward_number })}` : ""}` : m.name}
                    {m.favors < 0 ? ` · ${tr.t("ui-owe", { n: -m.favors })}` : ""}
                  </option>
                ))}
              </select>
            </label>
          </>
        ))}
      {/* Every option shows what it costs and what it risks before you commit (DEC-073). */}
      <p className="tradeoff" data-testid="tradeoff">
        {action === "advertise" ? (
          tr.t("tradeoff-advertise", { cost: formatMoney(p.ad_cost) })
        ) : (
          <>
            {useLate ? tr.t("ui-cost-late", { health: Math.max(0, cost.health) + 8 }) : tr.t("ui-cost-line", { slots: cost.slots, health: Math.max(0, cost.health) })} {tr.t(`tradeoff-${action.replace(/_/g, "-")}`)}
          </>
        )}
        {action === "town_hall" && <strong> {tr.t("ui-risk-hostile", { pct: pct(o.town.town_hall_risk[hoodIndex] ?? 0) })}</strong>}
        {action === "press" && <strong> {tr.t("ui-risk-gaffe", { pct: pct(p.press_gaffe_risk) })}</strong>}
        {action === "fundraise" && useLate && <strong> {tr.t("ui-risk-late-money")}</strong>}
        {(action === "lobby" || action === "trade_favor") && useLate && <strong> {tr.t("ui-risk-late-deal")}</strong>}
        {action === "lobby" && (exactOdds ? <strong> {tr.t("ui-odds", { pct: pct(useLate ? exactOdds.lobby_odds_late : exactOdds.lobby_odds) })}</strong> : <span> {tr.t("ui-odds-in-chamber")}</span>)}
      </p>
      {/* DEC-073: the chance it comes out, before you commit (a coarse band). */}
      {shady && (
        <p className="tradeoff risk-line" data-testid="risk-line">
          {shady.money > 0 && <>{tr.t("ui-shady-pays", { amount: formatMoney(shady.money) })} </>}
          {shady.money < 0 && <>{tr.t("ui-shady-costs", { amount: formatMoney(-shady.money) })} </>}
          {shady.health > 0 && <>{tr.t("ui-shady-heals", { n: Math.round(shady.health) })} </>}
          <strong>{tr.t("ui-risk-exposure", { risk: tr.t(`risk-${shady.risk}`), severity: tr.t(`severity-${shady.severity}`) })}</strong>
          {shady.unavailable_key && <span className="muted"> {tr.t(shady.unavailable_key)}</span>}
        </p>
      )}
      {lateAllowed && !lateOnly && !LATE_ONLY.has(action) && (
        <label className="check">
          <input type="checkbox" checked={late} disabled={!p.late_night_available} onChange={(e) => setLate(e.target.checked)} />
          {tr.t("ui-late-night-meeting")}
        </label>
      )}
      {showRoute && route?.to && (
        <p className="route small" data-testid="route">
          {route.here ? tr.t("ui-route-here") : tr.t("ui-route-to", { place: placeName(tr, route.to) })}
        </p>
      )}
      {landed && (
        <p className="canvass-landed" role="status" data-testid="canvass-landed">
          {tr.t(`ui-canvass-landed-${landed.fit}`, { issue: tr.t(landed.nameKey) })}
        </p>
      )}
      <button
        type="button"
        className="btn btn-primary btn-block"
        disabled={busy || !enough || !cmd}
        onClick={() => {
          if (!cmd) return;
          const worry = canvass?.worries.find((w) => w.id === issue) ?? null;
          setLanded(null);
          void perform(cmd).then(() => {
            // How the worry went down on the doorsteps (the engine's word for it, in the outcome).
            const t = useGame.getState().toast;
            const fit = t?.key === "outcome-canvass" ? t.args.fit : undefined;
            if (worry && fit) setLanded({ fit, nameKey: worry.name_key });
          });
        }}
      >
        {action === "advertise" ? tr.t("ui-buy-ads", { cost: formatMoney(p.ad_cost), left: p.ads_left }) : route?.here ? tr.t("ui-do-here") : tr.t("ui-do")}
      </button>
    </div>
  );
}

/** The agenda kinds, in the order the pocket lists them. */
export const AGENDA: FormAction[] = ["canvass", "town_hall", "press", "fundraise", "lobby", "trade_favor", "rest", "advertise", "take_bribe", "dip_funds", "leak_file"];
