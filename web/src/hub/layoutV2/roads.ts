// Layout v2 stage 4 (TOWN_GENERATION.md §5; DEC-090 D5 amended): the
// railway, then the arterials. The railway passes the town on a plausible
// line (straight, or with one or two bends) a short walk from the square,
// where the station goes, crossing water on a straight viaduct. The
// arterials run from the square's ring toward the real neighbouring towns
// by A* on a coarse 4-tile grid in eight directions with turn costs, so each
// is a few straight or clean 45° runs (never a staircase); a road that
// exists already is cheaper (roads share bridges) and running alongside one
// costs more (no near-parallel duplicates); they cross the railway square-on
// at level crossings and water on straight bridges. Only the town's real
// links reach the map edge; a road over the river with no link beyond it
// ends in the town. Cities get a ring road, kept between its junctions.
// Also the bridge namer, the terraces and the level edges.
import { Terrain } from "../townLayout";
import { BAND_CAPS, type GeoLink } from "./geo";
import { MinHeap, NO_BEARING, bearingGap, bearingOf, compassVec } from "./imath";
import { boxSeg, crowded, isDiagDir, levelCrossing, makeSeg, paintSegment, segCells } from "./net";
import { edgePoint } from "./terrain";
import { RoadClass, Use, WaterKind, isWaterBody, rngOf, type Arterial, type PlanConnector, type Segment, type Work } from "./work";

const G = 4;

