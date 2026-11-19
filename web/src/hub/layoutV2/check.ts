// Validity and readability checks of a layout v2 town (TOWN_GENERATION.md
// §8, §13), used by the tests and cheap enough for a debug check at runtime.
// Each returns human-readable problems; empty means valid.
import { reachableFrom } from "../pathfinding";
import { ROOF_ROWS, Terrain, surfaceOfTerrain, townGrid, type TownMap } from "../townLayout";
import type { TownInput } from "../townInput";
import { BAND_CAPS } from "./geo";
import type { TownPlan } from "./plan";
import { rectDistance } from "./plots";
import { RoadClass, WaterKind } from "./work";

const isWaterBody = (k: number) => k === WaterKind.River || k === WaterKind.Sea || k === WaterKind.Lake;

/** v1's invariants (places, footprints, doors, roofs, reachability, contiguous districts), adapted to plot states, plus v2's (water, levels, bridges, arterials). */
export function checkTownV2(map: TownMap, plan: TownPlan, input: TownInput): string[] {
  const out: string[] = [];
  const W = map.width;
  const N = W * map.height;
  const standing = (i: number) => map.buildings[i]?.state !== "reserved";
  // Every place: exactly one building, in its neighbourhood.
  for (const p of input.places) {
    const bs = map.buildings.filter((b) => b.place === p.id);
    if (bs.length !== 1) out.push(`place ${p.id} (${p.kind}) has ${bs.length} buildings`);
    const b = bs[0];
    if (b && map.districts[b.district]?.id !== p.neighborhood && input.neighborhoods.some((h) => h.id === p.neighborhood)) out.push(`place ${p.id} is not in its neighbourhood`);
    if (b && !standing(b.index)) out.push(`place ${p.id} stands on reserved land`);
  }
  // Footprints: in the map, no overlap, on dry land, on one level; painted when standing; the door in the bottom row, opening onto a walkable pavement.
  const owner = new Int32Array(N).fill(-1);
  for (const b of map.buildings) {
    if (b.x < 0 || b.y < 1 || b.x + b.w > W || b.y + b.h > map.height) {
      out.push(`building ${b.index} out of bounds`);
      continue;
    }
    const lv = map.elevation?.[b.y * W + b.x] ?? 0;
    for (let y = b.y; y < b.y + b.h; y++) {
      for (let x = b.x; x < b.x + b.w; x++) {
        const c = y * W + x;
        if ((owner[c] as number) >= 0) out.push(`buildings ${owner[c]} and ${b.index} overlap at ${x},${y}`);
        owner[c] = b.index;
        if ((plan.water[c] as number) !== WaterKind.None) out.push(`building ${b.index} stands on water at ${x},${y}`);
        if ((map.elevation?.[c] ?? 0) !== lv) out.push(`building ${b.index} crosses a level change at ${x},${y}`);
        const t = map.terrain[c];
        if (standing(b.index) && t !== Terrain.Building && t !== Terrain.Door) out.push(`building ${b.index} cell ${x},${y} is terrain ${t}`);
      }
    }
    if (b.doorY !== b.y + b.h - 1 || b.doorX < b.x || b.doorX >= b.x + b.w) out.push(`building ${b.index} door is not on its front wall`);
    if (b.front !== (b.doorY + 1) * W + b.doorX) out.push(`building ${b.index} front is not below its door`);
    if (standing(b.index) && (map.solid[b.front] || surfaceOfTerrain(map.terrain[b.front] as number) !== "pavement")) out.push(`building ${b.index} door does not open onto a walkable pavement`);
  }
  // Roofs never cover a doorstep or another standing building.
  const fronts = new Set(map.buildings.filter((b) => standing(b.index)).map((b) => b.front));
  for (const b of map.buildings) {
    if (!standing(b.index)) continue;
    for (let y = b.y - ROOF_ROWS; y < b.y; y++) {
      for (let x = b.x; x < b.x + b.w; x++) {
        if (y < 0) continue;
        const c = y * W + x;
        if (fronts.has(c)) out.push(`building ${b.index} roof covers a doorstep at ${x},${y}`);
        const o = owner[c] as number;
        if (o >= 0 && o !== b.index && standing(o)) out.push(`building ${b.index} roof covers building ${o}`);
      }
    }
  }
  // Every door, the start and every district's streets in one walkable component.
  const grid = townGrid(map);
  const doors = map.buildings.filter((b) => standing(b.index)).map((b) => b.front);
  if (doors.length) {
    const reach = reachableFrom(grid, doors[0] as number);
    for (const b of map.buildings) if (standing(b.index) && !reach[b.front]) out.push(`building ${b.index} (${b.kind}) cannot be reached`);
    if (!reach[map.start]) out.push("the start cell cannot be reached");
    for (const d of map.districts) if (d.street < 0 || !reach[d.street]) out.push(`streets of ${d.name} cannot be reached`);
    // Bridges: their ends are on walkable land joined to the streets.
    for (const k of map.connectors ?? []) {
      if (k.kind !== "bridge" && k.kind !== "footbridge") continue;
      const mid = k.axis === "ns" ? k.x + (k.w >> 1) : k.y + (k.h >> 1);
      const ends = k.axis === "ns" ? [(k.y - 1) * W + mid, (k.y + k.h) * W + mid] : [mid * W + k.x - 1, mid * W + k.x + k.w];
      for (const e of ends) if (e < 0 || e >= N || !reach[e] || isWaterBody(plan.water[e] as number)) out.push(`bridge ${k.name || k.kind} at ${k.x},${k.y} does not land on the streets`);
    }
  }
  // A street never runs on water: only bridges, piers and the viaduct's rails stand there.
  for (let c = 0; c < N; c++) {
    if (!isWaterBody(plan.water[c] as number)) continue;
    const t = map.terrain[c] as number;
    if (t !== Terrain.Water && t !== Terrain.Bridge && t !== Terrain.Pier && t !== Terrain.Rail) out.push(`terrain ${t} on water at ${c % W},${Math.floor(c / W)}`);
  }
  // Bridges span no more than their connector type allows.
  const spans = new Map(plan.recipe.connectors.map((k) => [k.id, k.max_span]));
  for (const k of plan.connectors) {
    if (k.kind !== "bridge" && k.kind !== "viaduct" && k.kind !== "footbridge") continue;
    const span = (k.axis === "ns" ? k.h : k.w) - 2;
    const max = spans.get(k.type) ?? 99;
    if (span > max + 2) out.push(`${k.kind} ${k.name} spans ${span} tiles (its type allows ${max})`);
  }
  // Districts: contiguous and non-empty.
  for (const d of map.districts) {
    let first = -1;
    let count = 0;
    for (let i = 0; i < N; i++) {
      if (map.district[i] !== d.index) continue;
      count++;
      if (first < 0) first = i;
    }
    if (count === 0) {
      out.push(`district ${d.name} is empty`);
      continue;
    }
    const seen = new Uint8Array(N);
    const q = [first];
    seen[first] = 1;
    for (let qi = 0; qi < q.length; qi++) {
      const c = q[qi] as number;
      const x = c % W;
      for (const nb of [x > 0 ? c - 1 : -1, x < W - 1 ? c + 1 : -1, c - W, c + W]) {
        if (nb < 0 || nb >= N || seen[nb] || map.district[nb] !== d.index) continue;
        seen[nb] = 1;
        q.push(nb);
      }
    }
    if (q.length !== count) out.push(`district ${d.name} is split (${q.length} of ${count} cells connected)`);
  }
  return out;
}

