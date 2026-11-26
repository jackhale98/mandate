// The pocket (DEC-113, structure B): two sides under one switch. Today is for
// doing: one page with the turn's three tiles, where you are, what needs you,
// what is under way, and (folded) an action to take and your standing. The
// Notebook is for looking things up: one row of four tabs (People, Places,
// Town, Policies). Save and quit, the story so far, the maps and the rest are
// in the ⋯ menu at the top, never under a scroll. A bottom sheet on phones; on
// wide screens Today is a panel on the right (it can stay docked) and the
// Notebook sits open on the left beside it (owner's answer 4). Always
// available, coarse, never the only path to anything vital (DEC-079 rule 3).
// Every order, reason and count is the engine's (`PocketView.needs_you`,
// `arcs`, `next_up`, `needs_count`): the UI sorts and counts nothing.
import { useEffect, useRef, useState, type ReactNode } from "react";
import type { ArcSummaryView } from "../engine/protocol/ArcSummaryView";
import type { CalendarEventView } from "../engine/protocol/CalendarEventView";
import type { NeedView } from "../engine/protocol/NeedView";
import type { PocketView } from "../engine/protocol/PocketView";
import { useGame } from "../state/gameStore";
import { useReport } from "../state/reportStore";
import { routeFor } from "../state/routing";
import { isWide } from "../state/devicePrefs";
import { formatDate, formatMoney } from "../i18n/format";
import { ActionForm, AGENDA, type FormAction } from "./ActionForm";
import { TALK_VERBS, VerbTalk } from "./talk/VerbTalk";
import { placeName } from "../places/labels";
import { useSheetClose } from "./useSheetClose";
import { capital } from "./DecisionSheet";
import { Legal } from "./Newspaper";
import { PocketTabs, pocketPanelId, pocketTabId } from "./PocketTabs";
import { Notebook, NotebookTabs } from "./notebook/Notebook";
import { AskRow } from "./asks/Asks";
import { BillRow } from "./notebook/Bills";
import { Fold, Row, RowList, Section, When, shortDate, useFold } from "./notebook/rows";
import { useScene } from "./exchange/sceneStore";

const TRACKS = ["record", "viability", "standing", "health", "legal_jeopardy", "threat"] as const;
/** Tracks where a high value is bad (shown with a "risk" bar plus the text label). */
const RISK = new Set<string>(["legal_jeopardy", "threat"]);

/** Whether the window is wide (900 px and over), following resizes. */
export function useWide(): boolean {
  const [wide, setWide] = useState(isWide);
  useEffect(() => {
    const m = window.matchMedia?.("(min-width: 900px)");
    if (!m) return;
    const on = () => setWide(m.matches);
    m.addEventListener("change", on);
    return () => m.removeEventListener("change", on);
  }, []);
  return wide;
}

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

/** A verb had in conversation (DEC-114, B5), from the agenda: whom with and where (the engine's), then the talk panel. */
function AgendaTalk({ p, action }: { p: PocketView; action: string }) {
  // The day's variant first; the late one where only it exists (the bribe).
  const t = p.verb_talks.find((x) => x.action === action && !x.late_night) ?? p.verb_talks.find((x) => x.action === action) ?? null;
  return <VerbTalk action={action} person={t?.person} to={t?.to ?? null} here={t?.here ?? false} topic={t?.topic} />;
}

/** The always-available agenda: every action, auto-routed to where it is done (DEC-079). Folded under "Take an action". */
function Agenda({ p }: { p: PocketView }) {
  const tr = useGame((s) => s.tr);
  const visit = useGame((s) => s.visit);
  // Nothing is chosen until the player picks an action.
  const [kind, setKind] = useState<string | null>(null);
  const kinds = AGENDA.filter((k) => k !== "advertise" || p.in_campaign);
  const go = (action: string) => {
    const to = routeFor(p, { action, lateNight: false, neighborhood: null })?.to;
    if (to) void visit(to.location);
  };
  return (
    <div className="rw-pad">
      <div className="segmented segmented-sm agenda-kinds" role="radiogroup" aria-label={tr.t("ui-agenda")}>
        {kinds.map((k) => (
          <label key={k} className={kind === k ? "seg seg-on" : "seg"}>
            <input type="radio" name="agenda" checked={kind === k} onChange={() => setKind(k)} />
            {tr.t(`action-${k.replace(/_/g, "-")}`)}
          </label>
        ))}
      </div>
      {kind && TALK_VERBS.has(kind) ? (
        <AgendaTalk key={kind} p={p} action={kind} />
      ) : kind ? (
        <ActionForm key={kind} action={kind as FormAction} showRoute />
      ) : (
        <p className="muted small" data-testid="agenda-choose">
          {tr.t("ui-agenda-choose")}
        </p>
      )}
      <h4 className="sub">{tr.t("ui-agenda-places")}</h4>
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
    </div>
  );
}

