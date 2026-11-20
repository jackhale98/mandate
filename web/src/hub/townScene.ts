// The walkable town as UI state (DEC-009: walking is never simulation state):
// the player, a few ambient townsfolk, tap handling and routes, and keyboard
// steering (a held direction walks the player over walkable ground, sliding
// along walls and round corners). Renderer- and DOM-independent; the view
// drives `step(dt)` and draws what is here.
// Townsfolk wander on a seeded stream, so a test (or a screenshot) sees the
// same town every time.
import { Walk, WALK_SPEED, cellCentre, facing4, planRoute, reachableFrom, resolveTap, walkable, type Footprint, type Grid, type Point, type TapTarget } from "./pathfinding";
import { stream, type Rng } from "./rng";
import { Terrain, locationCell, townFootprints, townGrid, type TownMap } from "./townLayout";
import type { TownLocation } from "./townInput";

export interface Walker {
  id: number;
  /** Feet position, tiles. */
  x: number;
  y: number;
  walk: Walk | null;
  /** 0 down, 1 up, 2 left, 3 right (./pathfinding facing4). */
  dir: number;
  /** Distance walked, tiles (drives the walk cycle). */
  dist: number;
  speed: number;
  /** Appearance variant (the player is -1). */
  variant: number;
  moving: boolean;
  wait: number;
  onArrive: (() => void) | null;
}

/**
 * Straightening slack for town routes (./pathfinding `straighten`): up to one
 * pavement-tile of extra cost per straight segment, so a route cuts across a
 * road corner instead of stepping round it on the zebra cells.
 */
export const ROUTE_SLACK = 1;

/** Keyboard steering: the player's half-width against walls (tiles), the run multiplier (Shift) and the longest sub-step (tiles). */
export const STEER_RADIUS = 0.2;
export const STEER_RUN = 1.8;
const STEER_STEP = 0.1;
/** How far to one side a held direction looks for an opening in the wall it meets (tiles). */
const STEER_NUDGE = 1.05;

export type SceneTarget = { kind: "building"; building: number; cell: number } | { kind: "ground"; cell: number };

/** Someone standing in the street to walk up to and talk to (DEC-093): a key the owner knows them by, and a look. */
export interface StandingPerson {
  key: string;
  variant: number;
}

/** How near (tiles) the player must stand to someone to talk to them. */
export const TALK_REACH = 1.6;

export interface TownSceneOptions {
  /** Ambient townsfolk (default 12). */
  townsfolk?: number;
  /** Seed for the townsfolk's wandering (default: the map's seed). */
  seed?: number;
  /** Player walking speed, tiles per second. */
  speed?: number;
}

const makeWalker = (id: number, p: Point, variant: number, speed: number): Walker => ({
  id,
  x: p.x,
  y: p.y,
  walk: null,
  dir: 0,
  dist: 0,
  speed,
  variant,
  moving: false,
  wait: 0,
  onArrive: null,
});

export class TownScene {
  readonly grid: Grid;
  readonly footprints: Footprint[];
  readonly player: Walker;
  readonly folk: Walker[] = [];
  /** Where the player is walking to (the renderer highlights it), or null. */
  target: SceneTarget | null = null;
  /** How many of `folk` are out (the first ones; the rest stay in, e.g. at night). */
  activeFolk: number;
  /** People standing in the street, to walk up to (`setStanding`): they do not wander. */
  readonly standing: { key: string; w: Walker }[] = [];
  /** Dialogue mode (DEC-093): the townsfolk stand where they are until the conversation ends. */
  folkPaused = false;
  private reach: Uint8Array;
  private rng: Rng;
  private spots: number[] = [];
  private steerX = 0;
  private steerY = 0;
  private steerRun = false;

  constructor(
    readonly map: TownMap,
    options: TownSceneOptions = {},
  ) {
    this.grid = townGrid(map);
    this.footprints = townFootprints(map);
    this.player = makeWalker(0, cellCentre(this.grid, map.start), -1, options.speed ?? WALK_SPEED * 1.25);
    this.reach = reachableFrom(this.grid, map.start);
    this.rng = stream(options.seed ?? map.seed, "townsfolk");
    // Townsfolk like pavements, the square and doorsteps.
    for (let i = 0; i < map.terrain.length; i++) {
      if (!this.reach[i]) continue;
      const t = map.terrain[i];
      if (t === Terrain.Pavement || t === Terrain.Plaza || t === Terrain.Quay || t === Terrain.Path) this.spots.push(i);
    }
    for (const b of map.buildings) if (b.place !== null) for (let k = 0; k < 8; k++) this.spots.push(b.front);
    const n = options.townsfolk ?? 12;
    for (let i = 0; i < n && this.spots.length; i++) {
      const w = makeWalker(i + 1, cellCentre(this.grid, this.rng.pick(this.spots)), i, 1.2 + this.rng.next() * 1.2);
      w.wait = this.rng.next() * 3;
      this.folk.push(w);
    }
    this.activeFolk = this.folk.length;
  }

