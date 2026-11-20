// Walking inside a room (DEC-088): the player on the room's tile grid (the
// composer's `walk`: blocked, floor, seat), routes by breadth-first search
// around furniture and the people present, and keyboard steering one tile at
// a time. Pure and framework-free (no DOM, no clock: `step(dt)` advances),
// so it is unit-tested; ./interiorView draws it.
import type { Facing } from "../../../tools/pixelkit/art/mandate/interiorsRecipes";

export interface Tile {
  x: number;
  y: number;
}

export interface SceneRoom {
  cols: number;
  rows: number;
  walk: Uint8Array;
  door: Tile;
}

export interface InteriorWalker {
  /** Tile coordinates (fractional while walking). */
  x: number;
  y: number;
  dir: Facing;
  /** Tiles walked (drives the walk frames). */
  dist: number;
  path: Tile[];
  then: (() => void) | null;
}

/** Walking speed, tiles a second (the street's pace); running with Shift. */
export const WALK_SPEED = 4.2;
export const RUN_SPEED = 7;

const DIRS: readonly (readonly [number, number])[] = [
  [0, -1],
  [-1, 0],
  [1, 0],
  [0, 1],
];

export const facingOf = (dx: number, dy: number): Facing => (Math.abs(dx) > Math.abs(dy) ? (dx < 0 ? "left" : "right") : dy < 0 ? "up" : "down");

export class InteriorScene {
  readonly room: SceneRoom;
  readonly player: InteriorWalker;
  /** Tiles where someone stands or sits (not walked through). */
  private occupied = new Set<number>();
  private steerDir: [number, number] = [0, 0];
  private run = false;

  constructor(room: SceneRoom) {
    this.room = room;
    this.player = { x: room.door.x, y: room.door.y, dir: "up", dist: 0, path: [], then: null };
  }

  /** The tiles people take. */
  setOccupied(tiles: Iterable<Tile>) {
    this.occupied = new Set([...tiles].map((t) => t.y * this.room.cols + t.x));
  }

  inside(x: number, y: number): boolean {
    return x >= 0 && y >= 0 && x < this.room.cols && y < this.room.rows;
  }

  /** Whether the player may stand on a tile: floor or a free seat, nobody there. */
  passable(x: number, y: number): boolean {
    if (!this.inside(x, y)) return false;
    const i = y * this.room.cols + x;
    return this.room.walk[i] !== 0 && !this.occupied.has(i);
  }

  /** The tile the player is on (or about to be on). */
  get tile(): Tile {
    return { x: Math.round(this.player.x), y: Math.round(this.player.y) };
  }

  get moving(): boolean {
    return this.player.path.length > 0;
  }

  get steering(): boolean {
    return this.steerDir[0] !== 0 || this.steerDir[1] !== 0;
  }

  /** A shortest route (4-neighbour) from the player's tile to `to`, or null. */
  route(to: Tile): Tile[] | null {
    const { cols } = this.room;
    const from = this.tile;
    if (!this.passable(to.x, to.y) && !(to.x === from.x && to.y === from.y)) return null;
    const start = from.y * cols + from.x;
    const goal = to.y * cols + to.x;
    const prev = new Map<number, number>([[start, -1]]);
    const queue = [start];
    while (queue.length) {
      const i = queue.shift() as number;
      if (i === goal) break;
      const x = i % cols;
      const y = (i - x) / cols;
      for (const [dx, dy] of DIRS) {
        const nx = x + dx;
        const ny = y + dy;
        const j = ny * cols + nx;
        if (!this.passable(nx, ny) || prev.has(j)) continue;
        prev.set(j, i);
        queue.push(j);
      }
    }
    if (!prev.has(goal)) return null;
    const path: Tile[] = [];
    for (let i = goal; i !== start && i !== -1; i = prev.get(i) as number) path.unshift({ x: i % cols, y: Math.floor(i / cols) });
    return path;
  }

  /** Walk to a tile; `then` runs on arrival (at once if already there). Returns false if it cannot be reached. */
  goTo(to: Tile, then: (() => void) | null = null, face?: Facing): boolean {
    const path = this.route(to);
    if (!path) return false;
    this.steerDir = [0, 0];
    const done = () => {
      if (face) this.player.dir = face;
      then?.();
    };
    this.player.path = path;
    this.player.then = done;
    if (path.length === 0) {
      this.player.then = null;
      done();
    }
    return true;
  }

  /** Walk up to a tile someone or something occupies: the nearest free neighbour, then face it. */
  goNear(target: Tile, then: (() => void) | null = null): boolean {
    let best: Tile[] | null = null;
    let bestTile: Tile | null = null;
    const here = this.tile;
    for (const [dx, dy] of DIRS) {
      const t = { x: target.x + dx, y: target.y + dy };
      if (t.x === here.x && t.y === here.y) {
        best = [];
        bestTile = t;
        break;
      }
      const p = this.route(t);
      if (p && (!best || p.length < best.length)) {
        best = p;
        bestTile = t;
      }
    }
    if (!best || !bestTile) return false;
    return this.goTo(bestTile, then, facingOf(target.x - bestTile.x, target.y - bestTile.y));
  }

  /** Put the player on a tile now (no walk). */
  place(t: Tile, dir: Facing = "up") {
    this.player.x = t.x;
    this.player.y = t.y;
    this.player.path = [];
    this.player.then = null;
    this.player.dir = dir;
  }

  /** Keyboard steering: hold a direction to walk tile by tile (0, 0 stops at the next tile). */
  steer(dx: number, dy: number, run = false) {
    this.steerDir = [Math.sign(dx), Math.sign(dy)];
    this.run = run;
    if (this.steering) this.player.then = null;
  }

  /** Advance by `dt` seconds. Returns whether the player moved. */
  step(dt: number): boolean {
    const p = this.player;
    if (p.path.length === 0 && this.steering) this.steerOnce();
    if (p.path.length === 0) return false;
    let budget = dt * (this.run ? RUN_SPEED : WALK_SPEED);
    while (budget > 0 && p.path.length > 0) {
      const next = p.path[0] as Tile;
      const dx = next.x - p.x;
      const dy = next.y - p.y;
      const d = Math.abs(dx) + Math.abs(dy);
      if (d > 1e-6) p.dir = facingOf(dx, dy);
      if (d <= budget) {
        p.x = next.x;
        p.y = next.y;
        p.dist += d;
        budget -= d;
        p.path.shift();
        if (p.path.length === 0) {
          if (this.steering) this.steerOnce();
          if (p.path.length === 0) {
            const then = p.then;
            p.then = null;
            then?.();
          }
        }
      } else {
        p.x += (dx / d) * budget;
        p.y += (dy / d) * budget;
        p.dist += budget;
        budget = 0;
      }
    }
    return true;
  }

  private steerOnce() {
    const t = this.tile;
    const [dx, dy] = this.steerDir;
    const tries: [number, number][] = dx !== 0 && dy !== 0 ? [[dx, 0], [0, dy]] : [[dx, dy]];
    for (const [sx, sy] of tries) {
      if (this.passable(t.x + sx, t.y + sy)) {
        this.player.path = [{ x: t.x + sx, y: t.y + sy }];
        return;
      }
    }
    // Blocked: turn to look that way.
    this.player.dir = facingOf(dx, dy);
  }

  /** Whether the player stands at the entrance (the door tile). */
  get atDoor(): boolean {
    const t = this.tile;
    return !this.moving && t.x === this.room.door.x && t.y === this.room.door.y;
  }
}
