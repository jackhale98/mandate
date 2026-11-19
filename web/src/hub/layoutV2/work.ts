// The shared state of one layout v2 planning run: the cell grids every
// stage reads and writes, keyed random streams, and small grid helpers.
// Internal to ./plan and its stages.
import { stream, type Rng } from "../rng";
import { Terrain, type TownConnector, type TownProp } from "../townLayout";
import type { Band, TownGeo } from "./geo";
import type { DistrictType, Recipe, TerrainFeature } from "./registry";

/** A stage could not fit what it must (a place, a district): the run retries with more room. */
export class LayoutOverflow extends Error {}

export const WaterKind = { None: 0, River: 1, Sea: 2, Lake: 3, Creek: 4 } as const;

/** Cell use flags (what a later stage must not build over). */
export const Use = { Road: 1, Square: 2, Plot: 4, Rail: 8, Bridge: 16, Keep: 32, Lane: 64 } as const;

/** Street classes per road cell (for names, lamps, routing). */
export const RoadClass = { None: 0, Arterial: 1, Street: 2, Lane: 3 } as const;

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** An axis-aligned street run, tiles (inclusive bounds of its whole width). */
export interface Segment {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
  /** "h": runs east–west (rows y0..y1 are its width); "v": north–south. */
  axis: "h" | "v";
  cls: number;
  style: "asphalt" | "dirt" | "path";
  district: number;
  /** The arterial it belongs to, or -1. */
  arterial: number;
}

export interface Arterial {
  index: number;
  name: string;
  mode: "road" | "highway" | "rail" | "local";
  bearing: number;
  toName: string;
  /** Road cells, for routes and labels. */
  cells: number[];
}

export interface PlanConnector extends TownConnector {
  /** Registry id of the connector type. */
  type: string;
  /** What it crosses. */
  crosses: "river" | "sea" | "lake" | "creek" | "rail";
  /** The arterial it carries, or -1. */
  arterial: number;
}

export interface WorkDistrict {
  index: number;
  id: number;
  name: string;
  kind: string;
  type: DistrictType;
  population: number;
  quota: number;
  seed: number;
  cells: number;
}

/** A building plot of the plan (a place's slot, a landmark or a filler). */
export interface PlanPlot {
  x: number;
  y: number;
  w: number;
  h: number;
  doorX: number;
  /** The walkable cell below the door. */
  front: number;
  district: number;
  /** Engine place id, or null. */
  place: number | null;
  kind: string;
  name: string;
  landmark: boolean;
  /** Order this plot is built in as the district grows (0 first). */
  fillRank: number;
  /** Order it empties as the district declines (0 first). */
  declineRank: number;
  /** Ground the plot shows while empty. */
  ground: number;
  /** Rows of front garden between the footprint and its frontage. */
  setback: number;
}

export interface Work {
  seed: number;
  attempt: number;
  geo: TownGeo;
  recipe: Recipe;
  band: Band;
  W: number;
  H: number;
  terrain: Uint8Array;
  level: Uint8Array;
  water: Uint8Array;
  use: Uint8Array;
  roadClass: Uint8Array;
  district: Int16Array;
  /** Steps to the nearest river or lake water (65535: none). */
  riverDist: Uint16Array;
  /** Steps to the nearest sea water (65535: none). */
  seaDist: Uint16Array;
  /** Props that block walking (trees, the fountain, spoil heaps). */
  blocked: Uint8Array;
  /** Width of the floodplain beside the river (tiles). */
  floodplain: number;
  centre: { x: number; y: number };
  /** The square: the civic strip (buildings) above the open plaza. */
  civic: Rect;
  plaza: Rect;
  /** The square block with its ring of streets. */
  ring: Rect;
  segments: Segment[];
  arterials: Arterial[];
  connectors: PlanConnector[];
  railCells: number[];
  districts: WorkDistrict[];
  plots: PlanPlot[];
  props: TownProp[];
  landmarks: { kind: string; x: number; y: number; name: string; plot: number }[];
  /** Ids of registry entries used (the recipe snapshot). */
  used: { districts: Set<string>; features: Set<string>; connectors: Set<string>; places: Set<string> };
  timings: Record<string, number>;
}

export function rngOf(w: Work, label: string, ...i: number[]): Rng {
  return stream(w.seed, label, w.attempt, ...i);
}

export function feature(w: Work, generator: string): TerrainFeature | undefined {
  const f = w.recipe.features.find((x) => x.generator === generator);
  if (f) w.used.features.add(f.id);
  return f;
}

/** An integer parameter of a feature, or a default. */
export function param(f: TerrainFeature | undefined, key: string, dflt: number): number {
  const v = f?.params[key];
  return typeof v === "number" ? v : dflt;
}
export function paramRange(f: TerrainFeature | undefined, key: string, dflt: [number, number]): [number, number] {
  const v = f?.params[key];
  return Array.isArray(v) && v.length === 2 && typeof v[0] === "number" ? (v as [number, number]) : dflt;
}

export const inMap = (w: Work, x: number, y: number) => x >= 0 && y >= 0 && x < w.W && y < w.H;

export const isWaterBody = (w: Work, c: number) => {
  const k = w.water[c] as number;
  return k === WaterKind.River || k === WaterKind.Sea || k === WaterKind.Lake;
};

/** Breadth-first steps (4-neighbour) from every source cell, through cells `pass` allows, up to `max`. */
export function distanceField(W: number, H: number, sources: (c: number) => boolean, pass: (c: number) => boolean, max = 65534): Uint16Array {
  const N = W * H;
  const d = new Uint16Array(N).fill(65535);
  const q = new Int32Array(N);
  let qt = 0;
  for (let c = 0; c < N; c++) {
    if (sources(c)) {
      d[c] = 0;
      q[qt++] = c;
    }
  }
  for (let qh = 0; qh < qt; qh++) {
    const c = q[qh] as number;
    const dc = d[c] as number;
    if (dc >= max) continue;
    const x = c % W;
    const nb = [x > 0 ? c - 1 : -1, x < W - 1 ? c + 1 : -1, c >= W ? c - W : -1, c + W < N ? c + W : -1];
    for (const n of nb) {
      if (n < 0 || (d[n] as number) !== 65535 || !pass(n)) continue;
      d[n] = dc + 1;
      q[qt++] = n;
    }
  }
  return d;
}

/** Terrain codes a building may not stand on (besides water). */
export function buildable(t: number): boolean {
  return t === Terrain.Grass || t === Terrain.Field || t === Terrain.Yard || t === Terrain.Garden || t === Terrain.Park;
}

/** Terrain a door may open onto (walked like a pavement). */
export function frontageTerrain(t: number): boolean {
  return (
    t === Terrain.Pavement ||
    t === Terrain.Path ||
    t === Terrain.Plaza ||
    t === Terrain.Dirt ||
    t === Terrain.Quay ||
    t === Terrain.Steps
  );
}
