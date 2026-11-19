// Town layout v2 (DEC-090, TOWN_GENERATION.md §3, §5): the static plan.
//
// `planTown(geo, seed, version, recipe)` is a pure function of the town's
// geography (./geo; stage A derives it on the client, ../townGeoAdapter),
// the town seed, the layout version and the registry recipe (./registry). It
// holds every street, plot, reserved slot and landmark the town will ever
// use; `dressTown` (./dress) then shows the simulation's state on it without
// moving anything. Integer maths only (./imath), keyed random streams per
// stage (../rng), so changing one stage never shifts another's draws. A stage
// that cannot fit what it must throws `LayoutOverflow`; the plan retries with
// more room (a new attempt number keys every stream), as v1 does.
import type { TownConnector, TownProp, WaterSide } from "../townLayout";
import { Terrain } from "../townLayout";
import { BAND_CAPS, bandOf, type Band, type TownGeo } from "./geo";
import { NO_BEARING, bearingOf, isqrt } from "./imath";
import { BASE_RECIPE, snapshotRecipe, type Recipe } from "./registry";
import { stageDistricts, stageStreets } from "./districts";
import { stageConnect, stageShortcuts, stageFillers, stageGround, stageLanes, stagePlaces, stageProps, stageRanks } from "./plots";
import { nameBridges, stageArterials, stageLevelEdges, stageRail, stageTerraces } from "./roads";
import { stageCentre, stageTerrain, stageWater, townSize } from "./terrain";
import { LayoutOverflow, RoadClass, Use, WaterKind, isWaterBody, type PlanConnector, type PlanPlot, type Rect, type Segment, type Work, type WorkDistrict } from "./work";

export const LAYOUT_V2 = 2;

export interface PlanDistrict {
  index: number;
  /** Engine neighbourhood id. */
  id: number;
  name: string;
  /** Neighbourhood kind (`core`, `inner`, …). */
  kind: string;
  /** Registry district type id and its Fluent name key. */
  type: string;
  nameKey: string;
  pattern: string;
  tags: string[];
  baselinePopulation: number;
  /** Label anchor (centroid), tiles. */
  centre: { x: number; y: number };
  /** A walkable street cell near the centre ("the streets of"). */
  street: number;
  cells: number;
  bounds: Rect;
  /** Filler plots built at baseline (the rest are reserved land, by fill rank). */
  baselineBuilt: number;
  fillers: number;
  /** Districts it borders, with what lies between (§8 edges). */
  neighbours: { district: number; edge: "street" | "river" | "rail" | "open" | "creek"; via: string }[];
  /** Share of its cells on the floodplain, per mille. */
  floodplainPm: number;
}

export interface TownPlan {
  version: number;
  seed: number;
  municipality: string;
  width: number;
  height: number;
  band: Band;
  /** Ground without buildings (dress paints the built plots). */
  terrain: Uint8Array;
  elevation: Uint8Array;
  district: Int16Array;
  /** WaterKind per cell (river, sea, lake, creek). */
  water: Uint8Array;
  /** Cells blocked by props (trees, the fountain, spoil heaps). */
  blocked: Uint8Array;
  plots: PlanPlot[];
  districts: PlanDistrict[];
  props: TownProp[];
  connectors: PlanConnector[];
  arterials: { index: number; name: string; mode: string; bearing: number; toName: string; length: number }[];
  /** 1 on the main roads (arterials, the ring road, district main streets and the square's ring). */
  mainRoad: Uint8Array;
  /** Every street run (arterials, the square's ring, district streets), tiles. */
  segments: Segment[];
  landmarks: { kind: string; x: number; y: number; name: string; plot: number }[];
  /** The square's open plaza and the civic row above it. */
  square: Rect;
  civic: Rect;
  waterSide: WaterSide | null;
  /** Ms per stage (§13 budgets). */
  stages: Record<string, number>;
  /** Registry entries this plan used (§6.6 option C: stored with the run). */
  recipe: Recipe;
  attempt: number;
  /** Why earlier attempts were retried (empty when the first one fitted). */
  retries: string[];
}

const now = () => (typeof performance !== "undefined" ? performance.now() : Date.now());

