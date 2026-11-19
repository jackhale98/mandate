// Layout v2 stages 0–3 (TOWN_GENERATION.md §5): the map's size from the
// population, ground levels from the relief, water (river, creeks, coast,
// lake) and the centre with its square. Integer maths only (./imath).
import { Terrain } from "../townLayout";
import { bandOf } from "./geo";
import { NO_BEARING, bearingGap, clamp, compassVec, isqrt, log2Q8, valueNoise } from "./imath";
import { Use, WaterKind, distanceField, feature, inMap, isWaterBody, param, paramRange, rngOf, type Work } from "./work";

/** Map size from population (§9.1, D6): side ≈ 40 + 22·log2(pop / 5,000), clamped to 56–240, aspect 1.4:1, capped at 240 × 160. Multiples of 4. */
export function townSize(population: number): { W: number; H: number } {
  const side = 40 + Math.floor((22 * (log2Q8(Math.max(1, population)) - log2Q8(5000))) / 256);
  const W = Math.floor(clamp(side, 56, 240) / 4) * 4;
  const H = Math.floor(clamp(Math.floor((W * 5 + 3) / 7), 40, 160) / 4) * 4;
  return { W, H };
}

/** The square block (civic strip + plaza), tiles, multiples of 4 so it sits on the arterials' 4-tile lattice. */
export function squareSize(population: number): { w: number; h: number; civicH: number } {
  switch (bandOf(population)) {
    case "village":
      return { w: 12, h: 12, civicH: 5 };
    case "small_town":
      return { w: 16, h: 12, civicH: 5 };
    case "town":
      return { w: 20, h: 16, civicH: 5 };
    case "city":
      return { w: 24, h: 16, civicH: 5 };
    case "metro":
      return { w: 32, h: 20, civicH: 6 };
  }
}

// ---------------------------------------------------------------------------
// Stage 1: levels

export function stageTerrain(w: Work) {
  const { W, H } = w;
  const hills = feature(w, "hills");
  const at = paramRange(hills, "levels_at", [150, 330]);
  const thresholds = (hills?.params.levels_at as number[] | undefined) ?? [at[0], at[1], 560];
  const rug = w.geo.relief.ruggedness_pm;
  let maxLevel = 0;
  for (const t of thresholds) if (rug >= t) maxLevel++;
  maxLevel = Math.min(3, maxLevel);
  if (maxLevel === 0) return;
  const noisePm = param(hills, "noise_pm", 450);
  const span = (maxLevel + 1) * 1024;
  const up = w.geo.relief.uphill;
  const v = up === NO_BEARING ? { dx: 0, dy: 0 } : compassVec(up);
  // Extent of the projection onto the uphill vector, for a ramp across the whole map.
  const ext = Math.max(1, Math.floor((Math.abs(v.dx) * W + Math.abs(v.dy) * H) / 2));
  const ns = (w.seed ^ 0x51ab) >>> 0;
  const aRamp = Math.floor((span * (1000 - noisePm)) / 2000);
  const aNoise = Math.floor((span * noisePm) / 1000);
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const proj = (x - W / 2) * v.dx + (y - H / 2) * v.dy;
      const ramp = Math.floor((proj * aRamp) / ext);
      const n = ((valueNoise(ns, x, y, 28) * 2 + valueNoise(ns + 1, x, y, 11)) / 3) | 0;
      const h = Math.floor(span / 2) + ramp + Math.floor(((n - 512) * aNoise) / 1024);
      w.level[y * W + x] = clamp(Math.floor(h / 1024), 0, maxLevel);
    }
  }
  smoothLevels(w, 48);
}

