// Works you can see (art round 8; DEC-099): the dress rules that turn a works
// project's phase and age into the parts the street draws. A funded project
// physically changes the town (a bridge's new deck, a flood wall, a clinic in
// a free slot, a busier mine); a project that stalls sits half-built and
// decays over months; finished works carry a plaque with the sponsor's name.
//
// `worksLook(kind, phase, progress, monthsStalled, seed)` is pure and keyed
// like the rest of the dress pass (TOWN_GENERATION.md §9.2): no clock, no
// `Math.random`, integer thresholds only. Its rules are monotonic, so nothing
// flickers from month to month:
//   - progress (whole percent) picks a build stage by fixed thresholds
//     (`BUILD_FROM`); a stalled project keeps the stage it stopped at;
//   - months since the stall pick a decay stage 0–3 (`DECAY_FROM`); each
//     decay mark (`MARKS`) has a fixed instance number and appears at a fixed
//     stage, so a later stage only adds marks, never removes one;
//   - worn things move one way along a wear chain (`WEAR`: a fence standing,
//     sagging, down; a board, faded, peeling), at stages fixed per instance by
//     the seed, so a fence that has fallen never stands up again.
// Where the parts stand on a site, and how they are drawn, is
// `composeWorks` (./worksCompose); the parts' art is ./worksParts and the
// runs of fencing and walls ./worksLines.
import { hash32 } from "../../src/core/hash";
import type { Look } from "./characters";

/** Bump when a rule changes what a project looks like for the same inputs (DEC-085: old versions stay callable). */
export const WORKS_RULES_VERSION = 1;

/**
 * Works kinds:
 *   - bridge: a bridge's deck replaced (a connector);
 *   - flood_wall, levee: a defence along a bank (a run of line pieces);
 *   - civic: a new clinic, library or community centre in a free slot;
 *   - housing: an infill or estate plot;
 *   - road: a resurfacing strip (road cells, orthogonal or 45°);
 *   - mine: a shaft sunk or reopened (headframe, spoil tip);
 *   - factory: a works or mill (its building, chimney and yard).
 * Mine and factory are also how the dress pass shows an existing works'
 * activity: open with an activity level (idle, working, busy), and closed as
 * stalled (shuttered, decaying).
 */
export const WORKS_KINDS = ["bridge", "flood_wall", "levee", "civic", "housing", "road", "mine", "factory"] as const;
export type WorksKind = (typeof WORKS_KINDS)[number];

/** Phases: building (by progress) → open; or stalled (decaying by months) → cleared. */
export const WORKS_PHASES = ["building", "open", "stalled", "cleared"] as const;
export type WorksPhase = (typeof WORKS_PHASES)[number];

/** Build stages: the site set up, the ground work, the structure rising, finishing, complete (awaiting the opening). */
export const BUILD_STAGES = ["site", "ground", "frame", "finish", "complete"] as const;
/** Progress (whole percent) at which each build stage starts. */
export const BUILD_FROM: readonly number[] = [0, 20, 45, 80, 100];
/** Months since the stall at which each decay stage starts (0: just stopped; 3: abandoned; 9 and 18: getting worse). */
export const DECAY_FROM: readonly number[] = [0, 3, 9, 18];
/** An open works' activity (0–100) at which each level starts: idle, working, busy. */
export const ACTIVITY_FROM: readonly number[] = [0, 34, 67];
export const ACTIVITY_LEVELS = ["idle", "working", "busy"] as const;

/**
 * Where a part goes on its site (resolved by `composeWorks` from the site's shape):
 *   - structure: the works itself (a deck state, a wall run, a building, the headframe);
 *   - ground: ground under it (a cleared lot, foundations, a shaft collar, new surfacing);
 *   - perimeter: the fence round the site (a line run; worn piece by piece);
 *   - closure: barriers across a way, the fence round a gap;
 *   - yard: free-standing things at fixed yard slots (`n`): 0 cabin, 1 machine, 2–5 piles and skip, 6 lorry, 7–9 goods;
 *   - front: at the site's front: 0 the board, 1 a sign, 2 gates, 3+ cones;
 *   - entrance: the ribbon, or the plaque;
 *   - crane: the tower crane;
 *   - marks: decay marks;
 *   - people: workers in hi-vis.
 */
