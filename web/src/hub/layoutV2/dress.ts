// Town layout v2: the dress pass (DEC-090 D2, TOWN_GENERATION.md §9.2).
// `dressTown(plan, state)` is a pure function of the static plan and the
// town's current state. It decides which plots stand (by the plan's fill
// ranks), which are boarded up or derelict (by its decline ranks), which
// are building sites, which place is closed, and where the player's home is.
// It never moves anything: every plot keeps its index and footprint, and
// thresholds are monotonic (more housing only ever adds built plots, more
// vacancy only adds boarded ones, the same ones in the same order).
//
// Stage A state comes from what the game sends today (see `stateFromViews`):
// current neighbourhood populations, the town's vacancy and unemployment.
// Gaps (TownStateView, §4.2, task E3): baselines (the plan's populations
// stand in), housing stock and pipeline, place status (closed works),
// protections and disasters.
import type { MunicipalityView } from "../../engine/protocol/MunicipalityView";
import type { TownLocation } from "../townInput";
import { Terrain, locationCell, type PlotState, type TownBuilding, type TownConnector, type TownDistrict, type TownMap } from "../townLayout";
import { HOOD_KINDS, type HoodKind } from "../townInput";
import { mix } from "../rng";
import type { TownPlan } from "./plan";
import { WaterKind, type PlanPlot } from "./work";

export interface TownState {
  /** The player's home neighbourhood (their house stands there), if any. */
  home: number | null;
  /** Where the player is now. */
  location: TownLocation;
  /** Population per neighbourhood now (absent: as at baseline). */
  hoods?: { id: number; population: number }[];
  /** Vacant share of homes, 0..1. */
  vacancy?: number;
  unemployment?: number;
  /** Housing stock against the baseline, per mille (absent: follows population). */
  housingPm?: number;
  /** Homes in the pipeline, as a share of stock per mille (building sites). */
  pipelinePm?: number;
  /** Places that are closed (a works shut down): shown closed, never removed. */
  closedPlaces?: number[];
}

/** The dress state from what the game sends today (stage A). */
export function stateFromViews(muni: Pick<MunicipalityView, "neighborhoods" | "economy"> | null, home: number | null, location: TownLocation): TownState {
  return {
    home,
    location,
    hoods: muni?.neighborhoods.map((h) => ({ id: h.id, population: Math.round(h.population) })) ?? [],
    vacancy: muni?.economy?.vacancy ?? 0.05,
    unemployment: muni?.economy?.unemployment ?? 0.06,
  };
}

const RESIDENTIAL = new Set(["house", "terrace", "farmhouse", "cottage"]);
const hoodKind = (k: string): HoodKind => ((HOOD_KINDS as readonly string[]).includes(k) ? (k as HoodKind) : "inner");

/** The connectors as the map carries them (DEC-090 vocabulary). */
function mapConnectors(plan: TownPlan): TownConnector[] {
  return plan.connectors.map((c) => ({ kind: c.kind, x: c.x, y: c.y, w: c.w, h: c.h, axis: c.axis, name: c.name }));
}