/** Merge regions of one level smaller than `min` cells into their most common neighbouring level (no specks of hill). */
export function smoothLevels(w: Work, min: number) {
  const { W, H } = w;
  const N = W * H;
  const seen = new Int32Array(N).fill(-1);
  const q = new Int32Array(N);
  for (let s = 0; s < N; s++) {
    if ((seen[s] as number) >= 0) continue;
    const lv = w.level[s] as number;
    let qt = 0;
    q[qt++] = s;
    seen[s] = s;
    const border = [0, 0, 0, 0];
    for (let qh = 0; qh < qt; qh++) {
      const c = q[qh] as number;
      const x = c % W;
      for (let k4 = 0; k4 < 4; k4++) {
        const n = k4 === 0 ? (x > 0 ? c - 1 : -1) : k4 === 1 ? (x < W - 1 ? c + 1 : -1) : k4 === 2 ? c - W : c + W;
        if (n < 0 || n >= N) continue;
        const ln = w.level[n] as number;
        if (ln !== lv) {
          border[ln] = (border[ln] as number) + 1;
          continue;
        }
        if ((seen[n] as number) >= 0) continue;
        seen[n] = s;
        q[qt++] = n;
      }
    }
    if (qt >= min) continue;
    let best = lv;
    let bestN = 0;
    for (let k = 0; k < 4; k++) if ((border[k] as number) > bestN) [best, bestN] = [k, border[k] as number];
    for (let i = 0; i < qt; i++) w.level[q[i] as number] = best;
  }
}

// ---------------------------------------------------------------------------
// Stage 2: water

/** Where a ray from (cx, cy) along a bearing leaves the map (inside it), tiles. */
export function edgePoint(W: number, H: number, b: number, cx: number, cy: number): { x: number; y: number } {
  const v = compassVec(b);
  const tx = v.dx > 0 ? ((W - 1 - cx) * 1024) / v.dx : v.dx < 0 ? (cx * 1024) / -v.dx : Infinity;
  const ty = v.dy > 0 ? ((H - 1 - cy) * 1024) / v.dy : v.dy < 0 ? (cy * 1024) / -v.dy : Infinity;
  const t = Math.min(tx, ty);
  return { x: clamp(cx + Math.round((v.dx * t) / 1024), 0, W - 1), y: clamp(cy + Math.round((v.dy * t) / 1024), 0, H - 1) };
}

type Pt = { x: number; y: number };

/** Midpoint displacement of a polyline, perpendicular to each segment (integer, keyed). */
function meander(w: Work, pts: Pt[], pm: number, rounds: number, label: string): Pt[] {
  let cur = pts;
  for (let r = 0; r < rounds; r++) {
    const rng = rngOf(w, label, r);
    const next: Pt[] = [cur[0] as Pt];
    for (let i = 1; i < cur.length; i++) {
      const a = cur[i - 1] as Pt;
      const b = cur[i] as Pt;
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const len = Math.max(1, isqrt(dx * dx + dy * dy));
      const amp = Math.floor((len * pm) / 2000);
      const d = amp > 0 ? rng.int(-amp, amp) : 0;
      next.push({ x: Math.floor((a.x + b.x) / 2) + Math.trunc((-dy * d) / len), y: Math.floor((a.y + b.y) / 2) + Math.trunc((dx * d) / len) });
      next.push(b);
    }
    cur = next;
  }
  return cur;
}

/** Walk a polyline cell by cell (4-connected), calling `visit` for each cell. */
function walkLine(pts: Pt[], visit: (x: number, y: number) => boolean | void) {
  for (let i = 1; i < pts.length; i++) {
    const a = pts[i - 1] as Pt;
    const b = pts[i] as Pt;
    let x = a.x;
    let y = a.y;
    const dx = Math.abs(b.x - a.x);
    const dy = -Math.abs(b.y - a.y);
    const sx = a.x < b.x ? 1 : -1;
    const sy = a.y < b.y ? 1 : -1;
    let err = dx + dy;
    if (visit(x, y) === true) return;
    while (x !== b.x || y !== b.y) {
      const e2 = 2 * err;
      // One axis per step: the line stays 4-connected.
      if (e2 - dy > dx - e2) {
        err += dy;
        x += sx;
      } else {
        err += dx;
        y += sy;
      }
      if (visit(x, y) === true) return;
    }
  }
}