export const ROLES = ["structure", "ground", "perimeter", "closure", "yard", "front", "entrance", "crane", "marks", "people"] as const;
export type Role = (typeof ROLES)[number];

/** A wear chain: ids by wear level, the decay stage each next level starts at, and how many stages later an instance may start (by the seed). */
export interface WearRule {
  chain: readonly string[];
  at: readonly number[];
  spread: number;
}

export interface WorksPart {
  /** A part of the kit (./worksParts), a line style (./worksLines), a structure state, or a mark. */
  id: string;
  role: Role;
  /** Instance within the role: a stable key (yard slot, mark number). */
  n: number;
  /** An idle loop of this many frames (a turning headframe, smoke, a sheet flapping, an excavator digging). */
  frames?: number;
  /** Its lit layer shows at night (the site has power). */
  lit?: boolean;
  /** A decay mark: once shown, shown at every later stage. */
  mark?: true;
  /** A structure's amount: percent built or surfaced, a heap's size, the share of windows lit. */
  level?: number;
  /** For a line part: how its pieces wear (each piece by `wearLevel`), at this decay stage. */
  wear?: WearRule;
  decay?: number;
  /** The wear chain's level this part is at (a single part already resolved to its chain id). */
  worn?: number;
}

export interface WorksLook {
  version: number;
  kind: WorksKind;
  phase: WorksPhase;
  /** Build stage reached (0–4); for a stalled project, the stage it stopped at. */
  build: number;
  /** Decay stage (0–3); 0 unless stalled or cleared. */
  decay: number;
  /** Activity level of an open mine or factory (0 idle, 1 working, 2 busy). */
  activity: number;
  parts: WorksPart[];
}

export interface WorksOptions {
  /** An open mine's or factory's activity, 0–100 (default 50: working). */
  activity?: number;
  /** Cumulative extraction at a mine, 0–100: the spoil heap's size (default 50). */
  extraction?: number;
  /** The open works carries a sponsor's plaque (default true: it was built by a project). */
  sponsor?: boolean;
}

/** Stage index from a value and ascending thresholds. */
const stageOf = (v: number, from: readonly number[]) => {
  let s = 0;
  for (let k = 0; k < from.length; k++) if (v >= (from[k] as number)) s = k;
  return s;
};
export const buildStage = (progress: number) => stageOf(Math.max(0, Math.min(100, Math.floor(progress))), BUILD_FROM);
export const decayStage = (months: number) => stageOf(Math.max(0, Math.floor(months)), DECAY_FROM);
export const activityLevel = (a: number) => stageOf(Math.max(0, Math.min(100, Math.floor(a))), ACTIVITY_FROM);

/**
 * The wear level of one instance at a decay stage: the count of chain steps
 * whose stage (the rule's, delayed for this instance by 0..spread stages
 * from the seed) has been reached. Monotonic in `decay` by construction.
 */
export function wearLevel(rule: WearRule, seed: number, role: string, n: number, decay: number): number {
  let level = 0;
  for (let k = 0; k < rule.at.length && k < rule.chain.length - 1; k++) {
    const delay = rule.spread ? hash32(seed, role, n, k, 0x3ea) % (rule.spread + 1) : 0;
    if (decay >= (rule.at[k] as number) + delay) level = k + 1;
    else break;
  }
  return level;
}

// ---------------------------------------------------------------------------
// The tables

