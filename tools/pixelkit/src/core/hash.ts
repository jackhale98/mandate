// The fixed hash (PIXEL_ART_TOOL.md §3.5 rule 9): 32-bit, integer-only, so
// every engine and device agrees. `hash32(a, b, ...)` folds words with the
// murmur3 finaliser; strings are folded as UTF-16 code units. Test vectors in
// test/hash.test.ts pin it (the engine will share them when it draws
// appearance in Rust).

const fmix = (h: number): number => {
  h ^= h >>> 16;
  h = Math.imul(h, 0x85ebca6b);
  h ^= h >>> 13;
  h = Math.imul(h, 0xc2b2ae35);
  h ^= h >>> 16;
  return h >>> 0;
};

/** Hash any number of 32-bit words or strings to a 32-bit unsigned integer. */
export function hash32(...parts: readonly (number | string)[]): number {
  let h = 0x9e3779b9;
  const word = (w: number) => {
    h = fmix((h ^ (w >>> 0)) + 0x7f4a7c15 + Math.imul(h, 31));
  };
  for (const p of parts) {
    if (typeof p === "number") word(p | 0);
    else {
      word(p.length);
      for (let i = 0; i < p.length; i++) word(p.charCodeAt(i));
    }
  }
  return h;
}

/** A hash as a fraction in [0, 1). */
export const unit = (...parts: readonly (number | string)[]): number => hash32(...parts) / 4294967296;

/**
 * Rendezvous (highest-random-weight) pick (§3.5 rule 6): every candidate gets
 * a score from (seed, its id), scaled by its weight, and the highest wins.
 * Adding a candidate only takes the picks it now wins; all others stay.
 * Weights are small whole numbers: a candidate of weight k enters k times
 * (`id#0` .. `id#k-1`), so the pick needs no logarithms and is bit-exact on
 * every engine.
 */
export function rendezvous<T extends { id: string; weight?: number }>(seed: number, salt: string, items: readonly T[]): T {
  let best: T | undefined;
  let bestScore = -1;
  for (const it of items) {
    const w = Math.round(it.weight ?? 1);
    for (let k = 0; k < w; k++) {
      const score = hash32(seed, salt, it.id, k);
      if (score > bestScore) {
        bestScore = score;
        best = it;
      }
    }
  }
  if (!best) throw new Error(`rendezvous: nothing to pick for ${salt}`);
  return best;
}

/** A small seeded generator (mulberry32) for sampling in lints and sheets; never for stored looks. */
export class Prng {
  private s: number;
  constructor(seed: number) {
    this.s = seed >>> 0;
  }
  next(): number {
    this.s = (this.s + 0x6d2b79f5) >>> 0;
    let t = this.s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  }
  int(lo: number, hi: number): number {
    return lo + Math.floor(this.next() * (hi - lo + 1));
  }
}
