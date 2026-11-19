// Layout v2's inputs (DEC-090, TOWN_GENERATION.md §4.1, §4.2).
//
// The engine sends both (task E2/E3): `TownGeographyView` (static for a run,
// quantised) and `TownStateView` (monthly, rounded unless the player is at
// their own town hall, DEC-079). `geoFromEngine` and `stateFromEngine` map
// them onto the shapes the plan and the dress pass read; callers prefer them.
//
// `geoFromViews` is the stage A derivation, kept only as a fallback for
// callers without the engine views (tests and old builds): it reads the
// municipality view and the national map's rasters and links, with known
// gaps (no elevation raster or flow direction, current populations for
// baselines, no adjacency or sector jobs).
import type { MapView } from "../engine/protocol/MapView";
import type { MunicipalityView } from "../engine/protocol/MunicipalityView";
import type { TownCharacterView } from "../engine/protocol/TownCharacterView";
import type { TownGeographyView } from "../engine/protocol/TownGeographyView";
import type { TownStateView } from "../engine/protocol/TownStateView";
import type { TownState } from "./layoutV2/dress";
import { NO_BEARING, bearingGap, bearingOf, opposite, type Compass16 } from "./layoutV2/imath";
import type { GeoLink, TownClass, TownGeo } from "./layoutV2/geo";
import type { TownInput, TownLocation } from "./townInput";

const decode = (b64: string): Uint8Array => {
  const s = atob(b64);
  const out = new Uint8Array(s.length);
  for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i);
  return out;
};

const pm = (v: number | undefined) => Math.max(0, Math.min(1000, Math.round((v ?? 0) * 1000)));

/** Decoded rasters of the national map, kept per map view (several towns of one world share them). */
interface Rasters {
  w: number;
  h: number;
  cover: Uint8Array;
  river: Uint8Array;
  shade: Uint8Array;
}
const rasterCache = new WeakMap<MapView, Rasters>();
function rasters(map: MapView): Rasters {
  let r = rasterCache.get(map);
  if (!r) {
    r = { w: map.width, h: map.height, cover: decode(map.cover_b64), river: decode(map.river_b64), shade: decode(map.shade_b64) };
    rasterCache.set(map, r);
  }
  return r;
}

export interface GeoSources {
  municipality: MunicipalityView;
  map: MapView;
  /** The town as the layout sees it (places and neighbourhood names; the visit or the player's town). */
  input: TownInput;
  character?: TownCharacterView | null;
}

/**
 * The geography of one town for layout v2, from the engine's view. The
 * places are the town's as the layout sees them (the visit, or the player's
 * town); the engine view carries everything else.
 */
export function geoFromEngine(v: TownGeographyView, input: Pick<TownInput, "places">): TownGeo {
  const r = v.river;
  return {
    municipality: v.municipality,
    name: v.name,
    class: v.class,
    regional_capital: v.regional_capital,
    national_capital: v.national_capital,
    baseline_population: Math.max(1, v.baseline_population),
    neighborhoods: v.neighborhoods.map((h) => ({ id: h.id, name: h.name, kind: h.kind, baseline_population: Math.max(1, h.baseline_population), adjacent: [...h.adjacent] })),
    places: input.places.map((p) => ({ id: p.id, kind: p.kind, name: p.name, neighborhood: p.neighborhood })),
    baseline_sectors: v.baseline_sectors.map((s) => ({ sector: s.sector, driver: s.driver, jobs: s.jobs })),
    river: r ? { class: Math.max(1, Math.min(3, r.class)) as 1 | 2 | 3, enters: r.enters, leaves: r.leaves, through_centre: r.through_centre } : null,
    creeks: v.creeks.map((c) => ({ enters: c.enters, joins_river: c.joins_river })),
    coast: v.coast ? { sea: v.coast.sea, shore: v.coast.shore, bay: v.coast.bay } : null,
    lake: v.lake ? { bearing: v.lake.bearing, near: v.lake.near } : null,
    relief: { ...v.relief },
    woods: { ...v.woods },
    minerals: { ...v.minerals },
    resources_pm: { ...v.resources_pm },
    links: v.links.map((l) => ({ mode: l.mode, bearing: l.bearing, to: l.to, to_name: l.to_name, foreign: l.foreign })),
    hazards_pm: { ...v.hazards_pm },
    climate: { ...v.climate },
    wind_from: v.wind_from,
    elevation_m: v.elevation_m,
    era: v.era,
  };
}

/** The dress state of one town from the engine's monthly view. */
export function stateFromEngine(v: TownStateView, home: number | null, location: TownLocation): TownState {
  return {
    home,
    location,
    hoods: v.neighborhoods.map((h) => ({ id: h.id, population: h.population })),
    vacancy: v.housing.vacancy_pm / 1000,
    unemployment: v.unemployment_pm / 1000,
    housingPm: v.housing.stock_pm,
    pipelinePm: v.housing.pipeline_pm,
    closedPlaces: v.places.filter((p) => p.status === "closed").map((p) => p.id),
  };
}

