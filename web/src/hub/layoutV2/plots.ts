// Layout v2 stages 7–12 (TOWN_GENERATION.md §5): blocks and plots (rows of
// buildings fronting the street below them, as every door faces south),
// places in slots by the place registry's placement rules (civic buildings
// on the square, the church on high ground, the mine at the edge toward the
// minerals with its headframe and spoil heap, the port at the water with a
// pier), filler buildings by each district type's mix, fill and decline
// ranks with reserved land, props, and the checks that make every door
// reachable.
import { mix } from "../rng";
import { Terrain, placeSize, type PropKind } from "../townLayout";
import { MinHeap, NO_BEARING, compassVec, isqrt, valueNoise } from "./imath";
import type { DistrictType, PlaceLayout } from "./registry";
import { LayoutOverflow, RoadClass, Use, WaterKind, buildable, distanceField, frontageTerrain, isWaterBody, rngOf, type PlanPlot, type Work, type WorkDistrict } from "./work";

const CIVIC = ["town_hall", "council_chamber", "parliament", "regional_assembly", "executive_residence"];

/** Per-run masks of door fronts and roof rows (a roof never covers a doorstep, §13). */
interface Masks {
  fronts: Uint8Array;
  roofs: Uint8Array;
}

function masks(w: Work): Masks {
  const m = (w as Work & { _masks?: Masks })._masks;
  if (m) return m;
  const n: Masks = { fronts: new Uint8Array(w.W * w.H), roofs: new Uint8Array(w.W * w.H) };
  (w as Work & { _masks?: Masks })._masks = n;
  return n;
}

const GROUND: Record<string, number> = { yard: Terrain.Yard, grass: Terrain.Grass, field: Terrain.Field, garden: Terrain.Garden };

// ---------------------------------------------------------------------------
// Stage 7a: ground of the blocks, small blocks as green, back lanes

export function stageGround(w: Work) {
  const { W, H } = w;
  const N = W * H;
  const fieldNoise = (w.seed ^ 0xf1e1d) >>> 0;
  for (let c = 0; c < N; c++) {
    if (w.terrain[c] !== Terrain.Grass || w.use[c]) continue;
    const d = w.district[c] as number;
    if (d >= 0) w.terrain[c] = GROUND[(w.districts[d] as WorkDistrict).type.plots.ground] ?? Terrain.Grass;
    else if (valueNoise(fieldNoise, c % W, Math.floor(c / W), 9) > 640 && w.geo.resources_pm.fertility !== undefined && (w.geo.resources_pm.fertility ?? 0) > 150) w.terrain[c] = Terrain.Field;
  }
  // Blocks too small for a row of buildings (§8: minimum 6 × 5) become green.
  const seen = new Uint8Array(N);
  const q = new Int32Array(N);
  for (let s = 0; s < N; s++) {
    if (seen[s] || !buildable(w.terrain[s] as number) || (w.district[s] as number) < 0 || w.use[s]) continue;
    const d = w.district[s] as number;
    let qt = 0;
    q[qt++] = s;
    seen[s] = 1;
    let x0 = W;
    let x1 = -1;
    let y0 = H;
    let y1 = -1;
    for (let qh = 0; qh < qt; qh++) {
      const c = q[qh] as number;
      const x = c % W;
      const y = Math.floor(c / W);
      if (x < x0) x0 = x;
      if (x > x1) x1 = x;
      if (y < y0) y0 = y;
      if (y > y1) y1 = y;
      for (let k4 = 0; k4 < 4; k4++) {
        const n = k4 === 0 ? (x > 0 ? c - 1 : -1) : k4 === 1 ? (x < W - 1 ? c + 1 : -1) : k4 === 2 ? c - W : c + W;
        if (n < 0 || n >= N || seen[n] || w.district[n] !== d || w.use[n] || !buildable(w.terrain[n] as number)) continue;
        seen[n] = 1;
        q[qt++] = n;
      }
    }
    if (x1 - x0 + 1 >= 6 && y1 - y0 + 1 >= 5 && qt >= 30) continue;
    const kind = (w.districts[d] as WorkDistrict).kind;
    for (let i = 0; i < qt; i++) {
      const c = q[i] as number;
      if (kind === "core" || kind === "inner" || kind === "suburb" || kind === "waterfront") w.terrain[c] = Terrain.Park;
      w.use[c] = (w.use[c] as number) | Use.Keep;
    }
  }
}

/**
 * Back lanes through deep blocks (district types with lanes): a path behind
 * the first row of buildings, so a second row can front it (doors face
 * south). The lane sits far enough up that the lower row's roofs stay off it.
 */
export function stageLanes(w: Work) {
  const { W } = w;
  const N = W * w.H;
  const walkable = (cc: number) => cc >= 0 && cc < N && ((w.use[cc] as number) & Use.Road) !== 0;
  const land = (cc: number, d: number) => cc >= 0 && w.district[cc] === d && !w.use[cc] && buildable(w.terrain[cc] as number);
  for (let F = 9; F < w.H; F++) {
    let x = 0;
    while (x < W) {
      const c = F * W + x;
      const d = w.district[c - W] as number;
      if (d < 0 || !frontageTerrain(w.terrain[c] as number) || (w.districts[d] as WorkDistrict).type.streets.lanes === "none" || !land(c - W, d)) {
        x++;
        continue;
      }
      // Depth of buildable land above this frontage cell.
      const depth = (xx: number) => {
        let k = 0;
        while (k < 14 && land((F - 1 - k) * W + xx, d)) k++;
        return k;
      };
      let b = x;
      let D = depth(x);
      while (b + 1 < W && frontageTerrain(w.terrain[F * W + b + 1] as number) && land((F - 1) * W + b + 1, d)) {
        D = Math.min(D, depth(b + 1));
        b++;
      }
      if (b - x + 1 >= 8 && D >= 11) {
        const L = F - 6;
        let lo = x;
        let hi = b;
        while (lo > x - 3 && lo > 0 && !walkable(L * W + lo - 1) && land(L * W + lo - 1, d)) lo--;
        while (hi < b + 3 && hi < W - 1 && !walkable(L * W + hi + 1) && land(L * W + hi + 1, d)) hi++;
        if (walkable(L * W + lo - 1) && walkable(L * W + hi + 1)) {
          const kind = (w.districts[d] as WorkDistrict).kind;
          for (let xx = lo; xx <= hi; xx++) {
            const cc = L * W + xx;
            w.terrain[cc] = kind === "core" ? Terrain.Pavement : kind === "industrial" ? Terrain.Yard : Terrain.Path;
            w.use[cc] = (w.use[cc] as number) | Use.Road | Use.Lane;
            w.roadClass[cc] = RoadClass.Lane;
          }
        }
      }
      x = b + 1;
    }
  }
}

// ---------------------------------------------------------------------------
// Fitting plots

interface Fit {
  x: number;
  F: number;
  w: number;
  h: number;
  s: number;
}

/** Whether a footprint fits with its front on row F (the frontage) and `s` rows of front garden. */
function fits(w: Work, d: number, x: number, F: number, bw: number, bh: number, s: number, m: Masks, allowAnyDistrict = false): boolean {
  const { W, H } = w;
  const bottom = F - 1 - s;
  const top = bottom - bh + 1;
  if (top < 1 || x < 0 || x + bw > W || F >= H) return false;
  const lv = w.level[bottom * W + x] as number;
  for (let y = top; y <= F - 1; y++) {
    for (let xx = x; xx < x + bw; xx++) {
      const c = y * W + xx;
      if ((!allowAnyDistrict && w.district[c] !== d) || w.use[c] || !buildable(w.terrain[c] as number) || w.level[c] !== lv || (y <= F - 1 - s && m.roofs[c])) return false;
    }
  }
  // Every column has a frontage below (a door may be anywhere along the front), or, beside a north–south or
  // 45° street, a forecourt to pave (an apron): the building steps along the street, its front a short
  // paved strip that reaches the street's side within a few cells.
  let apron = false;
  let front = false;
  for (let xx = x; xx < x + bw; xx++) {
    const f = F * W + xx;
    if (m.roofs[f - s * W] || w.water[f] !== WaterKind.None) return false;
    if (frontageTerrain(w.terrain[f] as number)) {
      front = true;
      continue;
    }
    if (s > 0) continue;
    if (!apronable(w, f, lv, m)) return false;
    apron = true;
  }
  const door = F * W + x + (bw >> 1);
  if (!frontageTerrain(w.terrain[door] as number) && !(apron && s === 0)) return false;
  if (apron && !front && apronReach(w, x, F, bw, lv, m) < 0) return false;
  // The roof row: never over a doorstep or another plot.
  for (let xx = x; xx < x + bw; xx++) {
    const c = (top - 1) * W + xx;
    if (m.fronts[c] || (w.use[c] as number) & Use.Plot) return false;
  }
  return true;
}