/** "5 days" over "by 14 Mar". */
function dueWhen(tr: ReturnType<typeof useGame.getState>["tr"], days: number | null, due: PocketView["date"] | null): ReactNode {
  if (days === null || due === null) return null;
  return <When top={tr.t("ui-days-left", { n: days })} under={tr.t("ui-by-date", { date: shortDate(tr, due) })} />;
}

const NEED_MARK: Record<string, string> = { decision: "❗", ask_offer: "🤝", ask_due: "📌", vote: "🗳️", beat: "⏳", scene: "🎤" };

/**
 * One thing that needs the player (Today's "Needs you"), in the engine's
 * order. A decision goes straight to its sheet ("Choose"); an ask opens in
 * place so its price is read before it is answered; a vote opens to where it
 * is cast; an arc's beat opens the arc's sheet; a scene (DEC-112,
 * EXCHANGE_SCENES §7) opens the scene's screen, as a decision opens its sheet.
 */
function NeedRow({ n, p }: { n: NeedView; p: PocketView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const openDecision = useGame((s) => s.openDecision);
  const openArc = useGame((s) => s.openArc);
  const visit = useGame((s) => s.visit);
  const arc = n.arc !== null ? p.arcs.find((a) => a.id === n.arc) : undefined;
  const arcTitle = arc ? tr.event(arc.title_key, arc.args) : null;
  const t = n.target;
  if (t.type === "ask") {
    const a = p.asks.find((x) => x.id === t.id);
    if (a) return <AskRow a={a} mark={NEED_MARK[n.kind]} />;
  }
  const title = capital(tr.event(n.title_key, n.args));
  if (t.type === "decision") {
    const id = t.id;
    // The deadline goes in the line: the title keeps the room "Choose" leaves it.
    const sub = [tr.t("ui-need-decision"), arcTitle, n.days_left !== null ? tr.t("ui-days-left", { n: n.days_left }) : null].filter(Boolean).join(" · ");
    return (
      <Row
        mark={NEED_MARK.decision}
        title={title}
        sub={sub}
        label={`${title}. ${sub}`}
        onActivate={() => void openDecision(id)}
        end={
          <>
            <button type="button" className="btn btn-sm btn-primary rw-go" tabIndex={-1} disabled={busy} aria-label={tr.t("ui-choose-label", { what: title })} onClick={() => void openDecision(id)} data-testid={`open-decision-${id}`}>
              {tr.t("ui-choose")}
            </button>
          </>
        }
        li={{ "data-testid": `need-decision-${id}`, "data-need": n.kind }}
      />
    );
  }
  if (t.type === "scene") {
    const { arc: run, beat } = t;
    const sub = [tr.t("ui-need-scene"), n.days_left !== null ? tr.t("ui-days-left", { n: n.days_left }) : null].filter(Boolean).join(" · ");
    const open = () => void useScene.getState().openScene(run, beat);
    return (
      <Row
        mark={NEED_MARK.scene}
        title={title}
        sub={sub}
        label={`${title}. ${sub}`}
        onActivate={open}
        end={
          <button type="button" className="btn btn-sm btn-primary rw-go" tabIndex={-1} disabled={busy} aria-label={`${tr.t("ui-go")}: ${title}`} onClick={open} data-testid={`open-scene-${run}`}>
            {tr.t("ui-go")}
          </button>
        }
        li={{ "data-testid": `need-scene-${run}`, "data-need": n.kind }}
        mainTestId={`need-scene-open-${run}`}
      />
    );
  }
  if (t.type === "bill") {
    const route = routeFor(p, { action: "vote", lateNight: false, neighborhood: null });
    const where = route?.to ? placeName(tr, route.to) : null;
    const sub = [tr.t("ui-need-vote"), where].filter(Boolean).join(" · ");
    return (
      <Row
        mark={NEED_MARK.vote}
        title={title}
        sub={sub}
        label={`${title}. ${sub}`}
        onActivate={() => route?.to && void visit(route.to.location)}
        end={dueWhen(tr, n.days_left, n.due)}
        li={{ "data-testid": `need-vote-${t.id}`, "data-need": n.kind }}
      />
    );
  }
  // An arc's beat in its last turn: its sheet says what can be done.
  const arcId = t.type === "beat" ? t.arc : n.arc;
  return (
    <Row
      mark={NEED_MARK[n.kind] ?? "⏳"}
      title={title}
      sub={arcTitle ?? undefined}
      label={[title, arcTitle].filter(Boolean).join(". ")}
      onActivate={() => arcId !== null && void openArc(arcId)}
      end={dueWhen(tr, n.days_left, n.due)}
      li={{ "data-testid": `need-beat-${arcId ?? 0}`, "data-need": n.kind }}
    />
  );
}

/** A running arc as a row (A3): its name and where, its stage in a line, how many things are the player's to do, its clock. Opens its sheet. */
export function ArcRow({ a }: { a: ArcSummaryView }) {
  const tr = useGame((s) => s.tr);
  const openArc = useGame((s) => s.openArc);
  const title = capital(tr.event(a.title_key, a.args));
  const stage = tr.event(a.stage.key, a.stage.args);
  const todo = a.open_beats > 0 ? tr.t("ui-arc-to-do", { n: a.open_beats }) : null;
  return (
    <Row
      mark="⏳"
      title={title}
      role={a.place}
      sub={stage}
      label={[title, a.place, stage, todo].filter(Boolean).join(". ")}
      onActivate={() => void openArc(a.id)}
      end={todo || a.clock ? <When top={todo ?? (a.clock ? tr.t("ui-days-left", { n: a.clock.days_left }) : "")} under={todo && a.clock ? tr.t("ui-days-left", { n: a.clock.days_left }) : null} /> : undefined}
      li={{ "data-testid": `arc-row-${a.id}`, "data-arc": String(a.id) }}
      mainTestId={`open-arc-${a.id}`}
    />
  );
}

/** What is under way: running arcs, accepted asks and promises (not already waiting above), the player's bills. Past nine rows with "Needs you", its tail folds (the engine's order decides which show). */
function UnderWay({ p }: { p: PocketView }) {
  const tr = useGame((s) => s.tr);
  const [all, setAll] = useState(false);
  const waiting = new Set(p.needs_you.flatMap((n) => (n.target.type === "ask" ? [n.target.id] : [])));
  const rows: ReactNode[] = [
    ...p.arcs.map((a) => <ArcRow key={`arc-${a.id}`} a={a} />),
    ...p.asks.filter((a) => a.status === "accepted" && !waiting.has(a.id)).map((a) => <AskRow key={`ask-${a.id}`} a={a} />),
    ...p.bills.map((b) => <BillRow key={`bill-${b.id}`} b={b} p={p} testId={`bill-${b.id}`} />),
  ];
  if (rows.length === 0 && p.asks_recent.length === 0) return null;
  const room = Math.max(3, 9 - p.needs_you.length);
  const shown = all ? rows : rows.slice(0, room);
  const more = rows.length - shown.length;
  return (
    <Section id="today-under-way" title={tr.t("ui-under-way")} testId="pocket-under-way">
      {rows.length > 0 && <RowList>{shown}</RowList>}
      {more > 0 && (
        <button type="button" className="btn btn-ghost rw-more-btn" onClick={() => setAll(true)} data-testid="under-way-more">
          {tr.t("ui-and-more", { n: more })}
        </button>
      )}
      {p.asks_recent.length > 0 && (
        <Fold id="today-settled" title={tr.t("ui-settled-lately")} count={p.asks_recent.length}>
          <RowList testId="asks-recent">
            {p.asks_recent.map((a) => (
              <AskRow key={a.id} a={a} />
            ))}
          </RowList>
        </Fold>
      )}
    </Section>
  );
}

/** Nothing waiting: say so, and name the next dated thing (the engine's `next_up`). */
function NextUp({ p }: { p: PocketView }) {
  const tr = useGame((s) => s.tr);
  const openArc = useGame((s) => s.openArc);
  const n = p.next_up;
  return (
    <div className="today-next" data-testid="pocket-next-up">
      <p className="today-empty">{tr.t("ui-nothing-waiting")}</p>
      {n && (
        <RowList>
          <Row
            mark="🔔"
            title={n.title ? capital(tr.event(n.title.key, n.title.args)) : tr.t(n.key, n.args)}
            sub={n.title ? tr.t(n.key, n.args) : undefined}
            end={<When top={shortDate(tr, n.date)} under={tr.t("ui-days-left", { n: n.days_left })} />}
            onActivate={() => n.arc !== null && void openArc(n.arc)}
            disabled={n.arc === null}
          />
        </RowList>
      )}
    </div>
  );
}

const CAL_MARK = "🔔";

/** One event of the fortnight (DEC-114 §5.5): its name, part and place, and "Go" (walk there, or open the place in quick mode). */
function CalendarRow({ e }: { e: CalendarEventView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const visit = useGame((s) => s.visit);
  const title = capital(tr.event(e.name.key, e.name.args));
  const where = placeName(tr, e.place);
  if (!e.today) {
    // Not on the day played: it went ahead on its own date, and is told, never shown as today.
    const sub = tr.t("ui-cal-went-ahead", { weekday: tr.t(e.weekday_key), day: e.date.day, month: tr.t(`month-${e.date.month}`) });
    return <Row mark={CAL_MARK} title={title} sub={sub} label={`${title}. ${sub}`} li={{ "data-testid": `cal-${e.id}`, "data-cal": "reported" }} />;
  }
  const line = tr.t("ui-cal-row", { part: e.part_key ? tr.t(e.part_key) : "", place: where });
  const state = e.attended ? tr.t("ui-cal-you-were-there") : e.expected ? tr.t("ui-cal-expected") : e.now ? tr.t("ui-cal-here-now") : null;
  const sub = [line, state].filter(Boolean).join(" · ");
  const go = () => void visit(e.place.location);
  return (
    <Row
      mark={CAL_MARK}
      title={title}
      sub={sub}
      label={`${title}. ${sub}`}
      onActivate={e.here ? undefined : go}
      end={
        e.here ? undefined : (
          <button type="button" className="btn btn-sm btn-primary rw-go" tabIndex={-1} disabled={busy} aria-label={tr.t("ui-cal-go-label", { event: title })} onClick={go} data-testid={`cal-go-${e.id}`}>
            {tr.t("ui-go")}
          </button>
        )
      }
      li={{ "data-testid": `cal-${e.id}`, "data-cal": e.expected ? "expected" : "held" }}
    />
  );
}

/** This fortnight (DEC-114): the turn's events on the day played, in the day's order, any that went ahead on another day, and the next turn's expected event with what staying away costs (shown a turn ahead). The engine decides all of it. */
function Fortnight({ p }: { p: PocketView }) {
  const tr = useGame((s) => s.tr);
  const n = p.next_expected;
  if (p.fortnight.length === 0 && !n) return null;
  return (
    <Section id="today-fortnight" title={tr.t("ui-this-fortnight")} testId="pocket-fortnight">
      {p.fortnight.length > 0 && (
        <RowList testId="pocket-fortnight-list">
          {p.fortnight.map((e) => (
            <CalendarRow key={e.id} e={e} />
          ))}
        </RowList>
      )}
      {n && (
        <p className="note small" data-testid="pocket-next-expected">
          {tr.t("ui-cal-next-expected", {
            event: tr.event(n.name.key, n.name.args),
            weekday: tr.t(n.weekday_key),
            part: n.part_key ? tr.t(n.part_key) : "",
            place: placeName(tr, n.place),
          })}{" "}
          {n.cost && tr.event(n.cost.key, n.cost.args)}
        </p>
      )}
    </Section>
  );
}

/** Today (DEC-113): one page. */
function Today({ p }: { p: PocketView }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const openPromises = useGame((s) => s.openPromises);
  const setTab = useGame((s) => s.setPocketTab);
  const setFold = useGame((s) => s.setFold);
  const more = AGENDA.length - 4;
  return (
    <>
      <div className="today-strip" data-testid="today-strip">
        <div>
          <span>{tr.t("ui-slots-short")}</span>
          <b>
            {p.slots_left}/{p.slot_cap}
          </b>
        </div>
        <div>
          <span>{tr.t("ui-money")}</span>
          <b>{formatMoney(p.money)}</b>
        </div>
        <div>
          <span>{tr.t("ui-mandate")}</span>
          <b>{String(Math.round(p.mandate))}</b>
        </div>
      </div>
      <p className="small today-where" data-testid="pocket-location">
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
      {p.notebook_new > 0 && (
        <p className="note" data-testid="today-notebook">
          {tr.t("ui-notebook-today", { n: p.notebook_new })}{" "}
          <button
            type="button"
            className="link-btn"
            onClick={() => {
              setFold("notebook-land", true);
              // On a wide screen the Notebook's panel opens again if it was hidden, and a place's panel over it gives way.
              if (isWide()) useGame.getState().uncoverNotebook();
              setTab("notebook");
            }}
            data-testid="today-notebook-open"
          >
            {tr.t("ui-notebook-today-open")}
          </button>
        </p>
      )}
      {p.legal && <Legal l={p.legal} />}
      {p.needs_you.length > 0 ? (
        <Section id="today-needs" title={tr.t("ui-needs-you")} testId="pocket-needs">
          <RowList testId="pocket-needs-list">
            {p.needs_you.map((n, i) => (
              <NeedRow key={`${n.kind}-${i}`} n={n} p={p} />
            ))}
          </RowList>
        </Section>
      ) : (
        <NextUp p={p} />
      )}
      {/* What waits on the player is Today's first section (POCKET_REWORK, L5); the fortnight follows. */}
      <Fortnight p={p} />
      <UnderWay p={p} />
      <section className="rw-sec" aria-label={tr.t("ui-today-do")}>
        {p.can_promise && (
          <button type="button" className="btn btn-sm" disabled={busy} onClick={() => void openPromises(true)} data-testid="open-promises">
            {tr.t("ui-promise-make")}
          </button>
        )}
        <Fold id="today-agenda" title={tr.t("ui-take-action")} hint={tr.t("ui-take-action-hint", { n: more })} count={null} testId="agenda" headTestId="agenda-open">
          <Agenda p={p} />
        </Fold>
        <Fold
          id="today-standing"
          title={p.days_to_election !== null ? tr.t("ui-election-in", { n: p.days_to_election }) : tr.t("ui-tracks")}
          hint={
            <span data-testid="pocket-election">
              {[p.next_election ? formatDate(tr, p.next_election) : null, `${tr.t("track-record")} ${Math.round(p.record)}`, `${tr.t("track-standing")} ${Math.round(p.tracks.standing)}`].filter(Boolean).join(" · ")}
            </span>
          }
          count={null}
          testId="pocket-standing"
        >
          <div className="rw-pad">
            <Tracks p={p} />
          </div>
        </Fold>
      </section>
    </>
  );
}

/** The ⋯ menu (owner's answer 3): the maps, the story so far, skip ahead, Legacy, save and leave, step away from politics (M6 note 23), report a problem, privacy. */
function MoreMenu() {
  const tr = useGame((s) => s.tr);
  const o = useGame((s) => s.office);
  const busy = useGame((s) => s.busy);
  const setView = useGame((s) => s.setView);
  const openMap = useGame((s) => s.openMap);
  const saveAndQuit = useGame((s) => s.saveAndQuit);
  const openStory = useGame((s) => s.openStory);
  const openTermReview = useGame((s) => s.openTermReview);
  const openLegacy = useGame((s) => s.openLegacy);
  const openStepAway = useGame((s) => s.openStepAway);
  const actOver = useGame((s) => s.overview?.act?.ended_key != null);
  const openReport = useReport((s) => s.openReport);
  const [open, setOpen] = useState(false);
  const wrap = useRef<HTMLDivElement>(null);
  const toggle = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!open) return;
    const away = (e: PointerEvent) => {
      if (wrap.current && !wrap.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", away);
    return () => document.removeEventListener("pointerdown", away);
  }, [open]);
  const act = (f: () => void) => () => {
    setOpen(false);
    f();
  };
  return (
    <div className="pocket-more" ref={wrap}>
      <button
        ref={toggle}
        type="button"
        className="btn btn-ghost pocket-icon"
        aria-expanded={open}
        aria-controls="pocket-menu"
        aria-label={tr.t("ui-pocket-more")}
        title={tr.t("ui-pocket-more")}
        onClick={() => setOpen(!open)}
        data-testid="pocket-more"
      >
        <span aria-hidden="true">⋯</span>
      </button>
      {open && (
        <nav
          id="pocket-menu"
          className="pocket-menu"
          aria-label={tr.t("ui-pocket-links")}
          data-testid="pocket-menu"
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              e.stopPropagation();
              setOpen(false);
              toggle.current?.focus();
            }
          }}
        >
          {o && <p className="muted small pocket-menu-who">{[o.player.name, tr.t(o.player.role_key), o.player.municipality].join(" · ")}</p>}
          <button type="button" className="btn" onClick={act(() => openMap("country"))} data-testid="pocket-country-map">
            {tr.t("ui-map")}
          </button>
          <button type="button" className="btn" onClick={act(() => setView("overview"))}>
            {tr.t("ui-overview")}
          </button>
          <button type="button" className="btn" onClick={act(() => void openStory(true))} data-testid="open-story">
            {tr.t("ui-story-so-far")}
          </button>
          {!actOver && (
            <button type="button" className="btn" disabled={busy} onClick={act(() => void openTermReview(true))} data-testid="open-term-review">
              {tr.t("ui-term-review-open")}
            </button>
          )}
          <button type="button" className="btn" onClick={act(() => void openLegacy(true))} data-testid="open-legacy">
            {actOver ? tr.t("ui-epilogue") : tr.t("ui-epilogue-legacy")}
          </button>
          <button type="button" className="btn" disabled={busy} onClick={act(() => void saveAndQuit())}>
            {tr.t("ui-save-quit")}
          </button>
          {!actOver && (
            <button type="button" className="btn" disabled={busy} onClick={act(() => openStepAway(true))} data-testid="open-step-away">
              {tr.t("ui-step-away")}
            </button>
          )}
          <button type="button" className="btn" onClick={act(() => openReport("menu"))} data-testid="report-problem">
            {tr.t("ui-report")}
          </button>
          <a className="btn" href={`${import.meta.env.BASE_URL}../privacy.html`} target="_blank" rel="noopener" title={tr.t("ui-privacy-note")} data-testid="privacy-link-pocket">
            {tr.t("ui-privacy")}
          </a>
        </nav>
      )}
    </div>
  );
}

