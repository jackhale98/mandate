// The hub's art behind a small interface (DEC-049, DEC-080). The renderer
// only asks this for sprites and ground tiles; the placeholder art
// (./placeholderArt) draws them in code, and real atlases from the art tool
// replace it by implementing the same interface (a sprite is a region of
// any image: an atlas page, or a canvas composed from atlas parts).
import type { TownBuilding, TownMap, TownProp } from "../townLayout";

/** A 2D context the renderer and the art draw with (a canvas's, or a test stub). */
export type Ctx = CanvasRenderingContext2D;

/** A canvas made by a `CanvasFactory` (DOM canvas in the browser, a stub in tests). */
export interface Layer {
  canvas: HTMLCanvasElement;
  ctx: Ctx;
}

/** Makes offscreen canvases (the renderer's layers, the art's sprite sheets). */
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
 * A region of an image and where its anchor is. Anchors, in sprite px:
 * - buildings: the footprint's top-left corner (`ay` = roof px above the footprint);
 * - props: the top-left of the prop's cell (`ay` = px the prop rises above it);
 * - walkers: the feet (bottom centre);
 * - lights and markers: their centre.
 * Everything above a building's or prop's anchor row is its "overhang": it is
 * drawn again over walkers standing behind it.
 */
export interface Sprite {
  image: CanvasImageSource;
  sx: number;
  sy: number;
  w: number;
  h: number;
  ax: number;
  ay: number;
}

export type MarkerKind = "here" | "pin" | "target" | "door";

export interface TownArt {
  /** Art px per tile (16). */
  readonly tile: number;
  /** Colour beyond the edge of the map. */
  readonly outside: string;
  /** Colour of cast shadows (drawn at the sun's strength). */
  readonly shadow: string;
  /** Paint the ground of cell (x, y) with its top-left at (dx, dy) art px. */
  ground(ctx: Ctx, map: TownMap, x: number, y: number, dx: number, dy: number): void;
  /** A building: its footprint plus the roof rows above it (ROOF_ROWS). */
  building(map: TownMap, b: TownBuilding): Sprite;
  /** The building's lit windows, in the same frame as `building`, or null. */
  buildingLights(map: TownMap, b: TownBuilding): Sprite | null;
  /** Height in tiles, for its shadow. */
  buildingHeight(b: TownBuilding): number;
  prop(map: TownMap, p: TownProp): Sprite;
  /** Height in tiles for the prop's shadow (0: none). */
  propHeight(p: TownProp): number;
  /** Where a street lamp's light is, art px from the lamp cell's top-left, or null for other props. */
  lampLight(p: TownProp): { x: number; y: number } | null;
  /** A walker: `variant` < 0 is the player; dir 0 down, 1 up, 2 left, 3 right; frame 0 stands, 1..walkFrames steps. */
  walker(variant: number, dir: number, frame: number): Sprite;
  readonly walkFrames: number;
  /** A soft pool of lamp light (drawn additively). */
  lightPool(): Sprite;
  marker(kind: MarkerKind): Sprite;
}
