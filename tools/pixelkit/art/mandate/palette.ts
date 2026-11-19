// Mandate's master palette (decision D3; PIXEL_ART_TOOL.md §6): our own, at
// most 128 colours, designed in OKLCH and frozen to hex. Anchors:
//   - the darkest neutral is the app's --bg (#11131a); --surface and --border
//     are the asphalt family's darks;
//   - warm lights come from the --accent gold (#e8b44c);
//   - the logo flag (#eb6834) is the signage accent, and the three party
//     colours (#2a78d6, #eb6834, #1baf7a) are ramps for posters and awnings;
//   - skin ramps are anchored on the Monk Skin Tone scale (CC BY 4.0),
//     spanning it from MST 1 to MST 10, shadows leaning red, not grey;
//   - everything else is hue-shifted: shadows cooler, highlights warmer.
// Never pure black or white.
import { buildPalette, type PaletteSpec, type RampSpec } from "../../src/core/palette";

const INK = "#11131a";
/** Outline colours (D2), shared so the outlines of a crowd read as one family: cool, warm, foliage, and a softer one for light skin. */
export const LINES = { cool: "#181b2a", warm: "#26181a", green: "#11251b", skin: "#3e2722" } as const;

/** World ramps: ground, materials, foliage, lights. No outline (tiles and buildings have none, D2) unless props use them. */
const WORLD: RampSpec[] = [
  { name: "ink", anchor: "#2f3446", fixed: [INK, "#1a1d27", "#2f3446"], darker: 0, lighter: 0, line: INK },
  { name: "asphalt", anchor: "#4a4f63", darker: 1, lighter: 1, dL: 0.07, shift: 6, line: LINES.cool },
  { name: "paving", anchor: "#aaa293", darker: 2, lighter: 1, dL: [0.08, 0.1], shift: 10, line: LINES.warm },
  { name: "stone", anchor: "#d5c6a4", darker: 2, lighter: 1, dL: [0.08, 0.1], shift: 12, line: LINES.warm },
  { name: "grass", anchor: "#5e9646", darker: 2, lighter: 1, dL: 0.08, shift: 14, line: LINES.green },
  { name: "foliage", anchor: "#3e7c46", darker: 2, lighter: 1, dL: 0.09, shift: 16, line: LINES.green },
  { name: "wood", anchor: "#7d5537", darker: 1, lighter: 1, dL: 0.1, shift: 12, line: LINES.warm },
  { name: "brick", anchor: "#a64b3a", darker: 2, lighter: 1, dL: [0.08, 0.09], shift: 10, line: LINES.warm },
  { name: "render", anchor: "#e2d2af", darker: 2, lighter: 1, dL: [0.07, 0.08], shift: 14, line: LINES.warm },
  { name: "slate", anchor: "#56607e", darker: 2, lighter: 1, dL: 0.08, shift: 8, line: LINES.cool },
  { name: "terracotta", anchor: "#b9603f", darker: 2, lighter: 1, dL: 0.08, shift: 12, line: LINES.warm },
  { name: "glass", anchor: "#5e7f9c", darker: 1, lighter: 2, dL: 0.1, shift: 10, line: LINES.cool },
  { name: "iron", anchor: "#3d4458", darker: 0, lighter: 1, dL: 0.12, shift: 8, line: LINES.cool },
  { name: "water", anchor: "#3d79b8", darker: 1, lighter: 1, dL: 0.1, shift: 10, line: LINES.cool },
  { name: "earth", anchor: "#8d7150", darker: 0, lighter: 1, dL: 0.08, shift: 10, line: LINES.warm },
  { name: "field", anchor: "#b39b56", darker: 0, lighter: 1, dL: 0.08, shift: 12, line: LINES.warm },
  // Light: the accent gold, warmer and paler toward the core of a lit window.
  { name: "lamp", anchor: "#e8b44c", darker: 1, lighter: 2, dL: [0.1, 0.12], shift: 10, coolHue: 40, warmHue: 95, chromaLight: 0.6, line: LINES.warm },
  { name: "chalk", anchor: "#ece5d6", fixed: ["#cfc8bb", "#ece5d6"], darker: 0, lighter: 0, line: LINES.cool },
  // Signage and party colours (posters, awnings, flags).
  { name: "flag", anchor: "#eb6834", darker: 1, lighter: 1, dL: 0.1, shift: 10, line: LINES.warm },
  { name: "party_blue", anchor: "#2a78d6", darker: 1, lighter: 1, dL: 0.1, shift: 8, line: LINES.cool },
  { name: "party_green", anchor: "#1baf7a", darker: 1, lighter: 1, dL: 0.1, shift: 8, line: LINES.green },
];

