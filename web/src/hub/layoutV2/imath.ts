// Integer maths for layout v2 (TOWN_GENERATION.md §2: ECMAScript's
// transcendental functions are implementation-approximated and differ between
// engines, so the generator never calls Math.sin, cos, exp, log, pow or
// hypot). Directions are 16-point compass steps with a fixed table of
// vectors (×1024), lengths are squared or integer square roots, noise is a
// fixed hash on a lattice, and everything else is integer arithmetic, which
// IEEE doubles do exactly below 2^53 in every engine.
import { mix } from "../rng";

/** 0 = north, clockwise in 22.5° steps (4 east, 8 south, 12 west); 255 = none. */
export type Compass16 = number;
export const NO_BEARING = 255;

/** round(1024 · sin(k · 22.5°)), k = 0..15. */
const SIN16 = [0, 392, 724, 946, 1024, 946, 724, 392, 0, -392, -724, -946, -1024, -946, -724, -392] as const;

/** The unit vector of a bearing, ×1024 (screen axes: y grows south). */
export function compassVec(k: Compass16): { dx: number; dy: number } {
  const i = k & 15;
  return { dx: SIN16[i] as number, dy: -(SIN16[(i + 4) & 15] as number) };
}

/** The bearing nearest a direction (dx, dy), by the largest dot product; none for a zero vector. */
export function bearingOf(dx: number, dy: number): Compass16 {
  if (dx === 0 && dy === 0) return NO_BEARING;
  let best = 0;
  let bestDot = -Infinity;
  for (let k = 0; k < 16; k++) {
    const v = compassVec(k);
    const d = dx * v.dx + dy * v.dy;
    if (d > bestDot) {
      bestDot = d;
      best = k;
    }
  }
  return best;
}

export const opposite = (k: Compass16): Compass16 => (k === NO_BEARING ? NO_BEARING : (k + 8) & 15);

/** Steps between two bearings, 0..8. */
export function bearingGap(a: Compass16, b: Compass16): number {
  const d = Math.abs((a & 15) - (b & 15));
  return d > 8 ? 16 - d : d;
}

/** floor(√n) for a non-negative integer. */
export function isqrt(n: number): number {
  if (n <= 0) return 0;
  let r = Math.floor(Math.sqrt(n));
  while (r * r > n) r--;
  while ((r + 1) * (r + 1) <= n) r++;
  return r;
}

/** floor(256 · log2(n)) for an integer n ≥ 1, by bit length and repeated squaring (exact integer steps). */
export function log2Q8(n: number): number {
  n = Math.max(1, Math.floor(n));
  let e = 0;
  while (2 ** (e + 1) <= n) e++;
  // Mantissa in [1, 2) as Q16.
  let m = Math.floor((n * 65536) / 2 ** e);
  let frac = 0;
  for (let bit = 128; bit >= 1; bit >>= 1) {
    m = Math.floor((m * m) / 65536);
    if (m >= 131072) {
      m = Math.floor(m / 2);
      frac |= bit;
    }
  }
  return e * 256 + frac;
}

/** round(65536 · 2^(i/16)), i = 0..16. */
const EXP2_16 = [65536, 68438, 71468, 74632, 77936, 81386, 84990, 88752, 92682, 96785, 101070, 105545, 110218, 115098, 120194, 125515, 131072] as const;

/** ≈ 2^(q/256) for q ≥ 0, as an integer (table and linear steps; the inverse of `log2Q8`). */
export function pow2Q8(q: number): number {
  q = Math.max(0, Math.floor(q));
  const e = Math.floor(q / 256);
  const f = q - e * 256;
  const i = f >> 4;
  const a = EXP2_16[i] as number;
  const b = EXP2_16[i + 1] as number;
  const m = a + Math.floor(((b - a) * (f & 15)) / 16);
  return Math.floor((m * 2 ** e) / 65536);
}

/** ≈ n^(num/den) for integers (areas ∝ population^0.7, §5 stage 5). */
export function ipow(n: number, num: number, den: number): number {
  return pow2Q8(Math.floor((log2Q8(n) * num) / den));
}

export const clamp =(v: number, lo: number, hi: number) => (v < lo ? lo : v > hi ? hi : v);

/** Integer division rounding to nearest (half away from zero). */
export const idiv = (a: number, b: number) => (a >= 0 ? Math.floor((a + Math.floor(b / 2)) / b) : -Math.floor((-a + Math.floor(b / 2)) / b));

/** A lattice hash in 0..1023. */
const lattice = (seed: number, x: number, y: number) => mix(seed, x | 0, y | 0) & 1023;

/** Smooth value noise in 0..1023 at (x, y) on a lattice of `cell` units (integers only). */
export function valueNoise(seed: number, x: number, y: number, cell: number): number {
  const gx = Math.floor(x / cell);
  const gy = Math.floor(y / cell);
  const tx = x - gx * cell;
  const ty = y - gy * cell;
  // Smoothstep weights in 0..cell²·… kept as integers: s(t) = t²(3c − 2t).
  const s = (t: number) => t * t * (3 * cell - 2 * t);
  const c3 = cell * cell * cell;
  const sx = s(tx);
  const sy = s(ty);
  const a = lattice(seed, gx, gy);
  const b = lattice(seed, gx + 1, gy);
  const c = lattice(seed, gx, gy + 1);
  const d = lattice(seed, gx + 1, gy + 1);
  const top = a * (c3 - sx) + b * sx;
  const bot = c * (c3 - sx) + d * sx;
  return Math.floor((top * (c3 - sy) + bot * sy) / (c3 * c3));
}

/** A binary min-heap of (key, value) integer pairs: the flood fills' and A*'s open sets. */
export class MinHeap {
  private k: Float64Array;
  private v: Int32Array;
  size = 0;
  constructor(capacity = 256) {
    this.k = new Float64Array(capacity);
    this.v = new Int32Array(capacity);
  }
  push(key: number, value: number) {
    if (this.size === this.k.length) {
      const k = new Float64Array(this.size * 2);
      k.set(this.k);
      const v = new Int32Array(this.size * 2);
      v.set(this.v);
      this.k = k;
      this.v = v;
    }
    let i = this.size++;
    while (i > 0) {
      const p = (i - 1) >> 1;
      const pk = this.k[p] as number;
      // Ties break on the value, so the order never depends on insertion history alone.
      if (pk < key || (pk === key && (this.v[p] as number) <= value)) break;
      this.k[i] = pk;
      this.v[i] = this.v[p] as number;
      i = p;
    }
    this.k[i] = key;
    this.v[i] = value;
  }
  /** The smallest key (Infinity when empty). */
  peekKey(): number {
    return this.size ? (this.k[0] as number) : Infinity;
  }
  /** Remove the smallest; returns its value (-1 when empty). `lastKey` holds its key. */
  lastKey = 0;
  pop(): number {
    if (!this.size) return -1;
    const top = this.v[0] as number;
    this.lastKey = this.k[0] as number;
    const n = --this.size;
    const key = this.k[n] as number;
    const value = this.v[n] as number;
    let i = 0;
    for (;;) {
      let c = 2 * i + 1;
      if (c >= n) break;
      const r = c + 1;
      if (r < n && ((this.k[r] as number) < (this.k[c] as number) || ((this.k[r] as number) === (this.k[c] as number) && (this.v[r] as number) < (this.v[c] as number)))) c = r;
      const ck = this.k[c] as number;
      if (key < ck || (key === ck && value <= (this.v[c] as number))) break;
      this.k[i] = ck;
      this.v[i] = this.v[c] as number;
      i = c;
    }
    this.k[i] = key;
    this.v[i] = value;
    return top;
  }
}
