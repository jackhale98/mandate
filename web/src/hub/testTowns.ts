// Towns for the hub tests (test-only): the layout v2 maps of the real towns
// dumped from the engine (./layoutV2/fixtures, ./fixtures/towns-v2.json) and
// of seeded random geographies, planned once per test file.
import { GEO_FIXTURES, randomGeo, type GeoFixture } from "./layoutV2/fixtures";
import { layoutTownV2, type TownPlan } from "./layoutV2";
import type { TownInput } from "./townInput";
import { townSeed, type TownMap } from "./townLayout";

export type { GeoFixture };
export const FIXTURES: readonly GeoFixture[] = GEO_FIXTURES;

const real = new Map<number, { plan: TownPlan; map: TownMap }>();
/** The v2 plan and map of real fixture `i` (its own town seed, its state as play began). */
export function fixtureTown(i: number): { plan: TownPlan; map: TownMap; fixture: GeoFixture } {
  const f = FIXTURES[i] as GeoFixture;
  let t = real.get(i);
  if (!t) {
    t = layoutTownV2(f.input, f.geo, townSeed(f.seed, f.input.municipality), { home: f.input.home, location: f.input.location, hoods: f.state.hoods, vacancy: f.state.vacancy, unemployment: f.state.unemployment });
    real.set(i, t);
  }
  return { ...t, fixture: f };
}

/** The v2 map of a seeded random geography (./layoutV2/fixtures `randomGeo`). */
export function randomTown(seed: number): { map: TownMap; input: TownInput } {
  const { input, geo } = randomGeo(seed);
  return { map: layoutTownV2(input, geo, seed).map, input };
}
