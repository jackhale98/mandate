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
import type { TownConnector, TownProp, TownStreetEnd, WaterSide } from "../townLayout";
import { Terrain } from "../townLayout";
import { BAND_CAPS, bandOf, type Band, type TownGeo } from "./geo";
import { NO_BEARING, isqrt } from "./imath";
import { BASE_RECIPE, snapshotRecipe, type Recipe } from "./registry";
import { stageDistricts } from "./districts";
import { stageStreetAxes, stageStreets } from "./streets";
import { streetEndOf } from "./net";
import { reserveStation, stageConnect, stageMinimumFill, stageSlivers, stageShortcuts, stageFillers, stageGround, stageLanes, stagePlaces, stageProps, stageRanks } from "./plots";
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
  /** The railway's runs. */
  rail: Segment[];
  /** `StreetAxis` per cell: the direction of diagonal streets (the art draws them smooth). */
  streetAxis: Uint8Array;
  /** How every street that joins no other street ends (DEC-090 street ends). */
  streetEnds: TownStreetEnd[];
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
  const size = townSize(geo.baseline_population);
  // Room for what the town holds: a small town the engine gave many places (or many
  // neighbourhoods) is drawn larger, and a retry after a failed fit adds room.
  let need = geo.neighborhoods.length * 250;
  for (const p of geo.places) {
    const fp = recipe.places.find((x) => x.key === p.kind)?.layout.footprint[1] ?? [4, 3];
    need += (fp[0] + 3) * (fp[1] + 6);
  }
  let grow = 100 + (attempt >= 2 ? (attempt - 1) * 12 : 0);
  if (size.W * size.H * 45 < need * 100) grow = Math.max(grow, isqrt(Math.floor((need * 100 * 10000) / (size.W * size.H * 45))));
  const W = Math.min(240, Math.floor((size.W * grow) / 400) * 4);
  const H = Math.min(160, Math.floor((size.H * grow) / 400) * 4);
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
    rail: [],
    axis: new Uint8Array(N),
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
  stage("centre", () => {
    stageCentre(w);
    w.ground = { terrain: new Uint8Array(w.terrain), level: new Uint8Array(w.level) };
  });
  stage("arterials", () => {
    const tr = now();
    stageRail(w);
    w.timings["arterials.rail"] = now() - tr;
    stageArterials(w);
    nameBridges(w);
  });
  stage("districts", () => stageDistricts(w));
  stage("streets", () => {
    reserveStation(w);
    stageStreets(w);
    stageTerraces(w);
    stageLevelEdges(w);
  });
  stage("plots", () => {
    stageGround(w);
    stageLanes(w);
  });
  stage("places", () => stagePlaces(w));
  stage("fillers", () => {
    stageFillers(w);
    stageMinimumFill(w);
    stageSlivers(w);
  });
  let baseline: number[] = [];
  stage("props", () => stageProps(w));
  stage("checks", () => {
    const tc = now();
    stageConnect(w);
    w.timings["checks.connect"] = now() - tc;
    const ts = now();
    stageShortcuts(w);
    w.timings["checks.shortcuts"] = now() - ts;
    baseline = stageRanks(w);
    stageStreetAxes(w);
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
  // Districts: centroid, a reachable street near it, bounds, neighbours and edges (one pass over the map).
  const D = w.districts.length;
  const acc = Array.from({ length: D }, () => ({ n: 0, sx: 0, sy: 0, x0: W, y0: H, x1: -1, y1: -1, flood: 0 }));
  for (let c = 0; c < N; c++) {
    const d = w.district[c] as number;
    if (d < 0) continue;
    const a = acc[d] as (typeof acc)[number];
    const x = c % W;
    const y = (c - x) / W;
    a.n++;
    a.sx += x;
    a.sy += y;
    if (x < a.x0) a.x0 = x;
    if (x > a.x1) a.x1 = x;
    if (y < a.y0) a.y0 = y;
    if (y > a.y1) a.y1 = y;
    if (w.geo.river && (w.riverDist[c] as number) <= w.floodplain) a.flood++;
  }
  const fillers = new Int32Array(D);
  for (const p of w.plots) if (p.place === null && !p.landmark) fillers[p.district] = (fillers[p.district] as number) + 1;
  const districts: PlanDistrict[] = w.districts.map((d: WorkDistrict) => {
    const a = acc[d.index] as (typeof acc)[number];
    const n = a.n;
    const centre = { x: n ? a.sx / n + 0.5 : W / 2, y: n ? a.sy / n + 0.5 : H / 2 };
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
      bounds: { x: a.x0, y: a.y0, w: a.x1 - a.x0 + 1, h: a.y1 - a.y0 + 1 },
      baselineBuilt: baseline[d.index] ?? 0,
      fillers: fillers[d.index] as number,
      neighbours: [],
      floodplainPm: n ? Math.floor((a.flood * 1000) / n) : 0,
    };
  });
  // The streets of each district: its walkable street cell nearest the centroid (else open ground).
  const solid = new Uint8Array(N);
  for (const p of w.plots) for (let y = p.y; y < p.y + p.h; y++) for (let x = p.x; x < p.x + p.w; x++) solid[y * W + x] = 1;
  // Only cells joined to the square count (a lane boxed in by gardens is not "the streets of" anywhere).
  const reach = new Uint8Array(N);
  {
    const q = new Int32Array(N);
    let qt = 0;
    const s0 = w.plaza.y * W + w.plaza.x;
    reach[s0] = 1;
    q[qt++] = s0;
    const open = (c: number) => !solid[c] && !w.blocked[c] && !isWaterBody(w, c) && walkableTerrain(w.terrain[c] as number);
    for (let qh = 0; qh < qt; qh++) {
      const c = q[qh] as number;
      const x = c % W;
      for (let k = 0; k < 4; k++) {
        const n = k === 0 ? (x > 0 ? c - 1 : -1) : k === 1 ? (x < W - 1 ? c + 1 : -1) : k === 2 ? c - W : c + W;
        if (n < 0 || n >= N || reach[n] || !(open(n) || w.terrain[n] === Terrain.Bridge || w.terrain[n] === Terrain.Pier)) continue;
        reach[n] = 1;
        q[qt++] = n;
      }
    }
  }
  const best = new Int32Array(D).fill(-1);
  const bestD = new Float64Array(D).fill(Infinity);
  const any = new Int32Array(D).fill(-1);
  const anyD = new Float64Array(D).fill(Infinity);
  for (let c = 0; c < N; c++) {
    const d = w.district[c] as number;
    if (d < 0 || solid[c] || w.blocked[c] || !reach[c]) continue;
    const t = w.terrain[c] as number;
    const ctr = (districts[d] as PlanDistrict).centre;
    const ex = (c % W) + 0.5 - ctr.x;
    const ey = Math.floor(c / W) + 0.5 - ctr.y;
    const dd = ex * ex + ey * ey;
    if (t === Terrain.Pavement || t === Terrain.Dirt || t === Terrain.Path || t === Terrain.Quay) {
      if (dd < (bestD[d] as number)) {
        bestD[d] = dd;
        best[d] = c;
      }
    } else if (dd < (anyD[d] as number)) {
      anyD[d] = dd;
      any[d] = c;
    }
  }
  for (const d of districts) d.street = (best[d.index] as number) >= 0 ? (best[d.index] as number) : (any[d.index] as number);
  // Neighbours: districts sharing a border, and those joined across the river by a bridge.
  const pairs = new Map<number, { a: number; b: number; road: number; rail: number; open: number; creek: number }>();
  const note = (a: number, b: number, c: number, c2: number) => {
    if (a < 0 || b < 0 || a === b) return;
    const lo = Math.min(a, b);
    const hi = Math.max(a, b);
    const k = lo * 65536 + hi;
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
      // Across a strip of fields or a road a few cells wide (the town is not one block of land).
      for (let k = 2; k <= 4; k++) {
        if (x + k < W && w.district[c + 1] === -1 && (w.district[c + k] as number) >= 0) {
          note(d, w.district[c + k] as number, c + 1, c + k - 1);
          break;
        }
      }
      for (let k = 2; k <= 4; k++) {
        if (y + k < H && w.district[c + W] === -1 && (w.district[c + k * W] as number) >= 0) {
          note(d, w.district[c + k * W] as number, c + W, c + (k - 1) * W);
          break;
        }
      }
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
    rail: w.rail,
    streetAxis: w.axis,
    streetEnds: w.segments.flatMap((s) => [streetEndOf(s, 0, W, H), streetEndOf(s, 1, W, H)].filter((e): e is TownStreetEnd => e !== null)),
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
  for (let i = 0; i < plan.streetAxis.length; i++) if (plan.streetAxis[i]) feed(i * 4 + (plan.streetAxis[i] as number));
  for (const e of plan.streetEnds) for (const n of [e.x, e.y, e.w, e.h, e.kind.length, e.facing.length]) feed(n);
  for (const p of plan.plots) for (const n of [p.x, p.y, p.w, p.h, p.doorX, p.place ?? 0xffff, p.district, p.fillRank, p.declineRank]) feed(n);
  for (const c of plan.connectors) for (const n of [c.x, c.y, c.w, c.h, c.axis === "ns" ? 1 : 0, c.name.length]) feed(n);
  for (const p of plan.props) feed(p.x * 31 + p.y);
  return (h >>> 0).toString(16).padStart(8, "0");
}

/** The connectors as the map carries them (DEC-090 vocabulary). */
export function mapConnectors(plan: TownPlan): TownConnector[] {
  return plan.connectors.map((c) => ({ kind: c.kind, x: c.x, y: c.y, w: c.w, h: c.h, axis: c.axis, name: c.name }));
}

/** Terrain a walker crosses (water, buildings, walls, rock and the track are not). */
function walkableTerrain(t: number): boolean {
  return !(t === Terrain.Water || t === Terrain.Building || t === Terrain.Door || t === Terrain.Creek || t === Terrain.Rock || t === Terrain.Embankment || t === Terrain.Rail || t === Terrain.Pit);
}
