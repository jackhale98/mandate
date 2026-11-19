// The compositor: layers as data (an ordered list, bottom first), each an
// indexed image placed at a position, optionally mirrored, clipped to what is
// under it, or drawn behind it; then slot recolouring and the automatic
// outline. Pure and integer-only: the same input gives the same pixels.
import { blank, blit, outline, remap, type BlitOptions, type Indexed } from "./image";
import type { Palette } from "./palette";

export interface LayerSpec extends BlitOptions {
  img: Indexed;
  x: number;
  y: number;
}

/** Stack layers bottom to top into a w×h image of codes. */
export function stack(w: number, h: number, layers: readonly LayerSpec[]): Indexed {
  const out = blank(w, h);
  for (const l of layers) blit(out, l.img, l.x, l.y, l);
  return out;
}

/**
 * Lengthen or shorten an image by repeating or removing one row (`row`):
 * how one drawing of a leg serves short, middle and tall figures.
 */
export function stretchRows(img: Indexed, row: number, delta: number): Indexed {
  if (delta === 0) return img;
  const h = img.h + delta;
  const out = blank(img.w, h);
  for (let y = 0; y < h; y++) {
    let sy: number;
    if (delta > 0) sy = y <= row ? y : y <= row + delta ? row : y - delta;
    else sy = y < row ? y : y - delta;
    out.px.set(img.px.subarray(sy * img.w, sy * img.w + img.w), y * img.w);
  }
  return out;
}

/** Oklab lightness per palette colour (for the outline's darkest-line rule), as integers 0..1000. */
export function paletteLightness(p: Palette, lightness: (hex: string) => number): number[] {
  return p.colors.map((c, i) => (i === 0 ? 1000 : Math.round(lightness(c) * 1000)));
}

/** Recolour slots through a table and add the outline (D2). */
export function finish(slots: Indexed, table: ArrayLike<number>, p: Palette, lightness: readonly number[]): Indexed {
  return outline(remap(slots, table), p.lineOf, lightness);
}