/** Bridges where a street runs over a river or lake: the run of water across it, plus a tile of abutment each end. Returns the connectors made. */
export function bridgeRuns(w: Work, s: Segment, kindOf: (span: number) => { id: string; kind: PlanConnector["kind"] } | null, paint = true): PlanConnector[] {
  const { W } = w;
  const out: PlanConnector[] = [];
  if (s.axis !== "h" && s.axis !== "v") return out;
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
          const x: number = s.axis === "h" ? p : s.x0 + q;
          const y: number = s.axis === "h" ? s.y0 + q : p;
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
// The coarse grid

interface Coarse {
  CW: number;
  CH: number;
  wet: Uint8Array;
  sea: Uint8Array;
  level: Uint8Array;
  rock: Uint8Array;
  road: Uint8Array;
  /** Cells next to a road (running alongside one costs more). */
  near: Uint8Array;
  /** Water too near a bridge for another (§8: bridges at least 40 tiles apart outside cities). */
  nearBridge: Uint8Array;
  blocked: Uint8Array;
  /** The railway: 1 an east–west run, 2 a north–south run, 3 a bend. */
  rail: Uint8Array;
}

function coarseOf(w: Work): Coarse {
  const CW = w.W / G;
  const CH = w.H / G;
  const n = CW * CH;
  const cg: Coarse = { CW, CH, wet: new Uint8Array(n), sea: new Uint8Array(n), level: new Uint8Array(n), rock: new Uint8Array(n), road: new Uint8Array(n), near: new Uint8Array(n), nearBridge: new Uint8Array(n), blocked: new Uint8Array(n), rail: new Uint8Array(n) };
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

function markNear(cg: Coarse, cells: Iterable<number>) {
  const { CW, CH } = cg;
  for (const c of cells) {
    const x = c % CW;
    const y = (c - x) / CW;
    for (let dy = -1; dy <= 1; dy++) {
      for (let dx = -1; dx <= 1; dx++) {
        const nx = x + dx;
        const ny = y + dy;
        if (nx >= 0 && ny >= 0 && nx < CW && ny < CH) cg.near[ny * CW + nx] = 1;
      }
    }
  }
}

const DX8 = [1, 1, 0, -1, -1, -1, 0, 1];
const DY8 = [0, -1, -1, -1, 0, 1, 1, 1];
const turnOf = (a: number, b: number) => {
  const d = Math.abs(a - b) & 7;
  return d > 4 ? 8 - d : d;
};

interface Costs8 {
  base: number;
  diag: number;
  slope: number;
  water: number;
  rock: number;
  road: number;
  t45: number;
  t90: number;
  near: number;
  /** A border cell other than the target (a road runs along the map's edge only where it leaves). */
  border: number;
}

/**
 * A* over coarse cells in eight directions; a state is (cell, the direction
 * it was entered in). Turns of 45° and 90° cost extra and sharper ones are
 * not allowed; water and the railway are crossed straight and square-on
 * (no diagonal steps onto them or past their corners).
 */
/** Search buffers, reused (each search resets what it uses). */
let scratch8: { g: Int32Array; from: Int32Array } | null = null;

function coarsePath8(cg: Coarse, starts: { c: number; d: number; g: number }[], isGoal: (c: number, d: number) => boolean, target: number, k: Costs8, exempt: (c: number) => boolean = (c) => c === target): { c: number; d: number }[] | null {
  const { CW, CH } = cg;
  const n = CW * CH;
  // A state is (cell, direction entered, whether that step turned): a turn needs a straight step before the next (no staircase).
  if (!scratch8 || scratch8.g.length < n * 16) scratch8 = { g: new Int32Array(n * 16), from: new Int32Array(n * 16) };
  const g = scratch8.g.fill(0x3fffffff, 0, n * 16);
  const from = scratch8.from.fill(-1, 0, n * 16);
  const heap = new MinHeap(1024);
  const gx = target % CW;
  const gy = Math.floor(target / CW);
  // A steady estimate (most steps cost about this much off the roads): fast, and the paths stay straight.
  const unit = Math.max(1, k.base - 2);
  const h = (c: number) => {
    const dx = Math.abs((c % CW) - gx);
    const dy = Math.abs(Math.floor(c / CW) - gy);
    return (Math.max(dx, dy) + Math.min(dx, dy) * 0.4) * unit;
  };
  for (const s of starts) {
    const st = (s.c * 8 + s.d) * 2;
    if (s.g < (g[st] as number)) {
      g[st] = s.g;
      heap.push(s.g + h(s.c), st);
    }
  }
  const bad = (c: number) => cg.sea[c] === 1 || cg.blocked[c] === 1;
  while (heap.size) {
    const st = heap.pop();
    const c = st >> 4;
    const d = (st >> 1) & 7;
    const turned = st & 1;
    const gc = g[st] as number;
    if (heap.lastKey - h(c) > gc + 1e-9) continue;
    if (isGoal(c, d)) {
      const path: { c: number; d: number }[] = [];
      for (let s = st; s >= 0; s = from[s] as number) path.push({ c: s >> 4, d: (s >> 1) & 7 });
      return path.reverse();
    }
    const x = c % CW;
    const y = (c - x) / CW;
    const locked = cg.wet[c] === 1 || cg.rail[c] !== 0;
    for (let nd = 0; nd < 8; nd++) {
      const turn = turnOf(d, nd);
      if (turn > 2 || (turn > 0 && turned)) continue;
      if (locked && nd !== d) continue;
      const nx = x + (DX8[nd] as number);
      const ny = y + (DY8[nd] as number);
      if (nx < 0 || ny < 0 || nx >= CW || ny >= CH) continue;
      const nc = ny * CW + nx;
      if (bad(nc)) continue;
      const diag = isDiagDir(nd);
      if (diag) {
        if (cg.wet[nc] || cg.rail[nc]) continue;
        const a = y * CW + nx;
        const b = ny * CW + x;
        if (bad(a) || bad(b) || cg.wet[a] || cg.wet[b] || cg.rail[a] || cg.rail[b]) continue;
      }
      const rl = cg.rail[nc] as number;
      if (rl === 3 || (rl === 1 && nd !== 2 && nd !== 6) || (rl === 2 && nd !== 0 && nd !== 4)) continue;
      let cost = (diag ? k.diag : k.base) + k.slope * Math.abs((cg.level[nc] as number) - (cg.level[c] as number));
      if (cg.wet[nc] && !cg.road[nc]) cost += k.water + (cg.nearBridge[nc] ? 900 : 0);
      if (cg.rock[nc]) cost += k.rock;
      if (cg.road[nc]) cost -= k.road;
      else if (cg.near[nc]) cost += k.near;
      if ((nx === 0 || ny === 0 || nx === CW - 1 || ny === CH - 1) && !exempt(nc)) cost += k.border;
      if (turn === 1) cost += k.t45;
      else if (turn === 2) cost += k.t90;
      const ns = (nc * 8 + nd) * 2 + (turn > 0 ? 1 : 0);
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

/** Straight runs of a coarse path: the first and last index of each, and its direction (runs share their turn cell). */
function runsOf8(path: { c: number; d: number }[]): { i0: number; i1: number; d: number }[] {
  const out: { i0: number; i1: number; d: number }[] = [];
  let s = 0;
  for (let i = 1; i < path.length; i++) {
    const d = (path[i] as { d: number }).d;
    if (i > 1 && d !== (path[i - 1] as { d: number }).d) {
      out.push({ i0: s, i1: i - 1, d: (path[i - 1] as { d: number }).d });
      s = i - 1;
    }
  }
  if (path.length > 1) out.push({ i0: s, i1: path.length - 1, d: (path[path.length - 1] as { d: number }).d });
  return out;
}

/** Tile runs of a road along a coarse path, `width` 4: orthogonal runs cover whole coarse cells, diagonals run between cell middles. */
function roadSegs(cg: Coarse, path: { c: number; d: number }[], base: Pick<Segment, "cls" | "style" | "district" | "arterial">): Segment[] {
  const segs: Segment[] = [];
  for (const r of runsOf8(path)) {
    const a = (path[r.i0] as { c: number }).c;
    const b = (path[r.i1] as { c: number }).c;
    const ai = a % cg.CW;
    const aj = Math.floor(a / cg.CW);
    const bi = b % cg.CW;
    const bj = Math.floor(b / cg.CW);
    if (isDiagDir(r.d)) {
      segs.push(makeSeg(ai * G + 1, aj * G + 1, r.d, Math.abs(bi - ai) * G, 5, base));
    } else if (aj === bj) {
      segs.push(boxSeg("h", Math.min(ai, bi) * G, aj * G, Math.max(ai, bi) * G + G - 1, aj * G + G - 1, base));
    } else {
      segs.push(boxSeg("v", ai * G, Math.min(aj, bj) * G, ai * G + G - 1, Math.max(aj, bj) * G + G - 1, base));
    }
  }
  return segs;
}

/** The coarse cell where a bearing leaves the map, moved along the border off the sea. */
function edgeCell(w: Work, cg: Coarse, b: number): number | null {
  const e = edgePoint(w.W, w.H, b, w.centre.x, w.centre.y);
  const i = Math.min(cg.CW - 1, Math.floor(e.x / G));
  const j = Math.min(cg.CH - 1, Math.floor(e.y / G));
  let best: number | null = null;
  let bestD = Infinity;
  const consider = (c: number) => {
    if (cg.sea[c] || cg.wet[c] || cg.rail[c]) return;
    let d = Math.abs((c % cg.CW) - i) + Math.abs(Math.floor(c / cg.CW) - j);
    // Not beside the railway (the road and the line would leave side by side).
    for (let k = -2; k <= 2; k++) {
      for (let l = -2; l <= 2; l++) {
        const x = (c % cg.CW) + k;
        const y = Math.floor(c / cg.CW) + l;
        if (x >= 0 && y >= 0 && x < cg.CW && y < cg.CH && cg.rail[y * cg.CW + x]) d += 2;
      }
    }
    if (d < bestD || (d === bestD && best !== null && c < best)) {
      bestD = d;
      best = c;
    }
  };
  for (let x = 1; x < cg.CW - 1; x++) {
    consider(x);
    consider((cg.CH - 1) * cg.CW + x);
  }
  for (let y = 1; y < cg.CH - 1; y++) {
    consider(y * cg.CW);
    consider(y * cg.CW + cg.CW - 1);
  }
  // Too far round the coast: that way is the sea.
  return best !== null && bestD <= Math.max(3, Math.floor((cg.CW + cg.CH) / 8)) ? best : null;
}

/** The direction that leaves the map from a border cell (orthogonal, outward). */
function outward(cg: Coarse, c: number): number {
  const x = c % cg.CW;
  const y = Math.floor(c / cg.CW);
  return x === 0 ? 4 : x === cg.CW - 1 ? 0 : y === 0 ? 2 : 6;
}

const COMPASS_NAMES = ["North", "North", "North-East", "East", "East", "East", "South-East", "South", "South", "South", "South-West", "West", "West", "West", "North-West", "North"];

// ---------------------------------------------------------------------------
// The railway (before the arterials, which cross it square-on)

/** A run of the railway on the coarse grid: row or column `at`, from `a` to `b` (inclusive, any order). */
interface RailRun {
  axis: "h" | "v";
  at: number;
  a: number;
  b: number;
}

type Side = 0 | 1 | 2 | 3; // north, east, south, west

function sideOf(w: Work, b: number): Side {
  const e = edgePoint(w.W, w.H, b, w.centre.x, w.centre.y);
  const v = compassVec(b);
  const onX = e.x === 0 || e.x === w.W - 1;
  const onY = e.y === 0 || e.y === w.H - 1;
  if (onX && (!onY || Math.abs(v.dx) >= Math.abs(v.dy))) return e.x === 0 ? 3 : 1;
  return e.y === 0 ? 0 : 2;
}

/**
 * The railway: two runs of track across the map past the core (stage 4).
 * The line is chosen among straight lines, Z shapes (two parallel runs and
 * a short link) and L shapes (when the line leaves by adjacent sides) on
 * the coarse grid: never through the square or near it, never over the sea,
 * over a river or lake only straight and within a viaduct's span, gentle
 * on the levels, with as few bends as it can, and passing the core at a
 * short walk (about three coarse cells from the square's ring), where the
 * station goes.
 */
export function stageRail(w: Work) {
  const rails = w.geo.links.filter((l) => l.mode === "rail");
  if (!rails.length) return;
  const cg = coarseOf(w);
  const { CW, CH } = cg;
  const b1 = (rails[0] as GeoLink).bearing;
  let b2 = rails.length > 1 ? (rails[rails.length - 1] as GeoLink).bearing : (b1 + 8) & 15;
  if (bearingGap(b1, b2) < 4) b2 = (b1 + 8) & 15;
  let sa = sideOf(w, b1);
  let sz = sideOf(w, b2);
  if (sa === sz) sz = ((sa + 2) & 3) as Side;
  // Normalise: opposite sides as north–south or west–east (the search is symmetric).
  if (((sa + 2) & 3) === sz) {
    sa = sa === 0 || sa === 2 ? 0 : 3;
    sz = sa === 0 ? 2 : 1;
  }
  const pa = edgePoint(w.W, w.H, b1, w.centre.x, w.centre.y);
  const pz = edgePoint(w.W, w.H, b2, w.centre.x, w.centre.y);
  const ends = [
    { x: Math.floor(pa.x / G), y: Math.floor(pa.y / G) },
    { x: Math.floor(pz.x / G), y: Math.floor(pz.y / G) },
  ];
  const rx0 = w.ring.x / G;
  const ry0 = w.ring.y / G;
  const rx1 = (w.ring.x + w.ring.w) / G - 1;
  const ry1 = (w.ring.y + w.ring.h) / G - 1;
  const ringGap = (i: number, j: number) => Math.max(rx0 - i, i - rx1, ry0 - j, j - ry1, 0) || -1;
  // Land joined to the square without crossing water: the station stands on the town's side of any river.
  const dry = new Uint8Array(CW * CH);
  {
    const q: number[] = [];
    for (let j = ry0; j <= ry1; j++) {
      for (let i = rx0; i <= rx1; i++) {
        dry[j * CW + i] = 1;
        q.push(j * CW + i);
      }
    }
    for (let qi = 0; qi < q.length; qi++) {
      const c = q[qi] as number;
      const x = c % CW;
      for (const n of [x > 0 ? c - 1 : -1, x < CW - 1 ? c + 1 : -1, c - CW, c + CW]) {
        if (n < 0 || n >= CW * CH || dry[n] || cg.wet[n] || cg.sea[n]) continue;
        dry[n] = 1;
        q.push(n);
      }
    }
  }
  const viaduct = w.recipe.connectors.filter((k) => k.roads.includes("rail")).reduce((m, k) => Math.max(m, k.max_span), 8);
  const maxWet = Math.max(1, Math.floor((viaduct + 2) / G));
  const r = rngOf(w, "rail");
  const jitter = r.int(0, 1000);

  /** Cost of a line (or Infinity): cells, level changes, water, bends, how it passes the core, how near its ends are to the links. */
  const cost = (runs: RailRun[], bound = Infinity): number => {
    let s = runs.length * 70;
    let closest = 99;
    let station = false;
    let first = -1;
    let last = -1;
    for (let ri = 0; ri < runs.length; ri++) {
      const run = runs[ri] as RailRun;
      const step = run.b >= run.a ? 1 : -1;
      let wet = 0;
      let prevLv = -1;
      for (let p = run.a; ; p += step) {
        const i = run.axis === "h" ? p : run.at;
        const j = run.axis === "h" ? run.at : p;
        const c = j * CW + i;
        if (cg.sea[c] || cg.blocked[c]) return Infinity;
        const gap = ringGap(i, j);
        if (gap > 0 && gap < 2) return Infinity;
        if (gap < 0) return Infinity;
        closest = Math.min(closest, gap);
        if (gap <= 6 && dry[c] && !cg.wet[c]) station = true;
        const bend = (p === run.a && ri > 0) || (p === run.b && ri < runs.length - 1);
        if (cg.wet[c]) {
          if (bend) return Infinity;
          wet++;
          if (wet > maxWet) return Infinity;
          s += 12;
        } else wet = 0;
        const lv = cg.level[c] as number;
        if (prevLv >= 0 && lv !== prevLv) s += 14 * Math.abs(lv - prevLv);
        prevLv = lv;
        s += cg.rock[c] ? 30 : 2;
        if (first < 0) first = c;
        last = c;
        if (p === run.b) break;
        if (s > bound) return Infinity;
      }
    }
    if (!station) return Infinity;
    s += Math.abs(closest - 4) * 30;
    const fx = first % CW;
    const fy = Math.floor(first / CW);
    const lx = last % CW;
    const ly = Math.floor(last / CW);
    const e0 = ends[0] as { x: number; y: number };
    const e1 = ends[1] as { x: number; y: number };
    const near = Math.min(Math.abs(fx - e0.x) + Math.abs(fy - e0.y) + Math.abs(lx - e1.x) + Math.abs(ly - e1.y), Math.abs(fx - e1.x) + Math.abs(fy - e1.y) + Math.abs(lx - e0.x) + Math.abs(ly - e0.y));
    return s + near * 3;
  };
  let best: RailRun[] | null = null;
  let bestCost = Infinity;
  const consider = (runs: RailRun[], k: number) => {
    const c = cost(runs, bestCost) + ((k * 2654435761 + jitter) % 7);
    if (c < bestCost) {
      bestCost = c;
      best = runs;
    }
  };
  if ((sa === 0 && sz === 2) || (sa === 3 && sz === 1)) {
    // Opposite sides: straight across, or a Z.
    const hz = sa === 3;
    const along = hz ? CW : CH;
    const across = hz ? CH : CW;
    const ax: "h" | "v" = hz ? "h" : "v";
    const cx: "h" | "v" = hz ? "v" : "h";
    for (let r1 = 1; r1 < across - 1; r1++) consider([{ axis: ax, at: r1, a: 0, b: along - 1 }], r1);
    for (let r1 = 1; r1 < across - 1; r1++) {
      for (let r2 = Math.max(1, r1 - 8); r2 <= Math.min(across - 2, r1 + 8); r2++) {
        if (Math.abs(r2 - r1) < 2) continue;
        for (let k = 3; k < along - 3; k++) {
          consider(
            [
              { axis: ax, at: r1, a: 0, b: k },
              { axis: cx, at: k, a: r1, b: r2 },
              { axis: ax, at: r2, a: k, b: along - 1 },
            ],
            r1 * 1000 + r2 * 37 + k,
          );
        }
      }
    }
  } else {
    // Adjacent sides: an L. One run leaves by the north or south side (a column), the other by the east or west (a row).
    const vSide = sa === 0 || sa === 2 ? sa : sz;
    const hSide = vSide === sa ? sz : sa;
    for (let i = 1; i < CW - 1; i++) {
      for (let j = 1; j < CH - 1; j++) {
        consider(
          [
            { axis: "v", at: i, a: vSide === 0 ? 0 : CH - 1, b: j },
            { axis: "h", at: j, a: i, b: hSide === 3 ? 0 : CW - 1 },
          ],
          i * 1000 + j,
        );
      }
    }
  }
  if (!best) return;
  const runs: RailRun[] = best;
  const { W } = w;
  const base = { cls: RoadClass.None, style: "path" as const, district: -1, arterial: -1 };
  runs.forEach((run, ri) => {
    const lo = Math.min(run.a, run.b);
    const hi = Math.max(run.a, run.b);
    // Bends meet in the middle of their coarse cell; the ends at the map's edge run to it.
    const startBend = ri > 0;
    const endBend = ri < runs.length - 1;
    const aIsLo = run.a <= run.b;
    const lowBend = aIsLo ? startBend : endBend;
    const highBend = aIsLo ? endBend : startBend;
    const t0 = lo * G + (lowBend ? 1 : 0);
    const t1 = hi * G + (highBend ? 2 : G - 1);
    const s = run.axis === "h" ? boxSeg("h", t0, run.at * G + 1, t1, run.at * G + 2, base) : boxSeg("v", run.at * G + 1, t0, run.at * G + 2, t1, base);
    w.rail.push(s);
    for (let y = s.y0; y <= s.y1; y++) {
      for (let x = s.x0; x <= s.x1; x++) {
        const c = y * W + x;
        w.railCells.push(c);
        w.terrain[c] = Terrain.Rail;
        w.level[c] = w.level[(run.axis === "h" ? s.y0 : y) * W + (run.axis === "h" ? x : s.x0)] as number;
        w.use[c] = (w.use[c] as number) | Use.Rail;
      }
    }
    for (const m of bridgeRuns(w, s, (span) => connectorFor(w, "rail", "river", span), false)) w.connectors.push({ ...m, kind: "viaduct", arterial: -1 });
  });
}

// ---------------------------------------------------------------------------
// Stage 4: arterials

export function stageArterials(w: Work) {
  const geo = w.geo;
  const caps = BAND_CAPS[w.band];
  const cg = coarseOf(w);
  const { CW, CH } = cg;
  for (const s of w.rail) {
    for (let y = s.y0; y <= s.y1; y++) {
      for (let x = s.x0; x <= s.x1; x++) {
        const k = Math.floor(y / G) * CW + Math.floor(x / G);
        const code = s.axis === "h" ? 1 : 2;
        cg.rail[k] = cg.rail[k] && cg.rail[k] !== code ? 3 : code;
      }
    }
  }
  // The square's ring of streets, on the coarse lattice; arterials leave it square-on (or at 45° from a corner).
  const rx0 = w.ring.x / G;
  const ry0 = w.ring.y / G;
  const rx1 = (w.ring.x + w.ring.w) / G - 1;
  const ry1 = (w.ring.y + w.ring.h) / G - 1;
  const ringBase = { cls: RoadClass.Street, style: "asphalt" as const, district: -1, arterial: -1 };
  const ringSegs: Segment[] = [
    boxSeg("h", w.ring.x, w.ring.y, w.ring.x + w.ring.w - 1, w.ring.y + 3, ringBase),
    boxSeg("h", w.ring.x, w.ring.y + w.ring.h - 4, w.ring.x + w.ring.w - 1, w.ring.y + w.ring.h - 1, ringBase),
    boxSeg("v", w.ring.x, w.ring.y, w.ring.x + 3, w.ring.y + w.ring.h - 1, ringBase),
    boxSeg("v", w.ring.x + w.ring.w - 4, w.ring.y, w.ring.x + w.ring.w - 1, w.ring.y + w.ring.h - 1, ringBase),
  ];
  for (let j = ry0; j <= ry1; j++) for (let i = rx0; i <= rx1; i++) cg.blocked[j * CW + i] = 1;
  // Running alongside the railway costs as running alongside a road.
  const railCells: number[] = [];
  for (let c = 0; c < CW * CH; c++) if (cg.rail[c]) railCells.push(c);
  markNear(cg, railCells);
  // Exits: two cells straight out from a side (in line with the ring's side streets, or clear of its corners, so
  // no road leaves a few cells off a street's line), or two cells out at 45° from a corner.
  const exits: { c: number; d: number; via: number[] }[] = [];
  const at = (i: number, j: number) => (i >= 0 && j >= 0 && i < CW && j < CH ? j * CW + i : -1);
  const sideOk = (k: number, lo: number, hi: number) => k === lo || k === hi || (k >= lo + 2 && k <= hi - 2);
  const exit = (i: number, j: number, d: number, ring: number) => {
    const dx = DX8[d] as number;
    const dy = DY8[d] as number;
    const c1 = at(i, j);
    const c2 = at(i + dx, j + dy);
    const c3 = at(i + 2 * dx, j + 2 * dy);
    if (c1 < 0 || c2 < 0) return;
    // A 45° exit runs three cells out before it may turn (clear of the ring's side streets' lines).
    if (isDiagDir(d)) {
      if (c3 >= 0) exits.push({ c: c3, d, via: [ring, c1, c2] });
    } else exits.push({ c: c2, d, via: [c1] });
  };
  for (let i = rx0; i <= rx1; i++) {
    if (!sideOk(i, rx0, rx1)) continue;
    exit(i, ry0 - 1, 2, at(i, ry0));
    exit(i, ry1 + 1, 6, at(i, ry1));
  }
  for (let j = ry0; j <= ry1; j++) {
    if (!sideOk(j, ry0, ry1)) continue;
    exit(rx0 - 1, j, 4, at(rx0, j));
    exit(rx1 + 1, j, 0, at(rx1, j));
  }
  exit(rx0 - 1, ry0 - 1, 3, at(rx0, ry0));
  exit(rx1 + 1, ry0 - 1, 1, at(rx1, ry0));
  exit(rx0 - 1, ry1 + 1, 5, at(rx0, ry1));
  exit(rx1 + 1, ry1 + 1, 7, at(rx1, ry1));
  const clear = (c: number) => c >= 0 && !cg.sea[c] && !cg.wet[c] && !cg.rail[c] && !cg.blocked[c];
  const usable = exits.filter((e) => clear(e.c) && e.via.every((v, k) => (isDiagDir(e.d) && k === 0) || clear(v)));
  // No arterial runs along the ring: the cells round it are only crossed by the exits' first cells.
  for (let j = ry0 - 1; j <= ry1 + 1; j++) {
    for (let i = rx0 - 1; i <= rx1 + 1; i++) {
      const c = at(i, j);
      if (c >= 0) cg.blocked[c] = 1;
    }
  }
  const taken = new Set<number>();
  const tooClose = (e: { c: number }) => {
    if (taken.has(e.c)) return false;
    const x = e.c % CW;
    const y = Math.floor(e.c / CW);
    for (const t of taken) if (Math.max(Math.abs((t % CW) - x), Math.abs(Math.floor(t / CW) - y)) <= 1) return true;
    return false;
  };

  // ---- which arterials: the town's road links, deduplicated by bearing, within the band's cap
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
  // A river town gets a road over the river: the link that way, else a town road that ends across it.
  let crossing: { bearing: number } | null = null;
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
    if (rb !== NO_BEARING && !picked.some((p) => bearingGap(p.bearing, rb) <= 2)) {
      const link = roads.find((l) => bearingGap(l.bearing, rb) <= 2 && edgeCell(w, cg, l.bearing) !== null);
      if (link) {
        if (picked.length >= caps.arterials[1]) picked.pop();
        picked.push({ bearing: link.bearing, link });
      } else crossing = { bearing: rb };
    }
  }
  picked.sort((a, b) => a.bearing - b.bearing);

  // ---- a ring road in cities: a coarse rectangle, broken where it meets water, the railway or rock (laid after the arterials, trimmed to its junctions)
  const ringChains: number[][] = [];
  if (caps.ring) {
    const i0 = Math.floor(CW * 0.18);
    const i1 = Math.ceil(CW * 0.82) - 1;
    const j0 = Math.floor(CH * 0.18);
    const j1 = Math.ceil(CH * 0.82) - 1;
    const loop: number[] = [];
    for (let i = i0; i < i1; i++) loop.push(j0 * CW + i);
    for (let j = j0; j < j1; j++) loop.push(j * CW + i1);
    for (let i = i1; i > i0; i--) loop.push(j1 * CW + i);
    for (let j = j1; j > j0; j--) loop.push(j * CW + i0);
    const railNear = (c: number) => [c - 1, c + 1, c - CW, c + CW].some((n) => n >= 0 && n < CW * CH && cg.rail[n] !== 0);
    const ok = (c: number) => !(cg.wet[c] || cg.sea[c] || cg.blocked[c] || cg.rock[c] || cg.rail[c] || railNear(c));
    const startAt = loop.findIndex((c) => !ok(c));
    if (startAt < 0) ringChains.push([...loop, loop[0] as number]);
    else {
      let run: number[] = [];
      for (let k = 1; k <= loop.length; k++) {
        const c = loop[(startAt + k) % loop.length] as number;
        if (ok(c)) run.push(c);
        else {
          if (run.length >= 2) ringChains.push(run);
          run = [];
        }
      }
    }
    for (const ch of ringChains) for (const c of ch) cg.road[c] = 1;
    markNear(cg, ringChains.flat());
  }

  const kinds = (span: number) => connectorFor(w, "arterial", "river", span);
  let bridges = 0;
  const names = new Set<string>();
  const artPath = new Uint8Array(CW * CH);
  const lay = (art: Arterial, path: { c: number; d: number }[]) => {
    const segs = roadSegs(cg, path, { cls: RoadClass.Arterial, style: "asphalt", district: -1, arterial: art.index });
    for (const s of segs) {
      for (const m of bridgeRuns(w, s, kinds)) {
        // A bridge another arterial already built here is shared, not counted twice.
        if (!w.connectors.some((o) => o.x === m.x && o.y === m.y && o.w === m.w && o.h === m.h)) {
          if (!caps.ring) {
            const bi = Math.floor((m.x + (m.w >> 1)) / G);
            const bj = Math.floor((m.y + (m.h >> 1)) / G);
            for (let j = 0; j < CH; j++) for (let i = 0; i < CW; i++) if (Math.abs(i - bi) + Math.abs(j - bj) <= 10) cg.nearBridge[j * CW + i] = 1;
          }
          w.connectors.push(m);
          bridges++;
        }
      }
      const lc = levelCrossing(w, s, paintSegment(w, s, true));
      if (lc) w.connectors.push(lc);
      w.segments.push(s);
      segCells(s, w.W, w.H, (c) => art.cells.push(c));
    }
    for (const p of path) {
      cg.road[p.c] = 1;
      artPath[p.c] = 1;
    }
    markNear(cg, path.map((p) => p.c));
  };
  const costs = (water: number): Costs8 => ({ base: 10, diag: 14, slope: 30, water, rock: 60, road: 6, t45: 35, t90: 50, near: 14, border: 120 });
  const startsFor = () => usable.filter((e) => !tooClose(e)).map((e) => ({ c: e.c, d: e.d, g: isDiagDir(e.d) ? 6 : 0 }));
  const withRingDry = (path: { c: number; d: number }[]) => {
    const first = path[0] as { c: number; d: number };
    const st = usable.find((s) => s.c === first.c && s.d === first.d);
    return st ? [...st.via.map((c) => ({ c, d: first.d })), ...path] : path;
  };
  const withRing = (path: { c: number; d: number }[]) => {
    const first = path[0] as { c: number; d: number };
    const st = usable.find((s) => s.c === first.c && s.d === first.d);
    if (!st) return path;
    for (const v of st.via) taken.add(v);
    taken.add(first.c);
    return [...st.via.map((c) => ({ c, d: first.d })), ...path];
  };
  /** The widest crossing a road may bridge (tiles of water). */
  const maxSpan = w.recipe.connectors.filter((k) => k.roads.includes("arterial") && k.crosses.includes("river")).reduce((m, k) => Math.max(m, k.max_span), 0);
  /** Water a run of a path crosses wider than any bridge: its coarse cells. */
  const tooWide = (segs: Segment[]): number[] => {
    const out: number[] = [];
    for (const s of segs) {
      if (s.axis !== "h" && s.axis !== "v") continue;
      let run = 0;
      const a0 = s.axis === "h" ? s.x0 : s.y0;
      const a1 = s.axis === "h" ? s.x1 : s.y1;
      for (let a = a0; a <= a1 + 1; a++) {
        let wet = false;
        if (a <= a1) {
          for (let k = s.axis === "h" ? s.y0 : s.x0; k <= (s.axis === "h" ? s.y1 : s.x1); k++) {
            const x = s.axis === "h" ? a : k;
            const y = s.axis === "h" ? k : a;
            if (isWaterBody(w, y * w.W + x)) wet = true;
          }
        }
        if (wet) run++;
        else {
          if (run > maxSpan) {
            const mid = a - 1 - (run >> 1);
            const x = s.axis === "h" ? mid : s.ax;
            const y = s.axis === "h" ? s.ay : mid;
            out.push(Math.floor(y / G) * CW + Math.floor(x / G));
          }
          run = 0;
        }
      }
    }
    return out;
  };
  /**
   * A route by `coarsePath8`, routed again where it would run alongside
   * another road or the railway (kept well clear), or cross water wider than
   * a bridge spans (that crossing barred).
   */
  const route = (starts: { c: number; d: number; g: number }[], isGoal: (c: number, d: number) => boolean, goal: number, water: number, exempt?: (c: number) => boolean) => {
    let k = costs(water);
    const barred: number[] = [];
    let path: { c: number; d: number }[] | null = null;
    for (let tries = 0; tries < 4; tries++) {
      const p = coarsePath8(cg, starts, isGoal, goal, k, exempt);
      if (!p) break;
      path = p;
      const segs = roadSegs(cg, withRingDry(p), { cls: RoadClass.Arterial, style: "asphalt", district: -1, arterial: -1 });
      const wide = maxSpan > 0 ? tooWide(segs) : [];
      const crowd = segs.some((s) => crowded(w, s, null));
      if (!wide.length && !crowd) break;
      for (const c of wide) {
        if (!cg.blocked[c]) {
          cg.blocked[c] = 1;
          barred.push(c);
        }
      }
      if (crowd) k = { ...k, near: 70 };
    }
    for (const c of barred) cg.blocked[c] = 0;
    return path;
  };
  for (const p of picked) {
    const goal = edgeCell(w, cg, p.bearing);
    if (goal === null) continue;
    // Once the band's bridges are built, a new crossing costs far more: roads share them.
    const water = bridges >= caps.bridges ? 900 : 70 + bridges * 60;
    const starts = startsFor();
    const out = outward(cg, goal);
    // Any border cell on that side within four of the bearing's, left straight out (no jog to hit one cell).
    const gx = goal % CW;
    const gy = Math.floor(goal / CW);
    const railBeside = (c: number) => [c - 1, c + 1, c - CW, c + CW, c - CW - 1, c - CW + 1, c + CW - 1, c + CW + 1].some((n) => n >= 0 && n < CW * CH && cg.rail[n] !== 0);
    const near = (c: number) => !cg.wet[c] && !cg.rail[c] && !railBeside(c) && outward(cg, c) === out && Math.abs((c % CW) - gx) + Math.abs(Math.floor(c / CW) - gy) <= 4 && (c % CW === 0 || c % CW === CW - 1 || c < CW || c >= CW * (CH - 1));
    const path = route(starts, (c, d) => d === out && near(c), goal, water, near);
    if (!path) continue;
    const toName = p.link?.to_name ?? "";
    let name = toName ? `${toName} Road` : `${COMPASS_NAMES[p.bearing] ?? "North"} Road`;
    if (names.has(name)) name = `Old ${name}`;
    names.add(name);
    const art: Arterial = { index: w.arterials.length, name, mode: p.link ? p.link.mode : "road", bearing: p.bearing, toName, cells: [] };
    lay(art, withRing(path));
    w.arterials.push(art);
  }
  if (crossing) {
    // A road over the river that ends in the town on the far bank (the street growth joins it up).
    const v = compassVec(crossing.bearing);
    const mx = w.centre.x;
    const my = w.centre.y;
    let tx = -1;
    let ty = -1;
    let wetSeen = false;
    for (let k = 4; k < 400; k += 2) {
      const x = mx + Math.trunc((v.dx * k) / 1024);
      const y = my + Math.trunc((v.dy * k) / 1024);
      if (x < 16 || y < 16 || x >= w.W - 16 || y >= w.H - 16) break;
      const c = y * w.W + x;
      if (isWaterBody(w, c)) wetSeen = true;
      else if (wetSeen && (w.riverDist[c] as number) >= Math.max(6, w.floodplain + 3)) {
        tx = x;
        ty = y;
        break;
      }
    }
    if (tx >= 0) {
      const goal = Math.floor(ty / G) * CW + Math.floor(tx / G);
      const water = bridges >= caps.bridges ? 900 : 70 + bridges * 60;
      const starts = startsFor();
      let path = route(starts, (c) => c === goal, goal, water);
      // The street ends a little past its bridge (the far bank's streets grow from it).
      if (path) {
        let last = -1;
        path.forEach((q, i) => {
          if (cg.wet[q.c]) last = i;
        });
        if (last >= 0) path = path.slice(0, Math.min(path.length, last + 3));
      }
      if (path && path.some((q) => cg.wet[q.c])) {
        const name = names.has("Bridge Street") ? "Bridge Road" : "Bridge Street";
        names.add(name);
        const art: Arterial = { index: w.arterials.length, name, mode: "local", bearing: crossing.bearing, toName: "", cells: [] };
        lay(art, withRing(path));
        w.arterials.push(art);
      }
    }
  }
  for (const s of ringSegs) {
    paintSegment(w, s);
    w.segments.push(s);
  }

  // ---- the ring road's pieces, each kept between its first and last junction with an arterial
  if (ringChains.length) {
    const art: Arterial = { index: w.arterials.length, name: "Ring Road", mode: "local", bearing: NO_BEARING, toName: "", cells: [] };
    const arterialCell = new Uint8Array(CW * CH);
    for (let c = 0; c < CW * CH; c++) arterialCell[c] = artPath[c] as number;
    // Never alongside an arterial: the ring breaks where one runs beside it (a cell off its line).
    const pieces: number[][] = [];
    for (const ch of ringChains) {
      const closed = ch.length > 2 && ch[0] === ch[ch.length - 1];
      let run: number[] = [];
      let broken = false;
      ch.forEach((c, k) => {
        const p = ch[Math.max(0, k - 1)] as number;
        const q = ch[Math.min(ch.length - 1, k + 1)] as number;
        const horiz = Math.floor(p / CW) === Math.floor(q / CW);
        const side = horiz ? [c - CW, c + CW] : [c % CW > 0 ? c - 1 : -1, c % CW < CW - 1 ? c + 1 : -1];
        const beside = !arterialCell[c] && side.some((n) => n >= 0 && n < CW * CH && arterialCell[n]);
        if (beside) {
          broken = true;
          if (run.length) pieces.push(run);
          run = [];
        } else run.push(c);
      });
      if (run.length) pieces.push(run);
      if (closed && !broken) {
        pieces.pop();
        pieces.push(ch);
      }
    }
    for (const ch of pieces) {
      const closed = ch.length > 2 && ch[0] === ch[ch.length - 1];
      let a = 0;
      let b = ch.length - 1;
      if (!closed) {
        while (a < ch.length && !arterialCell[ch[a] as number]) a++;
        while (b >= 0 && !arterialCell[ch[b] as number]) b--;
        if (b - a < 2) continue;
      }
      const piece = ch.slice(a, b + 1).map((c, k, arr) => {
        if (k === 0) return { c, d: 0 };
        const p = arr[k - 1] as number;
        const dx = (c % CW) - (p % CW);
        const dy = Math.floor(c / CW) - Math.floor(p / CW);
        return { c, d: dx > 0 ? 0 : dx < 0 ? 4 : dy < 0 ? 2 : 6 };
      });
      if (piece.length > 1) (piece[0] as { d: number }).d = (piece[1] as { d: number }).d;
      for (const s of roadSegs(cg, piece, { cls: RoadClass.Arterial, style: "asphalt", district: -1, arterial: art.index })) {
        const lc = levelCrossing(w, s, paintSegment(w, s, true));
        if (lc) w.connectors.push(lc);
        w.segments.push(s);
        segCells(s, w.W, w.H, (c) => art.cells.push(c));
      }
    }
    if (art.cells.length) w.arterials.push(art);
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