/** Plan a town. Deterministic for (geo, seed, version, recipe). */
export function planTown(geo: TownGeo, seed: number, version: number = LAYOUT_V2, recipe: Recipe = BASE_RECIPE): TownPlan {
  if (version !== LAYOUT_V2) throw new Error(`layout v2 planner asked for version ${version}`);
  const retries: string[] = [];
  for (let attempt = 0; attempt < 6; attempt++) {
    try {
      const p = plan(geo, seed >>> 0, attempt, recipe);
      p.retries = retries;
      return p;
    } catch (e) {
      if (!(e instanceof LayoutOverflow)) throw e;
      retries.push(e.message);
    }
  }
  throw new Error(`town layout v2 failed for ${geo.name}: ${retries.join("; ")}`);
}

function plan(geoIn: TownGeo, seed: number, attempt: number, recipe: Recipe): TownPlan {
  // Input order never matters: everything sorted by id.
  const geo: TownGeo = {
    ...geoIn,
    neighborhoods: [...geoIn.neighborhoods].sort((a, b) => a.id - b.id),
    places: [...geoIn.places].sort((a, b) => a.id - b.id),
    links: [...geoIn.links].sort((a, b) => a.bearing - b.bearing || a.mode.localeCompare(b.mode) || a.to - b.to),
  };
  const { W, H } = townSize(geo.baseline_population);
  const N = W * H;
  const w: Work = {
    seed,
    attempt,
    geo,
    recipe,
    band: bandOf(geo.baseline_population),
    W,
    H,
    terrain: new Uint8Array(N).fill(Terrain.Grass),
    level: new Uint8Array(N),
    water: new Uint8Array(N),
    use: new Uint8Array(N),
    roadClass: new Uint8Array(N),
    district: new Int16Array(N).fill(-1),
    riverDist: new Uint16Array(N).fill(65535),
    seaDist: new Uint16Array(N).fill(65535),
    blocked: new Uint8Array(N),
    floodplain: 0,
    centre: { x: W >> 1, y: H >> 1 },
    civic: { x: 0, y: 0, w: 0, h: 0 },
    plaza: { x: 0, y: 0, w: 0, h: 0 },
    ring: { x: 0, y: 0, w: 0, h: 0 },
    segments: [],
    arterials: [],
    connectors: [],
    railCells: [],
    districts: [],
    plots: [],
    props: [],
    landmarks: [],
    used: { districts: new Set(), features: new Set(), connectors: new Set(), places: new Set() },
    timings: {},
  };
  const stage = (name: string, f: () => void) => {
    const t0 = now();
    f();
    w.timings[name] = (w.timings[name] ?? 0) + now() - t0;
  };
  stage("terrain", () => stageTerrain(w));
  stage("water", () => stageWater(w));
  stage("centre", () => stageCentre(w));
  stage("arterials", () => {
    stageArterials(w);
    stageRail(w);
    nameBridges(w);
  });
  stage("districts", () => stageDistricts(w));
  stage("streets", () => {
    stageStreets(w);
    stageTerraces(w);
    stageLevelEdges(w);
  });
  stage("plots", () => {
    stageGround(w);
    stageLanes(w);
  });
  stage("places", () => stagePlaces(w));
  stage("fillers", () => stageFillers(w));
  let baseline: number[] = [];
  stage("props", () => stageProps(w));
  stage("checks", () => {
    stageConnect(w);
    stageShortcuts(w);
    baseline = stageRanks(w);
  });
  return finish(w, baseline);
}