export interface Readability {
  arterials: number;
  riverBridges: number;
  minBridgeGap: number;
  landmarks: number;
  levels: number;
  shortStreets: number;
  smallBlocks: number;
  /** Worst walking-detour factor from the square to a place (steps over the Manhattan distance), with and without a bridge on the way. */
  detour: number;
  detourAcrossWater: number;
}

/** The §8 readability measures of a town. */
export function readability(map: TownMap, plan: TownPlan): Readability {
  const W = map.width;
  const N = W * map.height;
  const rivers = plan.connectors.filter((c) => c.kind === "bridge" && c.crosses !== "creek");
  let minGap = Infinity;
  for (let i = 0; i < rivers.length; i++) {
    for (let j = i + 1; j < rivers.length; j++) {
      const a = rivers[i] as (typeof rivers)[number];
      const b = rivers[j] as (typeof rivers)[number];
      minGap = Math.min(minGap, Math.abs(a.x + a.w / 2 - b.x - b.w / 2) + Math.abs(a.y + a.h / 2 - b.y - b.h / 2));
    }
  }
  let levels = 0;
  for (let i = 0; i < N; i++) levels = Math.max(levels, (map.elevation?.[i] ?? 0) + 1);
  // Blocks: land between walkable streets; those with standing buildings must be at least 6 × 5.
  const isStreet = (t: number) => t === Terrain.Road || t === Terrain.Pavement || t === Terrain.Crossing || t === Terrain.Dirt || t === Terrain.Path || t === Terrain.Plaza || t === Terrain.Quay || t === Terrain.Bridge || t === Terrain.Steps;
  const seen = new Uint8Array(N);
  let smallBlocks = 0;
  const hasBuilding = new Uint8Array(N);
  for (const b of map.buildings) if (b.state !== "reserved") for (let y = b.y; y < b.y + b.h; y++) for (let x = b.x; x < b.x + b.w; x++) hasBuilding[y * W + x] = 1;
  for (let s = 0; s < N; s++) {
    if (seen[s] || isStreet(map.terrain[s] as number) || map.terrain[s] === Terrain.Water || !hasBuilding[s]) continue;
    const q = [s];
    seen[s] = 1;
    let x0 = W;
    let x1 = -1;
    let y0 = map.height;
    let y1 = -1;
    for (let qi = 0; qi < q.length; qi++) {
      const c = q[qi] as number;
      const x = c % W;
      const y = Math.floor(c / W);
      x0 = Math.min(x0, x);
      x1 = Math.max(x1, x);
      y0 = Math.min(y0, y);
      y1 = Math.max(y1, y);
      for (let k4 = 0; k4 < 4; k4++) {
        const n = k4 === 0 ? (x > 0 ? c - 1 : -1) : k4 === 1 ? (x < W - 1 ? c + 1 : -1) : k4 === 2 ? c - W : c + W;
        if (n < 0 || n >= N || seen[n] || isStreet(map.terrain[n] as number) || map.terrain[n] === Terrain.Water) continue;
        seen[n] = 1;
        q.push(n);
      }
    }
    // The square's civic row stands on the square's paving: not a block.
    const onSquare = x0 >= plan.civic.x && x1 < plan.civic.x + plan.civic.w && y0 >= plan.civic.y - 1 && y1 < plan.civic.y + plan.civic.h;
    if (!onSquare && (x1 - x0 + 1 < 6 || y1 - y0 + 1 < 5)) smallBlocks++;
  }
  // Walking detours from the square.
  const grid = townGrid(map);
  const sq = map.square;
  const from = sq ? (sq.y + (sq.h >> 1)) * W + sq.x + (sq.w >> 1) : map.start;
  const dist = new Int32Array(N).fill(-1);
  const q = new Int32Array(N);
  let qt = 0;
  // From the whole square (its open plaza): a place behind the town hall is not a detour round the square itself.
  if (sq) {
    for (let y = sq.y; y < sq.y + sq.h; y++) {
      for (let x = sq.x; x < sq.x + sq.w; x++) {
        const c = y * W + x;
        if (!((grid.cost[c] ?? 0) > 0)) continue;
        dist[c] = 0;
        q[qt++] = c;
      }
    }
  } else {
    dist[from] = 0;
    q[qt++] = from;
  }
  for (let qh = 0; qh < qt; qh++) {
    const c = q[qh] as number;
    const x = c % W;
    for (let k4 = 0; k4 < 4; k4++) {
        const n = k4 === 0 ? (x > 0 ? c - 1 : -1) : k4 === 1 ? (x < W - 1 ? c + 1 : -1) : k4 === 2 ? c - W : c + W;
      if (n < 0 || n >= N || (dist[n] as number) >= 0 || !((grid.cost[n] ?? 0) > 0)) continue;
      dist[n] = (dist[c] as number) + 1;
      q[qt++] = n;
    }
  }
  let detour = 0;
  let detourAcross = 0;
  const fx = from % W;
  const fy = Math.floor(from / W);
  for (const p of map.places) {
    const b = map.buildings[p.building];
    if (!b) continue;
    const steps = dist[b.front] as number;
    const md = sq ? rectDistance(sq, b.front % W, Math.floor(b.front / W)) : Math.abs((b.front % W) - fx) + Math.abs(Math.floor(b.front / W) - fy);
    if (steps < 0 || md < 16) continue;
    const f = steps / md;
    // Across water when the straight line meets the river.
    let wet = false;
    for (let k = 0; k <= 32 && !wet; k++) {
      const x = Math.round(fx + (((b.front % W) - fx) * k) / 32);
      const y = Math.round(fy + ((Math.floor(b.front / W) - fy) * k) / 32);
      if (isWaterBody(plan.water[y * W + x] as number)) wet = true;
    }
    if (wet) detourAcross = Math.max(detourAcross, f);
    else detour = Math.max(detour, f);
  }
  return {
    arterials: plan.arterials.filter((a) => a.name !== "Ring Road").length,
    riverBridges: rivers.length,
    minBridgeGap: minGap,
    landmarks: plan.landmarks.length,
    levels,
    shortStreets: plan.segments.filter((s) => Math.max(s.x1 - s.x0, s.y1 - s.y0) + 1 < 3).length,
    smallBlocks,
    detour,
    detourAcrossWater: detourAcross,
  };
}

