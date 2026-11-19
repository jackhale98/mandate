// Layout v2 stage 4 (TOWN_GENERATION.md §5): arterials toward the real
// neighbouring towns, routed by A* on a coarse 4-tile grid with costs for
// slope, water and rock (a road that exists already is cheaper, so roads
// share bridges), straight bridges where they cross water, a ring road in
// cities, and the railway with level crossings and a viaduct. Also the
// street painter every later stage uses, and the level edges (walls, steps).
import { Terrain } from "../townLayout";
import { BAND_CAPS, type GeoLink } from "./geo";
import { MinHeap, NO_BEARING, bearingGap, bearingOf, compassVec } from "./imath";
import { edgePoint } from "./terrain";
import { RoadClass, Use, WaterKind, isWaterBody, rngOf, type Arterial, type PlanConnector, type Segment, type Work } from "./work";

const G = 4;

// ---------------------------------------------------------------------------
// Painting streets

/** Terrain of each row (or column) across a street, by width and style. */
function profile(width: number, style: Segment["style"]): number[] {
  if (style === "path") return Array.from({ length: width }, () => Terrain.Path);
  if (style === "dirt") return width >= 4 ? [Terrain.Grass, Terrain.Dirt, Terrain.Dirt, Terrain.Grass] : Array.from({ length: width }, () => Terrain.Dirt);
  if (width >= 4) return [Terrain.Pavement, Terrain.Road, Terrain.Road, Terrain.Pavement];
  if (width === 3) return [Terrain.Pavement, Terrain.Road, Terrain.Road];
  if (width === 2) return [Terrain.Road, Terrain.Road];
  return [Terrain.Path];
}

const RANK: Record<number, number> = { [Terrain.Grass]: 0, [Terrain.Path]: 1, [Terrain.Dirt]: 2, [Terrain.Pavement]: 3, [Terrain.Road]: 4 };

/**
 * Paint a street. Cells on a river, lake or the sea are skipped (bridges are
 * painted by `bridgeRuns`); a creek becomes a bridge deck (or a ford for
 * lanes); a road never turns back into pavement. The street's cells are put
 * on one level across its width (streets are cut into the slope).
 */
export function paintSegment(w: Work, s: Segment) {
  const { W } = w;
  const width = s.axis === "h" ? s.y1 - s.y0 + 1 : s.x1 - s.x0 + 1;
  const prof = profile(width, s.style);
  const lanes = s.style === "path" || s.style === "dirt";
  const along0 = s.axis === "h" ? s.x0 : s.y0;
  const along1 = s.axis === "h" ? s.x1 : s.y1;
  for (let a = along0; a <= along1; a++) {
    // The level of the street's middle line here.
    const mid = s.axis === "h" ? (s.y0 + (width >> 1)) * W + a : a * W + s.x0 + (width >> 1);
    const lv = w.level[mid] as number;
    for (let k = 0; k < width; k++) {
      const x = s.axis === "h" ? a : s.x0 + k;
      const y = s.axis === "h" ? s.y0 + k : a;
      if (x < 0 || y < 0 || x >= w.W || y >= w.H) continue;
      const c = y * W + x;
      if (isWaterBody(w, c) || (w.use[c] as number) & Use.Square) continue;
      const t = prof[k] as number;
      const cur = w.terrain[c] as number;
      // Streets stop at the railway; the arterials painted before it get its level crossings.
      if ((w.use[c] as number) & Use.Rail) continue;
      if (w.water[c] === WaterKind.Creek) {
        w.terrain[c] = lanes ? Terrain.Ford : Terrain.Bridge;
        w.use[c] = (w.use[c] as number) | Use.Road | Use.Bridge;
        if (!w.roadClass[c] || s.cls < (w.roadClass[c] as number)) w.roadClass[c] = s.cls;
        continue;
      }
      if (cur === Terrain.Bridge || cur === Terrain.Crossing || cur === Terrain.Plaza || cur === Terrain.Ford) continue;
      const curRoad = (w.use[c] as number) & Use.Road;
      if (curRoad && (RANK[cur] ?? 0) >= (RANK[t] ?? 0)) {
        if (s.cls < (w.roadClass[c] as number)) w.roadClass[c] = s.cls;
        continue;
      }
      w.terrain[c] = t;
      w.level[c] = lv;
      if (t !== Terrain.Grass || !curRoad) w.use[c] = (w.use[c] as number) | Use.Road;
      if (!w.roadClass[c] || s.cls < (w.roadClass[c] as number)) w.roadClass[c] = s.cls;
    }
  }
}

