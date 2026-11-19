// Invariants of a laid-out town (used by the tests; cheap enough for a debug
// check at runtime). Returns human-readable problems; empty means valid.
import { reachableFrom, type Grid } from "./pathfinding";
import { ROOF_ROWS, Terrain, surfaceOfTerrain, townGrid, type TownMap } from "./townLayout";
import type { TownInput } from "./townInput";

export function checkTown(map: TownMap, input: TownInput, grid: Grid = townGrid(map)): string[] {
  const out: string[] = [];
  const W = map.width;
  // Every named place: exactly one building.
  for (const p of input.places) {
    const n = map.buildings.filter((b) => b.place === p.id).length;
    if (n !== 1) out.push(`place ${p.id} (${p.kind}) has ${n} buildings`);
    const b = map.buildings.find((x) => x.place === p.id);
    const d = b ? map.districts[b.district] : undefined;
    if (b && d?.id !== p.neighborhood && input.neighborhoods.some((h) => h.id === p.neighborhood)) out.push(`place ${p.id} is not in its neighbourhood`);
  }
  // Footprints: inside the map, no overlap, painted as building with the door in the bottom row.
  const owner = new Int32Array(W * map.height).fill(-1);
  for (const b of map.buildings) {
    if (b.x < 0 || b.y < 1 || b.x + b.w > W || b.y + b.h > map.height) out.push(`building ${b.index} out of bounds`);
    for (let y = b.y; y < b.y + b.h; y++) {
      for (let x = b.x; x < b.x + b.w; x++) {
        const c = y * W + x;
        if ((owner[c] as number) >= 0) out.push(`buildings ${owner[c]} and ${b.index} overlap at ${x},${y}`);
        owner[c] = b.index;
        const t = map.terrain[c];
        if (t !== Terrain.Building && t !== Terrain.Door) out.push(`building ${b.index} cell ${x},${y} is terrain ${t}`);
      }
    }
    if (b.doorY !== b.y + b.h - 1 || b.doorX < b.x || b.doorX >= b.x + b.w) out.push(`building ${b.index} door is not on its front wall`);
    if (b.front !== (b.doorY + 1) * W + b.doorX) out.push(`building ${b.index} front is not below its door`);
    if (map.solid[b.front] || surfaceOfTerrain(map.terrain[b.front] as number) !== "pavement") out.push(`building ${b.index} door does not open onto a walkable pavement`);
  }
  // Roofs (drawn above a footprint) never cover a doorstep or another building.
  const fronts0 = new Set(map.buildings.map((b) => b.front));
  for (const b of map.buildings) {
    for (let y = b.y - ROOF_ROWS; y < b.y; y++) {
      for (let x = b.x; x < b.x + b.w; x++) {
        const c = y * W + x;
        if (y < 0) continue;
        if (fronts0.has(c)) out.push(`building ${b.index} roof covers a doorstep at ${x},${y}`);
        if ((owner[c] as number) >= 0 && owner[c] !== b.index) out.push(`building ${b.index} roof covers building ${owner[c]}`);
      }
    }
  }
  // Every door reachable from every other door (one connected component holds all fronts).
  const fronts = map.buildings.map((b) => b.front);
  if (fronts.length) {
    const reach = reachableFrom(grid, fronts[0] as number);
    for (const b of map.buildings) if (!reach[b.front]) out.push(`building ${b.index} (${b.kind}) cannot be reached`);
    if (!reach[map.start]) out.push("the start cell cannot be reached");
    for (const d of map.districts) if (d.street < 0 || !reach[d.street]) out.push(`streets of ${d.name} cannot be reached`);
  }
  // Districts contiguous (4-connected) and non-empty.
  for (const d of map.districts) {
    let first = -1;
    let count = 0;
    for (let i = 0; i < map.district.length; i++) {
      if (map.district[i] === d.index) {
        count++;
        if (first < 0) first = i;
      }
    }
    if (count === 0) {
      out.push(`district ${d.name} is empty`);
      continue;
    }
    const seen = new Uint8Array(map.district.length);
    const queue = [first];
    seen[first] = 1;
    let n = 0;
    for (let qi = 0; qi < queue.length; qi++) {
      const c = queue[qi] as number;
      n++;
      const x = c % W;
      for (const nb of [x > 0 ? c - 1 : -1, x < W - 1 ? c + 1 : -1, c - W, c + W]) {
        if (nb < 0 || nb >= map.district.length || seen[nb] || map.district[nb] !== d.index) continue;
        seen[nb] = 1;
        queue.push(nb);
      }
    }
    if (n !== count) out.push(`district ${d.name} is split (${n} of ${count} cells connected)`);
  }
  return out;
}

/** Tiles between a building's door and the nearest cell of the central square (0 if on it). */
export function distanceToSquare(map: TownMap, buildingIndex: number): number {
  const b = map.buildings[buildingIndex];
  const s = map.square;
  if (!b || !s) return Infinity;
  const fx = b.front % map.width;
  const fy = Math.floor(b.front / map.width);
  const dx = Math.max(s.x - fx, 0, fx - (s.x + s.w - 1));
  const dy = Math.max(s.y - fy, 0, fy - (s.y + s.h - 1));
  return Math.hypot(dx, dy);
}
