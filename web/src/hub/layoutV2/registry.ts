// The layout v2 content registries (DEC-090 D4, TOWN_GENERATION.md §6):
// district types, terrain features, connectors and place layout blocks, as
// JSONC under ./registry, typed here and validated by `validateRecipe`.
//
// Stage A keeps them on the client because the content crates are being
// changed elsewhere; stage B moves the four files into mandate-content
// (Rust types, generated schemas, `mandate-cli content validate`) without
// changing their shape, and the engine sends them as `LayoutContentView`.
// A recipe is the set of entries a plan was made with (§6.6, option C): a
// run stores the snapshot of what its towns used, so later content changes
// never rearrange them.
import connectorsSrc from "./registry/connectors.jsonc?raw";
import placesSrc from "./registry/place_layouts.jsonc?raw";
import featuresSrc from "./registry/terrain_features.jsonc?raw";
import districtsSrc from "./registry/town_districts.jsonc?raw";
import { parseJsonc } from "./jsonc";

export const STREET_PATTERNS = ["organic", "grid", "curvy", "contour", "yards", "quay", "ribbon"] as const;
export type StreetPattern = (typeof STREET_PATTERNS)[number];

/** District siting predicates (§5 stage 5). */
export const SITE_PREDICATES = [
  "centre",
  "near_centre",
  "low_slope",
  "floodplain",
  "riverbank",
  "shore",
  "high_ground",
  "arterial",
  "near_rail",
  "by_water",
  "downwind",
  "upwind",
  "edge",
  "toward_minerals",
  "toward_woods",
  "cliff",
] as const;

/** Place placement predicates (§6.3, §6.5); `toward:<feature>` and `near_place:<kind>` take an argument. */
export const PLACE_PREDICATES = [
  "on_square",
  "near_square",
  "central",
  "arterial",
  "edge",
  "by_water",
  "shore",
  "riverbank",
  "near_rail",
  "high_ground",
  "largest_residential",
  "quiet",
] as const;

export const CONNECTOR_KINDS = ["bridge", "footbridge", "ford", "viaduct", "level_crossing"] as const;
export const HOOD_KIND_LIST = ["core", "inner", "waterfront", "industrial", "suburb", "rural"] as const;
export const PLOT_GROUNDS = ["yard", "grass", "field", "garden"] as const;

export type Range = [number, number];
export interface PmRange {
  min?: number;
  max?: number;
}

/** Geography conditions in `appears` (each a field of the geography view, §6.5). */
export interface GeoCondition {
  coast?: boolean;
  river?: boolean;
  port?: boolean;
  lake?: boolean;
  creeks?: boolean;
  shore?: string[];
  fishing_pm?: PmRange;
  minerals_pm?: PmRange;
  ruggedness_pm?: PmRange;
  woods_pm?: PmRange;
  flood_pm?: PmRange;
}

export interface DistrictType {
  id: string;
  name_key: string;
  appears: {
    hood_kinds: string[];
    geo?: GeoCondition;
    population?: PmRange;
    town_class?: string[];
    era?: string[];
    /** Place kinds the town must have (e.g. a mine for pit rows). */
    requires_place?: string[];
    weight: number;
  };
  site: { prefers: string[]; avoids?: string[]; anchor?: string };
  streets: { pattern: StreetPattern; block: Range; depth: Range; lanes: "none" | "back" | "stepped"; max_dead_ends: number; style: "asphalt" | "dirt" };
  plots: { frontage: Range; depth: Range; setback: number; joined: number; ground: (typeof PLOT_GROUNDS)[number] };
  buildings: { mix: [string, number][]; storeys: Range; density: number };
  hosts: string[];
  growth: { reserve: number; fill: "infill_first" | "outward" | "edge_estates"; decline: "boarded" | "derelict" };
  tags: string[];
  art: Record<string, string | string[]>;
}

export interface TerrainFeature {
  id: string;
  generator: string;
  appears: { geo?: GeoCondition };
  params: Record<string, number | boolean | number[] | number[][]>;
  art: Record<string, string>;
}