  /** The townsfolk who are out. */
  get outFolk(): readonly Walker[] {
    return this.activeFolk >= this.folk.length ? this.folk : this.folk.slice(0, Math.max(0, this.activeFolk));
  }

  /** The cell the player stands on. */
  get playerCell(): number {
    return Math.floor(this.player.y) * this.map.width + Math.floor(this.player.x);
  }

  /** The building whose door the player is standing at, if any. */
  get atBuilding(): number | null {
    if (this.player.walk) return null;
    const c = this.playerCell;
    const b = this.map.buildings.find((x) => x.front === c);
    return b ? b.index : null;
  }

  /** The place (or home) whose door the player stands at, not walking a route: what Enter visits. */
  get doorHere(): number | null {
    if (this.player.walk) return null;
    const c = this.playerCell;
    const b = this.map.buildings.find((x) => x.front === c && (x.place !== null || x.home) && x.state !== "reserved");
    return b ? b.index : null;
  }

  /**
   * Hold a direction (the keyboard; screen axes, x right and y down; each -1, 0
   * or 1): the player walks that way over walkable ground until let go (0, 0),
   * `run` faster. Steering stops a walk in progress.
   */
  steer(dx: number, dy: number, run = false) {
    this.steerX = Math.sign(dx);
    this.steerY = Math.sign(dy);
    this.steerRun = run;
    if (this.steering) this.cancelWalk();
  }

  /** Whether a direction is held. */
  get steering(): boolean {
    return this.steerX !== 0 || this.steerY !== 0;
  }

  /** Stop the player's walk where they are (no arrival). */
  cancelWalk() {
    const p = this.player;
    p.walk = null;
    p.onArrive = null;
    this.target = null;
  }

  /**
   * The people standing in the street (the talk hook): those already
   * standing stay where they are; new ones stand a few steps from the player
   * on pavement, a square or a path, apart from each other and off doorsteps
   * (a stable choice: the same people around the same spot stand in the same
   * places). Those no longer listed go.
   */
  setStanding(people: readonly StandingPerson[]) {
    const gone: Walker[] = [];
    for (let i = this.standing.length - 1; i >= 0; i--) {
      const s = this.standing[i] as { key: string; w: Walker };
      if (!people.some((p) => p.key === s.key)) gone.unshift(...this.standing.splice(i, 1).map((x) => x.w));
    }
    let fresh = people.filter((p) => !this.standing.some((s) => s.key === p.key));
    // Someone who went as someone new came (a passer-by met becomes someone the
    // engine stores, under a new key): the newcomer stands where they stood.
    while (fresh.length > 0 && gone.length > 0) {
      const p = fresh[0] as StandingPerson;
      // The same look, too.
      const w = gone.shift() as Walker;
      this.standing.push({ key: p.key, w });
      fresh = fresh.slice(1);
    }
    if (fresh.length === 0) return;
    const W = this.map.width;
    const pc = this.playerCell;
    const px = pc % W;
    const py = Math.floor(pc / W);
    const fronts = new Set(this.map.buildings.map((b) => b.front));
    const taken = this.standing.map((s) => Math.floor(s.w.y) * W + Math.floor(s.w.x));
    const cands: number[] = [];
    for (let y = py - 6; y <= py + 6; y++) {
      for (let x = px - 6; x <= px + 6; x++) {
        if (x < 0 || y < 0 || x >= W || y >= this.map.height) continue;
        const d = Math.max(Math.abs(x - px), Math.abs(y - py));
        if (d < 2) continue;
        const i = y * W + x;
        const t = this.map.terrain[i];
        if (!this.reach[i] || fronts.has(i) || !(t === Terrain.Pavement || t === Terrain.Plaza || t === Terrain.Path || t === Terrain.Quay)) continue;
        cands.push(i);
      }
    }
    const dist = (i: number) => Math.hypot((i % W) - px, Math.floor(i / W) - py);
    cands.sort((a, b) => dist(a) - dist(b) || a - b);
    for (const p of fresh) {
      const cell = cands.find((c) => taken.every((t) => Math.max(Math.abs((t % W) - (c % W)), Math.abs(Math.floor(t / W) - Math.floor(c / W))) >= 2));
      if (cell === undefined) continue;
      taken.push(cell);
      const w = makeWalker(-1000 - this.standing.length, cellCentre(this.grid, cell), p.variant, 0);
      this.standing.push({ key: p.key, w });
    }
  }