/** A cell a forecourt may be paved on: open buildable land on the building's level, under no roof. */
function apronable(w: Work, c: number, lv: number, m: Masks): boolean {
  return (w.district[c] as number) >= 0 && !w.use[c] && buildable(w.terrain[c] as number) && w.level[c] === lv && !m.roofs[c] && !w.blocked[c] && w.water[c] === WaterKind.None;
}

/** The side of a street: a paved or road cell of a street run (or the square). */
function streetSide(w: Work, c: number): boolean {
  const t = w.terrain[c] as number;
  return (((w.use[c] as number) & Use.Road) !== 0 && (t === Terrain.Road || t === Terrain.Pavement || t === Terrain.Dirt)) || t === Terrain.Plaza;
}

/** Where a forecourt along row F from the footprint [x, x + bw) meets a street within three cells (its x), or -1. */
function apronReach(w: Work, x: number, F: number, bw: number, lv: number, m: Masks): number {
  const { W } = w;
  for (const dir of [-1, 1]) {
    for (let k = 1; k <= 3; k++) {
      const xx = dir < 0 ? x - k : x + bw - 1 + k;
      if (xx < 0 || xx >= W) break;
      const c = F * W + xx;
      if (streetSide(w, c)) return xx;
      if (!apronable(w, c, lv, m)) break;
    }
  }
  return -1;
}

function addPlot(w: Work, d: number, f: Fit, spec: { place: number | null; kind: string; name: string; landmark: boolean; ground: number }, doorAt?: number): number {
  const { W } = w;
  const m = masks(w);
  const bottom = f.F - 1 - f.s;
  const top = bottom - f.h + 1;
  if (f.s === 0 && f.F < w.H) {
    // A forecourt: the cells under the front not yet paved, and the gap to the street beside.
    let any = false;
    for (let x = f.x; x < f.x + f.w; x++) if (frontageTerrain(w.terrain[f.F * W + x] as number)) any = true;
    const lv = w.level[bottom * W + f.x] as number;
    let lo = f.x;
    let hi = f.x + f.w - 1;
    if (!any) {
      const sx = apronReach(w, f.x, f.F, f.w, lv, m);
      if (sx >= 0 && sx < f.x) lo = sx + 1;
      if (sx >= 0 && sx > hi) hi = sx - 1;
    }
    const kind = (w.districts[d] as WorkDistrict | undefined)?.kind ?? "inner";
    const t = kind === "rural" || kind === "suburb" ? Terrain.Path : Terrain.Pavement;
    for (let x = lo; x <= hi; x++) {
      const c = f.F * W + x;
      if (frontageTerrain(w.terrain[c] as number) || (w.use[c] as number) & (Use.Road | Use.Square) || !buildable(w.terrain[c] as number) || w.water[c] !== WaterKind.None) continue;
      w.terrain[c] = t;
      w.use[c] = (w.use[c] as number) | Use.Keep;
    }
  }
  const r = rngOf(w, "door", f.x, top);
  const doorX = doorAt ?? (f.w >= 3 ? f.x + Math.floor(f.w / 2) - (f.w % 2 === 0 && r.chance(0.5) ? 1 : 0) : f.x);
  const plot: PlanPlot = {
    x: f.x,
    y: top,
    w: f.w,
    h: f.h,
    doorX,
    front: (bottom + 1) * W + doorX,
    district: d,
    place: spec.place,
    kind: spec.kind,
    name: spec.name,
    landmark: spec.landmark,
    fillRank: 0,
    declineRank: 0,
    ground: spec.ground,
    setback: f.s,
  };
  for (let y = top; y <= f.F - 1; y++) for (let x = f.x; x < f.x + f.w; x++) w.use[y * W + x] = (w.use[y * W + x] as number) | Use.Plot;
  for (let x = f.x; x < f.x + f.w; x++) if (top - 1 >= 0) m.roofs[(top - 1) * W + x] = 1;
  m.fronts[plot.front] = 1;
  w.use[plot.front] = (w.use[plot.front] as number) | Use.Keep;
  w.plots.push(plot);
  return w.plots.length - 1;
}

/** Frontage cells of a district: (x, F) where row F is walkable frontage and the cell above is buildable land of the district. */
function frontageOf(w: Work, d: number, any = false): number[] {
  const { W, H } = w;
  if (!any) {
    // One pass over the map for every district (kept for the stage; fits() rechecks what plots have taken since).
    const ix = (w as Work & { _front?: number[][] })._front ?? frontageIndex(w);
    (w as Work & { _front?: number[][] })._front = ix;
    return (ix[d] ?? []).filter((c) => !w.use[c - W] && buildable(w.terrain[c - W] as number) && (frontageTerrain(w.terrain[c] as number) || (!w.use[c] && buildable(w.terrain[c] as number))));
  }
  const out: number[] = [];
  for (let F = 2; F < H; F++) {
    for (let x = 0; x < W; x++) {
      const c = F * W + x;
      const up = c - W;
      if (!frontageTerrain(w.terrain[c] as number)) continue;
      if ((!any && w.district[up] !== d) || w.use[up] || !buildable(w.terrain[up] as number)) continue;
      out.push(c);
    }
  }
  return out;
}

/** Frontage cells of every district, in one pass (row-major order, as `frontageOf` gives them). */
function frontageIndex(w: Work): number[][] {
  const { W, H } = w;
  const out: number[][] = w.districts.map(() => []);
  // A forecourt cell: open land beside a street's side in its row (the building above steps along the street).
  const nearSide = (c: number) => {
    const x = c % W;
    for (let k = 1; k <= 4; k++) {
      if (x - k >= 0 && streetSide(w, c - k)) return true;
      if (x + k < W && streetSide(w, c + k)) return true;
    }
    return false;
  };
  for (let c = 2 * W; c < W * H; c++) {
    const d = w.district[c - W] as number;
    if (d < 0) continue;
    const t = w.terrain[c] as number;
    if (!frontageTerrain(t) && !(buildable(t) && !w.use[c] && nearSide(c))) continue;
    (out[d] as number[]).push(c);
  }
  return out;
}

/** Forget the frontage index (the streets changed). */
function resetFrontage(w: Work) {
  delete (w as Work & { _front?: number[][] })._front;
}

// ---------------------------------------------------------------------------
// Stage 8: places and landmarks

interface Scorer {
  (x: number, y: number, bw: number, bh: number): number;
}

function placeScorer(w: Work, preds: string[], d: number, nearKind: (k: string) => { x: number; y: number } | null): Scorer {
  const { W } = w;
  const cx = w.centre.x;
  const cy = w.centre.y;
  const dist = (x: number, y: number) => isqrt((x - cx) * (x - cx) + (y - cy) * (y - cy));
  const wd = w.districts[d] as WorkDistrict | undefined;
  const dc = wd ? { x: wd.seed % W, y: Math.floor(wd.seed / W) } : { x: cx, y: cy };
  const arterialDist = (w as Work & { _artDist?: Uint16Array })._artDist ?? distanceField(W, w.H, (c) => w.roadClass[c] === RoadClass.Arterial, (c) => !isWaterBody(w, c), 30);
  (w as Work & { _artDist?: Uint16Array })._artDist = arterialDist;
  const railDist = w.railCells.length ? ((w as Work & { _railDist?: Uint16Array })._railDist ?? distanceField(W, w.H, (c) => ((w.use[c] as number) & Use.Rail) !== 0, () => true, 40)) : null;
  if (railDist) (w as Work & { _railDist?: Uint16Array })._railDist = railDist;
  return (x, y, bw, bh) => {
    const ax = x + (bw >> 1);
    const ay = y + (bh >> 1);
    const c = ay * W + ax;
    const front = (y + bh) * W + ax;
    let s = 0;
    preds.forEach((p, i) => {
      const k = i === 0 ? 3 : i === 1 ? 2 : 1;
      const [base, arg] = p.split(":") as [string, string | undefined];
      let v = 0;
      switch (base) {
        case "on_square":
        case "near_square":
          v = -dist(ax, ay) * 4;
          break;
        case "central":
          v = -dist(ax, ay) * 2;
          break;
        case "edge":
          v = dist(ax, ay) * 3;
          break;
        case "quiet":
          v = Math.min(12, arterialDist[c] as number) * 4;
          break;
        case "arterial":
          v = (arterialDist[front] as number) <= 1 ? 120 : -Math.min(120, (arterialDist[front] as number) * 6);
          break;
        case "by_water":
          v = -Math.min(60, Math.min(w.riverDist[c] as number, w.seaDist[c] as number)) * 6;
          break;
        case "shore":
          v = -Math.min(80, w.seaDist[c] as number, w.geo.coast ? 999 : (w.riverDist[c] as number)) * 8;
          break;
        case "riverbank":
          v = -Math.min(80, w.riverDist[c] as number) * 8;
          break;
        case "near_rail":
          v = railDist ? -Math.min(60, railDist[c] as number) * 8 : 0;
          break;
        case "high_ground":
          v = (w.level[c] as number) * 70;
          break;
        case "largest_residential":
          v = -isqrt((ax - dc.x) * (ax - dc.x) + (ay - dc.y) * (ay - dc.y)) * 3;
          break;
        case "toward": {
          const b = arg === "minerals" ? w.geo.minerals.bearing : arg === "woods" ? w.geo.woods.bearing : NO_BEARING;
          if (b !== NO_BEARING) {
            const v2 = compassVec(b);
            v = Math.trunc(((ax - cx) * v2.dx + (ay - cy) * v2.dy) / 1024) * 4;
          }
          break;
        }
        case "near_place": {
          const at = arg ? nearKind(arg) : null;
          if (at) v = -isqrt((ax - at.x) * (ax - at.x) + (ay - at.y) * (ay - at.y)) * 3;
          break;
        }
      }
      s += v * k;
    });
    return s;
  };
}