/** Stage A fallback: the geography derived from the municipality and map views (see the header). */
export function geoFromViews(src: GeoSources): TownGeo {
  const mv = src.municipality;
  const map = src.map;
  const R = rasters(map);
  const mm = map.municipalities.find((m) => m.id === mv.id);
  const cx = mm?.x ?? Math.floor(R.w / 2);
  const cy = mm?.y ?? Math.floor(R.h / 2);
  const at = (a: Uint8Array, x: number, y: number) => (x < 0 || y < 0 || x >= R.w || y >= R.h ? 0 : (a[y * R.w + x] as number));
  const res = Object.fromEntries(mv.resources.map((r) => [r.key, pm(r.value)]));
  const haz = Object.fromEntries(mv.hazards.map((r) => [r.key, pm(r.value)]));

  // ---- the sea, a lake: offsets of their cells around the centre
  const sum = (test: (x: number, y: number) => boolean, rad: number) => {
    let sx = 0;
    let sy = 0;
    let n = 0;
    const octants = new Set<number>();
    for (let dy = -rad; dy <= rad; dy++) {
      for (let dx = -rad; dx <= rad; dx++) {
        if ((dx === 0 && dy === 0) || !test(cx + dx, cy + dy)) continue;
        sx += dx;
        sy += dy;
        n++;
        octants.add(bearingOf(dx, dy) >> 1);
      }
    }
    return { sx, sy, n, octants: octants.size };
  };
  const isSea = (x: number, y: number) => at(R.cover, x, y) === 0 && x >= 0 && y >= 0 && x < R.w && y < R.h;
  let sea = sum(isSea, 3);
  for (let rad = 4; sea.n === 0 && mv.coastal && rad <= 10; rad += 2) sea = sum(isSea, rad);

  // ---- relief from the hillshade: the NW→SE elevation difference per cell
  let shadeSum = 0;
  let shadeAbs = 0;
  let land = 0;
  for (let dy = -2; dy <= 2; dy++) {
    for (let dx = -2; dx <= 2; dx++) {
      if (isSea(cx + dx, cy + dy)) continue;
      const d = at(R.shade, cx + dx, cy + dy) - 128;
      shadeSum += d;
      shadeAbs += Math.abs(d);
      land++;
    }
  }
  const meanAbs = land ? Math.floor(shadeAbs / land) : 0;
  const meanSigned = land ? Math.trunc(shadeSum / land) : 0;
  // Mountain towns are rugged even where the shade is even; the shade's swing does the rest.
  const ruggedness = Math.max(0, Math.min(1000, meanAbs * 11 + Math.floor(Math.max(0, mv.elevation_m - 150) / 2)));
  let ux = meanSigned;
  let uy = meanSigned;
  if (sea.n > 0) {
    // Land rises away from the sea.
    ux -= Math.sign(sea.sx) * 30;
    uy -= Math.sign(sea.sy) * 30;
  }
  const uphill: Compass16 = ruggedness < 80 ? NO_BEARING : bearingOf(ux, uy) === NO_BEARING ? 6 : bearingOf(ux, uy);

  // ---- the river: the axis of the strongest class's cells in a 7×7 window
  let maxClass = 0;
  for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) maxClass = Math.max(maxClass, at(R.river, cx + dx, cy + dy));
  let river: TownGeo["river"] = null;
  const creeks: TownGeo["creeks"] = [];
  if (maxClass > 0 || mv.on_river) {
    const cls = Math.max(1, maxClass);
    const cells: [number, number][] = [];
    for (let dy = -3; dy <= 3; dy++) for (let dx = -3; dx <= 3; dx++) if (at(R.river, cx + dx, cy + dy) >= cls) cells.push([dx, dy]);
    let axis = 0;
    let best = -1;
    for (let k = 0; k < 8; k++) {
      // Spread of the cells along bearing k (×1024 vectors; integer).
      const v = [
        [0, -1024],
        [392, -946],
        [724, -724],
        [946, -392],
        [1024, 0],
        [946, 392],
        [724, 724],
        [392, 946],
      ][k] as [number, number];
      let s = 0;
      for (const [dx, dy] of cells) {
        const p = dx * v[0] + dy * v[1];
        s += p * p;
      }
      if (s > best) {
        best = s;
        axis = k;
      }
    }
    let enters: Compass16 = axis;
    // Downstream toward the sea when there is one: it leaves on the end nearer the sea.
    if (sea.n > 0) {
      const toSea = bearingOf(sea.sx, sea.sy);
      if (bearingGap(axis, toSea) < bearingGap(opposite(axis), toSea)) enters = opposite(axis);
    }
    const through = cells.some(([dx, dy]) => Math.abs(dx) <= 1 && Math.abs(dy) <= 1);
    river = { class: Math.min(3, cls) as 1 | 2 | 3, enters, leaves: opposite(enters), through_centre: through };
    // Creeks: class-1 streams in the 5×5 that are not the main river.
    if (cls >= 2) {
      const seen = new Set<number>();
      for (let dy = -2; dy <= 2; dy++) {
        for (let dx = -2; dx <= 2; dx++) {
          if (at(R.river, cx + dx, cy + dy) !== 1) continue;
          const b = bearingOf(dx, dy);
          if (b === NO_BEARING || seen.has(b >> 2) || (b & 7) === (enters & 7)) continue;
          seen.add(b >> 2);
          if (creeks.length < 2) creeks.push({ enters: b, joins_river: true });
        }
      }
    }
  }

  // ---- coast
  let coast: TownGeo["coast"] = null;
  if (sea.n > 0 || mv.coastal) {
    const b = sea.n > 0 ? bearingOf(sea.sx, sea.sy) : uphill === NO_BEARING ? 8 : opposite(uphill);
    coast = { sea: b === NO_BEARING ? 8 : b, shore: ruggedness > 700 ? "cliff" : ruggedness > 420 ? "rocky" : "beach", bay: sea.octants >= 3 && sea.n <= 24 };
  }
  // ---- a lake
  const lakeCells = sum((x, y) => at(R.cover, x, y) === 3, 3);
  const lake: TownGeo["lake"] = lakeCells.n > 0 ? { bearing: bearingOf(lakeCells.sx, lakeCells.sy) === NO_BEARING ? 0 : bearingOf(lakeCells.sx, lakeCells.sy), near: lakeCells.n >= 3 } : null;

  // ---- links: the transport links that end at (or pass through) the centre cell
  const centreCell = cy * R.w + cx;
  const byCell = new Map(map.municipalities.map((m) => [m.y * R.w + m.x, m]));
  const links: GeoLink[] = [];
  const addLink = (mode: string, path: readonly number[], from: number) => {
    const step = from === 0 ? 1 : -1;
    // Bearing toward a cell a few steps along the path (the first step may be diagonal noise).
    const k = Math.max(0, Math.min(path.length - 1, from + step * Math.min(4, path.length - 1)));
    const c = path[k] as number;
    const b = bearingOf((c % R.w) - cx, Math.floor(c / R.w) - cy);
    if (b === NO_BEARING) return;
    const end = byCell.get(path[from === 0 ? path.length - 1 : 0] as number);
    const m = mode === "highway" || mode === "rail" ? mode : "road";
    if (links.some((l) => l.mode === m && l.bearing === b)) return;
    links.push({ mode: m, bearing: b, to: end?.id ?? -1, to_name: end?.name ?? "", foreign: false });
  };
  for (const l of map.links) {
    const p = l.path;
    if (p.length < 2) continue;
    if (p[0] === centreCell) addLink(l.mode, p, 0);
    else if (p[p.length - 1] === centreCell) addLink(l.mode, p, p.length - 1);
    else {
      const i = p.indexOf(centreCell);
      if (i > 0) {
        addLink(l.mode, p.slice(i), 0);
        addLink(l.mode, p.slice(0, i + 1), i);
      }
    }
  }
  links.sort((a, b) => a.bearing - b.bearing || a.mode.localeCompare(b.mode) || a.to - b.to);

  // ---- woods and minerals: shares from the resource indexes, bearings toward high ground
  const forestBiome = /forest|taiga|woodland|rainforest/.test(mv.biome);
  const woodsShare = Math.min(1000, (res.timber ?? 0) + (forestBiome ? 250 : 0));
  const high = uphill === NO_BEARING ? (mv.id * 5 + 3) & 15 : uphill;

  const cls = (mm?.class ?? mv.class) as TownClass;
  const population = Math.max(1, Math.round(mv.population));
  const industry = src.character?.industry;
  return {
    municipality: mv.id,
    name: mv.name,
    class: ["metro", "city", "town", "rural"].includes(cls) ? cls : "town",
    regional_capital: mm?.regional_capital ?? false,
    national_capital: mm?.national_capital ?? false,
    baseline_population: population,
    neighborhoods: mv.neighborhoods.map((h) => ({ id: h.id, name: h.name, kind: h.kind, baseline_population: Math.max(1, Math.round(h.population)), adjacent: [] })),
    places: src.input.places.map((p) => ({ id: p.id, kind: p.kind, name: p.name, neighborhood: p.neighborhood })),
    baseline_sectors: industry ? [{ sector: industry.id, driver: "", jobs: Math.round(population * 0.45 * (src.character?.industry_share ?? 0.2)) }] : [],
    river,
    creeks,
    coast,
    lake,
    relief: { ruggedness_pm: ruggedness, uphill, high_ground: high, relief_m: Math.round((ruggedness * 120) / 1000) },
    woods: { share_pm: woodsShare, bearing: high },
    minerals: { value_pm: res.minerals ?? 0, bearing: ((high + 2) & 15) as Compass16 },
    resources_pm: res,
    links,
    hazards_pm: { flood: haz.flood ?? 0, storm: haz.storm ?? 0, wildfire: haz.wildfire ?? 0, seismic: haz.seismic ?? 0, drought: haz.drought ?? 0 },
    climate: { temperature_c: Math.round(mv.temperature_c), precipitation_mm: Math.round(mv.precipitation_mm), biome: mv.biome, snow: mv.temperature_c < 2 },
    // TODO(D3): prevailing wind is not in any view yet; a westerly.
    wind_from: 12,
    elevation_m: Math.round(mv.elevation_m),
    era: null,
  };
}
