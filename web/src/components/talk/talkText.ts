// Words for the talk panel from the engine's facts and keys (DEC-054): no
// figure is worked out here, only formatted.
import type { LearnedItemView } from "../../engine/protocol/LearnedItemView";
import type { Translator } from "../../i18n/fluent";

/** The engine's odds as a whole percentage ("62% chance"). */
export function oddsText(tr: Translator, odds: number): string {
  return tr.t("ui-talk-odds", { pct: Math.round(odds * 100) });
}

/** A subject's name: its own (a person, a neighbourhood, a named place), else its kind's or policy's name. */
export function aboutText(tr: Translator, label: string | null, labelKey: string | null): string {
  if (label) return label;
  return labelKey ? tr.t(labelKey) : "";
}

/** What a notebook entry learned in a conversation says: its title, then what the level reveals (the worries, a name, a seat). */
export function learnedText(tr: Translator, l: LearnedItemView): string {
  const a = l.args;
  const title = tr.event(l.title_key, a);
  const details: string[] = [];
  for (const k of ["concern_key", "concern2_key", "concern3_key"]) {
    const v = a[k];
    if (v) details.push(tr.t(v));
  }
  if (l.subject.type === "broker" && a.name) details.push(a.rumoured === "true" ? `${a.name} (${tr.t("knowledge-rumoured")})` : a.name);
  if (l.subject.type === "person" && a.role_key) details.push(tr.t(a.role_key));
  if (a.stake_key) details.push(tr.t(a.stake_key));
  if (a.seat_key) details.push(tr.t(a.seat_key, a.ward ? { ward: a.ward } : undefined));
  if (a.focus_key) details.push(tr.t(a.focus_key));
  if (a.band_key) details.push(tr.t(a.band_key));
  return details.length > 0 ? `${title}: ${details.join(", ")}` : title;
}
