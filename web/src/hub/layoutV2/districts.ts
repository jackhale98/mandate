// Layout v2 stages 5–6 (TOWN_GENERATION.md §5): every engine neighbourhood
// becomes one district of a registry type (picked by weighted rendezvous
// hashing among the types whose rules match), sited by its type's
// preferences and grown by a quota-limited flood fill in growth order, with
// rivers, the railway and slopes as preferred edges. Then each district
// runs its type's street pattern (organic, grid, curvy, contour, yards, quay,
// ribbon), and dead ends are joined up or trimmed.
import { hashString, mix } from "../rng";
import { HOOD_KINDS } from "../townInput";
import { Terrain } from "../townLayout";
import { MinHeap, NO_BEARING, compassVec, ipow, isqrt } from "./imath";
import type { GeoCondition, DistrictType, PmRange } from "./registry";
import { paintSegment } from "./roads";
import { LayoutOverflow, RoadClass, Use, WaterKind, distanceField, isWaterBody, rngOf, type Segment, type Work, type WorkDistrict } from "./work";

const inRange = (v: number, r: PmRange | undefined) => !r || (v >= (r.min ?? -Infinity) && v <= (r.max ?? Infinity));

function geoMatches(w: Work, g: GeoCondition | undefined): boolean {
  if (!g) return true;
  const geo = w.geo;
  const hasPort = geo.places.some((p) => p.kind === "port");
  if (g.coast !== undefined && !!geo.coast !== g.coast) return false;
  if (g.river !== undefined && !!geo.river !== g.river) return false;
  if (g.port !== undefined && hasPort !== g.port) return false;
  if (g.lake !== undefined && !!geo.lake !== g.lake) return false;
  if (g.creeks !== undefined && geo.creeks.length > 0 !== g.creeks) return false;
  if (g.shore && !(geo.coast && g.shore.includes(geo.coast.shore))) return false;
  if (!inRange(geo.resources_pm.fishing ?? 0, g.fishing_pm)) return false;
  if (!inRange(geo.minerals.value_pm, g.minerals_pm)) return false;
  if (!inRange(geo.relief.ruggedness_pm, g.ruggedness_pm)) return false;
  if (!inRange(geo.woods.share_pm, g.woods_pm)) return false;
  if (!inRange(geo.hazards_pm.flood, g.flood_pm)) return false;
  return true;
}

/** Whether a district type may be chosen for a neighbourhood of this kind in this town. */
export function eligible(w: Work, t: DistrictType, kind: string): boolean {
  const a = t.appears;
  if (!a.hood_kinds.includes(kind)) return false;
  if (!inRange(w.geo.baseline_population, a.population)) return false;
  if (a.town_class?.length && !a.town_class.includes(w.geo.class)) return false;
  if (a.era?.length && !(w.geo.era && a.era.includes(w.geo.era))) return false;
  if (a.requires_place?.length && !a.requires_place.every((k) => w.geo.places.some((p) => p.kind === k))) return false;
  return geoMatches(w, a.geo);
}

/** Weighted rendezvous pick (PIXEL_ART_TOOL §3.5 rule 6): `weight` tickets each, the highest hash wins. Stable as types are added. */
export function pickType(w: Work, hoodId: number, kind: string): DistrictType {
  const cands = w.recipe.districts.filter((t) => eligible(w, t, kind));
  if (!cands.length) throw new Error(`layout v2: no district type for a ${kind} neighbourhood`);
  let best = cands[0] as DistrictType;
  let bestScore = -1;
  for (const t of cands) {
    for (let k = 0; k < t.appears.weight; k++) {
      const s = mix(w.seed, 0xd157, hoodId, hashString(t.id), k);
      if (s > bestScore) {
        bestScore = s;
        best = t;
      }
    }
  }
  return best;
}

