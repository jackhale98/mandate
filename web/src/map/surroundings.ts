// What lies beyond the generated grid (presentation only): the engine's map
// ends at the edge of its grid, so a fitted view showed a hard cut. Around the
// grid we continue the open sea beyond coasts and neighbouring foreign land
// beyond land edges, falling off to sea at a distance, so the view never ends
// abruptly. Nothing here is part of the world: it has no towns, roads or
// labels, is drawn like foreign land, and never touches a real cell. It is
// deterministic from the world seed and the edge cells (DEC-041's ideas on
// the client: no Math.random, integer hashing only).
import type { MapView } from "../engine/protocol/MapView";
import { BIOME_RGB, decodeB64 } from "./palette";

const SEA = 0;
const FOREIGN = 1;
const COUNTRY = 2;
const LAKE = 3;
const GRASSLAND = 4;

/** Cells added on each side of the grid. */
export function surroundingsMargin(width: number, height: number): number {
  return Math.ceil(0.2 * Math.max(width, height));
}

export interface Surroundings {
  /** Cells added on each side; the real grid starts at (margin, margin). */
  margin: number;
  width: number;
  height: number;
  /** Per extended cell: 0 sea, 1 foreign, 2 country, 3 lake. Real cells are copied from the view. */
  cover: Uint8Array;
  biome: Uint8Array;
  /** Hillshade, 128 = flat. */
  shade: Uint8Array;
}

