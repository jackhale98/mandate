// Quarter-tile autotiling (PIXEL_ART_TOOL.md §2.5): each 16×16 tile is four
// 8×8 quarters, and each quarter is chosen by its two side neighbours and the
// corner neighbour between them: fill, horizontal edge, vertical edge, outer
// corner or inner corner. One hand-drawn 32×48 template per terrain (the
// RPG Maker "A2" layout) gives all 47 blob tiles, with lighting kept as drawn
// (nothing is rotated).
//
// Template layout, in 16 px tiles (columns × rows = 2 × 3):
//   (0,0) a preview tile (unused)      (1,0) the four inner corners
//   (0..1, 1..2) a 32×32 block: outer corners at its corners, edges along
//                its sides, and a seamless 16×16 fill in its middle.
import { blank, blit, crop, type Indexed } from "./image";

/** Neighbour bits (set when that neighbour is the same terrain group). */
export const NB = { N: 1, E: 2, S: 4, W: 8, NE: 16, SE: 32, SW: 64, NW: 128 } as const;

export type QuarterKind = "fill" | "edgeH" | "edgeV" | "outer" | "inner";

/** What a quarter shows (qx, qy: 0 left/top, 1 right/bottom). */
export function quarterKind(mask: number, qx: 0 | 1, qy: 0 | 1): QuarterKind {
  const vert = qy === 0 ? NB.N : NB.S;
  const horiz = qx === 0 ? NB.W : NB.E;
  const diag = qy === 0 ? (qx === 0 ? NB.NW : NB.NE) : qx === 0 ? NB.SW : NB.SE;
  const v = (mask & vert) !== 0;
  const h = (mask & horiz) !== 0;
  if (!v && !h) return "outer";
  if (!v) return "edgeH";
  if (!h) return "edgeV";
  return (mask & diag) !== 0 ? "fill" : "inner";
}

/** The template quarter (in 8 px units) for a quarter of a cell. */
export function templateQuarter(kind: QuarterKind, qx: 0 | 1, qy: 0 | 1): [number, number] {
  switch (kind) {
    case "fill":
      return [1 + qx, 3 + qy];
    case "edgeH":
      return [1 + qx, qy === 0 ? 2 : 5];
    case "edgeV":
      return [qx === 0 ? 0 : 3, 3 + qy];
    case "outer":
      return [qx * 3, 2 + qy * 3];
    case "inner":
      return [2 + qx, qy];
  }
}

/** Only the bits that change the tile (a corner matters only when both its sides join). */
export function canonicalMask(mask: number): number {
  let m = mask & 15;
  if (mask & NB.NE && mask & NB.N && mask & NB.E) m |= NB.NE;
  if (mask & NB.SE && mask & NB.S && mask & NB.E) m |= NB.SE;
  if (mask & NB.SW && mask & NB.S && mask & NB.W) m |= NB.SW;
  if (mask & NB.NW && mask & NB.N && mask & NB.W) m |= NB.NW;
  return m;
}

/** The 47 distinct canonical masks. */
export const BLOB_MASKS: readonly number[] = (() => {
  const s = new Set<number>();
  for (let m = 0; m < 256; m++) s.add(canonicalMask(m));
  return [...s].sort((a, b) => a - b);
})();

/**
 * Assemble one tile from a template. `fill`, if given, replaces the block's
 * middle for every fill quarter (a variant of the plain ground), so variety
 * carries into edge tiles too.
 */
export function autotile(template: Indexed, mask: number, fill?: Indexed): Indexed {
  if (template.w !== 32 || template.h !== 48) throw new Error(`an autotile template is 32×48, got ${template.w}×${template.h}`);
  const out = blank(16, 16);
  for (const qy of [0, 1] as const) {
    for (const qx of [0, 1] as const) {
      const kind = quarterKind(mask, qx, qy);
      if (kind === "fill" && fill) {
        blit(out, crop(fill, qx * 8, qy * 8, 8, 8), qx * 8, qy * 8);
        continue;
      }
      const [tx, ty] = templateQuarter(kind, qx, qy);
      blit(out, crop(template, tx * 8, ty * 8, 8, 8), qx * 8, qy * 8);
    }
  }
  return out;
}