const KIND_AREA: Record<string, number> = { core: 14, inner: 10, waterfront: 11, industrial: 13, suburb: 11, rural: 18 };
const BAND_SHARE: Record<string, number> = { village: 520, small_town: 640, town: 740, city: 840, metro: 930 };
const GROWTH_ORDER: Record<string, number> = { core: 0, inner: 1, waterfront: 2, industrial: 3, suburb: 4, rural: 5 };

const hoodKind = (k: string) => ((HOOD_KINDS as readonly string[]).includes(k) ? k : "inner");

/** Land the town may spread over: connected to the square over land, bridges and fords. */
function reachableLand(w: Work): Uint8Array {
  const { W, H } = w;
  const N = W * H;
  const reach = new Uint8Array(N);
  const q = new Int32Array(N);
  let qt = 0;
  const s = w.plaza.y * W + w.plaza.x;
  reach[s] = 1;
  q[qt++] = s;
  for (let qh = 0; qh < qt; qh++) {
    const c = q[qh] as number;
    const x = c % W;
    for (const n of [x > 0 ? c - 1 : -1, x < W - 1 ? c + 1 : -1, c - W, c + W]) {
      if (n < 0 || n >= N || reach[n]) continue;
      if (isWaterBody(w, n) && !((w.use[n] as number) & Use.Bridge)) continue;
      reach[n] = 1;
      q[qt++] = n;
    }
  }
  return reach;
}

