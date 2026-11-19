// Layout v2 stage 6 (TOWN_GENERATION.md §5; DEC-090 D5 amended "A + B"):
// streets grown from the square outward. The square's ring and the
// arterials (stage 4) are the trunk; the old core adds radial streets from
// the square's corners. Then, district by district in growth order, the
// land furthest from any street (measured with the district type's block
// and depth, so blocks come out the type's size) gets a street through it:
// straight between two streets (a T at each end, offset from any street on
// the far side, so crossings are offset rather than four-way), L-shaped
// (one bend) or, where the type has them, a cul-de-sac with a turning
// head. Streets run at 45° where the terrain or the square calls for it:
// along the river or the shore, along a hill's contours, or as a radial
// from the square. Each district type's pattern (organic, grid, curvy,
// contour, yards, quay, ribbon) sets widths, preferences and how often it
// bends. Every street joins the network at both ends (or ends in a designed
// turning head); a district the arterials miss has a chain of its streets
// promoted to a main street. Then small greens at junctions, and the
// direction code of every street cell for the art. Integer maths, keyed
// streams per district.
import { mix } from "../rng";
import { Terrain, type StreetEndKind } from "../townLayout";
import { MinHeap } from "./imath";
import { DIR8, axisOfDir, bandLo, crowded, forBand, headCells, isDiagDir, isHead, levelCrossing, makeSeg, paintHead, paintSegment, segCells, segSteps, stepOf, streetAxes } from "./net";
import { RoadClass, Use, WaterKind, countrysideMargin, distanceField, isWaterBody, rngOf, type Segment, type Work, type WorkDistrict } from "./work";

/** How a street pattern grows (defaults by pattern name; the registry gives block and depth). */
interface Growth {
  /** Widths to pick from, with weights; "lane" is a paved lane two tiles wide. */
  widths: { width: number; style: Segment["style"]; weight: number }[];
  /** Where 45° streets may run: radials from the square, along the terrain (river, shore, contours), or both. */
  diagonal: { radial: boolean; terrain: boolean };
  /** Bonus for an east–west street (rows of buildings front them). */
  rows: number;
  /** Penalty for a four-way crossing (offset crossings and T-junctions instead). */
  crossroads: number;
  /** Share of streets with one bend (L-shaped blocks). */
  bend: number;
  /** Share of streets that end in a turning head where the type allows dead ends. */
  culDeSac: number;
  /** The turning head a cul-de-sac ends in. */
  head: "turning_circle" | "hammerhead";
  /** The longest street run, tiles. */
  maxRun: number;
  /** How far a street may shift from the deepest point to find a better line, tiles. */
  shift: number;
}

const GROWTH: Record<string, Growth> = {
  organic: {
    widths: [
      { width: 2, style: "lane", weight: 5 },
      { width: 3, style: "asphalt", weight: 5 },
    ],
    diagonal: { radial: true, terrain: true },
    rows: 120,
    crossroads: 500,
    bend: 0.45,
    culDeSac: 0,
    head: "turning_circle",
    maxRun: 34,
    shift: 2,
  },
  grid: { widths: [{ width: 3, style: "asphalt", weight: 1 }], diagonal: { radial: false, terrain: true }, rows: 260, crossroads: 120, bend: 0.12, culDeSac: 0, head: "hammerhead", maxRun: 70, shift: 1 },
  curvy: {
    widths: [{ width: 3, style: "asphalt", weight: 1 }],
    diagonal: { radial: true, terrain: true },
    rows: 120,
    crossroads: 500,
    bend: 0.6,
    culDeSac: 0.45,
    head: "turning_circle",
    maxRun: 40,
    shift: 3,
  },
  contour: { widths: [{ width: 3, style: "asphalt", weight: 1 }], diagonal: { radial: false, terrain: true }, rows: 140, crossroads: 400, bend: 0.4, culDeSac: 0.3, head: "turning_circle", maxRun: 50, shift: 2 },
  yards: { widths: [{ width: 4, style: "asphalt", weight: 1 }], diagonal: { radial: false, terrain: false }, rows: 160, crossroads: 150, bend: 0.25, culDeSac: 0.35, head: "hammerhead", maxRun: 60, shift: 2 },
  quay: { widths: [{ width: 4, style: "asphalt", weight: 1 }], diagonal: { radial: false, terrain: true }, rows: 160, crossroads: 300, bend: 0.25, culDeSac: 0.3, head: "hammerhead", maxRun: 60, shift: 2 },
  ribbon: { widths: [{ width: 2, style: "dirt", weight: 1 }], diagonal: { radial: false, terrain: true }, rows: 80, crossroads: 300, bend: 0.4, culDeSac: 0.5, head: "turning_circle", maxRun: 70, shift: 3 },
};

const INF = 0x3fffffff;
/** Covered: within half a block across and half a block's depth down of a street (×1000). */
const COVERED = 1080;

interface Leg {
  x: number;
  y: number;
  dir: number;
  steps: number;
}

interface Cand {
  legs: Leg[];
  width: number;
  style: Segment["style"];
  /** The far end's termination (a cul-de-sac's head), or null. */
  end: StreetEndKind | null;
  score: number;
}