/** Bridges where a street runs over a river or lake: the run of water across it, plus a tile of abutment each end. Returns the connectors made. */
export function bridgeRuns(w: Work, s: Segment, kindOf: (span: number) => { id: string; kind: PlanConnector["kind"] } | null, paint = true): PlanConnector[] {
  const { W } = w;
  const out: PlanConnector[] = [];
  const width = s.axis === "h" ? s.y1 - s.y0 + 1 : s.x1 - s.x0 + 1;
  const along0 = s.axis === "h" ? s.x0 : s.y0;
  const along1 = s.axis === "h" ? s.x1 : s.y1;
  const wetAt = (a: number) => {
    for (let k = 0; k < width; k++) {
      const x = s.axis === "h" ? a : s.x0 + k;
      const y = s.axis === "h" ? s.y0 + k : a;
      const c = y * W + x;
      if (isWaterBody(w, c)) return w.water[c] as number;
    }
    return 0;
  };
  let a = along0;
  while (a <= along1) {
    const wk = wetAt(a);
    if (!wk) {
      a++;
      continue;
    }
    let b = a;
    while (b + 1 <= along1 && wetAt(b + 1)) b++;
    const a0 = Math.max(along0, a - 1);
    const b0 = Math.min(along1, b + 1);
    const span = b - a + 1;
    const k = kindOf(span);
    if (k) {
      for (let p = a0; p <= (paint ? b0 : a0 - 1); p++) {
        for (let q = 0; q < width; q++) {
          const x = s.axis === "h" ? p : s.x0 + q;
          const y = s.axis === "h" ? s.y0 + q : p;
          const c = y * W + x;
          w.terrain[c] = Terrain.Bridge;
          w.use[c] = (w.use[c] as number) | Use.Road | Use.Bridge;
          if (!w.roadClass[c] || s.cls < (w.roadClass[c] as number)) w.roadClass[c] = s.cls;
        }
      }
      out.push({
        kind: k.kind,
        type: k.id,
        x: s.axis === "h" ? a0 : s.x0,
        y: s.axis === "h" ? s.y0 : a0,
        w: s.axis === "h" ? b0 - a0 + 1 : width,
        h: s.axis === "h" ? width : b0 - a0 + 1,
        axis: s.axis === "h" ? "ew" : "ns",
        name: "",
        crosses: wk === WaterKind.Sea ? "sea" : wk === WaterKind.Lake ? "lake" : "river",
        arterial: s.arterial,
      });
    }
    a = b + 1;
  }
  return out;
}

/** The connector type for a crossing (first in registry order that allows it). */
export function connectorFor(w: Work, road: string, crosses: string, span: number): { id: string; kind: PlanConnector["kind"] } | null {
  for (const c of w.recipe.connectors) {
    if (!c.roads.includes(road) || !c.crosses.includes(crosses) || span > c.max_span) continue;
    if (c.town_class.length && !c.town_class.includes(w.geo.class)) continue;
    w.used.connectors.add(c.id);
    return { id: c.id, kind: c.kind };
  }
  // Nothing long enough: the longest bridge that allows the road, so a district is never cut off.
  const any = w.recipe.connectors.filter((c) => c.roads.includes(road) && c.crosses.includes(crosses)).sort((a, b) => b.max_span - a.max_span)[0];
  if (any) w.used.connectors.add(any.id);
  return any ? { id: any.id, kind: any.kind } : null;
}

// ---------------------------------------------------------------------------
// The coarse grid and A*

interface Coarse {
  CW: number;
  CH: number;
  wet: Uint8Array;
  sea: Uint8Array;
  level: Uint8Array;
  rock: Uint8Array;
  road: Uint8Array;
  blocked: Uint8Array;
}