/** The Notebook beside Today on wide screens: a panel on the left with its own head (covered while another sheet is open there). */
function NotebookPane({ onClose }: { onClose: () => void }) {
  const tr = useGame((s) => s.tr);
  const panel = useRef<HTMLDivElement>(null);
  const focusTick = useGame((s) => s.pocketFocus);
  useEffect(() => {
    if (focusTick === 0) return;
    const id = requestAnimationFrame(() => panel.current?.focus());
    return () => cancelAnimationFrame(id);
  }, [focusTick]);
  return (
    <section id="pocket-book" className="pocket-book" aria-labelledby="pocket-book-title" data-testid="pocket-notebook">
      <div className="sheet-head">
        <div className="pocket-book-title">
          <h2 id="pocket-book-title">{tr.t("ui-pocket-tab-notebook")}</h2>
          <button type="button" className="btn btn-ghost pocket-icon" onClick={onClose} aria-label={tr.t("ui-notebook-close")} data-testid="notebook-close">
            ✕
          </button>
        </div>
        <NotebookTabs />
      </div>
      <Notebook panelRef={panel} />
    </section>
  );
}

/**
 * The pocket. The switch (`./PocketTabs`) shows Today or the Notebook on a
 * phone; on a wide screen Today stays on the right and the Notebook opens on
 * the left, both at once.
 */