export function stageStreets(w: Work) {
  const { W, H } = w;
  const N = W * H;
  const t0 = performance.now();
  const pre = w.ground?.terrain ?? new Uint8Array(w.terrain);
  const preLevel = w.ground?.level ?? new Uint8Array(w.level);
  const railDist = w.rail.length ? distanceField(W, H, (c) => ((w.use[c] as number) & Use.Rail) !== 0, () => true, 4) : null;
  /**
   * Land a street may be laid on: the districts, and the countryside within a
   * few cells of them (so a street can reach a road that skirts the town), but
   * never near the map's edge.
   */
  const townDist = distanceField(W, H, (c) => (w.district[c] as number) >= 0, () => true, 5);
  const M = countrysideMargin(W, H);
  const land = new Uint8Array(N);
  for (let c = 0; c < N; c++) {
    if ((townDist[c] as number) > 4 || isWaterBody(w, c) || (w.use[c] as number) & (Use.Road | Use.Square | Use.Rail | Use.Bridge) || w.terrain[c] === Terrain.Rock) continue;
    const x = c % W;
    const y = (c - x) / W;
    if ((w.district[c] as number) < 0 && Math.min(x, y, W - 1 - x, H - 1 - y) < Math.max(3, M - 4)) continue;
    if (railDist && (railDist[c] as number) <= 1) continue;
    land[c] = 1;
  }
  const isRoad = (c: number) => ((w.use[c] as number) & Use.Road) !== 0;
  // The railway and the cell either side of it: a street may cross it square-on (a level crossing), a few times a town.
  const railAx = new Uint8Array(N);
  for (const s of w.rail) {
    const code = s.axis === "h" ? 1 : 2;
    for (let y = s.y0 - (code === 1 ? 1 : 0); y <= s.y1 + (code === 1 ? 1 : 0); y++) {
      for (let x = s.x0 - (code === 2 ? 1 : 0); x <= s.x1 + (code === 2 ? 1 : 0); x++) {
        if (x < 0 || y < 0 || x >= W || y >= H) continue;
        const c = y * W + x;
        if (isWaterBody(w, c) || (w.use[c] as number) & (Use.Bridge | Use.Road)) railAx[c] = 3;
        else railAx[c] = railAx[c] && railAx[c] !== code ? 3 : code;
      }
    }
  }
  let crossingsLeft = w.rail.length ? 2 + Math.floor(w.rail.reduce((n, s) => n + segSteps(s) + 1, 0) / 60) : 0;
  const crossable = (c: number, dir: number) => crossingsLeft > 0 && ((railAx[c] === 1 && (dir & 3) === 2) || (railAx[c] === 2 && (dir & 3) === 0));
  // Segment cover: how many runs cover each cell (dead ends and the joined test).
  const cover = new Uint16Array(N);
  const owner = new Int32Array(N).fill(-1);
  const addCover = (s: Segment, i: number) =>
    segCells(s, W, H, (c) => {
      cover[c] = (cover[c] as number) + 1;
      if ((owner[c] as number) < 0) owner[c] = i;
    });
  w.segments.forEach((s, i) => addCover(s, i));

  // ---- the distance field: weighted steps from the nearest street through land (each district's block and depth)
  const cx = new Int32Array(w.districts.length);
  const cy = new Int32Array(w.districts.length);
  for (const d of w.districts) {
    const b = (d.type.streets.block[0] + d.type.streets.block[1]) / 2;
    const dp = (d.type.streets.depth[0] + d.type.streets.depth[1]) / 2;
    cx[d.index] = Math.round(2000 / b);
    cy[d.index] = Math.round(2000 / dp);
  }
  const dist = new Int32Array(N).fill(INF);
  (STATS as unknown as { dist: Int32Array }).dist = dist;
  const heap = new MinHeap(4096);
  const relax = () => {
    while (heap.size) {
      const c = heap.pop();
      const dc = heap.lastKey;
      if (dc > (dist[c] as number)) continue;
      const x = c % W;
      for (let k4 = 0; k4 < 4; k4++) {
        const n = k4 === 0 ? (x > 0 ? c - 1 : -1) : k4 === 1 ? (x < W - 1 ? c + 1 : -1) : k4 === 2 ? c - W : c + W;
        // The railway is crossed square-on only (never followed along its length).
        if (n < 0 || n >= N || (!land[n] && !((railAx[n] === 1 && k4 >= 2) || (railAx[n] === 2 && k4 < 2)))) continue;
        const d = (w.district[n] as number) >= 0 ? (w.district[n] as number) : Math.max(0, w.district[c] as number);
        const nd = dc + (k4 < 2 ? (cx[d] as number) : (cy[d] as number));
        if (nd < (dist[n] as number)) {
          dist[n] = nd;
          heap.push(nd, n);
        }
      }
    }
  };
  for (let c = 0; c < N; c++) {
    if (!isRoad(c) && !((w.use[c] as number) & Use.Square)) continue;
    dist[c] = 0;
    heap.push(0, c);
  }
  relax();
  const fresh = (cells: number[]) => {
    for (const c of cells) {
      if ((dist[c] as number) === 0) continue;
      dist[c] = 0;
      heap.push(0, c);
    }
    relax();
  };

  // ---- the terrain's directions: along the river, the shore and the contours
  const tangent = (x: number, y: number): { dir: number; kind: "river" | "shore" | "contour" } | null => {
    const at = (f: Uint16Array | Uint8Array, xx: number, yy: number) => f[Math.min(H - 1, Math.max(0, yy)) * W + Math.min(W - 1, Math.max(0, xx))] as number;
    const tryField = (f: Uint16Array | Uint8Array, kind: "river" | "shore" | "contour", near: number) => {
      if (kind !== "contour" && at(f, x, y) > near) return null;
      const r = kind === "contour" ? 5 : 3;
      const gx = at(f, x + r, y) - at(f, x - r, y);
      const gy = at(f, x, y + r) - at(f, x, y - r);
      if (gx === 0 && gy === 0) return null;
      // Along the field's lines: perpendicular to its gradient.
      return { dir: snap8(-gy, gx), kind };
    };
    if (w.geo.river) {
      const t = tryField(w.riverDist, "river", 14);
      if (t) return t;
    }
    if (w.geo.coast) {
      const t = tryField(w.seaDist, "shore", 14);
      if (t) return t;
    }
    return tryField(w.level, "contour", 0);
  };
  const sqx = w.plaza.x + (w.plaza.w >> 1);
  const sqy = w.plaza.y + (w.plaza.h >> 1);

  // ---- tracing a band along a direction
  const bandStatus = (x: number, y: number, dir: number, width: number): 0 | 1 | 2 => {
    // 0: all land; 1: touches a street (and the rest is land or street); 2: blocked.
    let road = false;
    const lo = bandLo(width);
    for (let k = 0; k < width; k++) {
      const bx = axisOfDir(dir) === "h" ? x : x - lo + k;
      const by = axisOfDir(dir) === "h" ? y - lo + k : y;
      if (bx < 0 || by < 0 || bx >= W || by >= H) return 2;
      const c = by * W + bx;
      if (isRoad(c) || (w.use[c] as number) & Use.Square) road = true;
      else if (!land[c] && !crossable(c, dir)) return 2;
    }
    return road ? 1 : 0;
  };
  /** From (x, y) along `dir`: the step where the band first touches a street (`hit`), else where it is blocked. */
  const trace = (x: number, y: number, dir: number, width: number, max: number): { hit: boolean; steps: number } => {
    const [dx, dy] = DIR8[dir] as readonly [number, number];
    for (let t = 1; t <= max; t++) {
      const st = bandStatus(x + dx * t, y + dy * t, dir, width);
      if (st === 1) return { hit: true, steps: t };
      if (st === 2) return { hit: false, steps: t - 1 };
    }
    return { hit: false, steps: max };
  };

  /** Whether every cell of the band at (x, y) is street. */
  const allRoad = (x: number, y: number, dir: number, width: number) => {
    const lo = bandLo(width);
    const h = axisOfDir(dir) === "h";
    for (let k = 0; k < width; k++) {
      const bx = h ? x : x - lo + k;
      const by = h ? y - lo + k : y;
      if (bx < 0 || by < 0 || bx >= W || by >= H || !isRoad(by * W + bx)) return false;
    }
    return true;
  };
  /**
   * What lies across the street a run meets at its end (x, y: the run's
   * last cell of land before the street): a street continuing the run's line
   * (a four-way crossing), or one a few cells aside (a jog, never allowed).
   */
  const across = (x: number, y: number, dir: number, width: number): { four: boolean; jog: boolean } => {
    const [dx, dy] = DIR8[dir] as readonly [number, number];
    // Step over the street met: to the first band past it that is not all street.
    let t = 1;
    while (t < 10 && (t === 1 || allRoad(x + dx * t, y + dy * t, dir, width))) t++;
    if (t >= 10) return { four: false, jog: false };
    const px = -dy;
    const py = dx;
    let four = false;
    let jog = false;
    const span = width + 4;
    for (let o = -span; o <= span; o++) {
      let run = 0;
      for (let k = 0; k < 3; k++) {
        const bx = x + dx * (t + k) + px * o;
        const by = y + dy * (t + k) + py * o;
        if (bx < 0 || by < 0 || bx >= W || by >= H || !isRoad(by * W + bx)) break;
        run++;
      }
      if (run < 3) continue;
      // A street there running our way.
      const a = Math.abs(o);
      if (a <= Math.max(1, width >> 1)) four = true;
      else jog = true;
    }
    return { four, jog: jog && !four };
  };

  const segOf = (l: Leg, width: number, style: Segment["style"], d: number, cls: number, end: StreetEndKind | null = null) => makeSeg(l.x, l.y, l.dir, l.steps, width, { cls, style, district: d, arterial: -1, end });
  /** Whether a run would lie alongside another (or the railway) within a few cells: a near-parallel duplicate. */
  const crowds = (s: Segment) => crowded(w, s, null);

  const commit = (cand: Cand, d: number, cls = RoadClass.Street): Segment[] => {
    const made: Segment[] = [];
    cand.legs.forEach((l, i) => {
      const last = i === cand.legs.length - 1;
      const s = segOf(l, isDiagDir(l.dir) ? cand.width + 1 : cand.width, cand.style, d, cls, last ? cand.end : null);
      made.push(s);
    });
    const cells: number[] = [];
    for (const s of made) {
      const crossed = paintSegment(w, s, crossingsLeft > 0);
      if (crossed.length) {
        const lc = levelCrossing(w, s, crossed);
        if (lc) w.connectors.push(lc);
        crossingsLeft--;
      }
      const i = w.segments.length;
      w.segments.push(s);
      addCover(s, i);
      segCells(s, W, H, (c) => cells.push(c));
      for (const end of [0, 1] as const) {
        if (!isHead(s.ends[end])) continue;
        paintHead(w, s, end);
        for (const c of headCells(s, end, W, H)) {
          cells.push(c);
          cover[c] = (cover[c] as number) + 1;
        }
      }
    }
    for (const c of cells) land[c] = 0;
    fresh(cells.filter((c) => isRoad(c)));
    return made;
  };

  /** Score the run's direction at a point: the pattern's taste, the terrain and the square. */
  const dirScore = (g: Growth, x: number, y: number, dir: number, need: "rows" | "cols", tan: { dir: number; kind: string } | null): number => {
    const ax = axisOfDir(dir);
    const diag = isDiagDir(dir);
    let s = 0;
    if (ax === "h") s += need === "rows" ? g.rows + 160 : 0;
    if (ax === "v") s += need === "cols" ? 160 : 0;
    const radial = snap8(x - sqx, y - sqy);
    const toSquare = (Math.abs(x - sqx) + Math.abs(y - sqy)) < 70;
    if (tan && axisOfDir(tan.dir) === ax) s += tan.kind === "contour" ? 260 : 200;
    if (diag) {
      const okRadial = g.diagonal.radial && toSquare && axisOfDir(radial) === ax;
      const okTerrain = g.diagonal.terrain && tan !== null && axisOfDir(tan.dir) === ax;
      if (!okRadial && !okTerrain) return -INF;
      if (okRadial) s += 140;
    }
    return s;
  };

  /** Candidate streets through the point (x, y) for district d. */
  const candidates = (d: WorkDistrict, g: Growth, x: number, y: number, r: { next(): number; int(a: number, b: number): number; chance(p: number): boolean }, headsLeft: number): Cand[] => {
    const out: Cand[] = [];
    const pickW = () => {
      let total = 0;
      for (const o of g.widths) total += o.weight;
      let u = r.next() * total;
      for (const o of g.widths) {
        u -= o.weight;
        if (u < 0) return o;
      }
      return g.widths[0] as Growth["widths"][number];
    };
    const wo = pickW();
    const tan = tangent(x, y);
    // Which way the land is deep: rows (an east–west street) or columns.
    const runTo = (ddx: number, ddy: number) => {
      let k = 1;
      while (k < 60) {
        const xx = x + ddx * k;
        const yy = y + ddy * k;
        if (xx < 0 || yy < 0 || xx >= W || yy >= H) break;
        const c = yy * W + xx;
        if (isRoad(c) || !land[c]) break;
        k++;
      }
      return k;
    };
    const dv = Math.min(runTo(0, -1), runTo(0, 1)) * (cy[d.index] as number);
    const dh = Math.min(runTo(-1, 0), runTo(1, 0)) * (cx[d.index] as number);
    const need = dv >= dh ? "rows" : "cols";
    for (let dir = 0; dir < 4; dir++) {
      const diag = isDiagDir(dir);
      const width = diag ? wo.width + 1 : wo.width;
      const ds = dirScore(g, x, y, dir, need, tan);
      if (ds <= -INF) continue;
      const [dx, dy] = DIR8[dir] as readonly [number, number];
      const px = -dy;
      const py = dx;
      for (let o = -g.shift; o <= g.shift; o++) {
        const ox = x + px * o;
        const oy = y + py * o;
        if (bandStatus(ox, oy, dir, width) !== 0) continue;
        const fw = trace(ox, oy, dir, width, g.maxRun);
        const bw = trace(ox, oy, (dir + 4) & 7, width, g.maxRun);
        const base = ds - Math.abs(o) * 15 + r.int(0, 60);
        const endQuality = (ex: number, ey: number, ed: number) => {
          const a = across(ex, ey, ed, width);
          if (a.jog) return -INF;
          return a.four ? -g.crossroads : 0;
        };
        if (fw.hit && bw.hit) {
          const len = fw.steps + bw.steps;
          const q = endQuality(ox + dx * (fw.steps - 1), oy + dy * (fw.steps - 1), dir) + endQuality(ox - dx * (bw.steps - 1), oy - dy * (bw.steps - 1), (dir + 4) & 7);
          if (q > -INF && len >= 4) {
            out.push({ legs: [{ x: ox - dx * bw.steps, y: oy - dy * bw.steps, dir, steps: len }], width: wo.width, style: wo.style, end: null, score: base + q - len * 6 });
          }
        }
        // One end on a street: a bend toward another street, or a cul-de-sac.
        for (const [hitSide, other] of [
          [fw, bw],
          [bw, fw],
        ] as const) {
          if (!hitSide.hit) continue;
          const hd = hitSide === fw ? dir : (dir + 4) & 7;
          const od = (hd + 4) & 7;
          const [hx, hy] = DIR8[hd] as readonly [number, number];
          const [qx, qy] = DIR8[od] as readonly [number, number];
          const hq = endQuality(ox + hx * (hitSide.steps - 1), oy + hy * (hitSide.steps - 1), hd);
          if (hq <= -INF) continue;
          const startX = ox + hx * hitSide.steps;
          const startY = oy + hy * hitSide.steps;
          // A bend: run on `k` steps past the point, then turn.
          if (r.chance(g.bend) || !other.hit) {
            for (const k of [2, 4 + r.int(0, 4), other.steps - 2]) {
              if (k > other.steps) continue;
              const kx = ox + qx * k;
              const ky = oy + qy * k;
              for (const turn of diag ? [1, -1] : [2, -2]) {
                const td = (od + turn + 8) & 7;
                if (isDiagDir(td) && dirScore(g, kx, ky, td, need, tan) <= -INF) continue;
                const tw = isDiagDir(td) ? wo.width + 1 : wo.width;
                if (bandStatus(kx, ky, td, tw) !== 0) continue;
                const t2 = trace(kx, ky, td, tw, g.maxRun);
                if (!t2.hit || t2.steps < 3) continue;
                const [tx, ty] = DIR8[td] as readonly [number, number];
                const q2 = endQuality(kx + tx * (t2.steps - 1), ky + ty * (t2.steps - 1), td);
                if (q2 <= -INF) continue;
                const len1 = hitSide.steps + k;
                // The first leg runs past the corner by half the band, so the corner is paved.
                const over = diag || isDiagDir(td) ? 0 : tw - 1 - bandLo(tw);
                out.push({
                  legs: [
                    { x: startX, y: startY, dir: od, steps: len1 + over },
                    { x: kx, y: ky, dir: td, steps: t2.steps },
                  ],
                  width: wo.width,
                  style: wo.style,
                  end: null,
                  score: base + hq + q2 - (len1 + t2.steps) * 6 - 40,
                });
              }
            }
          }
          // A cul-de-sac: on past the point a few steps, with a turning head.
          if (headsLeft > 0 && g.culDeSac > 0 && !other.hit && other.steps >= 6) {
            const ext = Math.min(other.steps - 3, 3 + r.int(0, 4));
            const len = hitSide.steps + ext;
            if (len >= 6) {
              const s = segOf({ x: startX, y: startY, dir: od, steps: len }, width, wo.style, d.index, RoadClass.Street, g.head);
              if (paintHead(w, s, s.ends[1] ? 1 : 0, true) && headClear(s)) out.push({ legs: [{ x: startX, y: startY, dir: od, steps: len }], width: wo.width, style: wo.style, end: g.head, score: base + hq - len * 6 + (r.chance(g.culDeSac) ? 200 : -200) });
            }
          }
          // A spine into open land: later streets join it, and what runs past its last junction is cut back.
          if (!other.hit && other.steps >= 4) {
            const len = hitSide.steps + other.steps;
            out.push({ legs: [{ x: startX, y: startY, dir: od, steps: len }], width: wo.width, style: wo.style, end: null, score: base + hq - len * 4 - 260 });
          }
        }
      }
    }
    return out;
  };
  /** A turning head stands on land only (never on another street). */
  const headClear = (s: Segment) => headCells(s, s.ends[1] ? 1 : 0, W, H).every((c) => land[c] === 1);

  const accept = (cand: Cand, d: number): boolean => {
    const segs = cand.legs.map((l) => segOf(l, isDiagDir(l.dir) ? cand.width + 1 : cand.width, cand.style, d, RoadClass.Street));
    for (const s of segs) if (segSteps(s) < 2 || crowds(s)) {
      STATS.crowd++;
      return false;
    }
    return true;
  };

  // ---- the trunk: radial streets from the square's corners in the old core
  const core = w.districts.find((d) => d.kind === "core");
  if (core) {
    const g = GROWTH[core.type.streets.pattern] ?? (GROWTH.organic as Growth);
    if (g.diagonal.radial) {
      const r = rngOf(w, "radials", core.id);
      const corners = [
        { x: w.ring.x - 1, y: w.ring.y - 1, dir: 3 },
        { x: w.ring.x + w.ring.w, y: w.ring.y - 1, dir: 1 },
        { x: w.ring.x - 1, y: w.ring.y + w.ring.h, dir: 5 },
        { x: w.ring.x + w.ring.w, y: w.ring.y + w.ring.h, dir: 7 },
      ];
      for (const k of corners) {
        if (!r.chance(0.75)) continue;
        const [dx, dy] = DIR8[k.dir] as readonly [number, number];
        // Start a step out from the ring's corner so the band starts on land.
        const sx = k.x + dx;
        const sy = k.y + dy;
        if (bandStatus(sx, sy, k.dir, 4) !== 0) continue;
        const t = trace(sx, sy, k.dir, 4, 40);
        if (!t.hit || t.steps < 6) continue;
        const a = across(sx + dx * (t.steps - 1), sy + dy * (t.steps - 1), k.dir, 4);
        if (a.jog) continue;
        const cand: Cand = { legs: [{ x: k.x - dx, y: k.y - dy, dir: k.dir, steps: t.steps + 2 }], width: 3, style: "asphalt", end: null, score: 0 };
        if (!accept(cand, core.index)) continue;
        commit(cand, core.index);
      }
    }
  }

  /**
   * An access street to a district out of the streets' reach: from its seed
   * the cheapest way to the nearest street, over fields if it must and across
   * the railway square-on, in a few straight runs (turns cost); the seed's
   * end gets a turning head where the type has them (else growth or the end
   * pass joins it up).
   */
  const accessStreet = (d: WorkDistrict, g: Growth): Cand | null => {
    const wo = g.widths[g.widths.length - 1] as Growth["widths"][number];
    const width = Math.max(3, wo.width);
    const style = wo.style === "dirt" ? "dirt" : "asphalt";
    const lo = bandLo(width);
    // Fields may be crossed (not the map's edge), and the railway square-on.
    const passable = (x: number, y: number, dir: number): 0 | 1 | 2 => {
      let road = false;
      for (let k = 0; k < width; k++) {
        const bx = (dir & 3) === 0 ? x : x - lo + k;
        const by = (dir & 3) === 0 ? y - lo + k : y;
        if (bx < 3 || by < 3 || bx >= W - 3 || by >= H - 3) return 2;
        const c = by * W + bx;
        if (isRoad(c)) road = true;
        else if (railAx[c]) {
          if (!((railAx[c] === 1 && (dir & 3) === 2) || (railAx[c] === 2 && (dir & 3) === 0))) return 2;
        } else if (isWaterBody(w, c) || w.terrain[c] === Terrain.Rock || (w.use[c] as number) & (Use.Square | Use.Bridge | Use.Rail)) return 2;
        else if (!land[c] && (w.district[c] as number) >= 0) return 2;
      }
      return road ? 1 : 0;
    };
    let best: Cand | null = null;
    let bestLen = Infinity;
    const cs = cellsOf[d.index] as number[];
    for (let i = 0; i < cs.length; i += 3) {
      const c = cs[i] as number;
      if (!land[c] || skip[c]) continue;
      const x = c % W;
      const y = (c - x) / W;
      for (const dir of [0, 2, 4, 6]) {
        const [dx, dy] = DIR8[dir] as readonly [number, number];
        let hit = -1;
        for (let t = 1; t <= 70; t++) {
          const st = passable(x + dx * t, y + dy * t, dir);
          if (st === 2) break;
          if (st === 1) {
            hit = t;
            break;
          }
        }
        if (hit < 0) ACC.nohit++;
        if (hit < 3 || hit >= bestLen) continue;
        const ex = x + dx * (hit - 1);
        const ey = y + dy * (hit - 1);
        if (across(ex, ey, dir, width).jog) {
          ACC.jog++;
          continue;
        }
        // From the street reached, inward to the district (the far end is the district's).
        const leg: Leg = { x: x + dx * hit, y: y + dy * hit, dir: (dir + 4) & 7, steps: hit };
        if (crowds(segOf(leg, width, style, d.index, RoadClass.Street))) {
          ACC.crowd++;
          continue;
        }
        best = { legs: [leg], width, style, end: null, score: 0 };
        bestLen = hit;
      }
    }
    // No straight way: one bend (out of the district, then square-on to a street).
    for (let i = 0; i < cs.length && !best; i += 5) {
      const c = cs[i] as number;
      if (!land[c] || skip[c]) continue;
      const x = c % W;
      const y = (c - x) / W;
      for (const d1 of [0, 2, 4, 6]) {
        const [dx, dy] = DIR8[d1] as readonly [number, number];
        for (let k = 4; k <= 32; k += 4) {
          let clear = true;
          for (let t = 1; t <= k && clear; t++) if (passable(x + dx * t, y + dy * t, d1) !== 0) clear = false;
          if (!clear) break;
          const kx = x + dx * k;
          const ky = y + dy * k;
          for (const d2 of [(d1 + 2) & 7, (d1 + 6) & 7]) {
            const [ex, ey] = DIR8[d2] as readonly [number, number];
            let hit = -1;
            for (let t = 1; t <= 60; t++) {
              const st = passable(kx + ex * t, ky + ey * t, d2);
              if (st === 2) break;
              if (st === 1) {
                hit = t;
                break;
              }
            }
            if (hit < 3 || k + hit >= bestLen) continue;
            if (across(kx + ex * (hit - 1), ky + ey * (hit - 1), d2, width).jog) continue;
            const hi = width - 1 - bandLo(width);
            const legA: Leg = { x: kx + ex * hit, y: ky + ey * hit, dir: (d2 + 4) & 7, steps: hit + hi };
            const legB: Leg = { x: kx, y: ky, dir: (d1 + 4) & 7, steps: k };
            if (crowds(segOf(legA, width, style, d.index, RoadClass.Street)) || crowds(segOf(legB, width, style, d.index, RoadClass.Street))) continue;
            best = { legs: [legA, legB], width, style, end: null, score: 0 };
            bestLen = k + hit;
          }
        }
      }
    }
    if (best && g.culDeSac > 0) {
      const s = segOf(best.legs[best.legs.length - 1] as Leg, width, style, d.index, RoadClass.Street, g.head);
      if (paintHead(w, s, s.ends[1] ? 1 : 0, true) && headClear(s)) best.end = g.head;
    }
    return best;
  };

  // ---- growth, district by district from the core outward
  const order = [...w.districts].sort((a, b) => (GROW_ORDER[a.kind] ?? 9) - (GROW_ORDER[b.kind] ?? 9) || a.index - b.index);
  const cellsOf: number[][] = w.districts.map(() => []);
  // Targets stand two cells clear of the town's edge, water and the railway (a street through them fits).
  const skip = new Uint8Array(N);
  for (let c = 0; c < N; c++) {
    if (!land[c]) continue;
    const x = c % W;
    const y = (c - x) / W;
    let inner = x >= 2 && y >= 2 && x < W - 2 && y < H - 2;
    for (let yy = y - 2; yy <= y + 2 && inner; yy++) for (let xx = x - 2; xx <= x + 2 && inner; xx++) if (!land[yy * W + xx] && !isRoad(yy * W + xx)) inner = false;
    if (!inner) skip[c] = 1;
    if ((w.district[c] as number) >= 0) (cellsOf[w.district[c] as number] as number[]).push(c);
  }
  /** Targets stand two cells clear of the town's edge, water and the railway (and of the streets laid). */
  const resetSkip = (cells: number[]) => {
    for (const c of cells) {
      skip[c] = 0;
      const x = c % W;
      const y = (c - x) / W;
      for (let yy = y - 2; yy <= y + 2 && !skip[c]; yy++) for (let xx = x - 2; xx <= x + 2; xx++) if (xx < 0 || yy < 0 || xx >= W || yy >= H || (!land[yy * W + xx] && !isRoad(yy * W + xx))) skip[c] = 1;
    }
  };
  // Twice over: a district grown before its neighbours had streets is reached from them the second time.
  for (let pass = 0; pass < 2; pass++)
  for (const d of order) {
    const g = GROWTH[d.type.streets.pattern] ?? (GROWTH.grid as Growth);
    const r = rngOf(w, "grow", d.id, pass);
    let headsLeft = d.type.streets.max_dead_ends;
    const cells = cellsOf[d.index] as number[];
    const touched = () =>
      cells.some((c) => {
        const x = c % W;
        return (x > 0 && isRoad(c - 1)) || (x < W - 1 && isRoad(c + 1)) || (c >= W && isRoad(c - W)) || (c + W < N && isRoad(c + W));
      });
    if (pass > 0) {
      // Only a district part of which no street reached the first time (or that no street touches).
      const lim = Math.floor((g.maxRun * 2) / 3) * Math.max(cx[d.index] as number, cy[d.index] as number);
      if (!cells.some((c) => land[c] && (dist[c] as number) > lim) && touched()) continue;
      resetSkip(cells);
    }
    const guard = Math.floor(cells.length / 12) + 30;
    // The frontier: the deepest land a street can still reach (streets grow outward from those there).
    const reach = Math.floor((g.maxRun * 2) / 3) * Math.max(cx[d.index] as number, cy[d.index] as number);
    // A district no street can reach from the town (beyond fields, across the railway) gets an access street.
    const access = () => {
      const cand = accessStreet(d, g);
      (STATS as unknown as Record<string, unknown>)[`acc${d.index}p${pass}`] = cand ? JSON.stringify(cand.legs) : `null ${JSON.stringify(ACC)}`;
      if (!cand) return;
      crossingsLeft = Math.max(crossingsLeft, cand.legs.length);
      commit(cand, d.index);
      resetSkip(cells);
    };
    let grown = 0;
    let accessed = false;
    // (Or, the second time over, one no street touches yet.)
    if (!cells.some((c) => land[c] && (dist[c] as number) <= reach) || (pass > 0 && !touched())) {
      access();
      accessed = true;
    }
    for (let it = 0; it < guard; it++) {
      if (!accessed && !grown && it === 12) {
        access();
        accessed = true;
      }
      let best = 0;
      let nearFar = INF;
      for (const c of cells) {
        const dc = dist[c] as number;
        if (!land[c] || skip[c]) continue;
        if (dc > best && dc <= reach) best = dc;
        else if (dc > reach && dc < nearFar) nearFar = dc;
      }
      // Nothing in reach but land beyond it: the nearest of that land (a street reaches out to it).
      if (best <= COVERED && nearFar < INF) best = nearFar;
      if (best <= COVERED) break;
      const deep: number[] = [];
      for (const c of cells) if (land[c] && !skip[c] && (dist[c] as number) >= best - 150 && (dist[c] as number) <= best) deep.push(c);
      const target = deep[r.int(0, deep.length - 1)] as number;
      const tx = target % W;
      const ty = (target - tx) / W;
      const cands = candidates(d, g, tx, ty, r, headsLeft).sort((a, b) => b.score - a.score);
      STATS.cands += cands.length;
      if (!cands.length) STATS.none++;
      let done = false;
      for (const cand of cands.slice(0, 12)) {
        if (!accept(cand, d.index)) continue;
        commit(cand, d.index);
        if (cand.end) headsLeft--;
        done = true;
        STATS.ok++;
        grown++;
        break;
      }
      if (!done) STATS.fail++;
      if (!done) for (let yy = ty - 2; yy <= ty + 2; yy++) for (let xx = tx - 2; xx <= tx + 2; xx++) if (xx >= 0 && yy >= 0 && xx < W && yy < H) skip[yy * W + xx] = 1;
    }
    let inf = 0;
    let far = 0;
    let open = 0;
    let cov = 0;
    let sk = 0;
    for (const c of cells) {
      if (!land[c]) continue;
      const dc = dist[c] as number;
      if (skip[c]) sk++;
      else if (dc >= INF) inf++;
      else if (dc > reach) far++;
      else if (dc > COVERED) open++;
      else cov++;
    }
    (STATS as unknown as Record<string, string>)[`d${d.index}`] = `${d.type.streets.pattern} inf${inf} far${far} open${open} cov${cov} skip${sk}`;
  }
  w.timings["streets.grow"] = performance.now() - t0;

  // ---- every end joined (or a designed head): a stray end is run on to the street ahead, else cut back to its last junction
  const t1 = performance.now();
  if (!process.env.DEV_NORESOLVE) resolveEnds(w, cover, land, pre, preLevel);
  // ---- main streets: a district the arterials miss gets the chain of streets that joins it to one promoted
  promoteMainStreets(w);
  w.timings["streets.ends"] = performance.now() - t1;

  // ---- greens at junctions in the town's districts
  junctionGreens(w);

  // ---- quays along the water in waterfront districts with a quay pattern
  for (let c = 0; c < N; c++) {
    const d = w.district[c] as number;
    if (d < 0 || (w.districts[d] as WorkDistrict).type.streets.pattern !== "quay") continue;
    if ((w.use[c] as number) & (Use.Road | Use.Square | Use.Rail) || isWaterBody(w, c) || w.water[c] === WaterKind.Creek) continue;
    const near = Math.min(w.seaDist[c] as number, w.riverDist[c] as number);
    if (near >= 1 && near <= 2) {
      w.terrain[c] = Terrain.Quay;
      w.use[c] = (w.use[c] as number) | Use.Road;
      w.roadClass[c] = RoadClass.Lane;
    }
  }
}