export function stageDistricts(w: Work) {
  const { W, H } = w;
  const N = W * H;
  const geo = w.geo;
  const hoods = [...geo.neighborhoods].sort((a, b) => a.id - b.id);
  if (!hoods.length) hoods.push({ id: -1, name: geo.name, kind: "core", baseline_population: geo.baseline_population, adjacent: [] });
  const townHall = geo.places.find((p) => p.kind === "town_hall");
  const coreIndex = Math.max(0, hoods.findIndex((h) => (townHall ? h.id === townHall.neighborhood : h.kind === "core")));
  const placeLayouts = new Map(w.recipe.places.map((p) => [p.key, p]));

  // ---- types and quotas
  const reach = reachableLand(w);
  let land = 0;
  for (let c = 0; c < N; c++) if (reach[c] && !isWaterBody(w, c)) land++;
  const total = Math.floor((land * (BAND_SHARE[w.band] ?? 700)) / 1000) * (1 + 0) + 0;
  const grow = 100 + 20 * w.attempt;
  const weights = hoods.map((h) => Math.max(1, ipow(Math.max(100, h.baseline_population), 7, 10)) * (KIND_AREA[hoodKind(h.kind)] ?? 10));
  const sumW = weights.reduce((a, b) => a + b, 0);
  w.districts = hoods.map((h, index) => {
    const kind = index === coreIndex ? "core" : hoodKind(h.kind);
    const type = pickType(w, h.id, kind);
    w.used.districts.add(type.id);
    let need = 220;
    for (const p of geo.places) {
      if (p.neighborhood !== h.id) continue;
      const fp = placeLayouts.get(p.kind)?.layout.footprint[1] ?? [4, 3];
      need += (fp[0] + 3) * (fp[1] + 6) * 2;
    }
    const quota = Math.floor((Math.max(need, Math.floor((total * (weights[index] as number)) / sumW)) * grow) / 100);
    return { index, id: h.id, name: h.name, kind, type, population: h.baseline_population, quota, seed: -1, cells: 0 } satisfies WorkDistrict;
  });

  // ---- what seeds score on
  const arterialDist = distanceField(W, H, (c) => w.roadClass[c] === RoadClass.Arterial, (c) => !isWaterBody(w, c), 40);
  const railDist = w.railCells.length ? distanceField(W, H, (c) => ((w.use[c] as number) & Use.Rail) !== 0, (c) => !isWaterBody(w, c), 40) : null;
  const cx = w.centre.x;
  const cy = w.centre.y;
  const R = Math.max(8, isqrt(W * W + H * H) >> 1);
  const vec = (b: number) => (b === NO_BEARING ? { dx: 0, dy: 0 } : compassVec(b));
  const downwind = vec((geo.wind_from + 8) & 15);
  const minerals = vec(geo.minerals.bearing);
  const woods = vec(geo.woods.bearing);
  const free = (c: number) => reach[c] === 1 && !isWaterBody(w, c) && !((w.use[c] as number) & (Use.Square | Use.Rail | Use.Bridge)) && w.terrain[c] !== Terrain.Rock;
  const seeds: { x: number; y: number }[] = [];
  const hostsMine = (d: WorkDistrict) => geo.places.some((p) => p.neighborhood === d.id && p.kind === "mine");
  const hostsPort = (d: WorkDistrict) => geo.places.some((p) => p.neighborhood === d.id && p.kind === "port");
  const score = (d: WorkDistrict, x: number, y: number, rng: () => number): number => {
    const c = y * W + x;
    const dx = x - cx;
    const dy = y - cy;
    const dist = isqrt(dx * dx + dy * dy);
    const prefers = new Set(d.type.site.prefers);
    const avoids = new Set(d.type.site.avoids ?? []);
    if (hostsMine(d)) prefers.add("toward_minerals");
    if (hostsPort(d)) prefers.add("shore");
    let s = 0;
    const has = (p: string) => (prefers.has(p) ? 1 : avoids.has(p) ? -1 : 0);
    s += has("centre") * -dist * 6;
    s += has("near_centre") * -Math.abs(dist - Math.floor((R * 35) / 100)) * 4;
    s += has("edge") * dist * 3;
    const lv = w.level[c] as number;
    s += has("high_ground") * lv * 40;
    const rd = w.riverDist[c] as number;
    const sd = w.seaDist[c] as number;
    if (geo.river) {
      s += has("floodplain") * (rd <= w.floodplain + 1 ? 120 : 0);
      s += has("riverbank") * (rd >= 2 && rd <= w.floodplain + 7 ? 150 : -Math.min(150, rd * 3));
    }
    s += has("shore") * (sd <= 8 ? 160 : -Math.min(160, sd * 4));
    s += has("by_water") * (Math.min(rd, sd) <= 10 ? 100 : -Math.min(100, Math.min(rd, sd) * 2));
    s += has("arterial") * ((arterialDist[c] as number) <= 4 ? 80 : -Math.min(80, (arterialDist[c] as number) * 4));
    if (railDist) s += has("near_rail") * ((railDist[c] as number) <= 6 ? 110 : -Math.min(100, (railDist[c] as number) * 3));
    const proj = (v: { dx: number; dy: number }) => Math.trunc((dx * v.dx + dy * v.dy) / 1024);
    s += has("downwind") * proj(downwind) * 3;
    s += has("upwind") * -proj(downwind) * 3;
    s += has("toward_minerals") * proj(minerals) * 5;
    s += has("toward_woods") * proj(woods) * 4;
    s += has("cliff") * (w.terrain[c] === Terrain.Rock ? 100 : 0);
    // Low slope: one level in the neighbourhood of the seed.
    if (prefers.has("low_slope")) {
      let same = 0;
      for (let k = -3; k <= 3; k++) {
        if (x + k >= 0 && x + k < W && w.level[y * W + x + k] === lv) same++;
        if (y + k >= 0 && y + k < H && w.level[(y + k) * W + x] === lv) same++;
      }
      s += same * 3;
    }
    // Apart from the other districts' seeds, and not on the square's doorstep.
    let sep = R;
    for (const p of seeds) sep = Math.min(sep, isqrt((p.x - x) * (p.x - x) + (p.y - y) * (p.y - y)));
    s += Math.min(sep, Math.floor((R * 45) / 100)) * 7;
    if (dist < 14) s -= (14 - dist) * 30;
    return s + Math.floor(rng() * 24);
  };

  // ---- claim: the core holds the square and its ring; others seed in growth order
  const district = w.district;
  district.fill(-1);
  const heaps = w.districts.map(() => new MinHeap(512));
  const claim = (d: number, c: number) => {
    district[c] = d;
    const wd = w.districts[d] as WorkDistrict;
    wd.cells++;
  };
  const core = w.districts[coreIndex] as WorkDistrict;
  for (let y = w.ring.y; y < w.ring.y + w.ring.h; y++) {
    for (let x = w.ring.x; x < w.ring.x + w.ring.w; x++) {
      const c = y * W + x;
      if (isWaterBody(w, c)) continue;
      claim(core.index, c);
      (heaps[core.index] as MinHeap).push(0, c);
    }
  }
  core.seed = w.centre.y * W + w.centre.x;
  seeds.push({ x: cx, y: cy });
  const order = w.districts.filter((d) => d.index !== core.index).sort((a, b) => (GROWTH_ORDER[a.kind] ?? 9) - (GROWTH_ORDER[b.kind] ?? 9) || a.index - b.index);
  for (const d of order) {
    const r = rngOf(w, "district-seed", d.id);
    let best = -1;
    let bestS = -Infinity;
    const step = Math.max(2, Math.floor(W / 48));
    for (let y = 4; y < H - 4; y += step) {
      for (let x = 4; x < W - 4; x += step) {
        const c = y * W + x;
        if (!free(c) || (district[c] as number) >= 0) continue;
        const s = score(d, x, y, () => r.next());
        if (s > bestS) {
          bestS = s;
          best = c;
        }
      }
    }
    if (best < 0) throw new LayoutOverflow(`no room to seed ${d.name}`);
    d.seed = best;
    seeds.push({ x: best % W, y: Math.floor(best / W) });
    claim(d.index, best);
    (heaps[d.index] as MinHeap).push(0, best);
  }

  // ---- grow: the district furthest below its quota takes its cheapest frontier cell
  const avoidsFlood = w.districts.map((d) => (d.type.site.avoids ?? []).includes("floodplain"));
  const gcost = new Float64Array(N).fill(Infinity);
  for (let c = 0; c < N; c++) if ((district[c] as number) >= 0) gcost[c] = 0;
  const active = w.districts.map(() => true);
  for (;;) {
    let pick = -1;
    let ratio = Infinity;
    for (const d of w.districts) {
      if (!active[d.index]) continue;
      const rt = (d.cells * 1024) / d.quota;
      if (rt >= 1024) {
        active[d.index] = false;
        continue;
      }
      if (rt < ratio) {
        ratio = rt;
        pick = d.index;
      }
    }
    if (pick < 0) break;
    const heap = heaps[pick] as MinHeap;
    let grew = false;
    while (heap.size && !grew) {
      const c = heap.pop();
      const gc = heap.lastKey;
      if (district[c] !== pick) {
        if ((district[c] as number) >= 0) continue;
        claim(pick, c);
        grew = true;
      }
      const x = c % W;
      for (const n of [x > 0 ? c - 1 : -1, x < W - 1 ? c + 1 : -1, c - W, c + W]) {
        if (n < 0 || n >= N || (district[n] as number) >= 0 || !free(n)) continue;
        let cost = 10;
        if (w.level[n] !== w.level[c]) cost += 30;
        if (w.roadClass[n] === RoadClass.Arterial && w.roadClass[c] !== RoadClass.Arterial) cost += 14;
        if (w.water[n] === WaterKind.Creek) cost += 8;
        if (w.terrain[n] === Terrain.Sand || w.terrain[n] === Terrain.Reeds) cost += 6;
        if (avoidsFlood[pick] && geo.river && (w.riverDist[n] as number) <= w.floodplain) cost += 25;
        const ng = gc + cost;
        if (ng < (gcost[n] as number)) {
          gcost[n] = ng;
          heap.push(ng, n);
        }
      }
    }
    if (!grew) active[pick] = false;
  }
  for (const d of w.districts) {
    const need = Math.floor((d.quota * 100) / grow / 3);
    if (d.cells < Math.min(need, 40)) throw new LayoutOverflow(`${d.name} was boxed in (${d.cells} cells)`);
  }
}

