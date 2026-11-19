// Layout v2 test towns (test-only): real towns from the engine with the
// geography the stage A adapter derives for them (../fixtures/towns-v2.json,
// dumped by ../fixtures/dumpGeo.test.ts), and seeded random geographies
// (river × coast × relief × class) for property tests.
import towns from "../fixtures/towns-v2.json";
import { Rng } from "../rng";
import { randomInput } from "../testTowns";
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
