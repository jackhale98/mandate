// The hub's art behind one interface (DEC-049, DEC-080), shaped after the
// pixel-art tool plan (docs/design/research/PIXEL_ART_TOOL.md §3.5, §5.1):
// `ArtProvider` with ground, building, prop, character, grade and icon. The
// renderer asks it for frames and data and does no colour design of its own;
// the placeholder (./placeholderArt) draws everything in code, and the art
// pack from the tool replaces it by implementing the same calls:
//   1. the provider has the shape of `ArtProvider`;
//   2. character frames are 16×24, anchored at the feet; facings down, up and
//      side (the renderer mirrors side for facing right);
//   3. buildings come back as albedo frames plus windows and doors, each with
//      its own emissive frame and a use tag;
//   4. every shadow caster has a footprint polygon and a height in art px; the
//      renderer projects the shadows;
//   5. grading comes from `grade(minuteOfDay)`;
//   6. everything is drawn at art resolution, scaled by whole numbers.
import { mix } from "../rng";
import { Terrain, type TownBuilding, type TownMap } from "../townLayout";

/** A 2D context the renderer and the art draw with (a canvas's, or a test stub). */
export type Ctx = CanvasRenderingContext2D;

/** A canvas made by a `CanvasFactory` (DOM canvas in the browser, a stub in tests). */
export interface Layer {
  canvas: HTMLCanvasElement;
  ctx: Ctx;
}

/** Makes offscreen canvases (the renderer's layers, the placeholder's sheets). */
export type CanvasFactory = (width: number, height: number) => Layer;

export const domCanvasFactory: CanvasFactory = (width, height) => {
  const canvas = document.createElement("canvas");
  canvas.width = Math.max(1, Math.ceil(width));
  canvas.height = Math.max(1, Math.ceil(height));
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("no 2D canvas");
  ctx.imageSmoothingEnabled = false;
  return { canvas, ctx };
};

/**
 * A region of an image (an atlas page or a composed canvas) and its anchor,
 * in px from the region's top-left. Anchors:
 * - ground tiles: their top-left (0, 0);
 * - buildings: the footprint's top-left (`ay` = roof px above the footprint);
 * - props: the top-left of the prop's cell (`ay` = px it rises above it);
 * - characters: the feet (bottom centre);
 * - emissive frames: the top-left of the opening they light;
 * - pools and markers: their centre.
 * Whatever is above a building's or prop's anchor row is its overhang: the
 * renderer draws it again over walkers standing behind it.
 */
export interface FrameRef {
  image: CanvasImageSource;
  x: number;
  y: number;
  w: number;
  h: number;
  ax: number;
  ay: number;
}

export type Rgb = readonly [number, number, number];

/** Terrain kinds are the layout's codes (`Terrain` in ../townLayout). */
export type TerrainKind = number;

export type Facing = "down" | "up" | "side";
export type AnimKey = "idle" | "walk";

/**
 * What a character looks like. The engine's appearance genome (`LookView`,
 * PIXEL_ART_TOOL §3.5) replaces this placeholder: today it is a seed and
 * whether it is the player.
 */
export interface LookView {
  seed: number;
  player?: boolean;
}

/** Light schedules by use (homes at dusk, offices dark late, shops early). */
export type LightUse = "home" | "office" | "shop" | "civic" | "industrial";

/** A window or door: its rectangle in the albedo frame (px), what lights it, and its use. */
export interface Opening {
  x: number;
  y: number;
  w: number;
  h: number;
  emissive: FrameRef | null;
  use: LightUse;
}

/** A shadow caster: a footprint polygon (px from the frame's anchor) and a height (px). */
export interface ShadowCaster {
  footprint: readonly (readonly [number, number])[];
  height: number;
}

/**
 * What the art needs to know to draw a building: what the engine knows
 * (place kind, neighbourhood kind, conditions) and what the town layout
 * decided (footprint, door, storeys), plus a stable visual seed.
 */
export interface BuildingSpec {
  /** Place-registry key, or a filler kind (`house`, `shop`, `warehouse`, … `home`). */
  kind: string;
  /** Neighbourhood kind (`core`, `inner`, `suburb`, …). */
  hood: string;
  /** Footprint, tiles. */
  w: number;
  h: number;
  /** The door: tile column from the footprint's left, and the wall it is in. */
  door: { x: number; side: "south" };
  storeys: number;
  visualSeed: number;
  /** Simulation conditions (`boarded`, `construction`, `flooded`, …); none yet. */
  conditions: readonly string[];
  /** A named place (gets a sign), the player's home, a civic landmark. */
  named: boolean;
  home: boolean;
  landmark: boolean;
}

export interface BuiltBuilding {
  /** Footprint plus roof rows (anchor: the footprint's top-left). */
  albedo: FrameRef;
  windows: Opening[];
  doors: Opening[];
  shadow: ShadowCaster;
}