/** Best spot for a footprint among a district's frontage (largest size first, shrinking). */
function bestSpot(w: Work, d: number, sizes: [number, number][], setbacks: number[], score: Scorer, jitterKey: number, any = false): Fit | null {
  const m = masks(w);
  const front = frontageOf(w, d, any);
  const jitter = (c: number) => mix(jitterKey, c) & 7;
  for (const [bw, bh] of sizes) {
    for (const s of setbacks) {
      let best: Fit | null = null;
      let bestS = -Infinity;
      for (const f of front) {
        const F = Math.floor(f / w.W);
        const xc = f % w.W;
        // The footprint's door column is this frontage cell (any column of the building).
        const x = xc - (bw >> 1);
        if (!fits(w, d, x, F, bw, bh, s, m, any)) continue;
        const sc = score(x, F - 1 - s - bh + 1, bw, bh) + jitter(f);
        if (sc > bestS) {
          bestS = sc;
          best = { x, F, w: bw, h: bh, s };
        }
      }
      if (best) return best;
    }
  }
  return null;
}

function sizesOf(l: PlaceLayout | undefined, kind: string): [number, number][] {
  const [lo, hi] = l?.layout.footprint ?? [placeSize(kind), placeSize(kind)];
  const out: [number, number][] = [];
  const push = (a: number, b: number) => {
    if (!out.some(([x, y]) => x === a && y === b)) out.push([a, b]);
  };
  push(hi[0], hi[1]);
  push(hi[0], lo[1]);
  push(lo[0], hi[1]);
  push(lo[0], lo[1]);
  push(Math.max(3, lo[0] - 1), Math.max(2, lo[1] - 1));
  push(3, 2);
  return out;
}

export function stagePlaces(w: Work) {
  const { W } = w;
  resetFrontage(w);
  const geo = w.geo;
  const layouts = new Map(w.recipe.places.map((p) => [p.key, p]));
  const hoodDistrict = new Map(w.districts.map((d) => [d.id, d.index]));
  const coreD = w.districts.find((d) => d.kind === "core")?.index ?? 0;
  const places = [...geo.places].sort((a, b) => a.id - b.id);
  const placedAt = new Map<string, { x: number; y: number }>();
  const groundOf = (d: number) => GROUND[(w.districts[d] as WorkDistrict | undefined)?.type.plots.ground ?? "grass"] ?? Terrain.Grass;
  for (const p of places) if (layouts.has(p.kind)) w.used.places.add(p.kind);

  // ---- the square: civic buildings side by side along its north side, the town hall in the middle; then the market
  const onSquare = places.filter((p) => (CIVIC.includes(p.kind) || p.kind === "market") && (hoodDistrict.get(p.neighborhood) ?? coreD) === coreD && (layouts.get(p.kind)?.layout.place[0] ?? "") === "on_square");
  const order = [...onSquare].sort((a, b) => (a.kind === "market" ? 1 : 0) - (b.kind === "market" ? 1 : 0) || CIVIC.indexOf(a.kind) - CIVIC.indexOf(b.kind) || a.id - b.id);
  const chosen: { p: (typeof places)[number]; w: number; h: number }[] = [];
  let used = 0;
  for (const p of order) {
    const fp = layouts.get(p.kind)?.layout.footprint ?? [placeSize(p.kind), placeSize(p.kind)];
    let bw = fp[1][0];
    if (used + (chosen.length ? 1 : 0) + bw > w.civic.w - 2) bw = fp[0][0];
    if (used + (chosen.length ? 1 : 0) + bw > w.civic.w - 2) continue;
    used += (chosen.length ? 1 : 0) + bw;
    chosen.push({ p, w: bw, h: Math.min(w.civic.h, fp[1][1]) });
  }
  const arranged: typeof chosen = [];
  for (const c of chosen) {
    if (arranged.length % 2 === 0) arranged.push(c);
    else arranged.unshift(c);
  }
  let x = w.civic.x + Math.floor((w.civic.w - used) / 2);
  const placedIds = new Set<number>();
  for (const c of arranged) {
    // Civic buildings stand on the square's paving, their fronts on the plaza.
    const F = w.plaza.y;
    for (let y = F - c.h; y < F; y++) for (let xx = x; xx < x + c.w; xx++) w.use[y * W + xx] = (w.use[y * W + xx] as number) & ~Use.Square;
    const i = addPlotForce(w, coreD, { x, F, w: c.w, h: c.h, s: 0 }, { place: c.p.id, kind: c.p.kind, name: c.p.name, landmark: c.p.kind !== "market", ground: Terrain.Plaza });
    placedIds.add(c.p.id);
    placedAt.set(c.p.kind, { x: x + (c.w >> 1), y: F - 1 });
    if (c.p.kind !== "market") w.landmarks.push({ kind: c.p.kind, x: x + (c.w >> 1), y: F - c.h, name: c.p.name, plot: i });
    x += c.w + 1;
  }

  // ---- every other place in its own district, by its placement rules
  const priority = (k: string) => (CIVIC.includes(k) ? 0 : layouts.get(k)?.layout.landmark ? 1 : k === "park" ? 3 : 2);
  const queue = places.filter((p) => !placedIds.has(p.id)).sort((a, b) => priority(a.kind) - priority(b.kind) || a.id - b.id);
  for (const p of queue) {
    const l = layouts.get(p.kind);
    const d = hoodDistrict.get(p.neighborhood) ?? coreD;
    const preds = l?.layout.place ?? ["central"];
    const score = placeScorer(w, preds, d, (k) => placedAt.get(k) ?? null);
    const type = (w.districts[d] as WorkDistrict).type;
    let spot = bestSpot(w, d, sizesOf(l, p.kind), [type.plots.setback, 0], score, mix(w.seed, p.id));
    if (!spot) spot = bestSpot(w, d, [[3, 2]], [0], score, mix(w.seed, p.id, 1), false);
    if (!spot) spot = carveSpot(w, d, sizesOf(l, p.kind));
    if (!spot && process.env.DEV_SKIPPLACE) continue;
    if (!spot) throw new LayoutOverflow(`no room for ${p.kind} in ${(w.districts[d] as WorkDistrict).name}`);
    const isLandmark = l?.layout.landmark ?? false;
    const i = addPlot(w, d, spot, { place: p.id, kind: p.kind, name: p.name, landmark: CIVIC.includes(p.kind) || (isLandmark && spot.w * spot.h >= 20), ground: groundOf(d) });
    const plot = w.plots[i] as PlanPlot;
    placedAt.set(p.kind, { x: plot.x + (plot.w >> 1), y: plot.y + plot.h - 1 });
    if (isLandmark) w.landmarks.push({ kind: l?.layout.signature ?? p.kind, x: plot.x + (plot.w >> 1), y: plot.y, name: p.name, plot: i });
    if (p.kind === "park") parkAround(w, plot);
    if (p.kind === "mine") mineYard(w, plot);
    if (p.kind === "port") pier(w, plot);
  }

  // ---- landmarks the engine has no place for: the church on high ground in sight of the square, the station by the line
  const churchKind = w.band === "village" ? "chapel" : "church";
  const centralDs = w.districts.filter((d) => d.kind === "core" || d.kind === "inner").map((d) => d.index);
  let church: Fit | null = null;
  let churchD = coreD;
  const churchScore = placeScorer(w, ["near_square", "high_ground"], coreD, () => null);
  for (const d of centralDs.length ? centralDs : [coreD]) {
    const s = bestSpot(w, d, churchKind === "chapel" ? [[3, 4], [3, 3]] : [[4, 5], [4, 4], [3, 4]], [0], churchScore, mix(w.seed, 0xc4, d));
    if (s && (!church || churchScore(s.x, s.F - s.h, s.w, s.h) > churchScore(church.x, church.F - church.h, church.w, church.h))) {
      church = s;
      churchD = d;
    }
  }
  if (church) {
    const i = addPlot(w, churchD, church, { place: null, kind: churchKind, name: "", landmark: true, ground: groundOf(churchD) });
    w.landmarks.push({ kind: "spire", x: church.x + (church.w >> 1), y: church.F - church.h, name: "", plot: i });
  }
  if (w.rail.length) stationBySide(w, groundOf);
}

