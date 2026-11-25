// The Notebook's People tab (DEC-113 §3.1 to §3.3): everyone the player has
// heard of, from one query (`QueryDto::People`), in the engine's sections and
// order: who matters now first (with why), then in office, everyone else, and
// those who moved on. A row is two lines (three when the person matters now):
// the name, how they stand in a word with a shape, the role, and one action
// ("Talk" when they are here, "Go" when not). Tapped, it opens in place: why
// now, how they stand, what is known, the engine's one sentence where a gap
// changes something, where they are usually, Talk and Go to them, and
// everything about them (the character sheet). Knowledge is never drawn as a
// level (DEC-091). The engine decides everything shown; this lays it out.
import type { PeopleGroupView } from "../../engine/protocol/PeopleGroupView";
import type { PersonRowView } from "../../engine/protocol/PersonRowView";
import type { NowReasonView } from "../../engine/protocol/NowReasonView";
import type { SeenView } from "../../engine/protocol/SeenView";
import type { Translator } from "../../i18n/fluent";
import { useGame } from "../../state/gameStore";
import { personName } from "../Knowing";
import { TalkButton } from "../talk/TalkButton";
import { Portrait } from "../portrait/Portrait";
import { lookVariant } from "../portrait/look";
import { districtLabel } from "../posts";
import { capital } from "../DecisionSheet";
import { Fold, Row, RowList, Section, ShowMore, stanceMark, useShowMore } from "./rows";
import { itemFacts, leadText, type NotebookItem } from "./notebookModel";

/** Where the player saw them: where they usually are once seen twice, else the last sighting (S1). */
export function seenText(tr: Translator, s: SeenView): string {
  return tr.event(s.key, s.args);
}

/** A person's face, small (decorative: the row names them in words). */
export function Face({ id }: { id: number }) {
  return (
    <span className="rw-face">
      <Portrait variant={lookVariant(`c${id}`)} className="rw-portrait" />
    </span>
  );
}

/** A reason a person or place matters now, in its short line. */
export function reasonText(tr: Translator, r: NowReasonView): string {
  return tr.t(r.key, r.args);
}

/** The role in words, with the ward where they hold one. */
export function roleLine(tr: Translator, p: PersonRowView): string {
  const parts = [tr.t(p.role_key)];
  if (p.ward_number !== null) parts.push(districtLabel(tr, p.ward_number, "local"));
  return parts.join(" · ");
}

/** The stance as a shape and a word (five short lines beside the long ones). */
export function stanceText(tr: Translator, p: PersonRowView): string {
  return `${stanceMark(p.stance.shape)} ${tr.t(`ui-stance-${p.stance.band}`)}`;
}

/** Go to them: where the engine says their day has them; nowhere to go, the talk panel says where they are or why not. */
export function useGoToPerson() {
  const visit = useGame((s) => s.visit);
  const openTalk = useGame((s) => s.openTalk);
  return (p: PersonRowView) => {
    const to = p.go?.to;
    if (to && !p.go?.here) void visit(to.location);
    else void openTalk({ type: "person", person: p.id });
  };
}

/** What a reason opens: its ask (on Today), its decision, its arc. */
function ReasonLink({ r }: { r: NowReasonView }) {
  const tr = useGame((s) => s.tr);
  const openDecision = useGame((s) => s.openDecision);
  const openArc = useGame((s) => s.openArc);
  const setTab = useGame((s) => s.setPocketTab);
  const setFold = useGame((s) => s.setFold);
  if (r.decision !== null) {
    const id = r.decision;
    return (
      <button type="button" className="link-btn" onClick={() => void openDecision(id)}>
        {tr.t("ui-open-decision")}
      </button>
    );
  }
  if (r.ask !== null) {
    const id = r.ask;
    return (
      <button
        type="button"
        className="link-btn"
        onClick={() => {
          setFold(`row-ask-${id}`, true);
          setTab("today");
        }}
      >
        {tr.t("ui-open-ask")}
      </button>
    );
  }
  if (r.arc !== null) {
    const id = r.arc;
    return (
      <button type="button" className="link-btn" onClick={() => void openArc(id)}>
        {tr.t("ui-open-arc")}
      </button>
    );
  }
  return null;
}

