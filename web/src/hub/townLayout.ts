// Town layout for the walkable hub (DEC-009, DEC-079, DEC-080; M6 groundwork).
// `layoutTown(input, seed)` turns the town the engine describes (its
// neighbourhoods and places) into a tile map: streets, neighbourhood
// districts, a building with a door for every place, filler houses and
// shops, a central square with the town hall and council chamber on it, and
// optional water. Pure and deterministic for (input, seed, LAYOUT_VERSION):
// no Math.random, no clock, keyed random streams (./rng). The engine is the
// authority on what a town contains; this only decides where it is drawn
// (walking is UI state, DEC-009).
//
// Coordinates are tiles (16 art px each); cell index = y * width + x, the
// same as ./pathfinding. Doors all face south (the visible side in the
// three-quarter view, DEC-049): a building's door tile is in its bottom row
// and the walkable cell below it is where routes to it end.
import { buildGrid, type Footprint, type Grid, type Point, type Surface } from "./pathfinding";
import { mix, stream, type Rng } from "./rng";
import { HOOD_KINDS, type HoodKind, type TownInput, type TownLocation } from "./townInput";

/**
 * Version of the layout algorithm. A saved run's town must not rearrange
 * itself after an app update, so any change that moves a building, a street
 * or a district for the same (input, seed) bumps this, and the old version is
 * kept callable for towns first laid out with it (see DEC-085 proposal).
 * `townLayout.test.ts` pins map hashes of the committed fixtures to catch
 * accidental changes.
 */
export const LAYOUT_VERSION = 1;

/** Art pixels per tile. */
export const TILE = 16;

export const Terrain = {
  Grass: 0,
  Road: 1,
  Pavement: 2,
  Crossing: 3,
  Plaza: 4,
  Water: 5,
  Building: 6,
  Door: 7,
  Park: 8,
  /** Garden paths, lanes between building rows, rural door paths. */
  Path: 9,
  /** Paved back yards and industrial lots. */
  Yard: 10,
  Field: 11,
  /** Country lanes (rural streets): walked like a pavement. */
  Dirt: 12,
  Sand: 13,
  /** The promenade along the water. */
  Quay: 14,
  Pier: 15,
  /** Front gardens (open lawn). */
  Garden: 16,
  // ---- Layout v2 vocabulary (DEC-090; TOWN_GENERATION.md §7.7). v1 never emits these.
  // Codes must stay below 32 (the layout hash packs terrain into 5 bits).
  /** A one-tile stream (its own autotile set; stepping stones and fords cross it). */
  Creek: 17,
  /** Bare rock: cliff tops, quarry and pit floors. Not walkable. */
  Rock: 18,
  /** A built water edge or retaining wall face (urban banks, level changes facing south). Not walkable. */
  Embankment: 19,
  /** Railway track (an autotile with 45° pieces). Crossed only at a level crossing (`Crossing`). */
  Rail: 20,
  /** Reeds and marsh on rural banks: walkable, slow. */
  Reeds: 21,
  /** A bridge deck over water, rail or a creek. */
  Bridge: 22,
  /** A shallow crossing of a creek: walkable, slow. */
  Ford: 23,
  /** An open pit or quarry floor below a rock face. Not walkable. */
  Pit: 24,
  /** Steps or a ramp where a path crosses a level change. */
  Steps: 25,
} as const;
export type TerrainCode = (typeof Terrain)[keyof typeof Terrain];

/** How a pedestrian treats each terrain (feeds ./pathfinding's costs). */
export function surfaceOfTerrain(t: number): Surface {
  switch (t) {
    case Terrain.Road:
      return "road";
    case Terrain.Crossing:
      return "crossing";
    case Terrain.Pavement:
    case Terrain.Plaza:
    case Terrain.Path:
    case Terrain.Yard:
    case Terrain.Dirt:
    case Terrain.Quay:
    case Terrain.Pier:
    case Terrain.Bridge:
    case Terrain.Steps:
      return "pavement";
    case Terrain.Grass:
    case Terrain.Park:
    case Terrain.Field:
    case Terrain.Sand:
    case Terrain.Garden:
    case Terrain.Reeds:
    case Terrain.Ford:
      return "grass";
    default:
      return "blocked";
  }
}

export type FillerKind = "house" | "terrace" | "shop" | "warehouse" | "shed" | "farmhouse" | "barn" | "boathouse" | "home";

export interface TownBuilding {
  index: number;
  /** Footprint, tiles. The roof is drawn one row above it (`ROOF_ROWS`). */
  x: number;
  y: number;
  w: number;
  h: number;
  /** The door tile, in the footprint's bottom row. */
  doorX: number;
  doorY: number;
  /** The walkable cell in front of the door: where a route to this building ends. */
  front: number;
  /** Engine place id, or null for a filler (or the player's home). */
  place: number | null;
  /** A place-registry key for places, else a filler kind. */
  kind: string;
  /** The place's generated proper name, or "". */
  name: string;
  /** Index into `TownMap.districts`. */
  district: number;
  home: boolean;
  /** Civic landmark (larger, on or near the square). */
  landmark: boolean;
  /** Stable art variation. */
  style: number;
  /** Layout v2 only: the plot's dress state; v1 buildings are built. */
  state?: PlotState;
}

export interface TownDistrict {
  index: number;
  /** Engine neighbourhood id. */
  id: number;
  name: string;
  kind: HoodKind;
  /** Label anchor (centroid of its blocks), tiles. */
  centre: Point;
  /** A walkable cell near the centre: "the streets of" this neighbourhood. */
  street: number;
  cells: number;
  bounds: { x: number; y: number; w: number; h: number };
}

export type PropKind =
  | "tree"
  | "pine"
  | "lamp"
  | "bench"
  | "fountain"
  | "boat"
  | "crane"
  | "planter"
  | "flag"
  | "stall"
  // Layout v2 (DEC-090): a mine's winding tower and spoil heap; bank reeds; creek stepping stones.
  | "headframe"
  | "spoil_heap"
  | "reeds"
  | "stepping_stones"
  // Street ends (DEC-090 D5 amended): a row of bollards closing a road to traffic (a footpath carries on);
  // a field gate where a road or track meets farmland; a railing where a road meets water (quay end, bank).
  | "bollards"
  | "field_gate"
  | "railing";

/**
 * Layout v2: how a street that does not join another street ends. Every such end
 * must be one of these (no street just stops); the generator records them, the
 * art draws them, and the checks enforce them.
 */
export type StreetEndKind =
  /** A round turning circle (the road widens into a rounded bulb). */
  | "turning_circle"
  /** A T- or L-shaped turning head. */
  | "hammerhead"
  /** Bollards across the road; a footpath carries on beyond them. */
  | "bollards"
  /** The road's paving runs into a square or market. */
  | "plaza"
  /** A field gate onto farmland, or a road becoming a farm track. */
  | "field_gate"
  /** A railing at a quay end or riverbank. */
  | "railing";

/** Layout v2: one street end (the last road cells of the street, tiles). */
export interface TownStreetEnd {
  kind: StreetEndKind;
  x: number;
  y: number;
  w: number;
  h: number;
  /** The direction the street runs into its end: "n", "e", "s", "w", or a diagonal "ne", "nw", "se", "sw". */
  facing: "n" | "e" | "s" | "w" | "ne" | "nw" | "se" | "sw";
}