function coarseOf(w: Work): Coarse {
  const CW = w.W / G;
  const CH = w.H / G;
  const n = CW * CH;
  const cg: Coarse = { CW, CH, wet: new Uint8Array(n), sea: new Uint8Array(n), level: new Uint8Array(n), rock: new Uint8Array(n), road: new Uint8Array(n), blocked: new Uint8Array(n) };
  for (let j = 0; j < CH; j++) {
    for (let i = 0; i < CW; i++) {
      const k = j * CW + i;
      for (let y = j * G; y < j * G + G; y++) {
        for (let x = i * G; x < i * G + G; x++) {
          const c = y * w.W + x;
          const wk = w.water[c] as number;
          if (wk === WaterKind.River || wk === WaterKind.Lake) cg.wet[k] = 1;
          if (wk === WaterKind.Sea) cg.sea[k] = 1;
          if (w.terrain[c] === Terrain.Rock) cg.rock[k] = 1;
          if ((w.use[c] as number) & Use.Square) cg.blocked[k] = 1;
        }
      }
      cg.level[k] = w.level[(j * G + 1) * w.W + i * G + 1] as number;
    }
  }
  return cg;
}

interface Costs {
  base: number;
  slope: number;
  water: number;
  rock: number;
  road: number;
  turn: number;
}

/** A* over coarse cells from any of `starts` to `goal`, 4 directions, no turning on water (bridges stay straight). */
function coarsePath(cg: Coarse, starts: number[], goal: number, k: Costs): number[] | null {
  const { CW, CH } = cg;
  const n = CW * CH;
  const g = new Float64Array(n * 5).fill(Infinity);
  const from = new Int32Array(n * 5).fill(-1);
  const heap = new MinHeap(1024);
  const gx = goal % CW;
  const gy = Math.floor(goal / CW);
  const h = (c: number) => (Math.abs((c % CW) - gx) + Math.abs(Math.floor(c / CW) - gy)) * Math.max(1, k.base - k.road);
  for (const s of starts) {
    const st = s * 5 + 4;
    g[st] = 0;
    heap.push(h(s), st);
  }
  const DX = [1, -1, 0, 0];
  const DY = [0, 0, 1, -1];
  while (heap.size) {
    const st = heap.pop();
    const c = Math.floor(st / 5);
    const d = st % 5;
    const gc = g[st] as number;
    if (heap.lastKey - h(c) > gc + 1e-9) continue;
    if (c === goal) {
      const path: number[] = [];
      for (let s = st; s >= 0; s = from[s] as number) path.push(Math.floor(s / 5));
      return path.reverse();
    }
    const x = c % CW;
    const y = Math.floor(c / CW);
    for (let nd = 0; nd < 4; nd++) {
      if (cg.wet[c] && d !== 4 && nd !== d) continue;
      const nx = x + (DX[nd] as number);
      const ny = y + (DY[nd] as number);
      if (nx < 0 || ny < 0 || nx >= CW || ny >= CH) continue;
      const nc = ny * CW + nx;
      if (cg.sea[nc] || cg.blocked[nc]) continue;
      let cost = k.base + k.slope * Math.abs((cg.level[nc] as number) - (cg.level[c] as number));
      if (cg.wet[nc] && !cg.road[nc]) cost += k.water;
      if (cg.rock[nc]) cost += k.rock;
      if (cg.road[nc]) cost -= k.road;
      if (d !== 4 && nd !== d) cost += k.turn;
      const ns = nc * 5 + nd;
      const ng = gc + cost;
      if (ng < (g[ns] as number)) {
        g[ns] = ng;
        from[ns] = st;
        heap.push(ng + h(nc), ns);
      }
    }
  }
  return null;
}

/** Straight runs of a coarse path: [first index, last index, axis] (consecutive runs share their turn cell). */
function runsOf(cg: Coarse, path: number[]): { i0: number; i1: number; axis: "h" | "v" }[] {
  if (path.length === 1) return [{ i0: 0, i1: 0, axis: "h" }];
  const axisOf = (a: number, b: number): "h" | "v" => (Math.floor(a / cg.CW) === Math.floor(b / cg.CW) ? "h" : "v");
  const out: { i0: number; i1: number; axis: "h" | "v" }[] = [];
  let s = 0;
  let ax = axisOf(path[0] as number, path[1] as number);
  for (let i = 1; i < path.length - 1; i++) {
    const nx = axisOf(path[i] as number, path[i + 1] as number);
    if (nx !== ax) {
      out.push({ i0: s, i1: i, axis: ax });
      s = i;
      ax = nx;
    }
  }
  out.push({ i0: s, i1: path.length - 1, axis: ax });
  return out;
}

