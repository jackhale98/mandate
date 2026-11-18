// THROWAWAY SPIKE (DEC-050): renderer-agnostic scene data for the hub
// rendering prototype. One street block on a 48x32 tile grid, routing with
// the M6 hub module (web/src/hub), wandering townsfolk and a player. Nothing here is engine
// code: in the real game, places come from the simulation and walking is UI
// state (DEC-009).

import { Walk, buildGrid, facing4, planRoute, type Footprint, type Grid, type Point, type Surface } from "../../../src/hub/pathfinding";

export const TILE = 16;
export const W = 48;
export const H = 32;

export const GRASS = 0;
export const ROAD = 1;
export const PAVE = 2;
export const PATH = 3;
export const PLAZA = 4;
export const ZEBRA = 5;

/** Small seeded PRNG so every run (and every renderer) sees the same town. */
export function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
export type Rng = () => number;
export const randInt = (rng: Rng, lo: number, hi: number): number => lo + Math.floor(rng() * (hi - lo + 1));

export interface Building {
  id: number;
  x: number; // footprint, tiles
  y: number;
  w: number;
  h: number;
  doorX: number; // door tile (walkable, just south of the footprint)
  doorY: number;
  style: number;
  name: string | null;
  enterable: boolean;
}

export type PropKind = "tree" | "pine" | "lamp" | "bench" | "bin" | "hydrant" | "shelter" | "flag" | "planter";

export interface Prop {
  kind: PropKind;
  x: number; // tile
  y: number;
}

export interface Town {
  ground: Uint8Array; // tile type per cell
  blocked: Uint8Array; // 1 = not walkable
  grid: Grid; // pedestrian costs for routing
  footprints: Footprint[]; // tap targets, parallel to `buildings`
  buildings: Building[];
  props: Prop[];
  targets: number[]; // cells NPCs like to walk to
}

export const idx = (x: number, y: number): number => y * W + x;

export function generate(seed: number): Town {
  const rng = mulberry32(seed);
  const ground = new Uint8Array(W * H).fill(GRASS);
  const blocked = new Uint8Array(W * H);
  const set = (x: number, y: number, t: number) => {
    if (x >= 0 && y >= 0 && x < W && y < H) ground[idx(x, y)] = t;
  };

  // Pavements first, then roads over them.
  for (let x = 0; x < W; x++) for (const y of [14, 17, 29]) set(x, y, PAVE);
  for (let y = 0; y < H; y++) for (const x of [14, 17, 31, 34]) set(x, y, PAVE);
  for (let x = 0; x < W; x++) for (const y of [15, 16, 30, 31]) set(x, y, ROAD);
  for (let y = 0; y < H; y++) for (const x of [15, 16, 32, 33]) set(x, y, ROAD);
  // Zebra crossings mid-block.
  for (const x of [6, 7, 24, 25, 40, 41]) for (const y of [15, 16]) set(x, y, ZEBRA);
  for (const y of [7, 8, 22, 23]) for (const x of [15, 16, 32, 33]) set(x, y, ZEBRA);

  const buildings: Building[] = [];
  const names = ["Diner", "Newspaper office", "Party office", "Library", "Community centre", "Union hall"];
  let nameIdx = 0;
  const addBuilding = (x: number, y: number, w: number, h: number, style: number, name: string | null, enterable = false) => {
    const doorX = x + Math.floor(w / 2);
    const doorY = y + h;
    buildings.push({ id: buildings.length, x, y, w, h, doorX, doorY, style, name, enterable });
    for (let j = y; j < y + h; j++) for (let i = x; i < x + w; i++) blocked[idx(i, j)] = 1;
    set(doorX, doorY, PATH);
  };

  // Two buildings per block; blocks are [x0, x1]; top row faces the main road,
  // bottom row faces the southern road.
  const blocks: [number, number][] = [
    [0, 13],
    [18, 30],
    [35, 47],
  ];
  for (const [row, bottom, maxH] of [
    [0, 12, 8],
    [1, 27, 7],
  ] as const) {
    for (let b = 0; b < blocks.length; b++) {
      const [x0, x1] = blocks[b] as [number, number];
      if (row === 0 && b === 1) {
        // The council chamber, with a forecourt, and a news kiosk.
        for (let i = 19; i <= 26; i++) set(i, 13, PLAZA);
        addBuilding(19, 7, 8, 6, 9, "Council chamber", true);
        addBuilding(28, 10, 3, 3, 3, null);
        continue;
      }
      const avail = x1 - x0 + 1;
      const w1 = randInt(rng, 4, 6);
      const w2 = randInt(rng, 4, Math.min(7, avail - 3 - w1));
      const slack = avail - 3 - w1 - w2;
      const g0 = randInt(rng, 0, slack);
      const xa = x0 + 1 + g0;
      const xb = xa + w1 + 1 + randInt(rng, 0, slack - g0);
      for (const [x, w] of [
        [xa, w1],
        [xb, w2],
      ] as const) {
        const h = randInt(rng, 3, maxH);
        const name = rng() < 0.6 && nameIdx < names.length ? (names[nameIdx++] ?? null) : null;
        addBuilding(x, bottom - h + 1, w, h, randInt(rng, 0, 8), name);
      }
    }
  }

  const props: Prop[] = [];
  const isDoorish = (x: number, y: number) => buildings.some((b) => Math.abs(b.doorX - x) <= 1 && Math.abs(b.doorY - y) <= 1);
  const addProp = (kind: PropKind, x: number, y: number) => {
    if (x < 0 || y < 0 || x >= W || y >= H || blocked[idx(x, y)] || isDoorish(x, y)) return false;
    props.push({ kind, x, y });
    blocked[idx(x, y)] = 1;
    return true;
  };

  // Street furniture along the pavements.
  for (let x = 2; x < W; x += 6) {
    for (const y of [14, 17, 29]) if (ground[idx(x, y)] === PAVE) addProp("lamp", x, y);
  }
  for (let y = 3; y < H; y += 6) {
    for (const x of [14, 17, 31, 34]) if (ground[idx(x, y)] === PAVE) addProp("lamp", x, y);
  }
  addProp("shelter", 22, 17);
  for (const [x, y] of [
    [4, 17],
    [27, 17],
    [44, 14],
    [10, 29],
    [38, 29],
  ] as const)
    addProp("bench", x, y);
  for (const [x, y] of [
    [5, 14],
    [21, 29],
    [36, 17],
    [12, 17],
  ] as const)
    addProp("bin", x, y);
  for (const [x, y] of [
    [9, 14],
    [30, 29],
    [46, 17],
  ] as const)
    addProp("hydrant", x, y);
  addProp("flag", 19, 13);
  addProp("flag", 26, 13);
  for (const [x, y] of [
    [20, 13],
    [25, 13],
  ] as const)
    addProp("planter", x, y);

  // Trees on grass, spaced out, away from doors and building fronts.
  for (let tries = 0; tries < 600; tries++) {
    const x = randInt(rng, 0, W - 1);
    const y = randInt(rng, 1, H - 1);
    if (ground[idx(x, y)] !== GRASS || blocked[idx(x, y)]) continue;
    if (y === 13 || y === 28) continue; // front gardens stay open
    if (props.some((p) => (p.kind === "tree" || p.kind === "pine") && Math.abs(p.x - x) < 3 && Math.abs(p.y - y) < 3)) continue;
    // Keep the tile above free of buildings (the canopy would clip the roof oddly).
    if (y + 1 < H && blocked[idx(x, y + 1)] && buildings.some((b) => x >= b.x && x < b.x + b.w && y + 1 === b.y)) continue;
    addProp(rng() < 0.3 ? "pine" : "tree", x, y);
  }

  const surface = (x: number, y: number): Surface => {
    const i = idx(x, y);
    if (blocked[i]) return "blocked";
    const t = ground[i];
    return t === ROAD ? "road" : t === ZEBRA ? "crossing" : t === GRASS ? "grass" : "pavement";
  };
  const grid = buildGrid(W, H, surface);
  const targets: number[] = [];
  for (let i = 0; i < W * H; i++) {
    if (blocked[i]) continue;
    const t = ground[i];
    if (t === PAVE || t === PLAZA) targets.push(i);
  }
  for (const b of buildings) for (let k = 0; k < 6; k++) targets.push(idx(b.doorX, b.doorY));
  // The roof drawn one tile above the footprint is tappable too.
  const footprints = buildings.map((b) => ({ x: b.x, y: b.y, w: b.w, h: b.h, door: idx(b.doorX, b.doorY), roofRows: 1 }));
  return { ground, blocked, grid, footprints, buildings, props, targets };
}

