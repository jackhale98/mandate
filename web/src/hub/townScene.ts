// The walkable town as UI state (DEC-009: walking is never simulation state):
// the player, a few ambient townsfolk, tap handling and routes. Renderer- and
// DOM-independent; the view drives `step(dt)` and draws what is here.
// Townsfolk wander on a seeded stream, so a test (or a screenshot) sees the
// same town every time.
import { Walk, WALK_SPEED, cellCentre, facing4, planRoute, reachableFrom, resolveTap, type Footprint, type Grid, type Point, type TapTarget } from "./pathfinding";
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

export type SceneTarget = { kind: "building"; building: number; cell: number } | { kind: "ground"; cell: number };

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
  private reach: Uint8Array;
  private rng: Rng;
  private spots: number[] = [];

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
    if (!this.player.walk && !this.player.moving) this.target = null;
    for (const w of this.folk) {
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

  /** Whether the player is walking (the view keeps drawing at full rate). */
  get playerMoving(): boolean {
    return this.player.walk !== null;
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