/** Tile segments of a coarse path, `width` wide, at offset `off` inside each 4-tile cell; turns meet at a cell's middle when `trim`. */
function segmentsOf(cg: Coarse, path: number[], width: number, off: number, trim: boolean, base: Omit<Segment, "x0" | "y0" | "x1" | "y1" | "axis">): Segment[] {
  const segs: Segment[] = [];
  const runs = runsOf(cg, path);
  runs.forEach((r) => {
    const a = path[r.i0] as number;
    const b = path[r.i1] as number;
    const ax = a % cg.CW;
    const ay = Math.floor(a / cg.CW);
    const bx = b % cg.CW;
    const by = Math.floor(b / cg.CW);
    const turnA = r.i0 > 0;
    const turnB = r.i1 < path.length - 1;
    if (r.axis === "h") {
      const lo = Math.min(ax, bx);
      const hi = Math.max(ax, bx);
      const loTurn = ax <= bx ? turnA : turnB;
      const hiTurn = ax <= bx ? turnB : turnA;
      segs.push({ ...base, axis: "h", x0: lo * G + (trim && loTurn ? off : 0), x1: hi * G + (trim && hiTurn ? off + width - 1 : G - 1), y0: ay * G + off, y1: ay * G + off + width - 1 });
    } else {
      const lo = Math.min(ay, by);
      const hi = Math.max(ay, by);
      const loTurn = ay <= by ? turnA : turnB;
      const hiTurn = ay <= by ? turnB : turnA;
      segs.push({ ...base, axis: "v", y0: lo * G + (trim && loTurn ? off : 0), y1: hi * G + (trim && hiTurn ? off + width - 1 : G - 1), x0: ax * G + off, x1: ax * G + off + width - 1 });
    }
  });
  return segs;
}

/** The coarse cell where a bearing leaves the map, moved along the border off the sea. */
function edgeCell(w: Work, cg: Coarse, b: number): number | null {
  const e = edgePoint(w.W, w.H, b, w.centre.x, w.centre.y);
  const i = Math.min(cg.CW - 1, Math.floor(e.x / G));
  const j = Math.min(cg.CH - 1, Math.floor(e.y / G));
  const border: number[] = [];
  for (let x = 0; x < cg.CW; x++) border.push(x, (cg.CH - 1) * cg.CW + x);
  for (let y = 1; y < cg.CH - 1; y++) border.push(y * cg.CW, y * cg.CW + cg.CW - 1);
  let best: number | null = null;
  let bestD = Infinity;
  for (const c of border) {
    if (cg.sea[c] || cg.wet[c]) continue;
    const d = Math.abs((c % cg.CW) - i) + Math.abs(Math.floor(c / cg.CW) - j);
    if (d < bestD || (d === bestD && best !== null && c < best)) {
      bestD = d;
      best = c;
    }
  }
  // Too far round the coast: that way is the sea.
  return best !== null && bestD <= Math.max(3, Math.floor((cg.CW + cg.CH) / 8)) ? best : null;
}

const COMPASS_NAMES = ["North", "North", "North-East", "East", "East", "East", "South-East", "South", "South", "South", "South-West", "West", "West", "West", "North-West", "North"];

// ---------------------------------------------------------------------------
// Stage 4

