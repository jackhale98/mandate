// The words and choices of dialogue mode (DEC-093), from the engine's views
// (`TalkView`, `ExchangeView`): which topics can be raised and why not, the
// subjects of a topic grouped by kind, the cost line, and a reply split into
// pages short enough for a speech bubble on a 360 px phone. Nothing here
// decides an outcome: the odds, availability and lines are the engine's;
// this only formats and pages them.
import type { AboutChoiceView } from "../../engine/protocol/AboutChoiceView";
import type { ExchangeView } from "../../engine/protocol/ExchangeView";
import type { SpokenLineView } from "../../engine/protocol/SpokenLineView";
import type { TalkView } from "../../engine/protocol/TalkView";
import type { TopicView } from "../../engine/protocol/TopicView";
import type { Translator } from "../../i18n/fluent";
import { personName } from "../Knowing";
import { aboutText } from "./talkText";
import { topicSay } from "../asks/choiceText";

/** A topic as a line in the dialogue box. */
export interface TopicRow {
  topic: TopicView;
  /** It is about something: choosing it opens the subject step ("›"). */
  opens: boolean;
  /** It can be raised now (for a topic about something: on one of its subjects at least). */
  available: boolean;
  /** Why not (a `talk-why-*` key), when not available. */
  why: string | null;
  /** The engine's odds, where the topic itself is rolled (a topic about something shows them per subject). */
  odds: number | null;
}

export function topicRow(t: TopicView): TopicRow {
  if (t.about.length === 0) return { topic: t, opens: false, available: t.available, why: t.available ? null : t.unavailable_key, odds: t.odds };
  // On a subject, or (where it may be about nothing) on nothing in particular.
  const available = t.about.some((c) => c.available) || (!t.about_required && t.available);
  // Not on any: the subjects' own reason (out of time, not here), else the topic's.
  const why = available ? null : ((t.about_required ? t.about.find((c) => c.unavailable_key)?.unavailable_key : null) ?? t.unavailable_key);
  return { topic: t, opens: true, available, why, odds: null };
}

/** Subjects by kind, in the order the engine lists them; `q` keeps those whose name contains it. */
export function subjectGroups(tr: Translator, list: readonly AboutChoiceView[], q = ""): [kind: string, items: AboutChoiceView[]][] {
  const needle = q.trim().toLocaleLowerCase();
  const out: [string, AboutChoiceView[]][] = [];
  for (const c of list) {
    if (needle && !aboutText(tr, c.label, c.label_key, c.label_args).toLocaleLowerCase().includes(needle)) continue;
    const g = out.find(([k]) => k === c.kind);
    if (g) g[1].push(c);
    else out.push([c.kind, [c]]);
  }
  return out;
}

/**
 * A topic's subjects as the engine orders them: those the person could be
 * expected to know (`likely`, listed first), and the rest (for "Something
 * else…"). The order within each is the engine's.
 */
export function subjectSplit(list: readonly AboutChoiceView[]): { likely: AboutChoiceView[]; others: AboutChoiceView[] } {
  return { likely: list.filter((c) => c.likely), others: list.filter((c) => !c.likely) };
}

/** A subject step longer than this gets a search box. */
export const SEARCH_FROM = 8;

/** The cost in a few words ("½ slot · 2½ left"), and in a sentence for screen readers; null when out of time. */
export function costText(tr: Translator, v: TalkView): { short: string; long: string } | null {
  if (v.out_of_time) return null;
  if (v.late_night) return { short: tr.t("ui-dlg-cost-late"), long: tr.t("ui-talk-cost-late") };
  // Whole slots left, and the half left over from the last conversation ("2½").
  const left = v.half_slot_left ? (v.slots_left > 0 ? `${v.slots_left}½` : "½") : String(v.slots_left);
  const half = v.cost_halves <= 1;
  return {
    short: tr.t(half ? "ui-dlg-cost-half" : "ui-dlg-cost-slot", { left }),
    long: v.half_slot_left ? tr.t("ui-talk-cost-half-left") : tr.t(half ? "ui-talk-cost-half" : "ui-talk-cost-slot", { left: v.slots_left }),
  };
}