function finish(w: Work, baseline: number[]): TownPlan {
  const t0 = now();
  const { W, H } = w;
  const N = W * H;
  // Front gardens: the rows between a set-back footprint and its frontage.
  for (const p of w.plots) {
    for (let y = p.y + p.h; y < p.y + p.h + p.setback; y++) {
      for (let x = p.x; x < p.x + p.w; x++) {
        const c = y * W + x;
        if (c !== p.front && w.terrain[c] !== Terrain.Path) w.terrain[c] = Terrain.Garden;
      }
    }
  }
  // Districts: centroid, a reachable street near it, bounds, neighbours and edges.
  const districts: PlanDistrict[] = w.districts.map((d: WorkDistrict) => {
    let n = 0;
    let sx = 0;
    let sy = 0;
    let x0 = W;
    let y0 = H;
    let x1 = -1;
    let y1 = -1;
    let flood = 0;
    for (let c = 0; c < N; c++) {
      if (w.district[c] !== d.index) continue;
      const x = c % W;
      const y = Math.floor(c / W);
      n++;
      sx += x;
      sy += y;
      if (x < x0) x0 = x;
      if (x > x1) x1 = x;
      if (y < y0) y0 = y;
      if (y > y1) y1 = y;
      if (w.geo.river && (w.riverDist[c] as number) <= w.floodplain) flood++;
    }
    const centre = { x: n ? sx / n + 0.5 : W / 2, y: n ? sy / n + 0.5 : H / 2 };
    return {
      index: d.index,
      id: d.id,
      name: d.name,
      kind: d.kind,
      type: d.type.id,
      nameKey: d.type.name_key,
      pattern: d.type.streets.pattern,
      tags: [...d.type.tags],
      baselinePopulation: d.population,
      centre,
      street: -1,
      cells: n,
      bounds: { x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1 },
      baselineBuilt: baseline[d.index] ?? 0,
      fillers: w.plots.filter((p) => p.district === d.index && p.place === null && !p.landmark).length,
      neighbours: [],
      floodplainPm: n ? Math.floor((flood * 1000) / n) : 0,
    };
  });
  // The streets of each district: its walkable street cell nearest the centroid.
  const solid = new Uint8Array(N);
  for (const p of w.plots) for (let y = p.y; y < p.y + p.h; y++) for (let x = p.x; x < p.x + p.w; x++) solid[y * W + x] = 1;
  for (const d of districts) {
    let best = -1;
    let bestD = Infinity;
    let any = -1;
    let anyD = Infinity;
    for (let c = 0; c < N; c++) {
      if (w.district[c] !== d.index || solid[c] || w.blocked[c]) continue;
      const t = w.terrain[c] as number;
      const ex = (c % W) + 0.5 - d.centre.x;
      const ey = Math.floor(c / W) + 0.5 - d.centre.y;
      const dd = ex * ex + ey * ey;
      if (t === Terrain.Pavement || t === Terrain.Dirt || t === Terrain.Path || t === Terrain.Quay) {
        if (dd < bestD) {
          bestD = dd;
          best = c;
        }
      } else if ((t === Terrain.Grass || t === Terrain.Plaza || t === Terrain.Yard || t === Terrain.Field) && dd < anyD) {
        anyD = dd;
        any = c;
      }
    }
    d.street = best >= 0 ? best : any;
  }
  // Neighbours: districts sharing a border, and those joined across the river by a bridge.
  const pairs = new Map<string, { a: number; b: number; road: number; rail: number; open: number; creek: number }>();
  const note = (a: number, b: number, c: number, c2: number) => {
    if (a < 0 || b < 0 || a === b) return;
    const lo = Math.min(a, b);
    const hi = Math.max(a, b);
    const k = `${lo}:${hi}`;
    const e = pairs.get(k) ?? { a: lo, b: hi, road: 0, rail: 0, open: 0, creek: 0 };
    if ((w.use[c] as number) & Use.Rail || (w.use[c2] as number) & Use.Rail) e.rail++;
    else if (w.water[c] === WaterKind.Creek || w.water[c2] === WaterKind.Creek) e.creek++;
    else if ((w.use[c] as number) & Use.Road || (w.use[c2] as number) & Use.Road) e.road++;
    else e.open++;
    pairs.set(k, e);
  };
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const c = y * W + x;
      const d = w.district[c] as number;
      if (d < 0) continue;
      if (x + 1 < W) note(d, w.district[c + 1] as number, c, c + 1);
      if (y + 1 < H) note(d, w.district[c + W] as number, c, c + W);
    }
  }
  for (const e of [...pairs.values()].sort((p, q) => p.a - q.a || p.b - q.b)) {
    const edge = e.rail >= Math.max(e.road, e.open, e.creek) ? "rail" : e.creek >= Math.max(e.road, e.open) ? "creek" : e.road >= e.open ? "street" : "open";
    (districts[e.a] as PlanDistrict).neighbours.push({ district: e.b, edge, via: "" });
    (districts[e.b] as PlanDistrict).neighbours.push({ district: e.a, edge, via: "" });
  }
  for (const b of w.connectors) {
    if (b.kind !== "bridge" || b.crosses === "creek") continue;
    const ends = b.axis === "ns" ? [(b.y - 1) * W + b.x + 1, (b.y + b.h) * W + b.x + 1] : [(b.y + 1) * W + b.x - 1, (b.y + 1) * W + b.x + b.w];
    const [da, db] = ends.map((c) => (c >= 0 && c < N ? (w.district[c] as number) : -1)) as [number, number];
    if (da < 0 || db < 0 || da === db) continue;
    for (const [p, q] of [
      [da, db],
      [db, da],
    ] as const) {
      const list = (districts[p] as PlanDistrict).neighbours;
      const ex = list.find((n) => n.district === q);
      if (ex) {
        ex.edge = "river";
        ex.via = b.name;
      } else list.push({ district: q, edge: "river", via: b.name });
    }
  }
  const sea = w.geo.coast?.sea;
  const waterSide: WaterSide | null = sea === undefined || sea === NO_BEARING ? null : sea >= 14 || sea <= 1 ? "north" : sea <= 5 ? "east" : sea <= 9 ? "south" : "west";
  const roadLen = (cells: number[]) => new Set(cells).size;
  // Landmarks (§8: 3–12 unique silhouettes): the square's fountain, civic buildings, the spire, the station, the headframe, then the rest.
  const rank = (k: string) => (k === "square" ? 0 : ["town_hall", "council_chamber", "parliament", "regional_assembly", "executive_residence"].includes(k) ? 1 : k === "spire" ? 2 : k === "station" ? 3 : k === "headframe" ? 4 : 5);
  const marks = [{ kind: "square", x: w.plaza.x + (w.plaza.w >> 1), y: w.plaza.y + (w.plaza.h >> 1), name: "", plot: -1 }, ...w.landmarks];
  w.landmarks = marks.map((l, i) => ({ l, i })).sort((a, b) => rank(a.l.kind) - rank(b.l.kind) || a.i - b.i).slice(0, 12).map((x) => x.l);
  w.timings.finish = now() - t0;
  void isqrt;
  void bearingOf;
  void isWaterBody;
  void RoadClass;
  return {
    version: LAYOUT_V2,
    seed: w.seed,
    municipality: w.geo.name,
    width: W,
    height: H,
    band: w.band,
    terrain: w.terrain,
    elevation: w.level,
    district: w.district,
    water: w.water,
    blocked: w.blocked,
    plots: w.plots,
    districts,
    props: w.props,
    connectors: w.connectors,
    arterials: w.arterials.map((a) => ({ index: a.index, name: a.name, mode: a.mode, bearing: a.bearing, toName: a.toName, length: roadLen(a.cells) })),
    landmarks: w.landmarks,
    segments: w.segments,
    mainRoad: Uint8Array.from(w.roadClass, (k) => (k === RoadClass.Arterial ? 1 : 0)),
    square: w.plaza,
    civic: w.civic,
    waterSide,
    stages: w.timings,
    recipe: snapshotRecipe(w.recipe, w.used),
    attempt: w.attempt,
    retries: [],
  };
}

