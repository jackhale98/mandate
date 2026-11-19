// Colours of the town map overview (DEC-086), taken from the pixel-art
// pack's master palette (tools/pixelkit/art/mandate/palette.ts, DEC-084 D3:
// the ramp anchors, frozen to hex here so the app does not depend on the
// tool) and the app's brand tokens (DEC-081). Map-scale glyphs use the same
// materials as the street art: asphalt, paving, stone, render, brick,
// terracotta, slate, glass, iron, foliage, water, the lamp gold, the flag
// orange and the three party colours.
export type Rgb = readonly [number, number, number];

const hex = (h: string): Rgb => [parseInt(h.slice(1, 3), 16), parseInt(h.slice(3, 5), 16), parseInt(h.slice(5, 7), 16)];

export const MAP = {
  ink: "#11131a",
  line: "#181b2a",
  // Ground
  grass: "#5e9646",
  grassDark: "#4f8440",
  grassLight: "#72a855",
  garden: "#6aa24f",
  park: "#4f8a45",
  foliage: "#3e7c46",
  foliageDark: "#2f6439",
  foliageLight: "#5a9a52",
  field: "#b39b56",
  fieldDark: "#9d874a",
  earth: "#8d7150",
  earthLight: "#a3865f",
  sand: "#d9c79a",
  asphalt: "#4a4f63",
  asphaltDark: "#3f4456",
  asphaltLine: "#8d8a7e",
  paving: "#aaa293",
  pavingDark: "#968e80",
  stone: "#d5c6a4",
  stoneDark: "#bdae8e",
  quay: "#bfb398",
  wood: "#7d5537",
  woodDark: "#62422b",
  water: "#3d79b8",
  waterDark: "#33669f",
  waterLight: "#6aa0d6",
  // Materials
  render: "#e2d2af",
  renderDark: "#c7b590",
  brick: "#a64b3a",
  brickDark: "#86392d",
  terracotta: "#b9603f",
  terracottaDark: "#974c31",
  slate: "#56607e",
  slateDark: "#444c66",
  slateLight: "#6d789a",
  glass: "#5e7f9c",
  glassLight: "#8fb0cc",
  iron: "#3d4458",
  ironLight: "#5a6178",
  verdigris: "#4f9a8c",
  verdigrisDark: "#3c7d71",
  chalk: "#ece5d6",
  chalkDark: "#cfc8bb",
  // Light and signs
  lamp: "#e8b44c",
  lampLight: "#ffd77a",
  lampCore: "#fff1c4",
  flag: "#eb6834",
  partyBlue: "#2a78d6",
  partyGreen: "#1baf7a",
  cross: "#d64a3e",
  // Quiet filler roofs, by material
  roofHouse: "#a8674c",
  roofHouseDark: "#8a5440",
  roofTerrace: "#6a7088",
  roofShop: "#8f8577",
  roofShed: "#7d7468",
  roofWarehouse: "#727a86",
  wallQuiet: "#cdbf9f",
  wallQuietDark: "#ad9f82",
} as const;

export type MapColour = keyof typeof MAP;

/** Neighbourhood tints by kind (soft, laid over the ground): the town's districts read apart without shouting. */
export const DISTRICT_TINT: Record<string, Rgb> = {
  core: hex("#e8b44c"),
  inner: hex("#b9603f"),
  waterfront: hex("#3d79b8"),
  industrial: hex("#56607e"),
  suburb: hex("#a5c46a"),
  rural: hex("#b39b56"),
};

export const rgb = hex;
