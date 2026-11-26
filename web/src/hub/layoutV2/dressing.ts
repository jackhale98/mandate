// Street dressing (S7; DEC-114 §3, BEING_THERE.md §4.4–4.5; task B4): the
// registry from the engine's state to what the street shows. The dress pass
// (./dress) asks it where a thing the engine reports stands on the plan and
// which drawing shows it now; it decides nothing about the thing itself.
//
// Faults (B2's `FaultView`, on `TownStateView.faults`): the engine names a
// spot as a kind and a slot (`road`, `pavement`, `kerb`, `stop`, `green`;
// slot k of `slots`) and the cue to draw now (`faults.jsonc` `cue`: open,
// being repaired, new). Here a spot kind is a rule over the static plan that
// lists the neighbourhood's candidate cells, nearest its streets' cell first
// (where "See it" arrives), spaced apart; slot k is the k-th. The plan never
// changes for a run, so a fault's spot never moves, whatever else is open;
// the kinds' cells are disjoint by construction (terrain and a fixed
// chequer), so two faults never share a cell. A street-light fault stands on
// one of the plan's lamps, and its cue replaces that lamp while it lasts.
//
// Cues are art props named `cue_<cue>` (tools/pixelkit/art/mandate/
// faultCues.ts); a cue with no drawing yet shows its status's stand-in.
//
// Works (DEC-099) join this registry when the engine sends works in a street
// view; today the dress shows a closed works (`closedPlaces`) and building
// sites (the pipeline) as before.
import type { FaultView } from "../../engine/protocol/FaultView";
import { Terrain, type PropKind, type TownProp } from "../townLayout";
import type { TownPlan } from "./plan";
import { WaterKind } from "./work";

/** A fault as the dress reads it: where, and the cue to draw now. Only what the view says. */
export interface DressFault {
  id: number;
  /** Engine neighbourhood id. */
  hood: number;
  spot: { kind: string; slot: number; slots: number };
  /** `open`, `repairing`, `fixed`. */
  status: string;
  /** The cue the engine names (`lamp_dark`). */
  cue: string;
}

/** The dress state's faults from the engine's views. */
export function dressFaults(views: readonly FaultView[] | undefined): DressFault[] {
  return (views ?? []).map((f) => ({ id: f.id, hood: f.neighborhood, spot: { kind: f.spot.kind, slot: f.spot.slot, slots: f.spot.slots }, status: f.status, cue: f.cue }));
}

/** The cues drawn by the art kit (`FAULT_CUES` in tools/pixelkit/art/mandate/faultCues.ts; a test keeps them equal). */
export const DRAWN_CUES: readonly string[] = [
  "lamp_dark",
  "lamp_van",
  "lamp_new",
  "drain_flooded",
  "drain_cones",
  "drain_new",
  "pothole",
  "road_cones",
  "tarmac_new",
  "shelter_taped",
  "shelter_van",
  "shelter_new",
  "swing_taped",
  "swing_van",
  "swing_new",
  "fault_open",
  "fault_repairing",
  "fault_new",
];
const DRAWN = new Set(DRAWN_CUES);

/** The stand-in drawing for a cue the kit has not drawn, by status. */
const FALLBACK: Record<string, string> = { open: "fault_open", repairing: "fault_repairing", fixed: "fault_new" };

/** Cues drawn with a van beside the spot: a second, mirrored look puts the van on the other side. */
const VANS = new Set(["lamp_van", "shelter_van", "swing_van"]);

/** The art prop that shows a cue in a status. */
export function cueProp(cue: string, status: string): PropKind {
  const name = DRAWN.has(cue) ? cue : (FALLBACK[status] ?? "fault_open");
  return `cue_${name}`;
}

// ---------------------------------------------------------------------------
// Spots

/** What a spot kind stands on, and how far apart its slots are (Chebyshev, cells). */
interface SpotRule {
  /** Candidate cells of a district, best first, before spacing. */
  cells: (p: SpotPlan, district: number) => number[];
  spacing: number;
  /** A plan prop the cue stands in for while it lasts (the lamp it is on). */
  replaces?: PropKind;
}