export function stageArterials(w: Work) {
  const geo = w.geo;
  const caps = BAND_CAPS[w.band];
  const cg = coarseOf(w);
  // The square's ring of streets, on the coarse lattice.
  const rx0 = w.ring.x / G;
  const ry0 = w.ring.y / G;
  const rx1 = (w.ring.x + w.ring.w) / G - 1;
  const ry1 = (w.ring.y + w.ring.h) / G - 1;
  const ringCells: number[] = [];
  for (let j = ry0; j <= ry1; j++) for (let i = rx0; i <= rx1; i++) if (i === rx0 || i === rx1 || j === ry0 || j === ry1) ringCells.push(j * cg.CW + i);
  const ringBase = { cls: RoadClass.Street, style: "asphalt" as const, district: -1, arterial: -1 };
  const ringSegs: Segment[] = [
    { ...ringBase, axis: "h", x0: w.ring.x, x1: w.ring.x + w.ring.w - 1, y0: w.ring.y, y1: w.ring.y + 3 },
    { ...ringBase, axis: "h", x0: w.ring.x, x1: w.ring.x + w.ring.w - 1, y0: w.ring.y + w.ring.h - 4, y1: w.ring.y + w.ring.h - 1 },
    { ...ringBase, axis: "v", x0: w.ring.x, x1: w.ring.x + 3, y0: w.ring.y, y1: w.ring.y + w.ring.h - 1 },
    { ...ringBase, axis: "v", x0: w.ring.x + w.ring.w - 4, x1: w.ring.x + w.ring.w - 1, y0: w.ring.y, y1: w.ring.y + w.ring.h - 1 },
  ];
  for (const c of ringCells) cg.road[c] = 1;

  // ---- which arterials: the town's road links, deduplicated by bearing, within the band's caps
  const roads = geo.links.filter((l) => l.mode !== "rail");
  const picked: { bearing: number; link: GeoLink | null }[] = [];
  const minGap = (b: number) => Math.min(9, ...picked.map((p) => bearingGap(p.bearing, b)));
  const byPriority = [...roads].sort((a, b) => (a.mode === "highway" ? 0 : 1) - (b.mode === "highway" ? 0 : 1) || a.bearing - b.bearing || a.to - b.to);
  for (const l of byPriority) {
    if (picked.length >= caps.arterials[1]) break;
    if (minGap(l.bearing) < 2) continue;
    if (edgeCell(w, cg, l.bearing) === null) continue;
    picked.push({ bearing: l.bearing, link: l });
  }
  // Fewer than the band's minimum: add roads where the gaps are widest (across the river first).
  const r = rngOf(w, "arterials");
  while (picked.length < caps.arterials[0]) {
    let best = -1;
    let bestS = -Infinity;
    for (let b = 0; b < 16; b++) {
      if (edgeCell(w, cg, b) === null) continue;
      const s = minGap(b) * 10 + r.int(0, 3);
      if (s > bestS) {
        bestS = s;
        best = b;
      }
    }
    if (best < 0 || minGap(best) < 2) break;
    picked.push({ bearing: best, link: null });
  }
  // A river town gets a road over the river: an arterial toward the river from the centre.
  if (geo.river) {
    let best = Infinity;
    let rb = NO_BEARING;
    for (let c = 0; c < w.W * w.H; c++) {
      if (w.water[c] !== WaterKind.River) continue;
      const dx = (c % w.W) - w.centre.x;
      const dy = Math.floor(c / w.W) - w.centre.y;
      if (dx * dx + dy * dy < best) {
        best = dx * dx + dy * dy;
        rb = bearingOf(dx, dy);
      }
    }
    if (rb !== NO_BEARING && edgeCell(w, cg, rb) !== null && !picked.some((p) => bearingGap(p.bearing, rb) <= 2)) {
      if (picked.length >= caps.arterials[1]) {
        let k = 0;
        for (let i = 1; i < picked.length; i++) if (bearingGap((picked[i] as { bearing: number }).bearing, rb) < bearingGap((picked[k] as { bearing: number }).bearing, rb)) k = i;
        picked.splice(k, 1);
      }
      picked.push({ bearing: rb, link: roads.find((l) => bearingGap(l.bearing, rb) <= 2) ?? null });
    }
  }
  picked.sort((a, b) => a.bearing - b.bearing);

  const kinds = (span: number) => connectorFor(w, "arterial", "river", span);
  let bridges = 0;
  const names = new Set<string>();
  for (const p of picked) {
    const goal = edgeCell(w, cg, p.bearing);
    if (goal === null) continue;
    // Once the band's bridges are built, a new crossing costs far more: roads share them.
    const water = bridges >= caps.bridges ? 900 : 70 + bridges * 60;
    const path = coarsePath(cg, ringCells, goal, { base: 10, slope: 30, water, rock: 60, road: 6, turn: 6 });
    if (!path) continue;
    const toName = p.link?.to_name ?? "";
    let name = toName ? `${toName} Road` : `${COMPASS_NAMES[p.bearing] ?? "North"} Road`;
    if (names.has(name)) name = `Old ${name}`;
    names.add(name);
    const art: Arterial = { index: w.arterials.length, name, mode: p.link ? p.link.mode : "local", bearing: p.bearing, toName, cells: [] };
    const segs = segmentsOf(cg, path, 4, 0, false, { cls: RoadClass.Arterial, style: "asphalt", district: -1, arterial: art.index });
    for (const s of segs) {
      const made = bridgeRuns(w, s, kinds);
      for (const m of made) {
        // A bridge another arterial already built here is shared, not counted twice.
        if (!w.connectors.some((o) => o.x === m.x && o.y === m.y && o.w === m.w && o.h === m.h)) {
          w.connectors.push(m);
          bridges++;
        }
      }
      paintSegment(w, s);
      w.segments.push(s);
    }
    for (const c of path) cg.road[c] = 1;
    for (const s of segs) for (let y = s.y0; y <= s.y1; y++) for (let x = s.x0; x <= s.x1; x++) art.cells.push(y * w.W + x);
    w.arterials.push(art);
  }
  for (const s of ringSegs) {
    paintSegment(w, s);
    w.segments.push(s);
  }

  // ---- a ring road in cities: a coarse rectangle, broken where it would need a bridge
  if (caps.ring) {
    const i0 = Math.floor(cg.CW * 0.18);
    const i1 = Math.ceil(cg.CW * 0.82) - 1;
    const j0 = Math.floor(cg.CH * 0.18);
    const j1 = Math.ceil(cg.CH * 0.82) - 1;
    const art: Arterial = { index: w.arterials.length, name: "Ring Road", mode: "local", bearing: NO_BEARING, toName: "", cells: [] };
    const sides: number[][] = [[], [], [], []];
    for (let i = i0; i <= i1; i++) {
      (sides[0] as number[]).push(j0 * cg.CW + i);
      (sides[1] as number[]).push(j1 * cg.CW + i);
    }
    for (let j = j0; j <= j1; j++) {
      (sides[2] as number[]).push(j * cg.CW + i0);
      (sides[3] as number[]).push(j * cg.CW + i1);
    }
    for (const side of sides) {
      let run: number[] = [];
      const flush = () => {
        if (run.length >= 2) {
          for (const s of segmentsOf(cg, run, 4, 0, false, { cls: RoadClass.Arterial, style: "asphalt", district: -1, arterial: art.index })) {
            paintSegment(w, s);
            w.segments.push(s);
            for (let y = s.y0; y <= s.y1; y++) for (let x = s.x0; x <= s.x1; x++) art.cells.push(y * w.W + x);
          }
        }
        run = [];
      };
      for (const c of side) {
        if (cg.wet[c] || cg.sea[c] || cg.blocked[c] || cg.rock[c]) flush();
        else run.push(c);
      }
      flush();
    }
    if (art.cells.length) w.arterials.push(art);
  }
}

