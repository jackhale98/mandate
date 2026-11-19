// Hub movement: routing on the walkable town grid (DEC-080, M6 pathfinding
// requirements). Pure functions over plain data, renderer-independent: no DOM,
// no canvas. Walking is UI state (DEC-009); nothing here touches the engine.
//
// Coordinates: tile units. Cell (x, y) covers [x, x+1) × [y, y+1); its centre
// is (x + 0.5, y + 0.5). Cell index = y * width + x.

export interface Point {
  x: number;
  y: number;
}

/** A walkable grid: per-cell step cost, 0 = blocked. */
export interface Grid {
  readonly width: number;
  readonly height: number;
  readonly cost: ArrayLike<number>;
}

/** What a cell is to a pedestrian. */
export type Surface = "pavement" | "crossing" | "grass" | "road" | "blocked";

/**
 * Pedestrian costs per tile walked. Pavements (and paths and plazas) are the
 * cheapest, then marked crossings, then grass, then roads. A two-tile road
 * crossed straight costs 1.5 tiles more than pavement, so a crossing about a
 * tile out of the way is used, but nobody walks ten tiles to avoid a road
 * (the spike's 4× road cost did exactly that). Walking along a road is still
 * avoided when a pavement runs beside it.
 */
export const PEDESTRIAN_COSTS: Readonly<Record<Exclude<Surface, "blocked">, number>> = {
  pavement: 1,
  crossing: 1.1,
  grass: 1.25,
  road: 1.75,
};

export function buildGrid(
  width: number,
  height: number,
  surfaceAt: (x: number, y: number) => Surface,
  costs: Readonly<Record<Exclude<Surface, "blocked">, number>> = PEDESTRIAN_COSTS,
): Grid {
  const cost = new Float64Array(width * height);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const s = surfaceAt(x, y);
      cost[y * width + x] = s === "blocked" ? 0 : costs[s];
    }
  }
  return { width, height, cost };
}

export const cellIndex = (g: Grid, x: number, y: number): number => y * g.width + x;
export const cellCentre = (g: Grid, cell: number): Point => ({ x: (cell % g.width) + 0.5, y: Math.floor(cell / g.width) + 0.5 });
export const inGrid = (g: Grid, x: number, y: number): boolean => x >= 0 && y >= 0 && x < g.width && y < g.height;
export const walkable = (g: Grid, x: number, y: number): boolean => inGrid(g, x, y) && (g.cost[y * g.width + x] ?? 0) > 0;
/** The cell containing a point, clamped into the grid. */
export function cellAt(g: Grid, p: Point): number {
  const x = Math.min(g.width - 1, Math.max(0, Math.floor(p.x)));
  const y = Math.min(g.height - 1, Math.max(0, Math.floor(p.y)));
  return y * g.width + x;
}

const SQRT2 = Math.SQRT2;
// Neighbour order is fixed so results are deterministic: E, W, S, N, SE, SW, NE, NW.
const NX = [1, -1, 0, 0, 1, -1, 1, -1];
const NY = [0, 0, 1, -1, 1, 1, -1, -1];

/** Octile distance between cells (diagonal steps cost √2). */
export function octile(dx: number, dy: number): number {
  const ax = Math.abs(dx);
  const ay = Math.abs(dy);
  return Math.max(ax, ay) + (SQRT2 - 1) * Math.min(ax, ay);
}

function minPositiveCost(g: Grid): number {
  let m = Infinity;
  for (let i = 0; i < g.width * g.height; i++) {
    const c = g.cost[i] ?? 0;
    if (c > 0 && c < m) m = c;
  }
  return Number.isFinite(m) ? m : 1;
}

/** Cost of one 8-direction step: length × the mean of both cells' costs. */
const stepCost = (ca: number, cb: number, diagonal: boolean) => ((diagonal ? SQRT2 : 1) * (ca + cb)) / 2;

/** May we step from (x, y) by (dx, dy)? Diagonals never cut a blocked corner. */
function canStep(g: Grid, x: number, y: number, dx: number, dy: number): boolean {
  if (!walkable(g, x + dx, y + dy)) return false;
  if (dx !== 0 && dy !== 0) return walkable(g, x + dx, y) && walkable(g, x, y + dy);
  return true;
}

