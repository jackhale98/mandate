// The notebook's People section: the single place for people (DEC-088 "it
// later joins the notebook", DEC-091). The key figures are pinned at the top
// (who runs things, the rival, the executive and the members of the
// legislature, from the office view and the town's character as the views
// give them); below them everyone else the player knows, with the same card;
// then what is known of power and the rival's plans, and the placeholders.
// A card says how they stand with you, what you know, where you saw them
// (S1: last seen, or where they usually are), a Talk button (DEC-093) and Go
// to them (auto-routed, DEC-079: the engine's talk view says where they are).
// Tapping a name opens the character sheet. The engine decides everything
// shown; this only arranges it.
import { useEffect, useState, type ReactNode } from "react";
import type { CharacterView } from "../../engine/protocol/CharacterView";
import type { KnowledgeLevelDto } from "../../engine/protocol/KnowledgeLevelDto";
import type { SeenView } from "../../engine/protocol/SeenView";
import { engine, expectKind } from "../../engine/client";
import { formatDate } from "../../i18n/format";
import type { Translator } from "../../i18n/fluent";
import { useGame } from "../../state/gameStore";
import { personName } from "../Knowing";
import { TalkButton } from "../talk/TalkButton";
import { itemFacts, keyFigures, levelDots, matches, personItems, type KeyFigure, type NotebookItem } from "./notebookModel";

/** Character sheets for the cards, by id (how they stand with you, where they were seen). */
export type SheetLoader = (ids: readonly number[]) => Promise<ReadonlyMap<number, CharacterView>>;

/** The engine's character view for each (read-only queries; one failing leaves its card without). */
export const loadSheets: SheetLoader = async (ids) => {
  const out = new Map<number, CharacterView>();
  await Promise.all(
    ids.map(async (id) => {
      try {
        out.set(id, expectKind(await engine().request({ kind: "query", payload: { type: "character", id } }), "character"));
      } catch {
        // The card shows what the notebook knows without it.
      }
    }),
  );
  return out;
};

/** Fetched again after every command (the pocket view is new then). */
function useSheets(ids: readonly number[], load: SheetLoader): ReadonlyMap<number, CharacterView> {
  const pocket = useGame((s) => s.pocket);
  const key = ids.join(",");
  const [sheets, setSheets] = useState<ReadonlyMap<number, CharacterView>>(new Map());
  useEffect(() => {
    let live = true;
    const list = key === "" ? [] : key.split(",").map(Number);
    void load(list).then((m) => {
      if (live) setSheets(m);
    });
    return () => {
      live = false;
    };
  }, [key, pocket, load]);
  return sheets;
}

/** Where the player saw them: where they usually are once seen twice, else the last sighting (S1). */
export function seenText(tr: Translator, s: SeenView): string {
  return tr.event(s.key, s.args);
}

/** How they stand with you, in words (the engine's bands; never colour alone). */
function standing(tr: Translator, c: CharacterView | undefined, f: KeyFigure | null): string | null {
  if (c && !c.is_player) {
    const parts = [tr.t(`opinion-${c.opinion_band}`), tr.t(`trust-${c.trust_band}`)];
    if (c.owes_you > 0) parts.push(tr.t("ui-owes-you", { n: c.owes_you }));
    if (c.you_owe > 0) parts.push(tr.t("ui-you-owe-them", { n: c.you_owe }));
    return parts.join(" · ");
  }
  const m = f?.member;
  if (m) return [tr.t("ui-opinion-of-you", { n: Math.round(m.opinion_of_player) }), m.favors >= 0 ? tr.t("ui-favors", { n: m.favors }) : tr.t("ui-owe", { n: -m.favors })].join(" · ");
  return null;
}

