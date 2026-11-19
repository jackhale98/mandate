// The town map overview's renderer (DEC-086, DEC-080: Canvas 2D, a static
// layer baked once, cheap overlays per frame). The bake draws the DEC-085
// layout zoomed out, a few px per tile: the ground (with neighbourhoods as
// softly tinted districts and dotted borders), the main routes between
// places, quiet filler blocks, trees and water, and every named place as a
// small recognisable landmark at map scale (the town hall's clock tower, the
// chamber's dome, the market's canopies, …), in the pixel-art pack's palette
// (./townMapPalette). A second, transparent layer holds the lit windows and
// street lamps, faded in at dusk. Per frame: one scaled blit, an optional
// gentle time-of-day grade, and the overlays (the route being previewed or
// walked, your home, where you are, places where something is waiting).
//
// Text is never drawn here: names are HTML over the canvas (DEC-080, DEC-051).
import { gradeAt } from "../dayCycle";
import { mix } from "../rng";
import { ROOF_ROWS, Terrain, type TownBuilding, type TownMap, type TownProp } from "../townLayout";
import type { Point } from "../pathfinding";
import type { TownMapScene } from "../townMapScene";
import { domCanvasFactory, type CanvasFactory, type Ctx } from "./art";
import { DISTRICT_TINT, MAP, rgb, type Rgb } from "./townMapPalette";

export interface BakedTownMap {
  /** The static map: ground, routes, buildings, trees. */
  canvas: HTMLCanvasElement;
  /** Lit windows and lamps (transparent elsewhere), drawn over the grade at dusk. */
  lights: HTMLCanvasElement;
  /** Canvas px per tile. */
  px: number;
  /** Tiles. */
  width: number;
  height: number;
  /** Time the bake took, ms, and by stage. */
  ms: number;
  stages: { ground: number; routes: number; buildings: number; props: number };
}

export interface BakeOptions {
  createCanvas?: CanvasFactory;
  /** Canvas px per tile (default: as many as fit `maxSide`, 4..8). */
  px?: number;
  /** Longest side of the baked canvas, px (default 1400). */
  maxSide?: number;
}

/** Px per tile for a map: crisp when zoomed in, bounded memory and bake time for the largest capital. */
export function bakePx(map: TownMap, maxSide = 1400): number {
  return Math.max(4, Math.min(8, Math.floor(maxSide / Math.max(map.width, map.height))));
}

// ---------------------------------------------------------------------------
// Ground: one pixel per tile scaled up natively, then textures drawn over runs of cells

const C = Object.fromEntries(Object.entries(MAP).map(([k, v]) => [k, rgb(v)])) as Record<keyof typeof MAP, Rgb>;

function terrainColour(t: number): Rgb {
  switch (t) {
    case Terrain.Road:
    case Terrain.Crossing:
      return C.asphalt;
    case Terrain.Pavement:
    case Terrain.Building:
    case Terrain.Door:
      return C.paving;
    case Terrain.Plaza:
      return C.stone;
    case Terrain.Water:
      return C.water;
    case Terrain.Park:
      return C.park;
    case Terrain.Path:
      return C.earthLight;
    case Terrain.Yard:
      return C.pavingDark;
    case Terrain.Field:
      return C.field;
    case Terrain.Dirt:
      return C.earth;
    case Terrain.Sand:
      return C.sand;
    case Terrain.Quay:
      return C.quay;
    case Terrain.Pier:
      return C.wood;
    case Terrain.Garden:
      return C.garden;
    default:
      return C.grass;
  }
}

const isWater = (t: number) => t === Terrain.Water;
const isRoad = (t: number) => t === Terrain.Road || t === Terrain.Crossing;
/** Ground that takes the district tint (not streets, water or the square). */
const tinted = (t: number) => !isRoad(t) && !isWater(t) && t !== Terrain.Plaza && t !== Terrain.Pier;

/**
 * Every cell's ground colour (the district tint folded in) as RGBA pixels, one
 * per tile (W × H). The bake scales it up natively, so a large town costs a
 * few thousand pixels of script, not millions. Pure and deterministic (tests
 * hash it).
 */
export function cellColours(map: TownMap): Uint8ClampedArray {
  const n = map.width * map.height;
  const out = new Uint8ClampedArray(n * 4);
  const cache = new Map<number, Rgb>();
  for (let i = 0; i < n; i++) {
    const t = map.terrain[i] as number;
    const d = map.district[i] as number;
    const key = t * 65536 + d + 1;
    let c = cache.get(key);
    if (!c) {
      c = terrainColour(t);
      // Neighbourhoods as soft tints (alternating strength, so neighbours of one kind still read apart).
      if (d >= 0 && tinted(t)) {
        const tint = DISTRICT_TINT[map.districts[d]?.kind ?? "inner"] ?? DISTRICT_TINT.inner;
        const a = d % 2 === 0 ? 0.3 : 0.2;
        if (tint) c = [Math.round(c[0] + (tint[0] - c[0]) * a), Math.round(c[1] + (tint[1] - c[1]) * a), Math.round(c[2] + (tint[2] - c[2]) * a)];
      }
      cache.set(key, c);
    }
    out[i * 4] = c[0];
    out[i * 4 + 1] = c[1];
    out[i * 4 + 2] = c[2];
    out[i * 4 + 3] = 255;
  }
  return out;
}

/** A short hash of a pixel buffer (determinism tests). */
export function pixelHash(px: Uint8ClampedArray): string {
  let h = 0x811c9dc5;
  for (let i = 0; i < px.length; i += 4) {
    h ^= (px[i] as number) | ((px[i + 1] as number) << 8) | ((px[i + 2] as number) << 16);
    h = Math.imul(h, 0x01000193);
  }
  return (h >>> 0).toString(16).padStart(8, "0");
}

/** Runs of cells in a row that pass a test: [y, x0, x1) — one draw call per run, not per cell. */
function forRuns(map: TownMap, test: (t: number) => boolean, draw: (y: number, x0: number, x1: number) => void) {
  const W = map.width;
  for (let y = 0; y < map.height; y++) {
    let x0 = -1;
    for (let x = 0; x <= W; x++) {
      const on = x < W && test(map.terrain[y * W + x] as number);
      if (on && x0 < 0) x0 = x;
      else if (!on && x0 >= 0) {
        draw(y, x0, x);
        x0 = -1;
      }
    }
  }
}

/** A small repeating texture (ripples, furrows, tufts), drawn over runs of its terrain. */
function texture(make: CanvasFactory, w: number, h: number, dots: (put: (x: number, y: number, c: string) => void) => void, ctx: Ctx): CanvasPattern | string {
  const tile = make(w, h);
  dots((x, y, c) => {
    tile.ctx.fillStyle = c;
    tile.ctx.fillRect(x, y, 1, 1);
  });
  return ctx.createPattern(tile.canvas, "repeat") ?? "transparent";
}