/** Binary min-heap on (f, h, seq): ties go to the node nearer the goal, then the oldest. */
class Heap {
  private n: number[] = [];
  private f: number[] = [];
  private h: number[] = [];
  private s: number[] = [];
  private seq = 0;
  get size() {
    return this.n.length;
  }
  private less(i: number, j: number) {
    const fi = this.f[i] as number;
    const fj = this.f[j] as number;
    if (fi !== fj) return fi < fj;
    const hi = this.h[i] as number;
    const hj = this.h[j] as number;
    if (hi !== hj) return hi < hj;
    return (this.s[i] as number) < (this.s[j] as number);
  }
  private swap(i: number, j: number) {
    for (const a of [this.n, this.f, this.h, this.s]) {
      const t = a[i] as number;
      a[i] = a[j] as number;
      a[j] = t;
    }
  }
  push(node: number, f: number, h: number) {
    this.n.push(node);
    this.f.push(f);
    this.h.push(h);
    this.s.push(this.seq++);
    let i = this.n.length - 1;
    while (i > 0) {
      const p = (i - 1) >> 1;
      if (!this.less(i, p)) break;
      this.swap(i, p);
      i = p;
    }
  }
  pop(): number {
    const top = this.n[0] as number;
    const last = this.n.length - 1;
    this.swap(0, last);
    for (const a of [this.n, this.f, this.h, this.s]) a.pop();
    let i = 0;
    for (;;) {
      const l = 2 * i + 1;
      if (l >= this.n.length) break;
      const r = l + 1;
      const c = r < this.n.length && this.less(r, l) ? r : l;
      if (!this.less(c, i)) break;
      this.swap(c, i);
      i = c;
    }
    return top;
  }
}

export interface CellPath {
  /** Cells from start to goal, both included. */
  cells: number[];
  cost: number;
}

/**
 * 8-direction A* with an octile heuristic (scaled by the cheapest surface, so
 * it stays admissible) and no corner-cutting. The start cell may be blocked
 * (a walker standing in a doorway); the goal must be walkable.
 */
export function findPath(g: Grid, start: number, goal: number): CellPath | null {
  const size = g.width * g.height;
  if (start < 0 || goal < 0 || start >= size || goal >= size) return null;
  if (!((g.cost[goal] ?? 0) > 0)) return null;
  if (start === goal) return { cells: [start], cost: 0 };
  const hScale = minPositiveCost(g);
  const gx = goal % g.width;
  const gy = Math.floor(goal / g.width);
  const gScore = new Float64Array(size).fill(Infinity);
  const came = new Int32Array(size).fill(-1);
  const closed = new Uint8Array(size);
  const heap = new Heap();
  const h = (x: number, y: number) => hScale * octile(gx - x, gy - y);
  gScore[start] = 0;
  heap.push(start, h(start % g.width, Math.floor(start / g.width)), 0);
  while (heap.size) {
    const cur = heap.pop();
    if (cur === goal) break;
    if (closed[cur]) continue;
    closed[cur] = 1;
    const cx = cur % g.width;
    const cy = Math.floor(cur / g.width);
    const cc = (g.cost[cur] ?? 0) || 1;
    const g0 = gScore[cur] as number;
    for (let k = 0; k < 8; k++) {
      const dx = NX[k] as number;
      const dy = NY[k] as number;
      if (!canStep(g, cx, cy, dx, dy)) continue;
      const n = cur + dy * g.width + dx;
      if (closed[n]) continue;
      const ng = g0 + stepCost(cc, g.cost[n] as number, k >= 4);
      if (ng >= (gScore[n] as number)) continue;
      gScore[n] = ng;
      came[n] = cur;
      const hn = h(cx + dx, cy + dy);
      heap.push(n, ng + hn, hn);
    }
  }
  if (!Number.isFinite(gScore[goal] as number)) return null;
  const cells: number[] = [];
  for (let c = goal; c !== -1; c = came[c] as number) cells.push(c);
  cells.reverse();
  return { cells, cost: gScore[goal] as number };
}

/** Cells reachable from `start` (8-direction, no corner-cutting): 1 = reachable. */
export function reachableFrom(g: Grid, start: number): Uint8Array {
  const out = new Uint8Array(g.width * g.height);
  if (start < 0 || start >= out.length) return out;
  const queue = [start];
  out[start] = 1;
  for (let qi = 0; qi < queue.length; qi++) {
    const cur = queue[qi] as number;
    const cx = cur % g.width;
    const cy = Math.floor(cur / g.width);
    for (let k = 0; k < 8; k++) {
      const dx = NX[k] as number;
      const dy = NY[k] as number;
      if (!canStep(g, cx, cy, dx, dy)) continue;
      const n = cur + dy * g.width + dx;
      if (out[n]) continue;
      out[n] = 1;
      queue.push(n);
    }
  }
  return out;
}

const EPS = 1e-9;

