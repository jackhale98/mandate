// The player's desk in a room with seats at desks (the debating chamber, DEC-088): the free desk nearest the
// door by walking distance along clear floor, whatever the body's size or the recipe's variant, so the way to
// it never crosses or goes round a seated member. The recipe's own `player` spot is where it would be
// without this (a far end of a row); it is used only when no desk is on an aisle. Presentation only: the
// engine has no seating, who sits where decides no outcome (DEC-079).
import type { Facing, HotspotDef, SpotDef } from "../../../tools/pixelkit/art/mandate/interiorsRecipes";

/** Walk grid values (pixelkit's `TILE`). */
export const BLOCKED = 0;
export const FLOOR = 1;
export const SEAT = 2;

export interface Tile {
  x: number;
  y: number;
}

/** What the seat rules read of a room. */
export interface SeatRoom {
  cols: number;
  rows: number;
  walk: Uint8Array;
  door: Tile;
  spots: readonly SpotDef[];
}

export interface PlayerSeat {
  /** The desk the player takes. */
  seat: SpotDef;
  /** The recipe's own player spot (a member's desk when the player sits elsewhere). */
  home: SpotDef;
}

const STEPS: readonly (readonly [number, number])[] = [
  [0, -1],
  [-1, 0],
  [1, 0],
  [0, 1],
];
const AHEAD: Record<Facing, readonly [number, number]> = { up: [0, -1], down: [0, 1], left: [-1, 0], right: [1, 0] };

/** Walking distance from the door over clear floor only (desks are not walked across), -1 where not reached. */
function floorDistance(room: Pick<SeatRoom, "cols" | "rows" | "walk" | "door">): Int32Array {
  const { cols, rows, walk } = room;
  const dist = new Int32Array(cols * rows).fill(-1);
  const start = room.door.y * cols + room.door.x;
  dist[start] = 0;
  const queue = [start];
  for (let q = 0; q < queue.length; q++) {
    const i = queue[q] as number;
    const x = i % cols;
    const y = (i - x) / cols;
    for (const [dx, dy] of STEPS) {
      const nx = x + dx;
      const ny = y + dy;
      if (nx < 0 || ny < 0 || nx >= cols || ny >= rows) continue;
      const j = ny * cols + nx;
      if (dist[j] !== -1 || walk[j] !== FLOOR) continue;
      dist[j] = (dist[i] as number) + 1;
      queue.push(j);
    }
  }
  return dist;
}

/**
 * Of the recipe's player spot and the member desks like it (seated, facing the same way), the one nearest the
 * door by walking distance along clear floor; the recipe's order breaks ties. If no desk is on an aisle, the
 * recipe's own spot. Null where the room has no player spot.
 */
export function playerSeat(room: SeatRoom): PlayerSeat | null {
  const home = room.spots.find((s) => s.role === "player");
  if (!home) return null;
  if (home.pose !== "sit") return { seat: home, home };
  const dist = floorDistance(room);
  const reach = (s: SpotDef): number => {
    let best = Infinity;
    for (const [dx, dy] of STEPS) {
      const x = s.x + dx;
      const y = s.y + dy;
      if (x < 0 || y < 0 || x >= room.cols || y >= room.rows) continue;
      const d = dist[y * room.cols + x] as number;
      if (d >= 0) best = Math.min(best, d + 1);
    }
    return best;
  };
  let seat: SpotDef = home;
  let best = reach(home);
  for (const s of room.spots) {
    if (s === home || s.role !== "member" || s.pose !== "sit" || s.facing !== home.facing) continue;
    const d = reach(s);
    if (d < best) {
      best = d;
      seat = s;
    }
  }
  return { seat, home };
}

/** The room's spots with the player's desk marked `player` and the recipe's own player spot handed to the members. */
export function seatedSpots(room: SeatRoom): readonly SpotDef[] {
  const ps = playerSeat(room);
  if (!ps || ps.seat === ps.home) return room.spots;
  return room.spots.map((s) => (s === ps.seat ? { ...s, role: "player" } : s === ps.home ? { ...s, role: "member" } : s));
}

/** The shortest walk from the door to a desk over clear floor: the tiles after the door, ending on the desk. Empty if there is none. */
export function seatRoute(room: Pick<SeatRoom, "cols" | "rows" | "walk" | "door">, seat: Tile): Tile[] {
  const dist = floorDistance(room);
  const { cols, rows } = room;
  let at = -1;
  let best = Infinity;
  for (const [dx, dy] of STEPS) {
    const x = seat.x + dx;
    const y = seat.y + dy;
    if (x < 0 || y < 0 || x >= cols || y >= rows) continue;
    const d = dist[y * cols + x] as number;
    if (d >= 0 && d < best) {
      best = d;
      at = y * cols + x;
    }
  }
  if (at < 0) return [];
  const out: Tile[] = [{ x: seat.x, y: seat.y }];
  const door = room.door.y * cols + room.door.x;
  for (;;) {
    if (at !== door) out.unshift({ x: at % cols, y: Math.floor(at / cols) });
    if (at === door) break;
    const x = at % cols;
    const y = (at - x) / cols;
    let next = -1;
    for (const [dx, dy] of STEPS) {
      const nx = x + dx;
      const ny = y + dy;
      if (nx < 0 || ny < 0 || nx >= cols || ny >= rows) continue;
      if (dist[ny * cols + nx] === (dist[at] as number) - 1) {
        next = ny * cols + nx;
        break;
      }
    }
    if (next < 0) break;
    at = next;
  }
  return out;
}

/** The recipe's own player-desk hotspot (its stand is that spot) moved to the desk the player really takes. */
export function retargetHotspots<H extends HotspotDef>(room: SeatRoom, hotspots: readonly H[]): H[] {
  const ps = playerSeat(room);
  if (!ps || ps.seat === ps.home) return [...hotspots];
  const [dx, dy] = AHEAD[ps.seat.facing];
  return hotspots.map((h) =>
    h.stand.x === ps.home.x && h.stand.y === ps.home.y
      ? { ...h, x: ps.seat.x + dx, y: ps.seat.y + dy, w: 1, h: 1, stand: { x: ps.seat.x, y: ps.seat.y }, facing: ps.seat.facing }
      : h,
  );
}

/** Whether a hotspot is a desk the player sits at (its stand is a seat): a tap there is the player's own. */
export function isSeatHotspot(room: Pick<SeatRoom, "cols" | "walk">, h: HotspotDef): boolean {
  return room.walk[h.stand.y * room.cols + h.stand.x] === SEAT;
}