/** Layout v2 (DEC-090): what crosses water, rail or a creek. The cells it covers are `Bridge`, `Ford` or `Crossing`. */
export interface TownConnector {
  kind: "bridge" | "footbridge" | "ford" | "viaduct" | "level_crossing";
  /** Covered cells, tiles. */
  x: number;
  y: number;
  w: number;
  h: number;
  /** "ew": runs east–west (its south face, the arches, shows); "ns": runs north–south (the deck and parapets show). */
  axis: "ew" | "ns";
  /** A generated proper name ("the Old Bridge"), or "". */
  name: string;
}

/** Layout v2 street directions per cell (`TownMap.streetAxis`). */
export const StreetAxis = {
  /** Orthogonal, or not a street: the shape comes from the neighbours. */
  Orthogonal: 0,
  /** A diagonal street running north-east to south-west ("/"). */
  DiagonalNE: 1,
  /** A diagonal street running north-west to south-east ("\\"). */
  DiagonalNW: 2,
  /** A junction where a diagonal street meets another street. */
  Junction: 3,
} as const;

/** Layout v2 dress state of a building plot (DEC-090 D2). Absent on v1 maps, which are always "built". */
export type PlotState = "built" | "reserved" | "construction" | "boarded" | "derelict";

export interface TownProp {
  kind: PropKind;
  x: number;
  y: number;
  /** Whether it blocks walking (trees, the fountain); lamps and benches do not. */
  blocks: boolean;
  variant: number;
}

export type WaterSide = "north" | "south" | "east" | "west";

export interface TownMap {
  version: number;
  seed: number;
  municipality: string;
  width: number;
  height: number;
  /** Terrain code per cell. */
  terrain: Uint8Array;
  /** Layout v2 only: ground level per cell (0–3, about a storey each; DEC-090, TOWN_GENERATION.md §7.7). Absent means flat. */
  elevation?: Uint8Array;
  /** Layout v2 only: bridges, fords, viaducts and level crossings. */
  connectors?: TownConnector[];
  /**
   * Layout v2 only (street rework A + B): the direction of the street a road,
   * crossing or pavement cell belongs to: `StreetAxis` codes. Absent or 0 means
   * orthogonal (the art reads the shape from neighbours, as v1). Diagonal cells
   * form 45° staircases; the art draws them as smooth 45° roads and kerbs.
   */
  streetAxis?: Uint8Array;
  /** Layout v2 only: how every street that does not join another street ends (`TownStreetEnd`). */
  streetEnds?: TownStreetEnd[];
  /** 1 where walking is blocked (buildings, water, trees, the fountain). */
  solid: Uint8Array;
  /** District index per cell, -1 outside the town. */
  district: Int16Array;
  buildings: TownBuilding[];
  districts: TownDistrict[];
  props: TownProp[];
  /** Engine place id → building index, sorted by place id. */
  places: { place: number; building: number }[];
  /** The central square's open plaza, tiles. */
  square: { x: number; y: number; w: number; h: number } | null;
  water: WaterSide | null;
  /** The player's house. */
  homeBuilding: number | null;
  /** Where the player stands at the start (the cell for `input.location`). */
  start: number;
}

/** Roof rows drawn above a footprint (tappable, and they hide walkers behind them). */
export const ROOF_ROWS = 1;

const STREET = 4;
const MARGIN = 3;
const QUAY = 2;
const WATER = 7;
const CIVIC = ["town_hall", "council_chamber", "parliament", "regional_assembly", "executive_residence"];
/** Kinds that want to be near the square when their neighbourhood allows. */
const CENTRAL = new Set([...CIVIC, "news_office", "party_office", "hotel", "business_association", "market"]);
const OUTSKIRTS = new Set(["factory", "mine", "mill", "farm", "port"]);

/** Footprints (w × h tiles) by place kind: landmarks large, everyday places small. */
const PLACE_SIZE: Record<string, readonly [number, number]> = {
  town_hall: [9, 5],
  council_chamber: [7, 5],
  parliament: [11, 6],
  regional_assembly: [9, 5],
  executive_residence: [9, 5],
  university: [9, 5],
  tech_campus: [8, 5],
  factory: [9, 5],
  port: [8, 4],
  mine: [6, 4],
  mill: [6, 5],
  farm: [6, 4],
  hotel: [5, 5],
  school: [7, 4],
  clinic: [5, 4],
  community_centre: [6, 4],
  market: [6, 3],
  union_hall: [5, 4],
  business_association: [5, 4],
  news_office: [4, 4],
  party_office: [4, 3],
  diner: [4, 3],
  park: [3, 2],
};
export function placeSize(kind: string): readonly [number, number] {
  return PLACE_SIZE[kind] ?? [4, 3];
}

/**
 * Euclidean length with a correctly rounded square root: `Math.hypot` is not
 * specified bit-exactly and differs between engines, which could rearrange a
 * town between browsers.
 */
const len = (dx: number, dy: number) => Math.sqrt(dx * dx + dy * dy);

const hoodKind = (k: string): HoodKind => (HOOD_KINDS as readonly string[]).includes(k) ? (k as HoodKind) : "inner";

// ---------------------------------------------------------------------------

class LayoutOverflow extends Error {}

type SegStyle = "asphalt" | "dirt" | "lot" | "green";

interface Row {
  x0: number;
  x1: number;
  /** The walkable row the doors open onto. */
  front: number;
  /** Highest row a building may use. */
  top: number;
  /** 1 = a front-garden row between the buildings and `front`. */
  inset: number;
  occ: [number, number][];
}

interface Lot {
  i0: number;
  i1: number;
  j: number;
  x0: number;
  x1: number;
  y0: number;
  y1: number;
  hood: number;
  rows: Row[];
  buildings: number[];
  named: number;
  square: boolean;
  park: boolean;
  waterAdj: boolean;
}

interface Hood {
  index: number;
  id: number;
  name: string;
  kind: HoodKind;
}

const OUTER_RANK: Record<HoodKind, number> = { core: 0, inner: 1, waterfront: 2, industrial: 3, suburb: 4, rural: 5 };
const SEED_ORDER: Record<HoodKind, number> = { core: 0, waterfront: 1, inner: 2, industrial: 3, suburb: 4, rural: 5 };

/**
 * Lay out a town. Deterministic for (input, seed). Retries internally (with
 * more room, still deterministically) if the first grid is too tight.
 */
export function layoutTown(input: TownInput, seed: number, version: number = LAYOUT_VERSION): TownMap {
  // Older versions stay callable here once there are any (see LAYOUT_VERSION).
  if (version !== 1) throw new Error(`unknown town layout version ${version} (this build has 1..${LAYOUT_VERSION})`);
  let last = "";
  for (let attempt = 0; attempt < 8; attempt++) {
    try {
      return build(input, seed >>> 0, attempt);
    } catch (e) {
      if (!(e instanceof LayoutOverflow)) throw e;
      last = e.message;
    }
  }
  throw new Error(`town layout failed for ${input.municipality}: ${last}`);
}

/**
 * A stable layout seed for a town: the world seed and the municipality's name
 * (or id). It does not depend on LAYOUT_VERSION: the version picks the
 * algorithm, the seed stays the town's.
 */
export function townSeed(worldSeed: string, municipality: string | number): number {
  let h = 0x811c9dc5;
  for (const ch of `${worldSeed}\u0000${municipality}`) {
    h ^= ch.codePointAt(0) ?? 0;
    h = Math.imul(h, 0x01000193);
  }
  return mix(h >>> 0);
}

