// Palette builder (PIXEL_ART_TOOL.md §1, §6): ramps designed in OKLCH from an
// anchor colour, with the hue shifting as well as the lightness (shadows
// cooler, highlights warmer), chroma peaking at the anchor, and an optional
// outline ("line") colour per ramp, darker and cooler than its darkest step.
// The result is frozen to hex: the runtime only ever sees 8-bit colours.
import { deltaE, hexToLch, hexToRgb, hueToward, lchToHex, type Rgb8 } from "./color";

export interface RampSpec {
  name: string;
  /** The ramp's anchor (its mid step), as #rrggbb. Kept exactly. */
  anchor: string;
  /** Steps darker and lighter than the anchor. */
  darker: number;
  lighter: number;
  /** Oklab lightness change per step (default 0.11). A list gives each step's change outward from the anchor. */
  dL?: number | readonly number[];
  /** Hue shift per step, degrees (default 12). */
  shift?: number;
  /** Hue shadows lean toward (default 275, blue-violet; skin uses red). */
  coolHue?: number;
  /** Hue highlights lean toward (default 85, warm yellow). */
  warmHue?: number;
  /** Chroma kept per step away from the anchor, darker and lighter (defaults 0.88 and 0.8). */
  chromaDark?: number;
  chromaLight?: number;
  /** Outline colour: false for none, or its Oklab lightness (default 0.22), or an exact #rrggbb. */
  line?: false | number | string;
  /** Hand-picked steps (darkest first) instead of generated ones; the anchor must be one of them. */
  fixed?: readonly string[];
  /**
   * This ramp's steps reuse an existing palette colour closer than this (Oklab ΔE) instead of adding their own
   * (default: the palette's `merge`). A ramp that nearly duplicates an earlier one is merged into it this way
   * (D11, round 4: palette headroom); the ramp keeps its name, so parts and kits that use it do not change.
   */
  merge?: number;
}

export interface Ramp {
  name: string;
  /** Palette indices, darkest to lightest. */
  steps: number[];
  /** Index of the anchor within `steps`. */
  anchorStep: number;
  /** Palette index of the outline colour (the darkest step when the ramp has none). */
  line: number;
}

/** A frozen palette. Index 0 is transparent; colours are 1..n. */
export interface Palette {
  /** `colors[0]` is a placeholder for transparent. */
  colors: string[];
  ramps: Ramp[];
  /** Per colour: the outline colour to use next to it. */
  lineOf: number[];
  /** Per colour: its ramp's index in `ramps` (or -1), and its step in the ramp (-1 for a line). */
  rampOf: number[];
  stepOf: number[];
}

export interface PaletteSpec {
  ramps: readonly RampSpec[];
  /** Colours closer than this (Oklab ΔE) are merged (default 0.012); lines merge at `lineMerge` (default 0.035). */
  merge?: number;
  lineMerge?: number;
  /** Lightest a generated step may be (Oklab L, default 0.955): never pure white. */
  maxL?: number;
}

/** Build a palette from ramp specs. Deterministic for a given spec. */
export function buildPalette(spec: PaletteSpec): Palette {
  const colors: string[] = ["#000000"];
  const rgb: Rgb8[] = [[0, 0, 0]];
  const isLine: boolean[] = [false];
  const merge = spec.merge ?? 0.012;
  const lineMerge = spec.lineMerge ?? 0.035;
  const add = (hex: string, line: boolean, stepMerge = merge): number => {
    const c = hexToRgb(hex);
    // The nearest colour within the limit (the first one found, round 1–3, when only the default limit applies).
    let best = -1;
    let bestD = Infinity;
    for (let i = 1; i < colors.length; i++) {
      if (colors[i] === hex) return i;
      const limit = line && isLine[i] ? lineMerge : line ? merge : stepMerge;
      const d = deltaE(c, rgb[i] as Rgb8);
      if (d < limit && (stepMerge === merge || line ? best < 0 : d < bestD)) {
        best = i;
        bestD = d;
      }
    }
    if (best >= 0) return best;
    colors.push(hex);
    rgb.push(c);
    isLine.push(line);
    return colors.length - 1;
  };
  const ramps: Ramp[] = [];
  const rampOf: number[] = [-1];
  const stepOf: number[] = [-1];
  for (const r of spec.ramps) {
    const a = hexToLch(r.anchor);
    const shift = r.shift ?? 12;
    const cool = r.coolHue ?? 275;
    const warm = r.warmHue ?? 85;
    const dl = (k: number) => (typeof r.dL === "number" || r.dL === undefined ? (r.dL ?? 0.11) * k : (r.dL as readonly number[]).slice(0, k).reduce((s, v) => s + v, 0));
    const hexes: string[] = [];
    if (r.fixed) hexes.push(...r.fixed.map((h) => h.toLowerCase()));
    else {
      for (let i = r.darker; i >= 1; i--) {
        hexes.push(lchToHex({ L: a.L - dl(i), C: a.C * Math.pow(r.chromaDark ?? 0.88, i), h: hueToward(a.h, cool, shift * i) }));
      }
      hexes.push(r.anchor.toLowerCase());
      for (let i = 1; i <= r.lighter; i++) {
        hexes.push(lchToHex({ L: Math.min(spec.maxL ?? 0.955, a.L + dl(i)), C: a.C * Math.pow(r.chromaLight ?? 0.8, i), h: hueToward(a.h, warm, shift * i) }));
      }
    }
    const anchorStep = hexes.indexOf(r.anchor.toLowerCase());
    if (anchorStep < 0) throw new Error(`ramp ${r.name}: the anchor is not one of its steps`);
    const steps = hexes.map((h) => add(h, false, r.merge ?? merge));
    let line = steps[0] as number;
    if (typeof r.line === "string") line = add(r.line.toLowerCase(), true);
    else if (r.line !== false) {
      const darkest = hexToLch(hexes[0] as string);
      const L = r.line ?? 0.22;
      line = add(lchToHex({ L: Math.min(L, darkest.L - 0.05), C: Math.min(0.045, darkest.C * 0.7), h: hueToward(darkest.h, cool, 25) }), true);
    }
    const index = ramps.length;
    ramps.push({ name: r.name, steps, anchorStep, line });
    steps.forEach((s, k) => {
      rampOf[s] ??= index;
      stepOf[s] ??= k;
    });
    rampOf[line] ??= index;
    stepOf[line] ??= -1;
  }
  for (let i = 0; i < colors.length; i++) {
    rampOf[i] ??= -1;
    stepOf[i] ??= -1;
  }
  const lineOf = colors.map((_, i) => {
    const ri = rampOf[i] as number;
    return ri < 0 ? i : (ramps[ri] as Ramp).line;
  });
  return { colors, ramps, lineOf, rampOf, stepOf };
}

/** A ramp by name (throws if missing: the pack and its palette must agree). */
export function ramp(p: Palette, name: string): Ramp {
  const r = p.ramps.find((x) => x.name === name);
  if (!r) throw new Error(`no ramp "${name}" in the palette`);
  return r;
}

/** Palette colours as RGB triples (index 0 is transparent and black). */
export function paletteRgb(p: Palette): Rgb8[] {
  return p.colors.map((c) => hexToRgb(c));
}
