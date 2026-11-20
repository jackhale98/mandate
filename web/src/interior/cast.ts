// Who stands where inside (DEC-088, DEC-079): the engine's place view says who
// is present and how (`here_as`: presiding, member, works, visitor); the
// recipe's `cast` maps that to spot roles in order of preference, and people
// take the first free spot of a role in the recipe's order. Nobody is placed
// by office or name (DEC-098). Extra people stand on free floor near the
// door; staff roles nobody present takes (`extras`: the clerk, the cook) are
// drawn as scenery, like the street's townsfolk: no name, no conversation.
// Presentation only: this decides where people stand, never who is there.
import type { Facing, HotspotDef, InteriorRecipe, SpotDef } from "../../../tools/pixelkit/art/mandate/interiorsRecipes";

/** Walk grid values (pixelkit's `TILE`). */
export const BLOCKED = 0;

export interface CastPerson {
  /** `c<id>` for someone stored, `a<slot>` for a passer-by. */
  key: string;
  /** The place view's `here_as` for them. */
  hereAs: string;
}

export interface Placement {
  /** The person's key, or `x:<spot id>` for a scenery extra. */
  key: string;
  /** The spot they took, or null for someone standing on free floor. */
  spot: string | null;
  role: string;
  x: number;
  y: number;
  facing: Facing;
  pose: "stand" | "sit";
  /** Scenery staff (not a person the engine knows). */
  extra: boolean;
}

export interface CastRoom {
  cols: number;
  rows: number;
  walk: Uint8Array;
  door: { x: number; y: number };
  spots: readonly SpotDef[];
  hotspots: readonly HotspotDef[];
}

export interface CastOptions {
  /** Fill every staff spot nobody took with scenery (by day); otherwise the first of each extra role only (evening). */
  fullStaff?: boolean;
}

/**
 * Place the people present on the room's spots (`recipe.cast`, in the
 * recipe's spot order), then the scenery staff, then anyone left over on
 * free floor nearest the door (the door tile and the one above it kept clear,
 * as are hotspot stands). Deterministic for the same inputs.
 */
export function castRoom(room: CastRoom, recipe: Pick<InteriorRecipe, "cast" | "extras">, people: readonly CastPerson[], opts: CastOptions = {}): Placement[] {
  const free = room.spots.filter((s) => s.role !== "player");
  const taken = new Set<string>();
  const out: Placement[] = [];
  const over: CastPerson[] = [];
  const place = (key: string, s: SpotDef, extra: boolean) => {
    taken.add(s.id);
    out.push({ key, spot: s.id, role: s.role, x: s.x, y: s.y, facing: s.facing, pose: s.pose ?? "stand", extra });
  };
  for (const p of people) {
    const roles = recipe.cast[p.hereAs as keyof InteriorRecipe["cast"]] ?? recipe.cast.visitor ?? [];
    let spot: SpotDef | undefined;
    for (const role of roles) {
      spot = free.find((s) => s.role === role && !taken.has(s.id));
      if (spot) break;
    }
    if (spot) place(p.key, spot, false);
    else over.push(p);
  }
  for (const role of recipe.extras) {
    const open = free.filter((s) => s.role === role && !taken.has(s.id));
    const anyone = out.some((o) => o.role === role);
    const n = opts.fullStaff ? open.length : anyone ? 0 : Math.min(1, open.length);
    for (const s of open.slice(0, n)) place(`x:${s.id}`, s, true);
  }
  if (over.length > 0) {
    const used = new Set(out.map((o) => o.y * room.cols + o.x));
    const keep = new Set<number>([room.door.y * room.cols + room.door.x, (room.door.y - 1) * room.cols + room.door.x]);
    for (const s of room.spots) keep.add(s.y * room.cols + s.x);
    for (const h of room.hotspots) keep.add(h.stand.y * room.cols + h.stand.x);
    const tiles = floorFromDoor(room).filter((i) => !used.has(i) && !keep.has(i));
    over.forEach((p, k) => {
      const i = tiles[k];
      if (i === undefined) return;
      out.push({ key: p.key, spot: null, role: "public", x: i % room.cols, y: Math.floor(i / room.cols), facing: "up", pose: "stand", extra: false });
    });
  }
  return out;
}

/** Floor tiles (not seats, not blocked) in breadth-first order from the door. */
export function floorFromDoor(room: Pick<CastRoom, "cols" | "rows" | "walk" | "door">): number[] {
  const { cols, rows, walk } = room;
  const start = room.door.y * cols + room.door.x;
  const seen = new Uint8Array(cols * rows);
  const order: number[] = [];
  const queue = [start];
  seen[start] = 1;
  while (queue.length) {
    const i = queue.shift() as number;
    if (walk[i] === 1) order.push(i);
    const x = i % cols;
    const y = (i - x) / cols;
    for (const [dx, dy] of [
      [0, -1],
      [-1, 0],
      [1, 0],
      [0, 1],
    ] as const) {
      const nx = x + dx;
      const ny = y + dy;
      if (nx < 0 || ny < 0 || nx >= cols || ny >= rows) continue;
      const j = ny * cols + nx;
      if (seen[j] || walk[j] === BLOCKED) continue;
      seen[j] = 1;
      queue.push(j);
    }
  }
  return order;
}