/** Dev counters (scratch). */
export const STATS = { ok: 0, fail: 0, none: 0, crowd: 0, cands: 0 };
export const ACC = { nohit: 0, jog: 0, crowd: 0 };

const GROW_ORDER: Record<string, number> = { core: 0, inner: 1, waterfront: 2, industrial: 3, suburb: 4, rural: 5 };

/** The nearest of the eight directions to a vector, as an axis-defining direction 0..3 (east, north-east, north, north-west). */
export function snap8(vx: number, vy: number): number {
  const ax = Math.abs(vx);
  const ay = Math.abs(vy);
  // tan(22.5°) ≈ 0.4142: 70/169.
  if (ay * 169 <= ax * 70) return 0;
  if (ax * 169 <= ay * 70) return 2;
  return (vx > 0) === (vy < 0) ? 1 : 3;
}

/**
 * How a run's end (0: a, 1: b) meets the town: another run ("joined": a
 * shared band, the band just beyond, or a street at the end's side), the
 * square ("square"), the map's edge ("edge"), or nothing ("open").
 */
export function endState(s: Segment, end: 0 | 1, W: number, H: number, cover: (c: number) => number, square: (c: number) => boolean): "joined" | "square" | "edge" | "open" {
  const L = segSteps(s);
  const t = end === 0 ? 0 : L;
  const out = end === 0 ? -1 : 1;
  let joined = false;
  let sq = false;
  let edge = false;
  forBand(s, t, t, (x, y) => {
    if (x < 0 || y < 0 || x >= W || y >= H) return;
    if (cover(y * W + x) >= 2) joined = true;
  });
  forBand(s, t + out, t + out, (x, y) => {
    if (x < 0 || y < 0 || x >= W || y >= H) {
      edge = true;
      return;
    }
    const c = y * W + x;
    if (cover(c) >= 1) joined = true;
    else if (square(c)) sq = true;
  });
  if (!joined && !edge) {
    // A street met at the end's side (a corner joint, or a T onto the end): a neighbour of the end's band the run itself does not cover.
    const self = new Set<number>();
    forBand(s, Math.min(t, t - 2 * out), Math.max(t, t - 2 * out), (x, y) => {
      if (x >= 0 && y >= 0 && x < W && y < H) self.add(y * W + x);
    });
    forBand(s, t, t, (x, y) => {
      for (const [nx, ny] of [
        [x - 1, y],
        [x + 1, y],
        [x, y - 1],
        [x, y + 1],
      ] as const) {
        if (nx < 0 || ny < 0 || nx >= W || ny >= H) continue;
        const n = ny * W + nx;
        if (self.has(n)) continue;
        if (cover(n) >= 1) joined = true;
        else if (square(n)) sq = true;
      }
    });
  }
  return joined ? "joined" : sq ? "square" : edge ? "edge" : "open";
}