function build(input: TownInput, seed: number, attempt: number): TownMap {
  const rng = (label: string, ...i: number[]): Rng => stream(seed, label, attempt, ...i);

  // ---- normalise the input (sorted by id, so input order never matters)
  const hoodsIn = [...input.neighborhoods].sort((a, b) => a.id - b.id);
  if (hoodsIn.length === 0) hoodsIn.push({ id: -1, name: input.municipality, kind: "core" });
  const hoods: Hood[] = hoodsIn.map((h, index) => ({ index, id: h.id, name: h.name, kind: hoodKind(h.kind) }));
  const hoodIndex = new Map(hoods.map((h) => [h.id, h.index]));
  const placesIn = [...input.places].sort((a, b) => a.id - b.id);
  const townHall = placesIn.find((p) => p.kind === "town_hall");
  const centre =
    (townHall !== undefined ? hoodIndex.get(townHall.neighborhood) : undefined) ?? hoods.find((h) => h.kind === "core")?.index ?? 0;
  const places = placesIn.map((p) => ({ ...p, hood: hoodIndex.get(p.neighborhood) ?? centre }));
  const homeHood = input.home !== null ? (hoodIndex.get(input.home) ?? null) : null;

  // ---- water: a coast or river bank if the town has a waterfront or a port
  const hasWater = hoods.some((h) => h.kind === "waterfront") || places.some((p) => p.kind === "port");
  const sides: WaterSide[] = ["south", "east", "west", "north"];
  const water: WaterSide | null = hasWater ? rng("water").pick(sides) : null;

  // ---- which civic landmarks stand on the square
  const centreCivic = CIVIC.map((k) => places.find((p) => p.kind === k && p.hood === centre)).filter((p) => p !== undefined);
  const onSquare: typeof places = [];
  let squareW = 0;
  for (const p of centreCivic) {
    const [w] = placeSize(p.kind);
    if (onSquare.length > 0 && squareW + 1 + w > 30) continue;
    squareW += (onSquare.length ? 1 : 0) + w;
    onSquare.push(p);
  }
  const civicH = Math.max(4, ...onSquare.map((p) => placeSize(p.kind)[1]));

  // ---- blocks per neighbourhood
  const growth = 1 + 0.25 * attempt;
  const placeCount = hoods.map(() => 0);
  // Frontage the places need, off the square (a block row offers about ten tiles).
  const frontage = hoods.map(() => 0);
  for (const p of places) {
    placeCount[p.hood] = (placeCount[p.hood] ?? 0) + 1;
    if (!onSquare.includes(p)) frontage[p.hood] = (frontage[p.hood] ?? 0) + placeSize(p.kind)[0] + 1;
  }
  const quota = hoods.map((h) => {
    const n = placeCount[h.index] ?? 0;
    const base = { core: 2, inner: 1, waterfront: 2, industrial: 1, suburb: 1, rural: 1 }[h.kind];
    const per = { core: 0.3, inner: 0.4, waterfront: 0.5, industrial: 0.6, suburb: 0.4, rural: 0.6 }[h.kind];
    const need = Math.ceil((frontage[h.index] ?? 0) / 10) + (homeHood === h.index ? 1 : 0);
    return Math.ceil(Math.max(base + Math.ceil(n * per), need) * growth) + (h.index === centre ? 1 : 0);
  });
  const total = quota.reduce((a, b) => a + b, 0);
  const cells = Math.ceil(total * (1.1 + 0.1 * attempt)) + 1;
  const C = Math.max(3, Math.ceil(Math.sqrt(cells * 1.2)));
  const R = Math.max(3, Math.ceil(cells / C));
  const ci = Math.floor((C - 1) / 2);
  const cj = Math.floor((R - 1) / 2);

  const owner = assignBlocks(C, R, hoods, quota, centre, ci, cj, water, places, rng);
  if (!owner) throw new LayoutOverflow("blocks could not be shared out");
  const ownerAt = (i: number, j: number) => (i >= 0 && j >= 0 && i < C && j < R ? (owner[j * C + i] as number) : -1);

  // ---- block sizes (interior tiles)
  const rc = rng("sizes");
  const cw = Array.from({ length: C }, () => rc.int(10, 14));
  const rh = Array.from({ length: R }, () => rc.int(8, 10));
  cw[ci] = Math.max(cw[ci] as number, Math.min(32, squareW + 4), 16);
  rh[cj] = Math.max(rh[cj] as number, civicH + 6);

  const offX = MARGIN + (water === "west" ? QUAY + WATER : 0);
  const offY = MARGIN + (water === "north" ? QUAY + WATER : 0);
  const xS: number[] = [offX];
  const xB: number[] = [];
  for (let i = 0; i < C; i++) {
    xB.push((xS[i] as number) + STREET);
    xS.push((xB[i] as number) + (cw[i] as number));
  }
  const yS: number[] = [offY];
  const yB: number[] = [];
  for (let j = 0; j < R; j++) {
    yB.push((yS[j] as number) + STREET);
    yS.push((yB[j] as number) + (rh[j] as number));
  }
  const W = (xS[C] as number) + STREET + MARGIN + (water === "east" ? QUAY + WATER : 0);
  const H = (yS[R] as number) + STREET + MARGIN + (water === "south" ? QUAY + WATER : 0);

  const terrain = new Uint8Array(W * H).fill(Terrain.Grass);
  const solid = new Uint8Array(W * H);
  const at = (x: number, y: number) => (x >= 0 && y >= 0 && x < W && y < H ? (terrain[y * W + x] as number) : -1);
  const set = (x: number, y: number, t: number) => {
    if (x >= 0 && y >= 0 && x < W && y < H) terrain[y * W + x] = t;
  };
  const fillRect = (x0: number, y0: number, x1: number, y1: number, t: number) => {
    for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) set(x, y, t);
  };

  // ---- merges: big lots (industrial yards, rural fields) and green lanes (suburbs)
  const vSeg: SegStyle[][] = Array.from({ length: C + 1 }, () => Array.from({ length: R }, (): SegStyle => "asphalt"));
  const hSeg: SegStyle[][] = Array.from({ length: R + 1 }, () => Array.from({ length: C }, (): SegStyle => "asphalt"));
  const lotOf = new Int32Array(C * R).fill(-1);
  const lots: Lot[] = [];
  const rm = rng("merge");
  for (let j = 0; j < R; j++) {
    let i = 0;
    while (i < C) {
      const h = ownerAt(i, j);
      const kind = (hoods[h] as Hood).kind;
      let i1 = i;
      const maxSpan = kind === "rural" ? 3 : kind === "industrial" ? 2 : 1;
      const p = kind === "rural" ? 0.6 : kind === "industrial" ? 0.55 : 0;
      while (i1 + 1 < C && i1 - i + 1 < maxSpan && ownerAt(i1 + 1, j) === h && !(i1 + 1 === ci && j === cj) && !(i === ci && j === cj) && rm.chance(p)) i1++;
      for (let k = i + 1; k <= i1; k++) (vSeg[k] as SegStyle[])[j] = "lot";
      const lot: Lot = {
        i0: i,
        i1,
        j,
        x0: xB[i] as number,
        x1: (xB[i1] as number) + (cw[i1] as number) - 1,
        y0: yB[j] as number,
        y1: (yB[j] as number) + (rh[j] as number) - 1,
        hood: h,
        rows: [],
        buildings: [],
        named: 0,
        square: i === ci && j === cj,
        park: false,
        waterAdj:
          (water === "south" && j === R - 1) || (water === "north" && j === 0) || (water === "west" && i === 0) || (water === "east" && i1 === C - 1),
      };
      for (let k = i; k <= i1; k++) lotOf[j * C + k] = lots.length;
      lots.push(lot);
      // Suburbs: some streets between two of their blocks become a green footpath.
      if (i1 + 1 < C && kind === "suburb" && ownerAt(i1 + 1, j) === h && !(i1 + 1 === ci && j === cj) && rm.chance(0.4)) {
        (vSeg[i1 + 1] as SegStyle[])[j] = "green";
      }
      i = i1 + 1;
    }
  }
  // Country lanes where only rural blocks touch a street.
  const rural = (i: number, j: number) => {
    const h = ownerAt(i, j);
    return h < 0 || (hoods[h] as Hood).kind === "rural";
  };
  for (let k = 0; k <= C; k++) for (let j = 0; j < R; j++) if ((vSeg[k] as SegStyle[])[j] === "asphalt" && rural(k - 1, j) && rural(k, j)) (vSeg[k] as SegStyle[])[j] = "dirt";
  for (let l = 0; l <= R; l++) for (let i = 0; i < C; i++) if ((hSeg[l] as SegStyle[])[i] === "asphalt" && rural(i, l - 1) && rural(i, l)) (hSeg[l] as SegStyle[])[i] = "dirt";

  // ---- streets
  const open = (s: SegStyle | undefined) => s === "asphalt" || s === "dirt";
  const rl = rng("lanes");
  for (let k = 0; k <= C; k++) {
    const x = xS[k] as number;
    for (let j = 0; j < R; j++) {
      const s = (vSeg[k] as SegStyle[])[j];
      const y0 = yB[j] as number;
      const y1 = y0 + (rh[j] as number) - 1;
      if (s === "asphalt" || s === "dirt") {
        const side = s === "asphalt" ? Terrain.Pavement : Terrain.Grass;
        const road = s === "asphalt" ? Terrain.Road : Terrain.Dirt;
        fillRect(x, y0, x, y1, side);
        fillRect(x + 1, y0, x + 2, y1, road);
        fillRect(x + 3, y0, x + 3, y1, side);
      } else if (s === "green") {
        fillRect(x, y0, x + 3, y1, Terrain.Park);
        let c = rl.int(1, 2);
        for (let y = y0; y <= y1; y++) {
          const n = y === y0 || y === y1 ? c : rl.chance(0.3) ? 3 - c : c;
          set(x + c, y, Terrain.Path);
          set(x + n, y, Terrain.Path);
          c = n;
        }
      }
    }
  }
  for (let l = 0; l <= R; l++) {
    const y = yS[l] as number;
    for (let i = 0; i < C; i++) {
      const s = (hSeg[l] as SegStyle[])[i] as SegStyle;
      const x0 = xB[i] as number;
      const x1 = x0 + (cw[i] as number) - 1;
      const side = s === "asphalt" ? Terrain.Pavement : Terrain.Grass;
      const road = s === "asphalt" ? Terrain.Road : Terrain.Dirt;
      fillRect(x0, y, x1, y, side);
      fillRect(x0, y + 1, x1, y + 2, road);
      fillRect(x0, y + 3, x1, y + 3, side);
      // A mid-block zebra on long asphalt streets.
      if (s === "asphalt" && (cw[i] as number) >= 12) fillRect(x0 + Math.floor((cw[i] as number) / 2), y + 1, x0 + Math.floor((cw[i] as number) / 2), y + 2, Terrain.Crossing);
    }
  }
  for (let k = 0; k <= C; k++) {
    for (let l = 0; l <= R; l++) {
      const up = l > 0 ? (vSeg[k] as SegStyle[])[l - 1] : undefined;
      const down = l < R ? (vSeg[k] as SegStyle[])[l] : undefined;
      const left = k > 0 ? (hSeg[l] as SegStyle[])[k - 1] : undefined;
      const right = k < C ? (hSeg[l] as SegStyle[])[k] : undefined;
      const arms = [up, down, left, right];
      const asphalt = arms.includes("asphalt");
      const side = asphalt ? Terrain.Pavement : Terrain.Grass;
      const road = asphalt ? Terrain.Road : Terrain.Dirt;
      const x0 = xS[k] as number;
      const y0 = yS[l] as number;
      for (let dy = 0; dy < 4; dy++) {
        for (let dx = 0; dx < 4; dx++) {
          const rcol = dx === 1 || dx === 2;
          const rrow = dy === 1 || dy === 2;
          let t: number = side;
          if (rcol && rrow) t = road;
          else if (rcol) t = open(dy === 0 ? up : down) ? road : side;
          else if (rrow) t = open(dx === 0 ? left : right) ? road : side;
          // Zebras on every asphalt arm, where the pavements meet the road.
          if (asphalt && t === Terrain.Road && rcol !== rrow) t = Terrain.Crossing;
          set(x0 + dx, y0 + dy, t);
        }
      }
    }
  }

  // ---- lot ground and rows
  for (const lot of lots) {
    const kind = (hoods[lot.hood] as Hood).kind;
    const base = lot.square
      ? Terrain.Plaza
      : kind === "rural"
        ? Terrain.Field
        : kind === "industrial" || kind === "waterfront"
          ? Terrain.Yard
          : kind === "core"
            ? rng("yard", lot.i0, lot.j).chance(0.6)
              ? Terrain.Yard
              : Terrain.Grass
            : Terrain.Grass;
    fillRect(lot.x0, lot.y0, lot.x1, lot.y1, base);
    if (lot.square) {
      lot.rows.push({ x0: lot.x0 + 1, x1: lot.x1 - 1, front: lot.y0 + civicH, top: lot.y0, inset: 0, occ: [] });
    } else {
      lot.rows.push({ x0: lot.x0, x1: lot.x1, front: lot.y1 + 1, top: lot.y0, inset: kind === "suburb" ? 1 : 0, occ: [] });
    }
  }

  // ---- buildings
  const buildings: TownBuilding[] = [];
  const district = new Int16Array(W * H).fill(-1);
  const sqLot = lots.find((l) => l.square) as Lot;
  const sqCentre = { x: (sqLot.x0 + sqLot.x1 + 1) / 2, y: sqLot.y0 + civicH + (sqLot.y1 - sqLot.y0 + 1 - civicH) / 2 };
  const lotCentre = (l: Lot) => ({ x: (l.x0 + l.x1 + 1) / 2, y: (l.y0 + l.y1 + 1) / 2 });
  const distSq = (l: Lot) => {
    const c = lotCentre(l);
    return len(c.x - sqCentre.x, c.y - sqCentre.y);
  };

  const gaps = (row: Row): [number, number][] => {
    const out: [number, number][] = [];
    let x = row.x0;
    for (const [a, b] of [...row.occ].sort((p, q) => p[0] - q[0])) {
      if (a > x) out.push([x, a - 1]);
      x = Math.max(x, b + 1);
    }
    if (x <= row.x1) out.push([x, row.x1]);
    return out;
  };

  const addBuilding = (
    lot: Lot,
    row: Row,
    x: number,
    w: number,
    h: number,
    spec: { place: number | null; kind: string; name: string; home: boolean; landmark: boolean; doorAt?: number },
  ): TownBuilding => {
    const y = row.front - row.inset - h;
    const r = rng("door", x, y);
    const doorX = spec.doorAt ?? (w >= 3 ? x + Math.floor(w / 2) - (w % 2 === 0 && r.chance(0.5) ? 1 : 0) : x);
    const doorY = y + h - 1;
    const b: TownBuilding = {
      index: buildings.length,
      x,
      y,
      w,
      h,
      doorX,
      doorY,
      front: (row.front - row.inset) * W + doorX,
      place: spec.place,
      kind: spec.kind,
      name: spec.name,
      district: lot.hood,
      home: spec.home,
      landmark: spec.landmark,
      style: mix(seed, x, y, w * 31 + h),
    };
    buildings.push(b);
    lot.buildings.push(b.index);
    row.occ.push([x, x + w - 1]);
    return b;
  };

  const fits = (row: Row, w: number, h: number) => h + row.inset <= row.front - row.top && gaps(row).some(([a, b]) => b - a + 1 >= w);

  // The square: landmarks side by side along its top, the town hall in the middle.
  {
    const row = sqLot.rows[0] as Row;
    const order = [...onSquare];
    // Town hall centred, the chamber to one side, others outward.
    const arranged: typeof places = [];
    for (const p of order) {
      if (arranged.length % 2 === 0) arranged.push(p);
      else arranged.unshift(p);
    }
    const widths = arranged.map((p) => placeSize(p.kind)[0]);
    const total = widths.reduce((a, b) => a + b, 0) + Math.max(0, arranged.length - 1);
    let x = row.x0 + Math.floor((row.x1 - row.x0 + 1 - total) / 2);
    for (const p of arranged) {
      const [w, h] = placeSize(p.kind);
      // Shorter landmarks stand with their fronts on the square too.
      const r2 = { ...row, top: row.front - h };
      addBuilding(sqLot, r2 as Row, x, w, h, { place: p.id, kind: p.kind, name: p.name, home: false, landmark: true });
      row.occ.push([x, x + w - 1]);
      x += w + 1;
    }
    sqLot.named += arranged.length;
  }

  const placed = new Set(onSquare.map((p) => p.id));
  const priority = (kind: string) => (CIVIC.includes(kind) ? 0 : CENTRAL.has(kind) ? 1 : kind === "park" ? 3 : 2);
  const queue = places
    .filter((p) => !placed.has(p.id))
    .sort((a, b) => priority(a.kind) - priority(b.kind) || placeSize(b.kind)[0] * placeSize(b.kind)[1] - placeSize(a.kind)[0] * placeSize(a.kind)[1] || a.id - b.id);
  const homeSpec = homeHood !== null ? { hood: homeHood } : null;

  const placeOne = (hood: number, w0: number, h0: number, kind: string, score: (l: Lot) => number, parkLot: boolean) => {
    const cands = lots.filter((l) => l.hood === hood && !l.square && !l.park && (!parkLot || l.buildings.length === 0));
    for (let shrink = 0; shrink <= 3; shrink++) {
      const w = Math.max(3, w0 - shrink);
      const h = Math.max(2, h0 - (shrink > 1 ? shrink - 1 : 0));
      let best: { lot: Lot; row: Row; s: number } | null = null;
      for (const lot of cands) {
        for (const row of lot.rows) {
          if (!fits(row, w, h)) continue;
          const s = score(lot);
          if (!best || s > best.s) best = { lot, row, s };
        }
      }
      if (best) {
        const { lot, row } = best;
        const r = rng("x", lot.i0, lot.j, buildings.length);
        const g = gaps(row).filter(([a, b]) => b - a + 1 >= w);
        const [a, b] = r.pick(g);
        const slack = b - a + 1 - w;
        // Landmarks and parks sit centred in their gap; others hug an end so the rest stays usable.
        const x = parkLot || CIVIC.includes(kind) || slack < 3 ? a + Math.floor(slack / 2) : r.chance(0.5) ? a : b - w + 1;
        return { lot, row, x, w, h };
      }
    }
    return null;
  };

  for (const p of queue) {
    const [w, h] = placeSize(p.kind);
    const r = rng("score", p.id);
    const jitter = lots.map(() => r.next());
    const score = (l: Lot) => {
      const d = distSq(l);
      const j = jitter[lots.indexOf(l)] as number;
      if (p.kind === "port") return (l.waterAdj ? 100 : 0) - d / 10 + j;
      if (CENTRAL.has(p.kind)) return -d + j * 2 - l.named * 3;
      if (OUTSKIRTS.has(p.kind)) return d / 4 + j * 4 - l.named * 6;
      return j * 10 - l.named * 6 - d / 20;
    };
    const spot = placeOne(p.hood, w, h, p.kind, score, p.kind === "park");
    if (!spot) throw new LayoutOverflow(`no room for ${p.kind} in ${(hoods[p.hood] as Hood).name}`);
    addBuilding(spot.lot, spot.row, spot.x, spot.w, spot.h, { place: p.id, kind: p.kind, name: p.name, home: false, landmark: CIVIC.includes(p.kind) || w * h >= 35 });
    spot.lot.named++;
    if (p.kind === "park") spot.lot.park = true;
  }
  let homeBuilding: number | null = null;
  if (homeSpec) {
    const r = rng("home");
    const jitter = lots.map(() => r.next());
    const spot = placeOne(homeSpec.hood, 4, 3, "home", (l) => (jitter[lots.indexOf(l)] as number) - l.named, false);
    if (!spot) throw new LayoutOverflow("no room for the home");
    homeBuilding = addBuilding(spot.lot, spot.row, spot.x, spot.w, spot.h, { place: null, kind: "home", name: "", home: true, landmark: false }).index;
  }

  // ---- pocket parks, lanes and filler buildings
  for (const lot of lots) {
    if (lot.square) continue;
    const kind = (hoods[lot.hood] as Hood).kind;
    const r = rng("fill", lot.i0, lot.j);
    if (!lot.park && lot.buildings.length === 0 && (kind === "inner" || kind === "suburb" || kind === "core") && r.chance(kind === "core" ? 0.12 : 0.22)) {
      lot.park = true;
    }
    if (lot.park) {
      fillRect(lot.x0, lot.y0, lot.x1, lot.y1, Terrain.Park);
      // A loop of paths through the park, open to the streets on every side.
      const mx = Math.floor((lot.x0 + lot.x1) / 2);
      const my = Math.floor((lot.y0 + lot.y1) / 2);
      fillRect(mx, lot.y0, mx, lot.y1, Terrain.Path);
      fillRect(lot.x0, my, lot.x1, my, Terrain.Path);
      for (const bi of lot.buildings) {
        const b = buildings[bi] as TownBuilding;
        fillRect(b.x - 1, b.y + b.h - 1, b.x + b.w, lot.y1, Terrain.Path);
      }
      continue;
    }
    const row0 = lot.rows[0] as Row;
    const depth = lot.y1 - lot.y0 + 1;
    const tallest = Math.max(0, ...lot.buildings.map((bi) => (buildings[bi] as TownBuilding).h));
    const U = Math.floor((depth - 1) / 2);
    const L = depth - 1 - U;
    const split = { core: 0.85, inner: 0.7, suburb: 0.7, waterfront: 0.7, industrial: 0.5, rural: 0 }[kind];
    // The lower row keeps its roofs off the lane: the lane holds the upper row's doorsteps.
    if (depth >= 9 && tallest + row0.inset <= L - ROOF_ROWS && r.chance(split)) {
      const laneY = lot.y0 + U;
      fillRect(lot.x0, laneY, lot.x1, laneY, kind === "core" ? Terrain.Pavement : kind === "industrial" ? Terrain.Yard : Terrain.Path);
      row0.top = laneY + 1 + ROOF_ROWS;
      lot.rows.push({ x0: lot.x0, x1: lot.x1, front: laneY, top: lot.y0, inset: row0.inset, occ: [] });
    }
    for (const row of lot.rows) {
      const room = row.front - row.top - row.inset;
      for (const [a, b] of gaps(row)) {
        let x = a;
        while (x <= b) {
          const left = b - x + 1;
          const f = filler(kind, r, room);
          if (!f) break;
          if (left < f.min) break;
          let w = Math.min(f.w, left);
          // Never leave a sliver too narrow for a building.
          if (left - w > 0 && left - w < f.min + f.gap) w = left - (r.chance(0.5) ? 0 : Math.min(left - f.min, f.gap));
          if (w < f.min) break;
          if (r.chance(f.skip)) {
            x += w + f.gap;
            continue;
          }
          addBuilding(lot, row, x, w, Math.min(f.h, room), { place: null, kind: f.kind, name: "", home: false, landmark: false });
          x += w + f.gap;
        }
      }
    }
  }

  // ---- paint buildings, doors and what is in front of them
  for (const b of buildings) {
    fillRect(b.x, b.y, b.x + b.w - 1, b.y + b.h - 1, Terrain.Building);
    set(b.doorX, b.doorY, Terrain.Door);
  }
  for (const lot of lots) {
    for (const row of lot.rows) {
      if (row.inset) {
        const gy = row.front - 1;
        for (let x = row.x0; x <= row.x1; x++) if (at(x, gy) !== Terrain.Building && at(x, gy) !== Terrain.Door) set(x, gy, Terrain.Garden);
      }
    }
  }
  for (const b of buildings) {
    const fx = b.front % W;
    const fy = Math.floor(b.front / W);
    if (surfaceOfTerrain(at(fx, fy)) !== "pavement") set(fx, fy, Terrain.Path);
  }

  // ---- water, quay, piers
  if (water) {
    const quay = (x0: number, y0: number, x1: number, y1: number) => fillRect(x0, y0, x1, y1, Terrain.Quay);
    const sea = (x0: number, y0: number, x1: number, y1: number) => fillRect(x0, y0, x1, y1, Terrain.Water);
    const edgeS = (yS[R] as number) + STREET;
    const edgeE = (xS[C] as number) + STREET;
    if (water === "south") {
      quay(0, edgeS, W - 1, edgeS + QUAY - 1);
      sea(0, edgeS + QUAY, W - 1, H - 1);
    } else if (water === "north") {
      sea(0, 0, W - 1, offY - QUAY - 1);
      quay(0, offY - QUAY, W - 1, offY - 1);
    } else if (water === "east") {
      quay(edgeE, 0, edgeE + QUAY - 1, H - 1);
      sea(edgeE + QUAY, 0, W - 1, H - 1);
    } else {
      sea(0, 0, offX - QUAY - 1, H - 1);
      quay(offX - QUAY, 0, offX - 1, H - 1);
    }
    // A pier off the quay by each port.
    for (const b of buildings) {
      if (b.kind !== "port") continue;
      const lot = lots.find((l) => l.buildings.includes(b.index));
      if (!lot?.waterAdj) continue;
      if (water === "south") fillRect(b.doorX - 1, edgeS + QUAY, b.doorX + 1, edgeS + QUAY + 3, Terrain.Pier);
      else if (water === "north") fillRect(b.doorX - 1, offY - QUAY - 4, b.doorX + 1, offY - QUAY - 1, Terrain.Pier);
      else if (water === "east") fillRect(edgeE + QUAY, b.doorY, edgeE + QUAY + 3, b.doorY + 2, Terrain.Pier);
      else fillRect(offX - QUAY - 4, b.doorY, offX - QUAY - 1, b.doorY + 2, Terrain.Pier);
    }
  }

  // ---- districts: blocks, then the streets between them (nearest block wins)
  const town = { x0: xS[0] as number, y0: yS[0] as number, x1: (xS[C] as number) + STREET - 1, y1: (yS[R] as number) + STREET - 1 };
  {
    const queue: number[] = [];
    for (const lot of lots) {
      for (let y = lot.y0; y <= lot.y1; y++) {
        for (let x = lot.x0; x <= lot.x1; x++) {
          district[y * W + x] = lot.hood;
        }
      }
    }
    for (let y = town.y0; y <= town.y1; y++) for (let x = town.x0; x <= town.x1; x++) if ((district[y * W + x] as number) >= 0) queue.push(y * W + x);
    for (let qi = 0; qi < queue.length; qi++) {
      const c = queue[qi] as number;
      const x = c % W;
      const y = Math.floor(c / W);
      const d = district[c] as number;
      for (const [dx, dy] of [
        [1, 0],
        [-1, 0],
        [0, 1],
        [0, -1],
      ] as const) {
        const nx = x + dx;
        const ny = y + dy;
        if (nx < town.x0 || ny < town.y0 || nx > town.x1 || ny > town.y1) continue;
        const n = ny * W + nx;
        if ((district[n] as number) >= 0) continue;
        district[n] = d;
        queue.push(n);
      }
    }
  }

  // ---- props
  const props: TownProp[] = [];
  const doorFront = new Uint8Array(W * H);
  for (const b of buildings) doorFront[b.front] = 1;
  const nearDoor = (x: number, y: number) => {
    for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) if (doorFront[(y + dy) * W + x + dx] || at(x + dx, y + dy) === Terrain.Door) return true;
    return false;
  };
  const blocked = new Uint8Array(W * H);
  const addProp = (kind: PropKind, x: number, y: number, blocks: boolean) => {
    props.push({ kind, x, y, blocks, variant: mix(seed, x, y) & 0xff });
    if (blocks) blocked[y * W + x] = 1;
  };
  // The square: a fountain, benches, flags.
  if (sqLot) {
    const sq = { x0: sqLot.x0, y0: sqLot.y0 + civicH, x1: sqLot.x1, y1: sqLot.y1 };
    const fx = Math.floor((sq.x0 + sq.x1) / 2);
    const fy = sq.y0 + Math.floor((sq.y1 - sq.y0) / 2) + 1;
    for (const [dx, dy] of [
      [0, 0],
      [1, 0],
      [0, 1],
      [1, 1],
    ] as const)
      blocked[(fy + dy) * W + fx + dx] = 1;
    props.push({ kind: "fountain", x: fx, y: fy, blocks: true, variant: 0 });
    for (const bx of [sq.x0 + 1, sq.x1 - 1]) {
      addProp("planter", bx, sq.y0 + 1, false);
      addProp("bench", bx, fy, false);
      addProp("planter", bx, sq.y1 - 1, false);
    }
    for (const b of buildings) if (b.landmark && sqLot.buildings.includes(b.index)) addProp("flag", b.x + b.w - 1, b.y + b.h, false);
  }
  // Ports: cranes on the quay, boats on the water.
  const rp = rng("props");
  if (water) {
    for (let y = 0; y < H; y++) {
      for (let x = 0; x < W; x++) {
        if (at(x, y) !== Terrain.Water) continue;
        const nearQuay = [at(x, y - 2), at(x, y + 2), at(x - 2, y), at(x + 2, y)].some((t) => t === Terrain.Quay || t === Terrain.Pier);
        if (nearQuay && rp.chance(0.03)) addProp("boat", x, y, false);
      }
    }
  }
  // Street lamps along asphalt pavements (decor: they never block a pavement).
  for (let y = town.y0; y <= town.y1; y++) {
    for (let x = town.x0; x <= town.x1; x++) {
      if (at(x, y) !== Terrain.Pavement || doorFront[y * W + x]) continue;
      const rd = [at(x, y - 1), at(x, y + 1), at(x - 1, y), at(x + 1, y)].some((t) => t === Terrain.Road);
      if (rd && (x * 7 + y * 13) % 11 === 0) addProp("lamp", x, y, false);
    }
  }
  // Trees: thick at the edges and in parks, a few in back gardens and on verges.
  for (let y = 1; y < H - 1; y++) {
    for (let x = 1; x < W - 1; x++) {
      const t = at(x, y);
      const inTown = x >= town.x0 && x <= town.x1 && y >= town.y0 && y <= town.y1;
      let p = 0;
      if (t === Terrain.Grass) p = inTown ? 0.1 : 0.3;
      else if (t === Terrain.Park) p = 0.14;
      else if (t === Terrain.Field) p = 0.004;
      if (p === 0 || rp.next() >= p) continue;
      if (nearDoor(x, y)) continue;
      const below = at(x, y + 1);
      const above = at(x, y - 1);
      if (below === Terrain.Building || below === Terrain.Door || above === Terrain.Building || above === Terrain.Door) continue;
      // Keep front gardens and the cells beside paths open.
      if ([at(x - 1, y), at(x + 1, y), above, below].some((n) => n === Terrain.Path || n === Terrain.Garden)) continue;
      let crowded = false;
      for (let dy = -1; dy <= 1 && !crowded; dy++) for (let dx = -1; dx <= 1; dx++) if (blocked[(y + dy) * W + x + dx]) crowded = true;
      if (crowded) continue;
      addProp(inTown || rp.chance(0.6) ? "tree" : "pine", x, y, true);
    }
  }

  for (let i = 0; i < W * H; i++) {
    const t = terrain[i] as number;
    solid[i] = t === Terrain.Building || t === Terrain.Door || t === Terrain.Water || blocked[i] ? 1 : 0;
  }

  // ---- districts summary
  const districts: TownDistrict[] = hoods.map((h) => {
    let n = 0;
    let sx = 0;
    let sy = 0;
    let bx0 = W;
    let by0 = H;
    let bx1 = -1;
    let by1 = -1;
    for (const lot of lots) {
      if (lot.hood !== h.index) continue;
      const a = (lot.x1 - lot.x0 + 1) * (lot.y1 - lot.y0 + 1);
      n += a;
      sx += ((lot.x0 + lot.x1 + 1) / 2) * a;
      sy += ((lot.y0 + lot.y1 + 1) / 2) * a;
    }
    let cells = 0;
    for (let i = 0; i < W * H; i++) {
      if (district[i] !== h.index) continue;
      cells++;
      const x = i % W;
      const y = Math.floor(i / W);
      bx0 = Math.min(bx0, x);
      by0 = Math.min(by0, y);
      bx1 = Math.max(bx1, x);
      by1 = Math.max(by1, y);
    }
    const centre = { x: n ? sx / n : W / 2, y: n ? sy / n : H / 2 };
    // The streets of the neighbourhood: its pavement cell nearest the centre.
    let street = -1;
    let best = Infinity;
    for (let i = 0; i < W * H; i++) {
      if (district[i] !== h.index || solid[i]) continue;
      const t = terrain[i] as number;
      if (t !== Terrain.Pavement && t !== Terrain.Dirt && t !== Terrain.Path) continue;
      const d = len((i % W) + 0.5 - centre.x, Math.floor(i / W) + 0.5 - centre.y);
      if (d < best - 1e-9) {
        best = d;
        street = i;
      }
    }
    return { index: h.index, id: h.id, name: h.name, kind: h.kind, centre, street, cells, bounds: { x: bx0, y: by0, w: bx1 - bx0 + 1, h: by1 - by0 + 1 } };
  });

  const placeList = buildings
    .filter((b) => b.place !== null)
    .map((b) => ({ place: b.place as number, building: b.index }))
    .sort((a, b) => a.place - b.place);
  const square = { x: sqLot.x0, y: sqLot.y0 + civicH, w: sqLot.x1 - sqLot.x0 + 1, h: sqLot.y1 - sqLot.y0 + 1 - civicH };

  const map: TownMap = {
    version: 1,
    seed,
    municipality: input.municipality,
    width: W,
    height: H,
    terrain,
    solid,
    district,
    buildings,
    districts,
    props,
    places: placeList,
    square,
    water,
    homeBuilding,
    start: 0,
  };
  map.start = locationCell(map, input.location) ?? (districts[centre] as TownDistrict).street;
  return map;
}