/**
 * The station stands beside the track (§5 stage 4: the station on the line,
 * at the core's edge): its footprint one cell off the railway, that cell a
 * platform, as near the square as it fits; its door path is joined up by
 * the connectivity pass.
 */
function stationBySide(w: Work, groundOf: (d: number) => number) {
  const { W, H } = w;
  const N = W * H;
  const m = masks(w);
  const rd = distanceField(W, H, (c) => ((w.use[c] as number) & Use.Rail) !== 0 && w.terrain[c] === Terrain.Rail, () => true, 8);
  const sx = w.plaza.x + (w.plaza.w >> 1);
  const sy = w.plaza.y + (w.plaza.h >> 1);
  let best: { x: number; y: number; bw: number; bh: number; d: number; s: number } | null = null;
  // Strictly first (one district, one cell of platform); then a footprint across districts or on the fields at the town's edge.
  const coreD = w.districts.find((d) => d.kind === "core")?.index ?? 0;
  for (let pass = 0; pass < 2 && !best; pass++) {
    for (const [bw, bh] of [
      [6, 3],
      [5, 3],
      [4, 3],
    ] as const) {
      for (let y = 2; y + bh < H - 1; y++) {
        for (let x = 1; x + bw < W - 1; x++) {
          const c0 = y * W + x;
          let d = w.district[c0] as number;
          if (d < 0 && pass === 0) continue;
          const lv = w.level[c0] as number;
          let ok = true;
          let near = 99;
          for (let yy = y; yy < y + bh && ok; yy++) {
            for (let xx = x; xx < x + bw && ok; xx++) {
              const c = yy * W + xx;
              const dc = w.district[c] as number;
              if ((pass === 0 && dc !== d) || w.use[c] || !buildable(w.terrain[c] as number) || w.level[c] !== lv || (rd[c] as number) < 2 || isWaterBody(w, c) || m.roofs[c]) ok = false;
              if (d < 0 && dc >= 0) d = dc;
              near = Math.min(near, rd[c] as number);
            }
          }
          if (!ok || near !== 2) continue;
          if (d < 0) d = coreD;
          const front = (y + bh) * W + x + (bw >> 1);
          if (front >= N || isWaterBody(w, front) || (w.use[front] as number) & (Use.Plot | Use.Square) || w.blocked[front] || m.roofs[front] || !(frontageTerrain(w.terrain[front] as number) || buildable(w.terrain[front] as number))) continue;
          let roofOk = true;
          for (let xx = x; xx < x + bw; xx++) if (m.fronts[(y - 1) * W + xx] || (w.use[(y - 1) * W + xx] as number) & Use.Plot) roofOk = false;
          if (!roofOk) continue;
          const s = -(Math.abs(x + (bw >> 1) - sx) + Math.abs(y + bh - sy)) - (bw === 6 ? 0 : 12);
          if (!best || s > best.s) best = { x, y, bw, bh, d, s };
        }
      }
    }
  }
  if (!best) return;
  const i = addPlot(w, best.d, { x: best.x, F: best.y + best.bh, w: best.bw, h: best.bh, s: 0 }, { place: null, kind: "station", name: "", landmark: true, ground: groundOf(best.d) }, best.x + (best.bw >> 1));
  w.landmarks.push({ kind: "station", x: best.x + (best.bw >> 1), y: best.y, name: "", plot: i });
  // The platform: the cells between the building and the track.
  for (let y = best.y - 1; y <= best.y + best.bh; y++) {
    for (let x = best.x - 1; x <= best.x + best.bw; x++) {
      if (x < 0 || y < 0 || x >= W || y >= H) continue;
      const c = y * W + x;
      if ((rd[c] as number) !== 1 || w.use[c] || !buildable(w.terrain[c] as number)) continue;
      w.terrain[c] = Terrain.Pavement;
      w.use[c] = (w.use[c] as number) | Use.Keep;
    }
  }
}

/** A plot placed where it is told (the square's civic row), checked only against other plots. */
function addPlotForce(w: Work, d: number, f: Fit, spec: { place: number | null; kind: string; name: string; landmark: boolean; ground: number }): number {
  const i = addPlot(w, d, f, spec);
  return i;
}

/** No frontage fits: clear a spot on the district's open land beside a street (the connectivity pass joins it up). */
function carveSpot(w: Work, d: number, sizes: [number, number][]): Fit | null {
  const { W, H } = w;
  const m = masks(w);
  // First on open land; then over the district's small greens (a junction green or a leftover corner gives way to a place).
  for (let pass = 0; pass < 2; pass++) {
    for (const [bw, bh] of [...sizes].reverse()) {
      for (let F = bh + 2; F < H - 1; F++) {
        for (let x = 1; x + bw < W - 1; x++) {
          const freed: number[] = [];
          if (pass === 1) {
            for (let y = F - bh - 1; y <= F; y++) {
              for (let xx = x; xx < x + bw; xx++) {
                const c = y * W + xx;
                if (w.use[c] === Use.Keep && w.terrain[c] === Terrain.Park && !m.fronts[c] && w.district[c] === d) {
                  freed.push(c);
                  w.use[c] = 0;
                }
              }
            }
            if (!freed.length) continue;
          }
          // A row of open land of the district below the footprint becomes its frontage path.
          let open = true;
          for (let xx = x; xx < x + bw && open; xx++) {
            const fc = F * W + xx;
            const t = w.terrain[fc] as number;
            if (w.district[fc] !== d || (w.use[fc] as number) & (Use.Plot | Use.Square | Use.Rail | Use.Keep) || isWaterBody(w, fc) || (!buildable(t) && !frontageTerrain(t))) open = false;
          }
          if (open) {
            const was = Array.from({ length: bw }, (_, k) => w.terrain[F * W + x + k] as number);
            for (let k = 0; k < bw; k++) if (!frontageTerrain(was[k] as number)) w.terrain[F * W + x + k] = Terrain.Path;
            if (fits(w, d, x, F, bw, bh, 0, m)) {
              for (let k = 0; k < bw; k++) w.use[F * W + x + k] = (w.use[F * W + x + k] as number) | Use.Road;
              for (const c of freed) if (!((w.use[c] as number) & Use.Road)) w.terrain[c] = Terrain.Grass;
              return { x, F, w: bw, h: bh, s: 0 };
            }
            for (let k = 0; k < bw; k++) w.terrain[F * W + x + k] = was[k] as number;
          }
          for (const c of freed) w.use[c] = Use.Keep;
        }
      }
    }
  }
  return null;
}

/** A park place: the lodge, and its block laid out as a park. */
function parkAround(w: Work, plot: PlanPlot) {
  const { W, H } = w;
  const N = W * H;
  const start = (plot.y - 1) * W + plot.x;
  if (start < 0 || !buildable(w.terrain[start] as number) || (w.use[start] as number) & (Use.Road | Use.Plot) || w.district[start] !== plot.district) return;
  const q = [start];
  const seen = new Set([start]);
  for (let qi = 0; qi < q.length && q.length < 260; qi++) {
    const c = q[qi] as number;
    if (!((w.use[c] as number) & Use.Plot)) {
      w.terrain[c] = Terrain.Park;
      w.use[c] = (w.use[c] as number) | Use.Keep;
    }
    const x = c % W;
    for (let k4 = 0; k4 < 4; k4++) {
        const n = k4 === 0 ? (x > 0 ? c - 1 : -1) : k4 === 1 ? (x < W - 1 ? c + 1 : -1) : k4 === 2 ? c - W : c + W;
      if (n < 0 || n >= N || seen.has(n) || w.district[n] !== plot.district || (w.use[n] as number) & (Use.Road | Use.Plot) || !buildable(w.terrain[n] as number)) continue;
      seen.add(n);
      q.push(n);
    }
  }
}

