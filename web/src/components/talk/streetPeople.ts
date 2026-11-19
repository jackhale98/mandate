// The people on the street the player is on, as the walkable town stands them
// about (the talk hook, DEC-093): who is here comes from the engine (the
// place view's `people`); this only names them and picks a look (../portrait/look:
// the same look the conversation's portrait draws). A stranger is named by
// their role ("a retiree"), as everywhere (DEC-091).
import type { PersonRefView } from "../../engine/protocol/PersonRefView";
import type { Translator } from "../../i18n/fluent";
import { personName } from "../Knowing";
import { roleText } from "../CharacterSheet";
import { lookVariant, personKey } from "../portrait/look";

export interface StreetPersonEntry {
  /** `c<id>` for someone stored, `a<slot>` for a passer-by. */
  key: string;
  /** The name tag. */
  label: string;
  /** How hints and announcements name them. */
  name: string;
  /** Their look (a stable pick from the key). */
  variant: number;
}

/** How a person is named in the street and in a conversation: their name, or their role for a stranger ("Retiree"). */
export function displayName(tr: Translator, p: PersonRefView): string {
  const role = roleText(tr, p);
  return p.name === "" ? (role ? capitalise(role) : personName(tr, p.name)) : p.name;
}

/** Who stands in the street: everyone listed who can be talked to here (stored people and passers-by). */
export function streetPeople(tr: Translator, people: readonly PersonRefView[]): StreetPersonEntry[] {
  const out: StreetPersonEntry[] = [];
  for (const p of people) {
    if (!p.can_talk) continue;
    const key = personKey(p);
    if (key === null) continue;
    const name = displayName(tr, p);
    out.push({ key, label: name, name, variant: lookVariant(key) });
  }
  return out;
}

function capitalise(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}
