// Validity and readability checks of a layout v2 town (TOWN_GENERATION.md
// §8, §13), used by the tests and cheap enough for a debug check at runtime.
// Each returns human-readable problems; empty means valid.
import { reachableFrom } from "../pathfinding";
import { ROOF_ROWS, Terrain, surfaceOfTerrain, townGrid, type TownMap } from "../townLayout";
import type { TownInput } from "../townInput";
import { BAND_CAPS } from "./geo";
import { centreOffset, forBand, headCells, headShape, isHead, parallelGap, segCells, segSteps, streetEndOf } from "./net";
import type { TownPlan } from "./plan";
import { rectDistance } from "./plots";
import { endState } from "./streets";
import { RoadClass, WaterKind, type Segment } from "./work";

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
  out.push(...streetProblems(map, plan));
  return out;
}

const STREET = new Set<number>([Terrain.Road, Terrain.Pavement, Terrain.Crossing, Terrain.Bridge, Terrain.Plaza, Terrain.Quay, Terrain.Steps, Terrain.Dirt, Terrain.Path, Terrain.Ford]);

/**
 * The street network's faults (DEC-090 D5 amended; each a check that would
 * have caught a fault the owner reported):
 * - an end that meets nothing (only designed turning heads, and the roads to
 *   the neighbouring towns at the map's edge, may end), including a street
 *   that stops at the railway with no crossing;
 * - streets that do not join the network (one walkable network of streets);
 * - near-parallel duplicates (two runs alongside within three cells) and jogs
 *   (a street continuing a few cells aside across a junction);
 * - a road painted with no street run under it (a stray stub);
 * - the railway: crossed only square-on at level crossings or by bridges;
 *   a plausible line (at most three runs, no loop round the centre, ends on
 *   the map's edge, clear of the square), the station beside it;
 * - arterials: no staircase (no short run between two turns);
 * - the map's edge: crossed only by the roads to the neighbouring towns (and
 *   at most two farm tracks).
 */