/** The railway (stage 4): edge to edge past the core, gentle grades, level crossings over streets, a viaduct over water. */
export function stageRail(w: Work) {
  const rails = w.geo.links.filter((l) => l.mode === "rail");
  if (!rails.length) return;
  const cg = coarseOf(w);
  for (let c = 0; c < w.W * w.H; c++) if ((w.use[c] as number) & Use.Road) cg.road[Math.floor((c % w.W) / G) + Math.floor(Math.floor(c / w.W) / G) * cg.CW] = 1;
  const b1 = (rails[0] as GeoLink).bearing;
  const b2 = rails.length > 1 ? (rails[rails.length - 1] as GeoLink).bearing : (b1 + 8) & 15;
  const a = edgeCell(w, cg, b1);
  const z = edgeCell(w, cg, bearingGap(b1, b2) < 4 ? (b1 + 8) & 15 : b2);
  if (a === null || z === null || a === z) return;
  // A station beside the core: two coarse cells out from the square's ring, across the line's direction.
  const v = compassVec(b1);
  const horizontal = Math.abs(v.dx) >= Math.abs(v.dy);
  const rx0 = w.ring.x / G - 2;
  const ry0 = w.ring.y / G - 2;
  const rx1 = (w.ring.x + w.ring.w) / G + 1;
  const ry1 = (w.ring.y + w.ring.h) / G + 1;
  const mx = Math.floor((rx0 + rx1) / 2);
  const my = Math.floor((ry0 + ry1) / 2);
  const opts = horizontal
    ? [
        [mx, ry0],
        [mx, ry1],
      ]
    : [
        [rx0, my],
        [rx1, my],
      ];
  let via: number | null = null;
  for (const [i, j] of opts as [number, number][]) {
    if (i < 1 || j < 1 || i >= cg.CW - 1 || j >= cg.CH - 1) continue;
    const c = j * cg.CW + i;
    if (cg.wet[c] || cg.sea[c] || cg.blocked[c]) continue;
    via = c;
    break;
  }
  const costs = { base: 10, slope: 80, water: 40, rock: 80, road: -25, turn: 14 };
  const p1 = via !== null ? coarsePath(cg, [a], via, costs) : null;
  const p2 = via !== null && p1 ? coarsePath(cg, [via], z, costs) : null;
  const path = p1 && p2 ? [...p1, ...p2.slice(1)] : coarsePath(cg, [a], z, costs);
  if (!path) return;
  const segs = segmentsOf(cg, path, 2, 1, true, { cls: RoadClass.None, style: "path", district: -1, arterial: -1 });
  const { W } = w;
  const crossings = new Map<string, { x0: number; y0: number; x1: number; y1: number; axis: "ew" | "ns" }>();
  for (const s of segs) {
    for (let y = s.y0; y <= s.y1; y++) {
      for (let x = s.x0; x <= s.x1; x++) {
        const c = y * W + x;
        w.railCells.push(c);
        if ((w.use[c] as number) & Use.Road && !((w.use[c] as number) & Use.Bridge)) {
          // A level crossing where the line meets a street.
          w.terrain[c] = Terrain.Crossing;
          const key = `${s.axis}:${s.axis === "h" ? Math.floor(x / G) : Math.floor(y / G)}`;
          const cur = crossings.get(key);
          if (cur) {
            cur.x0 = Math.min(cur.x0, x);
            cur.y0 = Math.min(cur.y0, y);
            cur.x1 = Math.max(cur.x1, x);
            cur.y1 = Math.max(cur.y1, y);
          } else crossings.set(key, { x0: x, y0: y, x1: x, y1: y, axis: s.axis === "h" ? "ew" : "ns" });
        } else if ((w.use[c] as number) & Use.Bridge) {
          // Never over a road bridge's deck.
        } else {
          w.terrain[c] = Terrain.Rail;
          w.level[c] = w.level[(s.axis === "h" ? s.y0 : y) * W + (s.axis === "h" ? x : s.x0)] as number;
        }
        w.use[c] = (w.use[c] as number) | Use.Rail;
      }
    }
    // A viaduct over the river.
    const made = bridgeRuns(w, s, (span) => connectorFor(w, "rail", "river", span), false);
    for (const m of made) w.connectors.push({ ...m, kind: "viaduct", arterial: -1 });
  }
  const lc = connectorFor(w, "street", "rail", 1);
  for (const k of [...crossings.keys()].sort()) {
    const cr = crossings.get(k) as { x0: number; y0: number; x1: number; y1: number; axis: "ew" | "ns" };
    w.connectors.push({ kind: "level_crossing", type: lc?.id ?? "connector.level_crossing", x: cr.x0, y: cr.y0, w: cr.x1 - cr.x0 + 1, h: cr.y1 - cr.y0 + 1, axis: cr.axis === "ew" ? "ns" : "ew", name: "", crosses: "rail", arterial: -1 });
  }
}

