// The people on the street the player is on, as the walkable town stands them
// about (the talk hook, DEC-093): who is here comes from the engine (the
// place view's `people`); this only names them and picks a look. A stranger
// is named by their role ("a retiree"), as everywhere (DEC-091).
import type { PersonRefView } from "../../engine/protocol/PersonRefView";
import type { Translator } from "../../i18n/fluent";
import { personName } from "../Knowing";
import { roleText } from "../CharacterSheet";

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

/** A stable small hash of a string (FNV-1a). */
function hash(s: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h;
}

/** Who stands in the street: everyone listed who can be talked to here (stored people and passers-by). */
export function streetPeople(tr: Translator, people: readonly PersonRefView[]): StreetPersonEntry[] {
  const out: StreetPersonEntry[] = [];
  for (const p of people) {
    if (!p.can_talk) continue;
    const key = p.id !== null ? `c${p.id}` : p.ambient_slot !== null ? `a${p.ambient_slot}` : null;
    if (key === null) continue;
    const role = roleText(tr, p);
    const name = p.name === "" ? (role ? capitalise(role) : personName(tr, p.name)) : p.name;
    out.push({ key, label: name, name, variant: 100 + (hash(key) % 997) });
  }
  return out;
}

function capitalise(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}
