import { Fragment, useLayoutEffect, useRef } from "react";
import type { TownDataView } from "../engine/protocol/TownDataView";
import type { BudgetPapersView } from "../engine/protocol/BudgetPapersView";
import type { InternalPollView } from "../engine/protocol/InternalPollView";
import type { OverheardView } from "../engine/protocol/OverheardView";
import type { PlacePanelView } from "../engine/protocol/PlacePanelView";
import { useGame } from "../state/gameStore";
import { formatCount, formatMoney, formatPct } from "../i18n/format";
import { actionKey, placeIcon, placeName } from "../places/labels";
import { StatTile } from "../components/StatTile";
import { ActionForm, type FormAction } from "../components/ActionForm";
import { ChamberWhip } from "../components/Chamber";
import { Catalog, MyBills, RecentActions, TownOpinion } from "../components/Desk";
import { useSheetClose } from "../components/useSheetClose";
import { Calendar, CampaignHub } from "../components/CampaignCards";
import { PartyChip } from "../components/ElectionCharts";
import { roleText } from "../components/CharacterSheet";
import { personName } from "../components/Knowing";
import { capital } from "../components/DecisionSheet";
import { Newspaper } from "../components/Newspaper";
import { TalkButton } from "../components/talk/TalkButton";

/** Decisions whose scene happens here (DEC-079: here, the conversation). */
function WaitingHere({ v }: { v: PlacePanelView }) {
  const tr = useGame((s) => s.tr);
  const openDecision = useGame((s) => s.openDecision);
  return (
    <section className="card" data-testid="decisions-here">
      <h2>{tr.t("ui-waiting-here")}</h2>
      <ul className="plain-list">
        {v.decisions_here.map((d) => (
          <li key={d.id}>
            <button type="button" className="link-btn" onClick={() => void openDecision(d.id)}>
              {capital(tr.event(d.title_key, d.args))}
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}

function signed(v: number): string {
  const n = Math.round(v * 100);
  return n > 0 ? `+${n}` : String(n);
}

function TownData({ d }: { d: TownDataView }) {
  const tr = useGame((s) => s.tr);
  const hist = d.unemployment_history;
  const yearAgo = hist.length >= 12 ? hist[0] : undefined;
  const delta = yearAgo === undefined ? undefined : d.unemployment - yearAgo;
  const maxJobs = Math.max(1, ...d.sectors.map((x) => x.jobs));
  return (
    <section className="card" data-testid="town-data">
      <h2>{tr.t("service-town_data_detailed")}</h2>
      <div className="stats stats-2">
        <StatTile
          label={tr.t("ui-unemployment")}
          value={formatPct(d.unemployment, 1)}
          delta={delta === undefined ? undefined : { text: `${delta >= 0 ? "+" : "−"}${formatPct(Math.abs(delta), 1)}`, good: Math.abs(delta) < 1e-4 ? null : delta < 0 }}
          trend={hist}
          formatPoint={(v, ago) => `${formatPct(v, 1)} · ${tr.t("ui-months-ago", { n: ago })}`}
          trendLabel={tr.t("ui-trend-12m", { what: tr.t("ui-unemployment") })}
        />
        <StatTile label={tr.t("ui-population")} value={formatCount(d.population)} />
        <StatTile label={tr.t("ui-typical-income")} value={formatMoney(d.typical_income)} />
        <StatTile label={tr.t("ui-rent")} value={formatMoney(d.monthly_rent)} />
        <StatTile label={tr.t("ui-home-price")} value={formatMoney(d.home_price)} />
        <StatTile label={tr.t("ui-vacancy")} value={formatPct(d.vacancy, 1)} />
      </div>
      {d.communities.length > 1 && (
        <p className="muted" data-testid="town-communities">
          {tr.t("ui-communities")}: {d.communities.map((c) => `${tr.t(c.name_key)} ${formatPct(c.share, 0)}`).join(" · ")}
        </p>
      )}
      <h3 className="sub">{tr.t("ui-by-neighbourhood")}</h3>
      <div className="table-scroll">
        <table className="data-table">
          <thead>
            <tr>
              <th scope="col">{tr.t("ui-neighbourhood")}</th>
              <th scope="col">{tr.t("ui-population")}</th>
              <th scope="col">{tr.t("ui-know-you")}</th>
              <th scope="col">{tr.t("ui-approval")}</th>
              <th scope="col">{tr.t("ui-gov-approval")}</th>
              <th scope="col">{tr.t("ui-top-concern")}</th>
            </tr>
          </thead>
          <tbody>
            {d.neighborhoods.map((h) => (
              <tr key={h.id}>
                <th scope="row">{h.name}</th>
                <td className="num">{formatCount(h.population)}</td>
                <td className="num">{Math.round(h.player_awareness * 100)}%</td>
                <td className="num">{signed(h.player_approval)}</td>
                <td className="num">{signed(h.gov_approval)}</td>
                <td>{h.top_concern_key ? tr.t(h.top_concern_key) : "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <h3 className="sub">{tr.t("ui-jobs-by-sector")}</h3>
      <ul className="meters">
        {d.sectors.map((x) => (
          <li key={x.id}>
            <span>{tr.t(x.name_key)}</span>
            <span className="meter" aria-hidden="true">
              <span className="meter-fill" style={{ width: `${(x.jobs / maxJobs) * 100}%` }} />
            </span>
            <span className="num">{formatCount(x.jobs)}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}

function Budget({ b }: { b: BudgetPapersView }) {
  const tr = useGame((s) => s.tr);
  return (
    <section className="card" data-testid="budget-papers">
      <h2>{tr.t("service-budget_papers")}</h2>
      <div className="stats stats-3">
        <StatTile label={tr.t("ui-budget")} value={formatMoney(b.budget)} />
        <StatTile label={tr.t("ui-local-debt")} value={formatMoney(b.debt)} />
        <StatTile label={tr.t("ui-last-balance")} value={formatMoney(b.last_balance)} />
      </div>
      {b.programmes.length > 0 && (
        <>
          <h3 className="sub">{tr.t("ui-programmes")}</h3>
          <ul className="region-list">
            {b.programmes.map((p) => (
              <li key={p.id}>
                <span>{tr.t(p.name_key)}</span>
                <span className="num small">
                  {formatMoney(p.est_annual_cost)} · {tr.t("ui-funded", { pct: Math.round(p.funded_pct) })}
                </span>
              </li>
            ))}
          </ul>
        </>
      )}
    </section>
  );
}

function InternalPoll({ p }: { p: InternalPollView }) {
  const tr = useGame((s) => s.tr);
  const c = useGame((s) => s.campaign);
  return (
    <section className="card" data-testid="internal-poll">
      <h2>{tr.t("service-internal_polls")}</h2>
      {p.race && c && (
        <ul className="candidate-list">
          {p.race.candidates.map((cd) => (
            <li key={cd.name}>
              <span>
                <strong>{cd.name}</strong>
                {cd.is_player && <span className="chip chip-muted"> {tr.t("ui-you")}</span>}
                <br />
                <PartyChip parties={c.parties} party={cd.party} />
              </span>
              <span className="num big">{Math.round(cd.share * 100)}%</span>
            </li>
          ))}
        </ul>
      )}
      <div className="stats stats-2">
        <StatTile label={tr.t("ui-know-you")} value={`${Math.round(p.ward_awareness * 100)}%`} />
        <StatTile label={tr.t("ui-approval")} value={signed(p.ward_approval)} />
      </div>
      <p className="hint">{tr.t("ui-internal-poll-note", { pts: Math.round(p.margin * 100) })}</p>
    </section>
  );
}

function Overheard({ o }: { o: OverheardView }) {
  const tr = useGame((s) => s.tr);
  return (
    <section className="card" data-testid="overheard">
      <h2>{tr.t("service-overheard")}</h2>
      <p className="muted small">{tr.t("ui-overheard-intro", { hood: o.neighborhood_name })}</p>
      <ul className="plain quotes">
        {o.lines.map((l) => (
          <li key={l.id}>{tr.event(l.key, l.args)}</li>
        ))}
      </ul>
    </section>
  );
}

/** Who is here (DEC-018): characters who work or sit here, and passers-by to meet. */
function People({ v }: { v: PlacePanelView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const openCharacter = useGame((s) => s.openCharacter);
  const meet = useGame((s) => s.meet);
  // The chamber's members are in the whip list.
  if (v.services.includes("whip_exact") && v.here) return null;
  if (v.people.length === 0 && v.location.kind_key === "home") return null;
  return (
    <section className="card" data-testid="people-here">
      <h2>{tr.t("ui-people-here")}</h2>
      {v.people.length === 0 ? (
        <p className="muted small">{tr.t("ui-people-here-none")}</p>
      ) : (
        <ul className="people">
          {v.people.map((p, i) => (
            <li key={p.id ?? `a${p.ambient_slot ?? i}`}>
              {p.id !== null ? (
                <div className="person-row">
                  <button type="button" className="row-btn person" disabled={busy} onClick={() => p.id !== null && void openCharacter(p.id)} data-testid="person">
                    <span>
                      <strong>{personName(tr, p.name)}</strong>
                      <span className="muted small"> · {roleText(tr, p)}</span>
                    </span>
                    <span className="muted small">{tr.t(`tier-${p.tier}`)}</span>
                  </button>
                  {/* Talk to them (DEC-093): here now, or the panel says where they are. */}
                  <TalkButton person={p.id} name={p.name} />
                </div>
              ) : (
                <div className="row-btn person person-ambient">
                  <span>
                    <strong>{personName(tr, p.name)}</strong>
                    <span className="muted small"> · {roleText(tr, p)}</span>
                  </span>
                  {v.here && p.ambient_slot !== null && (
                    <span className="sheet-tools">
                      <button type="button" className="btn btn-sm" disabled={busy} onClick={() => p.ambient_slot !== null && void meet(p.ambient_slot)} data-testid="meet">
                        {tr.t("ui-meet")}
                      </button>
                      {/* Say hello (free), then talk. */}
                      <TalkButton person={null} slot={p.ambient_slot} name={p.name} />
                    </span>
                  )}
                </div>
              )}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

/** Generic actions a place offers, each with its costs and risks. */
function Actions({ v, skip }: { v: PlacePanelView; skip: Set<string> }) {
  const tr = useGame((s) => s.tr);
  const forms = new Set<FormAction>(["canvass", "town_hall", "press", "fundraise", "rest", "advertise", "lobby", "trade_favor", "take_bribe", "dip_funds", "leak_file"]);
  const campaign = useGame((s) => s.campaign);
  // Late-only actions (an envelope at the diner) are listed by their late variant.
  const shown = v.actions.filter((a) => (!a.late_night || a.action === "take_bribe") && !skip.has(a.action) && forms.has(a.action as FormAction) && (a.action !== "advertise" || campaign?.in_campaign));
  if (shown.length === 0) return null;
  const lateHere = new Set(v.actions.filter((a) => a.late_night).map((a) => a.action));
  return (
    <>
      {shown.map((a) => (
        <section key={a.action} className="card" data-testid={`place-action-${a.action}`}>
          <h2>{tr.t(actionKey(a.action))}</h2>
          <ActionForm action={a.action as FormAction} hood={a.action === "canvass" || a.action === "town_hall" ? v.location.neighborhood : null} allowLate={lateHere.has(a.action)} />
        </section>
      ))}
    </>
  );
}

/** The place's verbs (places with purpose): each with its cost, whether it can be done now, and its risk (the engine's odds). */
function Verbs({ v }: { v: PlacePanelView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const perform = useGame((s) => s.perform);
  if (v.verbs.length === 0) return null;
  return (
    <section className="card" data-testid="place-verbs">
      <h2>{tr.t("ui-verbs")}</h2>
      <ul className="plain-list">
        {v.verbs.map((x) => (
          <li key={x.verb} data-testid={`verb-${x.verb}`}>
            <p className="tradeoff">
              <strong>{tr.t(x.name_key)}</strong> · {tr.t(x.halves === 1 ? "ui-verb-cost-half" : "ui-verb-cost-slot")}
              {x.risk !== null && <strong> {tr.t("ui-verb-risk", { pct: Math.round(x.risk * 100) })}</strong>}
              {x.blocked_key && <span className="muted"> {tr.t(x.blocked_key)}</span>}
            </p>
            <button
              type="button"
              className="btn btn-primary btn-block"
              disabled={busy || !x.available}
              onClick={() => void perform({ type: "act", action: { type: "visit", verb: x.verb }, late_night: false })}
            >
              {tr.t("ui-verb-done")}
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}

/** What can be done and learned at a place (bottom sheet on phones, side panel from 900px). */
export function PlacePanel() {
  const tr = useGame((s) => s.tr);
  const v = useGame((s) => s.place);
  const open = useGame((s) => s.openPlace);
  const close = useGame((s) => s.openPanel);
  const visit = useGame((s) => s.visit);
  const busy = useGame((s) => s.busy);
  const o = useGame((s) => s.office);
  const c = useGame((s) => s.campaign);
  const sheet = useRef<HTMLElement>(null);
  const [closing, requestClose, sheetRef] = useSheetClose(() => void close(null), sheet);
  const where = open ? JSON.stringify(open) : "";
  // A new place starts at the top of the sheet; the sheet itself stays (no replayed slide-in).
  useLayoutEffect(() => {
    if (sheet.current) sheet.current.scrollTop = 0;
  }, [where]);
  if (!open) return null;
  const has = (s: string) => v?.services.includes(s) ?? false;
  const offers = (a: string) => v?.actions.some((x) => x.action === a) ?? false;
  const chamber = has("whip_exact");
  // Lobbying in the chamber is done from the whip list.
  const skip = new Set<string>(chamber ? ["lobby", "trade_favor"] : []);
  return (
    <>
      <div className={closing ? "sheet-scrim sheet-closing" : "sheet-scrim"} onClick={requestClose} aria-hidden="true" />
    <aside
      ref={sheetRef}
      className={closing ? "sheet place-panel sheet-closing" : "sheet place-panel"}
      aria-labelledby="place-title"
      data-testid="place-panel"
    >
      {/* Keyed by location: a new place starts fresh. */}
      <Fragment key={where}>
      <div className="sheet-head">
        <div>
          <h2 id="place-title">
            <span aria-hidden="true">{v ? placeIcon(v.location.kind_key) : ""} </span>
            {v ? placeName(tr, v.location) : "…"}
          </h2>
          {v && (
            <p className="muted small">
              {v.location.name && v.location.kind_key !== "streets" ? `${tr.t(`place-${v.location.kind_key}`)} · ` : ""}
              {v.location.neighborhood_name}
            </p>
          )}
        </div>
        <div className="sheet-tools">
          {o && (
            <span className="chip" data-testid="slots">
              {tr.t("ui-slots", { left: o.player.slots_left })}
            </span>
          )}
          <button type="button" className="btn btn-ghost" onClick={requestClose} aria-label={tr.t("ui-close")}>
            ✕
          </button>
        </div>
      </div>
      {v && (
        <div className="sheet-body place-body">
          {!v.here ? (
            <div className="note">
              <p>{tr.t("ui-not-here")}</p>
              <button type="button" className="btn btn-primary" disabled={busy} onClick={() => void visit(v.location.location)}>
                {tr.t("ui-go-here")}
              </button>
            </div>
          ) : (
            <p className="small muted" data-testid="you-are-here-panel">
              <span aria-hidden="true">📍 </span>
              {tr.t("ui-you-are-here")}
            </p>
          )}
          {v.actions.length === 0 && v.services.length === 0 && v.verbs.length === 0 && <p className="muted">{tr.t("ui-nothing-here")}</p>}
          {chamber && v.here && <ChamberWhip />}
          {chamber && !v.here && <p className="note">{tr.t("ui-whip-in-chamber")}</p>}
          {offers("amend") && v.here && <MyBills statuses={["under_consideration"]} testId="chamber-bills" />}
          {offers("draft") && (
            <>
              <MyBills statuses={["draft"]} testId="bills" />
              <Catalog />
            </>
          )}
          {offers("file") && c && <Calendar c={c} />}
          {v.town_data && <TownData d={v.town_data} />}
          {has("town_data_detailed") && o && v.here && <TownOpinion o={o} />}
          {v.budget && <Budget b={v.budget} />}
          {v.internal_poll && <InternalPoll p={v.internal_poll} />}
          {has("internal_polls") && v.here && <CampaignHub />}
          {v.overheard && <Overheard o={v.overheard} />}
          {v.newspaper && <Newspaper n={v.newspaper} />}
          {v.decisions_here.length > 0 && <WaitingHere v={v} />}
          <People v={v} />
          <Verbs v={v} />
          <Actions v={v} skip={skip} />
          {offers("rest") && <RecentActions />}
        </div>
      )}
      </Fragment>
    </aside>
    </>
  );
}