/** Dress a plan for a state: the TownMap the hub, the overview and the text alternative draw. */
export function dressTown(plan: TownPlan, state: TownState): TownMap {
  const W = plan.width;
  const H = plan.height;
  const N = W * H;
  const terrain = new Uint8Array(plan.terrain);
  const states: PlotState[] = plan.plots.map(() => "built");
  const closed = new Set(state.closedPlaces ?? []);
  const vacancy = Math.max(0, state.vacancy ?? 0.05);

  for (const d of plan.districts) {
    const pop = state.hoods?.find((h) => h.id === d.id)?.population ?? d.baselinePopulation;
    const popPm = d.baselinePopulation > 0 ? Math.floor((pop * 1000) / d.baselinePopulation) : 1000;
    const housingPm = state.housingPm ?? popPm;
    const fillers = plan.plots.map((p, i) => ({ p, i })).filter((x) => x.p.district === d.index && x.p.place === null && !x.p.landmark);
    const n = fillers.length;
    // Built: the first plots by fill rank, as many as the housing calls for (growth fills reserved land).
    const built = Math.max(0, Math.min(n, Math.floor((d.baselineBuilt * Math.max(0, housingPm)) / 1000)));
    const pipeline = Math.max(0, Math.min(n - built, Math.ceil((n * (state.pipelinePm ?? (housingPm > 1000 ? 30 : 0))) / 1000)));
    for (const { p, i } of fillers) states[i] = p.fillRank < built ? "built" : p.fillRank < built + pipeline ? "construction" : "reserved";
    // Decline, by decline rank among the built: derelict after a sustained fall of about 20%, boarded above about 5% vacancy.
    const fall = Math.max(0, 1000 - popPm);
    const derelict = fall > 200 ? Math.floor((built * Math.min(400, (fall - 200) * 2)) / 1000) : 0;
    const boarded = Math.floor((built * Math.min(500, Math.max(0, Math.floor((vacancy - 0.05) * 2000)))) / 1000);
    const declining = fillers.filter((x) => states[x.i] === "built").sort((a, b) => a.p.declineRank - b.p.declineRank || a.i - b.i);
    declining.forEach((x, k) => {
      if (k < derelict) states[x.i] = "derelict";
      else if (k < derelict + boarded) states[x.i] = "boarded";
    });
  }
  // Closed places stand boarded up (a closed works: gates shut, no smoke).
  plan.plots.forEach((p, i) => {
    if (p.place !== null) states[i] = closed.has(p.place) ? "boarded" : "built";
  });

  // The player's home: the first residential plot by fill rank in their neighbourhood, built for them.
  let homePlot = -1;
  const homeD = state.home !== null ? plan.districts.find((d) => d.id === state.home) : undefined;
  if (homeD) {
    const cands = plan.plots
      .map((p, i) => ({ p, i }))
      .filter((x) => x.p.district === homeD.index && x.p.place === null && !x.p.landmark)
      .sort((a, b) => (RESIDENTIAL.has(a.p.kind) ? 0 : 1) - (RESIDENTIAL.has(b.p.kind) ? 0 : 1) || a.p.fillRank - b.p.fillRank || a.i - b.i);
    // A neighbourhood whose land is all places (a small random town): the nearest house outside it.
    const dist = (p: PlanPlot) => Math.abs(p.x + (p.w >> 1) - homeD.centre.x) + Math.abs(p.y + (p.h >> 1) - homeD.centre.y);
    const nearest = () =>
      plan.plots
        .map((p, i) => ({ p, i }))
        .filter((x) => x.p.place === null && !x.p.landmark)
        .sort((a, b) => (RESIDENTIAL.has(a.p.kind) ? 0 : 1) - (RESIDENTIAL.has(b.p.kind) ? 0 : 1) || dist(a.p) - dist(b.p) || a.i - b.i)[0];
    const c = cands[0] ?? nearest();
    if (c) {
      homePlot = c.i;
      states[c.i] = "built";
    }
  }

  // Paint what stands.
  const buildings: TownBuilding[] = plan.plots.map((p: PlanPlot, i) => {
    const st = states[i] as PlotState;
    const home = i === homePlot;
    for (let y = p.y; y < p.y + p.h; y++) for (let x = p.x; x < p.x + p.w; x++) terrain[y * W + x] = st === "reserved" ? p.ground : Terrain.Building;
    if (st !== "reserved") terrain[(p.y + p.h - 1) * W + p.doorX] = Terrain.Door;
    return {
      index: i,
      x: p.x,
      y: p.y,
      w: p.w,
      h: p.h,
      doorX: p.doorX,
      doorY: p.y + p.h - 1,
      front: p.front,
      place: p.place,
      kind: home ? "home" : p.kind,
      name: p.name,
      district: p.district,
      home,
      landmark: p.landmark,
      style: mix(plan.seed, p.x, p.y, p.w * 31 + p.h),
      state: st,
    };
  });
  const solid = new Uint8Array(N);
  for (let c = 0; c < N; c++) {
    const t = terrain[c] as number;
    const wk = plan.water[c] as number;
    const wet = (wk === WaterKind.River || wk === WaterKind.Sea || wk === WaterKind.Lake) && t !== Terrain.Bridge && t !== Terrain.Pier;
    solid[c] =
      t === Terrain.Building ||
      t === Terrain.Door ||
      t === Terrain.Water ||
      t === Terrain.Creek ||
      t === Terrain.Rock ||
      t === Terrain.Embankment ||
      t === Terrain.Rail ||
      t === Terrain.Pit ||
      wet ||
      plan.blocked[c]
        ? 1
        : 0;
  }
  // Viaducts carry the railway, not walkers.
  for (const k of plan.connectors) if (k.kind === "viaduct") for (let y = k.y; y < k.y + k.h; y++) for (let x = k.x; x < k.x + k.w; x++) if (terrain[y * W + x] === Terrain.Rail) solid[y * W + x] = 1;

  const districts: TownDistrict[] = plan.districts.map((d) => ({
    index: d.index,
    id: d.id,
    name: d.name,
    kind: hoodKind(d.kind),
    centre: { x: d.centre.x, y: d.centre.y },
    street: d.street,
    cells: d.cells,
    bounds: { ...d.bounds },
  }));
  const places = buildings
    .filter((b) => b.place !== null)
    .map((b) => ({ place: b.place as number, building: b.index }))
    .sort((a, b) => a.place - b.place);
  const map: TownMap = {
    version: plan.version,
    seed: plan.seed,
    municipality: plan.municipality,
    width: W,
    height: H,
    terrain,
    elevation: plan.elevation,
    connectors: mapConnectors(plan),
    streetAxis: plan.streetAxis,
    streetEnds: plan.streetEnds.map((e) => ({ ...e })),
    solid,
    district: plan.district,
    buildings,
    districts,
    props: plan.props,
    places,
    square: { ...plan.square },
    water: plan.waterSide,
    homeBuilding: homePlot >= 0 ? homePlot : null,
    start: 0,
  };
  const core = plan.districts.find((d) => d.kind === "core") ?? plan.districts[0];
  map.start = locationCell(map, state.location) ?? core?.street ?? plan.square.y * W + plan.square.x;
  return map;
}