export function stageWater(w: Work) {
  const { W, H } = w;
  const N = W * H;
  const geo = w.geo;
  // ---- the coast first (the river runs into it)
  if (geo.coast) coast(w);
  if (geo.lake) lake(w);
  if (geo.river) river(w);
  w.riverDist = distanceField(W, H, (c) => w.water[c] === WaterKind.River || w.water[c] === WaterKind.Lake, (c) => !isWaterBody(w, c), 200);
  w.seaDist = distanceField(W, H, (c) => w.water[c] === WaterKind.Sea, (c) => !isWaterBody(w, c), 200);
  // Valleys: the floodplain is the lowest level, the terraces above it at most one up.
  if (geo.river) {
    for (let c = 0; c < N; c++) {
      const d = w.riverDist[c] as number;
      if (d <= w.floodplain) w.level[c] = 0;
      else if (d <= w.floodplain + 7) w.level[c] = Math.min(w.level[c] as number, 1);
    }
  }
  const shore = geo.coast?.shore ?? "beach";
  for (let c = 0; c < N; c++) {
    if ((w.seaDist[c] as number) <= (shore === "cliff" ? 0 : 4)) w.level[c] = 0;
    if (isWaterBody(w, c)) w.level[c] = 0;
  }
  smoothLevels(w, 40);
  for (const g of geo.creeks.slice(0, param(feature(w, "creek"), "max", 2))) creek(w, g.enters, g.joins_river, geo.creeks.indexOf(g));
  // Marsh: reeds on the low banks, away from the centre (rural edges).
  const marsh = feature(w, "marsh");
  if (geo.river && marsh && geo.hazards_pm.flood >= 150) {
    const share = param(marsh, "share_pm", 350);
    const r = rngOf(w, "marsh");
    const mx = W >> 1;
    const my = H >> 1;
    const far = ((W * W + H * H) * 9) / 100;
    for (let c = 0; c < N; c++) {
      if ((w.riverDist[c] as number) !== 1 || w.water[c] !== WaterKind.None) continue;
      const x = c % W;
      const y = (c / W) | 0;
      if ((x - mx) * (x - mx) + (y - my) * (y - my) < far) continue;
      if (r.int(0, 999) < share) w.terrain[c] = Terrain.Reeds;
    }
  }
}

function river(w: Work) {
  const { W, H } = w;
  const geo = w.geo;
  const g = geo.river;
  if (!g) return;
  const f = feature(w, "river");
  const widths = (f?.params.width_by_class as number[][] | undefined) ?? [
    [2, 3],
    [4, 6],
    [7, 9],
  ];
  const [wlo, whi] = (widths[g.class - 1] ?? [4, 6]) as [number, number];
  const r = rngOf(w, "river");
  const width = r.int(wlo, whi);
  const fp = paramRange(f, "floodplain", [2, 8]);
  w.floodplain = fp[0] + Math.floor(((fp[1] - fp[0]) * Math.min(1000, geo.hazards_pm.flood * 2)) / 1000) + (g.class - 1);
  // Its course: from the side it enters to the side it leaves, beside (or through) the centre.
  const axis = g.enters;
  const perp = compassVec((axis + 4) & 15);
  const off = Math.floor((param(f, "offset_pm", 140) * Math.min(W, H) * (g.through_centre ? 10 : 24)) / 10000);
  const side = r.chance(0.5) ? 1 : -1;
  const pc = { x: Math.floor(W / 2) + Math.trunc((perp.dx * off * side) / 1024), y: Math.floor(H / 2) + Math.trunc((perp.dy * off * side) / 1024) };
  const p0 = edgePoint(W, H, g.enters, pc.x, pc.y);
  const p1 = edgePoint(W, H, g.leaves, pc.x, pc.y);
  const out = (p: Pt, b: number) => {
    const v = compassVec(b);
    return { x: p.x + Math.trunc((v.dx * 6) / 1024), y: p.y + Math.trunc((v.dy * 6) / 1024) };
  };
  const pts = meander(w, [out(p0, g.enters), pc, out(p1, g.leaves)], param(f, "meander_pm", 160), 4, "river-meander");
  const rad = Math.ceil(width / 2);
  const w2 = width * width;
  walkLine(pts, (x, y) => {
    for (let oy = -rad; oy <= rad; oy++) {
      for (let ox = -rad; ox <= rad; ox++) {
        if (4 * (ox * ox + oy * oy) > w2) continue;
        const nx = x + ox;
        const ny = y + oy;
        if (!inMap(w, nx, ny)) continue;
        const c = ny * W + nx;
        if (w.water[c] === WaterKind.Sea) continue;
        w.water[c] = WaterKind.River;
        w.terrain[c] = Terrain.Water;
      }
    }
  });
}