  /** The person standing within `TALK_REACH` of the player (the nearest), when the player is not walking a route. */
  personNear(): string | null {
    if (this.player.walk) return null;
    let best: string | null = null;
    let bd = TALK_REACH;
    for (const s of this.standing) {
      const d = Math.hypot(s.w.x - this.player.x, s.w.y - this.player.y);
      if (d <= bd) {
        bd = d;
        best = s.key;
      }
    }
    return best;
  }

  /** The person standing at a point (tiles), if any (a tap on them). */
  personAt(p: Point): string | null {
    let best: string | null = null;
    let bd = 0.8;
    for (const s of this.standing) {
      const d = Math.hypot(s.w.x - p.x, s.w.y - 0.4 - p.y);
      if (d <= bd) {
        bd = d;
        best = s.key;
      }
    }
    return best;
  }

  /**
   * Walk up to someone standing in the street (beside them, left or right,
   * the nearer side first, so the conversation frames them face to face;
   * else in front or behind); `onArrive` once there (at once if already).
   */
  goToPerson(key: string, onArrive?: () => void): boolean {
    const s = this.standing.find((x) => x.key === key);
    if (!s) return false;
    if (this.personNear() === key) {
      this.cancelWalk();
      onArrive?.();
      return true;
    }
    const W = this.map.width;
    const cx = Math.floor(s.w.x);
    const cy = Math.floor(s.w.y);
    const around = [
      [0, 1],
      [1, 0],
      [-1, 0],
      [0, -1],
    ]
      .map(([dx, dy]) => (cy + (dy as number)) * W + cx + (dx as number))
      .filter((c) => c >= 0 && c < this.reach.length && this.reach[c])
      .sort((a, b) => {
        const side = (c: number) => (Math.floor(c / W) === cy ? 0 : 1);
        const d = (c: number) => Math.hypot((c % W) + 0.5 - this.player.x, Math.floor(c / W) + 0.5 - this.player.y);
        return side(a) - side(b) || d(a) - d(b);
      });
    for (const c of around) if (this.goToCell(c, onArrive)) return true;
    return false;
  }

  /** Resolve a tap at a point in tiles and walk there. Returns where to, or null. */
  tap(p: Point, onArrive?: () => void): TapTarget | null {
    const t = resolveTap(this.grid, this.footprints, this.player, p, this.reach);
    if (!t) return null;
    const ok = t.kind === "door" ? this.goToBuilding(t.building, onArrive) : this.goToCell(t.cell, onArrive);
    return ok ? t : null;
  }

  goToBuilding(index: number, onArrive?: () => void): boolean {
    const b = this.map.buildings[index];
    if (!b) return false;
    if (!this.walkTo(this.player, b.front, onArrive)) return false;
    this.target = { kind: "building", building: index, cell: b.front };
    return true;
  }

  goToCell(cell: number, onArrive?: () => void): boolean {
    if (!this.walkTo(this.player, cell, onArrive)) return false;
    this.target = { kind: "ground", cell };
    return true;
  }

  /** Walk to an engine location (a place's door, home, or a neighbourhood's streets). */
  goToLocation(loc: TownLocation, onArrive?: () => void): boolean {
    const cell = locationCell(this.map, loc);
    if (cell === null) return false;
    const b = this.map.buildings.find((x) => x.front === cell && (loc.type !== "home" || x.home));
    return b && loc.type !== "street" ? this.goToBuilding(b.index, onArrive) : this.goToCell(cell, onArrive);
  }

  /** Put the player somewhere at once (a skipped walk, or the engine moved them). */
  placePlayer(cell: number) {
    const c = cellCentre(this.grid, cell);
    this.player.x = c.x;
    this.player.y = c.y;
    this.player.walk = null;
    this.player.moving = false;
    this.target = null;
  }

  private walkTo(w: Walker, goal: number, onArrive?: () => void): boolean {
    const r = planRoute(this.grid, { x: w.x, y: w.y }, goal, 0.25, ROUTE_SLACK);
    if (!r) return false;
    w.walk = new Walk(r.points, w.speed);
    w.onArrive = onArrive ?? null;
    return true;
  }

