// The rendered-text corpus (content workbench, owner 2026-11-20): the
// engine's prose items (`mandate-cli content doctor --corpus`, keys and
// arguments with their situation) rendered exactly as the game renders
// them: the game's Translator and the same helpers its screens use
// (decision titles, scene beats, notebook entries, what a conversation
// taught). Pure: the Vitest runner in `corpus.test.ts` reads and writes the
// files. Dev tooling, never shipped.
import { capital } from "../../components/DecisionSheet";
import { subjectText } from "../../components/CharacterSheet";
import { itemFacts, itemTitle, type NotebookItem } from "../../components/notebook/notebookModel";
import { learnedText } from "../../components/talk/talkText";
import type { LearnedItemView } from "../../engine/protocol/LearnedItemView";
import type { NotebookEntryView } from "../../engine/protocol/NotebookEntryView";
import type { NotebookGapView } from "../../engine/protocol/NotebookGapView";
import type { SubjectView } from "../../engine/protocol/SubjectView";
import type { Translator } from "../../i18n/fluent";

/** One item as the CLI writes it (`raw.jsonl`). */
export interface RawItem {
  kind: string;
  part: string;
  id: string;
  voice: string | null;
  render: "event" | "title" | "beat" | "learned" | "entry" | "gap";
  view: Record<string, unknown>;
  situation: Record<string, unknown>;
}

export interface RenderedItem {
  kind: string;
  part: string;
  id: string;
  voice: string | null;
  text: string;
  key: string;
  args: Record<string, string>;
  situation: Record<string, unknown>;
  /** Signs a renderer saw: a missing key (`⟦…⟧`), an unfilled argument (`{$x}`). */
  flags: string[];
}

type KeyArgs = { key: string; args: Record<string, string> };

function notebookText(tr: Translator, item: NotebookItem): string {
  const facts = itemFacts(tr, item);
  const title = itemTitle(tr, item);
  return facts.length > 0 ? `${title}: ${facts.join(" · ")}` : title;
}

/** The text of one item, as its screen shows it. */
export function renderItem(tr: Translator, it: RawItem): { text: string; key: string; args: Record<string, string> } {
  switch (it.render) {
    case "title": {
      const v = it.view as KeyArgs;
      return { text: capital(tr.event(v.key, v.args)), key: v.key, args: v.args };
    }
    case "beat": {
      // As the decision sheet's beat: a quoted memory first, as `$memory`.
      const v = it.view as KeyArgs & { quote: { key: string; subject: SubjectView | null; year: number } | null };
      const args: Record<string, string> = { ...v.args };
      if (v.quote) args.memory = tr.t(v.quote.key, { subject: subjectText(tr, v.quote.subject), year: String(v.quote.year) });
      return { text: tr.event(v.key, args), key: v.key, args };
    }
    case "learned": {
      const v = it.view as unknown as LearnedItemView;
      return { text: learnedText(tr, v), key: v.title_key, args: v.args };
    }
    case "entry": {
      const e = it.view as unknown as NotebookEntryView;
      const item: NotebookItem = { key: "", subject: e.subject, level: e.level, titleKey: e.title_key, args: e.args, entry: e, lead: e.lead, isNew: false };
      return { text: notebookText(tr, item), key: e.title_key, args: e.args };
    }
    case "gap": {
      const g = it.view as unknown as NotebookGapView;
      const item: NotebookItem = { key: "", subject: g.subject, level: "unknown", titleKey: g.title_key, args: g.args, entry: null, lead: g.lead, isNew: false };
      return { text: notebookText(tr, item), key: g.title_key, args: g.args };
    }
    default: {
      const v = it.view as KeyArgs;
      return { text: tr.event(v.key, v.args), key: v.key, args: v.args };
    }
  }
}

function flags(text: string): string[] {
  const out: string[] = [];
  if (text.includes("⟦")) out.push("missing-key");
  if (/\{\s*\$[a-z_]+\s*\}/.test(text)) out.push("unfilled-argument");
  if (/\{\s*[a-z][a-z0-9-]*\s*\}/.test(text)) out.push("unresolved-reference");
  if (/ {2,}| [,.;:!?]/.test(text)) out.push("spacing");
  if (text.trim() === "") out.push("empty");
  return out;
}

/** Render every item; identical renderings of the same item once (different fills are kept). */
export function renderAll(tr: Translator, raw: RawItem[]): RenderedItem[] {
  const seen = new Set<string>();
  const out: RenderedItem[] = [];
  for (const it of raw) {
    const r = renderItem(tr, it);
    const k = `${it.kind}|${it.part}|${it.id}|${it.voice ?? ""}|${r.text}`;
    if (seen.has(k)) continue;
    seen.add(k);
    out.push({ kind: it.kind, part: it.part, id: it.id, voice: it.voice, text: r.text, key: r.key, args: r.args, situation: it.situation, flags: flags(r.text) });
  }
  return out;
}

