// Colour spaces: sRGB (8-bit and linear), Oklab and OKLCH (Björn Ottosson,
// https://bottosson.github.io/posts/oklab/). Used at build time to design
// palettes, grading and moods, and by the lints to measure contrast. The
// runtime never calls these: everything it needs is frozen to 8-bit RGB.

export type Rgb8 = readonly [number, number, number];
export interface Lab {
  L: number;
  a: number;
  b: number;
}
export interface Lch {
  L: number;
  C: number;
  /** Degrees, 0..360. */
  h: number;
}

const clamp01 = (x: number) => (x < 0 ? 0 : x > 1 ? 1 : x);

export function hexToRgb(hex: string): Rgb8 {
  const s = hex.startsWith("#") ? hex.slice(1) : hex;
  if (!/^[0-9a-fA-F]{6}$/.test(s)) throw new Error(`not a #rrggbb colour: ${hex}`);
  return [parseInt(s.slice(0, 2), 16), parseInt(s.slice(2, 4), 16), parseInt(s.slice(4, 6), 16)];
}

export function rgbToHex(c: Rgb8): string {
  return `#${c.map((v) => Math.round(Math.min(255, Math.max(0, v))).toString(16).padStart(2, "0")).join("")}`;
}

const toLinear = (v: number) => {
  const c = v / 255;
  return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
};
const fromLinear = (c: number) => 255 * (c <= 0.0031308 ? 12.92 * c : 1.055 * Math.pow(c, 1 / 2.4) - 0.055);

export function linearToLab(r: number, g: number, b: number): Lab {
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return {
    L: 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    a: 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    b: 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  };
}

export function labToLinear(c: Lab): [number, number, number] {
  const l = (c.L + 0.3963377774 * c.a + 0.2158037573 * c.b) ** 3;
  const m = (c.L - 0.1055613458 * c.a - 0.0638541728 * c.b) ** 3;
  const s = (c.L - 0.0894841775 * c.a - 1.291485548 * c.b) ** 3;
  return [4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s, -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s, -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s];
}

export function rgbToLab(c: Rgb8): Lab {
  return linearToLab(toLinear(c[0]), toLinear(c[1]), toLinear(c[2]));
}

const inGamut = (lin: readonly number[]) => lin.every((v) => v >= -1e-4 && v <= 1 + 1e-4);

export function labToRgb(c: Lab): Rgb8 {
  const lin = labToLinear(c);
  return [Math.round(fromLinear(clamp01(lin[0]))), Math.round(fromLinear(clamp01(lin[1]))), Math.round(fromLinear(clamp01(lin[2])))];
}

export function labToLch(c: Lab): Lch {
  const C = Math.sqrt(c.a * c.a + c.b * c.b);
  let h = (Math.atan2(c.b, c.a) * 180) / Math.PI;
  if (h < 0) h += 360;
  return { L: c.L, C, h };
}

export function lchToLab(c: Lch): Lab {
  const r = (c.h * Math.PI) / 180;
  return { L: c.L, a: c.C * Math.cos(r), b: c.C * Math.sin(r) };
}

/** OKLCH to sRGB, reducing chroma (keeping lightness and hue) until it fits the gamut. */
export function lchToRgb(c: Lch): Rgb8 {
  const L = clamp01(c.L);
  if (inGamut(labToLinear(lchToLab({ ...c, L })))) return labToRgb(lchToLab({ ...c, L }));
  let lo = 0;
  let hi = c.C;
  for (let i = 0; i < 24; i++) {
    const mid = (lo + hi) / 2;
    if (inGamut(labToLinear(lchToLab({ L, C: mid, h: c.h })))) lo = mid;
    else hi = mid;
  }
  return labToRgb(lchToLab({ L, C: lo, h: c.h }));
}

export const rgbToLch = (c: Rgb8): Lch => labToLch(rgbToLab(c));
export const hexToLch = (hex: string): Lch => rgbToLch(hexToRgb(hex));
export const lchToHex = (c: Lch): string => rgbToHex(lchToRgb(c));

/** Euclidean distance in Oklab (about 0.02 is a just-noticeable step for flat colours). */
export function deltaE(x: Rgb8, y: Rgb8): number {
  const a = rgbToLab(x);
  const b = rgbToLab(y);
  return Math.sqrt((a.L - b.L) ** 2 + (a.a - b.a) ** 2 + (a.b - b.b) ** 2);
}

/** Oklab lightness, 0..1. */
export const lightness = (c: Rgb8): number => rgbToLab(c).L;

/** Signed shortest turn from hue `a` to hue `b`, degrees (-180..180]. */
export function hueDelta(a: number, b: number): number {
  let d = (((b - a) % 360) + 360) % 360;
  if (d > 180) d -= 360;
  return d;
}

/** Move hue `h` toward `target` by at most `deg` degrees. */
export function hueToward(h: number, target: number, deg: number): number {
  const d = hueDelta(h, target);
  const step = Math.sign(d) * Math.min(Math.abs(d), Math.abs(deg));
  return (((h + step) % 360) + 360) % 360;
}

/** Mix two colours in Oklab (t = 0 gives a, 1 gives b). */
export function mixLab(x: Rgb8, y: Rgb8, t: number): Rgb8 {
  const a = rgbToLab(x);
  const b = rgbToLab(y);
  return labToRgb({ L: a.L + (b.L - a.L) * t, a: a.a + (b.a - a.a) * t, b: a.b + (b.b - a.b) * t });
}

// ---------------------------------------------------------------------------
// Colour-vision deficiency (Machado, Oliveira and Fernandes 2009, severity 1.0),
// applied in linear RGB.

const MACHADO = {
  protan: [0.152286, 1.052583, -0.204868, 0.114503, 0.786281, 0.099216, -0.003882, -0.048116, 1.051998],
  deutan: [0.367322, 0.860646, -0.227968, 0.280085, 0.672501, 0.047413, -0.01182, 0.04294, 0.968881],
} as const;

export function simulateCvd(c: Rgb8, kind: keyof typeof MACHADO): Rgb8 {
  const m = MACHADO[kind];
  const r = toLinear(c[0]);
  const g = toLinear(c[1]);
  const b = toLinear(c[2]);
  const out = [m[0] * r + m[1] * g + m[2] * b, m[3] * r + m[4] * g + m[5] * b, m[6] * r + m[7] * g + m[8] * b];
  return [Math.round(fromLinear(clamp01(out[0] as number))), Math.round(fromLinear(clamp01(out[1] as number))), Math.round(fromLinear(clamp01(out[2] as number)))];
}