/** The mine: a yard with the headframe beside the winding house and the spoil heap behind (§6.3 `with`). */
function mineYard(w: Work, plot: PlanPlot) {
  const { W } = w;
  const free = (x: number, y: number) => {
    if (x < 1 || y < 1 || x >= W - 1 || y >= w.H - 1) return false;
    const c = y * W + x;
    const d = w.district[c] as number;
    return (d === plot.district || d < 0) && !(w.use[c] as number) && !w.blocked[c] && buildable(w.terrain[c] as number);
  };
  const by = plot.y + plot.h - 1;
  // The headframe stands beside the building (on its bottom row, else nearby).
  const spots: [number, number][] = [];
  for (const hx of [plot.x + plot.w + 1, plot.x - 2, plot.x + plot.w, plot.x - 1]) spots.push([hx, by]);
  for (let r = 1; r <= 6; r++) for (let dx = -r; dx <= plot.w + r; dx++) spots.push([plot.x + dx, plot.y - 1 - r]);
  for (const [hx, hy] of spots) {
    if (!free(hx, hy) || !free(hx, hy - 1) || (hy === by ? false : !free(hx, hy + 1))) continue;
    const by2 = hy;
    for (const c of [by2 * W + hx, (by2 - 1) * W + hx]) {
      w.terrain[c] = Terrain.Yard;
      w.use[c] = (w.use[c] as number) | Use.Keep;
    }
    w.blocked[by2 * W + hx] = 1;
    w.props.push({ kind: "headframe", x: hx, y: by2, blocks: true, variant: mix(w.seed, hx, by2) & 0xff });
    w.landmarks.push({ kind: "headframe", x: hx, y: by2 - 2, name: "", plot: w.plots.indexOf(plot) });
    break;
  }
  // The spoil heap: three by two tiles behind or beside the pit.
  const tries: [number, number][] = [];
  for (let dy = -2; dy >= -8; dy--) for (const dx of [0, plot.w - 3, -4, plot.w + 1]) tries.push([plot.x + dx, plot.y + dy]);
  for (const dx of [plot.w + 3, -6, plot.w + 4, -7]) tries.push([plot.x + dx, by - 1]);
  for (let r = 2; r <= 12; r++) for (let dx = -r - 3; dx <= plot.w + r; dx += 2) tries.push([plot.x + dx, plot.y - r - 1], [plot.x + dx, by + r]);
  for (const [sx, sy] of tries) {
    let ok = true;
    for (let y = sy; y < sy + 2 && ok; y++) for (let x = sx; x < sx + 3 && ok; x++) if (!free(x, y)) ok = false;
    if (!ok) continue;
    for (let y = sy; y < sy + 2; y++) {
      for (let x = sx; x < sx + 3; x++) {
        const c = y * W + x;
        w.terrain[c] = Terrain.Yard;
        w.use[c] = (w.use[c] as number) | Use.Keep;
        w.blocked[c] = 1;
      }
    }
    w.props.push({ kind: "spoil_heap", x: sx, y: sy + 1, blocks: true, variant: mix(w.seed, sx, sy) & 0xff });
    break;
  }
}

/** The port: a pier from the nearest shore into the water, cranes on the quay. */
function pier(w: Work, plot: PlanPlot) {
  const { W, H } = w;
  const N = W * H;
  // Nearest land cell beside open water, from the port's front.
  const q = [plot.front];
  const seen = new Set(q);
  let shore = -1;
  let dir = 0;
  for (let qi = 0; qi < q.length && qi < 4000 && shore < 0; qi++) {
    const c = q[qi] as number;
    const x = c % W;
    const nb = [x < W - 1 ? c + 1 : -1, x > 0 ? c - 1 : -1, c + W < N ? c + W : -1, c - W];
    for (let k = 0; k < 4; k++) {
      const n = nb[k] as number;
      if (n < 0 || n >= N) continue;
      if (isWaterBody(w, n) && !((w.use[n] as number) & (Use.Bridge | Use.Rail))) {
        shore = c;
        dir = k;
        break;
      }
      if (seen.has(n) || (w.use[n] as number) & Use.Plot || isWaterBody(w, n)) continue;
      seen.add(n);
      q.push(n);
    }
  }
  if (shore < 0) return;
  const step = [1, -1, W, -W][dir] as number;
  const side = dir < 2 ? W : 1;
  let c = shore + step;
  let made = 0;
  for (let k = 0; k < 7; k++, c += step) {
    if (c < 0 || c >= N || !isWaterBody(w, c) || (w.use[c] as number) & (Use.Bridge | Use.Rail)) break;
    for (const o of [-side, 0, side]) {
      const cc = c + o;
      if (cc < 0 || cc >= N || !isWaterBody(w, cc) || (w.use[cc] as number) & (Use.Bridge | Use.Rail)) continue;
      w.terrain[cc] = Terrain.Pier;
      w.use[cc] = (w.use[cc] as number) | Use.Road;
    }
    made++;
  }
  if (made) {
    // The shore end is quay, so the pier joins the street.
    for (const o of [-side, 0, side]) {
      const cc = shore + o;
      if (cc >= 0 && cc < N && buildable(w.terrain[cc] as number) && !((w.use[cc] as number) & Use.Plot)) {
        w.terrain[cc] = Terrain.Quay;
        w.use[cc] = (w.use[cc] as number) | Use.Road;
      }
    }
    const cx = shore % W;
    const cy = Math.floor(shore / W);
    w.props.push({ kind: "crane", x: cx + (dir < 2 ? 0 : 2), y: cy + (dir < 2 ? 2 : 0), blocks: false, variant: 0 });
  }
}

// ---------------------------------------------------------------------------
// Stage 10: filler buildings and their ranks; stage 9: reserved land

function pickKind(t: DistrictType, r: { next(): number }): string {
  const total = t.buildings.mix.reduce((a, [, wt]) => a + wt, 0);
  let u = r.next() * total;
  for (const [k, wt] of t.buildings.mix) {
    u -= wt;
    if (u < 0) return k;
  }
  return (t.buildings.mix[0] as [string, number])[0];
}

export function stageFillers(w: Work) {
  resetFrontage(w);
  const { W } = w;
  const m = masks(w);
  for (const d of w.districts) {
    const t = d.type;
    const r = rngOf(w, "fill", d.id);
    const front = frontageOf(w, d.index);
    // Runs of frontage, row by row, left to right.
    let i = 0;
    while (i < front.length) {
      const c = front[i] as number;
      const F = Math.floor(c / W);
      let x = c % W;
      let j = i;
      while (j + 1 < front.length && (front[j + 1] as number) === (front[j] as number) + 1 && Math.floor((front[j + 1] as number) / W) === F) j++;
      const xEnd = (front[j] as number) % W;
      while (x <= xEnd) {
        const bw = r.int(t.plots.frontage[0], t.plots.frontage[1]);
        const bh = r.int(t.plots.depth[0], t.plots.depth[1]);
        let placed = false;
        for (const s of t.plots.setback ? [t.plots.setback, 0] : [0]) {
          for (let hh = bh; hh >= Math.min(2, t.plots.depth[0]) && !placed; hh--) {
            for (let ww = Math.min(bw, xEnd - x + 1); ww >= Math.max(2, t.plots.frontage[0] - 1) && !placed; ww--) {
              if (!fits(w, d.index, x, F, ww, hh, s, m)) continue;
              if (r.next() < t.buildings.density) {
                const kind = pickKind(t, r);
                addPlot(w, d.index, { x, F, w: ww, h: hh, s }, { place: null, kind, name: "", landmark: false, ground: GROUND[t.plots.ground] ?? Terrain.Grass });
              }
              x += ww + (r.next() < t.plots.joined ? 0 : r.int(1, 2));
              placed = true;
            }
          }
          if (placed) break;
        }
        if (!placed) x++;
      }
      i = j + 1;
    }
  }
}

/** A district its streets gave no buildings still holds a few (§8: even the smallest district holds a street of buildings). */
export function stageMinimumFill(w: Work) {
  for (const d of w.districts) {
    let have = 0;
    for (const p of w.plots) if (p.district === d.index && p.place === null && !p.landmark) have++;
    for (let k = have; k < 3; k++) {
      const kind = pickKind(d.type, rngOf(w, "minfill", d.id, k));
      const spot = bestSpot(w, d.index, [[3, 3], [3, 2]], [0], () => 0, mix(w.seed, d.id, k)) ?? carveSpot(w, d.index, [[3, 3], [3, 2]]);
      if (!spot) break;
      addPlot(w, d.index, spot, { place: null, kind, name: "", landmark: false, ground: GROUND[d.type.plots.ground] ?? Terrain.Grass });
    }
  }
}