export function Pocket() {
  const tr = useGame((s) => s.tr);
  const p = useGame((s) => s.pocket);
  const open = useGame((s) => s.pocketOpen);
  const setOpen = useGame((s) => s.setPocketOpen);
  const side = useGame((s) => s.pocketTab);
  const setTab = useGame((s) => s.setPocketTab);
  const docked = useGame((s) => s.pocketDocked);
  const setDocked = useGame((s) => s.setPocketDocked);
  const focusTick = useGame((s) => s.pocketFocus);
  const leftBusy = useGame((s) => s.openPlace !== null || s.character !== null || s.decision !== null || s.story !== null || s.arc !== null || s.talk !== null || s.termReview !== null || s.legacy !== null || s.promiseMenu !== null || s.skipSummary !== null);
  const [bookShut, setBookShut] = useFold("pocket-book-shut", false);
  const uncover = useGame((s) => s.uncoverNotebook);
  const wide = useWide();
  const [closing, requestClose, sheetRef] = useSheetClose(() => setOpen(false));
  const panelRef = useRef<HTMLDivElement>(null);
  const shown = open && p !== null;
  const book = wide && !bookShut;
  // "Skip to Places" (and anything else that asks) moves the keyboard into the open side.
  useEffect(() => {
    if (!shown || focusTick === 0 || (wide && side === "notebook")) return;
    const id = requestAnimationFrame(() => panelRef.current?.focus());
    return () => cancelAnimationFrame(id);
  }, [focusTick, shown, wide, side]);
  // "/" goes to the Notebook's search (a desktop key), unless the player is typing.
  const setQ = useGame((s) => s.setNotebookQuery);
  useEffect(() => {
    if (!shown) return;
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement | null;
      if (e.key !== "/" || e.ctrlKey || e.metaKey || e.altKey || (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable))) return;
      e.preventDefault();
      if (!wide) setTab("notebook");
      else setBookShut(false);
      setQ("");
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [shown, wide, setTab, setQ, setBookShut]);
  if (!shown) return null;
  const bookShown = book && !leftBusy;
  // With the Notebook beside it, the pocket appears in place: a slide would carry the Notebook's panel with it.
  const cls = ["sheet", "pocket", docked ? "pocket-docked" : "", bookShown ? "pocket-with-book" : "", closing ? "sheet-closing" : ""].filter(Boolean).join(" ");
  // On a phone the switch shows one side; on a wide screen Today is always this panel.
  const showing = wide ? "today" : side;
  const onSwitch = (t: "today" | "notebook") => {
    if (wide) {
      if (t === "notebook") {
        // Asked for, the Notebook comes back from under whatever covers it on the left.
        setBookShut(false);
        uncover();
      }
      setTab(t);
      return;
    }
    setTab(t);
  };
  return (
    <>
      <div className={closing ? "sheet-scrim sheet-closing" : "sheet-scrim"} onClick={requestClose} aria-hidden="true" />
      <aside ref={sheetRef} className={cls} aria-label={tr.t("ui-pocket")} data-testid="pocket" data-side={showing}>
        <div className="sheet-head pocket-head">
          <div className="pocket-head-row">
            <PocketTabs side={showing} onChange={onSwitch} p={p} wide={wide} bookOpen={bookShown} />
            <MoreMenu />
            <button
              type="button"
              className="btn btn-ghost pocket-icon pocket-dock"
              aria-pressed={docked}
              onClick={() => setDocked(!docked)}
              aria-label={tr.t("ui-pocket-dock")}
              title={tr.t(docked ? "ui-pocket-undock-hint" : "ui-pocket-dock-hint")}
              data-testid="pocket-dock"
            >
              <span aria-hidden="true">{docked ? "◧" : "▭"}</span>
            </button>
            <button type="button" className="btn btn-ghost pocket-icon" onClick={requestClose} aria-label={tr.t("ui-close")}>
              ✕
            </button>
          </div>
          {!wide && side === "notebook" && <NotebookTabs />}
        </div>
        <div className="sheet-body pocket-body">
          {showing === "today" ? (
            <div ref={panelRef} role={wide ? "region" : "tabpanel"} id={pocketPanelId("today")} aria-labelledby={wide ? undefined : pocketTabId("today")} aria-label={wide ? tr.t("ui-pocket-tab-today") : undefined} tabIndex={-1} className="pocket-panel" data-testid="pocket-panel-today">
              <Today p={p} />
            </div>
          ) : (
            <div role="tabpanel" id={pocketPanelId("notebook")} aria-labelledby={pocketTabId("notebook")} className="pocket-panel" data-testid="pocket-panel-notebook">
              <Notebook panelRef={panelRef} />
            </div>
          )}
        </div>
        {bookShown && <NotebookPane onClose={() => setBookShut(true)} />}
      </aside>
    </>
  );
}

/** The bar's two counts (owner's answer 5): what needs you (the engine's count) and what is new in the notebook. */
export function barBadges(tr: ReturnType<typeof useGame.getState>["tr"], p: PocketView): { needs: { n: number; text: string }; notebook: { n: number; text: string } } {
  return {
    needs: { n: p.needs_count, text: tr.t("ui-needs-count", { n: p.needs_count }) },
    notebook: { n: p.notebook_new, text: tr.t("ui-notebook-new-count", { n: p.notebook_new }) },
  };
}