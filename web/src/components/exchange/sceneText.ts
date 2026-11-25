// The scene's words (EXCHANGE_SCENES §7.2, §7.4): every number, range, step
// and lean here is the engine's (`ArcSceneView`); this file only says them.
// Glyphs are for the eyes; each has its words for a screen reader.
import type { ArcSceneView } from "../../engine/protocol/ArcSceneView";
import type { SceneChangeView } from "../../engine/protocol/SceneChangeView";
import type { SceneGroupView } from "../../engine/protocol/SceneGroupView";
import type { SceneLeanView } from "../../engine/protocol/SceneLeanView";
import type { SceneOptionView } from "../../engine/protocol/SceneOptionView";
import type { TextView } from "../../engine/protocol/TextView";
import type { Translator } from "../../i18n/fluent";

export const say = (tr: Translator, t: TextView, extra?: Record<string, string>) => tr.event(t.key, extra ? { ...t.args, ...extra } : t.args);

/** A group's names: full (for words), short (for a chip). */
export function groupName(tr: Translator, v: ArcSceneView, key: string, short = false): string {
  const g = v.groups.find((x) => x.key === key);
  if (!g) return "";
  return say(tr, short ? g.short : g.label);
}

/** "▲▲", "▼", "?" (a range whose ends differ in sign), "·" (no change). */
export function arrows(c: { lo_steps: number; hi_steps: number }): string {
  const { lo_steps: lo, hi_steps: hi } = c;
  if (lo !== hi && (Math.sign(lo) !== Math.sign(hi) || lo === 0 || hi === 0)) return "?";
  const n = Math.abs(lo) > Math.abs(hi) ? lo : hi;
  if (n === 0) return "·";
  return (n > 0 ? "▲" : "▼").repeat(Math.min(3, Math.abs(n)));
}

/** A change in words: "Lands well with Quarryrow", "Not known how it lands with Millfield". */
export function landsText(tr: Translator, v: ArcSceneView, c: SceneChangeView, short = false): string {
  const group = groupName(tr, v, c.group, short);
  const a = arrows(c);
  if (a === "?") return tr.t("ui-scene-lands-unknown", { group });
  const n = Math.max(Math.abs(c.lo_steps), Math.abs(c.hi_steps));
  if (n === 0) return tr.t("ui-scene-lands-none", { group });
  const up = c.lo_steps + c.hi_steps > 0;
  return tr.t(`ui-scene-lands-${up ? "up" : "down"}-${Math.min(3, n)}`, { group });
}

const signed = (n: number) => (n > 0 ? `+${n}` : n < 0 ? `−${-n}` : "0");

/** A change as numbers: "+6", or "−2 to +8" where the player does not know exactly. */
export function pointsText(tr: Translator, c: SceneChangeView): string {
  return c.exact ? signed(c.lo) : tr.t("ui-scene-range", { lo: signed(c.lo), hi: signed(c.hi) });
}

/** A cost or note, short for a row's tag (its `-short` form, where it has one). */
export function shortNote(tr: Translator, n: TextView): string | null {
  const key = `${n.key}-short`;
  return tr.has(key) ? tr.event(key, n.args) : null;
}

/** A row's accessible name: its label and its tags in words (§7.4). */
export function optionName(tr: Translator, v: ArcSceneView, o: SceneOptionView): string {
  const parts = [say(tr, o.label)];
  for (const t of o.tags) parts.push(landsText(tr, v, t));
  if (o.chance !== null) parts.push(tr.t("ui-scene-chance", { pct: o.chance }));
  const costs = o.notes.map((n) => shortNote(tr, n)).filter((x): x is string => x !== null);
  parts.push(costs.length > 0 ? costs.join(". ") : tr.t("ui-scene-no-cost"));
  return parts.join(". ").replace(/\.\./g, ".");
}


/** A chip in words: "Quarryrow: leaning your way; moved towards you." */
export function chipName(tr: Translator, g: SceneGroupView): string {
  const name = say(tr, g.label);
  const lean = tr.t(g.lean_key);
  const moved = g.moved_steps === null || g.moved_steps === 0 ? null : tr.t(g.moved_steps > 0 ? "ui-scene-moved-up" : "ui-scene-moved-down");
  return moved ? tr.t("ui-scene-chip-moved", { group: name, lean, moved }) : tr.t("ui-scene-chip", { group: name, lean });
}

/** The leaning of a result's before or after. */
export function leanText(tr: Translator, lean: SceneLeanView | null): string {
  if (!lean) return tr.t("ui-scene-not-known");
  return tr.t("ui-scene-lean-points", { lean: tr.t(lean.key), n: signed(lean.points) });
}

/** A change over the scene: its points where known, else its words. */
export function resultChange(tr: Translator, g: { steps: number; points: number | null; moved_key: string }): string {
  return g.points !== null ? signed(g.points) : tr.t(g.moved_key);
}
