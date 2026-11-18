// THROWAWAY SPIKE (DEC-050): renderer-agnostic scene data for the hub
// rendering prototype. One street block on a 48x32 tile grid, A* on the
// walkable grid, wandering townsfolk and a player. Nothing here is engine
// code: in the real game, places come from the simulation and walking is UI
// state (DEC-009).

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
  cost: Uint8Array; // A* step cost (0 = blocked)
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

  const cost = new Uint8Array(W * H);
  const targets: number[] = [];
  for (let i = 0; i < W * H; i++) {
    if (blocked[i]) continue;
    const t = ground[i];
    cost[i] = t === ROAD ? 4 : t === GRASS ? 2 : 1;
    if (t === PAVE || t === PLAZA) targets.push(i);
  }
  for (const b of buildings) for (let k = 0; k < 6; k++) targets.push(idx(b.doorX, b.doorY));
  return { ground, blocked, cost, buildings, props, targets };
}

// ---------------------------------------------------------------- A*

const gScore = new Float64Array(W * H);
const came = new Int32Array(W * H);
const stamp = new Uint32Array(W * H);
const closed = new Uint32Array(W * H);
let gen = 0;
const heapN: number[] = [];
const heapF: number[] = [];

function heapPush(n: number, f: number) {
  let i = heapN.length;
  heapN.push(n);
  heapF.push(f);
  while (i > 0) {
    const p = (i - 1) >> 1;
    if ((heapF[p] as number) <= f) break;
    heapN[i] = heapN[p] as number;
    heapF[i] = heapF[p] as number;
    i = p;
  }
  heapN[i] = n;
  heapF[i] = f;
}
function heapPop(): number {
  const top = heapN[0] as number;
  const n = heapN.pop() as number;
  const f = heapF.pop() as number;
  const len = heapN.length;
  if (len > 0) {
    let i = 0;
    for (;;) {
      const l = 2 * i + 1;
      if (l >= len) break;
      const r = l + 1;
      const c = r < len && (heapF[r] as number) < (heapF[l] as number) ? r : l;
      if ((heapF[c] as number) >= f) break;
      heapN[i] = heapN[c] as number;
      heapF[i] = heapF[c] as number;
      i = c;
    }
    heapN[i] = n;
    heapF[i] = f;
  }
  return top;
}

/** 4-neighbour A* over the cost grid. Writes cells (start excluded) into `out`. */
export function astar(town: Town, start: number, goal: number, out: number[]): boolean {
  out.length = 0;
  if (start === goal) return true;
  if (!town.cost[goal]) return false;
  gen++;
  heapN.length = 0;
  heapF.length = 0;
  const gx = goal % W;
  const gy = (goal / W) | 0;
  gScore[start] = 0;
  stamp[start] = gen;
  came[start] = -1;
  heapPush(start, 0);
  while (heapN.length) {
    const cur = heapPop();
    if (cur === goal) break;
    if (closed[cur] === gen) continue;
    closed[cur] = gen;
    const cx = cur % W;
    const cy = (cur / W) | 0;
    const g0 = gScore[cur] as number;
    for (let k = 0; k < 4; k++) {
      const nx = cx + (k === 0 ? 1 : k === 1 ? -1 : 0);
      const ny = cy + (k === 2 ? 1 : k === 3 ? -1 : 0);
      if (nx < 0 || ny < 0 || nx >= W || ny >= H) continue;
      const n = ny * W + nx;
      const c = town.cost[n] as number;
      if (!c) continue;
      const g = g0 + c;
      if (stamp[n] === gen && g >= (gScore[n] as number)) continue;
      stamp[n] = gen;
      gScore[n] = g;
      came[n] = cur;
      heapPush(n, g + Math.abs(nx - gx) + Math.abs(ny - gy));
    }
  }
  if (stamp[goal] !== gen) return false;
  for (let c = goal; c !== start; c = came[c] as number) out.push(c);
  out.reverse();
  return true;
}

/** Nearest walkable cell to (x, y), searching outward. */
export function nearestWalkable(town: Town, x: number, y: number): number {
  for (let r = 0; r < 8; r++) {
    for (let j = y - r; j <= y + r; j++) {
      for (let i = x - r; i <= x + r; i++) {
        if (i < 0 || j < 0 || i >= W || j >= H) continue;
        if (Math.max(Math.abs(i - x), Math.abs(j - y)) !== r) continue;
        if (town.cost[idx(i, j)]) return idx(i, j);
      }
    }
  }
  return -1;
}

// ---------------------------------------------------------------- walkers

export const DIR_DOWN = 0;
export const DIR_UP = 1;
export const DIR_LEFT = 2;
export const DIR_RIGHT = 3;

export interface Walker {
  id: number;
  x: number; // feet position, world px
  y: number;
  cell: number;
  path: number[];
  pi: number;
  speed: number; // px per second
  wait: number;
  variant: number;
  dir: number;
  dist: number; // distance walked, drives the walk cycle
  onArrive: (() => void) | null;
}

export const cellX = (c: number): number => (c % W) * TILE + TILE / 2;
export const cellY = (c: number): number => ((c / W) | 0) * TILE + TILE - 3;

export function makeWalker(id: number, cell: number, variant: number, speed: number): Walker {
  return { id, x: cellX(cell), y: cellY(cell), cell, path: [], pi: 0, speed, wait: 0, variant, dir: DIR_DOWN, dist: 0, onArrive: null };
}

export function walkTo(town: Town, w: Walker, goal: number, onArrive: (() => void) | null = null): boolean {
  const path: number[] = [];
  if (!astar(town, w.cell, goal, path)) return false;
  w.path = path;
  w.pi = 0;
  w.onArrive = onArrive;
  if (path.length === 0 && onArrive) onArrive();
  return true;
}

/** Advance along the path. Returns true while moving. */
export function stepWalker(w: Walker, dt: number): boolean {
  let budget = w.speed * dt;
  let moved = false;
  while (budget > 0 && w.pi < w.path.length) {
    const next = w.path[w.pi] as number;
    const tx = cellX(next);
    const ty = cellY(next);
    const dx = tx - w.x;
    const dy = ty - w.y;
    const d = Math.abs(dx) + Math.abs(dy);
    if (d > 0.001) w.dir = Math.abs(dx) > Math.abs(dy) ? (dx > 0 ? DIR_RIGHT : DIR_LEFT) : dy > 0 ? DIR_DOWN : DIR_UP;
    if (d <= budget) {
      w.x = tx;
      w.y = ty;
      w.cell = next;
      w.pi++;
      budget -= d;
      w.dist += d;
    } else {
      w.x += (dx / d) * budget;
      w.y += (dy / d) * budget;
      w.dist += budget;
      budget = 0;
    }
    moved = true;
  }
  if (w.pi >= w.path.length && w.path.length) {
    w.path = [];
    w.pi = 0;
    const cb = w.onArrive;
    w.onArrive = null;
    if (cb) cb();
  }
  return moved;
}

/** Walk-cycle frame: 0 stand, 1 left foot, 2 right foot. */
export function walkFrame(w: Walker, moving: boolean): number {
  if (!moving) return 0;
  const phase = Math.floor(w.dist / 5) % 4;
  return phase === 0 ? 1 : phase === 2 ? 2 : 0;
}