/** Fill and decline ranks per district (fixed in the plan, so growth and decline are monotonic), and which plots are reserved at baseline. */
export function stageRanks(w: Work): number[] {
  const { W } = w;
  const baseline: number[] = w.districts.map(() => 0);
  for (const d of w.districts) {
    const plots = w.plots.map((p, i) => ({ p, i })).filter((x) => x.p.district === d.index && x.p.place === null && !x.p.landmark);
    const sx = d.seed % W;
    const sy = Math.floor(d.seed / W);
    const key = (p: PlanPlot) => {
      const ax = p.x + (p.w >> 1);
      const ay = p.y + p.h;
      const fromSeed = (ax - sx) * (ax - sx) + (ay - sy) * (ay - sy);
      const fromSquare = (ax - w.centre.x) * (ax - w.centre.x) + (ay - w.centre.y) * (ay - w.centre.y);
      const base = d.type.growth.fill === "infill_first" ? fromSeed : fromSquare;
      return base * 16 + (mix(w.seed, p.x, p.y) & 15) * 4;
    };
    plots.sort((a, b) => key(a.p) - key(b.p) || a.i - b.i);
    plots.forEach((x, k) => (x.p.fillRank = k));
    // Decline: the far edges first (and hash order), so it spreads inward.
    const dkey = (p: PlanPlot) => -key(p) + (mix(w.seed, p.x, p.y, 0xdec) & 1023) * 8;
    const byDecline = [...plots].sort((a, b) => dkey(a.p) - dkey(b.p) || a.i - b.i);
    byDecline.forEach((x, k) => (x.p.declineRank = k));
    const n = plots.length;
    const reserved = Math.floor(n * d.type.growth.reserve);
    baseline[d.index] = n - reserved;
    for (const x of plots) {
      if (x.p.fillRank >= n - reserved) x.p.ground = d.type.growth.fill === "edge_estates" || d.kind === "rural" ? Terrain.Field : x.p.ground === Terrain.Yard ? Terrain.Yard : Terrain.Grass;
    }
  }
  return baseline;
}

// ---------------------------------------------------------------------------
// Stage 11: props

export function stageProps(w: Work) {
  const { W, H } = w;
  const N = W * H;
  const add = (kind: PropKind, x: number, y: number, blocks: boolean) => {
    w.props.push({ kind, x, y, blocks, variant: mix(w.seed, x, y) & 0xff });
    if (blocks) w.blocked[y * W + x] = 1;
  };
  const m = masks(w);
  const at = (x: number, y: number) => (x >= 0 && y >= 0 && x < W && y < H ? (w.terrain[y * W + x] as number) : -1);
  // ---- the square: fountain, benches, planters, flags, the market's stalls
  const sq = w.plaza;
  const fx = sq.x + (sq.w >> 1) - 1;
  const fy = sq.y + (sq.h >> 1);
  for (const [dx, dy] of [
    [0, 0],
    [1, 0],
    [0, 1],
    [1, 1],
  ] as const)
    w.blocked[(fy + dy) * W + fx + dx] = 1;
  w.props.push({ kind: "fountain", x: fx, y: fy, blocks: true, variant: 0 });
  for (const bx of [sq.x + 1, sq.x + sq.w - 2]) {
    add("planter", bx, sq.y + 1, false);
    add("bench", bx, fy, false);
    add("planter", bx, sq.y + sq.h - 2, false);
  }
  const market = w.plots.some((p) => p.kind === "market");
  if (market) {
    for (let y = sq.y + 2; y < sq.y + sq.h - 1; y += 3) {
      for (let x = sq.x + 3; x < sq.x + sq.w - 3; x += 2) {
        if (Math.abs(x - fx) <= 2 && Math.abs(y - fy) <= 2) continue;
        if (m.fronts[y * W + x] || m.fronts[(y - 1) * W + x]) continue;
        add("stall", x, y, false);
      }
    }
  }
  for (const p of w.plots) if (p.landmark && p.y + p.h === sq.y && p.place !== null) add("flag", p.x + p.w - 1, p.y + p.h, false);
  // ---- lamps along asphalt pavements in town
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const c = y * W + x;
      if (w.terrain[c] !== Terrain.Pavement || m.fronts[c] || (w.district[c] as number) < 0) continue;
      if ((w.districts[w.district[c] as number] as WorkDistrict).kind === "rural") continue;
      const rd = [at(x, y - 1), at(x, y + 1), at(x - 1, y), at(x + 1, y)].some((t) => t === Terrain.Road);
      if (rd && (x * 7 + y * 13) % 11 === 0) add("lamp", x, y, false);
    }
  }
  // ---- water: boats by the quays and piers; reeds; stepping stones at fords
  const rp = rngOf(w, "props");
  for (let c = 0; c < N; c++) {
    const x = c % W;
    const y = Math.floor(c / W);
    const t = w.terrain[c] as number;
    if (t === Terrain.Water) {
      const nearQuay = [at(x, y - 2), at(x, y + 2), at(x - 2, y), at(x + 2, y)].some((n) => n === Terrain.Quay || n === Terrain.Pier);
      if (nearQuay && rp.chance(0.04)) add("boat", x, y, false);
    } else if (t === Terrain.Reeds) {
      if (rp.chance(0.35)) add("reeds", x, y, false);
    } else if (t === Terrain.Ford && ((w.use[c] as number) & Use.Road) !== 0 && at(x - 1, y) !== Terrain.Ford && at(x, y - 1) !== Terrain.Ford) {
      add("stepping_stones", x, y, false);
    }
  }
  // ---- trees: parks, gardens, river banks, and the woods on the town's wooded side
  const woods = w.geo.woods.share_pm;
  const wv = w.geo.woods.bearing === NO_BEARING ? { dx: 0, dy: 0 } : compassVec(w.geo.woods.bearing);
  const wn = (w.seed ^ 0x700d5) >>> 0;
  const nearDoor = (x: number, y: number) => {
    for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) if (m.fronts[(y + dy) * W + x + dx] || at(x + dx, y + dy) === Terrain.Door) return true;
    return false;
  };
  for (let y = 1; y < H - 1; y++) {
    for (let x = 1; x < W - 1; x++) {
      const c = y * W + x;
      const t = w.terrain[c] as number;
      if ((w.use[c] as number) & (Use.Plot | Use.Road | Use.Square | Use.Rail | Use.End)) continue;
      const d = w.district[c] as number;
      let p = 0;
      if (t === Terrain.Park) p = 0.14;
      else if (t === Terrain.Grass || t === Terrain.Field) {
        if (d < 0) {
          // The countryside: woods toward their bearing, thicker with the town's forest share.
          const proj = Math.trunc(((x - (W >> 1)) * wv.dx + (y - (H >> 1)) * wv.dy) / 1024);
          const n = valueNoise(wn, x, y, 7) + proj * 6 + Math.floor(woods / 3);
          p = t === Terrain.Field ? 0.004 : n > 820 ? 0.55 : n > 640 ? 0.12 : 0.02;
        } else {
          const kind = (w.districts[d] as WorkDistrict).kind;
          p = (w.riverDist[c] as number) <= 2 ? 0.16 : kind === "suburb" ? 0.06 : kind === "rural" ? 0.05 : 0.025;
          if (t === Terrain.Field) p = 0.004;
        }
      }
      if (p === 0 || rp.next() >= p) continue;
      if (nearDoor(x, y)) continue;
      const below = w.use[c + W] as number;
      const above = w.use[c - W] as number;
      if (below & (Use.Plot | Use.Road) || above & Use.Plot) continue;
      if ([at(x - 1, y), at(x + 1, y), at(x, y - 1), at(x, y + 1)].some((n) => n === Terrain.Path || n === Terrain.Garden || n === Terrain.Steps)) continue;
      let crowded = false;
      for (let dy = -1; dy <= 1 && !crowded; dy++) for (let dx = -1; dx <= 1; dx++) if (w.blocked[(y + dy) * W + x + dx]) crowded = true;
      if (crowded) continue;
      add(d < 0 && (w.level[c] as number) >= 1 && rp.chance(0.6) ? "pine" : d < 0 && rp.chance(0.3) ? "pine" : "tree", x, y, true);
    }
  }
}

// ---------------------------------------------------------------------------
// Stage 12: every door reachable (carving paths, or dropping a filler that cannot be reached)