/** Filler buildings by neighbourhood kind: what fills the blocks between the places. */
function filler(kind: HoodKind, r: Rng, room: number): { kind: FillerKind; w: number; h: number; min: number; gap: number; skip: number } | null {
  if (room < 2) return null;
  const hh = (lo: number, hi: number) => Math.max(2, Math.min(room, r.int(lo, hi)));
  switch (kind) {
    case "core":
      return r.chance(0.6)
        ? { kind: "shop", w: r.int(3, 5), h: hh(3, 4), min: 3, gap: r.chance(0.75) ? 0 : 1, skip: 0.05 }
        : { kind: "terrace", w: r.int(3, 4), h: hh(3, 4), min: 3, gap: 0, skip: 0.05 };
    case "inner":
      return { kind: r.chance(0.75) ? "terrace" : "shop", w: r.int(3, 4), h: hh(3, 3), min: 3, gap: r.chance(0.5) ? 0 : 1, skip: 0.08 };
    case "suburb":
      return { kind: "house", w: r.int(3, 4), h: hh(3, 3), min: 3, gap: r.int(1, 2), skip: 0.12 };
    case "waterfront":
      return r.chance(0.4)
        ? { kind: "boathouse", w: r.int(4, 6), h: hh(3, 4), min: 4, gap: 1, skip: 0.1 }
        : { kind: r.chance(0.5) ? "shop" : "terrace", w: r.int(3, 4), h: hh(3, 4), min: 3, gap: r.chance(0.6) ? 0 : 1, skip: 0.08 };
    case "industrial":
      return r.chance(0.7)
        ? { kind: "warehouse", w: r.int(5, 8), h: hh(4, 6), min: 5, gap: r.int(1, 2), skip: 0.08 }
        : { kind: "shed", w: r.int(3, 4), h: hh(3, 3), min: 3, gap: r.int(1, 2), skip: 0.1 };
    case "rural":
      return r.chance(0.55)
        ? { kind: "farmhouse", w: r.int(3, 4), h: hh(3, 3), min: 3, gap: r.int(4, 7), skip: 0.45 }
        : { kind: "barn", w: r.int(4, 5), h: hh(3, 4), min: 4, gap: r.int(4, 7), skip: 0.45 };
  }
}

