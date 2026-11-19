// Text for the town screen: the street, the town map overview (DEC-086), the
// bar and the pocket's tabs (DEC-088 main screen). The engine owns the Fluent
// strings (content/base/locale); keys the content pack does not have yet
// fall back to English here, so the screen works before they are added
// (the same pattern as `ui-agenda-choose`). Presentation only.
import type { FluentVariable } from "@fluent/bundle";
import type { Translator } from "../i18n/fluent";

const FALLBACK: Record<string, string> = {
  "ui-town-map": "Town map",
  "ui-town-map-open": "Map",
  "ui-town-map-close": "Close the map",
  "ui-town-map-pick": "tap a place to walk there",
  "ui-town-map-label": "Map of { $town }: { $n } places to go. Tap one to walk there; the pocket's Places tab has the same places as text.",
  "ui-town-map-help": "Drag to pan, pinch or scroll to zoom. Arrow keys move between places, Enter goes there.",
  "ui-town-map-fit": "Show the whole town",
  "ui-town-map-zoom-in": "Zoom in",
  "ui-town-map-zoom-out": "Zoom out",
  "ui-town-map-waiting": "{ $what } waiting",
  "ui-town-map-vote": "Vote to cast",
  "ui-town-map-quiet": "{ $place }: nothing to do here yet.",
  "ui-town-street": "Street",
  "ui-town-street-label": "The streets of { $town }, drawn. Tap a building to walk to it; the pocket's Places tab has every place as text.",
  "ui-town-street-help": "Tap a building to walk there. Drag to look around, pinch or scroll to zoom.",
  "ui-town-street-loading": "Drawing the streets…",
  "ui-town-planning": "Laying out the town…",
  "ui-passage-day": "A day later",
  "ui-passage-week": "A week later",
  "ui-passage-weeks": "{ $n } weeks later",
  "ui-passage-days": "{ $n } days later",
  "ui-passage-working": "The days pass…",
  "ui-passage-skip": "Tap to carry on",
  "ui-skip-places": "Skip to Places",
  "ui-game-bar": "Game",
  "ui-pocket-tabs": "Pocket sections",
  "ui-pocket-tab-today": "Today",
  "ui-pocket-tab-town": "Town",
  "ui-pocket-tab-places": "Places",
  "ui-pocket-tab-people": "People",
  "ui-pocket-dock": "Keep the pocket open beside the town",
  "ui-pocket-dock-hint": "Dock the pocket beside the town",
  "ui-pocket-undock-hint": "Let the pocket close when you go somewhere",
  "ui-notebook-new-count": "{ $n } new in your notebook",
  "ui-places-help": "Every place in town. Choose one to go straight there and open it.",
  "ui-town-hoods": "Neighbourhoods",
  "ui-town-hood-worry": "Worried most about { $concern }",
  "ui-people-town": "Who matters in town",
  "ui-people-help": "Tap a name to see how they stand with you.",
  "ui-people-learned": "You learned { $n } new things this turn.",
};

/** Translate a key, or use the English fallback when the content pack does not have it yet. */
export function say(tr: Translator, key: string, args?: Record<string, FluentVariable>): string {
  if (tr.has(key)) return tr.t(key, args);
  const f = FALLBACK[key];
  if (f === undefined) return tr.t(key, args);
  return f.replace(/\{ ?\$(\w+) ?\}/g, (_, k: string) => String(args?.[k] ?? ""));
}