export function streetProblems(map: TownMap, plan: TownPlan): string[] {
  const out: string[] = [];
  const W = map.width;
  const H = map.height;
  const N = W * H;
  const segs = plan.segments;
  const cover = new Uint16Array(N);
  for (const s of segs) {
    segCells(s, W, H, (c) => (cover[c] = (cover[c] as number) + 1));
    for (const end of [0, 1] as const) if (isHead(s.ends[end])) for (const c of headCells(s, end, W, H)) cover[c] = (cover[c] as number) + 1;
  }
  const sq = (c: number) => map.terrain[c] === Terrain.Plaza && rectDistance(plan.square, c % W, Math.floor(c / W)) === 0;
  const link = (s: Segment) => s.arterial >= 0 && (plan.arterials[s.arterial]?.mode ?? "local") !== "local";
  const name = (s: Segment) => `${s.arterial >= 0 ? (plan.arterials[s.arterial]?.name ?? "arterial") : s.district >= 0 ? `a street of ${plan.districts[s.district]?.name ?? "?"}` : "the square's ring"} (${s.axis} ${s.ax},${s.ay}–${s.bx},${s.by})`;
  const railAt = (c: number) => map.terrain[c] === Terrain.Rail;
  // ---- ends: a junction, a road to another town at the map's edge, or a recorded termination with what it needs there
  const headed = new Uint8Array(N);
  const recorded = map.streetEnds ?? [];
  const propAt = (kind: string, cells: number[]) => map.props.some((p) => p.kind === kind && cells.includes(p.y * W + p.x));
  for (const s of segs) {
    for (const end of [0, 1] as const) {
      const kind = s.ends[end];
      const t = end === 0 ? 0 : segSteps(s);
      const out1 = end === 0 ? -1 : 1;
      const band = (p: number) => {
        const cells: number[] = [];
        forBand(s, p, p, (x, y) => {
          if (x >= 0 && y >= 0 && x < W && y < H) cells.push(y * W + x);
        });
        return cells;
      };
      if (kind) {
        const rec = streetEndOf(s, end, W, H);
        if (!rec || !recorded.some((r) => r.kind === kind && r.x === rec.x && r.y === rec.y && r.w === rec.w && r.h === rec.h && r.facing === rec.facing)) out.push(`${kind} of ${name(s)} is not recorded in the street ends`);
        const endCells = band(t);
        const beyond = [...band(t + out1), ...band(t + 2 * out1)];
        if (isHead(kind)) {
          const { road, all } = headShape(s, end, W, H);
          const paved = road.filter((c) => STREET.has(map.terrain[c] as number) || map.terrain[c] === Terrain.Embankment).length;
          if (paved * 10 < road.length * 8) out.push(`${kind} of ${name(s)} is not paved`);
          for (const c of all) headed[c] = 1;
        } else if (kind === "plaza") {
          if (endState(s, end, W, H, (c) => cover[c] as number, sq) !== "square") out.push(`plaza end of ${name(s)} does not meet the square`);
        } else if (kind === "bollards") {
          if (!propAt("bollards", endCells)) out.push(`bollards end of ${name(s)} has no bollards`);
          if (!band(t + out1).some((c) => map.terrain[c] === Terrain.Path)) out.push(`bollards end of ${name(s)} has no footpath beyond`);
        } else if (kind === "railing") {
          if (!propAt("railing", endCells)) out.push(`railing end of ${name(s)} has no railing`);
          if (!band(t + out1).some((c) => isWaterBody(plan.water[c] as number) || map.terrain[c] === Terrain.Quay)) out.push(`railing end of ${name(s)} is not at the water`);
        } else if (kind === "field_gate") {
          if (!propAt("field_gate", endCells)) out.push(`field gate end of ${name(s)} has no gate`);
          if (!band(t + out1).some((c) => map.terrain[c] === Terrain.Dirt || map.terrain[c] === Terrain.Field)) out.push(`field gate end of ${name(s)} does not open onto a track or a field`);
        }
        void beyond;
        // The end's art zone (its cells and two round them, three on a 45° street) holds no other street or junction.
        if (rec) {
          const pad = s.axis === "ne" || s.axis === "nw" ? 3 : 2;
          const own = new Set<number>();
          segCells(s, W, H, (c) => own.add(c));
          for (const e2 of [0, 1] as const) if (isHead(s.ends[e2])) for (const c of headCells(s, e2, W, H)) own.add(c);
          let near = false;
          for (let y = rec.y - pad; y < rec.y + rec.h + pad && !near; y++) {
            for (let x = rec.x - pad; x < rec.x + rec.w + pad && !near; x++) {
              if (x < 0 || y < 0 || x >= W || y >= H) continue;
              const c = y * W + x;
              const n = cover[c] as number;
              if ((own.has(c) ? n >= 2 : n >= 1) || (kind !== "plaza" && sq(c))) near = true;
            }
          }
          if (near) out.push(`${kind} of ${name(s)} is within ${pad} cells of another street`);
        }
        continue;
      }
      const st = endState(s, end, W, H, (c) => cover[c] as number, sq);
      if (st === "joined") continue;
      if (st === "square") {
        out.push(`${name(s)} meets the square with no plaza end`);
        continue;
      }
      if (st === "edge" && link(s)) continue;
      let rail = false;
      const tb = end === 0 ? -1 : segSteps(s) + 1;
      forBand(s, tb, tb, (x, y) => {
        if (x >= 0 && y >= 0 && x < W && y < H && railAt(y * W + x)) rail = true;
      });
      out.push(st === "edge" ? `${name(s)} leaves the map but is no road to another town` : rail ? `${name(s)} stops at the railway with no crossing` : `${name(s)} ends in nothing`);
    }
  }
  // ---- every road cell under a run (or a turning head); no run broken by the railway; crossings square-on
  for (const s of segs) {
    let onRail = 0;
    let crossings = 0;
    segCells(s, W, H, (c) => {
      if (railAt(c)) onRail++;
      if (map.terrain[c] === Terrain.Crossing) crossings++;
    });
    if (onRail) out.push(`${name(s)} meets the railway with no crossing (${onRail} cells)`);
    if (crossings && s.axis !== "h" && s.axis !== "v") out.push(`${name(s)} crosses the railway at 45°`);
  }
  for (let c = 0; c < N; c++) {
    const t = map.terrain[c] as number;
    if (t === Terrain.Road && !cover[c] && !headed[c]) out.push(`road at ${c % W},${Math.floor(c / W)} belongs to no street`);
  }
  // ---- duplicates and jogs (two arterial runs joined by a 45° run of the arterials are a clean bend, not a jog)
  const boxes = (p: Segment, q: Segment) => p.x0 <= q.x1 + 1 && q.x0 <= p.x1 + 1 && p.y0 <= q.y1 + 1 && q.y0 <= p.y1 + 1;
  const bentAt45 = (a: Segment, b: Segment) => (a.arterial >= 0 || b.arterial >= 0) && segs.some((d) => d.arterial >= 0 && (d.axis === "ne" || d.axis === "nw") && segSteps(d) >= 4 && boxes(d, a) && boxes(d, b));
  /** The bounding box of a run's end band (the end nearer to `p`), one cell wider all round. */
  const endBox = (s: Segment, p: Segment) => {
    const da = Math.abs(s.ax - p.ax) + Math.abs(s.ay - p.ay) + Math.abs(s.ax - p.bx) + Math.abs(s.ay - p.by);
    const db = Math.abs(s.bx - p.ax) + Math.abs(s.by - p.ay) + Math.abs(s.bx - p.bx) + Math.abs(s.by - p.by);
    const t = da <= db ? 0 : segSteps(s);
    let x0 = W;
    let y0 = H;
    let x1 = -1;
    let y1 = -1;
    forBand(s, t, t, (x, y) => {
      x0 = Math.min(x0, x - 1);
      y0 = Math.min(y0, y - 1);
      x1 = Math.max(x1, x + 1);
      y1 = Math.max(y1, y + 1);
    });
    return { x0, y0, x1, y1 };
  };
  const meets = (q: Segment, r: { x0: number; y0: number; x1: number; y1: number }) => q.x0 <= r.x1 && r.x0 <= q.x1 && q.y0 <= r.y1 && r.y0 <= q.y1;
  /** A jog: the two runs' facing ends meet the same cross street (a street continuing a few cells aside across a junction). */
  const acrossOneJunction = (a: Segment, b: Segment) => {
    const ea = endBox(a, b);
    const eb = endBox(b, a);
    return segs.some((x) => x !== a && x !== b && x.axis !== a.axis && meets(x, ea) && meets(x, eb));
  };
  for (let i = 0; i < segs.length; i++) {
    const a = segs[i] as Segment;
    for (let j = i + 1; j < segs.length; j++) {
      const b = segs[j] as Segment;
      if (a.axis !== b.axis) continue;
      const g = parallelGap(a, b);
      const off = centreOffset(a, b);
      if (off === 0) continue;
      if (g.overlap >= 4 && g.gap <= 3) out.push(`${name(a)} and ${name(b)} run alongside ${Math.max(0, g.gap)} cells apart`);
      else if (g.overlap > -7 && g.overlap < 4 && off <= 5 && g.gap > -Math.max(a.width, b.width) && !bentAt45(a, b) && acrossOneJunction(a, b)) out.push(`${name(a)} and ${name(b)} jog ${off} cells`);
    }
    for (const r of plan.rail) {
      if (a.axis !== r.axis) continue;
      const g = parallelGap(a, r);
      if (g.overlap >= 4 && g.gap <= 2 && g.gap >= -1) out.push(`${name(a)} runs alongside the railway ${Math.max(0, g.gap)} cells away`);
    }
  }
  // ---- one network: every street joined to the square
  {
    const seen = new Uint8Array(N);
    const q: number[] = [];
    const c0 = plan.square.y * W + plan.square.x;
    seen[c0] = 1;
    q.push(c0);
    const street = (c: number) => STREET.has(map.terrain[c] as number);
    for (let qi = 0; qi < q.length; qi++) {
      const c = q[qi] as number;
      const x = c % W;
      for (const n of [x > 0 ? c - 1 : -1, x < W - 1 ? c + 1 : -1, c - W, c + W]) {
        if (n < 0 || n >= N || seen[n] || !street(n)) continue;
        seen[n] = 1;
        q.push(n);
      }
    }
    for (const s of segs) {
      let any = false;
      let reached = false;
      segCells(s, W, H, (c) => {
        const t = map.terrain[c];
        if (t === Terrain.Road || t === Terrain.Pavement || t === Terrain.Dirt) {
          any = true;
          if (seen[c]) reached = true;
        }
      });
      if (any && !reached) out.push(`${name(s)} does not join the town's streets`);
    }
  }
  // ---- the railway
  if (plan.rail.length) {
    if (plan.rail.length > 3) out.push(`the railway has ${plan.rail.length} runs (at most 3)`);
    let len = 0;
    for (const r of plan.rail) len += segSteps(r) + 1;
    const ends = railEnds(plan.rail);
    const onEdge = (p: { x: number; y: number }) => p.x <= 0 || p.y <= 0 || p.x >= W - 2 || p.y >= H - 2;
    if (!ends || !onEdge(ends[0]) || !onEdge(ends[1])) out.push("the railway does not run edge to edge");
    else {
      const md = Math.abs(ends[0].x - ends[1].x) + Math.abs(ends[0].y - ends[1].y);
      if (len > 1.15 * md + 12) out.push(`the railway loops (${len} tiles between ends ${md} apart)`);
    }
    let near = Infinity;
    for (const r of plan.rail) segCells(r, W, H, (c) => (near = Math.min(near, rectDistance(plan.square, c % W, Math.floor(c / W)))));
    if (near < 6) out.push(`the railway passes ${near} tiles from the square`);
    const st = map.buildings.find((b) => b.kind === "station");
    if (!st) out.push("the railway has no station");
    else {
      let d = Infinity;
      for (const r of plan.rail) segCells(r, W, H, (c) => (d = Math.min(d, rectDistance(st, c % W, Math.floor(c / W)))));
      if (d > 3) out.push(`the station is ${d} tiles from the railway`);
    }
  }
  // ---- arterials: straight runs and clean 45° runs, no staircase
  for (const a of plan.arterials) {
    if (a.name === "Ring Road") continue;
    const legs = segs.filter((s) => s.arterial === a.index);
    for (let k = 1; k < legs.length - 1; k++) {
      const l = legs[k] as Segment;
      if (segSteps(l) + 1 < 8) out.push(`${a.name} zigzags (a run of ${segSteps(l) + 1} tiles between turns)`);
    }
  }
  // ---- the map's edge
  let tracks = 0;
  const border: number[] = [];
  for (let x = 0; x < W; x++) border.push(x, (H - 1) * W + x);
  for (let y = 1; y < H - 1; y++) border.push(y * W, y * W + W - 1);
  const linkCell = new Uint8Array(N);
  for (const s of segs) if (link(s)) segCells(s, W, H, (c) => (linkCell[c] = 1));
  for (const c of border) {
    const t = map.terrain[c] as number;
    if (t === Terrain.Dirt || t === Terrain.Path || t === Terrain.Ford) tracks++;
    else if (STREET.has(t) && !linkCell[c]) out.push(`a street reaches the map's edge at ${c % W},${Math.floor(c / W)}`);
  }
  if (tracks > 4) out.push(`${tracks} track cells on the map's edge (at most two tracks)`);
  return out;
}