  /** Advance everyone by dt seconds. Returns whether anything moved. */
  step(dt: number): boolean {
    let any = this.stepWalker(this.player, dt);
    if (this.steering) any = this.stepSteer(dt) || any;
    if (!this.player.walk && !this.player.moving) this.target = null;
    for (const w of this.folkPaused ? [] : this.outFolk) {
      const m = this.stepWalker(w, dt);
      any ||= m;
      if (!w.walk) {
        w.wait -= dt;
        if (w.wait <= 0) {
          const goal = this.rng.pick(this.spots);
          w.wait = this.walkTo(w, goal) ? 1 + this.rng.next() * 4 : 1;
        }
      }
    }
    return any;
  }

  /** Whether the player is walking, a route or steered (the view keeps drawing at full rate). */
  get playerMoving(): boolean {
    return this.player.walk !== null || (this.steering && this.player.moving);
  }

  private stepSteer(dt: number): boolean {
    const p = this.player;
    const len = Math.hypot(this.steerX, this.steerY);
    const dx = this.steerX / len;
    const dy = this.steerY / len;
    let left = p.speed * (this.steerRun ? STEER_RUN : 1) * Math.max(0, dt);
    let mx = 0;
    let my = 0;
    while (left > 1e-9) {
      const s = Math.min(left, STEER_STEP);
      left -= s;
      const m = this.slide(dx, dy, s);
      if (!m) break;
      mx += m.x;
      my += m.y;
    }
    const moved = Math.hypot(mx, my);
    p.dist += moved;
    p.dir = moved > 0 ? facing4(mx, my, p.dir) : facing4(dx, dy, p.dir);
    p.moving = moved > 0;
    return moved > 0;
  }

  /** Whether the player's box (half-width r) at (x, y) is on walkable cells only. */
  private free(x: number, y: number, r: number): boolean {
    const g = this.grid;
    const x0 = Math.floor(x - r);
    const x1 = Math.floor(x + r);
    const y0 = Math.floor(y - r);
    const y1 = Math.floor(y + r);
    return walkable(g, x0, y0) && walkable(g, x1, y0) && walkable(g, x0, y1) && walkable(g, x1, y1);
  }

  /**
   * One sub-step of `s` tiles along the unit vector (dx, dy): straight on if
   * free; held diagonally against a wall, along it; held straight into a
   * corner or a diagonal street's step, 45° round it. Returns the move made.
   */
  private slide(dx: number, dy: number, s: number): Point | null {
    const p = this.player;
    // In a wall's margin (stood at a door, at a route's corner): a point is enough to walk out.
    const r = this.free(p.x, p.y, STEER_RADIUS) ? STEER_RADIUS : 0;
    const go = (ux: number, uy: number, k: number): Point | null => {
      const nx = p.x + ux * s * k;
      const ny = p.y + uy * s * k;
      if (!this.free(nx, ny, r)) return null;
      const m = { x: nx - p.x, y: ny - p.y };
      p.x = nx;
      p.y = ny;
      return m;
    };
    const straight = go(dx, dy, 1);
    if (straight) return straight;
    if (dx !== 0 && dy !== 0) return go(Math.sign(dx), 0, Math.abs(dx)) ?? go(0, Math.sign(dy), Math.abs(dy));
    const c = Math.SQRT1_2;
    const round = go((dx - dy) * c, (dy + dx) * c, 1) ?? go((dx + dy) * c, (dy - dx) * c, 1);
    if (round) return round;
    // Held straight at a wall with an opening just to one side (a side street, an alley, a diagonal
    // street's next step): sidestep towards the nearer opening. A plain wall stops the player.
    const open = [true, true];
    for (let d = STEER_STEP; d <= STEER_NUDGE + 1e-9; d += STEER_STEP) {
      for (const [k, side] of [1, -1].entries()) {
        if (!open[k]) continue;
        const sx = p.x - dy * side * d;
        const sy = p.y + dx * side * d;
        if (!this.free(sx, sy, r)) open[k] = false;
        else if (this.free(sx + dx * s, sy + dy * s, r)) return go(-dy * side, dx * side, 1);
      }
    }
    return null;
  }

  private stepWalker(w: Walker, dt: number): boolean {
    const walk = w.walk;
    if (!walk) {
      w.moving = false;
      return false;
    }
    const before = walk.distance;
    const p = walk.advance(dt);
    w.x = p.x;
    w.y = p.y;
    w.dist += walk.distance - before;
    w.dir = facing4(p.dx, p.dy, w.dir);
    w.moving = walk.distance > before;
    if (p.done) {
      w.walk = null;
      w.moving = false;
      const cb = w.onArrive;
      w.onArrive = null;
      cb?.();
    }
    return walk.distance > before || p.done;
  }
}

