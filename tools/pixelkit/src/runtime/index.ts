// pixelkit's runtime: the small part of the tool a game imports (§4.2). Pure
// functions over indexed images; no DOM, no colour maths (palettes arrive
// frozen to hex). The game turns indexed images into pixels with `toRgba`.
export type { Indexed } from "../core/image";
export { blank, blit, mirrorX, remap } from "../core/image";
export { autotile, canonicalMask } from "../core/autotile";
export { packFromJson, packToJson } from "../core/serial";
export { hash32, rendezvous } from "../core/hash";

/** RGBA bytes per palette index (index 0 transparent) from #rrggbb colours. */
export function rgbaTable(colors: readonly string[]): Uint8Array {
  const t = new Uint8Array(colors.length * 4);
  colors.forEach((c, i) => {
    if (i === 0) return;
    t[i * 4] = parseInt(c.slice(1, 3), 16);
    t[i * 4 + 1] = parseInt(c.slice(3, 5), 16);
    t[i * 4 + 2] = parseInt(c.slice(5, 7), 16);
    t[i * 4 + 3] = 255;
  });
  return t;
}

/** An indexed image's pixels as RGBA, through a table from `rgbaTable`. */
export function toRgba(img: { w: number; h: number; px: Uint8Array }, table: Uint8Array, out?: Uint8ClampedArray): Uint8ClampedArray {
  const o = out ?? new Uint8ClampedArray(img.w * img.h * 4);
  for (let i = 0; i < img.px.length; i++) {
    const v = (img.px[i] as number) * 4;
    o[i * 4] = table[v] as number;
    o[i * 4 + 1] = table[v + 1] as number;
    o[i * 4 + 2] = table[v + 2] as number;
    o[i * 4 + 3] = table[v + 3] as number;
  }
  return o;
}
