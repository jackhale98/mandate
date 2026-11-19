// Seeded, keyed random streams for the hub (DEC-041 ideas applied to UI
// layout): no Math.random, no clock. Each part of the layout draws from its
// own stream keyed by (seed, label, index), so changing how one part uses
// randomness never shifts another part's numbers. That keeps a town's look
// stable when the algorithm is touched in one place (see LAYOUT_VERSION).

/** 32-bit FNV-1a over a string. */
export function hashString(s: string, h = 0x811c9dc5): number {
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

/** Mix 32-bit integers into one well-spread 32-bit hash (murmur3 finaliser per word). */
export function mix(...words: number[]): number {
  let h = 0x9e3779b9;
  for (const w0 of words) {
    let k = Math.imul(w0 >>> 0, 0xcc9e2d51);
    k = (k << 15) | (k >>> 17);
    k = Math.imul(k, 0x1b873593);
    h ^= k;
    h = (h << 13) | (h >>> 19);
    h = (Math.imul(h, 5) + 0xe6546b64) >>> 0;
  }
  h ^= h >>> 16;
  h = Math.imul(h, 0x85ebca6b);
  h ^= h >>> 13;
  h = Math.imul(h, 0xc2b2ae35);
  h ^= h >>> 16;
  return h >>> 0;
}

/** A small seeded generator (mulberry32). */
export class Rng {
  private a: number;
  constructor(seed: number) {
    this.a = seed >>> 0;
  }
  /** Uniform in [0, 1). */
  next(): number {
    this.a = (this.a + 0x6d2b79f5) >>> 0;
    let t = this.a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  }
  /** Integer in [lo, hi], both included. */
  int(lo: number, hi: number): number {
    return lo + Math.floor(this.next() * (hi - lo + 1));
  }
  chance(p: number): boolean {
    return this.next() < p;
  }
  pick<T>(items: readonly T[]): T {
    const i = Math.floor(this.next() * items.length);
    return items[Math.min(items.length - 1, i)] as T;
  }
}

/** A stream keyed by (seed, label, ...indices): independent of every other stream. */
export function stream(seed: number, label: string, ...index: number[]): Rng {
  return new Rng(mix(seed, hashString(label), ...index));
}

/** A stable per-item number in [0, 1) without a stream (art variation). */
export function noise01(...words: number[]): number {
  return mix(...words) / 4294967296;
}
