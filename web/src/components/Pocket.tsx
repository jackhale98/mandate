import { useEffect, useRef, useState } from "react";
import type { PocketView } from "../engine/protocol/PocketView";
import type { PocketBillView } from "../engine/protocol/PocketBillView";
import { useGame } from "../state/gameStore";
import { routeFor } from "../state/routing";
import { formatDate, formatMoney } from "../i18n/format";
import { StatTile } from "./StatTile";
import { ActionForm, AGENDA, type FormAction } from "./ActionForm";
import { placeIcon, placeName } from "../places/labels";
import { useSheetClose } from "./useSheetClose";
import { capital, deadlineText } from "./DecisionSheet";
import { Legal } from "./Newspaper";
import { PocketTabs, pocketPanelId, pocketTabId, type TabBadge } from "./PocketTabs";
import { TownTab } from "./TownCard";
import { PlaceList } from "./PlaceList";
import { PeopleList } from "./PeopleList";
import { say } from "./townMapText";
import type { PocketTab } from "../state/devicePrefs";
import type { Translator } from "../i18n/fluent";

const TRACKS = ["record", "viability", "standing", "health", "legal_jeopardy", "threat"] as const;
/** Tracks where a high value is bad (shown with a "risk" bar plus the text label). */
const RISK = new Set<string>(["legal_jeopardy", "threat"]);