/** The plan as the spot rules read it, worked out once per plan. */
interface SpotPlan {
  plan: TownPlan;
  /** Cells no cue may take: plot footprints, the rows their roofs hide, their doorsteps, blocked cells, street ends, connectors, other props. */
  taken: Uint8Array;
  /** The plan's lamps by cell (prop index). */
  lamps: Map<number, number>;
  /** Pavement cells beside a road. */
  kerbside: Uint8Array;
}

/** Rows above a plot that its building can cover (the art draws up to three roof rows: `PixelkitArt.maxRoofRows`). */
const ROOF_HIDES = 3;

const CHEQUER = (c: number, W: number) => ((c % W) + Math.floor(c / W)) % 2;

function spotPlan(plan: TownPlan): SpotPlan {
  const W = plan.width;
  const H = plan.height;
  const N = W * H;
  const taken = new Uint8Array(N);
  const mark = (x: number, y: number, w: number, h: number) => {
    for (let yy = Math.max(0, y); yy < Math.min(H, y + h); yy++) for (let xx = Math.max(0, x); xx < Math.min(W, x + w); xx++) taken[yy * W + xx] = 1;
  };
  for (const p of plan.plots) {
    // The plot, its front garden, and the rows its roof can hide behind it (built or not: a plot may be built later).
    mark(p.x, p.y - ROOF_HIDES, p.w, p.h + p.setback + ROOF_HIDES);
    if (p.front >= 0 && p.front < N) taken[p.front] = 1;
  }
  for (const e of plan.streetEnds) mark(e.x, e.y, e.w, e.h);
  for (const k of plan.connectors) mark(k.x - 1, k.y - 1, k.w + 2, k.h + 2);
  for (let c = 0; c < N; c++) {
    const wk = plan.water[c] as number;
    if (plan.blocked[c] || wk === WaterKind.River || wk === WaterKind.Sea || wk === WaterKind.Lake || wk === WaterKind.Creek) taken[c] = 1;
  }
  const lamps = new Map<number, number>();
  plan.props.forEach((pr, i) => {
    const c = pr.y * W + pr.x;
    if (pr.kind === "lamp") lamps.set(c, i);
    else taken[c] = 1;
  });
  const kerbside = new Uint8Array(N);
  const t = (x: number, y: number) => (x < 0 || y < 0 || x >= W || y >= H ? -1 : (plan.terrain[y * W + x] as number));
  for (let c = 0; c < N; c++) {
    if (plan.terrain[c] !== Terrain.Pavement) continue;
    const x = c % W;
    const y = Math.floor(c / W);
    if ([t(x, y - 1), t(x, y + 1), t(x - 1, y), t(x + 1, y)].includes(Terrain.Road)) kerbside[c] = 1;
  }
  return { plan, taken, lamps, kerbside };
}

/** A district's cells passing a test, nearest its streets' cell first (ties by cell). */
function near(p: SpotPlan, district: number, test: (c: number) => boolean): number[] {
  const plan = p.plan;
  const W = plan.width;
  const d = plan.districts[district];
  if (!d) return [];
  const sx = d.street % W;
  const sy = Math.floor(d.street / W);
  const out: { c: number; k: number }[] = [];
  const b = d.bounds;
  for (let y = b.y; y < b.y + b.h; y++) {
    for (let x = b.x; x < b.x + b.w; x++) {
      const c = y * W + x;
      if (plan.district[c] !== district || !test(c)) continue;
      out.push({ c, k: Math.abs(x - sx) + Math.abs(y - sy) });
    }
  }
  return out.sort((a, b2) => a.k - b2.k || a.c - b2.c).map((o) => o.c);
}

/** Road cells beside a kerb (the gutter) and the road's middle, split between drains and potholes by the chequer. */
function roadSide(p: SpotPlan, c: number): boolean {
  const W = p.plan.width;
  const x = c % W;
  const y = Math.floor(c / W);
  const pave = (xx: number, yy: number) => xx >= 0 && yy >= 0 && xx < W && yy < p.plan.height && p.plan.terrain[yy * W + xx] === Terrain.Pavement;
  return pave(x, y - 1) || pave(x, y + 1) || pave(x - 1, y) || pave(x + 1, y);
}

const free = (p: SpotPlan, c: number) => p.taken[c] === 0;
const terrainIs = (p: SpotPlan, c: number, ...ts: number[]) => ts.includes(p.plan.terrain[c] as number);