/** Give bridges names (stable: in order of position), shared by arterials that use them. */
export function nameBridges(w: Work) {
  const r = rngOf(w, "bridge-names");
  const stems = ["Old", "New", "Town", "Mill", "Iron", "Stone", "Market", "Victoria", "Queen's", "King's", "Abbey", "Ferry"];
  const taken = new Set<string>();
  const bridges = w.connectors.filter((c) => c.kind === "bridge" && c.crosses !== "creek").sort((a, b) => a.y - b.y || a.x - b.x);
  bridges.forEach((b, i) => {
    let stem = i === 0 ? "Old" : (r.pick(stems) as string);
    const toName = w.arterials[b.arterial]?.toName;
    if (i > 0 && toName && r.chance(0.5)) stem = toName;
    let name = `${stem} Bridge`;
    for (let k = 0; taken.has(name) && k < stems.length; k++) name = `${stems[(i + k) % stems.length]} Bridge`;
    taken.add(name);
    b.name = name;
  });
  for (const c of w.connectors) if (c.kind === "viaduct") c.name = c.name || "the Viaduct";
}

/**
 * Level edges (§7.7): a cell above a lower one to its south is a bank or
 * retaining wall (a rock face where it drops two levels), drawn like a south
 * face; a path crossing it becomes steps and a road a ramp. Other drops of
 * two levels are rock. North-facing changes are lips (drawn, not blocking).
 */