/** Districts touching no main road (an arterial, the ring road, a district's main street or the square's ring; §5 stage 5). */
export function offMainRoads(map: TownMap, plan: TownPlan): string[] {
  const W = map.width;
  const N = W * map.height;
  const main = new Uint8Array(plan.mainRoad);
  for (const s of plan.segments) if (s.cls === RoadClass.Arterial || (s.district < 0 && s.arterial < 0)) for (let y = s.y0; y <= s.y1; y++) for (let x = s.x0; x <= s.x1; x++) main[y * W + x] = 1;
  const touches = new Uint8Array(map.districts.length);
  for (let i = 0; i < N; i++) {
    const d = map.district[i] as number;
    if (d < 0 || touches[d]) continue;
    const x = i % W;
    if (main[i] || (x > 0 && main[i - 1]) || (x < W - 1 && main[i + 1]) || (i >= W && main[i - W]) || (i + W < N && main[i + W])) touches[d] = 1;
  }
  return map.districts.filter((d) => !touches[d.index]).map((d) => `district ${d.name} touches no main road`);
}

/** The §8 caps a town's readability must keep, as problems. */
export function readabilityProblems(map: TownMap, plan: TownPlan): string[] {
  const r = readability(map, plan);
  const caps = BAND_CAPS[plan.band];
  const out: string[] = [...offMainRoads(map, plan)];
  if (r.arterials > caps.arterials[1]) out.push(`${r.arterials} arterials (cap ${caps.arterials[1]})`);
  if (r.riverBridges > caps.bridges) out.push(`${r.riverBridges} bridges (cap ${caps.bridges})`);
  if (r.riverBridges > 1 && plan.band !== "city" && plan.band !== "metro" && r.minBridgeGap < 40) out.push(`bridges ${r.minBridgeGap} tiles apart (at least 40)`);
  if (r.landmarks > 12) out.push(`${r.landmarks} landmarks (at most 12)`);
  if (r.landmarks < 3) out.push(`${r.landmarks} landmarks (at least 3)`);
  if (r.levels > 4) out.push(`${r.levels} levels (at most 4)`);
  if (r.shortStreets) out.push(`${r.shortStreets} streets shorter than 3 tiles`);
  if (r.smallBlocks) out.push(`${r.smallBlocks} built blocks smaller than 6 × 5`);
  if (r.detour > 1.6) out.push(`walking detour ${r.detour.toFixed(2)} (at most 1.6)`);
  if (r.detourAcrossWater > 2.2) out.push(`walking detour across water ${r.detourAcrossWater.toFixed(2)} (at most 2.2)`);
  return out;
}
