// The notebook's arrangement (DEC-091, K5): what the engine's NotebookView
// says the player knows, filed by tab for finding things (people, places,
// worries by neighbourhood, policies, leads), with the engine's placeholders
// (`gaps`) beside what is known. Presentation only: the engine decides what
// is known, what is new and what a lead would teach; this sorts and words it.
import type { KnowledgeLevelDto } from "../../engine/protocol/KnowledgeLevelDto";
import type { LeadView } from "../../engine/protocol/LeadView";
import type { MemberView } from "../../engine/protocol/MemberView";
import type { OfficeView } from "../../engine/protocol/OfficeView";
import type { PersonRefView } from "../../engine/protocol/PersonRefView";
import type { TownCharacterView } from "../../engine/protocol/TownCharacterView";
import type { NotebookEntryView } from "../../engine/protocol/NotebookEntryView";
import type { NotebookView } from "../../engine/protocol/NotebookView";
import type { SubjectDto } from "../../engine/protocol/SubjectDto";
import type { Translator } from "../../i18n/fluent";

export type NotebookTab = "people" | "places" | "issues" | "policies" | "leads";
export const NOTEBOOK_TABS: readonly NotebookTab[] = ["people", "places", "issues", "policies", "leads"];
/** The tabs that hold entries (leads have their own). */
export type EntryTab = Exclude<NotebookTab, "leads">;

/** A subject as a stable key (the engine's tagged form). */
export const subjectKey = (s: SubjectDto): string => JSON.stringify(s);

/** Where a subject is filed. */
export function tabOf(s: SubjectDto): EntryTab {
  switch (s.type) {
    case "person":
    case "broker":
    case "rival_plans":
    case "secret":
      return "people";
    case "town_concerns":
    case "hood_concerns":
      return "issues";
    case "place":
    case "layer":
      return "places";
    case "policy_support":
    case "proposal":
    case "situational":
      return "policies";
  }
}

/** One line of the notebook: something known (`entry`), or a placeholder for something public not known yet. */
export interface NotebookItem {
  key: string;
  subject: SubjectDto;
  level: KnowledgeLevelDto;
  titleKey: string;
  args: Record<string, string>;
  /** Null for a placeholder. */
  entry: NotebookEntryView | null;
  /** How to learn more (the engine's open lead, or where it is usually learned). */
  lead: LeadView | null;
  /** New since the player last looked (kept while they look). */
  isNew: boolean;
}

export interface NotebookGroups {
  people: NotebookItem[];
  places: NotebookItem[];
  issues: NotebookItem[];
  policies: NotebookItem[];
  /** `open`: leads the player picked up, newest first; `standing`: ways to learn what is not known. */
  leads: { open: LeadView[]; standing: LeadView[] };
}

/** Subjects the engine says are new (learned this turn, not yet seen). */
export function freshKeys(v: NotebookView): string[] {
  return v.sections.flatMap((s) => s.entries.filter((e) => e.new_this_turn).map((e) => subjectKey(e.subject)));
}

/** Order within People: who runs things, the rival, then people by name, gossip last. */
const PEOPLE_RANK: Partial<Record<SubjectDto["type"], number>> = { broker: 0, rival_plans: 1, person: 2, secret: 3 };

function sortKey(i: NotebookItem): [number, string] {
  switch (i.subject.type) {
    case "town_concerns":
      return [0, ""];
    case "hood_concerns":
      return [1, i.args.hood ?? ""];
    case "person":
      return [PEOPLE_RANK.person ?? 2, i.args.name ?? ""];
    default:
      return [PEOPLE_RANK[i.subject.type] ?? 0, ""];
  }
}

function byKey(a: NotebookItem, b: NotebookItem): number {
  const [ra, sa] = sortKey(a);
  const [rb, sb] = sortKey(b);
  return ra - rb || sa.localeCompare(sb);
}

const leadId = (l: LeadView) => `${subjectKey(l.subject)}|${l.key}|${l.level}`;