export function stageConnect(w: Work) {
  const { W, H } = w;
  const N = W * H;
  // Plot footprints are solid; their door fronts and gardens are not.
  const solidPlot = new Uint8Array(N);
  for (const p of w.plots) for (let y = p.y; y < p.y + p.h; y++) for (let x = p.x; x < p.x + p.w; x++) solidPlot[y * W + x] = 1;
  const walk = (c: number) => !solidPlot[c] && walkableGround(w, c);
  const reach = new Uint8Array(N);
  const q = new Int32Array(N);
  const flood = (starts: number[]) => {
    let qt = 0;
    for (const s of starts) {
      if (reach[s] || !walk(s)) continue;
      reach[s] = 1;
      q[qt++] = s;
    }
    for (let qh = 0; qh < qt; qh++) {
      const c = q[qh] as number;
      const x = c % W;
      for (let k4 = 0; k4 < 4; k4++) {
        const n = k4 === 0 ? (x > 0 ? c - 1 : -1) : k4 === 1 ? (x < W - 1 ? c + 1 : -1) : k4 === 2 ? c - W : c + W;
        if (n < 0 || n >= N || reach[n] || !walk(n)) continue;
        reach[n] = 1;
        q[qt++] = n;
      }
    }
  };
  flood([w.plaza.y * W + w.plaza.x + 1]);
  const carve = (from: number): boolean => {
    // Breadth-first from the door front through land that can take a path, to the reached network.
    const prev = new Map<number, number>([[from, -1]]);
    const bq = [from];
    let hit = -1;
    for (let qi = 0; qi < bq.length && qi < 20000; qi++) {
      const c = bq[qi] as number;
      if (reach[c]) {
        hit = c;
        break;
      }
      const x = c % W;
      for (let k4 = 0; k4 < 4; k4++) {
        const n = k4 === 0 ? (x > 0 ? c - 1 : -1) : k4 === 1 ? (x < W - 1 ? c + 1 : -1) : k4 === 2 ? c - W : c + W;
        if (n < 0 || n >= N || prev.has(n) || solidPlot[n] || (nearEdge(w, n) && !walkableGround(w, n))) continue;
        const t = w.terrain[n] as number;
        if ((isWaterBody(w, n) && !walkableGround(w, n)) || t === Terrain.Rock) continue;
        prev.set(n, c);
        bq.push(n);
      }
    }
    if (hit < 0) return false;
    const made: number[] = [];
    for (let c = prev.get(hit) as number; c >= 0; c = prev.get(c) as number) {
      const t = w.terrain[c] as number;
      if (!walkableGround(w, c) || t === Terrain.Grass || t === Terrain.Field || t === Terrain.Yard || t === Terrain.Park || t === Terrain.Sand || t === Terrain.Reeds) {
        if (w.water[c] === WaterKind.Creek) {
          w.terrain[c] = Terrain.Ford;
          w.connectors.push({ kind: "ford", type: "connector.ford", x: c % W, y: Math.floor(c / W), w: 1, h: 1, axis: "ns", name: "", crosses: "creek", arterial: -1 });
          w.used.connectors.add("connector.ford");
        } else if (t === Terrain.Embankment) w.terrain[c] = Terrain.Steps;
        else if (t === Terrain.Rail) {
          w.terrain[c] = Terrain.Crossing;
          w.connectors.push({ kind: "level_crossing", type: "connector.level_crossing", x: c % W, y: Math.floor(c / W), w: 1, h: 1, axis: "ns", name: "", crosses: "rail", arterial: -1 });
        }
        else if (t !== Terrain.Door) w.terrain[c] = Terrain.Path;
        w.use[c] = (w.use[c] as number) | Use.Road;
        w.blocked[c] = 0;
      }
      made.push(c);
    }
    // Trees on the new path go.
    if (made.length) {
      const set = new Set(made);
      w.props = w.props.filter((p) => !(p.blocks && set.has(p.y * W + p.x) && (p.kind === "tree" || p.kind === "pine")));
    }
    flood(made);
    return true;
  };
  // Every district can be walked into ("the streets of" it): a district cut off gets a path from its seed.
  const reached = new Uint8Array(w.districts.length);
  for (let c = 0; c < N; c++) if (reach[c] && (w.district[c] as number) >= 0) reached[w.district[c] as number] = 1;
  for (const d of w.districts) {
    if (reached[d.index]) continue;
    let from = -1;
    for (let c = 0; c < N && from < 0; c++) if (w.district[c] === d.index && !solidPlot[c] && !isWaterBody(w, c) && w.terrain[c] !== Terrain.Rock) from = c;
    if (from >= 0) carve(from);
  }
  // Places and landmarks first; a filler that cannot be reached is dropped.
  const order = w.plots.map((_, i) => i).sort((a, b) => ((w.plots[a] as PlanPlot).place === null ? 1 : 0) - ((w.plots[b] as PlanPlot).place === null ? 1 : 0) || a - b);
  const drop = new Set<number>();
  for (const i of order) {
    const p = w.plots[i] as PlanPlot;
    const f = p.front;
    if (w.terrain[f] === Terrain.Garden || buildable(w.terrain[f] as number)) {
      w.terrain[f] = Terrain.Path;
    }
    if (reach[f]) continue;
    if (!carve(f)) {
      if (p.place !== null) throw new LayoutOverflow(`the ${p.kind} cannot be reached`);
      drop.add(i);
    }
  }
  if (drop.size) {
    const remap = new Map<number, number>();
    const keep: PlanPlot[] = [];
    w.plots.forEach((p, i) => {
      if (drop.has(i)) return;
      remap.set(i, keep.length);
      keep.push(p);
    });
    w.plots.length = 0;
    w.plots.push(...keep);
    w.landmarks = w.landmarks.filter((l) => !drop.has(l.plot)).map((l) => ({ ...l, plot: remap.get(l.plot) ?? -1 }));
  }
}

/** Whether a cell can be walked, ignoring plots (the caller handles footprints). */
function walkableGround(w: Work, c: number): boolean {
  const t = w.terrain[c] as number;
  if (w.blocked[c]) return false;
  if (t === Terrain.Bridge || t === Terrain.Pier || t === Terrain.Ford || t === Terrain.Crossing || t === Terrain.Steps) return !(t === Terrain.Bridge && (w.use[c] as number) & Use.Rail && !((w.use[c] as number) & Use.Road));
  if (isWaterBody(w, c) || w.water[c] === WaterKind.Creek) return false;
  return !(t === Terrain.Rock || t === Terrain.Embankment || t === Terrain.Rail || t === Terrain.Pit || t === Terrain.Water || t === Terrain.Creek);
}

// ---------------------------------------------------------------------------
// Stage 12b: no long way round (§8: every place within a walking-detour factor of 1.6 of the square, 2.2 across water)

const DETOUR = 1.6;
const DETOUR_WATER = 2.2;

/**
 * Where the walk from the square to a place goes a long way round (behind a
 * creek, the railway, a row of blocks), a path is cut along the shortest line
 * over land that can take one (fields, yards, a ford over a creek, steps up a
 * bank, a level crossing over the line); where the long way is round the
 * river, a footbridge crosses it straight, at the narrowest place that
 * shortens the walk most (the doc's mitigation for bridges as choke points).
 */