/** A person opened in place (§3.2). */
function Opened({ p, item }: { p: PersonRowView; item: NotebookItem | null }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const openCharacter = useGame((s) => s.openCharacter);
  const go = useGoToPerson();
  const name = personName(tr, p.name);
  const stand = [tr.t(`opinion-${p.opinion_band}`), tr.t(`trust-${p.trust_band}`)];
  if (p.owes_you > 0) stand.push(tr.t("ui-owes-you", { n: p.owes_you }));
  if (p.you_owe > 0) stand.push(tr.t("ui-you-owe-them", { n: p.you_owe }));
  // The role is on the row; the rest of what the notebook knows follows.
  const roleFact = tr.t(p.role_key);
  const facts = item ? itemFacts(tr, item).filter((f) => f !== roleFact) : [];
  const seen = p.usually ?? p.last_seen;
  // The usual-place lead says what the seen line says; other leads stay (DEC-091: finding things is guided).
  const lead = item?.lead && !item.lead.key.startsWith("lead-usually") ? item.lead : null;
  return (
    <>
      {p.now.map((r, i) => (
        <p key={i} className="rw-why" data-testid="person-why">
          <b>{reasonText(tr, r)}</b>
          {r.title && <>: {capital(tr.event(r.title.key, r.title.args))}</>}
          {r.due && r.days_left !== null && <> · {tr.t("ui-days-left", { n: r.days_left })}</>} <ReasonLink r={r} />
        </p>
      ))}
      <p data-testid="notebook-standing">{stand.join(" · ")}</p>
      {facts.length > 0 && <p className="nb-facts">{facts.join(". ")}</p>}
      {p.note && (
        <p className="rw-know" data-testid="person-note">
          {tr.event(p.note.key, p.note.args)}
        </p>
      )}
      {lead && (
        <p className="rw-know">
          <span className="nb-lead-label">{tr.t("ui-notebook-lead")}: </span>
          {leadText(tr, lead)}
        </p>
      )}
      {seen && (
        <p className="muted" data-testid="notebook-seen">
          {seenText(tr, seen)}
        </p>
      )}
      <div className="rw-acts">
        <TalkButton person={p.id} name={p.name} />
        {!p.here && (
          <button type="button" className="btn btn-sm" disabled={busy} aria-label={tr.t("ui-people-go-to", { name })} onClick={() => go(p)} data-testid="notebook-go-to">
            <span aria-hidden="true">📍 </span>
            {tr.t("ui-people-go")}
          </button>
        )}
      </div>
      <p>
        <button type="button" className="link-btn" disabled={busy} onClick={() => void openCharacter(p.id)} data-testid="notebook-person-name">
          {tr.t("ui-person-everything", { name })}
        </button>
      </p>
    </>
  );
}

/** One person as a row (§3.1). `openId`/`setOpen`: one row open at a time in a list. */
export function PersonRow({ p, item, open, onToggle, isNew }: { p: PersonRowView; item: NotebookItem | null; open: boolean; onToggle: () => void; isNew: boolean }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const go = useGoToPerson();
  const name = personName(tr, p.name);
  const role = roleLine(tr, p);
  const first = p.now[0];
  const reason = first ? reasonText(tr, first) : p.note && p.group !== "now" ? tr.event(p.note.key, p.note.args) : null;
  const stance = stanceText(tr, p);
  const label = [name, role, tr.t(`opinion-${p.opinion_band}`), p.is_rival ? tr.t("ui-rival-line") : null, first ? reasonText(tr, first) : null, isNew ? tr.t("ui-notebook-new") : null].filter(Boolean).join(". ");
  const end = p.here && p.can_talk ? (
    <TalkButton person={p.id} name={p.name} />
  ) : (
    <button type="button" className="btn btn-sm rw-go" tabIndex={-1} disabled={busy} aria-label={tr.t("ui-people-go-to", { name })} onClick={() => go(p)} data-testid="notebook-go-to">
      {tr.t("ui-go")}
    </button>
  );
  return (
    <Row
      mark={<Face id={p.id} />}
      title={
        <>
          {name}
          {isNew && (
            <span className="badge rw-new" data-testid="notebook-new">
              {tr.t("ui-notebook-new")}
            </span>
          )}
        </>
      }
      sub={
        <>
          <span className="rw-stance">{stance}</span>
          {p.is_rival && <> · {tr.t("ui-rival-line")}</>} · {role}
        </>
      }
      reason={reason}
      end={end}
      label={label}
      open={open}
      onToggle={onToggle}
      li={{ "data-testid": "notebook-person", "data-person": String(p.id), "data-group": p.group, "data-office": p.group === "office" ? "true" : undefined, "data-rival": p.is_rival ? "true" : undefined, "data-new": isNew ? "true" : undefined }}
    >
      <Opened p={p} item={item} />
    </Row>
  );
}