// ---------------------------------------------------------------------------
// Stage 6: streets
//
// Each district's land is cut into blocks by its type's street pattern: the
// land between the streets already there (arterials, the square's ring) is
// split, component by component, by an east–west street wherever it is
// deeper than the type's block depth and by a north–south street wherever it
// is wider than its block width, until every block fits. Cuts run edge to
// edge of their component, so they join the streets that bound it; the
// patterns differ in how they cut (`PATTERN`): a grid keeps a lattice,
// organic cores jitter and jog, curvy suburbs jog and leave cul-de-sacs,
// contour streets sit on level changes, yards are big, ribbons are lanes.

interface PatternParams {
  width: number;
  style: Segment["style"];
  /** Cuts snap to a lattice (grid, yards). */
  lattice: boolean;
  /** Chance an east–west cut jogs a row or two halfway. */
  jog: number;
  /** Chance a north–south cut stops short (a cul-de-sac). */
  stubs: number;
  /** Cut position jitter, tiles. */
  jitter: number;
}

const PATTERN: Record<string, PatternParams> = {
  organic: { width: 4, style: "asphalt", lattice: false, jog: 0.3, stubs: 0, jitter: 2 },
  grid: { width: 4, style: "asphalt", lattice: true, jog: 0, stubs: 0, jitter: 0 },
  curvy: { width: 3, style: "asphalt", lattice: false, jog: 0.6, stubs: 0.45, jitter: 1 },
  contour: { width: 3, style: "asphalt", lattice: false, jog: 0, stubs: 0.15, jitter: 0 },
  yards: { width: 4, style: "asphalt", lattice: true, jog: 0, stubs: 0, jitter: 1 },
  quay: { width: 4, style: "asphalt", lattice: false, jog: 0, stubs: 0, jitter: 1 },
  ribbon: { width: 2, style: "dirt", lattice: false, jog: 0.3, stubs: 0.5, jitter: 3 },
};

