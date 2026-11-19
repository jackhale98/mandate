// Layout v2 stages 5–6 (TOWN_GENERATION.md §5): every engine neighbourhood
// becomes one district of a registry type (picked by weighted rendezvous
// hashing among the types whose rules match), sited by its type's
// preferences and grown by a quota-limited flood fill in growth order, with
// rivers, the railway and slopes as preferred edges. The town stops short of
// the map's edge: a margin of countryside (fields, woods, the odd farm)
// that only the roads to the neighbouring towns cross. Stage 6 (./streets)
// grows the streets.
import { hashString, mix } from "../rng";
import { HOOD_KINDS } from "../townInput";
import { Terrain } from "../townLayout";
import { MinHeap, NO_BEARING, compassVec, ipow, isqrt } from "./imath";
import type { GeoCondition, DistrictType, PmRange } from "./registry";
import { LayoutOverflow, RoadClass, Use, WaterKind, countrysideMargin, distanceField, isWaterBody, rngOf, type Work, type WorkDistrict } from "./work";

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
function reachableLand(w: Work, M = 0): Uint8Array {
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
    for (let k4 = 0; k4 < 4; k4++) {
        const n = k4 === 0 ? (x > 0 ? c - 1 : -1) : k4 === 1 ? (x < W - 1 ? c + 1 : -1) : k4 === 2 ? c - W : c + W;
      if (n < 0 || n >= N || reach[n]) continue;
      const nx = n % W;
      const ny = (n - nx) / W;
      if (nx < M || ny < M || nx >= W - M || ny >= H - M) continue;
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
  // Land joined to the square inside the countryside margin (no district within it, nor one reached only through it).
  const reach = reachableLand(w, countrysideMargin(W, H));
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

  const tSeeds = performance.now();
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
  const sites = new Map<number, { prefers: Set<string>; avoids: Set<string> }>();
  const siteOf = (d: WorkDistrict) => {
    let s = sites.get(d.index);
    if (!s) {
      s = { prefers: new Set(d.type.site.prefers), avoids: new Set(d.type.site.avoids ?? []) };
      if (hostsMine(d)) s.prefers.add("toward_minerals");
      if (hostsPort(d)) s.prefers.add("shore");
      sites.set(d.index, s);
    }
    return s;
  };
  const score = (d: WorkDistrict, x: number, y: number, rng: () => number): number => {
    const c = y * W + x;
    const dx = x - cx;
    const dy = y - cy;
    const dist = isqrt(dx * dx + dy * dy);
    const { prefers, avoids } = siteOf(d);
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
    let sep2 = R * R;
    for (const p of seeds) sep2 = Math.min(sep2, (p.x - x) * (p.x - x) + (p.y - y) * (p.y - y));
    const sep = isqrt(sep2);
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
    const step = Math.max(2, Math.floor(W / 40));
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

  w.timings["districts.seeds"] = performance.now() - tSeeds;
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
      for (let k4 = 0; k4 < 4; k4++) {
        const n = k4 === 0 ? (x > 0 ? c - 1 : -1) : k4 === 1 ? (x < W - 1 ? c + 1 : -1) : k4 === 2 ? c - W : c + W;
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
