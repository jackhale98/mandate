// People coming and going in a room (play note 18, "people pop into a room"). A
// conversation takes time, routines move people, and the engine's view of who is
// here changes: newcomers walk in from the door to their spot, and those who
// are gone walk out through the door. Pure and framework-free (no DOM, no
// clock: `step(dt)` advances), so it is unit-tested; ./interiorView draws it.
// Presentation only: who is here is the engine's, and where they stand is
// ./cast's. With reduced motion nobody walks (the view does not call `sync`).
import type { Facing } from "../../../tools/pixelkit/art/mandate/interiorsRecipes";
import type { Placement } from "./cast";
import { facingOf, type SceneRoom, type Tile } from "./interiorScene";

/** A calmer pace than the player's (tiles a second). */
export const CROWD_SPEED = 2.6;

const DIRS: readonly (readonly [number, number])[] = [
  [0, -1],
  [-1, 0],
  [1, 0],
  [0, 1],
];

/** Someone on their way in (to their spot) or out (to the door). */
export interface CrowdWalker {
  key: string;
  /** Tile coordinates (fractional while walking). */
  x: number;
  y: number;
  dir: Facing;
  /** Tiles walked (drives the walk frames). */
  dist: number;
  path: Tile[];
  leaving: boolean;
  /** Their placement: where they are going (entering) or stood (leaving). */
  person: Placement;
}

/** Who enters and who leaves between two casts, and from where to where. */
export interface CrowdChange {
  entering: { key: string; from: Tile; to: Tile }[];
  leaving: { key: string; from: Tile; to: Tile }[];
}

/** A shortest 4-neighbour route over floor, round `blocked` tiles; the ends are allowed whatever they are. Null if none. */
export function routeTiles(room: Pick<SceneRoom, "cols" | "rows" | "walk">, from: Tile, to: Tile, blocked: ReadonlySet<number>): Tile[] | null {
  const { cols, rows, walk } = room;
  const start = from.y * cols + from.x;
  const goal = to.y * cols + to.x;
  if (start === goal) return [];
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
      if (nx < 0 || ny < 0 || nx >= cols || ny >= rows) continue;
      const j = ny * cols + nx;
      if (prev.has(j)) continue;
      if (j !== goal && (walk[j] !== 1 || blocked.has(j))) continue;
      prev.set(j, i);
      queue.push(j);
    }
  }
  if (!prev.has(goal)) return null;
  const path: Tile[] = [];
  for (let i = goal; i !== start && i !== -1; i = prev.get(i) as number) path.unshift({ x: i % cols, y: Math.floor(i / cols) });
  return path;
}

export class RoomCrowd {
  readonly walkers = new Map<string, CrowdWalker>();

  constructor(
    private readonly room: SceneRoom,
    private readonly speed = CROWD_SPEED,
  ) {}

  get moving(): boolean {
    return this.walkers.size > 0;
  }

  /** Forget everyone on the way (they are simply where the cast puts them). */
  clear() {
    this.walkers.clear();
  }

  /**
   * The cast changed from `before` to `after` (people only, not scenery staff). Newcomers start at the
   * door and walk to their spot; those gone walk from where they stood to the door; someone who comes
   * back while leaving stays; someone leaving mid-entry turns round where they are.
   */
  sync(before: readonly Placement[], after: readonly Placement[]): CrowdChange {
    const { cols } = this.room;
    const door = this.room.door;
    const was = new Map(before.filter((p) => !p.extra).map((p) => [p.key, p]));
    const now = new Map(after.filter((p) => !p.extra).map((p) => [p.key, p]));
    const standing = (except: string) => new Set(after.filter((p) => p.key !== except).map((p) => p.y * cols + p.x));
    const tileOf = (w: CrowdWalker): Tile => ({ x: Math.round(w.x), y: Math.round(w.y) });
    const change: CrowdChange = { entering: [], leaving: [] };
    // Walkers whose walk no longer fits are stopped where they are and re-planned below.
    const stopped = new Map<string, { at: Tile; leaving: boolean }>();
    for (const [key, w] of [...this.walkers]) {
      const p = now.get(key);
      const fits = w.leaving ? !p : !!p && p.x === w.person.x && p.y === w.person.y;
      if (fits) continue;
      stopped.set(key, { at: tileOf(w), leaving: w.leaving });
      this.walkers.delete(key);
    }
    // Newcomers (and leavers who are back, and entrants whose spot moved) walk to their spot.
    for (const [key, p] of now) {
      const s = stopped.get(key);
      if (this.walkers.has(key) || (was.has(key) && !s)) continue;
      const from = s?.at ?? { x: door.x, y: door.y };
      const path = routeTiles(this.room, from, { x: p.x, y: p.y }, standing(key));
      if (!path || path.length === 0) continue;
      this.walkers.set(key, { key, x: from.x, y: from.y, dir: "up", dist: 0, path, leaving: false, person: p });
      change.entering.push({ key, from, to: { x: p.x, y: p.y } });
    }
    // Those gone walk to the door from where they stood (or are, if they were still coming in).
    for (const [key, p] of was) {
      if (now.has(key)) continue;
      const from = stopped.get(key)?.at ?? { x: p.x, y: p.y };
      const path = routeTiles(this.room, from, door, standing(key));
      if (!path || path.length === 0) continue;
      this.walkers.set(key, { key, x: from.x, y: from.y, dir: p.facing, dist: 0, path, leaving: true, person: p });
      change.leaving.push({ key, from, to: { x: door.x, y: door.y } });
    }
    return change;
  }


  /** Advance by `dt` seconds. Arrivals and exits are done with. Returns whether anyone moved. */
  step(dt: number): boolean {
    let any = false;
    for (const w of [...this.walkers.values()]) {
      let budget = Math.max(0, dt) * this.speed;
      while (budget > 0 && w.path.length > 0) {
        const next = w.path[0] as Tile;
        const dx = next.x - w.x;
        const dy = next.y - w.y;
        const d = Math.abs(dx) + Math.abs(dy);
        if (d > 1e-6) w.dir = facingOf(dx, dy);
        if (d <= budget) {
          w.x = next.x;
          w.y = next.y;
          w.dist += d;
          budget -= d;
          w.path.shift();
        } else {
          w.x += (dx / d) * budget;
          w.y += (dy / d) * budget;
          w.dist += budget;
          budget = 0;
        }
        any = true;
      }
      if (w.path.length === 0) {
        this.walkers.delete(w.key);
        any = true;
      }
    }
    return any;
  }
}
