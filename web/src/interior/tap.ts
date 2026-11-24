// What a tap inside a room lands on, first among someone's figure and the player's own desk (the chamber).
// A figure's box reaches above its tile (a standing figure is taller than the tile), so a neighbour behind
// the player's desk overlaps the desk's tiles; there the desk wins. A tap on the neighbour's own tile still
// talks to them. Pure (room pixels in, a target out), so it is unit-tested; ./interiorView calls it.
import { ANCHOR } from "../../../tools/pixelkit/art/mandate/pack";
import type { HotspotDef } from "../../../tools/pixelkit/art/mandate/interiorsRecipes";
import { isSeatHotspot, type SeatRoom } from "./seat";

/** A tile's size in room pixels. */
export const TILE_PX = 16;

export interface TapPerson {
  key: string;
  x: number;
  y: number;
  extra?: boolean;
}

export type TapTarget = { kind: "person"; key: string } | { kind: "hotspot"; id: string } | null;

/**
 * `px`, `py` are room pixels from the floor's top-left corner. `people` are as drawn (tile units, fractional
 * while walking). Returns the person or desk the tap is on, or null (the door, other hotspots and the floor
 * are the view's own).
 */
export function tapTarget(room: Pick<SeatRoom, "cols" | "walk">, hotspots: readonly HotspotDef[], people: readonly TapPerson[], px: number, py: number): TapTarget {
  const tx = Math.floor(px / TILE_PX);
  const ty = Math.floor(py / TILE_PX);
  // Your own desk: its seat and the desk in front of it, unless somebody is on that very tile.
  const own = hotspots.find((h) => isSeatHotspot(room, h) && ((tx >= h.x && tx < h.x + h.w && ty >= h.y && ty < h.y + h.h) || (tx === h.stand.x && ty === h.stand.y)));
  if (own && !people.some((p) => !p.extra && Math.floor(p.x) === tx && Math.floor(p.y) === ty)) return { kind: "hotspot", id: own.id };
  // Someone: their figure's box (a seated one's upper half), the nearer (lower) figure first.
  for (const p of [...people].sort((a, b) => b.y - a.y)) {
    if (p.extra) continue;
    const x0 = p.x * TILE_PX;
    const top = p.y * TILE_PX + 14 - ANCHOR.y;
    if (px >= x0 + 2 && px < x0 + 14 && py >= top && py <= p.y * TILE_PX + 15) return { kind: "person", key: p.key };
  }
  return null;
}