export function stageShortcuts(w: Work) {
  const { W, H } = w;
  const N = W * H;
  const solidPlot = new Uint8Array(N);
  for (const p of w.plots) for (let y = p.y; y < p.y + p.h; y++) for (let x = p.x; x < p.x + p.w; x++) solidPlot[y * W + x] = 1;
  const walk = (c: number) => !solidPlot[c] && walkableGround(w, c);
  const sq = (w.plaza.y + (w.plaza.h >> 1)) * W + w.plaza.x + (w.plaza.w >> 1);
  const sx = sq % W;
  const sy = Math.floor(sq / W);
  const fromSquare = (): Int32Array => {
    const d = new Int32Array(N).fill(-1);
    const q = new Int32Array(N);
    let qt = 0;
    // From the whole square (its open plaza), as the readability check measures.
    for (let y = w.plaza.y; y < w.plaza.y + w.plaza.h; y++) {
      for (let x = w.plaza.x; x < w.plaza.x + w.plaza.w; x++) {
        const c = y * W + x;
        if (!walk(c)) continue;
        d[c] = 0;
        q[qt++] = c;
      }
    }
    for (let qh = 0; qh < qt; qh++) {
      const c = q[qh] as number;
      const x = c % W;
      for (let k4 = 0; k4 < 4; k4++) {
        const n = k4 === 0 ? (x > 0 ? c - 1 : -1) : k4 === 1 ? (x < W - 1 ? c + 1 : -1) : k4 === 2 ? c - W : c + W;
        if (n < 0 || n >= N || (d[n] as number) >= 0 || !walk(n)) continue;
        d[n] = (d[c] as number) + 1;
        q[qt++] = n;
      }
    }
    return d;
  };
  /** Extra cost of cutting a path through a cell (0: walkable already; -1: never). */
  const cutCost = (c: number): number => {
    if (nearEdge(w, c) && !walk(c)) return -1;
    if (walk(c)) return 0;
    if (solidPlot[c] || (w.use[c] as number) & (Use.Plot | Use.Square) || w.blocked[c] || isWaterBody(w, c)) return -1;
    const t = w.terrain[c] as number;
    if (t === Terrain.Rock || t === Terrain.Pit) return -1;
    if (w.water[c] === WaterKind.Creek) return 3;
    if (t === Terrain.Embankment) return 3;
    if (t === Terrain.Rail) return 6;
    return 2;
  };
  const cut = (c: number) => {
    const t = w.terrain[c] as number;
    if (walk(c)) return;
    if (w.water[c] === WaterKind.Creek) {
      w.terrain[c] = Terrain.Ford;
      w.connectors.push({ kind: "ford", type: "connector.ford", x: c % W, y: Math.floor(c / W), w: 1, h: 1, axis: "ns", name: "", crosses: "creek", arterial: -1 });
      w.used.connectors.add("connector.ford");
    } else if (t === Terrain.Embankment) w.terrain[c] = Terrain.Steps;
    else if (t === Terrain.Rail) {
      w.terrain[c] = Terrain.Crossing;
      w.connectors.push({ kind: "level_crossing", type: "connector.level_crossing", x: c % W, y: Math.floor(c / W), w: 1, h: 1, axis: "ns", name: "", crosses: "rail", arterial: -1 });
    } else w.terrain[c] = Terrain.Path;
    w.use[c] = (w.use[c] as number) | Use.Road;
  };
  /** The cheapest way from a door to the square's network, cutting what it must; carved. */
  const desireLine = (from: number, dSq: Int32Array, scale: number) => {
    const g = new Float64Array(N).fill(Infinity);
    const prev = new Int32Array(N).fill(-1);
    const heap = new MinHeap(256);
    g[from] = 0;
    heap.push(0, from);
    let best = Infinity;
    let bestCell = -1;
    while (heap.size) {
      const c = heap.pop();
      const gc = heap.lastKey;
      if (gc > (g[c] as number) || gc >= best) continue;
      if ((dSq[c] as number) >= 0 && gc + (dSq[c] as number) < best) {
        best = gc + (dSq[c] as number);
        bestCell = c;
      }
      const x = c % W;
      for (let k4 = 0; k4 < 4; k4++) {
        const n = k4 === 0 ? (x > 0 ? c - 1 : -1) : k4 === 1 ? (x < W - 1 ? c + 1 : -1) : k4 === 2 ? c - W : c + W;
        if (n < 0 || n >= N) continue;
        const k = cutCost(n);
        if (k < 0) continue;
        const ng = gc + 1 + k * scale;
        if (ng < (g[n] as number)) {
          g[n] = ng;
          prev[n] = c;
          heap.push(ng, n);
        }
      }
    }
    for (let c = bestCell; c >= 0; c = prev[c] as number) cut(c);
  };
  const places = w.plots.filter((p) => p.place !== null || p.landmark);
  let dSq = fromSquare();
  const bridged = new Set<PlanPlot>();
  for (let round = 0; round < 3; round++) {
    let fixed = 0;
    for (const p of places) {
      const f = p.front;
      const md = rectDistance(w.plaza, f % W, Math.floor(f / W));
      if (md < 16) continue;
      const steps = dSq[f] as number;
      const wet = crossesWater(w, sx, sy, f % W, Math.floor(f / W));
      if (steps >= 0 && steps <= (wet ? DETOUR_WATER : DETOUR) * md) continue;
      if (wet && round >= 1 && !bridged.has(p)) {
        bridged.add(p);
        // The far landing joins the streets on its own side too.
        const far = footbridge(w, p, dSq);
        if (far >= 0) desireLine(far, fromSquare(), 1);
      }
      // A second pass cuts straighter (cheaper cuts) where the first still left a long way round.
      desireLine(f, dSq, round === 0 ? 1 : round === 1 ? 0.25 : 0.05);
      dSq = fromSquare();
      fixed++;
    }
    if (!fixed) break;
  }
}

/** Whether the straight line between two cells crosses a river, lake or the sea. */
function crossesWater(w: Work, x0: number, y0: number, x1: number, y1: number): boolean {
  for (let k = 0; k <= 32; k++) {
    const x = Math.round(x0 + ((x1 - x0) * k) / 32);
    const y = Math.round(y0 + ((y1 - y0) * k) / 32);
    if (isWaterBody(w, y * w.W + x)) return true;
  }
  return false;
}

/** A straight footbridge over the river where it shortens the walk from the square to a plot most. */
function footbridge(w: Work, p: PlanPlot, dSq: Int32Array): number {
  const { W, H } = w;
  const N = W * H;
  const max = w.recipe.connectors.find((c) => c.kind === "footbridge")?.max_span ?? 10;
  const fx = p.front % W;
  const fy = Math.floor(p.front / W);
  // A landing is walkable ground, or open ground a path can be laid on.
  const land = (c: number) => {
    if (c < 0 || c >= N || isWaterBody(w, c) || (w.use[c] as number) & (Use.Plot | Use.Square | Use.Rail) || w.blocked[c] || nearEdge(w, c)) return false;
    const t = w.terrain[c] as number;
    return walkableGround(w, c) || buildable(t) || t === Terrain.Sand || t === Terrain.Reeds;
  };
  let best: { a: number; b: number; dir: number; span: number; s: number } | null = null;
  const dirs = [1, -1, W, -W];
  for (let a = 0; a < N; a++) {
    if (!land(a) || (dSq[a] as number) < 0) continue;
    const ax = a % W;
    for (const d of dirs) {
      const n = a + d;
      if (n < 0 || n >= N || !isWaterBody(w, n) || (d === 1 && ax === W - 1) || (d === -1 && ax === 0)) continue;
      let k = 1;
      while (k <= max + 1 && a + d * k >= 0 && a + d * k < N && isWaterBody(w, a + d * k)) k++;
      const b = a + d * k;
      if (k > max || !land(b) || Math.abs((b % W) - ax) > max + 1) continue;
      // The far end is the plot's side: score by the walk it makes.
      const s = (dSq[a] as number) + k + Math.abs((b % W) - fx) + Math.abs(Math.floor(b / W) - fy);
      if (!best || s < best.s) best = { a, b, dir: d, span: k - 1, s };
    }
  }
  if (!best) return -1;
  const horiz = best.dir === 1 || best.dir === -1;
  for (let k = 1; k <= best.span; k++) {
    const c = best.a + best.dir * k;
    w.terrain[c] = Terrain.Bridge;
    w.use[c] = (w.use[c] as number) | Use.Road | Use.Bridge;
  }
  const c0 = best.a + best.dir;
  const c1 = best.a + best.dir * best.span;
  const x0 = Math.min(c0 % W, c1 % W);
  const y0 = Math.min(Math.floor(c0 / W), Math.floor(c1 / W));
  w.connectors.push({
    kind: "footbridge",
    type: w.recipe.connectors.find((c) => c.kind === "footbridge")?.id ?? "connector.timber_footbridge",
    x: x0,
    y: y0,
    w: horiz ? best.span : 1,
    h: horiz ? 1 : best.span,
    axis: horiz ? "ew" : "ns",
    name: "",
    crosses: w.water[c0] === WaterKind.Sea ? "sea" : w.water[c0] === WaterKind.Lake ? "lake" : "river",
    arterial: -1,
  });
  const used = w.recipe.connectors.find((c) => c.kind === "footbridge");
  if (used) w.used.connectors.add(used.id);
  for (const c of [best.a, best.b]) {
    if (c < 0 || c >= N || isWaterBody(w, c) || (w.use[c] as number) & (Use.Plot | Use.Square | Use.Rail)) continue;
    if (buildable(w.terrain[c] as number) || w.terrain[c] === Terrain.Sand || w.terrain[c] === Terrain.Reeds) w.terrain[c] = Terrain.Path;
    w.use[c] = (w.use[c] as number) | Use.Road;
  }
  return best.b;
}

/** Manhattan distance from a cell to the nearest cell of a rectangle (0 inside). */
export function rectDistance(r: { x: number; y: number; w: number; h: number }, x: number, y: number): number {
  return Math.max(r.x - x, 0, x - (r.x + r.w - 1)) + Math.max(r.y - y, 0, y - (r.y + r.h - 1));
}

/** Within two cells of the map's edge: no path or footbridge is cut there (only the roads to other towns reach it). */
function nearEdge(w: Work, c: number): boolean {
  const x = c % w.W;
  const y = (c - x) / w.W;
  return x < 2 || y < 2 || x >= w.W - 2 || y >= w.H - 2;
}