export interface Counts {
  rendered_items: number;
  by_kind: Record<string, { items: number; ids: number; flagged: number }>;
  flags: Record<string, number>;
}

export function countItems(items: RenderedItem[]): Counts {
  const by: Record<string, { items: number; ids: Set<string>; flagged: number }> = {};
  const fl: Record<string, number> = {};
  for (const i of items) {
    const b = (by[i.kind] ??= { items: 0, ids: new Set(), flagged: 0 });
    b.items++;
    b.ids.add(i.id);
    if (i.flags.length > 0) b.flagged++;
    for (const f of i.flags) fl[f] = (fl[f] ?? 0) + 1;
  }
  const byKind: Counts["by_kind"] = {};
  for (const [k, v] of Object.entries(by)) byKind[k] = { items: v.items, ids: v.ids.size, flagged: v.flagged };
  return { rendered_items: items.length, by_kind: byKind, flags: fl };
}

const KIND_ORDER = ["dilemma", "scene", "storyline", "press", "overheard", "talk", "notebook", "outcome"];

/** The situation in one line: who, where, what, and the facts beside it. */
function situationLine(s: Record<string, unknown>): string {
  const parts: string[] = [];
  const v = (k: string) => s[k];
  parts.push(`seed ${String(v("seed"))} · ${String(v("date"))} · ${String(v("town"))} (${String(v("town_class"))}) · ${String(v("strategy"))}/${String(v("storyteller"))}`);
  const sp = v("speaker") as Record<string, unknown> | undefined;
  if (sp) {
    const traits = (sp.traits as string[] | undefined)?.join(", ");
    parts.push(
      `speaker ${String(sp.name)} (${String(sp.role ?? "-")}${v("voice") ? `, ${String(v("voice"))}` : ""}${traits ? `; ${traits}` : ""}; ${String(sp.opinion_band ?? "-")} ${String(sp.opinion ?? "")}; ${((sp.convictions as string[] | undefined) ?? []).join(", ")}; home worries ${((sp.home_worries as string[] | undefined) ?? []).join(", ")})`,
    );
  }
  if (v("place")) parts.push(`at ${String(v("place"))}${v("hood") ? `, ${String(v("hood"))}` : ""} (worries ${((v("hood_worries") as string[] | undefined) ?? []).join(", ")})`);
  if (v("topic")) parts.push(`topic ${String(v("topic"))}${v("about") ? ` about ${JSON.stringify(v("about"))}` : ""}`);
  parts.push(`town worries ${((v("town_worries") as string[] | undefined) ?? []).join(", ")}; crisis ${String(v("crisis") ?? "none")}; broker ${String(v("broker") ?? "none")}; rival ${String(v("rival") ?? "none")}; player ${String(v("player_post"))}`);
  for (const k of ["cast", "subject", "source", "action", "status", "level", "concern"]) {
    if (v(k) !== undefined && v(k) !== null) parts.push(`${k} ${JSON.stringify(v(k))}`);
  }
  return parts.join("  \n  ");
}

/** The corpus as Markdown, grouped by kind, then by template or line id. */
export function markdown(items: RenderedItem[], counts: Counts): string {
  const lines: string[] = ["# Rendered-text corpus", "", "Every piece of player-visible prose from simulated careers, as the player sees it, with its situation.", ""];
  lines.push("| kind | items | ids | flagged |", "|---|---|---|---|");
  for (const k of KIND_ORDER) {
    const c = counts.by_kind[k];
    if (c) lines.push(`| ${k} | ${c.items} | ${c.ids} | ${c.flagged} |`);
  }
  lines.push("");
  for (const k of KIND_ORDER) {
    const group = items.filter((i) => i.kind === k);
    if (group.length === 0) continue;
    lines.push(`## ${k}`, "");
    const ids = [...new Set(group.map((i) => i.id))].sort();
    for (const id of ids) {
      lines.push(`### ${id}`, "");
      for (const i of group.filter((g) => g.id === id)) {
        const tag = [i.part, i.voice].filter(Boolean).join(" · ");
        const fl = i.flags.length > 0 ? ` **[${i.flags.join(", ")}]**` : "";
        lines.push(`- ${tag ? `*${tag}*: ` : ""}${i.text}${fl}  `, `  ${situationLine(i.situation)}`);
      }
      lines.push("");
    }
  }
  return lines.join("\n");
}