/**
 * Grow neighbourhoods over a grid of blocks: the centre at the middle, each
 * other kind seeded where it belongs (waterfront on the water, rural at the
 * edge) and grown block by block, so every neighbourhood is contiguous.
 * Leftover blocks join a neighbour (outer kinds first). Returns null if a
 * neighbourhood was boxed in short of its quota.
 */
function assignBlocks(
  C: number,
  R: number,
  hoods: Hood[],
  quota: number[],
  centre: number,
  ci: number,
  cj: number,
  water: WaterSide | null,
  places: { kind: string; hood: number }[],
  rng: (label: string, ...i: number[]) => Rng,
): Int16Array | null {
  const owner = new Int16Array(C * R).fill(-1);
  const size = hoods.map(() => 0);
  const dist = (i: number, j: number) => len((i - ci) * 1.2, j - cj);
  const edge = (i: number, j: number) => Math.min(i, C - 1 - i, j, R - 1 - j);
  const adjWater = (i: number, j: number) =>
    (water === "south" && j === R - 1) || (water === "north" && j === 0) || (water === "west" && i === 0) || (water === "east" && i === C - 1);
  const hasPort = hoods.map((h) => places.some((p) => p.hood === h.index && p.kind === "port"));
  const pref = (h: Hood, i: number, j: number) => {
    const d = dist(i, j);
    switch (h.kind) {
      case "core":
        return -2 * d;
      case "inner":
        return -Math.abs(d - 1.5) * 1.5;
      case "waterfront":
        return (water ? (adjWater(i, j) ? 6 : -4) : 0) - Math.abs(d - 2) * 0.5;
      case "industrial":
        return -Math.abs(d - 2.5) - edge(i, j) * 0.5 + (hasPort[h.index] && adjWater(i, j) ? 2 : 0);
      case "suburb":
        return -Math.abs(d - 2.5);
      case "rural":
        return d * 0.8 - edge(i, j) * 1.5;
    }
  };
  const take = (h: number, i: number, j: number) => {
    owner[j * C + i] = h;
    size[h] = (size[h] ?? 0) + 1;
  };
  take(centre, ci, cj);
  const N4 = [
    [1, 0],
    [-1, 0],
    [0, 1],
    [0, -1],
  ] as const;
  const frontierBest = (h: number, r: Rng): number => {
    const hood = hoods[h] as Hood;
    let best = -1;
    let bestS = -Infinity;
    for (let j = 0; j < R; j++) {
      for (let i = 0; i < C; i++) {
        if ((owner[j * C + i] as number) >= 0) continue;
        let own = 0;
        for (const [dx, dy] of N4) {
          const x = i + dx;
          const y = j + dy;
          if (x >= 0 && y >= 0 && x < C && y < R && owner[y * C + x] === h) own++;
        }
        if (own === 0) continue;
        const s = pref(hood, i, j) + 1.5 * own + r.next() * 0.3;
        if (s > bestS) {
          bestS = s;
          best = j * C + i;
        }
      }
    }
    return best;
  };
  // The centre grows first, so the neighbourhoods seeded around it can never box it in.
  const rc = rng("centre");
  while ((size[centre] as number) < (quota[centre] as number)) {
    const b = frontierBest(centre, rc);
    if (b < 0) break;
    take(centre, b % C, Math.floor(b / C));
  }
  const seeds: [number, number][] = [];
  for (let j = 0; j < R; j++) for (let i = 0; i < C; i++) if (owner[j * C + i] === centre) seeds.push([i, j]);
  const rs = rng("seeds");
  const order = hoods.filter((h) => h.index !== centre).sort((a, b) => SEED_ORDER[a.kind] - SEED_ORDER[b.kind] || a.index - b.index);
  for (const h of order) {
    let best = -1;
    let bestS = -Infinity;
    for (let j = 0; j < R; j++) {
      for (let i = 0; i < C; i++) {
        if ((owner[j * C + i] as number) >= 0) continue;
        const sep = Math.min(3, ...seeds.map(([a, b]) => len(a - i, b - j)));
        const s = pref(h, i, j) + 1.2 * sep + rs.next() * 0.5;
        if (s > bestS) {
          bestS = s;
          best = j * C + i;
        }
      }
    }
    if (best < 0) return null;
    take(h.index, best % C, Math.floor(best / C));
    seeds.push([best % C, Math.floor(best / C)]);
  }
  const rg = rng("grow");
  const stuck = hoods.map(() => false);
  for (;;) {
    let h = -1;
    let need = 0;
    for (const hd of hoods) {
      const q = quota[hd.index] as number;
      const left = (q - (size[hd.index] as number)) / q;
      if (stuck[hd.index] || left <= 0) continue;
      if (left > need + 1e-9) {
        need = left;
        h = hd.index;
      }
    }
    if (h < 0) break;
    const best = frontierBest(h, rg);
    if (best < 0) {
      stuck[h] = true;
      continue;
    }
    take(h, best % C, Math.floor(best / C));
  }
  // A neighbourhood boxed in short of its quota is accepted: placement fails
  // (and the layout retries with more room) only if its places do not fit.
  // Leftover blocks join a neighbour, outer kinds first.
  const free = [];
  for (let j = 0; j < R; j++) for (let i = 0; i < C; i++) if ((owner[j * C + i] as number) < 0) free.push(j * C + i);
  free.sort((a, b) => dist(a % C, Math.floor(a / C)) - dist(b % C, Math.floor(b / C)) || a - b);
  while (free.length) {
    let progressed = false;
    for (let k = 0; k < free.length; k++) {
      const c = free[k] as number;
      const i = c % C;
      const j = Math.floor(c / C);
      let pick = -1;
      for (const [dx, dy] of N4) {
        const x = i + dx;
        const y = j + dy;
        if (x < 0 || y < 0 || x >= C || y >= R) continue;
        const o = owner[y * C + x] as number;
        if (o < 0) continue;
        if (pick < 0 || OUTER_RANK[(hoods[o] as Hood).kind] > OUTER_RANK[(hoods[pick] as Hood).kind] || (OUTER_RANK[(hoods[o] as Hood).kind] === OUTER_RANK[(hoods[pick] as Hood).kind] && o < pick)) pick = o;
      }
      if (pick < 0) continue;
      owner[c] = pick;
      free.splice(k, 1);
      k--;
      progressed = true;
    }
    if (!progressed) return null;
  }
  return owner;
}

