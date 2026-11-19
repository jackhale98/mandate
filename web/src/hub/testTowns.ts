// Towns for the hub tests: real ones dumped from the engine
// (fixtures/dump-towns.mjs) and seeded random ones. Test-only.
import towns from "./fixtures/towns.json";
import { Rng } from "./rng";
import { HOOD_KINDS, type TownInput } from "./townInput";

export interface Fixture {
  seed: string;
  source: "town" | "largest";
  input: TownInput;
}
export const FIXTURES = towns as Fixture[];

/** Place kinds from content/base/places.jsonc (worldgen's PlaceKind). */
export const PLACE_KINDS = [
  "town_hall",
  "council_chamber",
  "diner",
  "market",
  "school",
  "clinic",
  "community_centre",
  "park",
  "news_office",
  "party_office",
  "union_hall",
  "business_association",
  "factory",
  "port",
  "farm",
  "mine",
  "mill",
  "hotel",
  "university",
  "tech_campus",
  "regional_assembly",
  "parliament",
  "executive_residence",
];

/** A random but plausible town: 1-14 neighbourhoods, civic places in the first, others anywhere. */
export function randomInput(seed: number): TownInput {
  const r = new Rng(seed);
  const n = r.int(1, 14);
  const neighborhoods = Array.from({ length: n }, (_, i) => ({ id: 100 + i * 3, name: `Hood ${i}`, kind: i === 0 && r.chance(0.8) ? "core" : r.pick(HOOD_KINDS) }));
  const places: TownInput["places"] = [];
  let id = 500;
  const add = (kind: string, hood: number) => places.push({ id: id++, kind, name: r.chance(0.3) ? `The ${kind}` : "", neighborhood: hood });
  const centre = (neighborhoods[0] as { id: number }).id;
  if (r.chance(0.9)) add("town_hall", centre);
  if (r.chance(0.9)) add("council_chamber", r.chance(0.9) ? centre : r.pick(neighborhoods).id);
  if (r.chance(0.2)) add("parliament", centre);
  if (r.chance(0.2)) add("regional_assembly", centre);
  const extra = r.int(0, 24);
  for (let k = 0; k < extra; k++) add(r.pick(PLACE_KINDS.slice(2)), r.pick(neighborhoods).id);
  const home = r.chance(0.85) ? r.pick(neighborhoods).id : null;
  const someHood = r.pick(neighborhoods).id;
  const loc = r.next();
  const location: TownInput["location"] =
    loc < 0.4 || places.length === 0 ? (home !== null ? { type: "home" } : { type: "street", neighborhood: someHood }) : loc < 0.7 ? { type: "place", place: r.pick(places).id } : { type: "street", neighborhood: someHood };
  return { municipality: `Town ${seed}`, neighborhoods, places, home, location };
}

