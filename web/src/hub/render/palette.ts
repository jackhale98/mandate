// The placeholder art's palette: a small, fixed set derived from the app's
// design tokens (web/src/styles.css): the dark navy background family, the
// accent gold (DEC-081), the logo flag orange, the info blue, the danger red,
// the muted greys and the light theme's creams. Greens, browns and the road
// grey are the only additions, and they lean towards the navy so the town
// sits in the same world as the rest of the app.
export const PAL = {
  // Tokens
  ink: "#11131a", // --bg (dark): outlines, the darkest shade
  navy: "#1a1d27", // --surface
  navy2: "#232736", // --surface-2
  slate: "#2f3446", // --border
  slate2: "#3a4363", // --sheet-border
  slate3: "#5d6273", // --muted (light)
  grey: "#9aa0b4", // --muted (dark)
  white: "#e9eaf0", // --text (dark)
  cream: "#f5f2ea", // --bg (light)
  cream2: "#efeae0", // --surface-2 (light)
  cream3: "#d9d2c3", // --border (light)
  gold: "#e8b44c", // --accent
  goldDark: "#a8721a", // --accent (light)
  goldInk: "#1b1405", // --accent-ink
  flag: "#eb6834", // --logo-flag
  blue: "#5fa8e5", // --info
  red: "#e5675f", // --danger
  // Derived
  terracotta: "#b5523a",
  terracotta2: "#8e3f2e",
  brick: "#a14a3c",
  brick2: "#7e3a31",
  wood: "#7a5638",
  wood2: "#5a3e2a",
  thatch: "#b39a5c",
  thatch2: "#8c7643",
  teal: "#3f7f86",
  teal2: "#2f5f66",
  water: "#3d79b8",
  water2: "#33669f",
  waterLight: "#7fb6e6",
  grass: "#5d8f4c",
  grass2: "#4f7f42",
  grass3: "#74a55b",
  park: "#4c8445",
  tree: "#3b6c3b",
  tree2: "#2b5231",
  treeLight: "#5f9a4c",
  field: "#a88f4e",
  field2: "#8f7840",
  dirt: "#8d7150",
  dirt2: "#77603f",
  sand: "#d9c79a",
  road: "#444a5c",
  road2: "#3a3f50",
  pavement: "#a7a59e",
  pavement2: "#929089",
  plaza: "#c9c1ae",
  plaza2: "#b5ac97",
  yard: "#8e9092",
  yard2: "#7f8183",
  glass: "#6f8fae",
  glass2: "#4f6d8c",
  lit: "#ffd77a",
  litCore: "#fff1c4",
} as const;

export type PaletteKey = keyof typeof PAL;
