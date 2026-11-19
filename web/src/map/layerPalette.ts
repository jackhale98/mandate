// Colours of the country map's layers (DEC-089), one set per theme. Steps are
// from the dataviz reference palette (blue ramp; red arm matched to the blue
// steps' OKLCH lightness at the palette red's hue), checked with its validator:
//  - sequential: one hue, light → dark on light themes; on dark themes the low
//    end recedes into the dark (adjacent ΔL ≥ 0.06 for every class count);
//  - diverging: blue below, red above, a grey middle class; equal steps per arm;
//  - categorical: slots 1–3 (the party colours), which pass the all-pairs CVD
//    and normal-vision floors a map needs, then a grey "Other" that clears them
//    too. More categories fold into Other; colours are never cycled.
// Colour is never the only signal: the legend names every class, the readout
// gives each town's figure and the List shows them all (DEC-051).
import type { MapLayerScaleView } from "../engine/protocol/MapLayerScaleView";

export type Scheme = "light" | "dark";

const SEQUENTIAL: Record<Scheme, Record<number, readonly string[]>> = {
  light: {
    2: ["#6da7ec", "#184f95"],
    3: ["#b7d3f6", "#2a78d6", "#0d366b"],
    4: ["#b7d3f6", "#5598e7", "#1c5cab", "#0d366b"],
    5: ["#b7d3f6", "#6da7ec", "#2a78d6", "#1c5cab", "#0d366b"],
    6: ["#cde2fb", "#86b6ef", "#3987e5", "#256abf", "#184f95", "#0d366b"],
    7: ["#cde2fb", "#9ec5f4", "#6da7ec", "#3987e5", "#256abf", "#184f95", "#0d366b"],
  },
  dark: {
    2: ["#1c5cab", "#86b6ef"],
    3: ["#184f95", "#3987e5", "#b7d3f6"],
    4: ["#184f95", "#2a78d6", "#6da7ec", "#b7d3f6"],
    5: ["#184f95", "#256abf", "#3987e5", "#6da7ec", "#b7d3f6"],
    6: ["#184f95", "#256abf", "#3987e5", "#6da7ec", "#9ec5f4", "#cde2fb"],
    7: ["#0d366b", "#184f95", "#256abf", "#3987e5", "#6da7ec", "#9ec5f4", "#cde2fb"],
  },
};

/** Diverging scales by class count: blue arm (strong → mild), grey, red arm (mild → strong). */
const DIVERGING: Record<Scheme, Record<number, readonly string[]>> = {
  light: {
    3: ["#2a78d6", "#f0efec", "#c74845"],
    5: ["#1c5cab", "#86b6ef", "#f0efec", "#ea9a93", "#9e3432"],
    7: ["#104281", "#2a78d6", "#9ec5f4", "#f0efec", "#f1aea8", "#c74845", "#762221"],
  },
  dark: {
    3: ["#3987e5", "#383835", "#d75853"],
    5: ["#6da7ec", "#1c5cab", "#383835", "#9e3432", "#e4857e"],
    7: ["#9ec5f4", "#3987e5", "#184f95", "#383835", "#892b2a", "#d75853", "#f1aea8"],
  },
};

const CATEGORICAL: Record<Scheme, readonly string[]> = {
  light: ["#2a78d6", "#eb6834", "#1baf7a"],
  dark: ["#3987e5", "#d95926", "#199e70"],
};
const OTHER: Record<Scheme, string> = { light: "#c3c2b7", dark: "#52514e" };

/**
 * The colour of each class of a layer, lowest first. `other` is whether the
 * last class is a categorical layer's "Other".
 */
export function classColours(scale: MapLayerScaleView, classes: number, scheme: Scheme, other = false): string[] {
  const n = Math.max(1, classes);
  if (scale === "categorical") {
    const named = other ? n - 1 : n;
    const out = CATEGORICAL[scheme].slice(0, Math.min(named, CATEGORICAL[scheme].length));
    if (other) out.push(OTHER[scheme]);
    return out;
  }
  const table = scale === "diverging" ? DIVERGING[scheme] : SEQUENTIAL[scheme];
  const exact = table[n];
  if (exact) return [...exact];
  // A count the tables do not hold (a sequential layer whose breaks merged
  // to one class, or an even diverging count): spread over the widest scale.
  const keys = Object.keys(table).map(Number);
  const widest = table[Math.max(...keys)] ?? [];
  if (n === 1) return [widest[Math.floor(widest.length / 2)] ?? "#888888"];
  return Array.from({ length: n }, (_, i) => widest[Math.round((i * (widest.length - 1)) / (n - 1))] ?? "#888888");
}

export function hexToRgb(hex: string): [number, number, number] {
  const n = parseInt(hex.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}