// ---------------------------------------------------------------------------
// Helpers over a finished map

/** The walkable grid for ./pathfinding (pedestrian costs per terrain; buildings, water, trees blocked). */
export function townGrid(map: TownMap, costs?: Parameters<typeof buildGrid>[3]): Grid {
  return buildGrid(map.width, map.height, (x, y) => {
    const i = y * map.width + x;
    return map.solid[i] ? "blocked" : surfaceOfTerrain(map.terrain[i] as number);
  }, costs);
}

/** Tap targets for ./pathfinding's `resolveTap`, parallel to `map.buildings`. */
export function townFootprints(map: TownMap): Footprint[] {
  // Layout v2: reserved land is open ground, not a building to tap (an empty footprint keeps the indices parallel).
  return map.buildings.map((b) => (b.state === "reserved" ? { x: b.x, y: b.y, w: 0, h: 0, door: b.front, roofRows: 0 } : { x: b.x, y: b.y, w: b.w, h: b.h, door: b.front, roofRows: ROOF_ROWS }));
}

export function buildingOfPlace(map: TownMap, place: number): TownBuilding | null {
  const e = map.places.find((p) => p.place === place);
  return e ? (map.buildings[e.building] ?? null) : null;
}

/** The cell a location stands for: a building's door, the home's door, or a neighbourhood's streets. */
export function locationCell(map: TownMap, loc: TownLocation): number | null {
  if (loc.type === "place") return buildingOfPlace(map, loc.place)?.front ?? null;
  if (loc.type === "home") return map.homeBuilding !== null ? (map.buildings[map.homeBuilding]?.front ?? null) : null;
  const d = map.districts.find((x) => x.id === loc.neighborhood);
  return d && d.street >= 0 ? d.street : null;
}

/** A short stable hash of the whole map (layout-stability tests, debugging). */
export function mapHash(map: TownMap): string {
  let h = 0x811c9dc5;
  const feed = (n: number) => {
    h ^= n & 0xff;
    h = Math.imul(h, 0x01000193);
    h ^= (n >>> 8) & 0xff;
    h = Math.imul(h, 0x01000193);
  };
  feed(map.width);
  feed(map.height);
  for (let i = 0; i < map.terrain.length; i++) feed((map.terrain[i] as number) | ((map.solid[i] as number) << 5) | (((map.district[i] as number) + 1) << 6));
  for (const b of map.buildings) for (const n of [b.x, b.y, b.w, b.h, b.doorX, b.place ?? 0xffff, b.district]) feed(n);
  for (const p of map.props) feed(p.x * 31 + p.y);
  return (h >>> 0).toString(16).padStart(8, "0");
}