function creek(w: Work, enters: number, joins: boolean, k: number) {
  const { W, H } = w;
  const f = feature(w, "creek");
  const start = edgePoint(W, H, enters, W >> 1, H >> 1);
  // Toward the river's nearest cell to the map's middle, else across to the far side.
  let end: Pt = edgePoint(W, H, (enters + 8) & 15, W >> 1, H >> 1);
  if (joins && w.geo.river) {
    let best = Infinity;
    for (let c = 0; c < W * H; c++) {
      if (w.water[c] !== WaterKind.River) continue;
      const x = c % W;
      const y = (c / W) | 0;
      const d = (x - start.x) * (x - start.x) + (y - start.y) * (y - start.y);
      if (d < best) {
        best = d;
        end = { x, y };
      }
    }
  }
  const pts = meander(w, [start, end], param(f, "meander_pm", 220), 4, `creek-meander-${k}`);
  walkLine(pts, (x, y) => {
    if (!inMap(w, x, y)) return;
    const c = y * W + x;
    if (isWaterBody(w, c)) return true;
    w.water[c] = WaterKind.Creek;
    w.terrain[c] = Terrain.Creek;
  });
}

function coast(w: Work) {
  const { W, H } = w;
  const g = w.geo.coast;
  if (!g) return;
  const f = feature(w, "coast");
  const v = compassVec(g.sea);
  const proj = (x: number, y: number) => (x - W / 2) * v.dx + (y - H / 2) * v.dy;
  const corners = [proj(0, 0), proj(W - 1, 0), proj(0, H - 1), proj(W - 1, H - 1)];
  const pmax = Math.max(...corners);
  const pmin = Math.min(...corners);
  const spanP = pmax - pmin;
  const depth = Math.floor((spanP * param(f, "depth_pm", 200)) / 1000);
  const wobble = param(f, "wobble", 4) * 1024;
  const bay = g.bay ? Math.floor((spanP * param(f, "bay_pm", 120)) / 1000) : 0;
  const bw = Math.max(8, Math.floor(Math.min(W, H) / 3));
  const ns = (w.seed ^ 0xc0a57) >>> 0;
  const T = pmax - depth;
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      // Position along the shore (tiles).
      const u = Math.trunc(((x - W / 2) * -v.dy + (y - H / 2) * v.dx) / 1024);
      const wob = Math.floor(((valueNoise(ns, u + 512, 0, 14) - 512) * wobble) / 512);
      const bump = bay && Math.abs(u) < bw ? Math.floor((bay * (bw * bw - u * u)) / (bw * bw)) : 0;
      if (proj(x, y) > T + wob - bump) {
        const c = y * W + x;
        w.water[c] = WaterKind.Sea;
        w.terrain[c] = Terrain.Water;
      }
    }
  }
  // Shore: a beach, rocks or a cliff, except at the harbour (the shore nearest the middle).
  const seaDist = distanceField(W, H, (c) => w.water[c] === WaterKind.Sea, (c) => w.water[c] !== WaterKind.Sea, 6);
  const beach = paramRange(feature(w, "beach"), "width", [2, 3]);
  const r = rngOf(w, "shore");
  const bwid = r.int(beach[0], beach[1]);
  const rockPm = param(feature(w, "cliff"), "rock_pm", 600);
  const harbour = Math.floor(Math.min(W, H) / 4);
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const c = y * W + x;
      const d = seaDist[c] as number;
      if (d === 0 || d > 3) continue;
      const u = Math.trunc(((x - W / 2) * -v.dy + (y - H / 2) * v.dx) / 1024);
      const nearHarbour = Math.abs(u) < harbour;
      if (g.shore === "beach") {
        if (d <= bwid) w.terrain[c] = Terrain.Sand;
      } else if (g.shore === "cliff" && !nearHarbour) {
        if (d <= 2) w.terrain[c] = Terrain.Rock;
      } else if (d === 1) {
        if (!nearHarbour && r.int(0, 999) < rockPm) w.terrain[c] = Terrain.Rock;
        else if (d <= 1) w.terrain[c] = Terrain.Sand;
      }
    }
  }
}