export interface PropSprite {
  albedo: FrameRef;
  /** Lit parts (a lamp head), drawn over the grade at the anchor. */
  emissive?: FrameRef;
  /** A light pool on the ground, centred at (x, y) px from the anchor. */
  pool?: { frame: FrameRef; x: number; y: number };
  shadow?: ShadowCaster;
}

/** One moment's grading: everything the renderer applies for the time of day. */
export interface Grade {
  /** Multiply tint (brightness folded in), 0..255. */
  mul: Rgb;
  /** Soft-light colour and strength (warmth at sunrise, blue at night). */
  glow: Rgb;
  glowAlpha: number;
  /** Saturation, 0..1. */
  sat: number;
  /** Shadow vector (px of offset per px of height), strength and colour. The vector changes in steps, so shadow layers re-bake rarely. */
  shadow: { dx: number; dy: number; alpha: number; color: string };
  /** How far lights are on (windows, doors, lamp heads), 0..1; each opening has its own switch-on point. */
  emissive: number;
  /** Strength of lamp pools, 0..1. */
  pool: number;
}

export type MarkerKind = "here" | "pin" | "target" | "door";

export interface ArtProvider {
  /** Art px per tile (16). */
  readonly tile: number;
  /** Colour beyond the edge of the map. */
  readonly outside: string;
  /** Walk frames per facing (frames 1..walkFrames; 0 stands). */
  readonly walkFrames: number;
  ground(kind: TerrainKind, cornerMask: number, variant: number): FrameRef;
  building(spec: BuildingSpec): BuiltBuilding;
  prop(key: string, variant: number): PropSprite;
  character(look: LookView, anim: AnimKey, facing: Facing, frame: number): FrameRef;
  grade(minuteOfDay: number): Grade;
  /** An SVG symbol id for a place kind's icon (HTML labels and lists). */
  icon(key: string): string;
  /** UI markers on the map (not in the tool plan; the placeholder draws them). */
  marker(kind: MarkerKind): FrameRef;
}

// ---------------------------------------------------------------------------
// From the layout to what the art asks for

/** Terrain kinds that join each other (a crossing is part of the road, a door part of the pavement). */
export function terrainGroup(t: number): number {
  switch (t) {
    case Terrain.Road:
    case Terrain.Crossing:
      return Terrain.Road;
    case Terrain.Pavement:
    case Terrain.Door:
    case Terrain.Building:
      return Terrain.Pavement;
    case Terrain.Water:
    case Terrain.Pier:
      return Terrain.Water;
    default:
      return t;
  }
}

/** Neighbour bits of `cornerMask`: set when that neighbour is in the same terrain group. */
export const MASK = { N: 1, E: 2, S: 4, W: 8, NE: 16, SE: 32, SW: 64, NW: 128 } as const;

/** The ground tile for every cell: neighbour mask and a stable variant (0..7); the kind is `map.terrain`. */
export function groundCells(map: TownMap): { mask: Uint8Array; variant: Uint8Array } {
  const W = map.width;
  const H = map.height;
  const mask = new Uint8Array(W * H);
  const variant = new Uint8Array(W * H);
  const dirs: [number, number, number][] = [
    [0, -1, MASK.N],
    [1, 0, MASK.E],
    [0, 1, MASK.S],
    [-1, 0, MASK.W],
    [1, -1, MASK.NE],
    [1, 1, MASK.SE],
    [-1, 1, MASK.SW],
    [-1, -1, MASK.NW],
  ];
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const i = y * W + x;
      const g = terrainGroup(map.terrain[i] as number);
      let m = 0;
      for (const [dx, dy, bit] of dirs) {
        const nx = x + dx;
        const ny = y + dy;
        // Off the map counts as joined, so edges draw no seams.
        if (nx < 0 || ny < 0 || nx >= W || ny >= H || terrainGroup(map.terrain[ny * W + nx] as number) === g) m |= bit;
      }
      mask[i] = m;
      // The low bit is a checkerboard, so dashed road lines alternate evenly.
      variant[i] = (mix(map.seed, x, y, 0x6e) & 6) | ((x + y) & 1);
    }
  }
  return { mask, variant };
}

/** Storeys a building is drawn with (the tool's kits read it). */
function storeysOf(b: TownBuilding): number {
  if (b.landmark) return 3;
  if (b.kind === "shed" || b.kind === "park" || b.kind === "barn") return 1;
  return b.h >= 4 ? 3 : 2;
}

/** The art's view of a laid-out building. */
export function buildingSpec(map: TownMap, b: TownBuilding): BuildingSpec {
  return {
    kind: b.home ? "home" : b.kind,
    hood: map.districts[b.district]?.kind ?? "inner",
    w: b.w,
    h: b.h,
    door: { x: b.doorX - b.x, side: "south" },
    storeys: storeysOf(b),
    // Places look the same wherever they end up; fillers by where they stand.
    visualSeed: b.place !== null ? mix(map.seed, 0x504c, b.place) : b.style,
    conditions: [],
    named: b.place !== null,
    home: b.home,
    landmark: b.landmark,
  };
}