/** A rural lane's cells (no road, no kerb) are shared out among the street kinds in four fixed classes. */
const LANE = (c: number, W: number) => ((c % W) + Math.floor(c / W)) % 4;
/** A neighbourhood of fields and no streets: its open field is shared among all five kinds (the green takes class 4). */
const FIELD = (c: number, W: number) => ((c % W) + Math.floor(c / W)) % 5;
const field = (p: SpotPlan, d: number, k: number) => near(p, d, (c) => free(p, c) && terrainIs(p, c, Terrain.Field) && FIELD(c, p.plan.width) === k);

/**
 * The spot kinds (`FaultSpotView.kind`, `faults.jsonc` `spot.kind`). Each
 * lists its candidates in order of preference; the kinds never share a cell:
 * pavement and stops split the kerbside by the chequer, drains and potholes
 * split the road by it, a lane's track is split four ways, the green takes
 * open ground no street uses, and in a neighbourhood with no streets at all
 * the field is split five ways.
 */
export const SPOTS: Record<string, SpotRule> = {
  /** A street light: the plan's lamps first, then kerbside pavement on the light chequer squares, then a lane, then pavement away from the kerb. */
  pavement: {
    spacing: 2,
    replaces: "lamp",
    cells: (p, d) => [
      ...near(p, d, (c) => p.lamps.has(c)),
      ...near(p, d, (c) => free(p, c) && !p.lamps.has(c) && p.kerbside[c] === 1 && CHEQUER(c, p.plan.width) === 1),
      ...near(p, d, (c) => free(p, c) && terrainIs(p, c, Terrain.Dirt) && LANE(c, p.plan.width) === 0),
      ...near(p, d, (c) => free(p, c) && !p.lamps.has(c) && terrainIs(p, c, Terrain.Pavement) && p.kerbside[c] === 0),
      ...field(p, d, 0),
    ],
  },
  /** A bus stop: kerbside pavement on a main road, on the dark chequer squares; then any kerbside pavement on them; then a lane. */
  stop: {
    spacing: 4,
    cells: (p, d) => [
      ...near(p, d, (c) => free(p, c) && !p.lamps.has(c) && p.kerbside[c] === 1 && CHEQUER(c, p.plan.width) === 0 && besideMainRoad(p, c)),
      ...near(p, d, (c) => free(p, c) && !p.lamps.has(c) && p.kerbside[c] === 1 && CHEQUER(c, p.plan.width) === 0 && !besideMainRoad(p, c)),
      ...near(p, d, (c) => free(p, c) && terrainIs(p, c, Terrain.Dirt) && LANE(c, p.plan.width) === 1),
      ...field(p, d, 1),
    ],
  },
  /** A drain: the road's gutter on the dark chequer squares, then the rest of the road on them, then a lane. */
  kerb: {
    spacing: 3,
    cells: (p, d) => [
      ...near(p, d, (c) => free(p, c) && terrainIs(p, c, Terrain.Road) && roadSide(p, c) && CHEQUER(c, p.plan.width) === 0),
      ...near(p, d, (c) => free(p, c) && terrainIs(p, c, Terrain.Road) && !roadSide(p, c) && CHEQUER(c, p.plan.width) === 0),
      ...near(p, d, (c) => free(p, c) && terrainIs(p, c, Terrain.Dirt) && LANE(c, p.plan.width) === 2),
      ...field(p, d, 2),
    ],
  },
  /** A pothole: the road's middle on the light chequer squares, then its gutter on them, then a lane. */
  road: {
    spacing: 3,
    cells: (p, d) => [
      ...near(p, d, (c) => free(p, c) && terrainIs(p, c, Terrain.Road) && !roadSide(p, c) && CHEQUER(c, p.plan.width) === 1),
      ...near(p, d, (c) => free(p, c) && terrainIs(p, c, Terrain.Road) && roadSide(p, c) && CHEQUER(c, p.plan.width) === 1),
      ...near(p, d, (c) => free(p, c) && terrainIs(p, c, Terrain.Dirt) && LANE(c, p.plan.width) === 3),
      ...field(p, d, 3),
    ],
  },
  /** A play area: a park, open grass, the square's paving, a footpath's verge, a field's edge, then spare yard. */
  green: {
    spacing: 4,
    cells: (p, d) => [
      ...near(p, d, (c) => free(p, c) && terrainIs(p, c, Terrain.Park)),
      ...near(p, d, (c) => free(p, c) && terrainIs(p, c, Terrain.Grass)),
      ...near(p, d, (c) => free(p, c) && terrainIs(p, c, Terrain.Plaza)),
      ...near(p, d, (c) => free(p, c) && terrainIs(p, c, Terrain.Path)),
      ...field(p, d, 4),
      ...near(p, d, (c) => free(p, c) && terrainIs(p, c, Terrain.Yard)),
    ],
  },
};

