// The player's town in the game (DEC-086, DEC-088, DEC-090): its layout v2
// map from the engine's views. The geography (`TownGeographyView`, static
// for a run) and the layout registries (`LayoutContentView`, per content
// hash) are fetched once and the plan comes from the plan cache (worker,
// memory, IndexedDB: ./townPlanCache); the dress pass (`TownStateView`,
// monthly) runs here, it takes a few ms. The town map overview and the
// walkable street both draw the dressed map, so they always agree.
// Presentation only: the engine decides what the town contains.
import { engine, expectKind } from "../engine/client";
import type { LayoutContentView } from "../engine/protocol/LayoutContentView";
import type { QueryDto } from "../engine/protocol/QueryDto";
import type { TownGeographyView } from "../engine/protocol/TownGeographyView";
import type { TownPlacesView } from "../engine/protocol/TownPlacesView";
import type { TownStateView } from "../engine/protocol/TownStateView";
import { dressTown, type TownState } from "./layoutV2/dress";
import type { TownPlan } from "./layoutV2/plan";
import { recipeFromView } from "./layoutV2/registry";
import { geoFromEngine, stateFromEngine } from "./townGeoAdapter";
import { townInputFromView } from "./townInput";
import { townSeed, type TownMap } from "./townLayout";
import { IdbPlanStore, TownPlanCache, type PlanResult } from "./townPlanCache";
import { TOWN_GENERATOR, planInWorker, townJobs } from "./townWorkerClient";

let cache: TownPlanCache | null = null;
/** The app's plan cache: plans made in the town worker, kept in memory and (in a built app) IndexedDB. */
export function townPlans(): TownPlanCache {
  if (!cache) {
    // Development builds have no hashed generator id, so a stored plan could outlive a code change: memory only.
    const persist = !import.meta.env.DEV && typeof indexedDB !== "undefined";
    cache = new TownPlanCache((geo, seed, recipe) => planInWorker(townJobs(), geo, seed, recipe), persist ? new IdbPlanStore() : null, TOWN_GENERATOR);
  }
  return cache;
}

const ask = (payload: QueryDto) => engine().request({ kind: "query", payload });

const geographies = new Map<string, Promise<TownGeographyView>>();
let content: Promise<LayoutContentView> | null = null;

function geography(runSeed: string, id: number): Promise<TownGeographyView> {
  const key = `${runSeed}|${id}`;
  let p = geographies.get(key);
  if (!p) {
    p = ask({ type: "town_geography", id }).then((b) => expectKind(b, "town_geography"));
    p.catch(() => geographies.delete(key));
    geographies.set(key, p);
  }
  return p;
}

function layoutContent(): Promise<LayoutContentView> {
  content ??= ask({ type: "layout_content" }).then((b) => expectKind(b, "layout_content")).catch((e: unknown) => {
    content = null;
    throw e;
  });
  return content;
}

/** The layout registries the engine serves, as the generator's recipe. */
export async function liveRecipe() {
  return recipeFromView(await layoutContent());
}

/** The town's state this month. */
export function townState(id: number): Promise<TownStateView> {
  return ask({ type: "town_state", id }).then((b) => expectKind(b, "town_state"));
}

/** The plan of the player's town (cached). Its places are the town view's, so a place added in a run is a new plan. */
export async function planPlayerTown(runSeed: string, municipality: number, town: TownPlacesView): Promise<PlanResult> {
  const [geoView, view] = await Promise.all([geography(runSeed, municipality), layoutContent()]);
  const input = townInputFromView(town);
  const geo = geoFromEngine(geoView, input);
  return townPlans().get({ runSeed, town: municipality, contentHash: view.content_hash, geo, seed: townSeed(runSeed, town.municipality), recipe: recipeFromView(view) });
}

const coarse = (v: number, step: number) => Math.round(v / step) * step;

/**
 * The dress state from the engine's monthly view, at the precision the view
 * has away from the town hall (counts to 100, shares to 10 per mille). The
 * view is precise when the player stands in their own town hall (DEC-079);
 * without this the town would redress itself as they walk in and out.
 */
export function dressStateOf(v: TownStateView, home: number | null): TownState {
  const c: TownStateView = v.detailed
    ? {
        ...v,
        neighborhoods: v.neighborhoods.map((h) => ({ ...h, population: coarse(h.population, 100) })),
        housing: { ...v.housing, vacancy_pm: coarse(v.housing.vacancy_pm, 10), stock_pm: coarse(v.housing.stock_pm, 10), pipeline_pm: coarse(v.housing.pipeline_pm, 10) },
        unemployment_pm: coarse(v.unemployment_pm, 10),
      }
    : v;
  // Where the player stands is not part of the dress (the map's start cell only): home.
  return stateFromEngine(c, home, { type: "home" });
}

/** A key for what the dress reads (the same key: the same map). */
export function dressKey(s: TownState): string {
  return JSON.stringify([s.home, s.hoods, s.vacancy, s.unemployment, s.housingPm, s.pipelinePm, s.closedPlaces]);
}

const dressed = new WeakMap<TownPlan, { key: string; map: TownMap }>();
/** The last dress pass's time, ms (measurements). */
export const dressStats = { ms: 0 };
/** The plan dressed for a state, made once per (plan, state). */
export function dressedTown(plan: TownPlan, state: TownState): TownMap {
  const key = dressKey(state);
  const hit = dressed.get(plan);
  if (hit?.key === key) return hit.map;
  const t0 = performance.now();
  const map = dressTown(plan, state);
  dressStats.ms = performance.now() - t0;
  dressed.set(plan, { key, map });
  return map;
}