/**
 * Walk the cells a straight segment passes through (a supercover: where the
 * segment crosses a cell corner exactly, both side cells must be walkable, so
 * a straight line can never slip between two diagonal obstacles). Returns the
 * segment's cost (length within each cell × that cell's cost), or Infinity if
 * it touches a blocked cell or leaves the grid. The start cell is allowed to
 * be blocked (see `findPath`).
 */
export function segmentCost(g: Grid, a: Point, b: Point): number {
  let x = Math.floor(a.x);
  let y = Math.floor(a.y);
  if (!inGrid(g, x, y)) return Infinity;
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const len = Math.hypot(dx, dy);
  const startCost = (g.cost[y * g.width + x] ?? 0) || 1;
  if (len < EPS) return 0;
  const sx = dx > 0 ? 1 : dx < 0 ? -1 : 0;
  const sy = dy > 0 ? 1 : dy < 0 ? -1 : 0;
  const tdx = sx !== 0 ? 1 / Math.abs(dx) : Infinity;
  const tdy = sy !== 0 ? 1 / Math.abs(dy) : Infinity;
  let tmx = sx > 0 ? (x + 1 - a.x) / dx : sx < 0 ? (a.x - x) / -dx : Infinity;
  let tmy = sy > 0 ? (y + 1 - a.y) / dy : sy < 0 ? (a.y - y) / -dy : Infinity;
  let t = 0;
  let total = 0;
  let first = true;
  for (;;) {
    const c = first ? startCost : (g.cost[y * g.width + x] ?? 0);
    if (!(c > 0)) return Infinity;
    first = false;
    const tNext = Math.min(tmx, tmy, 1);
    total += (tNext - t) * len * c;
    if (tNext >= 1 - EPS) return total;
    t = tNext;
    if (Math.abs(tmx - tmy) < EPS) {
      // Exactly through a corner: both side cells must be open.
      if (!walkable(g, x + sx, y) || !walkable(g, x, y + sy)) return Infinity;
      x += sx;
      y += sy;
      tmx += tdx;
      tmy += tdy;
    } else if (tmx < tmy) {
      x += sx;
      tmx += tdx;
    } else {
      y += sy;
      tmy += tdy;
    }
    if (!inGrid(g, x, y)) return Infinity;
  }
}

/** True if a walker of half-width `clearance` can follow the segment without touching a blocked cell. */
export function clearSegment(g: Grid, a: Point, b: Point, clearance: number): boolean {
  if (!Number.isFinite(segmentCost(g, a, b))) return false;
  if (clearance <= 0) return true;
  const len = Math.hypot(b.x - a.x, b.y - a.y);
  if (len < EPS) return true;
  const nx = (-(b.y - a.y) / len) * clearance;
  const ny = ((b.x - a.x) / len) * clearance;
  for (const s of [1, -1]) {
    const pa = { x: a.x + s * nx, y: a.y + s * ny };
    const pb = { x: b.x + s * nx, y: b.y + s * ny };
    // `segmentCost` lets a segment start in a blocked cell; an offset line may
    // only do so when it starts in the walker's own cell.
    const own = Math.floor(pa.x) === Math.floor(a.x) && Math.floor(pa.y) === Math.floor(a.y);
    if (!own && !walkable(g, Math.floor(pa.x), Math.floor(pa.y))) return false;
    if (!Number.isFinite(segmentCost(g, pa, pb))) return false;
  }
  return true;
}

/**
 * String-pulling: replace runs of grid steps with straight segments wherever
 * a walker fits (line of sight with `clearance`) and the straight segment
 * costs no more than the steps it replaces, so a route never takes more road
 * than the A* route did (plus `slack`, in cost units per segment: a little
 * slack lets a route cut a road corner instead of stepping round it on the
 * zebra cells, which reads as a wiggle). Greedy: from each anchor, the goal if it qualifies,
 * else the farthest point that does (scanning ahead, giving up after
 * `lookahead` misses in a row).
 */
export function straighten(g: Grid, points: readonly Point[], clearance = 0.25, lookahead = 12, slack = 0): Point[] {
  if (points.length <= 2) return points.map((p) => ({ ...p }));
  const n = points.length;
  const prefix = new Float64Array(n);
  for (let i = 1; i < n; i++) {
    const a = points[i - 1] as Point;
    const b = points[i] as Point;
    const c = segmentCost(g, a, b);
    // A grid step from an off-centre start can graze a corner: count it at its length.
    prefix[i] = (prefix[i - 1] as number) + (Number.isFinite(c) ? c : Math.hypot(b.x - a.x, b.y - a.y) * 2);
  }
  const ok = (i: number, j: number) => {
    const a = points[i] as Point;
    const b = points[j] as Point;
    return segmentCost(g, a, b) <= (prefix[j] as number) - (prefix[i] as number) + slack + 1e-6 && clearSegment(g, a, b, clearance);
  };
  const out: Point[] = [{ ...(points[0] as Point) }];
  let i = 0;
  while (i < n - 1) {
    let best = i + 1;
    if (i + 1 < n - 1 && ok(i, n - 1)) {
      // The goal itself is in plain sight: one segment.
      best = n - 1;
    } else {
      let misses = 0;
      for (let j = i + 2; j < n && misses < lookahead; j++) {
        if (ok(i, j)) {
          best = j;
          misses = 0;
        } else {
          misses++;
        }
      }
    }
    out.push({ ...(points[best] as Point) });
    i = best;
  }
  return out;
}