const GROUP_KEY: Record<string, string> = { now: "ui-people-now", office: "ui-people-office", rest: "ui-people-others", gone: "ui-people-gone" };

/** People in a list, one open at a time, the engine's first `show` then "Show the other N". */
export function PeopleList({ rows, show, items, fresh, testId }: { rows: readonly PersonRowView[]; show: number; items: ReadonlyMap<number, NotebookItem>; fresh: ReadonlySet<number>; testId?: string }) {
  const folds = useGame((s) => s.folds);
  const setFold = useGame((s) => s.setFold);
  const { shown, more, showAll } = useShowMore(rows, show);
  // One person open at a time (§3.2), remembered for the session like a fold.
  const isOpen = (id: number) => folds[`person-${id}`] === true;
  const toggle = (id: number) => {
    const now = isOpen(id);
    for (const r of rows) if (r.id !== id && isOpen(r.id)) setFold(`person-${r.id}`, false);
    setFold(`person-${id}`, !now);
  };
  return (
    <>
      <RowList testId={testId}>
        {shown.map((p) => (
          <PersonRow key={p.id} p={p} item={items.get(p.id) ?? null} open={isOpen(p.id)} onToggle={() => toggle(p.id)} isNew={p.new_this_turn || fresh.has(p.id)} />
        ))}
      </RowList>
      <ShowMore more={more} onClick={showAll} />
    </>
  );
}

/** The People tab: "Matter now" open, then folded sections with what they hold counted. */
export function PeopleTab({ items, fresh }: { items: ReadonlyMap<number, NotebookItem>; fresh: ReadonlySet<number> }) {
  const tr = useGame((s) => s.tr);
  const people = useGame((s) => s.people);
  if (!people) return <p className="muted small">…</p>;
  if (people.rows.length === 0) return <p className="today-empty">{tr.t("ui-people-empty")}</p>;
  const rowsOf = (g: PeopleGroupView) => people.rows.filter((r) => r.group === g.group);
  const hasNow = people.groups.some((g) => g.group === "now" && g.count > 0);
  return (
    <div className="nb-people">
      {people.groups.map((g) => {
        const rows = rowsOf(g);
        if (rows.length === 0) return null;
        const title = tr.t(GROUP_KEY[g.group] ?? "ui-people-others");
        if (g.group === "now")
          return (
            <Section key={g.group} id="nb-people-now" title={title} testId="notebook-people-now">
              <PeopleList rows={rows} show={g.show} items={items} fresh={fresh} />
            </Section>
          );
        // "In office" opens when nobody matters now (§3.7).
        return (
          <Fold key={g.group} id={`people-${g.group}`} title={title} count={g.count} countText={tr.t("ui-people-count", { n: g.count })} open={g.group === "office" && !hasNow} testId={g.group === "office" ? "notebook-key-figures" : `notebook-people-${g.group}`}>
            <PeopleList rows={rows} show={g.show} items={items} fresh={fresh} />
          </Fold>
        );
      })}
    </div>
  );
}
