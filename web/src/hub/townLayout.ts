// The town map model for the walkable hub and the town map overview
// (DEC-009, DEC-079, DEC-080, DEC-086): terrain codes, buildings, districts,
// props, connectors and street ends on a tile grid, and helpers over a
// finished map. Layout v2 (./layoutV2, DEC-090) makes these maps: `planTown`
// from the town's geography, then `dressTown` for its current state.
//
// Coordinates are tiles (16 art px each); cell index = y * width + x, the
// same as ./pathfinding. Doors all face south (the visible side in the
// three-quarter view, DEC-049): a building's door tile is in its bottom row
// and the walkable cell below it is where routes to it end.
import { buildGrid, type Footprint, type Grid, type Point, type Surface } from "./pathfinding";
import { mix } from "./rng";
import type { HoodKind, TownLocation } from "./townInput";

/**
 * Version of the town layout algorithm (DEC-085, DEC-090). Before release a
 * development run may show a rearranged town after an update (DEC-085's
 * pre-release note) and the pinned plan hashes are refreshed as needed; at
 * release the shipped version is frozen and later changes ship as a new one.
 */
export const LAYOUT_VERSION = 2;

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
  // ---- Terrain, water and level vocabulary (DEC-090; TOWN_GENERATION.md §7.7).
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
  /** The plot's dress state (absent: built). */
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
  // DEC-090: a mine's winding tower and spoil heap; bank reeds; creek stepping stones.
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
 * How a street that does not join another street ends. Every such end
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

/** One street end (the last road cells of the street, tiles). */
export interface TownStreetEnd {
  kind: StreetEndKind;
  x: number;
  y: number;
  w: number;
  h: number;
  /** The direction the street runs into its end: "n", "e", "s", "w", or a diagonal "ne", "nw", "se", "sw". */
  facing: "n" | "e" | "s" | "w" | "ne" | "nw" | "se" | "sw";
}

/** What crosses water, rail or a creek. The cells it covers are `Bridge`, `Ford` or `Crossing`. */
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

/** Street directions per cell (`TownMap.streetAxis`). */
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

/** The dress state of a building plot (DEC-090 D2). */
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
  /** Ground level per cell (0–3, about a storey each; DEC-090, TOWN_GENERATION.md §7.7). Absent (hand-made test maps): flat. */
  elevation?: Uint8Array;
  /** Bridges, fords, viaducts and level crossings (absent: none). */
  connectors?: TownConnector[];
  /**
   * Street rework A + B (DEC-090 D5 amended): the direction of the street a road,
   * crossing or pavement cell belongs to: `StreetAxis` codes. Absent or 0 means
   * orthogonal (the art reads the shape from neighbours). Diagonal cells
   * form 45° staircases; the art draws them as smooth 45° roads and kerbs.
   */
  streetAxis?: Uint8Array;
  /** How every street that does not join another street ends (`TownStreetEnd`). */
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

/** Default footprints (w × h tiles) by place kind, where a place layout block gives none. */
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
  // Reserved land is open ground, not a building to tap (an empty footprint keeps the indices parallel).
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