/**
 * Every end joins a street or ends in a designed termination (DEC-090 street
 * ends). An open end is run on to the street ahead when one is close (a
 * paved lane in the old core becomes a footpath behind bollards instead);
 * else, by what it meets: a railing at the water, a field gate at the
 * countryside, a turning head where the district type has cul-de-sacs;
 * else it is cut back to its last junction (a run with none goes).
 */
function resolveEnds(w: Work, cover: Uint16Array, land: Uint8Array, pre: Uint8Array, preLevel: Uint8Array) {
  const { W, H } = w;
  const sq = (c: number) => ((w.use[c] as number) & Use.Square) !== 0;
  const cov = (c: number) => cover[c] as number;
  const inMap = (x: number, y: number) => x >= 0 && y >= 0 && x < W && y < H;
  const unpaint = (s: Segment, t0: number, t1: number) => {
    forBand(s, t0, t1, (x, y) => {
      if (!inMap(x, y)) return;
      const c = y * W + x;
      cover[c] = Math.max(0, (cover[c] as number) - 1);
      if ((cover[c] as number) > 0 || (w.use[c] as number) & (Use.Bridge | Use.Rail)) return;
      w.terrain[c] = pre[c] as number;
      w.level[c] = preLevel[c] as number;
      w.use[c] = (w.use[c] as number) & ~Use.Road;
      w.roadClass[c] = RoadClass.None;
      if ((w.district[c] as number) >= 0 && !isWaterBody(w, c)) land[c] = 1;
    });
  };
  const free = (c: number) => land[c] === 1;
  const prop = (kind: "bollards" | "railing" | "field_gate", x: number, y: number) => w.props.push({ kind, x, y, blocks: false, variant: mix(w.seed, x, y, 0xe7d) & 0xff });
  for (let round = 0; round < 8; round++) {
    let changed = false;
    for (let i = 0; i < w.segments.length; i++) {
      const s = w.segments[i] as Segment;
      if (s.arterial >= 0 && w.arterials[s.arterial]?.mode !== "local") continue;
      if (s.district < 0 && s.arterial < 0) continue;
      for (const end of [0, 1] as const) {
        if (s.ends[end]) continue;
        const L = segSteps(s);
        if (L < 0) continue;
        const st = endState(s, end, W, H, cov, sq);
        if (st === "joined") continue;
        if (st === "square") {
          // The paving runs into the square.
          s.ends[end] = "plaza";
          continue;
        }
        const [sx, sy] = stepOf(s.axis);
        const out = end === 0 ? -1 : 1;
        const t = end === 0 ? 0 : L;
        const ex = end === 0 ? s.ax : s.bx;
        const ey = end === 0 ? s.ay : s.by;
        // ---- run on: up to 12 steps of land to a street
        let reach = -1;
        if (st !== "edge") {
          for (let k = 1; k <= 12 && reach < 0; k++) {
            let ok = true;
            let hit = false;
            forBand(s, t + out * k, t + out * k, (x, y) => {
              if (!inMap(x, y)) {
                ok = false;
                return;
              }
              const c = y * W + x;
              if (cov(c) >= 1 || sq(c)) hit = true;
              else if (!free(c)) ok = false;
            });
            if (hit) reach = k - 1;
            else if (!ok) break;
          }
        }
        if (reach >= 0 && s.style === "lane" && reach >= 3) {
          // A paved lane in the old core: bollards across its end, a footpath on to the street ahead.
          forBand(s, t, t, (x, y) => {
            if (inMap(x, y)) prop("bollards", x, y);
          });
          for (let k = 1; k <= reach + 1; k++) {
            const x = ex + sx * out * k;
            const y = ey + sy * out * k;
            if (!inMap(x, y)) break;
            const c = y * W + x;
            if (cov(c) >= 1 || sq(c)) break;
            w.terrain[c] = Terrain.Path;
            w.use[c] = (w.use[c] as number) | Use.Lane | Use.End;
            land[c] = 0;
          }
          s.ends[end] = "bollards";
          changed = true;
          continue;
        }
        if (reach >= 0) {
          const grown: Segment = withBoxOf({ ...s, ends: [null, null] });
          if (end === 0) {
            grown.ax = s.ax - sx * (reach + 1);
            grown.ay = s.ay - sy * (reach + 1);
          } else {
            grown.bx = s.bx + sx * (reach + 1);
            grown.by = s.by + sy * (reach + 1);
          }
          if (crowded(w, withBoxOf(grown), s)) reach = -1;
        }
        if (reach >= 0) {
          const e: Segment = { ...s, ends: [null, null] };
          if (end === 0) {
            e.ax = s.ax - sx * (reach + 1);
            e.ay = s.ay - sy * (reach + 1);
            e.bx = s.ax - sx;
            e.by = s.ay - sy;
          } else {
            e.ax = s.bx + sx;
            e.ay = s.by + sy;
            e.bx = s.bx + sx * (reach + 1);
            e.by = s.by + sy * (reach + 1);
          }
          paintSegment(w, withBoxOf(e));
          forBand(e, 0, segSteps(e), (x, y) => {
            if (inMap(x, y)) {
              cover[y * W + x] = (cover[y * W + x] as number) + 1;
              land[y * W + x] = 0;
            }
          });
          if (end === 0) {
            s.ax = e.ax;
            s.ay = e.ay;
          } else {
            s.bx = e.bx;
            s.by = e.by;
          }
          withBoxOf(s);
          changed = true;
          continue;
        }
        // ---- a termination by what the end meets
        if (st !== "edge") {
          let water = false;
          let country = false;
          for (let k = 1; k <= 2; k++) {
            forBand(s, t + out * k, t + out * k, (x, y) => {
              if (!inMap(x, y)) return;
              const c = y * W + x;
              if (isWaterBody(w, c)) water = true;
              else if (k === 1 && (w.district[c] as number) < 0 && !((w.use[c] as number) & (Use.Rail | Use.Road))) country = true;
            });
          }
          if (water) {
            forBand(s, t, t, (x, y) => {
              if (inMap(x, y)) prop("railing", x, y);
            });
            s.ends[end] = "railing";
            changed = true;
            continue;
          }
          const gx = ex + sx * out;
          const gy = ey + sy * out;
          if (country && inMap(gx, gy) && !w.use[gy * W + gx] && !isWaterBody(w, gy * W + gx) && w.terrain[gy * W + gx] !== Terrain.Rock) {
            prop("field_gate", gx, gy);
            w.use[gy * W + gx] = (w.use[gy * W + gx] as number) | Use.End;
            land[gy * W + gx] = 0;
            s.ends[end] = "field_gate";
            changed = true;
            continue;
          }
          // A turning head where the district type has cul-de-sacs.
          const g = s.district >= 0 ? GROWTH[(w.districts[s.district] as WorkDistrict).type.streets.pattern] : undefined;
          if (g && g.culDeSac > 0 && L >= 4) {
            s.ends[end] = g.head;
            const cells = headCells(s, end, W, H);
            const self = new Set<number>();
            segCells(s, W, H, (c) => self.add(c));
            if (paintHead(w, s, end, true) && cells.every((c) => land[c] === 1 || self.has(c))) {
              paintHead(w, s, end);
              for (const c of cells) {
                cover[c] = (cover[c] as number) + 1;
                land[c] = 0;
              }
              changed = true;
              continue;
            }
            s.ends[end] = null;
          }
        }
        // ---- cut back to the last junction along the run (a shared band, a street at its side, or the far end meeting a street)
        let keep = -1;
        const dirIn = end === 0 ? 1 : -1;
        const shared = (x: number, y: number) => inMap(x, y) && (cover[y * W + x] as number) >= 2;
        const beside = (x: number, y: number) => inMap(x, y) && ((cover[y * W + x] as number) >= 1 || sq(y * W + x));
        for (let k = 0; k <= L && keep < 0; k++) {
          const p = t + dirIn * k;
          forBand(s, p, p, (x, y, kk) => {
            if (shared(x, y)) keep = p;
            if (kk === 0 || kk === s.width - 1) {
              const o = kk === 0 ? -1 : 1;
              if (beside(s.axis === "h" ? x : x + o, s.axis === "h" ? y + o : y)) keep = p;
            }
          });
          if (keep < 0 && k === L) {
            forBand(s, p + dirIn, p + dirIn, (x, y) => {
              if (beside(x, y)) keep = p;
            });
          }
        }
        if (keep < 0 || Math.abs(keep - t) >= L - 1 || L - Math.abs(keep - t) + 1 < 3) {
          // No junction: the whole run goes (its other end's termination with it).
          unpaint(s, 0, L);
          const other = s.ends[end === 0 ? 1 : 0];
          if (isHead(other)) {
            for (const c of headCells(s, end === 0 ? 1 : 0, W, H)) {
              cover[c] = Math.max(0, (cover[c] as number) - 1);
              if ((cover[c] as number) > 0) continue;
              w.terrain[c] = pre[c] as number;
              w.level[c] = preLevel[c] as number;
              w.use[c] = (w.use[c] as number) & ~Use.Road;
              w.roadClass[c] = RoadClass.None;
            }
          }
          s.ends = [null, null];
          s.bx = s.ax - sx;
          s.by = s.ay - sy;
          changed = true;
          break;
        }
        if (end === 0) {
          unpaint(s, 0, keep - 1);
          s.ax += sx * keep;
          s.ay += sy * keep;
        } else {
          unpaint(s, keep + 1, L);
          s.bx = s.ax + sx * keep;
          s.by = s.ay + sy * keep;
        }
        withBoxOf(s);
        changed = true;
      }
    }
    // Runs cut to nothing go.
    for (let i = w.segments.length - 1; i >= 0; i--) if (segSteps(w.segments[i] as Segment) < 0) w.segments.splice(i, 1);
    if (!changed) break;
  }
}