export interface ConnectorType {
  id: string;
  kind: (typeof CONNECTOR_KINDS)[number];
  max_span: number;
  roads: string[];
  crosses: string[];
  town_class: string[];
  era: string | null;
  art: Record<string, string>;
}

export interface PlaceLayout {
  id: string;
  key: string;
  name_key: string;
  layout: {
    slot: string;
    footprint: [Range, Range];
    place: string[];
    signature?: string;
    with?: string[];
    landmark: boolean;
    art: Record<string, string>;
  };
}

export interface Recipe {
  districts: DistrictType[];
  features: TerrainFeature[];
  connectors: ConnectorType[];
  places: PlaceLayout[];
}

const items = <T>(src: string, kind: string): T[] => {
  const doc = parseJsonc(src) as { kind?: string; items?: T[] };
  if (doc.kind !== kind || !Array.isArray(doc.items)) throw new Error(`layout registry: expected kind "${kind}" with items`);
  return doc.items;
};

/** Read the four registry files (JSONC text) into a recipe. */
export function recipeFromSources(src: { districts: string; features: string; connectors: string; places: string }): Recipe {
  return {
    districts: items<DistrictType>(src.districts, "town_districts"),
    features: items<TerrainFeature>(src.features, "terrain_features"),
    connectors: items<ConnectorType>(src.connectors, "connectors"),
    places: items<PlaceLayout>(src.places, "place_layouts"),
  };
}

/** The base game's registries. */
export const BASE_RECIPE: Recipe = recipeFromSources({ districts: districtsSrc, features: featuresSrc, connectors: connectorsSrc, places: placesSrc });

// ---------------------------------------------------------------------------
// Validation (stage B: generated schemas and `mandate-cli content validate`)

const isRange = (r: unknown, lo = 0, hi = 1e9): r is Range =>
  Array.isArray(r) && r.length === 2 && r.every((v) => typeof v === "number" && Number.isInteger(v) && v >= lo && v <= hi) && (r[0] as number) <= (r[1] as number);
const isShare = (v: unknown) => typeof v === "number" && v >= 0 && v <= 1;
const keyRe = /^[a-z][a-z0-9_]*(\.[a-z0-9_]+)*$/;

function checkGeo(g: GeoCondition | undefined, where: string, out: string[]) {
  if (!g) return;
  const allowed = new Set(["coast", "river", "port", "lake", "creeks", "shore", "fishing_pm", "minerals_pm", "ruggedness_pm", "woods_pm", "flood_pm"]);
  for (const [k, v] of Object.entries(g)) {
    if (!allowed.has(k)) out.push(`${where}: unknown geography condition "${k}"`);
    else if (k.endsWith("_pm")) {
      const r = v as PmRange;
      if (typeof r !== "object" || r === null || Object.keys(r).some((x) => x !== "min" && x !== "max")) out.push(`${where}: ${k} must be { min?, max? }`);
      else if ((r.min ?? 0) < 0 || (r.max ?? 1000) > 1000 || (r.min ?? 0) > (r.max ?? 1000)) out.push(`${where}: ${k} out of 0..1000`);
    } else if (k === "shore") {
      if (!Array.isArray(v) || v.some((s) => !["beach", "rocky", "cliff"].includes(s as string))) out.push(`${where}: shore must list beach | rocky | cliff`);
    } else if (typeof v !== "boolean") out.push(`${where}: ${k} must be true or false`);
  }
}

