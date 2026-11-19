// Text for the town map overview (DEC-086). The engine owns the Fluent
// strings (content/base/locale); keys the content pack does not have yet
// fall back to English here, so the map works before they are added
// (the same pattern as `ui-agenda-choose`). Presentation only.
import type { FluentVariable } from "@fluent/bundle";
import type { Translator } from "../i18n/fluent";

const FALLBACK: Record<string, string> = {
  "ui-town-view": "Town view",
  "ui-town-view-map": "Map",
  "ui-town-view-list": "List",
  "ui-town-view-list-hint": "text view of every place",
  "ui-town-map-label": "Map of { $town }: { $n } places to go. Tap one to go there; the List view has the same places as text.",
  "ui-town-map-help": "Drag to pan, pinch or scroll to zoom. Arrow keys move between places, Enter goes there.",
  "ui-town-map-fit": "Show the whole town",
  "ui-town-map-zoom-in": "Zoom in",
  "ui-town-map-zoom-out": "Zoom out",
  "ui-town-map-waiting": "{ $what } waiting",
  "ui-town-map-vote": "Vote to cast",
  "ui-town-map-quiet": "{ $place }: nothing to do here yet.",
};

/** Translate a key, or use the English fallback when the content pack does not have it yet. */
export function say(tr: Translator, key: string, args?: Record<string, FluentVariable>): string {
  if (tr.has(key)) return tr.t(key, args);
  const f = FALLBACK[key];
  if (f === undefined) return tr.t(key, args);
  return f.replace(/\{ ?\$(\w+) ?\}/g, (_, k: string) => String(args?.[k] ?? ""));
}