/** The readability caps that apply to a plan's band (§8). */
export function capsOf(plan: TownPlan) {
  return BAND_CAPS[plan.band];
}

/** A short stable hash of a plan (geometry only; stage timings excluded). */
export function planHash(plan: TownPlan): string {
  let h = 0x811c9dc5;
  const feed = (n: number) => {
    h ^= n & 0xff;
    h = Math.imul(h, 0x01000193);
    h ^= (n >>> 8) & 0xff;
    h = Math.imul(h, 0x01000193);
  };
  feed(plan.width);
  feed(plan.height);
  for (let i = 0; i < plan.terrain.length; i++) feed((plan.terrain[i] as number) | ((plan.elevation[i] as number) << 5) | (((plan.district[i] as number) + 1) << 7));
  for (const p of plan.plots) for (const n of [p.x, p.y, p.w, p.h, p.doorX, p.place ?? 0xffff, p.district, p.fillRank, p.declineRank]) feed(n);
  for (const c of plan.connectors) for (const n of [c.x, c.y, c.w, c.h, c.axis === "ns" ? 1 : 0, c.name.length]) feed(n);
  for (const p of plan.props) feed(p.x * 31 + p.y);
  return (h >>> 0).toString(16).padStart(8, "0");
}

/** The connectors as the map carries them (DEC-090 vocabulary). */
export function mapConnectors(plan: TownPlan): TownConnector[] {
  return plan.connectors.map((c) => ({ kind: c.kind, x: c.x, y: c.y, w: c.w, h: c.h, axis: c.axis, name: c.name }));
}