function withBoxOf(s: Segment): Segment {
  const lo = bandLo(s.width);
  const hi = s.width - 1 - lo;
  if (s.axis === "h") {
    s.x0 = s.ax;
    s.x1 = s.bx;
    s.y0 = s.ay - lo;
    s.y1 = s.ay + hi;
  } else if (s.axis === "v") {
    s.y0 = s.ay;
    s.y1 = s.by;
    s.x0 = s.ax - lo;
    s.x1 = s.ax + hi;
  } else {
    s.x0 = s.ax - lo;
    s.x1 = s.bx + hi;
    s.y0 = Math.min(s.ay, s.by);
    s.y1 = Math.max(s.ay, s.by);
  }
  return s;
}

/**
 * Every district touches a main road (§5 stage 5): where the arterials miss
 * one, the chain of streets that joins it to the nearest main road the
 * shortest way becomes its main street (no new road is laid).
 */
function promoteMainStreets(w: Work) {
  const { W, H } = w;
  const N = W * H;
  const S = w.segments.length;
  // Each cell's first and last run (a junction's cells are shared), heads counted as their run's.
  const first = new Int32Array(N).fill(-1);
  const last = new Int32Array(N).fill(-1);
  const segAt: number[][] = Array.from({ length: S }, () => []);
  w.segments.forEach((s, i) => {
    const put = (c: number) => {
      if ((first[c] as number) < 0) first[c] = i;
      last[c] = i;
      (segAt[i] as number[]).push(c);
    };
    segCells(s, W, H, put);
    for (const end of [0, 1] as const) if (isHead(s.ends[end])) for (const c of headCells(s, end, W, H)) put(c);
  });
  // Runs that touch: through shared or neighbouring cells.
  const adj: Set<number>[] = Array.from({ length: S }, () => new Set());
  for (let i = 0; i < S; i++) {
    for (const c of segAt[i] as number[]) {
      const x = c % W;
      for (const n of [c, x > 0 ? c - 1 : -1, x < W - 1 ? c + 1 : -1, c - W, c + W]) {
        if (n < 0 || n >= N) continue;
        for (const o of [first[n] as number, last[n] as number]) {
          if (o >= 0 && o !== i) {
            (adj[i] as Set<number>).add(o);
            (adj[o] as Set<number>).add(i);
          }
        }
      }
    }
  }
  const isMain = (s: Segment) => s.cls === RoadClass.Arterial || (s.district < 0 && s.arterial < 0);
  const mainCell = new Uint8Array(N);
  w.segments.forEach((s, i) => {
    if (isMain(s)) for (const c of segAt[i] as number[]) mainCell[c] = 1;
  });
  for (const d of w.districts) {
    let touches = false;
    for (let c = 0; c < N && !touches; c++) {
      if (w.district[c] !== d.index) continue;
      const x = c % W;
      for (const n of [c, x > 0 ? c - 1 : -1, x < W - 1 ? c + 1 : -1, c - W, c + W]) if (n >= 0 && n < N && mainCell[n]) touches = true;
    }
    if (touches) continue;
    // Breadth-first over the runs from the main roads to a run that touches the district.
    const prev = new Int32Array(S).fill(-2);
    const q: number[] = [];
    w.segments.forEach((s, i) => {
      if (isMain(s)) {
        prev[i] = -1;
        q.push(i);
      }
    });
    const touchesD = (i: number) => {
      for (const c of segAt[i] as number[]) {
        const x = c % W;
        for (const n of [c, x > 0 ? c - 1 : -1, x < W - 1 ? c + 1 : -1, c - W, c + W]) if (n >= 0 && n < N && w.district[n] === d.index) return true;
      }
      return false;
    };
    let hit = -1;
    for (let qi = 0; qi < q.length && hit < 0; qi++) {
      const i = q[qi] as number;
      for (const o of adj[i] as Set<number>) {
        if ((prev[o] as number) !== -2) continue;
        prev[o] = i;
        if (touchesD(o)) {
          hit = o;
          break;
        }
        q.push(o);
      }
    }
    (STATS as unknown as Record<string, unknown>)[`prom${d.index}`] = `${d.name} hit ${hit} touching ${w.segments.map((_, i) => i).filter((i) => touchesD(i)).length} reach ${[...prev].filter((p) => p !== -2).length}`;
    for (let i = hit; i >= 0 && !isMain(w.segments[i] as Segment); i = prev[i] as number) {
      const s = w.segments[i] as Segment;
      s.cls = RoadClass.Arterial;
      for (const c of segAt[i] as number[]) {
        mainCell[c] = 1;
        if ((w.use[c] as number) & Use.Road) w.roadClass[c] = RoadClass.Arterial;
      }
    }
  }
  void H;
}