/** What the player says, as their bubble shows it: the topic, and its subject. */
export function questionText(tr: Translator, t: TopicView, about: AboutChoiceView | null): string {
  const topic = topicSay(tr, t);
  return about ? tr.t("ui-dlg-asked-about", { topic, about: aboutText(tr, about.label, about.label_key, about.label_args) }) : topic;
}

/** One stretch of a reply: a stage direction (italic: an opener, an unquoted closer, a change of mood) or a spoken line. */
export interface Segment {
  text: string;
  stage: boolean;
}
export type Page = Segment[];

/** About five lines of a bubble on a 360 px phone. */
export const PAGE_CHARS = 170;

/** A long line in pieces of at most `limit` characters, broken between sentences (or, failing that, between words). */
export function splitLong(text: string, limit = PAGE_CHARS): string[] {
  if (text.length <= limit) return [text];
  const sentences = text.split(/(?<=[.!?…]["”’)]?)\s+/);
  const out: string[] = [];
  let cur = "";
  const flush = () => {
    if (cur) out.push(cur);
    cur = "";
  };
  for (const s of sentences) {
    if (s.length > limit) {
      // One very long sentence: by words.
      flush();
      for (const w of s.split(/\s+/)) {
        if (cur && cur.length + 1 + w.length > limit) flush();
        cur = cur ? `${cur} ${w}` : w;
      }
      continue;
    }
    if (cur && cur.length + 1 + s.length > limit) flush();
    cur = cur ? `${cur} ${s}` : s;
  }
  flush();
  return out;
}

/**
 * A reply as speech-bubble pages, in order: the parts as spoken (an opener
 * is a stage direction), then the leads in the speaker's voice, then how
 * their mood moved (a stage direction). A page holds as much as fits in
 * `limit` characters; a stage direction keeps to the line after it.
 */
export function replyPages(tr: Translator, x: ExchangeView, limit = PAGE_CHARS): Page[] {
  const segs: Segment[] = x.parts.map((p) => {
    const text = tr.event(p.key, p.args);
    // An opener is a stage direction; so is a closer that is not in quotes (the speaker smiles, turns away).
    return { text, stage: p.part === "opener" || (p.part === "closer" && !/^["“‘'«]/.test(text)) };
  });
  for (const l of x.leads) segs.push({ text: tr.event(l.key, l.args), stage: false });
  if (x.relationship) segs.push({ text: tr.t(x.relationship === "warmer" ? "ui-talk-warmer" : "ui-talk-cooler", { name: personName(tr, x.speaker.name) }), stage: true });
  const pages: Page[] = [];
  let cur: Segment[] = [];
  let len = 0;
  for (const seg of segs) {
    for (const text of splitLong(seg.text, limit)) {
      if (!text) continue;
      if (cur.length > 0 && len + text.length > limit) {
        // A stage direction goes on with the line it introduces.
        const carry = !seg.stage && cur.length > 1 && cur[cur.length - 1]?.stage ? (cur.pop() as Segment) : null;
        pages.push(cur);
        cur = carry ? [carry] : [];
        len = carry ? carry.text.length : 0;
      }
      cur.push({ text, stage: seg.stage });
      len += text.length;
    }
  }
  if (cur.length > 0) pages.push(cur);
  return pages;
}

/** The greeting (what the person says as the conversation opens, before any topic): one page, a stage direction as an opener is. */
export function greetingPage(tr: Translator, g: SpokenLineView): Page {
  return [{ text: tr.event(g.key, g.args), stage: true }];
}

/** A page's text as one string (the live region, the typewriter's length). */
export function pageText(p: Page): string {
  return p.map((s) => s.text).join(" ");
}
