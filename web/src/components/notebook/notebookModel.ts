// The notebook's arrangement (DEC-091, DEC-113): what the engine's
// NotebookView says the player knows, filed under the Notebook's four tabs
// (People, Places, Town, Policies). Only what is known is listed: there are
// no placeholders and nothing counts knowledge (DEC-091, 2026-11-25: knowledge
// is not a checklist). The engine's gaps stay for one thing: a search for
// something not known answers with the way to find out. Presentation only:
// the engine decides what is known, what is new and what a lead would teach;
// this files and words it.
import type { KnowledgeLevelDto } from "../../engine/protocol/KnowledgeLevelDto";
import type { LeadView } from "../../engine/protocol/LeadView";
import type { NotebookEntryView } from "../../engine/protocol/NotebookEntryView";
import type { NotebookGapView } from "../../engine/protocol/NotebookGapView";
import type { NotebookView } from "../../engine/protocol/NotebookView";
import type { SubjectDto } from "../../engine/protocol/SubjectDto";
import type { Translator } from "../../i18n/fluent";
import { NOTEBOOK_TABS, type NotebookTab } from "../../state/devicePrefs";

export { NOTEBOOK_TABS, type NotebookTab };

/** A subject as a stable key (the engine's tagged form). */
export const subjectKey = (s: SubjectDto): string => JSON.stringify(s);

/** Where a subject is filed: people under People; a neighbourhood's worries and what is known of places under Places; the town's worries, who runs things, the rival's plans and gossip under Town; support and proposals under Policies. */
export function tabOf(s: SubjectDto): NotebookTab {
  switch (s.type) {
    case "person":
      return "people";
    case "broker":
    case "rival_plans":
    case "secret":
    case "town_concerns":
      return "town";
    case "hood_concerns":
    case "place":
    case "layer":
    case "sector":
    case "site":
    case "service":
      return "places";
    case "policy_support":
    case "proposal":
    case "situational":
      return "policies";
  }
}

/** One thing known (an entry of the engine's notebook). */
export interface NotebookItem {
  key: string;
  subject: SubjectDto;
  level: KnowledgeLevelDto;
  titleKey: string;
  args: Record<string, string>;
  entry: NotebookEntryView | null;
  /** How to learn more, where the engine gives one (shown on the thing it is about). */
  lead: LeadView | null;
  /** New since the player last looked (kept while they look). */
  isNew: boolean;
}

export interface NotebookGroups {
  people: NotebookItem[];
  places: NotebookItem[];
  town: NotebookItem[];
  policies: NotebookItem[];
  /** Leads the player picked up, newest first ("Loose ends", under Town). */
  leads: LeadView[];
  /** What is public and not known: never listed, only answered when searched for. */
  gaps: NotebookGapView[];
}

/** Subjects the engine says are new (learned this turn, not yet seen). */
export function freshKeys(v: NotebookView): string[] {
  return v.sections.flatMap((s) => s.entries.filter((e) => e.new_this_turn).map((e) => subjectKey(e.subject)));
}

/** Order inside a tab: the town's worries, who runs things, the rival's plans, gossip; neighbourhoods by name. */
const RANK: Partial<Record<SubjectDto["type"], number>> = { town_concerns: 0, broker: 1, rival_plans: 2, secret: 3, hood_concerns: 4 };

function byKey(a: NotebookItem, b: NotebookItem): number {
  const ra = RANK[a.subject.type] ?? 5;
  const rb = RANK[b.subject.type] ?? 5;
  return ra - rb || (a.args.hood ?? a.args.name ?? "").localeCompare(b.args.hood ?? b.args.name ?? "");
}

/** File the notebook by tab. `fresh` holds what stays marked new while the player looks. */
export function groupNotebook(v: NotebookView, fresh: ReadonlySet<string> = new Set()): NotebookGroups {
  const g: NotebookGroups = { people: [], places: [], town: [], policies: [], leads: v.leads, gaps: v.gaps };
  for (const section of v.sections) {
    for (const e of section.entries) {
      const key = subjectKey(e.subject);
      g[tabOf(e.subject)].push({ key, subject: e.subject, level: e.level, titleKey: e.title_key, args: e.args, entry: e, lead: e.lead, isNew: e.new_this_turn || fresh.has(key) });
    }
  }
  for (const tab of NOTEBOOK_TABS) g[tab].sort(byKey);
  return g;
}

/** The notebook's entry for each person, by character id (what they do, care about, owe). */
export function personItems(items: readonly NotebookItem[]): Map<number, NotebookItem> {
  const m = new Map<number, NotebookItem>();
  for (const i of items) if (i.subject.type === "person") m.set(i.subject.person, i);
  return m;
}