// ---------------------------------------------------------------- walkers
// Routing, tap targets, and smooth movement come from the M6 hub module
// (web/src/hub, DEC-080): 8-direction A* with pedestrian costs, string-pulled
// routes, and sub-tile interpolation.

export const DIR_DOWN = 0;
export const DIR_UP = 1;
export const DIR_LEFT = 2;
export const DIR_RIGHT = 3;

/** Feet sit this many px below a tile's centre. */
const FEET = 5;

export interface Walker {
  id: number;
  x: number; // feet position, world px
  y: number;
  cell: number;
  walk: Walk | null;
  speed: number; // px per second
  wait: number;
  variant: number;
  dir: number;
  dist: number; // distance walked, drives the walk cycle
  onArrive: (() => void) | null;
}

export const cellX = (c: number): number => (c % W) * TILE + TILE / 2;
export const cellY = (c: number): number => Math.floor(c / W) * TILE + TILE / 2 + FEET;
/** A walker's position in tile units (the hub module's coordinates). */
export const tileOf = (w: Walker): Point => ({ x: w.x / TILE, y: (w.y - FEET) / TILE });

export function makeWalker(id: number, cell: number, variant: number, speed: number): Walker {
  return { id, x: cellX(cell), y: cellY(cell), cell, walk: null, speed, wait: 0, variant, dir: DIR_DOWN, dist: 0, onArrive: null };
}

export function walkTo(town: Town, w: Walker, goal: number, onArrive: (() => void) | null = null): boolean {
  const route = planRoute(town.grid, tileOf(w), goal);
  if (!route) return false;
  w.walk = new Walk(route.points, w.speed / TILE);
  w.onArrive = onArrive;
  return true;
}

/** Advance along the route. Returns true while moving. */
export function stepWalker(w: Walker, dt: number): boolean {
  const walk = w.walk;
  if (!walk) return false;
  const before = walk.distance;
  const p = walk.advance(dt);
  w.x = p.x * TILE;
  w.y = p.y * TILE + FEET;
  w.dist += (walk.distance - before) * TILE;
  w.dir = facing4(p.dx, p.dy, w.dir);
  w.cell = Math.min(H - 1, Math.max(0, Math.floor(p.y))) * W + Math.min(W - 1, Math.max(0, Math.floor(p.x)));
  if (p.done) {
    w.walk = null;
    const cb = w.onArrive;
    w.onArrive = null;
    if (cb) cb();
  }
  return walk.distance > before;
}

/** Walk-cycle frame: 0 stand, 1 left foot, 2 right foot. */
export function walkFrame(w: Walker, moving: boolean): number {
  if (!moving) return 0;
  const phase = Math.floor(w.dist / 5) % 4;
  return phase === 0 ? 1 : phase === 2 ? 2 : 0;
}