/** The two ends of the railway (its runs share their bend cells). */
function railEnds(rail: Segment[]): [{ x: number; y: number }, { x: number; y: number }] | null {
  const pts: { x: number; y: number }[] = [];
  for (const r of rail) pts.push({ x: r.ax, y: r.ay }, { x: r.bx, y: r.by });
  // An end is a point no other run comes near.
  const ends = pts.filter((p, i) => !pts.some((q, j) => Math.floor(j / 2) !== Math.floor(i / 2) && Math.abs(p.x - q.x) + Math.abs(p.y - q.y) <= 3));
  return ends.length === 2 ? [ends[0] as { x: number; y: number }, ends[1] as { x: number; y: number }] : null;
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
  // Blocks: land between the streets (their runs and turning heads, the square, the quays, water and the railway;
  // forecourts, lanes and paths inside a block are part of it); those with standing buildings must be at least 6 × 5.
  const street = new Uint8Array(N);
  for (const s of plan.segments) {
    segCells(s, W, map.height, (c) => (street[c] = 1));
    for (const end of [0, 1] as const) if (isHead(s.ends[end])) for (const c of headCells(s, end, W, map.height)) street[c] = 1;
  }
  const isStreetAt = (c: number) => {
    const t = map.terrain[c] as number;
    return street[c] === 1 || t === Terrain.Plaza || t === Terrain.Quay || t === Terrain.Rail || t === Terrain.Crossing;
  };
  const seen = new Uint8Array(N);
  let smallBlocks = 0;
  SMALL_BLOCKS.length = 0;
  const hasBuilding = new Uint8Array(N);
  for (const b of map.buildings) if (b.state !== "reserved") for (let y = b.y; y < b.y + b.h; y++) for (let x = b.x; x < b.x + b.w; x++) hasBuilding[y * W + x] = 1;
  for (let s = 0; s < N; s++) {
    if (seen[s] || isStreetAt(s) || map.terrain[s] === Terrain.Water || !hasBuilding[s]) continue;
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
        if (n < 0 || n >= N || seen[n] || isStreetAt(n) || map.terrain[n] === Terrain.Water) continue;
        seen[n] = 1;
        q.push(n);
      }
    }
    // The square's civic row stands on the square's paving: not a block.
    const onSquare = x0 >= plan.civic.x && x1 < plan.civic.x + plan.civic.w && y0 >= plan.civic.y - 1 && y1 < plan.civic.y + plan.civic.h;
    if (!onSquare && (x1 - x0 + 1 < 6 || y1 - y0 + 1 < 5)) {
      smallBlocks++;
      SMALL_BLOCKS.push({ x0, y0, x1, y1 });
    }
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
    shortStreets: plan.segments.filter((s) => segSteps(s) + 1 < 3).length,
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
  for (const s of plan.segments) if (s.cls === RoadClass.Arterial || (s.district < 0 && s.arterial < 0)) segCells(s, W, map.height, (c) => (main[c] = 1));
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

/** The last readability pass's small built blocks (bounds), for diagnostics. */
export const SMALL_BLOCKS: { x0: number; y0: number; x1: number; y1: number }[] = [];