function besideMainRoad(p: SpotPlan, c: number): boolean {
  const W = p.plan.width;
  const x = c % W;
  const y = Math.floor(c / W);
  return [
    [x, y - 1],
    [x, y + 1],
    [x - 1, y],
    [x + 1, y],
  ].some(([xx, yy]) => (xx as number) >= 0 && (yy as number) >= 0 && (xx as number) < W && (yy as number) < p.plan.height && p.plan.mainRoad[(yy as number) * W + (xx as number)] === 1 && p.plan.terrain[(yy as number) * W + (xx as number)] === Terrain.Road);
}

const plans = new WeakMap<TownPlan, { sp: SpotPlan; slots: Map<string, number[]> }>();

/**
 * The cells of a district's spots of a kind, slot by slot (`slots` of them,
 * or fewer where the district has no more room): a pure function of the
 * plan, so a spot's cell never moves.
 */
export function spotCells(plan: TownPlan, district: number, kind: string, slots: number): number[] {
  let hit = plans.get(plan);
  if (!hit) {
    hit = { sp: spotPlan(plan), slots: new Map() };
    plans.set(plan, hit);
  }
  const key = `${district}|${kind}|${slots}`;
  const done = hit.slots.get(key);
  if (done) return done;
  const rule = SPOTS[kind];
  const out: number[] = [];
  if (rule) {
    const W = plan.width;
    const cands = [...new Set(rule.cells(hit.sp, district))];
    // Spaced apart; where the neighbourhood is too small for that, the rest closer together.
    for (const spacing of [rule.spacing, 1]) {
      for (const c of cands) {
        if (out.length >= slots) break;
        const x = c % W;
        const y = Math.floor(c / W);
        if (out.some((o) => Math.max(Math.abs((o % W) - x), Math.abs(Math.floor(o / W) - y)) < spacing)) continue;
        out.push(c);
      }
    }
  }
  hit.slots.set(key, out);
  return out;
}

/** A fault's cell on the plan, or null (an unknown neighbourhood, a spot kind with no rule, a district with no room). */
export function faultCell(plan: TownPlan, f: DressFault): number | null {
  const d = plan.districts.find((x) => x.id === f.hood);
  if (!d) return null;
  return spotCells(plan, d.index, f.spot.kind, Math.max(f.spot.slots, f.spot.slot + 1))[f.spot.slot] ?? null;
}

/** Where each fault's cue stands and what it draws. */
export interface CuePlacement {
  fault: number;
  cell: number;
  status: string;
  prop: TownProp;
  /** The plan prop it stands in for (a lamp), by index, or null. */
  replaces: number | null;
}

/** The cues of the faults the engine reports, on the plan (faults with no cell are left out). */
export function placeCues(plan: TownPlan, faults: readonly DressFault[]): CuePlacement[] {
  const W = plan.width;
  const sp = plans.get(plan)?.sp;
  const out: CuePlacement[] = [];
  const used = new Set<number>();
  for (const f of faults) {
    const cell = faultCell(plan, f);
    if (cell === null || used.has(cell)) continue;
    used.add(cell);
    const x = cell % W;
    const y = Math.floor(cell / W);
    const rule = SPOTS[f.spot.kind];
    const lamp = rule?.replaces === "lamp" ? (sp ?? spotPlan(plan)).lamps.get(cell) : undefined;
    const prop = cueProp(f.cue, f.status);
    // A van parks on the side away from a house front (the mirrored look parks it on the left).
    const right = x + 1 < W && !plan.plots.some((p) => x + 1 >= p.x && x + 1 < p.x + p.w && y >= p.y && y < p.y + p.h);
    const variant = VANS.has(prop.slice(4)) && !right ? 1 : 0;
    out.push({ fault: f.id, cell, status: f.status, prop: { kind: prop, x, y, blocks: false, variant, fault: f.id }, replaces: lamp ?? null });
  }
  return out;
}