export function stageLevelEdges(w: Work) {
  const { W, H } = w;
  for (let y = 0; y < H - 1; y++) {
    for (let x = 0; x < W; x++) {
      const c = y * W + x;
      if (isWaterBody(w, c) || w.water[c] === WaterKind.Creek) continue;
      const lv = w.level[c] as number;
      const below = w.level[c + W] as number;
      const t = w.terrain[c] as number;
      let drop = lv - below;
      if (drop <= 0) {
        // Steep sides east, west or north: rock.
        const e = x < W - 1 ? (w.level[c + 1] as number) : lv;
        const wv = x > 0 ? (w.level[c - 1] as number) : lv;
        const n = y > 0 ? (w.level[c - W] as number) : lv;
        drop = Math.max(lv - e, lv - wv, lv - n) >= 2 ? 2 : 0;
        if (drop < 2 || (w.use[c] as number) & (Use.Road | Use.Square | Use.Rail)) continue;
        w.terrain[c] = Terrain.Rock;
        continue;
      }
      if (isWaterBody(w, c + W)) continue;
      if (t === Terrain.Road || t === Terrain.Crossing || t === Terrain.Bridge || t === Terrain.Rail || (w.use[c] as number) & Use.Square) continue;
      if (t === Terrain.Pavement || t === Terrain.Path || t === Terrain.Dirt || t === Terrain.Quay) {
        // A street's south kerb above lower ground is its retaining wall; elsewhere a path steps down.
        const roadAbove = y > 0 && (w.terrain[c - W] === Terrain.Road || w.terrain[c - W] === Terrain.Dirt);
        const roadSide = (x > 0 && w.terrain[c - 1] === Terrain.Road) || (x < W - 1 && w.terrain[c + 1] === Terrain.Road);
        w.terrain[c] = roadAbove && !roadSide && t === Terrain.Pavement ? Terrain.Embankment : Terrain.Steps;
        continue;
      }
      w.terrain[c] = drop >= 2 ? Terrain.Rock : Terrain.Embankment;
    }
  }
}

/**
 * Terraces (§7.7 contour cues, §8 terrain restraint): each block of a
 * district stands on one level (its most common), and every street takes the
 * level of the land to its north, so level changes run along streets (as
 * their retaining walls) and up them (as ramps), never through a block.
 */
export function stageTerraces(w: Work) {
  const { W, H } = w;
  const N = W * H;
  const seen = new Uint8Array(N);
  const q = new Int32Array(N);
  const isLand = (c: number) => (w.district[c] as number) >= 0 && !((w.use[c] as number) & (Use.Road | Use.Rail | Use.Square)) && !isWaterBody(w, c) && w.water[c] !== WaterKind.Creek;
  for (let s = 0; s < N; s++) {
    if (seen[s] || !isLand(s)) continue;
    const d = w.district[s] as number;
    let qt = 0;
    q[qt++] = s;
    seen[s] = 1;
    const counts = [0, 0, 0, 0];
    for (let qh = 0; qh < qt; qh++) {
      const c = q[qh] as number;
      counts[w.level[c] as number] = (counts[w.level[c] as number] as number) + 1;
      const x = c % W;
      for (let k4 = 0; k4 < 4; k4++) {
        const n = k4 === 0 ? (x > 0 ? c - 1 : -1) : k4 === 1 ? (x < W - 1 ? c + 1 : -1) : k4 === 2 ? c - W : c + W;
        if (n < 0 || n >= N || seen[n] || w.district[n] !== d || !isLand(n)) continue;
        seen[n] = 1;
        q[qt++] = n;
      }
    }
    // Large open land (a park, the fringe) keeps its slopes; blocks are levelled.
    if (qt > 900) continue;
    let lv = 0;
    for (let k = 1; k < 4; k++) if ((counts[k] as number) > (counts[lv] as number)) lv = k;
    for (let i = 0; i < qt; i++) w.level[q[i] as number] = lv;
  }
  // Streets take the level of the land north of them (a street on a slope is cut into it).
  for (let y = 1; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const c = y * W + x;
      if (!((w.use[c] as number) & Use.Road) || (w.use[c] as number) & (Use.Bridge | Use.Square)) continue;
      let k = c - W;
      let steps = 0;
      while (k >= 0 && (w.use[k] as number) & Use.Road && steps < 6) {
        k -= W;
        steps++;
      }
      if (k >= 0 && isLand(k)) w.level[c] = w.level[k] as number;
      else if (k >= 0 && steps < 6) w.level[c] = w.level[c - W] as number;
    }
  }
}