/** Small greens in the inside corners of junctions (organic, grid and curvy districts), a few per district. */
function junctionGreens(w: Work) {
  const { W, H } = w;
  const road = (x: number, y: number) => x >= 0 && y >= 0 && x < W && y < H && ((w.use[y * W + x] as number) & Use.Road) !== 0;
  const per: Record<string, number> = { organic: 2, grid: 2, curvy: 3, contour: 2 };
  const made: { x: number; y: number }[] = [];
  const count = new Int32Array(w.districts.length);
  const r = rngOf(w, "greens");
  for (let y = 1; y < H - 1; y++) {
    for (let x = 1; x < W - 1; x++) {
      const c = y * W + x;
      const d = w.district[c] as number;
      if (d < 0 || (w.use[c] as number) || isWaterBody(w, c) || w.terrain[c] !== Terrain.Grass) continue;
      const dd = w.districts[d] as WorkDistrict;
      if ((count[d] as number) >= (per[dd.type.streets.pattern] ?? 0)) continue;
      // An inside corner: streets on two adjacent sides.
      const rx = road(x - 1, y) ? 1 : road(x + 1, y) ? -1 : 0;
      const ry = road(x, y - 1) ? 1 : road(x, y + 1) ? -1 : 0;
      if (!rx || !ry || !road(x - rx, y - ry)) continue;
      if (made.some((m) => Math.abs(m.x - x) + Math.abs(m.y - y) < 16)) continue;
      if (!r.chance(0.08)) continue;
      const sw = 3 + (r.chance(0.5) ? 1 : 0);
      const sh = 3;
      const cells: number[] = [];
      let ok = true;
      for (let j = 0; j < sh && ok; j++) {
        for (let i = 0; i < sw && ok; i++) {
          const xx = x + rx * i;
          const yy = y + ry * j;
          if (xx < 0 || yy < 0 || xx >= W || yy >= H) {
            ok = false;
            break;
          }
          const cc = yy * W + xx;
          if (w.district[cc] !== d || w.use[cc] || w.terrain[cc] !== Terrain.Grass || w.level[cc] !== w.level[c]) ok = false;
          cells.push(cc);
        }
      }
      if (!ok) continue;
      for (const cc of cells) {
        w.terrain[cc] = Terrain.Park;
        w.use[cc] = (w.use[cc] as number) | Use.Keep;
      }
      made.push({ x, y });
      count[d] = (count[d] as number) + 1;
    }
  }
}

/** The street direction codes of the plan's runs (after every stage that paints streets). */
export function stageStreetAxes(w: Work) {
  const isStreet = (c: number) => {
    const t = w.terrain[c] as number;
    return t === Terrain.Road || t === Terrain.Pavement || t === Terrain.Crossing || t === Terrain.Dirt || t === Terrain.Path || t === Terrain.Bridge;
  };
  w.axis = streetAxes(w.W, w.H, w.segments, isStreet);
}