/**
 * Remove short jogs left after string-pulling: a vertex between two legs, one
 * of them under `maxLeg` tiles, is dropped when the walker fits along the
 * direct line and it costs at most `slack` more than the two legs. Repeats
 * until nothing changes. Used by town routes (see `planRoute`'s `slack`).
 */
export function dejog(g: Grid, points: readonly Point[], clearance = 0.25, slack = 1, maxLeg = 2): Point[] {
  const out = points.map((p) => ({ ...p }));
  let changed = true;
  while (changed && out.length > 2) {
    changed = false;
    for (let i = 1; i < out.length - 1; i++) {
      const a = out[i - 1] as Point;
      const b = out[i] as Point;
      const c = out[i + 1] as Point;
      if (Math.min(Math.hypot(b.x - a.x, b.y - a.y), Math.hypot(c.x - b.x, c.y - b.y)) >= maxLeg) continue;
      const direct = segmentCost(g, a, c);
      if (direct <= segmentCost(g, a, b) + segmentCost(g, b, c) + slack + 1e-6 && clearSegment(g, a, c, clearance)) {
        out.splice(i, 1);
        changed = true;
        break;
      }
    }
  }
  return out;
}

export interface Route {
  /** The A* cells, start to goal. */
  cells: number[];
  /** A* cost of the cells. */
  cost: number;
  /** The straightened route to walk, from the walker's position to the goal cell's centre. */
  points: Point[];
  /** Length of `points`, in tiles. */
  length: number;
}

export function polylineLength(points: readonly Point[]): number {
  let d = 0;
  for (let i = 1; i < points.length; i++) {
    const a = points[i - 1] as Point;
    const b = points[i] as Point;
    d += Math.hypot(b.x - a.x, b.y - a.y);
  }
  return d;
}

/** Plan a walk from a sub-tile position to a goal cell (`slack`: see `straighten`). */
export function planRoute(g: Grid, from: Point, goal: number, clearance = 0.25, slack = 0): Route | null {
  const start = cellAt(g, from);
  const path = findPath(g, start, goal);
  if (!path) return null;
  const centres = path.cells.map((c) => cellCentre(g, c));
  // Walk from where the walker actually is, not from its cell's centre; via
  // the centre only when the first step from there would graze a corner.
  const next = centres[1];
  const direct = next !== undefined && Number.isFinite(segmentCost(g, from, next));
  const raw = direct ? [{ ...from }, ...centres.slice(1)] : [{ ...from }, ...centres];
  const pulled = straighten(g, raw, clearance, 12, slack);
  const points = slack > 0 ? dejog(g, pulled, clearance, slack) : pulled;
  return { cells: path.cells, cost: path.cost, points, length: polylineLength(points) };
}

// ---------------------------------------------------------------------------
// Tap-to-target

/** A building's footprint (cells), the cell in front of its door, and how many rows of roof above it are tappable. */
export interface Footprint {
  x: number;
  y: number;
  w: number;
  h: number;
  door: number;
  roofRows?: number;
}

export type TapTarget = { kind: "door"; building: number; cell: number } | { kind: "ground"; cell: number };

/**
 * Resolve a tap (tile coordinates) to where the walker should go:
 * - on a building (its footprint or roof): that building's door;
 * - on open ground the walker can reach: that cell;
 * - anywhere else (a tree, a wall, a fenced yard): the nearest reachable
 *   cell on the walker's side. The line from the tap back toward the walker
 *   finds the first reachable cell in front of the obstacle; the result is
 *   the cell nearest the tap that is in plain sight of that one, so it is
 *   never behind the building.
 * Returns null if the walker cannot move at all.
 */