/** Go to them: where the engine says they can be talked to now (its talk view's route); here, or nowhere to go, the talk panel says so. */
function GoToThem({ person, name }: { person: number; name: string }) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const visit = useGame((s) => s.visit);
  const openTalk = useGame((s) => s.openTalk);
  const go = async () => {
    const to = { type: "person" as const, person };
    try {
      const v = expectKind(await engine().request({ kind: "query", payload: { type: "talk", to, late_night: false } }), "talk");
      if (v.route?.to) return await visit(v.route.to.location);
    } catch {
      // The talk panel says where they are, or why not.
    }
    await openTalk(to);
  };
  return (
    <button type="button" className="btn btn-sm btn-ghost" disabled={busy} aria-label={tr.t("ui-people-go-to", { name: personName(tr, name) })} onClick={() => void go()} data-testid="notebook-go-to">
      <span aria-hidden="true">📍 </span>
      {tr.t("ui-people-go")}
    </button>
  );
}

interface CardProps {
  id: number;
  name: string;
  /** Their role, ward and why they are pinned, in words. */
  role: string;
  figure: KeyFigure | null;
  item: NotebookItem | null;
  sheet: CharacterView | undefined;
}

function PersonCard({ id, name, role, figure, item, sheet }: CardProps) {
  const tr = useGame((s) => s.tr);
  const busy = useGame((s) => s.busy);
  const openCharacter = useGame((s) => s.openCharacter);
  const shown = personName(tr, name);
  const level: KnowledgeLevelDto = item?.level ?? sheet?.knowledge.level ?? figure?.member?.level ?? figure?.person?.level ?? "unknown";
  const last = item?.entry?.history[item.entry.history.length - 1];
  // The role is said above; the rest of what the notebook knows follows.
  const roleFact = item?.args.role_key ? tr.t(item.args.role_key) : null;
  const facts = item ? itemFacts(tr, item).filter((f) => f !== roleFact) : [];
  const seen = sheet?.usually ?? sheet?.last_seen ?? item?.entry?.seen ?? null;
  const stand = standing(tr, sheet, figure);
  const rumoured = figure?.person?.rumoured === true;
  // The usual-place lead says what the seen line says; other leads stay.
  const lead = item?.lead && !item.lead.key.startsWith("lead-usually") ? item.lead : null;
  return (
    <li className={figure ? "nb-item nb-person nb-person-key" : "nb-item nb-person"} data-testid="notebook-person" data-person={id} data-office={figure?.member ? "true" : undefined} data-level={level} data-new={item?.isNew ? "true" : undefined}>
      <div className="nb-head">
        <h4 className="nb-title">
          <button type="button" className="link-btn nb-person-name" disabled={busy} onClick={() => void openCharacter(id)} aria-label={tr.t("ui-notebook-person-open", { name: shown })} data-testid="notebook-person-name">
            {shown}
          </button>
          {rumoured && <span className="muted small"> ({tr.t("knowledge-rumoured")})</span>}
        </h4>
        {item?.isNew && (
          <span className="badge nb-new" data-testid="notebook-new">
            {tr.t("ui-notebook-new")}
          </span>
        )}
      </div>
      {role && <p className="muted small nb-person-role">{role}</p>}
      {stand && (
        <p className="small nb-person-standing" data-testid="notebook-standing">
          {stand}
        </p>
      )}
      <p className="nb-level small">
        <span className="nb-dots" aria-hidden="true">
          {levelDots(level)}
        </span>{" "}
        {tr.t(`knowledge-level-${level}`)}
        {last && (
          <span className="muted nb-from" data-testid="notebook-from">
            {" · "}
            {tr.t("ui-notebook-from", { source: tr.event(last.key, last.args), date: formatDate(tr, last.on) })}
          </span>
        )}
      </p>
      {facts.map((f, i) => (
        <p key={i} className="nb-fact">
          {f}
        </p>
      ))}
      {figure?.member && figure.member.convictions.length > 0 && <p className="nb-fact">{figure.member.convictions.map((c) => tr.t(`conviction-${c}`)).join(" · ")}</p>}
      {seen && (
        <p className="small nb-seen" data-testid="notebook-seen">
          {seenText(tr, seen)}
        </p>
      )}
      {lead && (
        <p className="small nb-person-lead">
          <span className="nb-lead-label">{tr.t("ui-notebook-lead")}: </span>
          {tr.event(lead.key, lead.args)}
        </p>
      )}
      <div className="nb-person-actions">
        <TalkButton person={id} name={name} />
        <GoToThem person={id} name={name} />
      </div>
    </li>
  );
}