/** Wear chains: fences sag then fall panel by panel; boards fade then peel; machines and cabins rust and get broken into. */
export const WEAR: Readonly<Record<string, WearRule>> = {
  heras: { chain: ["heras", "heras_sag", "heras_down"], at: [1, 2], spread: 2 },
  hoarding: { chain: ["hoarding", "hoarding_sag", "hoarding_down"], at: [2, 3], spread: 1 },
  barrier: { chain: ["barrier", "barrier_sag", "barrier_down"], at: [1, 2], spread: 2 },
  board: { chain: ["board", "board_faded", "board_peeling"], at: [1, 3], spread: 0 },
  cabin: { chain: ["cabin", "cabin_broken"], at: [2], spread: 0 },
  crane: { chain: ["crane", "crane_rust"], at: [2], spread: 0 },
  excavator: { chain: ["excavator", "excavator_rust"], at: [2], spread: 0 },
  cone: { chain: ["cone", "cone_down"], at: [1], spread: 2 },
  road_sign: { chain: ["road_sign", "road_sign_faded"], at: [2], spread: 0 },
  gates_shut: { chain: ["gates_shut", "gates_shut_rust"], at: [2], spread: 0 },
  headframe: { chain: ["headframe", "headframe_rust"], at: [2], spread: 0 },
  "facade.boarded": { chain: ["facade.boarded", "facade.derelict"], at: [3], spread: 0 },
};

/**
 * Decay marks per kind: how many of each are shown at decay stages 0–3
 * (cumulative, never fewer at a later stage). What each mark is:
 * weeds, scrub (on bare earth and a spoil tip), moss (on concrete), litter
 * and puddles on the ground; potholes in a road; gullies down a bare bank;
 * graffiti on painted and concrete faces; rust streaks on steel; a plastic
 * sheet flapping and buddleia on an exposed frame.
 */
export const MARKS: Readonly<Record<WorksKind, Readonly<Record<string, readonly [number, number, number, number]>>>> = {
  bridge: { weeds: [0, 4, 9, 15], rust: [0, 3, 7, 12], graffiti: [0, 1, 2, 4], puddle: [0, 1, 2, 3], litter: [0, 1, 3, 5] },
  flood_wall: { weeds: [0, 5, 11, 18], moss: [0, 2, 5, 9], rust: [0, 3, 6, 10], graffiti: [0, 1, 2, 4], puddle: [0, 1, 2, 3], litter: [0, 1, 2, 4] },
  levee: { scrub: [0, 5, 12, 20], gully: [0, 1, 3, 5], puddle: [0, 1, 2, 3], litter: [0, 1, 2, 3] },
  civic: { weeds: [0, 6, 14, 24], graffiti: [0, 1, 3, 5], rust: [0, 3, 6, 10], puddle: [0, 1, 2, 3], litter: [0, 2, 4, 7], sheet: [0, 0, 1, 2], buddleia: [0, 0, 1, 3] },
  housing: { weeds: [0, 5, 12, 20], graffiti: [0, 1, 2, 4], rust: [0, 2, 4, 7], puddle: [0, 1, 2, 2], litter: [0, 2, 3, 5], sheet: [0, 0, 1, 1], buddleia: [0, 0, 1, 2] },
  road: { pothole: [0, 2, 5, 9], weeds: [0, 2, 5, 9], puddle: [0, 0, 2, 4], litter: [0, 1, 3, 5] },
  mine: { weeds: [0, 6, 14, 22], scrub: [0, 4, 10, 18], rust: [0, 3, 6, 10], graffiti: [0, 1, 2, 3], litter: [0, 2, 3, 5], puddle: [0, 1, 2, 3] },
  factory: { weeds: [0, 8, 18, 30], graffiti: [0, 2, 4, 6], rust: [0, 2, 5, 8], litter: [0, 3, 6, 9], puddle: [0, 1, 3, 4], buddleia: [0, 0, 1, 2] },
};

type Spec = Omit<WorksPart, "role" | "n"> & { role: Role; n?: number };
const P = (id: string, role: Role, n = 0, extra: Partial<WorksPart> = {}): Spec => ({ id, role, n, ...extra });
const workers = (k: number): Spec[] => Array.from({ length: k }, (_, n) => P("worker", "people", n));
const cones = (k: number, from = 3): Spec[] => Array.from({ length: k }, (_, i) => P("cone", "front", from + i));

/** Per kind: the parts at each build stage (0–4), when open (per activity level for mine and factory), and once cleared. */
export interface KindRules {
  build: readonly (readonly Spec[])[];
  open: readonly (readonly Spec[])[];
  cleared: readonly Spec[];
  /** Open works get the sponsor's plaque. */
  plaque: boolean;
}