function Tracks({ p }: { p: PocketView }) {
  const tr = useGame((s) => s.tr);
  return (
    <ul className="bars">
      {TRACKS.map((k) => {
        const v = Math.round(k === "record" ? p.record : p.tracks[k]);
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
  );
}

function PocketBill({ b }: { b: PocketBillView }) {
  const tr = useGame((s) => s.tr);
  const pocket = useGame((s) => s.pocket);
  const visit = useGame((s) => s.visit);
  const route = routeFor(pocket, { action: b.status === "draft" ? "draft" : "vote", lateNight: false, neighborhood: null });
  return (
    <li className="pocket-bill">
      <div>
        <strong>{tr.t(b.name_key)}</strong>
        <div className="muted small">
          {tr.t(`status-${b.status}`)}
          {b.status === "under_consideration" && <> · {tr.t("ui-whip-about", { yes: b.about_yes, seats: b.seats, needed: b.needed })}</>}
          {b.next_vote && b.next_stage_key && <> · {tr.t("ui-next-vote", { stage: tr.t(b.next_stage_key), date: formatDate(tr, b.next_vote) })}</>}
          {b.can_vote && <> · {tr.t("ui-awaits-your-vote")}</>}
        </div>
      </div>
      {route?.to && (
        <button type="button" className="btn btn-sm btn-ghost" onClick={() => route.to && void visit(route.to.location)}>
          {placeIcon(route.to.kind_key)} {placeName(tr, route.to)}
        </button>
      )}
    </li>
  );
}

/** Decisions waiting for the player (DEC-079: always listed and answerable here). */
function Decisions({ p }: { p: PocketView }) {
  const tr = useGame((s) => s.tr);
  const openDecision = useGame((s) => s.openDecision);
  return (
    <section className="card" data-testid="pocket-decisions">
      <h2>{tr.t("ui-decisions")}</h2>
      {p.decisions.length === 0 ? (
        <p className="muted small">{tr.t("ui-decisions-none")}</p>
      ) : (
        <ul className="plain-list">
          {p.decisions.map((d) => (
            <li key={d.id} className="pocket-bill">
              <div>
                <strong>{capital(tr.event(d.title_key, d.args))}</strong>
                <div className="muted small">
                  {tr.t(`ui-decision-kind-${d.kind}`)} · {deadlineText(tr, d)}
                </div>
              </div>
              <button type="button" className="btn btn-sm" onClick={() => void openDecision(d.id)} data-testid={`open-decision-${d.id}`}>
                {tr.t("ui-choose")}
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

/** The always-available agenda: every action, auto-routed to where it is done (DEC-079). */
function Agenda({ p }: { p: PocketView }) {
  const tr = useGame((s) => s.tr);
  const visit = useGame((s) => s.visit);
  // Nothing is chosen until the player picks an action.
  const [kind, setKind] = useState<FormAction | null>(null);
  const kinds = AGENDA.filter((k) => k !== "advertise" || p.in_campaign);
  const go = (action: string) => {
    const to = routeFor(p, { action, lateNight: false, neighborhood: null })?.to;
    if (to) void visit(to.location);
  };
  return (
    <section className="card" data-testid="agenda">
      <h2>{tr.t("ui-agenda")}</h2>
      <p className="hint">{tr.t("ui-agenda-help")}</p>
      <div className="segmented segmented-sm agenda-kinds" role="radiogroup" aria-label={tr.t("ui-agenda")}>
        {kinds.map((k) => (
          <label key={k} className={kind === k ? "seg seg-on" : "seg"}>
            <input type="radio" name="agenda" checked={kind === k} onChange={() => setKind(k)} />
            {tr.t(`action-${k.replace(/_/g, "-")}`)}
          </label>
        ))}
      </div>
      {kind ? (
        <ActionForm key={kind} action={kind} showRoute />
      ) : (
        <p className="muted small" data-testid="agenda-choose">
          {tr.has("ui-agenda-choose") ? tr.t("ui-agenda-choose") : "Choose an action."}
        </p>
      )}
      <h3 className="sub">{tr.t("ui-agenda-places")}</h3>
      <div className="bill-actions">
        <button type="button" className="btn btn-sm" onClick={() => go("draft")}>
          {tr.t("ui-agenda-draft")}
        </button>
        <button type="button" className="btn btn-sm" onClick={() => go("vote")}>
          {tr.t("ui-agenda-vote")}
        </button>
        <button type="button" className="btn btn-sm" onClick={() => go("file")}>
          {tr.t("ui-agenda-file")}
        </button>
      </div>
    </section>
  );
}

/** Today: the turn, what is waiting (decisions, alerts, votes), the agenda and your standing. */
function TodayTab({ p }: { p: PocketView }) {
  const tr = useGame((s) => s.tr);
  return (
    <>
      <div className="stats stats-3">
        <StatTile label={tr.t("ui-slots-short")} value={`${p.slots_left}/${p.slot_cap}`} />
        <StatTile label={tr.t("ui-money")} value={formatMoney(p.money)} />
        <StatTile label={tr.t("ui-mandate")} value={String(Math.round(p.mandate))} />
      </div>
      <p className="small" data-testid="pocket-location">
        {tr.t("ui-you-are-at", { place: placeName(tr, p.location) })}
      </p>
      {p.alerts.length > 0 && (
        <ul className="alerts" aria-label={tr.t("ui-alerts")} data-testid="pocket-alerts">
          {p.alerts.map((a, i) => (
            <li key={i} className="note">
              {tr.event(a.key, a.args)}
            </li>
          ))}
        </ul>
      )}
      <Decisions p={p} />
      {p.legal && <Legal l={p.legal} />}
      <Agenda p={p} />
      <h3 className="sub">{tr.t("ui-your-bills")}</h3>
      {p.bills.length === 0 ? (
        <p className="muted small">{tr.t("ui-no-bills")}</p>
      ) : (
        <ul className="plain-list" data-testid="pocket-bills">
          {p.bills.map((b) => (
            <PocketBill key={b.id} b={b} />
          ))}
        </ul>
      )}
      <h3 className="sub">{tr.t("ui-next-election")}</h3>
      <p className="small" data-testid="pocket-election">
        {p.next_election ? formatDate(tr, p.next_election) : "—"}
        {p.days_to_election !== null && <span className="muted"> · {tr.t("ui-days-left", { n: p.days_to_election })}</span>}
      </p>
      <h3 className="sub">{tr.t("ui-tracks")}</h3>
      <Tracks p={p} />
    </>
  );
}

/** What waits on each tab, counted (the tabs' badges and the pocket button's). */
export function pocketBadges(tr: Translator, p: PocketView): Partial<Record<PocketTab, TabBadge>> {
  const today = [p.decisions.length > 0 ? tr.t("ui-decision-count", { n: p.decisions.length }) : null, p.alerts.length > 0 ? tr.t("ui-alert-count", { n: p.alerts.length }) : null].filter((x): x is string => x !== null);
  return {
    today: { n: p.decisions.length + p.alerts.length, text: today.join(", ") },
    people: { n: p.notebook_new, text: say(tr, "ui-notebook-new-count", { n: p.notebook_new }) },
  };
}

/**
 * The pocket (phone and notebook): always available, coarse, never the only
 * path to anything vital (DEC-079 rule 3). Tabs keep it short (DEC-088 main
 * screen): Today, Town (the town card), Places (every place, no walking) and
 * People. A bottom sheet on phones; on wide screens a side panel that can stay
 * docked open beside the town.
 */
export function Pocket() {
  const tr = useGame((s) => s.tr);
  const p = useGame((s) => s.pocket);
  const o = useGame((s) => s.office);
  const open = useGame((s) => s.pocketOpen);
  const setOpen = useGame((s) => s.setPocketOpen);
  const tab = useGame((s) => s.pocketTab);
  const setTab = useGame((s) => s.setPocketTab);
  const docked = useGame((s) => s.pocketDocked);
  const setDocked = useGame((s) => s.setPocketDocked);
  const focusTick = useGame((s) => s.pocketFocus);
  const setView = useGame((s) => s.setView);
  const busy = useGame((s) => s.busy);
  const saveAndQuit = useGame((s) => s.saveAndQuit);
  const openStory = useGame((s) => s.openStory);
  const [closing, requestClose, sheetRef] = useSheetClose(() => setOpen(false));
  const panelRef = useRef<HTMLDivElement>(null);
  const shown = open && p !== null;
  // "Skip to Places" (and anything else that asks) moves the keyboard into the open tab.
  useEffect(() => {
    if (!shown || focusTick === 0) return;
    const id = requestAnimationFrame(() => panelRef.current?.focus());
    return () => cancelAnimationFrame(id);
  }, [focusTick, shown]);
  if (!shown) return null;
  const cls = ["sheet", "pocket", docked ? "pocket-docked" : "", closing ? "sheet-closing" : ""].filter(Boolean).join(" ");
  return (
    <>
      <div className={closing ? "sheet-scrim sheet-closing" : "sheet-scrim"} onClick={requestClose} aria-hidden="true" />
      <aside ref={sheetRef} className={cls} aria-label={tr.t("ui-pocket")} data-testid="pocket">
        <div className="sheet-head pocket-head">
          <div className="pocket-head-row">
            <div>
              <h2>{tr.t("ui-pocket")}</h2>
              {o && (
                <p className="muted small">
                  {o.player.name} · {tr.t(o.player.role_key)} · {o.player.municipality}
                </p>
              )}
            </div>
            <div className="sheet-tools">
              <button
                type="button"
                className="btn btn-ghost pocket-dock"
                aria-pressed={docked}
                onClick={() => setDocked(!docked)}
                aria-label={say(tr, "ui-pocket-dock")}
                title={say(tr, docked ? "ui-pocket-undock-hint" : "ui-pocket-dock-hint")}
                data-testid="pocket-dock"
              >
                <span aria-hidden="true">{docked ? "◧" : "▭"}</span>
              </button>
              <button type="button" className="btn btn-ghost" onClick={requestClose} aria-label={tr.t("ui-close")}>
                ✕
              </button>
            </div>
          </div>
          <PocketTabs tab={tab} onChange={setTab} badges={pocketBadges(tr, p)} />
        </div>
        <div className="sheet-body pocket-body">
          <div ref={panelRef} role="tabpanel" id={pocketPanelId(tab)} aria-labelledby={pocketTabId(tab)} tabIndex={-1} className="pocket-panel" data-testid={`pocket-panel-${tab}`}>
            {tab === "today" ? <TodayTab p={p} /> : tab === "town" ? <TownTab /> : tab === "places" ? <PlaceList /> : <PeopleList />}
          </div>
          <nav className="pocket-links" aria-label={tr.t("ui-pocket-links")}>
            <button type="button" className="btn" onClick={() => setView("map")}>
              {tr.t("ui-map")}
            </button>
            <button type="button" className="btn" onClick={() => setView("overview")}>
              {tr.t("ui-overview")}
            </button>
            <button type="button" className="btn" onClick={() => void openStory(true)} data-testid="open-story">
              {tr.t("ui-story-so-far")}
            </button>
            <button type="button" className="btn btn-ghost" disabled={busy} onClick={() => void saveAndQuit()}>
              {tr.t("ui-save-quit")}
            </button>
          </nav>
        </div>
      </aside>
    </>
  );
}