/** FNV-1a over the seed string. */
export function seedHash(seed: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < seed.length; i++) {
    h ^= seed.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

/** A hash of a lattice point to [0, 1). */
function hash2(x: number, y: number, s: number): number {
  let h = Math.imul(x | 0, 0x27d4eb2d) ^ Math.imul(y | 0, 0x165667b1) ^ s;
  h = Math.imul(h ^ (h >>> 15), 0x85ebca6b);
  h = Math.imul(h ^ (h >>> 13), 0xc2b2ae35);
  h ^= h >>> 16;
  return (h >>> 0) / 4294967296;
}

/** Smooth value noise in [0, 1) with a feature size of `scale` cells. */
function valueNoise(x: number, y: number, scale: number, s: number): number {
  const fx = x / scale;
  const fy = y / scale;
  const x0 = Math.floor(fx);
  const y0 = Math.floor(fy);
  const tx = fx - x0;
  const ty = fy - y0;
  const sx = tx * tx * (3 - 2 * tx);
  const sy = ty * ty * (3 - 2 * ty);
  const a = hash2(x0, y0, s);
  const b = hash2(x0 + 1, y0, s);
  const c = hash2(x0, y0 + 1, s);
  const d = hash2(x0 + 1, y0 + 1, s);
  return a + (b - a) * sx + (c - a) * sy + (a - b - c + d) * sx * sy;
}

function fbm(x: number, y: number, s: number): number {
  return 0.55 * valueNoise(x, y, 14, s) + 0.3 * valueNoise(x, y, 6, s ^ 0x9e3779b9) + 0.15 * valueNoise(x, y, 3, s ^ 0x7f4a7c15);
}

function smoothstep(a: number, b: number, t: number): number {
  const k = Math.max(0, Math.min(1, (t - a) / (b - a)));
  return k * k * (3 - 2 * k);
}

type GridView = Pick<MapView, "width" | "height" | "biome_b64" | "cover_b64" | "shade_b64">;

/**
 * The grid with a ring of surroundings `margin` cells wide. Each outside cell
 * looks at the nearest real cell: near a land edge (foreign or the country) it
 * continues as foreign land with a noisy coast; near a sea edge it stays sea;
 * everything falls off to sea by 85% of the margin, so the outer ring is open
 * sea that matches the map's background.
 */
export function extendWorld(view: GridView, seed: string, margin = surroundingsMargin(view.width, view.height)): Surroundings {
  const { width: w, height: h } = view;
  const cover = decodeB64(view.cover_b64);
  const biome = decodeB64(view.biome_b64);
  const shade = decodeB64(view.shade_b64);
  const W = w + 2 * margin;
  const H = h + 2 * margin;
  const out: Surroundings = { margin, width: W, height: H, cover: new Uint8Array(W * H), biome: new Uint8Array(W * H), shade: new Uint8Array(W * H).fill(128) };
  const s = seedHash(seed);
  const land = (x: number, y: number) => {
    const c = cover[y * w + x] ?? SEA;
    return c !== SEA ? 1 : 0;
  };
  // How much land there is near each edge cell (a 9×9 window), so the coast
  // beyond follows the edge's shape rather than single cells.
  const R = 4;
  const nearLand = new Float32Array(w * h).fill(-1);
  const landNear = (cx: number, cy: number) => {
    const key = cy * w + cx;
    const hit = nearLand[key] ?? -1;
    if (hit >= 0) return hit;
    let n = 0;
    let t = 0;
    for (let y = Math.max(0, cy - R); y <= Math.min(h - 1, cy + R); y++) {
      for (let x = Math.max(0, cx - R); x <= Math.min(w - 1, cx + R); x++) {
        n += land(x, y);
        t++;
      }
    }
    const v = t ? n / t : 0;
    nearLand[key] = v;
    return v;
  };
  // A land biome to continue: the edge cell's, else the nearest land inward.
  const landBiome = (cx: number, cy: number) => {
    const dx = cx === 0 ? 1 : cx === w - 1 ? -1 : 0;
    const dy = cy === 0 ? 1 : cy === h - 1 ? -1 : 0;
    for (let i = 0; i < 12; i++) {
      const x = Math.min(w - 1, Math.max(0, cx + dx * i));
      const y = Math.min(h - 1, Math.max(0, cy + dy * i));
      const c = cover[y * w + x] ?? SEA;
      const b = biome[y * w + x] ?? GRASSLAND;
      if (c === FOREIGN || c === COUNTRY) return b > 1 ? b : GRASSLAND;
    }
    return GRASSLAND;
  };

  for (let Y = 0; Y < H; Y++) {
    for (let X = 0; X < W; X++) {
      const I = Y * W + X;
      const x = X - margin;
      const y = Y - margin;
      if (x >= 0 && y >= 0 && x < w && y < h) {
        const i = y * w + x;
        out.cover[I] = cover[i] ?? SEA;
        out.biome[I] = biome[i] ?? 0;
        out.shade[I] = shade[i] ?? 128;
        continue;
      }
      // Outside cells start as open sea (the arrays' initial values).
      const ex = Math.min(w - 1, Math.max(0, x));
      const ey = Math.min(h - 1, Math.max(0, y));
      const t = Math.hypot(x - ex, y - ey) / margin;
      if (t >= 0.85) continue;
      // Wobble the edge sample along the edge so the land does not streak outward.
      const jx = Math.round((valueNoise(X, Y, 9, s ^ 0x51ed27) - 0.5) * 10);
      const jy = Math.round((valueNoise(X, Y, 9, s ^ 0x2545f4) - 0.5) * 10);
      const cx = Math.min(w - 1, Math.max(0, x + (y < 0 || y >= h ? jx : 0)));
      const cy = Math.min(h - 1, Math.max(0, y + (x < 0 || x >= w ? jy : 0)));
      const near = landNear(cx, cy);
      // Beyond open sea the noise alone (at most +0.2) never makes land.
      if (near === 0) continue;
      // Where the land gives way to sea wanders between 15% and 85% of the
      // margin, so the far coast is never a line parallel to the grid's edge.
      const reach = 0.15 + 0.7 * Math.max(0, Math.min(1, (fbm(X, Y, s ^ 0x1b873593) - 0.5) * 2.4 + 0.5));
      const v = near * (1 - smoothstep(reach - 0.1, reach + 0.1, t)) + (fbm(X, Y, s) - 0.5) * 0.4;
      if (v > 0.5) {
        out.cover[I] = FOREIGN;
        out.biome[I] = landBiome(cx, cy);
        // Relief from the slope of a second noise field, lit from the north-west.
        const hgt = (px: number, py: number) => valueNoise(px, py, 5, s ^ 0x3c6ef372);
        const slope = hgt(X - 1, Y - 1) - hgt(X + 1, Y + 1);
        out.shade[I] = Math.max(40, Math.min(216, Math.round(128 + slope * 500)));
      }
    }
  }
  return out;
}

/**
 * The surroundings at one pixel per cell (RGBA, row-major), drawn like the map's foreign land
 * (desaturated), with the real grid left transparent for the real raster.
 * Where the country itself reaches the edge of the grid, the first ring of
 * foreign land beyond it is drawn as a border line.
 */
export function paintSurroundings(s: Surroundings): Uint8ClampedArray<ArrayBuffer> {
  const { width: W, height: H, margin } = s;
  const px = new Uint8ClampedArray(W * H * 4);
  const w = W - 2 * margin;
  const h = H - 2 * margin;
  for (let Y = 0; Y < H; Y++) {
    for (let X = 0; X < W; X++) {
      const x = X - margin;
      const y = Y - margin;
      if (x >= 0 && y >= 0 && x < w && y < h) continue;
      const I = Y * W + X;
      const c = s.cover[I] ?? SEA;
      let [r, g, b] = BIOME_RGB[c === SEA ? 0 : (s.biome[I] ?? GRASSLAND)] ?? [0, 0, 0];
      if (c !== SEA && c !== LAKE) {
        const k = 1 + (((s.shade[I] ?? 128) - 128) / 128) * 0.7;
        const avg = ((r + g + b) / 3) * k;
        r = avg * 0.55 + 70;
        g = avg * 0.55 + 70;
        b = avg * 0.55 + 74;
        const ex = Math.min(w - 1, Math.max(0, x));
        const ey = Math.min(h - 1, Math.max(0, y));
        const touching = Math.max(Math.abs(x - ex), Math.abs(y - ey)) === 1;
        if (touching && s.cover[(ey + margin) * W + ex + margin] === COUNTRY) [r, g, b] = [36, 28, 30];
      }
      const p = I * 4;
      px[p] = r;
      px[p + 1] = g;
      px[p + 2] = b;
      px[p + 3] = 255;
    }
  }
  return px;
}