/** File the notebook by tab. `fresh` holds what stays marked new while the player looks. */
export function groupNotebook(v: NotebookView, fresh: ReadonlySet<string> = new Set()): NotebookGroups {
  const g: NotebookGroups = { people: [], places: [], issues: [], policies: [], leads: { open: v.leads, standing: [] } };
  for (const section of v.sections) {
    for (const e of section.entries) {
      const key = subjectKey(e.subject);
      g[tabOf(e.subject)].push({ key, subject: e.subject, level: e.level, titleKey: e.title_key, args: e.args, entry: e, lead: e.lead, isNew: e.new_this_turn || fresh.has(key) });
    }
  }
  for (const gap of v.gaps) {
    g[tabOf(gap.subject)].push({ key: subjectKey(gap.subject), subject: gap.subject, level: "unknown", titleKey: gap.title_key, args: gap.args, entry: null, lead: gap.lead, isNew: false });
  }
  // Worries by neighbourhood and people by name; policies known first; in
  // every tab what is new comes first, where the player lands to see it.
  g.people.sort(byKey);
  g.issues.sort(byKey);
  g.policies.sort((a, b) => Number(a.entry === null) - Number(b.entry === null));
  for (const tab of ["people", "places", "issues", "policies"] as const) g[tab].sort((a, b) => Number(b.isNew) - Number(a.isNew));
  // Standing leads: every way to learn more the open leads do not already give.
  const seen = new Set(v.leads.map(leadId));
  for (const tab of ["people", "places", "issues", "policies"] as const) {
    for (const i of g[tab]) {
      if (i.lead && !seen.has(leadId(i.lead))) {
        seen.add(leadId(i.lead));
        g.leads.standing.push(i.lead);
      }
    }
  }
  return g;
}

/** The first tab holding something new (where the Today link lands). */
export function firstNewTab(g: NotebookGroups): EntryTab | null {
  for (const tab of ["people", "places", "issues", "policies"] as const) if (g[tab].some((i) => i.isNew)) return tab;
  return null;
}

/** Counts of new items per tab (the notebook's own tab badges). */
export function newCounts(g: NotebookGroups): Record<EntryTab, number> {
  const n = (t: EntryTab) => g[t].filter((i) => i.isNew).length;
  return { people: n("people"), places: n("places"), issues: n("issues"), policies: n("policies") };
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
        if (a.seat_key) out.push(tr.t("ui-notebook-rival-seat", { seat: tr.t(a.seat_key, { ward: a.ward ?? "" }) }));
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

/** Whether an item (or a lead) matches a search. An empty search matches everything. */
export function matches(text: string, q: string): boolean {
  const words = q.trim().toLowerCase().split(/\s+/).filter(Boolean);
  return words.every((w) => text.includes(w));
}

/** The level as a mark that does not rely on colour (DEC-051): filled dots out of three, with the level in words beside it. */
export function levelDots(level: KnowledgeLevelDto): string {
  const n = { unknown: 0, heard: 1, known: 2, detailed: 3 }[level];
  return "●".repeat(n) + "○".repeat(3 - n);
}

// --- People (the notebook's People section: the single place for people) ---

/** Someone pinned at the top of People: who runs things, the rival, the holder of the executive post and the members of the legislature. */
export interface KeyFigure {
  /** Their character (`QueryDto::Character`, `Talk`). */
  id: number;
  /** As far as the player knows them (empty below `heard`). */
  name: string;
  /** Why they are pinned besides their post: `broker` (who runs things), `rival`, or none. */
  pin: "broker" | "rival" | null;
  /** Their role as the views give it: the post's role word from the constitution (`role-<word>`), or an occupation's name key. */
  roleKey: string;
  ward: number | null;
  /** In office: the office view's line on them (opinion of you, favours, convictions). */
  member: MemberView | null;
  /** The broker or the rival as the town's character gives them (the name may be a rumour). */
  person: PersonRefView | null;
}

/**
 * The key figures, in order: the broker (once heard of), the rival, then
 * whoever holds the executive post and the members of the legislative body,
 * as the office view lists them (the constitution's posts, DEC-098: its two
 * fields are the executive and the body, whatever the constitution calls
 * them; the words shown are the posts' role keys). The player is left out;
 * someone in two places is pinned once, at the first.
 */
export function keyFigures(office: OfficeView | null, character: TownCharacterView | null): KeyFigure[] {
  const out: KeyFigure[] = [];
  const add = (f: KeyFigure) => {
    const was = out.find((x) => x.id === f.id);
    if (!was) out.push(f);
    else if (!was.member && f.member) {
      was.member = f.member;
      was.ward = f.ward;
    }
  };
  const fromRef = (p: PersonRefView | null | undefined, pin: "broker" | "rival") => {
    if (p?.id === null || p?.id === undefined) return;
    add({ id: p.id, name: p.name, pin, roleKey: p.role_key, ward: null, member: null, person: p });
  };
  fromRef(character?.broker, "broker");
  fromRef(character?.rival, "rival");
  const executive = office?.mayor ?? null;
  const legislature = office?.council ?? [];
  for (const m of [executive, ...legislature]) {
    if (!m || m.is_player) continue;
    add({ id: m.character, name: m.name, pin: null, roleKey: m.role_key, ward: m.ward_number, member: m, person: null });
  }
  return out;
}

/** The people the notebook knows of, by character id. */
export function personItems(items: readonly NotebookItem[]): Map<number, NotebookItem> {
  const m = new Map<number, NotebookItem>();
  for (const i of items) if (i.subject.type === "person") m.set(i.subject.person, i);
  return m;
}
