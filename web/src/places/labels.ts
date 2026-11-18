// How places are named and pictured (presentation only; the engine's place
// registry says what each kind offers). Emoji are decorative: every place is
// also named in text (DEC-051).
import type { LocationView } from "../engine/protocol/LocationView";
import type { Translator } from "../i18n/fluent";

const ICONS: Record<string, string> = {
  home: "🏠",
  streets: "🚶",
  council_chamber: "🏛️",
  town_hall: "🏢",
  diner: "☕",
  market: "🧺",
  school: "🏫",
  clinic: "🏥",
  community_centre: "🏘️",
  park: "🌳",
  news_office: "📰",
  party_office: "🎗️",
  union_hall: "✊",
  business_association: "💼",
  factory: "🏭",
  port: "⚓",
  farm: "🌾",
  mine: "⛏️",
  mill: "🪵",
  hotel: "🏨",
  university: "🎓",
  tech_campus: "💻",
  regional_assembly: "🏛️",
  parliament: "🏛️",
  executive_residence: "🏰",
};

export function placeIcon(kind: string): string {
  return ICONS[kind] ?? "📍";
}

/** A location's display name: its proper name, or its kind (streets name their neighbourhood). */
export function placeName(tr: Translator, l: LocationView): string {
  if (l.kind_key === "streets") return tr.t("ui-streets-of", { hood: l.neighborhood_name });
  const kind = tr.t(`place-${l.kind_key}`);
  return l.name ? `${l.name}` : kind;
}

/** Fluent key of an action's display name. */
export function actionKey(action: string, late = false): string {
  return late ? `action-${action.replace(/_/g, "-")}-late` : `action-${action.replace(/_/g, "-")}`;
}