const lot = (k: number) => P("lot", "ground", 0, { level: k });

export const WORKS_RULES: Readonly<Record<WorksKind, KindRules>> = {
  bridge: {
    build: [
      [P("deck", "structure", 0, { level: 0 }), P("scaffold", "structure", 1, { level: 50 }), P("barrier", "closure", 0), P("cabin", "yard", 0, { lit: true }), P("board", "front", 0), P("road_sign", "front", 1), ...cones(3), ...workers(2)],
      [P("deck", "structure", 0, { level: 1 }), P("scaffold", "structure", 1, { level: 100 }), P("barrier", "closure", 0), P("cabin", "yard", 0, { lit: true }), P("excavator", "yard", 1, { frames: 2 }), P("pile_rebar", "yard", 2), P("board", "front", 0), P("road_sign", "front", 1), ...cones(3), ...workers(2)],
      [P("deck", "structure", 0, { level: 2 }), P("scaffold", "structure", 1, { level: 100 }), P("barrier", "closure", 1), P("heras", "closure", 2), P("cabin", "yard", 0, { lit: true }), P("pile_rebar", "yard", 2), P("pile_sheets", "yard", 3), P("board", "front", 0), P("road_sign", "front", 1), ...workers(3)],
      [P("deck", "structure", 0, { level: 3 }), P("railing", "structure", 2, { level: 70 }), P("scaffold", "structure", 1, { level: 50 }), P("barrier", "closure", 1), P("cabin", "yard", 0, { lit: true }), P("pile_timber", "yard", 3), P("board", "front", 0), P("road_sign", "front", 1), ...workers(2)],
      [P("deck", "structure", 0, { level: 4 }), P("railing", "structure", 2, { level: 100 }), P("ribbon", "entrance", 0), P("board", "front", 0)],
    ],
    open: [[P("deck", "structure", 0, { level: 4 }), P("railing", "structure", 2, { level: 100 })]],
    cleared: [P("deck", "structure", 0, { level: 5 })],
    plaque: true,
  },
  flood_wall: {
    build: [
      [P("piles", "structure", 1, { level: 30 }), P("heras", "perimeter", 0), P("cabin", "yard", 0, { lit: true }), P("excavator", "yard", 1, { frames: 2 }), P("pile_sheets", "yard", 2), P("board", "front", 0), P("hazard_sign", "front", 1), ...workers(2)],
      [P("piles", "structure", 1, { level: 75 }), P("heras", "perimeter", 0), P("cabin", "yard", 0, { lit: true }), P("excavator", "yard", 1, { frames: 2 }), P("pile_sheets", "yard", 2), P("pile_rebar", "yard", 3), P("board", "front", 0), P("hazard_sign", "front", 1), ...workers(3)],
      [P("wall", "structure", 0, { level: 45 }), P("piles", "structure", 1, { level: 100 }), P("heras", "perimeter", 0), P("cabin", "yard", 0, { lit: true }), P("excavator", "yard", 1, { frames: 2 }), P("pile_rebar", "yard", 3), P("pile_sand", "yard", 4), P("board", "front", 0), P("hazard_sign", "front", 1), ...workers(3)],
      [P("wall", "structure", 0, { level: 85 }), P("piles", "structure", 1, { level: 100 }), P("heras", "perimeter", 0), P("cabin", "yard", 0, { lit: true }), P("pile_sand", "yard", 4), P("board", "front", 0), ...workers(2)],
      [P("wall", "structure", 0, { level: 100 }), P("ribbon", "entrance", 0), P("board", "front", 0)],
    ],
    open: [[P("wall", "structure", 0, { level: 100 })]],
    cleared: [],
    plaque: true,
  },
  levee: {
    build: [
      [P("levee_bare", "structure", 1, { level: 25 }), P("cabin", "yard", 0, { lit: true }), P("excavator", "yard", 1, { frames: 2 }), P("pile_sand", "yard", 2), P("board", "front", 0), ...workers(1)],
      [P("levee_bare", "structure", 1, { level: 55 }), P("cabin", "yard", 0, { lit: true }), P("excavator", "yard", 1, { frames: 2 }), P("pile_sand", "yard", 2), P("board", "front", 0), ...workers(2)],
      [P("levee_bare", "structure", 1, { level: 100 }), P("cabin", "yard", 0, { lit: true }), P("excavator", "yard", 1, { frames: 2 }), P("pile_sand", "yard", 2), P("board", "front", 0), ...workers(2)],
      [P("levee", "structure", 0, { level: 60 }), P("levee_bare", "structure", 1, { level: 100 }), P("cabin", "yard", 0, { lit: true }), P("board", "front", 0), ...workers(1)],
      [P("levee", "structure", 0, { level: 100 }), P("ribbon", "entrance", 0), P("board", "front", 0)],
    ],
    open: [[P("levee", "structure", 0, { level: 100 })]],
    cleared: [],
    plaque: true,
  },
  civic: {
    build: [
      [lot(0), P("hoarding", "perimeter", 0), P("cabin", "yard", 0, { lit: true }), P("excavator", "yard", 1, { frames: 2 }), P("pile_sand", "yard", 2), P("board", "front", 0), P("hazard_sign", "front", 1), ...workers(2)],
      [lot(1), P("hoarding", "perimeter", 0), P("crane", "crane", 0, { lit: true, id: "crane_load" }), P("cabin", "yard", 0, { lit: true }), P("excavator", "yard", 1, { frames: 2 }), P("pile_rebar", "yard", 2), P("pile_pipes", "yard", 3), P("skip", "yard", 5), P("board", "front", 0), P("hazard_sign", "front", 1), ...workers(3)],
      [lot(2), P("frame", "structure", 0), P("scaffold", "structure", 1, { level: 60 }), P("hoarding", "perimeter", 0), P("crane", "crane", 0, { lit: true, id: "crane_load" }), P("cabin", "yard", 0, { lit: true }), P("pile_bricks", "yard", 2), P("pile_timber", "yard", 3), P("skip", "yard", 5), P("board", "front", 0), P("hazard_sign", "front", 1), ...workers(3)],
      [lot(2), P("facade.unlit", "structure", 0), P("scaffold", "structure", 1, { level: 100 }), P("hoarding", "perimeter", 0), P("crane", "crane", 0, { lit: true }), P("cabin", "yard", 0, { lit: true }), P("pile_timber", "yard", 3), P("skip", "yard", 5), P("board", "front", 0), ...workers(2)],
      [P("facade.unlit", "structure", 0), P("ribbon", "entrance", 0), P("board", "front", 0)],
    ],
    open: [[P("facade", "structure", 0)]],
    cleared: [lot(3), P("rubble", "yard", 2)],
    plaque: true,
  },
  housing: {
    build: [
      [P("facade.reserved", "structure", 0), P("cabin", "yard", 0, { lit: true }), P("excavator", "yard", 1, { frames: 2 }), ...workers(1)],
      [lot(1), P("heras", "perimeter", 0), P("cabin", "yard", 0, { lit: true }), P("excavator", "yard", 1, { frames: 2 }), P("pile_bricks", "yard", 2), P("board", "front", 0), ...workers(2)],
      [P("facade.construction", "structure", 0), P("heras", "perimeter", 0), P("cabin", "yard", 0, { lit: true }), P("pile_bricks", "yard", 2), P("pile_timber", "yard", 3), P("board", "front", 0), ...workers(2)],
      [P("facade.unlit", "structure", 0), P("scaffold", "structure", 1, { level: 100 }), P("heras", "perimeter", 0), P("cabin", "yard", 0, { lit: true }), P("board", "front", 0), ...workers(2)],
      [P("facade.unlit", "structure", 0), P("board", "front", 0)],
    ],
    open: [[P("facade", "structure", 0)]],
    cleared: [lot(3), P("rubble", "yard", 2)],
    plaque: false,
  },
  road: {
    build: [
      [P("surface", "ground", 0, { level: 0 }), P("barrier", "closure", 0), P("road_sign", "front", 1), P("road_sign", "front", 2), ...cones(4), ...workers(1)],
      [P("surface", "ground", 0, { level: 1 }), P("barrier", "closure", 0), P("excavator", "yard", 1, { frames: 2 }), P("road_sign", "front", 1), P("road_sign", "front", 2), ...cones(4), ...workers(2)],
      [P("surface", "ground", 0, { level: 2 }), P("barrier", "closure", 0), P("road_sign", "front", 1), P("road_sign", "front", 2), ...cones(4), ...workers(2)],
      [P("surface", "ground", 0, { level: 3 }), P("barrier", "closure", 0), P("road_sign", "front", 1), P("road_sign", "front", 2), ...cones(4), ...workers(1)],
      [P("surface", "ground", 0, { level: 4 }), ...cones(2)],
    ],
    open: [[P("surface", "ground", 0, { level: 4 })]],
    cleared: [P("surface", "ground", 0, { level: 5 })],
    plaque: false,
  },
  mine: {
    build: [
      [P("heras", "perimeter", 0), P("cabin", "yard", 0, { lit: true }), P("excavator", "yard", 1, { frames: 2 }), P("board", "front", 0), P("hazard_sign", "front", 1), ...workers(2)],
      [P("collar", "ground", 0), P("heras", "perimeter", 0), P("crane", "crane", 0, { lit: true, id: "crane_load" }), P("cabin", "yard", 0, { lit: true }), P("pile_sheets", "yard", 2), P("board", "front", 0), P("hazard_sign", "front", 1), ...workers(2)],
      [P("headframe_half", "structure", 0), P("heras", "perimeter", 0), P("crane", "crane", 0, { lit: true, id: "crane_load" }), P("cabin", "yard", 0, { lit: true }), P("pile_rebar", "yard", 2), P("board", "front", 0), P("hazard_sign", "front", 1), ...workers(3)],
      [P("headframe", "structure", 0), P("heras", "perimeter", 0), P("crane", "crane", 0, { lit: true }), P("cabin", "yard", 0, { lit: true }), P("board", "front", 0), ...workers(2)],
      [P("headframe", "structure", 0), P("ribbon", "entrance", 0), P("board", "front", 0)],
    ],
    open: [
      [P("headframe", "structure", 0)],
      [P("headframe", "structure", 0, { frames: 3 }), ...workers(1)],
      [P("headframe", "structure", 0, { frames: 3 }), P("lorry_tipper", "yard", 6), ...workers(2)],
    ],
    cleared: [P("cap", "ground", 0)],
    plaque: true,
  },
  factory: {
    build: [
      [lot(0), P("hoarding", "perimeter", 0), P("cabin", "yard", 0, { lit: true }), P("excavator", "yard", 1, { frames: 2 }), P("board", "front", 0), P("hazard_sign", "front", 1), ...workers(2)],
      [lot(1), P("hoarding", "perimeter", 0), P("crane", "crane", 0, { lit: true, id: "crane_load" }), P("cabin", "yard", 0, { lit: true }), P("pile_pipes", "yard", 2), P("pile_sheets", "yard", 3), P("board", "front", 0), P("hazard_sign", "front", 1), ...workers(3)],
      [lot(2), P("frame", "structure", 0), P("scaffold", "structure", 1, { level: 60 }), P("hoarding", "perimeter", 0), P("crane", "crane", 0, { lit: true, id: "crane_load" }), P("cabin", "yard", 0, { lit: true }), P("pile_sheets", "yard", 3), P("board", "front", 0), P("hazard_sign", "front", 1), ...workers(3)],
      [lot(2), P("facade.unlit", "structure", 0), P("chimney", "structure", 2), P("scaffold", "structure", 1, { level: 100 }), P("hoarding", "perimeter", 0), P("crane", "crane", 0, { lit: true }), P("cabin", "yard", 0, { lit: true }), P("board", "front", 0), ...workers(2)],
      [P("facade.unlit", "structure", 0), P("chimney", "structure", 2), P("gates_open", "front", 2), P("ribbon", "entrance", 0), P("board", "front", 0)],
    ],
    open: [
      [P("facade", "structure", 0, { level: 15 }), P("chimney", "structure", 2), P("gates_open", "front", 2)],
      [P("facade", "structure", 0, { level: 55 }), P("chimney", "structure", 2), P("smoke", "structure", 3, { frames: 2 }), P("gates_open", "front", 2), P("goods", "yard", 7), ...workers(1)],
      [P("facade", "structure", 0, { level: 100 }), P("chimney", "structure", 2), P("smoke", "structure", 3, { frames: 2 }), P("gates_open", "front", 2), P("lorry", "yard", 6), P("goods", "yard", 7), P("goods", "yard", 8), P("goods", "yard", 9), ...workers(2)],
    ],
    cleared: [lot(3), P("rubble", "yard", 2)],
    plaque: true,
  },
};

