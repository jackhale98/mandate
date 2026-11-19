// Town layout v2 (DEC-090): plan + dress, beside v1 (../townLayout, which
// stays the default everywhere; DEC-085 keeps v1 callable for runs that
// started with it). Stage A: selected only by `?layout=v2` in the walkable
// town prototype.
import type { TownInput } from "../townInput";
import type { TownMap } from "../townLayout";
import { dressTown, type TownState } from "./dress";
import type { GeoHood, TownGeo } from "./geo";
import { NO_BEARING } from "./imath";
import { LAYOUT_V2, planTown, type TownPlan } from "./plan";
import { BASE_RECIPE, type Recipe } from "./registry";

export { dressTown, dressHash, stateFromViews, type TownState } from "./dress";
export { LAYOUT_V2, planHash, planTown, type PlanDistrict, type TownPlan } from "./plan";
export { BASE_RECIPE, validateRecipe, type Recipe } from "./registry";
export type { TownGeo } from "./geo";

/** Plan and dress a town in one call: the geography's places are the input's (the engine's list wins). */
export function layoutTownV2(input: TownInput, geo: TownGeo, seed: number, state: Partial<TownState> = {}, recipe: Recipe = BASE_RECIPE): { plan: TownPlan; map: TownMap } {
  const g: TownGeo = { ...geo, places: input.places.map((p) => ({ id: p.id, kind: p.kind, name: p.name, neighborhood: p.neighborhood })) };
  // Neighbourhoods the geography does not know (a fixture without one) join at a nominal population.
  const known = new Set(g.neighborhoods.map((h) => h.id));
  const extra: GeoHood[] = input.neighborhoods.filter((h) => !known.has(h.id)).map((h) => ({ id: h.id, name: h.name, kind: h.kind, baseline_population: Math.max(1000, Math.floor(g.baseline_population / Math.max(1, input.neighborhoods.length))), adjacent: [] }));
  g.neighborhoods = [...g.neighborhoods.filter((h) => input.neighborhoods.some((x) => x.id === h.id)), ...extra];
  const plan = planTown(g, seed, LAYOUT_V2, recipe);
  const map = dressTown(plan, { home: input.home, location: input.location, ...state });
  return { plan, map };
}

/** A plain geography for a town the engine has described only by its places (tests, and towns without map data): flat, inland, roads four ways. */
export function geoFromInput(input: TownInput, population = 60_000): TownGeo {
  const per = Math.max(1000, Math.floor(population / Math.max(1, input.neighborhoods.length)));
  return {
    municipality: 0,
    name: input.municipality,
    class: population >= 1_000_000 ? "metro" : population >= 300_000 ? "city" : population >= 20_000 ? "town" : "rural",
    regional_capital: false,
    national_capital: false,
    baseline_population: population,
    neighborhoods: input.neighborhoods.map((h) => ({ id: h.id, name: h.name, kind: h.kind, baseline_population: per, adjacent: [] })),
    places: input.places.map((p) => ({ id: p.id, kind: p.kind, name: p.name, neighborhood: p.neighborhood })),
    baseline_sectors: [],
    river: null,
    creeks: [],
    coast: null,
    lake: null,
    relief: { ruggedness_pm: 0, uphill: NO_BEARING, high_ground: 0, relief_m: 0 },
    woods: { share_pm: 200, bearing: 2 },
    minerals: { value_pm: 0, bearing: 6 },
    resources_pm: { fertility: 400, timber: 200, minerals: 0, fishing: 0, energy: 200, scenic: 200 },
    links: [0, 4, 8, 12].map((b) => ({ mode: "road" as const, bearing: b, to: -1, to_name: "", foreign: false })),
    hazards_pm: { flood: 0, storm: 0, wildfire: 0, seismic: 0, drought: 0 },
    climate: { temperature_c: 10, precipitation_mm: 700, biome: "temperate_forest", snow: false },
    wind_from: 12,
    elevation_m: 100,
    era: null,
  };
}