export function stageStreets(w: Work) {
  const { W, H } = w;
  const N = W * H;
  const local: Segment[] = [];
  for (const d of w.districts) local.push(...districtStreets(w, d));
  for (const s of local) {
    paintSegment(w, s);
    w.segments.push(s);
  }
  // Open ends: a street that stops at another district's land runs on to the nearest street within reach.
  const road = (c: number) => ((w.use[c] as number) & Use.Road) !== 0;
  const passable = (c: number) => !isWaterBody(w, c) && !((w.use[c] as number) & (Use.Square | Use.Rail | Use.Plot)) && w.terrain[c] !== Terrain.Rock;
  for (const s of local) {
    const width = s.axis === "h" ? s.y1 - s.y0 + 1 : s.x1 - s.x0 + 1;
    for (const dir of [-1, 1] as const) {
      const end = s.axis === "h" ? (dir > 0 ? s.x1 : s.x0) : dir > 0 ? s.y1 : s.y0;
      const line = (a: number) => {
        const out: number[] = [];
        for (let k = 0; k < width; k++) {
          const x = s.axis === "h" ? a : s.x0 + k;
          const y = s.axis === "h" ? s.y0 + k : a;
          out.push(x >= 0 && y >= 0 && x < W && y < H ? y * W + x : -1);
        }
        return out;
      };
      if (line(end + dir).some((c) => c >= 0 && road(c))) continue;
      let reach = -1;
      for (let k = 1; k <= 8; k++) {
        const cells = line(end + dir * k);
        if (cells.some((c) => c < 0 || !passable(c))) break;
        if (line(end + dir * (k + 1)).some((c) => c >= 0 && road(c))) {
          reach = end + dir * k;
          break;
        }
      }
      if (reach < 0) continue;
      const ext: Segment = s.axis === "h" ? { ...s, x0: Math.min(end + dir, reach), x1: Math.max(end + dir, reach) } : { ...s, y0: Math.min(end + dir, reach), y1: Math.max(end + dir, reach) };
      paintSegment(w, ext);
      w.segments.push(ext);
    }
  }
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

/** One district's streets: its land cut into blocks by its pattern. */
function districtStreets(w: Work, d: WorkDistrict): Segment[] {
  const { W, H } = w;
  const N = W * H;
  const t = d.type;
  const pat = PATTERN[t.streets.pattern] ?? (PATTERN.grid as PatternParams);
  const r = rngOf(w, "streets", d.id);
  const sw = pat.width;
  const style: Segment["style"] = t.streets.style === "dirt" || pat.style === "dirt" ? "dirt" : "asphalt";
  const base = { cls: RoadClass.Street, style, district: d.index, arterial: -1 };
  const [bd0, bd1] = t.streets.depth;
  const [bw0, bw1] = t.streets.block;
  const mask = new Uint8Array(N);
  for (let c = 0; c < N; c++) {
    if (w.district[c] !== d.index || isWaterBody(w, c) || (w.use[c] as number) & (Use.Road | Use.Square | Use.Rail | Use.Bridge) || w.terrain[c] === Terrain.Rock) continue;
    mask[c] = 1;
  }
  const segs: Segment[] = [];
  // A lattice for grids: fixed spacing from the district's seed.
  const latticeY = Math.floor(d.seed / W);
  const latticeX = d.seed % W;
  const pY = Math.floor((bd0 + bd1) / 2) + sw;
  const pX = Math.floor((bw0 + bw1) / 2) + sw;
  const snap = (v: number, origin: number, p: number, lo: number, hi: number) => {
    const k = Math.round((v - origin) / p);
    const s = origin + k * p;
    return s >= lo && s <= hi ? s : v;
  };
  const flood = (start: number, seen: Uint8Array): number[] => {
    const out = [start];
    seen[start] = 1;
    for (let i = 0; i < out.length; i++) {
      const c = out[i] as number;
      const x = c % W;
      for (const n of [x > 0 ? c - 1 : -1, x < W - 1 ? c + 1 : -1, c - W, c + W]) {
        if (n < 0 || n >= N || seen[n] || !mask[n]) continue;
        seen[n] = 1;
        out.push(n);
      }
    }
    return out;
  };
  const components = (cells: number[]): number[][] => {
    const seen = new Uint8Array(N);
    const out: number[][] = [];
    for (const c of cells) if (mask[c] && !seen[c]) out.push(flood(c, seen));
    return out;
  };
  /** Cut rows y..y+sw-1 (axis h) or columns x..x+sw-1 (axis v) of a component: its full runs become street pieces. */
  const cut = (axis: "h" | "v", at: number, from: number, to: number, minLen = 3): Segment[] => {
    const made: Segment[] = [];
    let start = -1;
    for (let a = from; a <= to + 1; a++) {
      let ok = a <= to;
      for (let k = 0; k < sw && ok; k++) {
        const x = axis === "h" ? a : at + k;
        const y = axis === "h" ? at + k : a;
        if (x < 0 || y < 0 || x >= W || y >= H || !inComp(y * W + x)) ok = false;
      }
      if (ok && start < 0) start = a;
      if (!ok && start >= 0) {
        if (a - start >= minLen) made.push(axis === "h" ? { ...base, axis, x0: start, x1: a - 1, y0: at, y1: at + sw - 1 } : { ...base, axis, x0: at, x1: at + sw - 1, y0: start, y1: a - 1 });
        start = -1;
      }
    }
    return made;
  };
  const take = (s: Segment) => {
    segs.push(s);
    for (let y = s.y0; y <= s.y1; y++) for (let x = s.x0; x <= s.x1; x++) mask[y * W + x] = 0;
  };
  const stamp = new Int32Array(N);
  let gen = 0;
  const inComp = (c: number) => stamp[c] === gen;
  let queue = components(Array.from({ length: N }, (_, i) => i).filter((c) => mask[c]));
  let guard = 0;
  while (queue.length && guard++ < 4000) {
    const comp = queue.shift() as number[];
    if (comp.length < 30) continue;
    let x0 = W;
    let y0 = H;
    let x1 = -1;
    let y1 = -1;
    for (const c of comp) {
      const x = c % W;
      const y = Math.floor(c / W);
      if (x < x0) x0 = x;
      if (x > x1) x1 = x;
      if (y < y0) y0 = y;
      if (y > y1) y1 = y;
    }
    const bw = x1 - x0 + 1;
    const bh = y1 - y0 + 1;
    const depth = r.int(bd0, bd1);
    const across = r.int(bw0, bw1);
    gen++;
    for (const c of comp) stamp[c] = gen;
    let made: Segment[] = [];
    if (bh >= depth + sw + 5) {
      // An east–west street `depth` below the block's top (the row of buildings fronting it is above).
      let y = y0 + depth + (pat.jitter ? r.int(-pat.jitter, pat.jitter) : 0);
      if (pat.lattice) y = snap(y, latticeY, pY, y0 + bd0, y1 - sw - 4);
      if (t.streets.pattern === "contour") {
        // On the level change, where there is one: the street is the terrace's edge.
        let best = -1;
        let bestN = 0;
        for (let yy = y0 + bd0; yy <= Math.min(y1 - sw - 4, y0 + bd1 + 2); yy++) {
          let n = 0;
          for (let x = x0; x <= x1; x++) if (inComp(yy * W + x) && inComp((yy + sw) * W + x) && w.level[yy * W + x] !== w.level[(yy + sw) * W + x]) n++;
          if (n > bestN) {
            bestN = n;
            best = yy;
          }
        }
        if (best >= 0) y = best;
      }
      y = Math.max(y0 + bd0 - 1, Math.min(y, y1 - sw - 4));
      if (pat.jog && bw >= 18 && r.chance(pat.jog)) {
        const mid = x0 + Math.floor(bw / 2) + r.int(-3, 3);
        const dy = r.pick([-2, -1, 1, 2]);
        const left = cut("h", y, x0, mid + sw - 1, 6);
        const right = cut("h", y + dy, mid, x1, 6);
        if (left.length && right.length) {
          made = [...left, ...right, { ...base, axis: "v", x0: mid, x1: mid + sw - 1, y0: Math.min(y, y + dy), y1: Math.max(y, y + dy) + sw - 1 }];
        }
      }
      if (!made.length) made = cut("h", y, x0, x1, 6);
    }
    if (!made.length && bw >= across + sw + 6) {
      let x = x0 + across + (pat.jitter ? r.int(-pat.jitter, pat.jitter) : 0);
      if (pat.lattice) x = snap(x, latticeX, pX, x0 + bw0, x1 - sw - 5);
      x = Math.max(x0 + 6, Math.min(x, x1 - sw - 5));
      if (pat.stubs && r.chance(pat.stubs) && bh >= 12) {
        // A cul-de-sac in from the bottom (or top) street, stopping short: the block stays whole.
        const fromTop = r.chance(0.5);
        const len = Math.floor((bh * r.int(50, 70)) / 100);
        made = cut("v", x, fromTop ? y0 : y1 - len + 1, fromTop ? y0 + len - 1 : y1, 4);
        for (const s of made) take(s);
        continue;
      }
      made = cut("v", x, y0, y1, 3);
    }
    if (!made.length) continue;
    for (const s of made) take(s);
    queue = queue.concat(components(comp));
  }
  return segs;
}