/**
 * What stalling does to a working site: lit things go dark, moving things
 * stop (the crane's load is set down, the excavator parks, the headframe
 * stops), people leave, the gate is shut. A mine or factory that closes
 * (stalled while open) is shuttered: its building boarded, gates chained.
 */
const STALL: Readonly<Record<string, string | null>> = {
  worker: null,
  crane_load: "crane",
  smoke: null,
  gates_open: "gates_shut",
  ribbon: null,
  "facade.unlit": "facade.boarded",
  facade: "facade.boarded",
  lorry: null,
  lorry_tipper: null,
  goods: null,
};

/** Line parts (runs) carry their wear rule; single parts resolve to their chain id. */
const LINE_PARTS = new Set(["heras", "hoarding", "barrier"]);

function stalledParts(kind: WorksKind, base: readonly Spec[], decay: number, seed: number): Spec[] {
  const out: Spec[] = [];
  for (const s of base) {
    let id = s.id;
    if (id in STALL) {
      const to = STALL[id];
      if (to === null || to === undefined) continue;
      id = to;
    }
    const part: Spec = { ...s, id };
    delete part.lit;
    delete part.frames;
    const rule = WEAR[id];
    if (rule && LINE_PARTS.has(id)) {
      part.wear = rule;
      part.decay = decay;
    } else if (rule) {
      const w = wearLevel(rule, seed, s.role, s.n ?? 0, decay);
      part.id = rule.chain[w] as string;
      if (w) part.worn = w;
    }
    out.push(part);
  }
  // A closed mine is fenced off round its shaft; a closed works gets nothing new but its marks.
  if (kind === "mine" && !out.some((p) => p.role === "perimeter")) {
    const rule = WEAR.heras as WearRule;
    out.push(P("heras", "perimeter", 0, { wear: rule, decay }));
  }
  // Marks that need something to hang on: a sheet on an exposed frame or scaffold, buddleia on a frame or an empty building.
  const has = (ids: readonly string[]) => out.some((p) => ids.includes(p.id));
  const needs: Readonly<Record<string, readonly string[]>> = { sheet: ["frame", "scaffold"], buddleia: ["frame", "facade.boarded", "facade.derelict", "facade.construction"] };
  for (const [id, counts] of Object.entries(MARKS[kind])) {
    if (needs[id] && !has(needs[id] as readonly string[])) continue;
    const k = counts[decay] as number;
    for (let n = 0; n < k; n++) out.push(P(id, "marks", n, { mark: true, ...(id === "sheet" ? { frames: 2 } : {}) }));
  }
  return out;
}

