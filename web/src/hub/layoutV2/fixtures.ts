// Layout v2 test towns (test-only): real towns from the engine with their
// geography and state views (../fixtures/towns-v2.json, dumped by
// ../fixtures/dumpGeo.test.ts), and seeded random towns and geographies
// (river × coast × relief × class) for property tests.
import towns from "../fixtures/towns-v2.json";
import { Rng } from "../rng";
import { HOOD_KINDS, type TownInput } from "../townInput";
import type { TownGeo } from "./geo";
import { NO_BEARING } from "./imath";

export interface GeoFixture {
  seed: string;
  source: "town" | "largest" | "smallest";
  input: TownInput;
  geo: TownGeo;
  state: { vacancy: number; unemployment: number; hoods: { id: number; population: number }[] };
}

export const GEO_FIXTURES = towns as unknown as GeoFixture[];

/** A random geography for a random town: any band, river or not, coast or not, flat to rugged, links any way. */
export function randomGeo(seed: number): { input: TownInput; geo: TownGeo } {
  const r = new Rng(seed ^ 0x9e0);
  const input = randomInput(seed);
  // Every neighbourhood kind sometimes, and a capital's count sometimes.
  if (r.chance(0.15)) {
    const n = r.int(14, 22);
    input.neighborhoods = Array.from({ length: n }, (_, i) => ({ id: 100 + i * 3, name: `Hood ${i}`, kind: i === 0 ? "core" : r.pick(HOOD_KINDS) }));
    input.places = input.places.map((p) => ({ ...p, neighborhood: r.pick(input.neighborhoods).id }));
    const th = input.places.find((p) => p.kind === "town_hall");
    if (th) th.neighborhood = 100;
  }
  const band = r.int(0, 4);
  const population = [r.int(8_000, 29_000), r.int(30_000, 99_000), r.int(100_000, 299_000), r.int(300_000, 990_000), r.int(1_000_000, 3_500_000)][band] as number;
  // The engine's neighbourhood count follows population (`count_for(pop) = round(√pop / 50)`, 1–22).
  const most = Math.max(1, Math.min(22, Math.round(Math.sqrt(population) / 50) + 1));
  if (input.neighborhoods.length > most) {
    input.neighborhoods = input.neighborhoods.slice(0, most);
    const ids = new Set(input.neighborhoods.map((h) => h.id));
    const first = (input.neighborhoods[0] as { id: number }).id;
    input.places = input.places.map((p) => (ids.has(p.neighborhood) ? p : { ...p, neighborhood: first }));
    if (input.home !== null && !ids.has(input.home)) input.home = first;
    if (input.location.type === "street" && !ids.has(input.location.neighborhood)) input.location = { type: "street", neighborhood: first };
  }
  const per = Math.floor(population / Math.max(1, input.neighborhoods.length));
  const river = r.chance(0.55) ? { class: r.pick([1, 2, 2, 3] as const), enters: r.int(0, 15), leaves: 0, through_centre: r.chance(0.6) } : null;
  if (river) river.leaves = (river.enters + 8 + r.int(-2, 2)) & 15;
  const coast = r.chance(0.3) ? { sea: r.int(0, 15), shore: r.pick(["beach", "rocky", "cliff"] as const), bay: r.chance(0.4) } : null;
  const rug = r.pick([0, 60, 200, 380, 520, 800, 1000]);
  const mineral = r.int(0, 1000);
  const links = Array.from({ length: r.int(1, 7) }, () => ({ mode: r.pick(["road", "road", "highway", "rail"] as const), bearing: r.int(0, 15), to: r.int(0, 50), to_name: r.chance(0.7) ? `Town${r.int(1, 99)}` : "", foreign: false }));
  const geo: TownGeo = {
    municipality: seed,
    name: input.municipality,
    class: band >= 4 ? "metro" : band === 3 ? "city" : band === 0 && r.chance(0.5) ? "rural" : "town",
    regional_capital: r.chance(0.2),
    national_capital: band >= 4 && r.chance(0.5),
    baseline_population: population,
    neighborhoods: input.neighborhoods.map((h) => ({ id: h.id, name: h.name, kind: h.kind, baseline_population: Math.max(500, per + r.int(-per >> 1, per >> 1)), adjacent: [] })),
    places: input.places.map((p) => ({ ...p })),
    baseline_sectors: [],
    river,
    creeks: river && r.chance(0.5) ? [{ enters: r.int(0, 15), joins_river: true }] : r.chance(0.2) ? [{ enters: r.int(0, 15), joins_river: false }] : [],
    coast,
    lake: !coast && r.chance(0.15) ? { bearing: r.int(0, 15), near: r.chance(0.5) } : null,
    relief: { ruggedness_pm: rug, uphill: rug < 80 ? NO_BEARING : r.int(0, 15), high_ground: r.int(0, 15), relief_m: rug / 8 },
    woods: { share_pm: r.int(0, 900), bearing: r.int(0, 15) },
    minerals: { value_pm: mineral, bearing: r.int(0, 15) },
    resources_pm: { fertility: r.int(0, 900), timber: r.int(0, 900), minerals: mineral, fishing: coast ? r.int(0, 900) : 0, energy: r.int(0, 900), scenic: r.int(0, 900) },
    links,
    hazards_pm: { flood: river ? r.int(0, 700) : 0, storm: coast ? r.int(0, 600) : 0, wildfire: r.int(0, 500), seismic: r.int(0, 300), drought: r.int(0, 500) },
    climate: { temperature_c: r.int(-5, 25), precipitation_mm: r.int(200, 1500), biome: "temperate_forest", snow: false },
    wind_from: r.int(0, 15),
    elevation_m: r.int(5, 1500),
    era: null,
  };
  return { input, geo };
}

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

