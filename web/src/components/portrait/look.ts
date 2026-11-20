// A person's look (the pixel-art character the street draws), from the key
// the UI knows them by: `c<id>` for someone stored, `a<slot>` for a
// passer-by. The engine has no appearance yet (PIXEL_ART_TOOL §3.5: P1), so
// the look is a stable pick from the key, the same one the street and the
// portrait use. A passer-by met becomes someone stored under a new key; the
// street keeps the look they had (`TownScene.setStanding` hands the new key
// the same walker), and `keepLook` tells the portrait so, for this session.
// Presentation only.

/** A stable small hash of a string (FNV-1a). */
function hash(s: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h;
}

const kept = new Map<string, number>();

/** The look (the renderer's walker `variant`, a seed for `lookFromSeed`) for a key. */
export function lookVariant(key: string): number {
  return kept.get(key) ?? 100 + (hash(key) % 997);
}

/** Someone now known by `key` keeps the look `variant` (a passer-by met). */
export function keepLook(key: string, variant: number) {
  if (variant >= 0 && variant !== lookVariant(key)) kept.set(key, variant);
}

/** The key for a person in the engine's terms (a stored id, or a passer-by's slot). */
export function personKey(p: { id: number | null; ambient_slot: number | null }): string | null {
  return p.id !== null ? `c${p.id}` : p.ambient_slot !== null ? `a${p.ambient_slot}` : null;
}

/** Tests only: forget kept looks. */
export function resetLooks() {
  kept.clear();
}