/** Plain data: no `undefined` fields, a stable order (role, n, id). */
function finish(parts: readonly Spec[]): WorksPart[] {
  const order = (r: Role) => ROLES.indexOf(r);
  return parts
    .map((s) => {
      const p: WorksPart = { id: s.id, role: s.role, n: s.n ?? 0 };
      if (s.frames && s.frames > 1) p.frames = s.frames;
      if (s.lit) p.lit = true;
      if (s.mark) p.mark = true;
      if (s.level !== undefined) p.level = s.level;
      if (s.wear) {
        p.wear = s.wear;
        p.decay = s.decay ?? 0;
      }
      if (s.worn) p.worn = s.worn;
      return p;
    })
    .sort((a, b) => order(a.role) - order(b.role) || a.n - b.n || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
}

/**
 * The parts a works project shows: pure, deterministic, monotonic (see the
 * file header). `progress` is whole percent built (for stalled and cleared
 * projects, how far it got); `monthsStalled` counts from the stall (ignored
 * unless stalled or cleared); `seed` keys the per-instance wear and the
 * marks' positions (the town seed and project id hashed, as `interiorSeed`).
 */
export function worksLook(kind: WorksKind, phase: WorksPhase, progress: number, monthsStalled: number, seed: number, opts: WorksOptions = {}): WorksLook {
  const rules = WORKS_RULES[kind];
  const industry = kind === "mine" || kind === "factory";
  const activity = industry ? activityLevel(opts.activity ?? 50) : 0;
  let build = buildStage(progress);
  let decay = 0;
  let parts: Spec[];
  if (phase === "building") parts = [...(rules.build[build] as Spec[])];
  else if (phase === "open") {
    build = 4;
    parts = [...((rules.open[Math.min(activity, rules.open.length - 1)] ?? []) as Spec[])];
    if (rules.plaque && opts.sponsor !== false) parts.push(P("plaque", "entrance", 0));
  } else {
    decay = decayStage(monthsStalled);
    // A works that was open and closes (a mine or factory shutting) stalls from its open look; a project from its build stage.
    const base = industry && build === 4 ? (rules.open[0] as Spec[]) : (rules.build[build] as Spec[]);
    const stalled = stalledParts(kind, base, decay, seed);
    if (phase === "stalled") parts = stalled;
    else {
      // Cleared: the site furniture taken away; what was built stays (or comes down) as the kind says; the marks on the ground stay.
      parts = [...rules.cleared];
      if (kind === "flood_wall" || kind === "levee") parts.push(...stalled.filter((p) => p.role === "structure" && p.id !== "piles"));
      if (kind === "mine") parts.push(P("heap", "structure", 1, { level: 0 }));
      const keep = new Set(["weeds", "scrub", "moss", "litter", "puddle"]);
      parts.push(...stalled.filter((p) => p.mark && keep.has(p.id)));
    }
  }
  if (kind === "mine" && !parts.some((p) => p.id === "heap")) {
    // The spoil heap grows with extraction (round 4's three heaps, then this kit's flat-topped tip); a new shaft has none yet.
    const heap = phase === "building" ? 0 : 1 + stageOf(Math.max(0, Math.min(100, opts.extraction ?? 50)), [0, 25, 50, 80]);
    if (heap > 0) parts.push(P("heap", "structure", 1, { level: heap }));
  }
  return { version: WORKS_RULES_VERSION, kind, phase, build, decay, activity, parts: finish(parts) };
}

/** A site worker's look: hi-vis over work clothes, a cap in safety yellow or orange (characters are composed on the device, D10). */
export function workerLook(seed: number, n: number): Look {
  const h = hash32(seed, "worker", n);
  const skins = [0, 1, 2, 3, 4, 5];
  return {
    build: (["slim", "average", "broad"] as const)[h % 3] as Look["build"],
    height: ((h >>> 3) % 3) as 0 | 1 | 2,
    skin: skins[(h >>> 5) % 6] as number,
    hair: "crop",
    hairColour: (["black", "dark_brown", "brown", "auburn", "blonde"] as const)[(h >>> 8) % 5] as Look["hairColour"],
    beard: (h >>> 11) % 4 === 0,
    old: false,
    hat: "cap",
    hatRamp: (h >>> 13) % 2 ? "cloth_mustard" : "flag",
    glasses: false,
    carry: "none",
    bagRamp: "cloth_charcoal",
    top: "hivis",
    topRamp: (["cloth_charcoal", "cloth_navy", "cloth_denim"] as const)[(h >>> 15) % 3] as string,
    bottom: "trousers",
    bottomRamp: (["cloth_denim", "cloth_charcoal", "cloth_navy"] as const)[(h >>> 17) % 3] as string,
    accentRamp: (h >>> 19) % 3 ? "cloth_mustard" : "flag",
    shoeRamp: "wood",
  };
}
