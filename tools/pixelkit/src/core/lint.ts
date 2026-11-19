// The lints that apply to P0 (PIXEL_ART_TOOL.md §7.1): pure measurements on
// indexed images. The CLI runs them over the pack with the thresholds in
// §7.1 and reports errors and warnings.
import type { Indexed } from "./image";

const SIDES = [
  [0, 1],
  [0, -1],
  [-1, 0],
  [1, 0],
] as const;

const opaque = (img: Indexed, x: number, y: number) => x >= 0 && y >= 0 && x < img.w && y < img.h && img.px[y * img.w + x] !== 0;

/** Palette conformance: pixels whose value is not a colour of a palette of `n` colours (indices 1..n). */
export function offPalette(img: Indexed, n: number): number {
  let bad = 0;
  for (const v of img.px) if (v > n) bad++;
  return bad;
}

/** Orphans: opaque pixels with no side neighbour of the same value, except values in `allowed` (eyes, glints). */
export function orphans(img: Indexed, allowed: ReadonlySet<number> = new Set()): number {
  let n = 0;
  for (let y = 0; y < img.h; y++) {
    for (let x = 0; x < img.w; x++) {
      const v = img.px[y * img.w + x] as number;
      if (v === 0 || allowed.has(v)) continue;
      if (!SIDES.some(([dx, dy]) => opaque(img, x + dx, y + dy) && img.px[(y + dy) * img.w + x + dx] === v)) n++;
    }
  }
  return n;
}

/** Outline consistency: opaque pixels on the silhouette's edge that are not outline colours (0 for a closed outline). */
export function openOutline(img: Indexed, isLine: (v: number) => boolean): number {
  let n = 0;
  for (let y = 0; y < img.h; y++) {
    for (let x = 0; x < img.w; x++) {
      const v = img.px[y * img.w + x] as number;
      if (v === 0 || isLine(v)) continue;
      if (SIDES.some(([dx, dy]) => !opaque(img, x + dx, y + dy))) n++;
    }
  }
  return n;
}

/** Edge pixels of a silhouette (opaque, touching transparency on a side). */
export function edgePixels(img: Indexed): number[] {
  const out: number[] = [];
  for (let y = 0; y < img.h; y++) {
    for (let x = 0; x < img.w; x++) {
      const v = img.px[y * img.w + x] as number;
      if (v !== 0 && SIDES.some(([dx, dy]) => !opaque(img, x + dx, y + dy))) out.push(v);
    }
  }
  return out;
}

/** Intersection over union of two silhouettes (same size). */
export function silhouetteIoU(a: Indexed, b: Indexed): number {
  let i = 0;
  let u = 0;
  for (let k = 0; k < a.px.length; k++) {
    const x = a.px[k] !== 0;
    const y = b.px[k] !== 0;
    if (x && y) i++;
    if (x || y) u++;
  }
  return u === 0 ? 1 : i / u;
}

/** Distinct colours in an image. */
export function colourCount(img: Indexed): number {
  const s = new Set<number>();
  for (const v of img.px) if (v !== 0) s.add(v);
  return s.size;
}

/** Pairs in a crowd that share at least `k` of their visible features (the clone-rate lint). */
export function nearClonePairs(features: readonly (readonly string[])[], k: number): number {
  let n = 0;
  for (let i = 0; i < features.length; i++) {
    for (let j = i + 1; j < features.length; j++) {
      const a = features[i] as readonly string[];
      const b = features[j] as readonly string[];
      let same = 0;
      for (let f = 0; f < a.length; f++) if (a[f] === b[f]) same++;
      if (same >= k) n++;
    }
  }
  return n;
}
