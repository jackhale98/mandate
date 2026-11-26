// "Why?" (the causal spine, DEC-108): the engine's WhyView laid out as a short
// sequence a player can read, newest at the top. Presentation only. The
// engine decides what led to what, what the player may see and whose act is
// behind it; this puts its steps in reading order and words each typed value
// with its Fluent line (`why-…`, content/base/locale/en/why.ftl). Nothing
// here is computed about the world.
import type { CauseView } from "../../engine/protocol/CauseView";
import type { WhyStepView } from "../../engine/protocol/WhyStepView";
import type { WhyView } from "../../engine/protocol/WhyView";
import type { Translator } from "../../i18n/fluent";
import { formatDate } from "../../i18n/format";

/** Where a chain starts: every kind of cause but a link to an earlier event. */
export type RootKind = Exclude<CauseView["type"], "event">;

export interface WhyRoot {
  kind: RootKind;
  text: string;
}

/** One step of the chain, as it is read. */
export interface WhyRow {
  id: number;
  /** Links back from the event asked about (which is 0). */
  depth: number;
  date: string;
  /** The step's sentence: the engine's event line. */
  text: string;
  /** How it joins the later step it led to ("because of", "after", …); null for the event asked about. */
  join: string | null;
  /** The later step it joins, in that step's words, when that is not the row just above (a second line of causes). */
  from: string | null;
  /** What the step names as a start: the player's act, someone's choice, a process, a pressure, a shock. */
  roots: WhyRoot[];
}

export interface WhyLayout {
  rows: WhyRow[];
  /** The player's act nearest behind the event, and the day of the step that names it. */
  player: { text: string; date: string } | null;
  /** More lies further back than is shown. */
  more: boolean;
  /** A longer walk back can still be asked for. */
  further: boolean;
}

/** The first letter as a capital (a blanked name can start a sentence). */
function sentence(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

/**
 * A step's sentence. A name the engine left empty (someone the player has
 * not heard of, DEC-091) reads as "someone you have not heard of".
 */
export function stepText(tr: Translator, s: Pick<WhyStepView, "text_key" | "args">): string {
  const args: Record<string, string> = {};
  for (const [k, v] of Object.entries(s.args)) args[k] = v === "" && !k.endsWith("_key") ? tr.t("ui-why-name-unknown") : v;
  return sentence(tr.event(s.text_key, args));
}

/** A cause that starts a chain, in words; null for a link to an earlier event. */
export function rootOf(tr: Translator, c: CauseView): WhyRoot | null {
  switch (c.type) {
    case "event":
      return null;
    case "player":
      return { kind: "player", text: tr.t(`why-player-${c.act}`) };
    case "actor":
      return { kind: "actor", text: c.person && c.person.name !== "" ? tr.t("why-actor", { name: c.person.name }) : tr.t("why-actor-unnamed") };
    case "process":
      return { kind: "process", text: tr.t(`why-process-${c.process}`) };
    case "pressure":
      // A worry names its concern as said in a sentence ("the rent").
      if (c.concern_key) return { kind: "pressure", text: tr.t(`why-pressure-${c.pressure}-named`, { concern: tr.t(c.concern_key) }) };
      return { kind: "pressure", text: tr.t(`why-pressure-${c.pressure}`) };
    case "exogenous":
      return { kind: "exogenous", text: tr.t(`why-exogenous-${c.shock}`) };
    case "history":
      return { kind: "history", text: tr.t("why-history") };
    case "forgotten":
      return { kind: "forgotten", text: tr.t("why-forgotten") };
    case "unknown":
      return { kind: "unknown", text: tr.t("why-unknown") };
  }
}

/**
 * The steps in reading order: the event, then each line of causes followed
 * back before the next one starts (the engine gives them nearest first,
 * level by level; read that way a second line of causes would cut into the
 * first). Every step the engine gave is kept, once.
 */
export function readingOrder(chain: readonly WhyStepView[]): WhyStepView[] {
  const out: WhyStepView[] = [];
  const seen = new Set<number>();
  const visit = (s: WhyStepView) => {
    if (seen.has(s.id)) return;
    seen.add(s.id);
    out.push(s);
    for (const next of chain) if (next.led_to?.event === s.id) visit(next);
  };
  for (const s of chain) if (s.led_to === null) visit(s);
  // A step whose later end is not among the steps still shows.
  for (const s of chain) visit(s);
  return out;
}

/** The view as rows to read, with the line about the player's own act. */
export function layoutWhy(tr: Translator, view: WhyView): WhyLayout {
  const texts = new Map<number, string>();
  for (const s of view.chain) texts.set(s.id, stepText(tr, s));
  const ordered = readingOrder(view.chain);
  const rows = ordered.map((s, i): WhyRow => {
    const roots: WhyRoot[] = [];
    for (const c of s.causes) {
      const r = rootOf(tr, c);
      if (r && !roots.some((x) => x.text === r.text)) roots.push(r);
    }
    const above = ordered[i - 1];
    return {
      id: s.id,
      depth: s.depth,
      date: formatDate(tr, s.date),
      text: texts.get(s.id) ?? "",
      join: s.led_to ? tr.t(`why-link-${s.led_to.link}`) : null,
      from: s.led_to && above && above.id !== s.led_to.event ? (texts.get(s.led_to.event) ?? null) : null,
      roots,
    };
  });
  const named = view.player ? view.chain.find((s) => s.id === view.player?.event) : undefined;
  return {
    rows,
    player: view.player ? { text: tr.t(`why-player-${view.player.act}`), date: named ? formatDate(tr, named.date) : "" } : null,
    more: view.truncated,
    further: view.truncated && view.depth < view.max_depth,
  };
}