/** The ground at `px` per tile: cell colours scaled up, then textures, crossings, shores and district borders. */
function drawGround(ctx: Ctx, map: TownMap, px: number, make: CanvasFactory) {
  const W = map.width;
  const H = map.height;
  const cells = make(W, H);
  if (typeof ImageData !== "undefined") cells.ctx.putImageData(new ImageData(cellColours(map) as Uint8ClampedArray<ArrayBuffer>, W, H), 0, 0);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(cells.canvas, 0, 0, W * px, H * px);
  const seed = map.seed;
  const scatter = (w: number, h: number, n: number, colour: string, salt: number) => (put: (x: number, y: number, c: string) => void) => {
    for (let k = 0; k < n; k++) {
      const r = mix(seed, salt, k);
      put(r % w, (r >>> 12) % h, colour);
    }
  };
  // Water: ripples and glints.
  const ripples = texture(make, 4 * px, 2 * px, (put) => {
    for (let k = 0; k < 3; k++) {
      const r = mix(seed, 0x51, k);
      const x = r % (4 * px);
      const y = (r >>> 10) % (2 * px);
      for (let u = 0; u < 3; u++) put((x + u) % (4 * px), y, MAP.waterDark);
    }
    scatter(4 * px, 2 * px, 2, MAP.waterLight, 0x52)(put);
  }, ctx);
  const furrows = texture(make, 1, 4, (put) => {
    put(0, 0, MAP.fieldDark);
    put(0, 1, MAP.fieldDark);
  }, ctx);
  const tufts = texture(make, 5 * px + 3, 4 * px + 1, scatter(5 * px + 3, 4 * px + 1, 14, MAP.grassDark, 0x53), ctx);
  const leaves = texture(make, 5 * px + 3, 4 * px + 1, scatter(5 * px + 3, 4 * px + 1, 16, MAP.foliage, 0x54), ctx);
  const flags = texture(make, px, px, (put) => {
    for (let u = 0; u < px; u++) put(u, 0, MAP.stoneDark);
    for (let v = 1; v < px; v++) put(0, v, MAP.stoneDark);
  }, ctx);
  const planks = texture(make, 1, 3, (put) => put(0, 0, MAP.woodDark), ctx);
  const layer = (pattern: CanvasPattern | string, test: (t: number) => boolean) => {
    ctx.fillStyle = pattern;
    forRuns(map, test, (y, x0, x1) => ctx.fillRect(x0 * px, y * px, (x1 - x0) * px, px));
  };
  layer(ripples, (t) => t === Terrain.Water);
  layer(furrows, (t) => t === Terrain.Field);
  layer(tufts, (t) => t === Terrain.Grass || t === Terrain.Garden);
  layer(leaves, (t) => t === Terrain.Park);
  layer(flags, (t) => t === Terrain.Plaza);
  layer(planks, (t) => t === Terrain.Pier);
  // Zebra crossings, light shores, and dotted borders between neighbourhoods (off the streets, which already part them).
  const tAt = (x: number, y: number) => (x < 0 || y < 0 || x >= W || y >= H ? Terrain.Grass : (map.terrain[y * W + x] as number));
  const dAt = (x: number, y: number) => (x < 0 || y < 0 || x >= W || y >= H ? -1 : (map.district[y * W + x] as number));
  const dots = texture(make, 2, 2, (put) => {
    put(0, 0, MAP.line);
    put(1, 1, MAP.line);
  }, ctx);
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const t = map.terrain[y * W + x] as number;
      const X = x * px;
      const Y = y * px;
      if (t === Terrain.Crossing) {
        const roadH = isRoad(tAt(x - 1, y)) || isRoad(tAt(x + 1, y));
        ctx.fillStyle = MAP.chalkDark;
        for (let a = 0; a < px; a += 2) {
          if (roadH) ctx.fillRect(X + a, Y + 1, 1, px - 2);
          else ctx.fillRect(X + 1, Y + a, px - 2, 1);
        }
      } else if (t === Terrain.Water) {
        ctx.fillStyle = MAP.waterLight;
        if (!isWater(tAt(x, y - 1))) ctx.fillRect(X, Y, px, 1);
        if (!isWater(tAt(x, y + 1))) ctx.fillRect(X, Y + px - 1, px, 1);
        if (!isWater(tAt(x - 1, y))) ctx.fillRect(X, Y, 1, px);
        if (!isWater(tAt(x + 1, y))) ctx.fillRect(X + px - 1, Y, 1, px);
      }
      const d = map.district[y * W + x] as number;
      if (d >= 0 && !isRoad(t)) {
        const dr = dAt(x + 1, y);
        const db = dAt(x, y + 1);
        const right = dr >= 0 && dr !== d && !isRoad(tAt(x + 1, y));
        const below = db >= 0 && db !== d && !isRoad(tAt(x, y + 1));
        if (right || below) {
          ctx.fillStyle = dots;
          if (right) ctx.fillRect(X + px - 1, Y, 1, px);
          if (below) ctx.fillRect(X, Y + px - 1, px, 1);
        }
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Buildings at map scale

interface Pen {
  ctx: Ctx;
  lit: Ctx;
  px: number;
}

const rect = (ctx: Ctx, colour: string, x: number, y: number, w: number, h: number) => {
  if (w <= 0 || h <= 0) return;
  ctx.fillStyle = colour;
  ctx.fillRect(Math.round(x), Math.round(y), Math.max(1, Math.round(w)), Math.max(1, Math.round(h)));
};

const disc = (ctx: Ctx, colour: string, x: number, y: number, r: number) => {
  ctx.fillStyle = colour;
  ctx.beginPath();
  ctx.arc(x, y, Math.max(0.5, r), 0, Math.PI * 2);
  ctx.fill();
};

const poly = (ctx: Ctx, colour: string, pts: readonly (readonly [number, number])[]) => {
  ctx.fillStyle = colour;
  ctx.beginPath();
  pts.forEach(([x, y], i) => (i === 0 ? ctx.moveTo(x, y) : ctx.lineTo(x, y)));
  ctx.closePath();
  ctx.fill();
};

interface Look {
  roof: string;
  roofDark: string;
  wall: string;
  wallDark: string;
  window: string;
  /** Windows that light up at dusk (0..1 of them). */
  lit: number;
}

const LOOKS: Record<string, Look> = {
  house: { roof: MAP.roofHouse, roofDark: MAP.roofHouseDark, wall: MAP.wallQuiet, wallDark: MAP.wallQuietDark, window: MAP.glass, lit: 0.5 },
  terrace: { roof: MAP.roofTerrace, roofDark: MAP.slateDark, wall: MAP.wallQuiet, wallDark: MAP.wallQuietDark, window: MAP.glass, lit: 0.5 },
  shop: { roof: MAP.roofShop, roofDark: MAP.pavingDark, wall: MAP.wallQuiet, wallDark: MAP.wallQuietDark, window: MAP.glass, lit: 0.6 },
  warehouse: { roof: MAP.roofWarehouse, roofDark: MAP.slateDark, wall: MAP.pavingDark, wallDark: MAP.slateDark, window: MAP.iron, lit: 0.1 },
  shed: { roof: MAP.roofShed, roofDark: MAP.woodDark, wall: MAP.earthLight, wallDark: MAP.earth, window: MAP.iron, lit: 0 },
  farmhouse: { roof: MAP.roofHouse, roofDark: MAP.roofHouseDark, wall: MAP.render, wallDark: MAP.renderDark, window: MAP.glass, lit: 0.5 },
  barn: { roof: MAP.brickDark, roofDark: MAP.woodDark, wall: MAP.brick, wallDark: MAP.brickDark, window: MAP.woodDark, lit: 0 },
  boathouse: { roof: MAP.slate, roofDark: MAP.slateDark, wall: MAP.wood, wallDark: MAP.woodDark, window: MAP.woodDark, lit: 0.2 },
  home: { roof: MAP.terracotta, roofDark: MAP.terracottaDark, wall: MAP.chalk, wallDark: MAP.chalkDark, window: MAP.glassLight, lit: 1 },
};

const PLACE_LOOKS: Record<string, Look> = {
  town_hall: { roof: MAP.slate, roofDark: MAP.slateDark, wall: MAP.stone, wallDark: MAP.stoneDark, window: MAP.glass, lit: 0.8 },
  council_chamber: { roof: MAP.slateLight, roofDark: MAP.slate, wall: MAP.chalk, wallDark: MAP.chalkDark, window: MAP.glass, lit: 0.7 },
  parliament: { roof: MAP.slateLight, roofDark: MAP.slate, wall: MAP.chalk, wallDark: MAP.chalkDark, window: MAP.glass, lit: 0.7 },
  regional_assembly: { roof: MAP.slateLight, roofDark: MAP.slate, wall: MAP.stone, wallDark: MAP.stoneDark, window: MAP.glass, lit: 0.7 },
  executive_residence: { roof: MAP.slate, roofDark: MAP.slateDark, wall: MAP.chalk, wallDark: MAP.chalkDark, window: MAP.glass, lit: 0.8 },
  market: { roof: MAP.paving, roofDark: MAP.pavingDark, wall: MAP.wood, wallDark: MAP.woodDark, window: MAP.wood, lit: 0.3 },
  diner: { roof: MAP.terracotta, roofDark: MAP.terracottaDark, wall: MAP.chalk, wallDark: MAP.chalkDark, window: MAP.glassLight, lit: 1 },
  news_office: { roof: MAP.slate, roofDark: MAP.slateDark, wall: MAP.brick, wallDark: MAP.brickDark, window: MAP.glass, lit: 0.9 },
  party_office: { roof: MAP.terracotta, roofDark: MAP.terracottaDark, wall: MAP.render, wallDark: MAP.renderDark, window: MAP.glass, lit: 0.9 },
  union_hall: { roof: MAP.slateDark, roofDark: MAP.iron, wall: MAP.brick, wallDark: MAP.brickDark, window: MAP.glass, lit: 0.8 },
  business_association: { roof: MAP.iron, roofDark: MAP.line, wall: MAP.glass, wallDark: MAP.slateDark, window: MAP.glassLight, lit: 0.6 },
  clinic: { roof: MAP.chalkDark, roofDark: MAP.paving, wall: MAP.chalk, wallDark: MAP.chalkDark, window: MAP.glass, lit: 0.9 },
  school: { roof: MAP.slate, roofDark: MAP.slateDark, wall: MAP.brick, wallDark: MAP.brickDark, window: MAP.glass, lit: 0.3 },
  community_centre: { roof: MAP.wood, roofDark: MAP.woodDark, wall: MAP.render, wallDark: MAP.renderDark, window: MAP.glass, lit: 0.8 },
  hotel: { roof: MAP.slate, roofDark: MAP.slateDark, wall: MAP.render, wallDark: MAP.renderDark, window: MAP.glass, lit: 0.9 },
  university: { roof: MAP.slate, roofDark: MAP.slateDark, wall: MAP.stone, wallDark: MAP.stoneDark, window: MAP.glass, lit: 0.6 },
  tech_campus: { roof: MAP.partyGreen, roofDark: MAP.verdigrisDark, wall: MAP.glass, wallDark: MAP.slateDark, window: MAP.glassLight, lit: 0.8 },
  factory: { roof: MAP.ironLight, roofDark: MAP.iron, wall: MAP.brick, wallDark: MAP.brickDark, window: MAP.iron, lit: 0.4 },
  port: { roof: MAP.slate, roofDark: MAP.slateDark, wall: MAP.wood, wallDark: MAP.woodDark, window: MAP.iron, lit: 0.3 },
  farm: { roof: MAP.brickDark, roofDark: MAP.woodDark, wall: MAP.brick, wallDark: MAP.brickDark, window: MAP.woodDark, lit: 0.2 },
  mine: { roof: MAP.iron, roofDark: MAP.line, wall: MAP.earth, wallDark: MAP.woodDark, window: MAP.iron, lit: 0.2 },
  mill: { roof: MAP.slate, roofDark: MAP.slateDark, wall: MAP.stone, wallDark: MAP.stoneDark, window: MAP.glass, lit: 0.3 },
  park: { roof: MAP.foliage, roofDark: MAP.foliageDark, wall: MAP.foliage, wallDark: MAP.foliageDark, window: MAP.foliage, lit: 0 },
};

function lookOf(b: TownBuilding): Look {
  if (b.home) return LOOKS.home as Look;
  if (b.place !== null) return PLACE_LOOKS[b.kind] ?? (LOOKS.shop as Look);
  return LOOKS[b.kind] ?? (LOOKS.house as Look);
}

/** A block in three-quarter view at map scale: a shadow, a roof with a ridge, a front wall with windows and a door. */
function block(p: Pen, b: TownBuilding, look: Look, seed: number, opts: { wallRatio?: number; windows?: boolean } = {}) {
  const { ctx, lit, px } = p;
  const x = b.x * px;
  const y = (b.y - ROOF_ROWS) * px;
  const w = b.w * px;
  const h = (b.h + ROOF_ROWS) * px;
  const wallH = Math.max(3, Math.min(Math.round(px * 1.7), Math.round(h * (opts.wallRatio ?? 0.38))));
  const roofH = h - wallH;
  // Shadow to the south-east (the street art's afternoon sun).
  ctx.fillStyle = "rgba(17,19,26,0.28)";
  ctx.fillRect(Math.round(x + px * 0.4), Math.round(y + px * 0.4), Math.round(w), Math.round(h));
  // Roof: lighter far slope, darker near slope, eave line.
  rect(ctx, look.roof, x, y, w, roofH);
  rect(ctx, look.roofDark, x, y + Math.round(roofH * 0.55), w, roofH - Math.round(roofH * 0.55));
  rect(ctx, look.roofDark, x, y + roofH - 1, w, 1);
  if (roofH >= 5) rect(ctx, look.roof === MAP.chalk ? MAP.chalkDark : "rgba(255,255,255,0.16)", x + 1, y + 1, w - 2, 1);
  // Wall.
  rect(ctx, look.wall, x, y + roofH, w, wallH);
  rect(ctx, look.wallDark, x, y + h - 1, w, 1);
  // Windows: one per tile on the wall (doors excepted), some lit at dusk.
  if (opts.windows !== false && wallH >= 4) {
    const ww = Math.max(1, Math.round(px * 0.3));
    const wh = Math.max(1, Math.round(wallH * 0.35));
    const wy = y + roofH + Math.max(1, Math.round(wallH * 0.2));
    for (let tx = b.x; tx < b.x + b.w; tx++) {
      if (tx === b.doorX) continue;
      const wx = tx * px + Math.round((px - ww) / 2);
      rect(ctx, look.window, wx, wy, ww, wh);
      if (((mix(seed, tx, 0x11) % 100) / 100) < look.lit) rect(lit, MAP.lampLight, wx, wy, ww, wh);
    }
  }
  // Door.
  const dw = Math.max(2, Math.round(px * 0.45));
  const dh = Math.max(2, Math.round(wallH * 0.62));
  const dx = b.doorX * px + Math.round((px - dw) / 2);
  rect(ctx, MAP.woodDark, dx, y + h - dh, dw, dh);
  if (b.place !== null || b.home) rect(lit, MAP.lamp, dx, y + h - dh, dw, Math.max(1, Math.round(dh / 2)));
  return { x, y, w, h, roofH, wallH };
}

/** A pennant on a pole rising from (x, y): the logo's flag (DEC-081) or a party colour. */
function flagpole(p: Pen, x: number, y: number, colour: string) {
  const { ctx, px } = p;
  const ph = Math.round(px * 1.4);
  rect(ctx, MAP.ink, x, y - ph, 1, ph);
  rect(ctx, colour, x + 1, y - ph, Math.max(3, Math.round(px * 0.55)), Math.max(2, Math.round(px * 0.35)));
}

/** Columns along a front wall (civic buildings). */
function columns(p: Pen, box: { x: number; y: number; w: number; h: number; roofH: number; wallH: number }, colour: string, every: number) {
  const { ctx } = p;
  for (let cx = box.x + 2; cx < box.x + box.w - 1; cx += every) rect(ctx, colour, cx, box.y + box.roofH + 1, 1, box.wallH - 2);
}

function drawPlace(p: Pen, b: TownBuilding, seed: number) {
  const { ctx, lit, px } = p;
  const look = lookOf(b);
  const u = px / 8;
  switch (b.kind) {
    case "town_hall": {
      const box = block(p, b, look, seed);
      columns(p, box, MAP.stoneDark, Math.max(3, Math.round(px * 0.75)));
      // Clock tower in the middle, rising above the roof, with a gold face and a pointed cap.
      const tw = Math.max(5, Math.round(px * 1.5));
      const tx = Math.round(box.x + box.w / 2 - tw / 2);
      const top = box.y - Math.round(px * 1.2);
      rect(ctx, MAP.line, tx - 1, top - 1, tw + 2, box.roofH + Math.round(px * 1.2) + 1);
      rect(ctx, MAP.stone, tx, top, tw, box.roofH + Math.round(px * 1.2));
      rect(ctx, MAP.stoneDark, tx + tw - 1, top, 1, box.roofH + Math.round(px * 1.2));
      poly(ctx, MAP.slateDark, [[tx - 1, top], [tx + tw / 2, top - Math.round(px * 1.0)], [tx + tw + 1, top]]);
      disc(ctx, MAP.lamp, tx + tw / 2, top + Math.round(px * 0.65), Math.max(1.5, px * 0.42));
      disc(lit, MAP.lampCore, tx + tw / 2, top + Math.round(px * 0.65), Math.max(1.5, px * 0.42));
      rect(ctx, MAP.ink, Math.round(tx + tw / 2), top + Math.round(px * 0.35), 1, Math.max(1, Math.round(px * 0.3)));
      flagpole(p, Math.round(tx + tw / 2), top - Math.round(px * 1.0), MAP.flag);
      break;
    }
    case "council_chamber":
    case "parliament":
    case "regional_assembly": {
      const box = block(p, b, look, seed, { wallRatio: 0.42 });
      columns(p, box, MAP.chalkDark, Math.max(2, Math.round(px * 0.5)));
      // Pediment and a verdigris dome with a lantern.
      const cx = box.x + box.w / 2;
      const r = Math.max(3, Math.min(box.w * 0.28, px * (b.kind === "parliament" ? 2.2 : 1.7)));
      const base = box.y + box.roofH * 0.6;
      rect(ctx, MAP.chalkDark, cx - r - 1, base - 1, 2 * r + 2, 2);
      ctx.fillStyle = MAP.verdigris;
      ctx.beginPath();
      ctx.arc(cx, base, r, Math.PI, 0);
      ctx.fill();
      ctx.fillStyle = MAP.verdigrisDark;
      ctx.beginPath();
      ctx.arc(cx, base, r, Math.PI * 1.5, 0);
      ctx.lineTo(cx, base);
      ctx.fill();
      rect(ctx, MAP.chalk, cx - Math.max(1, u), base - r - Math.round(px * 0.5), Math.max(2, 2 * u), Math.round(px * 0.5));
      if (b.kind !== "council_chamber") flagpole(p, Math.round(cx), base - r - Math.round(px * 0.5), MAP.flag);
      break;
    }
    case "executive_residence": {
      const box = block(p, b, look, seed);
      columns(p, box, MAP.chalkDark, Math.max(3, Math.round(px * 0.7)));
      const cx = box.x + box.w / 2;
      poly(ctx, MAP.chalkDark, [[cx - px * 1.6, box.y + box.roofH], [cx, box.y - px * 0.3], [cx + px * 1.6, box.y + box.roofH]]);
      flagpole(p, Math.round(cx), Math.round(box.y - px * 0.3), MAP.flag);
      break;
    }
    case "market": {
      // A row of striped canopies over stalls: no walls, the square shows between them.
      const x = b.x * px;
      const y = (b.y - ROOF_ROWS) * px;
      const w = b.w * px;
      const h = (b.h + ROOF_ROWS) * px;
      rect(ctx, MAP.pavingDark, x, y + h * 0.45, w, h * 0.55);
      const n = Math.max(2, Math.round(b.w / 1.5));
      const cw = w / n;
      const stripes = [MAP.flag, MAP.partyBlue, MAP.partyGreen, MAP.lamp];
      for (let i = 0; i < n; i++) {
        const sx = x + i * cw + 1;
        const colour = stripes[(i + (seed & 3)) % stripes.length] as string;
        ctx.fillStyle = "rgba(17,19,26,0.25)";
        ctx.fillRect(Math.round(sx + 1), Math.round(y + h * 0.3 + 2), Math.round(cw - 2), Math.round(h * 0.4));
        rect(ctx, colour, sx, y + h * 0.25, cw - 2, h * 0.32);
        for (let k = 1; k < cw - 2; k += 3) rect(ctx, MAP.chalk, sx + k, y + h * 0.25, 1, h * 0.32);
        // Scalloped edge and the stall beneath.
        rect(ctx, colour, sx, y + h * 0.57, cw - 2, Math.max(1, u));
        rect(ctx, MAP.wood, sx + 1, y + h * 0.62, cw - 4, h * 0.22);
        rect(lit, MAP.lampLight, sx + cw / 2 - 1, y + h * 0.6, 2, 1);
      }
      break;
    }
    case "diner": {
      const box = block(p, b, look, seed);
      // A striped awning along the front and a round sign on the roof.
      const ay = box.y + box.roofH;
      rect(ctx, MAP.cross, box.x, ay, box.w, Math.max(2, Math.round(px * 0.35)));
      for (let k = 0; k < box.w; k += 2) rect(ctx, MAP.chalk, box.x + k, ay, 1, Math.max(2, Math.round(px * 0.35)));
      const sx = box.x + box.w * 0.72;
      rect(ctx, MAP.ink, sx, box.y - Math.round(px * 0.5), 1, Math.round(px * 0.6));
      disc(ctx, MAP.lamp, sx, box.y - Math.round(px * 0.7), Math.max(2, px * 0.45));
      disc(ctx, MAP.flag, sx, box.y - Math.round(px * 0.7), Math.max(1, px * 0.22));
      disc(lit, MAP.lampLight, sx, box.y - Math.round(px * 0.7), Math.max(2, px * 0.45));
      break;
    }
    case "news_office": {
      const box = block(p, b, look, seed);
      // A broadsheet on the roof: a white sheet with columns of type.
      const sw = Math.max(5, Math.round(box.w * 0.6));
      const sh = Math.max(4, Math.round(px * 1.1));
      const sx = Math.round(box.x + (box.w - sw) / 2);
      const sy = box.y - Math.round(sh * 0.45);
      rect(ctx, MAP.chalk, sx, sy, sw, sh);
      rect(ctx, MAP.ink, sx + 1, sy + 1, sw - 2, 1);
      for (let ly = sy + 3; ly < sy + sh - 1; ly += 2) rect(ctx, MAP.ironLight, sx + 1, ly, sw - 2, 1);
      rect(ctx, MAP.chalkDark, sx + sw, sy + 1, 1, sh);
      break;
    }
    case "party_office": {
      const box = block(p, b, look, seed);
      // Bunting in the three party colours and a flag.
      const by = box.y + box.roofH + 1;
      const cols = [MAP.partyBlue, MAP.flag, MAP.partyGreen];
      for (let k = 0, i = 0; k < box.w - 1; k += 2, i++) rect(ctx, cols[i % 3] as string, box.x + k + 1, by, 1, 2);
      flagpole(p, Math.round(box.x + box.w * 0.25), box.y + 1, MAP.partyBlue);
      break;
    }
    case "union_hall": {
      const box = block(p, b, look, seed);
      // A gable front and a red banner hanging from it.
      const cx = box.x + box.w / 2;
      poly(ctx, MAP.brickDark, [[box.x, box.y + box.roofH], [cx, box.y - px * 0.4], [box.x + box.w, box.y + box.roofH]]);
      poly(ctx, MAP.brick, [[box.x + 2, box.y + box.roofH], [cx, box.y - px * 0.4 + 2], [box.x + box.w - 2, box.y + box.roofH]]);
      rect(ctx, MAP.cross, cx - px * 0.45, box.y + box.roofH * 0.35, px * 0.9, box.roofH * 0.5 + box.wallH * 0.4);
      rect(ctx, MAP.chalk, cx - px * 0.2, box.y + box.roofH * 0.5, px * 0.4, Math.max(1, u));
      break;
    }
    case "business_association":
    case "tech_campus": {
      // A glass tower: taller than its street.
      const box = block(p, b, look, seed, { wallRatio: 0.6 });
      for (let gy = box.y + box.roofH + 1; gy < box.y + box.h - 2; gy += 2) rect(ctx, MAP.glassLight, box.x + 1, gy, box.w - 2, 1);
      for (let gy = box.y + box.roofH + 1; gy < box.y + box.h - 2; gy += 4) if (mix(seed, gy) & 1) rect(lit, MAP.lampLight, box.x + 1, gy, box.w - 2, 1);
      break;
    }
    case "clinic": {
      const box = block(p, b, look, seed);
      // A red cross on a white roof.
      const s = Math.max(3, Math.round(px * 0.9));
      const cx = Math.round(box.x + box.w / 2);
      const cy = Math.round(box.y + box.roofH / 2);
      const t = Math.max(1, Math.round(s / 3));
      rect(ctx, MAP.cross, cx - Math.floor(s / 2), cy - Math.floor(t / 2), s, t);
      rect(ctx, MAP.cross, cx - Math.floor(t / 2), cy - Math.floor(s / 2), t, s);
      break;
    }
    case "school": {
      const box = block(p, b, look, seed);
      // A belfry on the ridge.
      const cx = Math.round(box.x + box.w / 2);
      const bw = Math.max(3, Math.round(px * 0.8));
      rect(ctx, MAP.chalk, cx - bw / 2, box.y - Math.round(px * 0.5), bw, Math.round(px * 0.8));
      poly(ctx, MAP.slateDark, [[cx - bw / 2 - 1, box.y - Math.round(px * 0.5)], [cx, box.y - Math.round(px * 1.1)], [cx + bw / 2 + 1, box.y - Math.round(px * 0.5)]]);
      disc(ctx, MAP.lamp, cx, box.y - Math.round(px * 0.15), Math.max(1, px * 0.18));
      break;
    }
    case "community_centre": {
      const box = block(p, b, look, seed);
      // A round window in a gable.
      const cx = box.x + box.w / 2;
      poly(ctx, MAP.woodDark, [[box.x + 1, box.y + box.roofH], [cx, box.y - px * 0.3], [box.x + box.w - 1, box.y + box.roofH]]);
      disc(ctx, MAP.lamp, cx, box.y + box.roofH * 0.5, Math.max(1.5, px * 0.35));
      disc(lit, MAP.lampLight, cx, box.y + box.roofH * 0.5, Math.max(1.5, px * 0.35));
      break;
    }
    case "hotel": {
      const box = block(p, b, look, seed, { wallRatio: 0.55 });
      // Rows of windows, an awning and a flag.
      for (let ry = box.y + box.roofH + 2; ry < box.y + box.h - 3; ry += 3)
        for (let tx = b.x; tx < b.x + b.w; tx++) {
          rect(ctx, MAP.glass, tx * px + px * 0.35, ry, Math.max(1, px * 0.3), 1);
          if (mix(seed, tx, ry) % 3 === 0) rect(lit, MAP.lampLight, tx * px + px * 0.35, ry, Math.max(1, px * 0.3), 1);
        }
      rect(ctx, MAP.partyBlue, b.doorX * px - 1, box.y + box.h - Math.round(box.wallH * 0.75), px + 2, Math.max(1, u * 1.5));
      flagpole(p, Math.round(box.x + 2), box.y + 1, MAP.flag);
      break;
    }
    case "university": {
      const box = block(p, b, look, seed);
      columns(p, box, MAP.stoneDark, Math.max(3, Math.round(px * 0.6)));
      const tw = Math.max(4, Math.round(px));
      const tx = Math.round(box.x + box.w * 0.2);
      rect(ctx, MAP.stone, tx, box.y - Math.round(px * 0.9), tw, box.roofH + Math.round(px * 0.9));
      poly(ctx, MAP.slateDark, [[tx - 1, box.y - Math.round(px * 0.9)], [tx + tw / 2, box.y - Math.round(px * 1.6)], [tx + tw + 1, box.y - Math.round(px * 0.9)]]);
      break;
    }
    case "factory": {
      const x = b.x * px;
      const y = (b.y - ROOF_ROWS) * px;
      const w = b.w * px;
      const box = block(p, b, look, seed, { wallRatio: 0.35 });
      // Sawtooth roof and a chimney with a puff of smoke.
      const teeth = Math.max(2, Math.round(b.w / 1.5));
      const tw = w / teeth;
      for (let i = 0; i < teeth; i++) {
        const tx = x + i * tw;
        poly(ctx, MAP.ironLight, [[tx, y + box.roofH], [tx + tw, y + box.roofH], [tx + tw, y + 1]]);
        rect(ctx, MAP.glassLight, tx + tw - 2, y + 2, 1, box.roofH - 3);
      }
      const cx = Math.round(x + w * 0.8);
      rect(ctx, MAP.brickDark, cx, y - Math.round(px * 1.6), Math.max(2, Math.round(px * 0.5)), Math.round(px * 1.6) + 2);
      disc(ctx, "rgba(236,229,214,0.8)", cx + px * 0.6, y - px * 1.9, px * 0.45);
      disc(ctx, "rgba(236,229,214,0.6)", cx + px * 1.2, y - px * 2.3, px * 0.55);
      break;
    }
    case "port": {
      const box = block(p, b, look, seed, { wallRatio: 0.4 });
      // Containers on the quay and a crane.
      const cols = [MAP.flag, MAP.partyBlue, MAP.partyGreen];
      for (let i = 0; i < Math.max(2, b.w - 3); i++) rect(ctx, cols[(i + seed) % 3] as string, box.x + i * px + 1, box.y + box.roofH * 0.3, px - 2, box.roofH * 0.5);
      const cx = box.x + box.w - px;
      rect(ctx, MAP.lamp, cx, box.y - px * 1.6, Math.max(1, u * 1.5), px * 1.6 + box.roofH);
      rect(ctx, MAP.lamp, cx - px * 1.8, box.y - px * 1.6, px * 2.6, Math.max(1, u * 1.5));
      rect(ctx, MAP.ink, cx - px * 1.4, box.y - px * 1.6, 1, px * 0.9);
      break;
    }
    case "farm": {
      const box = block(p, b, look, seed);
      // A barn door with a white cross brace.
      const dw = Math.max(4, Math.round(px * 1.4));
      const dx = Math.round(box.x + box.w / 2 - dw / 2);
      const dh = box.wallH - 1;
      rect(ctx, MAP.brickDark, dx, box.y + box.h - dh, dw, dh);
      ctx.strokeStyle = MAP.chalk;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(dx, box.y + box.h - dh);
      ctx.lineTo(dx + dw, box.y + box.h);
      ctx.moveTo(dx + dw, box.y + box.h - dh);
      ctx.lineTo(dx, box.y + box.h);
      ctx.stroke();
      break;
    }
    case "mine": {
      const box = block(p, b, look, seed, { wallRatio: 0.3 });
      // A headframe: an A of girders with a wheel.
      const cx = box.x + box.w * 0.35;
      ctx.strokeStyle = MAP.iron;
      ctx.lineWidth = Math.max(1, u * 1.5);
      ctx.beginPath();
      ctx.moveTo(cx - px, box.y + box.roofH);
      ctx.lineTo(cx, box.y - px * 1.4);
      ctx.lineTo(cx + px, box.y + box.roofH);
      ctx.stroke();
      disc(ctx, MAP.ironLight, cx, box.y - px * 1.3, Math.max(1.5, px * 0.4));
      poly(ctx, MAP.earthLight, [[box.x + box.w * 0.6, box.y + box.roofH], [box.x + box.w * 0.8, box.y + 1], [box.x + box.w, box.y + box.roofH]]);
      break;
    }
    case "mill": {
      const box = block(p, b, look, seed);
      // A water wheel on the side.
      const r = Math.max(2, px * 0.9);
      const cx = box.x + box.w - r;
      const cy = box.y + box.h - r;
      disc(ctx, MAP.woodDark, cx, cy, r);
      disc(ctx, MAP.wood, cx, cy, r * 0.65);
      rect(ctx, MAP.woodDark, cx - r, cy, 2 * r, 1);
      rect(ctx, MAP.woodDark, cx, cy - r, 1, 2 * r);
      break;
    }
    case "park": {
      // Trees round a path cross and a bench: no building.
      const x = b.x * px;
      const y = (b.y - ROOF_ROWS) * px;
      const w = b.w * px;
      const h = (b.h + ROOF_ROWS) * px;
      rect(ctx, MAP.park, x, y, w, h);
      rect(ctx, MAP.earthLight, x, y + h / 2, w, Math.max(1, u * 1.5));
      rect(ctx, MAP.earthLight, x + w / 2, y, Math.max(1, u * 1.5), h);
      for (const [fx, fy] of [[0.22, 0.25], [0.78, 0.28], [0.25, 0.78], [0.76, 0.75]] as const) tree(p, x + w * fx, y + h * fy, px * 0.5, seed);
      break;
    }
    default:
      block(p, b, look, seed);
  }
}

function tree(p: Pen, x: number, y: number, r: number, seed: number) {
  const { ctx } = p;
  disc(ctx, "rgba(17,19,26,0.25)", x + r * 0.35, y + r * 0.45, r);
  disc(ctx, MAP.foliageDark, x, y, r);
  disc(ctx, (seed & 1) === 0 ? MAP.foliage : MAP.foliageLight, x - r * 0.2, y - r * 0.2, r * 0.7);
}

function prop(p: Pen, pr: TownProp, seed: number) {
  const { ctx, lit, px } = p;
  const cx = (pr.x + 0.5) * px;
  const cy = (pr.y + 0.5) * px;
  switch (pr.kind) {
    case "tree":
      tree(p, cx, cy - px * 0.2, px * 0.48, mix(seed, pr.x, pr.y));
      break;
    case "pine":
      disc(ctx, "rgba(17,19,26,0.25)", cx + px * 0.2, cy + px * 0.25, px * 0.35);
      poly(ctx, MAP.foliageDark, [[cx - px * 0.4, cy + px * 0.3], [cx, cy - px * 0.6], [cx + px * 0.4, cy + px * 0.3]]);
      break;
    case "lamp":
      rect(ctx, MAP.iron, cx, cy - px * 0.3, 1, 1);
      rect(lit, MAP.lampCore, cx - 0.5, cy - px * 0.3 - 0.5, 2, 2);
      lit.fillStyle = "rgba(255,215,122,0.35)";
      lit.fillRect(Math.round(cx - px * 0.5), Math.round(cy - px * 0.3), Math.round(px), Math.round(px * 0.7));
      break;
    case "fountain":
      disc(ctx, MAP.stoneDark, cx, cy, px * 0.75);
      disc(ctx, MAP.waterLight, cx, cy, px * 0.5);
      disc(ctx, MAP.chalk, cx, cy, px * 0.15);
      break;
    case "boat":
      poly(ctx, MAP.wood, [[cx - px * 0.6, cy - px * 0.15], [cx + px * 0.6, cy - px * 0.15], [cx + px * 0.4, cy + px * 0.25], [cx - px * 0.4, cy + px * 0.25]]);
      rect(ctx, MAP.chalk, cx - 1, cy - px * 0.6, 1, px * 0.45);
      break;
    case "crane":
      rect(ctx, MAP.lamp, cx, cy - px * 1.2, 1, px * 1.2);
      rect(ctx, MAP.lamp, cx - px * 0.8, cy - px * 1.2, px * 1.4, 1);
      break;
    case "stall":
      rect(ctx, MAP.flag, cx - px * 0.4, cy - px * 0.4, px * 0.8, px * 0.4);
      rect(ctx, MAP.chalk, cx - px * 0.1, cy - px * 0.4, 1, px * 0.4);
      break;
    case "flag":
      flagpole(p, Math.round(cx), Math.round(cy), MAP.flag);
      break;
    default:
      break;
  }
}

/**
 * How far (tiles) a place's glyph rises above its roof row: clock towers,
 * chimneys, cranes, flags. Name chips sit above it, so a name never hides a
 * landmark's most recognisable part.
 */
export const GLYPH_RISE: Readonly<Record<string, number>> = {
  town_hall: 3.2,
  council_chamber: 0.8,
  parliament: 1.2,
  regional_assembly: 1.2,
  executive_residence: 1.0,
  factory: 2.3,
  port: 1.6,
  mine: 1.5,
  school: 1.1,
  university: 1.6,
  diner: 1.0,
  news_office: 0.5,
  party_office: 1.4,
  hotel: 1.4,
  union_hall: 0.4,
  community_centre: 0.3,
};

// ---------------------------------------------------------------------------
// The bake

/** Bake the static layers of a town map. */
export function bakeTownMap(scene: TownMapScene, options: BakeOptions = {}): BakedTownMap {
  const t0 = performance.now();
  const map = scene.map;
  const make = options.createCanvas ?? domCanvasFactory;
  const px = options.px ?? bakePx(map, options.maxSide);
  const PW = map.width * px;
  const PH = map.height * px;
  const base = make(PW, PH);
  const lights = make(PW, PH);
  const ctx = base.ctx;
  ctx.imageSmoothingEnabled = false;
  // Ground.
  drawGround(ctx, map, px, make);
  const t1 = performance.now();
  // Main routes: the streets that join the square to every place, drawn as a lighter line along them.
  ctx.lineJoin = "round";
  ctx.lineCap = "round";
  const stroke = (pts: readonly Point[], colour: string, width: number) => {
    ctx.strokeStyle = colour;
    ctx.lineWidth = width;
    ctx.beginPath();
    pts.forEach((q, i) => (i === 0 ? ctx.moveTo(q.x * px, q.y * px) : ctx.lineTo(q.x * px, q.y * px)));
    ctx.stroke();
  };
  for (const r of scene.routes) stroke(r, MAP.asphaltDark, Math.max(2, px * 0.6));
  for (const r of scene.routes) stroke(r, MAP.stone, Math.max(1, px * 0.28));
  const t3 = performance.now();
  const pen: Pen = { ctx, lit: lights.ctx, px };
  // Buildings from the back to the front (towers behind are overlapped by roofs in front).
  const order = [...map.buildings].sort((a, b) => a.y + a.h - (b.y + b.h) || a.x - b.x);
  for (const b of order) {
    const seed = b.place !== null ? mix(map.seed, 0x504c, b.place) : b.style;
    if (b.place !== null) drawPlace(pen, b, seed);
    else block(pen, b, lookOf(b), seed, { windows: b.kind !== "shed" && b.kind !== "barn" });
  }
  // Trees, lamps, boats and the fountain.
  const t4 = performance.now();
  for (const pr of map.props) prop(pen, pr, map.seed);
  const t5 = performance.now();
  return {
    canvas: base.canvas,
    lights: lights.canvas,
    px,
    width: map.width,
    height: map.height,
    ms: t5 - t0,
    stages: { ground: t1 - t0, routes: t3 - t1, buildings: t4 - t3, props: t5 - t4 },
  };
}

// ---------------------------------------------------------------------------
// A frame

export interface MapCamera {
  /** Tile at the view centre. */
  x: number;
  y: number;
  /** CSS px per tile. */
  zoom: number;
}

export interface MapMarker {
  at: Point;
  kind: "you" | "home" | "waiting";
}

export interface MapFrame {
  cam: MapCamera;
  /** View size, CSS px. */
  width: number;
  height: number;
  /** Backing-store px per CSS px. */
  scale: number;
  /** Minutes after midnight for the gentle time-of-day grade, or null for none. */
  minutes: number | null;
  /** Strength of the grade (0..1; the street uses 1). */
  gradeStrength: number;
  /** The route previewed or walked (tiles), or null. */
  route: readonly Point[] | null;
  /** How far along `route` the walk has gone (tiles), or null for a preview. */
  walked: number | null;
  markers: readonly MapMarker[];
}

/** The colour beyond the map's edge: the countryside the town sits in. */
export const OUTSIDE = MAP.grass;

/** Draw one frame: the baked map scaled, the grade, the lights, the overlays. */
export function drawTownMapFrame(ctx: Ctx, baked: BakedTownMap, f: MapFrame) {
  const { cam, width, height, scale } = f;
  ctx.setTransform(scale, 0, 0, scale, 0, 0);
  ctx.globalAlpha = 1;
  ctx.globalCompositeOperation = "source-over";
  const ox = width / 2 - cam.x * cam.zoom;
  const oy = height / 2 - cam.y * cam.zoom;
  const mw = baked.width * cam.zoom;
  const mh = baked.height * cam.zoom;
  if (ox > 0 || oy > 0 || ox + mw < width || oy + mh < height) {
    ctx.fillStyle = OUTSIDE;
    ctx.fillRect(0, 0, width, height);
  }
  // Upscaled: nearest neighbour (pixel art); downscaled: smoothed, so thin streets do not shimmer.
  ctx.imageSmoothingEnabled = cam.zoom * scale < baked.px;
  ctx.drawImage(baked.canvas, ox, oy, mw, mh);
  if (f.minutes !== null && f.gradeStrength > 0) {
    const g = gradeAt(f.minutes);
    const s = f.gradeStrength;
    const m = g.mul.map((v) => Math.round(255 - (255 - v) * s));
    ctx.globalCompositeOperation = "multiply";
    ctx.fillStyle = `rgb(${m[0]},${m[1]},${m[2]})`;
    ctx.fillRect(0, 0, width, height);
    if (g.glowAlpha > 0) {
      ctx.globalCompositeOperation = "soft-light";
      ctx.globalAlpha = Math.min(1, g.glowAlpha * s);
      ctx.fillStyle = `rgb(${g.glow[0]},${g.glow[1]},${g.glow[2]})`;
      ctx.fillRect(0, 0, width, height);
    }
    ctx.globalCompositeOperation = "source-over";
    const glow = g.lights * Math.min(1, s * 1.6);
    if (glow > 0.02) {
      ctx.globalAlpha = Math.min(1, glow);
      ctx.drawImage(baked.lights, ox, oy, mw, mh);
    }
    ctx.globalAlpha = 1;
  }
  const at = (p: Point) => ({ x: ox + p.x * cam.zoom, y: oy + p.y * cam.zoom });
  // The route: a gold dashed line with a dark casing (shape and colour); walked part solid.
  if (f.route && f.route.length > 1) {
    const pts = f.route.map(at);
    ctx.lineJoin = "round";
    ctx.lineCap = "round";
    const path = () => {
      ctx.beginPath();
      pts.forEach((q, i) => (i === 0 ? ctx.moveTo(q.x, q.y) : ctx.lineTo(q.x, q.y)));
    };
    ctx.setLineDash([]);
    ctx.strokeStyle = "rgba(17,19,26,0.85)";
    ctx.lineWidth = 6;
    path();
    ctx.stroke();
    ctx.strokeStyle = MAP.lamp;
    ctx.lineWidth = 3;
    ctx.setLineDash([7, 5]);
    path();
    ctx.stroke();
    ctx.setLineDash([]);
  }
  for (const m of f.markers) {
    const p = at(m.at);
    if (m.kind === "home") homeBadge(ctx, p.x, p.y);
    else if (m.kind === "waiting") waitingBadge(ctx, p.x, p.y);
  }
  // Where you are, on top of everything.
  for (const m of f.markers) if (m.kind === "you") youPin(ctx, at(m.at).x, at(m.at).y);
}

/** A pin whose tip is at (x, y): a gold head with a dark ring and a dark centre. */
function youPin(ctx: Ctx, x: number, y: number) {
  const hy = y - 15;
  ctx.fillStyle = "rgba(17,19,26,0.35)";
  ctx.beginPath();
  ctx.ellipse(x, y, 6, 2.5, 0, 0, Math.PI * 2);
  ctx.fill();
  ctx.fillStyle = MAP.ink;
  ctx.beginPath();
  ctx.moveTo(x, y);
  ctx.lineTo(x - 7, hy + 3);
  ctx.arc(x, hy, 9, Math.PI * 0.8, Math.PI * 0.2);
  ctx.closePath();
  ctx.fill();
  ctx.fillStyle = MAP.lamp;
  ctx.beginPath();
  ctx.moveTo(x, y - 2.5);
  ctx.lineTo(x - 5.2, hy + 2.6);
  ctx.arc(x, hy, 7, Math.PI * 0.8, Math.PI * 0.2);
  ctx.closePath();
  ctx.fill();
  ctx.fillStyle = MAP.ink;
  ctx.beginPath();
  ctx.arc(x, hy, 2.8, 0, Math.PI * 2);
  ctx.fill();
}

/** A little house above (x, y): pale walls, a terracotta roof, a gold door. */
function homeBadge(ctx: Ctx, x: number, y: number) {
  const s = 7;
  const by = y - 4;
  ctx.fillStyle = MAP.ink;
  ctx.beginPath();
  ctx.moveTo(x - s - 2, by - s + 1);
  ctx.lineTo(x, by - 2 * s - 2);
  ctx.lineTo(x + s + 2, by - s + 1);
  ctx.lineTo(x + s, by - s + 1);
  ctx.lineTo(x + s, by + 1);
  ctx.lineTo(x - s, by + 1);
  ctx.lineTo(x - s, by - s + 1);
  ctx.closePath();
  ctx.fill();
  ctx.fillStyle = MAP.chalk;
  ctx.fillRect(x - s + 1.5, by - s + 1, 2 * s - 3, s - 1);
  ctx.fillStyle = MAP.terracotta;
  ctx.beginPath();
  ctx.moveTo(x - s, by - s + 0.5);
  ctx.lineTo(x, by - 2 * s);
  ctx.lineTo(x + s, by - s + 0.5);
  ctx.closePath();
  ctx.fill();
  ctx.fillStyle = MAP.lamp;
  ctx.fillRect(x - 1.5, by - 4.5, 3, 4.5);
}

/** A speech bubble with an exclamation mark at (x, y): something is waiting here. */
function waitingBadge(ctx: Ctx, x: number, y: number) {
  ctx.fillStyle = MAP.ink;
  ctx.beginPath();
  ctx.arc(x, y, 8.5, 0, Math.PI * 2);
  ctx.moveTo(x - 6, y + 4);
  ctx.lineTo(x - 9, y + 11);
  ctx.lineTo(x - 1, y + 7);
  ctx.fill();
  ctx.fillStyle = MAP.flag;
  ctx.beginPath();
  ctx.arc(x, y, 6.8, 0, Math.PI * 2);
  ctx.moveTo(x - 5, y + 3.5);
  ctx.lineTo(x - 7, y + 8.5);
  ctx.lineTo(x - 1.5, y + 5.5);
  ctx.fill();
  ctx.fillStyle = MAP.ink;
  ctx.fillRect(x - 1.2, y - 4.5, 2.4, 5.5);
  ctx.fillRect(x - 1.2, y + 2.2, 2.4, 2.4);
}