/** A short stable hash of a dressed map (v1's `mapHash` plus elevation, connectors and plot states). */
export function dressHash(map: TownMap): string {
  let h = 0x811c9dc5;
  const feed = (n: number) => {
    h ^= n & 0xff;
    h = Math.imul(h, 0x01000193);
    h ^= (n >>> 8) & 0xff;
    h = Math.imul(h, 0x01000193);
  };
  const ST: Record<string, number> = { built: 1, reserved: 2, construction: 3, boarded: 4, derelict: 5 };
  feed(map.width);
  feed(map.height);
  for (let i = 0; i < map.terrain.length; i++) feed((map.terrain[i] as number) | ((map.solid[i] as number) << 5) | (((map.elevation?.[i] as number | undefined) ?? 0) << 6));
  for (const b of map.buildings) for (const n of [b.x, b.y, b.w, b.h, b.doorX, b.place ?? 0xffff, b.district, ST[b.state ?? "built"] ?? 0, b.home ? 1 : 0]) feed(n);
  for (const c of map.connectors ?? []) for (const n of [c.x, c.y, c.w, c.h]) feed(n);
  for (const e of map.streetEnds ?? []) for (const n of [e.x, e.y, e.w, e.h, e.kind.length, e.facing.length]) feed(n);
  if (map.streetAxis) for (let i = 0; i < map.streetAxis.length; i++) if (map.streetAxis[i]) feed(i * 4 + (map.streetAxis[i] as number));
  for (const p of map.props) feed(p.x * 31 + p.y);
  return (h >>> 0).toString(16).padStart(8, "0");
}