/** An item's title. */
export function itemTitle(tr: Translator, i: NotebookItem): string {
  return tr.event(i.titleKey, i.args);
}

/** The facts an item holds, one line each, as far as its level reveals (the engine leaves out what is not known). */
export function itemFacts(tr: Translator, i: NotebookItem): string[] {
  const a = i.args;
  const k = (key: string | undefined) => (key ? tr.t(key) : "");
  const out: string[] = [];
  switch (i.subject.type) {
    case "town_concerns":
    case "hood_concerns": {
      const worries = [a.concern_key, a.concern2_key, a.concern3_key].filter((x): x is string => !!x).map((x) => tr.t(x));
      if (worries.length === 1) out.push(tr.t("ui-notebook-top-worry", { a: worries[0] ?? "" }));
      else if (worries.length === 2) out.push(tr.t("ui-notebook-worries-2", { a: worries[0] ?? "", b: worries[1] ?? "" }));
      else if (worries.length >= 3) out.push(tr.t("ui-notebook-worries-3", { a: worries[0] ?? "", b: worries[1] ?? "", c: worries[2] ?? "" }));
      break;
    }
    case "broker":
      if (a.name) out.push(a.rumoured === "true" ? `${a.name} ${tr.t("ui-notebook-rumoured")}` : a.name);
      if (a.stake_key) out.push(tr.t("ui-notebook-stake", { stake: k(a.stake_key) }));
      break;
    case "person":
      if (a.role_key) out.push(k(a.role_key));
      if (a.cares_key) out.push(tr.t("ui-notebook-cares", { issue: k(a.cares_key) }));
      if (a.owes) out.push(tr.t("ui-notebook-owes", { name: a.owes }));
      break;
    case "rival_plans":
      if (a.rival) out.push(tr.t("ui-notebook-rival", { name: a.rival }));
      if (i.entry) {
        if (a.seat_key) out.push(tr.t("ui-notebook-rival-seat", { seat: tr.t(a.seat_key, { ward: a.ward ?? "", post: a.post_key ? tr.t(a.post_key) : "" }) }));
        if (a.focus_key) out.push(tr.t("ui-notebook-rival-focus", { issue: k(a.focus_key) }));
        if (!a.seat_key && !a.focus_key) out.push(tr.t("ui-notebook-rival-something"));
      }
      break;
    case "policy_support":
      if (i.entry && a.band_key) {
        out.push(a.support !== undefined ? tr.t("ui-notebook-support", { band: k(a.band_key), pct: signedPoints(a.support) }) : tr.t("ui-notebook-support-band", { band: k(a.band_key) }));
      }
      break;
    case "place":
      if (a.kind_key) out.push(tr.t("ui-notebook-place-kind", { kind: k(a.kind_key) }));
      break;
    case "situational":
      if (a.reason_key) out.push(tr.t("ui-notebook-proposal-reason", { reason: k(a.reason_key) }));
      break;
    default:
      break;
  }
  return out;
}

function signedPoints(v: string): string {
  const n = Number(v);
  return Number.isFinite(n) && n > 0 ? `+${n}` : v;
}

/** A lead in words. */
export function leadText(tr: Translator, l: LeadView): string {
  return tr.event(l.key, l.args);
}

/** Everything an item says, for the search box. */
export function itemText(tr: Translator, i: NotebookItem): string {
  return [itemTitle(tr, i), ...itemFacts(tr, i), i.lead ? leadText(tr, i.lead) : ""].join(" ").toLowerCase();
}

/** Whether a text matches a search. An empty search matches everything. */
export function matches(text: string, q: string): boolean {
  const words = q.trim().toLowerCase().split(/\s+/).filter(Boolean);
  return words.every((w) => text.toLowerCase().includes(w));
}

/** Counts of new items per tab (the Notebook's tab badges); people come from the People view's own marks. */
export function newCounts(g: NotebookGroups, peopleNew: number): Record<NotebookTab, number> {
  const n = (t: NotebookTab) => g[t].filter((i) => i.isNew).length;
  return { people: peopleNew, places: n("places"), town: n("town"), policies: n("policies") };
}

/** The level as dots (the town map's layer panel only; the pocket draws no meter, DEC-113). */
export function levelDots(level: KnowledgeLevelDto): string {
  const n = { unknown: 0, heard: 1, known: 2, detailed: 3 }[level];
  return "●".repeat(n) + "○".repeat(3 - n);
}