/** A key figure's role in words: why pinned (who runs things, your rival), their post's role word, the ward. */
export function figureRole(tr: Translator, f: KeyFigure): string {
  const parts: string[] = [];
  if (f.pin === "broker") parts.push(tr.t("ui-power-broker-line"));
  if (f.pin === "rival") parts.push(tr.t("ui-rival-line"));
  if (f.roleKey) parts.push(tr.t(f.roleKey));
  if (f.ward !== null) parts.push(tr.t("ui-ward", { n: f.ward }));
  return parts.join(" · ");
}

/**
 * The People section's body. `items` are the notebook's People entries and
 * placeholders (already searched); `q` the search, for the key figures.
 * `Rest` renders what is not a person (power, the rival's plans, gaps).
 */
export function PeopleSection({ items, q, Rest, load = loadSheets }: { items: NotebookItem[]; q: string; Rest: (p: { items: NotebookItem[] }) => ReactNode; load?: SheetLoader }) {
  const tr = useGame((s) => s.tr);
  const office = useGame((s) => s.office);
  const town = useGame((s) => s.town);
  const byPerson = personItems(items);
  const figures = keyFigures(office, town?.character ?? null);
  const pinned = new Set(figures.map((f) => f.id));
  const texts = (f: KeyFigure) => [personName(tr, f.name), figureRole(tr, f)].join(" ").toLowerCase();
  const shownFigures = figures.filter((f) => byPerson.has(f.id) || matches(texts(f), q));
  const others = items.filter((i) => i.subject.type === "person" && !pinned.has(i.subject.person));
  const rest = items.filter((i) => i.subject.type !== "person");
  const ids = [...shownFigures.map((f) => f.id), ...others.map((i) => (i.subject.type === "person" ? i.subject.person : -1))];
  const sheets = useSheets(ids, load);
  if (shownFigures.length + others.length + rest.length === 0) return <p className="muted small">{q.trim() !== "" ? tr.t("ui-notebook-filter-none", { q }) : tr.t("ui-notebook-empty-people")}</p>;
  return (
    <>
      {shownFigures.length > 0 && (
        <section aria-labelledby="nb-people-key" data-testid="notebook-key-figures">
          <h3 className="sub" id="nb-people-key">
            {tr.t("ui-people-key")}
          </h3>
          <ul className="nb-list">
            {shownFigures.map((f) => (
              <PersonCard key={f.id} id={f.id} name={f.name} role={figureRole(tr, f)} figure={f} item={byPerson.get(f.id) ?? null} sheet={sheets.get(f.id)} />
            ))}
          </ul>
        </section>
      )}
      {others.length > 0 && (
        <section aria-labelledby="nb-people-others" data-testid="notebook-people-others">
          <h3 className="sub" id="nb-people-others">
            {tr.t("ui-people-others")}
          </h3>
          <ul className="nb-list">
            {others.map((i) => {
              const id = i.subject.type === "person" ? i.subject.person : -1;
              const role = i.args.role_key ? tr.t(i.args.role_key) : "";
              return <PersonCard key={i.key} id={id} name={i.args.name ?? ""} role={role} figure={null} item={i} sheet={sheets.get(id)} />;
            })}
          </ul>
        </section>
      )}
      {rest.length > 0 && (
        <section aria-labelledby="nb-people-power" data-testid="notebook-people-power">
          <h3 className="sub" id="nb-people-power">
            {tr.t("ui-people-power")}
          </h3>
          <Rest items={rest} />
        </section>
      )}
      <p className="hint">{tr.t("ui-people-help")}</p>
    </>
  );
}