function lake(w: Work) {
  const { W, H } = w;
  const g = w.geo.lake;
  if (!g) return;
  const f = feature(w, "lake");
  const [lo, hi] = paramRange(f, "radius", [8, 13]);
  const r = rngOf(w, "lake");
  const rx = r.int(lo, hi);
  const ry = Math.max(4, Math.floor((rx * 3) / 4));
  const v = compassVec(g.bearing);
  const e = edgePoint(W, H, g.bearing, W >> 1, H >> 1);
  const pull = g.near ? rx + 2 : Math.floor(rx / 2);
  const cx = e.x - Math.trunc((v.dx * pull) / 1024);
  const cy = e.y - Math.trunc((v.dy * pull) / 1024);
  const ns = (w.seed ^ 0x1a4e) >>> 0;
  for (let y = cy - ry - 3; y <= cy + ry + 3; y++) {
    for (let x = cx - rx - 3; x <= cx + rx + 3; x++) {
      if (!inMap(w, x, y)) continue;
      const dx = x - cx;
      const dy = y - cy;
      const n = valueNoise(ns, x, y, 6) - 512;
      if (1024 * (dx * dx * ry * ry + dy * dy * rx * rx) <= rx * rx * ry * ry * (1024 + (n >> 1))) {
        const c = y * W + x;
        if (w.water[c] !== WaterKind.None) continue;
        w.water[c] = WaterKind.Lake;
        w.terrain[c] = Terrain.Water;
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Stage 3: the centre and its square

export function stageCentre(w: Work) {
  const { W, H } = w;
  const geo = w.geo;
  const sq = squareSize(geo.baseline_population);
  const coastV = geo.coast ? compassVec(geo.coast.sea) : null;
  const r = rngOf(w, "centre");
  let best: { bx: number; by: number; s: number } | null = null;
  const margin = 12;
  // Summed-area tables: wet (or rock) cells and cells per level, so each candidate is O(1).
  const W1 = W + 1;
  const sat = Array.from({ length: 5 }, () => new Int32Array(W1 * (H + 1)));
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const c = y * W + x;
      const lv = w.level[c] as number;
      const wetC = w.water[c] !== WaterKind.None || w.terrain[c] === Terrain.Rock ? 1 : 0;
      for (let k = 0; k < 5; k++) {
        const t = sat[k] as Int32Array;
        const v = k === 4 ? wetC : lv === k ? 1 : 0;
        t[(y + 1) * W1 + x + 1] = v + (t[y * W1 + x + 1] as number) + (t[(y + 1) * W1 + x] as number) - (t[y * W1 + x] as number);
      }
    }
  }
  const area = (k: number, x0: number, y0: number, x1: number, y1: number) => {
    const t = sat[k] as Int32Array;
    return (t[y1 * W1 + x1] as number) - (t[y0 * W1 + x1] as number) - (t[y1 * W1 + x0] as number) + (t[y0 * W1 + x0] as number);
  };
  for (let by = margin; by + sq.h + margin <= H; by += 4) {
    for (let bx = margin; bx + sq.w + margin <= W; bx += 4) {
      // The ring of streets round the block must be dry land too.
      const wx0 = bx - 6;
      const wy0 = by - 6;
      const wx1 = bx + sq.w + 6;
      const wy1 = by + sq.h + 6;
      if (area(4, wx0, wy0, wx1, wy1) > 0) continue;
      let levels: number[] = [0, 1, 2, 3].map((k) => area(k, wx0, wy0, wx1, wy1));
      const mx = bx + (sq.w >> 1);
      const my = by + (sq.h >> 1);
      const dx = mx - (W >> 1);
      const dy = my - (H >> 1);
      let s = -((dx * dx + dy * dy) * 4) / Math.max(1, W >> 2);
      // Flat ground: most of the block on one level.
      levels = levels.sort((a, b) => b - a);
      s -= Math.floor(((sq.w + 12) * (sq.h + 12) - (levels[0] as number)) / 6);
      const c = my * W + mx;
      if (geo.river) {
        // On the terrace above the floodplain, near the crossing.
        const d = w.riverDist[c] as number;
        const target = w.floodplain + 4 + (sq.h >> 1);
        s -= Math.abs(d - target) * 10;
      }
      if (coastV) {
        const d = w.seaDist[c] as number;
        s -= Math.abs(d - (8 + (sq.h >> 1))) * 8;
      }
      s += r.int(0, 6);
      if (!best || s > best.s) best = { bx, by, s };
    }
  }
  if (!best) {
    // No dry site: clear one in the middle (a tiny island town).
    const bx = Math.floor((W - sq.w) / 8) * 4;
    const by = Math.floor((H - sq.h) / 8) * 4;
    for (let y = by - 6; y < by + sq.h + 6; y++) {
      for (let x = bx - 6; x < bx + sq.w + 6; x++) {
        const c = y * W + x;
        w.water[c] = WaterKind.None;
        w.terrain[c] = Terrain.Grass;
      }
    }
    best = { bx, by, s: 0 };
  }
  const { bx, by } = best;
  w.civic = { x: bx, y: by, w: sq.w, h: sq.civicH };
  w.plaza = { x: bx, y: by + sq.civicH, w: sq.w, h: sq.h - sq.civicH };
  w.ring = { x: bx - 4, y: by - 4, w: sq.w + 8, h: sq.h + 8 };
  w.centre = { x: bx + (sq.w >> 1), y: by + sq.civicH + ((sq.h - sq.civicH) >> 1) };
  // The square and its ring stand on one level (the block's most common).
  const counts = new Int32Array(4);
  for (let y = by; y < by + sq.h; y++) for (let x = bx; x < bx + sq.w; x++) counts[w.level[y * W + x] as number] = (counts[w.level[y * W + x] as number] as number) + 1;
  let lv = 0;
  for (let k = 1; k < 4; k++) if ((counts[k] as number) > (counts[lv] as number)) lv = k;
  for (let y = by - 6; y < by + sq.h + 6; y++) for (let x = bx - 6; x < bx + sq.w + 6; x++) w.level[y * W + x] = lv;
  for (let y = by; y < by + sq.h; y++) {
    for (let x = bx; x < bx + sq.w; x++) {
      const c = y * W + x;
      w.terrain[c] = Terrain.Plaza;
      w.use[c] = (w.use[c] as number) | Use.Square;
    }
  }
  void bearingGap;
}