/** Monk Skin Tone anchors (MST 1, 4, 5, 6, 8, 10): the full range, light to dark. */
export const SKIN_ANCHORS = ["#f6ede4", "#eadaba", "#d7bd96", "#a07e56", "#604134", "#292420"] as const;

const SKIN: RampSpec[] = SKIN_ANCHORS.map((anchor, i) => ({
  name: `skin${i + 1}`,
  anchor,
  darker: 1,
  lighter: 1,
  // Light skin needs a deeper shade to keep a visible shadow; dark skin a brighter highlight.
  dL: i < 2 ? [0.11, 0.05] : i < 4 ? [0.09, 0.07] : [0.06, 0.1],
  shift: 10,
  coolHue: 25,
  warmHue: 70,
  chromaDark: i < 2 ? 1.9 : 1.15,
  chromaLight: 0.9,
  line: i < 4 ? LINES.skin : LINES.warm,
}));

/** Hair: black, dark brown, brown, auburn, blonde, grey. */
export const HAIR_NAMES = ["black", "dark_brown", "brown", "auburn", "blonde", "grey"] as const;
const HAIR: RampSpec[] = (["#2a2322", "#4b3427", "#73502f", "#94472a", "#cfa65f", "#a9a7a6"] as const).map((anchor, i) => ({
  name: `hair_${HAIR_NAMES[i]}`,
  anchor,
  darker: 1,
  lighter: 1,
  dL: i === 0 ? [0.04, 0.12] : 0.1,
  shift: 14,
  line: LINES.warm,
}));

/** Cloth: what outfits are recoloured from. */
export const CLOTH_NAMES = ["navy", "charcoal", "teal", "sky", "red", "mustard", "green", "plum", "shirt", "denim"] as const;
const CLOTH_ANCHORS: Record<(typeof CLOTH_NAMES)[number], string> = {
  navy: "#2f3d68",
  charcoal: "#4a4f63",
  teal: "#3c8f89",
  sky: "#6596cc",
  red: "#b8473e",
  mustard: "#d0a03c",
  green: "#4f8b4c",
  plum: "#7b4b80",
  shirt: "#cfc8bb",
  denim: "#4d6d9e",
};
const COOL_CLOTH = new Set<string>(["navy", "charcoal", "teal", "sky", "green", "plum", "denim", "shirt"]);
// Charcoal shares the asphalt ramp and shirt white the chalk greys (one palette: materials double as cloth).
const FIXED_CLOTH: Partial<Record<(typeof CLOTH_NAMES)[number], readonly string[]>> = { charcoal: ["#393c4d", "#4a4f63", "#606273"], shirt: ["#aaa293", "#cfc8bb", "#ece5d6"] };
const CLOTH: RampSpec[] = CLOTH_NAMES.map((n) => ({ name: `cloth_${n}`, anchor: CLOTH_ANCHORS[n], ...(FIXED_CLOTH[n] ? { fixed: FIXED_CLOTH[n] } : {}), darker: 1, lighter: 1, dL: n === "navy" || n === "charcoal" ? [0.07, 0.11] : 0.1, shift: 12, line: COOL_CLOTH.has(n) ? LINES.cool : LINES.warm }));

export const PALETTE_SPEC: PaletteSpec = { ramps: [...WORLD, ...SKIN, ...HAIR, ...CLOTH], merge: 0.012, lineMerge: 0.05 };

export const buildMandatePalette = () => buildPalette(PALETTE_SPEC);