export function resolveTap(g: Grid, buildings: readonly Footprint[], from: Point, tap: Point, reach?: Uint8Array): TapTarget | null {
  // Front-most building first (largest base row), then list order.
  let hit = -1;
  let hitBase = -Infinity;
  for (let i = 0; i < buildings.length; i++) {
    const b = buildings[i] as Footprint;
    const top = b.y - (b.roofRows ?? 0);
    if (tap.x >= b.x && tap.x < b.x + b.w && tap.y >= top && tap.y < b.y + b.h && b.y + b.h > hitBase) {
      hit = i;
      hitBase = b.y + b.h;
    }
  }
  if (hit >= 0) return { kind: "door", building: hit, cell: (buildings[hit] as Footprint).door };

  const r = reach ?? reachableFrom(g, cellAt(g, from));
  const t = { x: Math.min(g.width - 1e-6, Math.max(0, tap.x)), y: Math.min(g.height - 1e-6, Math.max(0, tap.y)) };
  const tc = cellAt(g, t);
  if (r[tc]) return { kind: "ground", cell: tc };

  // Step from the tap back toward the walker until a reachable cell.
  const dist = Math.hypot(from.x - t.x, from.y - t.y);
  const steps = Math.max(1, Math.ceil(dist / 0.25));
  let exit = -1;
  for (let s = 1; s <= steps; s++) {
    const p = { x: t.x + ((from.x - t.x) * s) / steps, y: t.y + ((from.y - t.y) * s) / steps };
    const c = cellAt(g, p);
    if (r[c]) {
      exit = c;
      break;
    }
  }
  if (exit < 0) {
    const start = cellAt(g, from);
    return r[start] ? { kind: "ground", cell: start } : null;
  }
  // The reachable cell nearest the tap that the exit cell can see.
  const ec = cellCentre(g, exit);
  const radius = Math.hypot(ec.x - t.x, ec.y - t.y);
  let best = exit;
  let bestD = radius;
  const x0 = Math.max(0, Math.floor(t.x - radius));
  const x1 = Math.min(g.width - 1, Math.ceil(t.x + radius));
  const y0 = Math.max(0, Math.floor(t.y - radius));
  const y1 = Math.min(g.height - 1, Math.ceil(t.y + radius));
  for (let y = y0; y <= y1; y++) {
    for (let x = x0; x <= x1; x++) {
      const c = y * g.width + x;
      if (!r[c]) continue;
      const d = Math.hypot(x + 0.5 - t.x, y + 0.5 - t.y);
      if (d >= bestD - 1e-9) continue;
      if (!Number.isFinite(segmentCost(g, ec, { x: x + 0.5, y: y + 0.5 }))) continue;
      best = c;
      bestD = d;
    }
  }
  return { kind: "ground", cell: best };
}

// ---------------------------------------------------------------------------
// Smooth movement

/** Position at `distance` along a polyline, with the direction of travel there. */
export function pointAlong(points: readonly Point[], distance: number): { x: number; y: number; dx: number; dy: number } {
  const first = points[0];
  if (!first) return { x: 0, y: 0, dx: 0, dy: 0 };
  let left = Math.max(0, distance);
  for (let i = 1; i < points.length; i++) {
    const a = points[i - 1] as Point;
    const b = points[i] as Point;
    const seg = Math.hypot(b.x - a.x, b.y - a.y);
    if (seg < EPS) continue;
    const dx = (b.x - a.x) / seg;
    const dy = (b.y - a.y) / seg;
    if (left <= seg || i === points.length - 1) {
      const k = Math.min(left, seg);
      return { x: a.x + dx * k, y: a.y + dy * k, dx, dy };
    }
    left -= seg;
  }
  return { x: first.x, y: first.y, dx: 0, dy: 0 };
}

/** Default walking speed, tiles per second. */
export const WALK_SPEED = 4;

/** A walk along a route at a constant speed, advanced by frame time. */
export class Walk {
  readonly length: number;
  distance = 0;
  constructor(
    readonly points: readonly Point[],
    readonly speed = WALK_SPEED,
  ) {
    this.length = polylineLength(points);
  }
  get done(): boolean {
    return this.distance >= this.length - EPS;
  }
  /** Advance by `dt` seconds; returns the new position and heading. */
  advance(dt: number): { x: number; y: number; dx: number; dy: number; done: boolean } {
    this.distance = Math.min(this.length, this.distance + Math.max(0, dt) * this.speed);
    const p = pointAlong(this.points, this.distance);
    return { ...p, done: this.done };
  }
}

/** Sprite facing for a heading: 0 down, 1 up, 2 left, 3 right (ties favour the vertical). */
export function facing4(dx: number, dy: number, previous = 0): number {
  if (Math.abs(dx) < EPS && Math.abs(dy) < EPS) return previous;
  if (Math.abs(dx) > Math.abs(dy)) return dx > 0 ? 3 : 2;
  return dy > 0 ? 0 : 1;
}