/** Problems with a recipe (empty: valid). Checks shapes, ranges, references and that every neighbourhood kind has a fallback type. */
export function validateRecipe(r: Recipe): string[] {
  const out: string[] = [];
  const ids = new Set<string>();
  const unique = (id: string, where: string) => {
    if (typeof id !== "string" || !keyRe.test(id)) out.push(`${where}: bad id ${JSON.stringify(id)}`);
    if (ids.has(id)) out.push(`${where}: duplicate id ${id}`);
    ids.add(id);
  };
  for (const d of r.districts) {
    const w = d.id ?? "district";
    unique(d.id, "town_districts");
    if (!/^district-[a-z0-9_]+$/.test(d.name_key ?? "")) out.push(`${w}: name_key must be district-<name>`);
    if (!Array.isArray(d.appears?.hood_kinds) || d.appears.hood_kinds.length === 0) out.push(`${w}: appears.hood_kinds is empty`);
    else for (const k of d.appears.hood_kinds) if (!(HOOD_KIND_LIST as readonly string[]).includes(k)) out.push(`${w}: unknown neighbourhood kind ${k}`);
    if (!Number.isInteger(d.appears?.weight) || d.appears.weight < 1) out.push(`${w}: appears.weight must be a positive integer`);
    checkGeo(d.appears?.geo, w, out);
    const pop = d.appears?.population;
    if (pop && (pop.min ?? 0) > (pop.max ?? Infinity)) out.push(`${w}: population min > max`);
    for (const p of [...(d.site?.prefers ?? []), ...(d.site?.avoids ?? [])]) if (!(SITE_PREDICATES as readonly string[]).includes(p)) out.push(`${w}: unknown site predicate ${p}`);
    if (!(STREET_PATTERNS as readonly string[]).includes(d.streets?.pattern)) out.push(`${w}: unknown street pattern ${d.streets?.pattern}`);
    if (!isRange(d.streets?.block, 6, 40)) out.push(`${w}: streets.block must be a range within 6..40 (§8: minimum block 6 × 5)`);
    if (!isRange(d.streets?.depth, 5, 20)) out.push(`${w}: streets.depth must be a range within 5..20`);
    if (!["none", "back", "stepped"].includes(d.streets?.lanes)) out.push(`${w}: streets.lanes must be none | back | stepped`);
    if (!Number.isInteger(d.streets?.max_dead_ends) || d.streets.max_dead_ends < 0) out.push(`${w}: streets.max_dead_ends must be a non-negative integer`);
    if (!["asphalt", "dirt"].includes(d.streets?.style)) out.push(`${w}: streets.style must be asphalt | dirt`);
    if (!isRange(d.plots?.frontage, 2, 12)) out.push(`${w}: plots.frontage must be a range within 2..12`);
    if (!isRange(d.plots?.depth, 2, 8)) out.push(`${w}: plots.depth must be a range within 2..8`);
    if (!Number.isInteger(d.plots?.setback) || d.plots.setback < 0 || d.plots.setback > 2) out.push(`${w}: plots.setback must be 0..2`);
    if (!isShare(d.plots?.joined)) out.push(`${w}: plots.joined must be 0..1`);
    if (!(PLOT_GROUNDS as readonly string[]).includes(d.plots?.ground)) out.push(`${w}: plots.ground must be ${PLOT_GROUNDS.join(" | ")}`);
    if (!Array.isArray(d.buildings?.mix) || d.buildings.mix.length === 0 || d.buildings.mix.some((m) => !Array.isArray(m) || typeof m[0] !== "string" || !(m[1] > 0))) out.push(`${w}: buildings.mix must be [kind, weight > 0] pairs`);
    if (!isRange(d.buildings?.storeys, 1, 8)) out.push(`${w}: buildings.storeys must be a range within 1..8`);
    if (!isShare(d.buildings?.density)) out.push(`${w}: buildings.density must be 0..1`);
    if (!isShare(d.growth?.reserve) || d.growth.reserve > 0.5) out.push(`${w}: growth.reserve must be 0..0.5`);
    if (!["infill_first", "outward", "edge_estates"].includes(d.growth?.fill)) out.push(`${w}: growth.fill must be infill_first | outward | edge_estates`);
    if (!["boarded", "derelict"].includes(d.growth?.decline)) out.push(`${w}: growth.decline must be boarded | derelict`);
    if (!Array.isArray(d.hosts) || !Array.isArray(d.tags)) out.push(`${w}: hosts and tags must be lists`);
    if (typeof d.art?.family !== "string" || d.art.family === "") out.push(`${w}: art.family is required (the last-resort art parent, §7.2)`);
  }
  for (const kind of HOOD_KIND_LIST) {
    if (!r.districts.some((d) => d.appears?.hood_kinds?.includes(kind) && !d.appears.geo && !d.appears.population && !d.appears.town_class && !d.appears.era && !d.appears.requires_place))
      out.push(`town_districts: no unconditional type for neighbourhood kind ${kind} (a pick must always exist)`);
  }
  const generators = new Set(["river", "creek", "coast", "lake", "hills", "cliff", "woods", "marsh", "beach"]);
  for (const f of r.features) {
    unique(f.id, "terrain_features");
    if (!generators.has(f.generator)) out.push(`${f.id}: unknown generator ${f.generator}`);
    checkGeo(f.appears?.geo, f.id, out);
    if (typeof f.params !== "object" || f.params === null) out.push(`${f.id}: params must be an object`);
    for (const [k, v] of Object.entries(f.params ?? {})) {
      const nums = (Array.isArray(v) ? v.flat() : [v]) as unknown[];
      if (nums.some((n) => typeof n !== "boolean" && (typeof n !== "number" || !Number.isInteger(n)))) out.push(`${f.id}: params.${k} must be integers (the generator is integer-only)`);
    }
  }
  for (const g of ["river", "creek", "coast", "hills"]) if (!r.features.some((f) => f.generator === g)) out.push(`terrain_features: the base game needs a ${g} feature`);
  for (const c of r.connectors) {
    unique(c.id, "connectors");
    if (!(CONNECTOR_KINDS as readonly string[]).includes(c.kind)) out.push(`${c.id}: unknown connector kind ${c.kind}`);
    if (!Number.isInteger(c.max_span) || c.max_span < 1 || c.max_span > 32) out.push(`${c.id}: max_span must be 1..32`);
    if (!Array.isArray(c.roads) || c.roads.some((x) => !["arterial", "street", "lane", "path", "rail"].includes(x))) out.push(`${c.id}: roads must list arterial | street | lane | path | rail`);
    if (!Array.isArray(c.crosses) || c.crosses.some((x) => !["river", "sea", "lake", "creek", "rail"].includes(x))) out.push(`${c.id}: crosses must list river | sea | lake | creek | rail`);
    if (!Array.isArray(c.town_class)) out.push(`${c.id}: town_class must be a list`);
  }
  for (const need of ["bridge", "level_crossing", "ford"]) if (!r.connectors.some((c) => c.kind === need)) out.push(`connectors: the base game needs a ${need}`);
  for (const p of r.places) {
    unique(p.id, "place_layouts");
    const l = p.layout;
    if (!l) {
      out.push(`${p.id}: layout block missing`);
      continue;
    }
    const pair = (v: unknown) => Array.isArray(v) && v.length === 2 && v.every((n) => Number.isInteger(n) && n >= 2 && n <= 16);
    if (!pair(l.footprint?.[0]) || !pair(l.footprint?.[1])) out.push(`${p.id}: footprint must be [[w, h], [w, h]] within 2..16`);
    else if (l.footprint[0][0] > l.footprint[1][0] || l.footprint[0][1] > l.footprint[1][1]) out.push(`${p.id}: footprint min exceeds max`);
    for (const pr of l.place ?? []) {
      const base = pr.split(":")[0] as string;
      if (base === "toward" || base === "near_place") continue;
      if (!(PLACE_PREDICATES as readonly string[]).includes(pr)) out.push(`${p.id}: unknown placement predicate ${pr}`);
    }
    if (typeof l.landmark !== "boolean") out.push(`${p.id}: landmark must be true or false`);
    if (typeof l.art?.family !== "string") out.push(`${p.id}: art.family is required`);
  }
  return out;
}

/** The registry entries a plan used (§6.6 option C): what a run stores beside LAYOUT_VERSION for a town. */
export function snapshotRecipe(r: Recipe, used: { districts: Iterable<string>; features: Iterable<string>; connectors: Iterable<string>; places: Iterable<string> }): Recipe {
  const pick = <T extends { id: string }>(all: T[], ids: Iterable<string>) => {
    const s = new Set(ids);
    return all.filter((x) => s.has(x.id));
  };
  // Picks are maxima (rendezvous) or first matches in registry order, so a plan made
  // from the snapshot makes the same picks as one made from the full registry.
  return {
    districts: pick(r.districts, used.districts),
    features: pick(r.features, used.features),
    connectors: pick(r.connectors, used.connectors),
    places: r.places.filter((p) => new Set(used.places).has(p.key)),
  };
}
