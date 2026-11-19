// Interior parts (DEC-088, P1-7 "Interiors"; art round 7): floors, walls,
// windows, doors, furniture and fixtures for the council chamber, the town
// hall's front office and the diner, drawn by hand as text grids (D5; no
// commissioned artist, DEC-084 note). Same scale and view as the street:
// 16 px floor tiles, people 16×24, three-quarter top-down (the floor from
// above, the back wall's face, no side-wall faces). Furniture is a prop: it
// gets the automatic outline (D2). Floors, walls and things hung on the wall
// are building faces: no outline, edges come from value.
//
// Parts are drawn in *roles*, not colours. A room picks a ramp for each
// variable role from its recipe (keyed by the seed), so one drawing of a
// desk serves oak, walnut and mahogany, and one booth red, teal or mustard
// vinyl. Fixed roles always use the same ramp.
//
// Legend (role, step from the ramp's anchor):
//   W w k    wood -1 0 +1 (variable)          U u v    upholstery -1 0 +1 (variable)
//   A a r    accent -1 0 +1 (variable)        E e f    second accent -1 0 +1 (variable)
//   Q q z    wall -1 0 +1 (the room's wall)    D d K    trim -1 0 +1 (the room's trim)
//   M m n N  steel/chrome (glass) -1 0 +1 +2  i I      iron 0 +1
//   b B h    brass (lamp) -1 0 +1             L l      lamp light +1 +2 (glows when `lit`)
//   C c      paper (chalk) -1 0               T S s t  stone -2 -1 0 +1
//   J G g j  leaves (foliage) -2 -1 0 +1      P p      pot (terracotta) -1 0
//   o O      ink -1 0                          F x      ketchup red (flag) -1 0
//   R Y      cork / mustard (cloth_mustard) -1 0
//   1 2 3 4 5  the view through a window: high sky, low sky, far roofs, far
//            walls, far windows (lit at night). Graded as outdoors.
import { blank, blit, crop, gridFrames, gridToIndexed, mirrorX, type Indexed } from "../../src/core/image";

export type VarRole = "wood" | "uph" | "acc" | "acc2" | "wall" | "trim";
export const VAR_ROLES: readonly VarRole[] = ["wood", "uph", "acc", "acc2", "wall", "trim"];

/** Fixed roles: always this ramp. */
export const FIXED_RAMPS: Readonly<Record<string, string>> = {
  metal: "glass",
  iron: "iron",
  brass: "lamp",
  paper: "chalk",
  stone: "stone",
  leaf: "foliage",
  pot: "terracotta",
  ink: "ink",
  red: "flag",
  cork: "cloth_mustard",
  sky: "glass",
};

/** Grid character → [role, step]. */
export const ILEGEND: Readonly<Record<string, readonly [string, number]>> = {
  W: ["wood", -1],
  w: ["wood", 0],
  k: ["wood", 1],
  U: ["uph", -1],
  u: ["uph", 0],
  v: ["uph", 1],
  A: ["acc", -1],
  a: ["acc", 0],
  r: ["acc", 1],
  E: ["acc2", -1],
  e: ["acc2", 0],
  f: ["acc2", 1],
  Q: ["wall", -1],
  q: ["wall", 0],
  z: ["wall", 1],
  D: ["trim", -1],
  d: ["trim", 0],
  K: ["trim", 1],
  M: ["metal", -1],
  m: ["metal", 0],
  n: ["metal", 1],
  N: ["metal", 2],
  i: ["iron", 0],
  I: ["iron", 1],
  b: ["brass", -1],
  B: ["brass", 0],
  h: ["brass", 1],
  L: ["brass", 1],
  l: ["brass", 2],
  C: ["paper", -1],
  c: ["paper", 0],
  T: ["stone", -2],
  S: ["stone", -1],
  s: ["stone", 0],
  t: ["stone", 1],
  J: ["leaf", -2],
  G: ["leaf", -1],
  g: ["leaf", 0],
  j: ["leaf", 1],
  P: ["pot", -1],
  p: ["pot", 0],
  o: ["ink", -1],
  O: ["ink", 0],
  F: ["red", -1],
  x: ["red", 0],
  R: ["cork", -1],
  Y: ["cork", 0],
  // The view through a window (slot codes, coloured per time of day as the street is).
  "1": ["sky", 2],
  "2": ["sky", 1],
  "3": ["sky", -1],
  "4": ["sky", 0],
  "5": ["sky", 0],
};
/** Characters whose pixels are outdoors (graded like the street, not the room). */
export const OUTDOOR_CHARS = "12345";
/** Far windows: lit at night (the town behind the glass). */
export const FAR_LIT_CHARS = "5";

/** Codes: one per legend character, 1-based, in a fixed order (the slot image's values). */
export const ICODES: Readonly<Record<string, number>> = Object.fromEntries(Object.keys(ILEGEND).map((ch, i) => [ch, i + 1]));
export const ICHARS: readonly string[] = ["", ...Object.keys(ILEGEND)];

// ---------------------------------------------------------------------------
// Surfaces: 16×16 seamless tiles. Letters a b c d are the surface's ramp at
// steps -2 -1 0 +1; e f g h the same steps of its second ramp.

export const SLEGEND: Readonly<Record<string, readonly [number, number]>> = {
  a: [0, -2],
  b: [0, -1],
  c: [0, 0],
  d: [0, 1],
  e: [1, -2],
  f: [1, -1],
  g: [1, 0],
  h: [1, 1],
};
export const SCODES: Readonly<Record<string, number>> = { a: 1, b: 2, c: 3, d: 4, e: 5, f: 6, g: 7, h: 8 };

export interface SurfaceDef {
  id: string;
  /** Floor or wall. */
  on: "floor" | "wall";
  grid: string;
  desc: string;
}

export const SURFACES: readonly SurfaceDef[] = [
  {
    id: "planks",
    on: "floor",
    desc: "floorboards: four boards a tile, staggered butt joints, a grain streak each",
    grid: `
      cccccccccccbdddc
      cccdddcccccbcccc
      cccccccccccbcccc
      bbbbbbbbbbbbbbbb
      ccbddcccccccccdc
      ccbccccccccdddcc
      ccbccccccccccccc
      bbbbbbbbbbbbbbbb
      ccccccdddcccccbd
      cdddccccccccccbc
      ccccccccccccccbc
      bbbbbbbbbbbbbbbb
      ccccccbdcccccccc
      ccccccbccccdddcc
      ccccccbccccccccc
      bbbbbbbbbbbbbbbb`,
  },
  {
    id: "parquet",
    on: "floor",
    desc: "basket-weave parquet: blocks of three slats, turned each 8 px",
    grid: `
      dccccccbccbccbcc
      cccccccbccbccbcc
      bbbbbbbbccbccbcc
      dccccccbccbccbcc
      cccccccbccbccbcc
      bbbbbbbbccbccbcc
      dcccccccdcbccbcc
      bbbbbbbbbbbbbbbb
      ccbccbccdccccccb
      ccbccbcccccccccb
      ccbccbccbbbbbbbb
      ccbccbccdccccccb
      ccbccbcccccccccb
      ccbccbccbbbbbbbb
      ccbccbccdccccccb
      bbbbbbbbbbbbbbbb`,
  },
  {
    id: "checker",
    on: "floor",
    desc: "linoleum checkerboard: 8 px squares of the two ramps, a sheen on the light ones",
    grid: `
      ccccccccgggggggg
      cddcccccgggggggg
      cdcccccccggggggg
      cccccccggggggggg
      cccccccggggggggg
      cccccccggggggggg
      cccccccggggggggg
      ccccccccgggggggg
      ggggggggcccccccc
      ggggggggcddccccc
      gggggggggdcccccc
      gggggggcccccccbc
      gggggggccccccccc
      gggggggccccccccc
      gggggggccccccccc
      ggggggggcccccccc`,
  },
  {
    id: "carpet",
    on: "floor",
    desc: "carpet: a small lozenge repeat in the pile's dark and light",
    grid: `
      cccccccccccccccc
      cccbcccccccccccc
      ccbdbccccccccccc
      cccbcccccccccccc
      cccccccccccccccc
      cccccccccccccccc
      cccccccccccccccc
      cccccccccccccccc
      cccccccccccccccc
      cccccccccccbcccc
      ccccccccccbdbccc
      cccccccccccbcccc
      cccccccccccccccc
      cccccccccccccccc
      cccccccccccccccc
      cccccccccccccccc`,
  },
  {
    id: "stone_tiles",
    on: "floor",
    desc: "stone flags in a half-bond, grout in the dark step, a worn sheen",
    grid: `
      ccccccccccccccbc
      cddddcccccccccbc
      cdccccccccccccbc
      ccccccccccccccbc
      ccccccccccccccbc
      ccccccccccccccbc
      ccccccccccccccbc
      bbbbbbbbbbbbbbbb
      ccccccbccccccccc
      ccccccbcddddcccc
      ccccccbcdccccccc
      ccccccbccccccccc
      ccccccbccccccccc
      ccccccbccccccccc
      ccccccbccccccccc
      bbbbbbbbbbbbbbbb`,
  },
  {
    id: "terrazzo",
    on: "floor",
    desc: "terrazzo: chips of the second ramp in clusters, brass strips every 16 px",
    grid: `
      ccccccccccccccce
      ccgcccccccccccce
      cggcccccchhccccc
      ccccccccchccccce
      cccccccccccccgce
      cccchcccccccggce
      ccchhcccccccccce
      ccccccccgccccccc
      ccccccccggccccce
      cgcccccccccchcce
      cggcccccccchhcce
      ccccccchccccccce
      cccccchhccccccce
      ccccccccccccgcce
      ccccccccccccggce
      eeeeeeeeeeeeeeee`,
  },
  {
    id: "lino",
    on: "floor",
    desc: "sheet linoleum: speckle in pairs, a welded seam every two tiles",
    grid: `
      ccccccccccccccdc
      cbbcccccccccccdc
      ccccccccccccccdc
      cccccccdcccccccc
      ccccccddcccccccc
      ccccccccccccbccc
      cccccccccccbbccc
      ccdccccccccccccc
      cddccccccccccccc
      ccccccccbbcccccc
      cccccccccccccccc
      ccccccccccccddcc
      cccbcccccccccdcc
      ccbbcccccccccccc
      cccccccccccccccc
      cccccccccccccccc`,
  },
  {
    id: "plaster",
    on: "wall",
    desc: "painted plaster: flat, with a few soft trowel marks",
    grid: `
      cccccccccccccccc
      cccccccccccccccc
      cccccdcccccccccc
      ccccddcccccccccc
      cccccccccccccccc
      cccccccccccccccc
      cccccccccccccccc
      ccccccccccccbccc
      cccccccccccbbccc
      cccccccccccccccc
      cccccccccccccccc
      cccccccccccccccc
      cdcccccccccccccc
      cddccccccccccccc
      cccccccccccccccc
      cccccccccccccccc`,
  },
  {
    id: "panelling",
    on: "wall",
    desc: "raised wood panels: a light top-left bevel, a dark bottom-right one",
    grid: `
      bccccccccccccccb
      bcddddddddddddcb
      bcdccccccccccbcb
      bcdccccccccccbcb
      bcdccccccccccbcb
      bcdccccccccccbcb
      bcdccccccccccbcb
      bcdccccccccccbcb
      bcdccccccccccbcb
      bcdccccccccccbcb
      bcdccccccccccbcb
      bcdccccccccccbcb
      bcdccccccccccbcb
      bcdbbbbbbbbbbbcb
      bccccccccccccccb
      bccccccccccccccb`,
  },
  {
    id: "brick",
    on: "wall",
    desc: "exposed brick: 8×4 bricks in a half bond, mortar in the second ramp",
    grid: `
      ccccccchcccccccg
      cdddcccbcdddcccb
      bbbbbbbbbbbbbbbb
      gggggggggggggggg
      cccghcccccccghcc
      cccbcdddcccbcddd
      bbbbbbbbbbbbbbbb
      gggggggggggggggg
      ccccccchcccccccg
      cdddcccbcdddcccb
      bbbbbbbbbbbbbbbb
      gggggggggggggggg
      cccghcccccccghcc
      cccbcdddcccbcddd
      bbbbbbbbbbbbbbbb
      gggggggggggggggg`,
  },
  {
    id: "subway",
    on: "wall",
    desc: "glazed tiles 8×4, grout in the dark step, a glint on each",
    grid: `
      cddccccbcddccccb
      ccccccccccccccbb
      ccccccccccccccbb
      bbbbbbbbbbbbbbbb
      cccbcddccccbcddc
      cccbbcccccccbccc
      cccbbcccccccbccc
      bbbbbbbbbbbbbbbb
      cddccccbcddccccb
      ccccccccccccccbb
      ccccccccccccccbb
      bbbbbbbbbbbbbbbb
      cccbcddccccbcddc
      cccbbcccccccbccc
      cccbbcccccccbccc
      bbbbbbbbbbbbbbbb`,
  },
  {
    id: "stripes",
    on: "wall",
    desc: "striped wallpaper: broad bands of the paper and a fine line of the second ramp",
    grid: `
      ccccccddddddfccf
      ccccccddddddfccf
      ccccccddddddfccf
      ccccccddddddfccf
      ccccccddddddfccf
      ccccccddddddfccf
      ccccccddddddfccf
      ccccccddddddfccf
      ccccccddddddfccf
      ccccccddddddfccf
      ccccccddddddfccf
      ccccccddddddfccf
      ccccccddddddfccf
      ccccccddddddfccf
      ccccccddddddfccf
      ccccccddddddfccf`,
  },
];

// ---------------------------------------------------------------------------
// Parts

export type PartKind = "furniture" | "seat" | "wall" | "top" | "decal";

export interface PartDef {
  id: string;
  kind: PartKind;
  desc: string;
  /** One drawing (or the frames of a strip). */
  grid?: string;
  /**
   * A strip: furniture that runs `len` tiles. "h": start, middle (repeated),
   * end, each 16 px wide; "v": start, middle (16 px, repeated len - 1 times)
   * and end, whose heights sum to 16.
   */
  strip?: { dir: "h" | "v"; start: string; mid: string; end: string };
  /** Footprint in tiles across the strip (h: rows; v: columns), or of a single drawing. */
  foot?: readonly [number, number];
  /** Characters that glow when the room's lamps are on (or always, for screens and signs: `glow`). */
  lit?: string;
  glow?: "always";
  /** Seats: the row a sitter's hips go to, and the first row drawn over the sitter (the seat's front or a backrest). */
  sit?: number;
  front?: number;
  /** Top parts: px above the supporting tile's top where the part's bottom row rests (default 1). */
  rest?: number;
}

export const PARTS: readonly PartDef[] = [
  // ---- windows (on the back wall; no outline). The panes show the street outside.
  {
    id: "window_arch",
    kind: "wall",
    desc: "tall arched window: stone surround, sash bars, a deep sill (chamber)",
    grid: `
      .....KKKKK.....
      ...KKddddddK...
      ..KdD11111DdK..
      .KdD1111111DdK.
      .KdD1111111DdK.
      KdD111D111111DK
      KdD111D111111dK
      KdDDDDDDDDDDDdK
      Kd11111D11111dK
      Kd11111D11111dK
      Kd11111D11111dK
      Kd11111D11111dK
      Kd22222D22222dK
      Kd22222D22222dK
      KdDDDDDDDDDDDdK
      Kd22222D22222dK
      Kd22222D22222dK
      Kd22222D22222dK
      Kd22222D22222dK
      KdDDDDDDDDDDDdK
      Kd22222D22222dK
      Kd22222D22222dK
      Kd33333D33532dK
      Kd34343D34332dK
      Kd44544D44444dK
      KdDDDDDDDDDDDdK
      KKKKKKKKKKKKKKK
      dddddddddddddDD
      DDDDDDDDDDDDDDD`,
  },
  {
    id: "window_sash",
    kind: "wall",
    desc: "sash window with a painted frame and a sill (town hall)",
    grid: `
      KKKKKKKKKKKKKK
      KDDDDDDDDDDDDK
      KD1111D11111DK
      KD1111D11111DK
      KD1111D11111DK
      KD2222D22222DK
      KD2222D22222DK
      KDDDDDDDDDDDDK
      KD2222D22222DK
      KD2222D22222DK
      KD3332D23353DK
      KD4344D34443DK
      KD4544D44444DK
      KDDDDDDDDDDDDK
      dddddddddddddd
      DDDDDDDDDDDDDD`,
  },
  {
    id: "window_wide",
    kind: "wall",
    desc: "wide diner window with a half-drawn venetian blind",
    grid: `
      KKKKKKKKKKKKKKKKKKKKKKKKKKKKKK
      KDccccccccccccccccccccccccccDK
      KDCCCCCCCCCCCCCCCCCCCCCCCCCCDK
      KDccccccccccccccccccccccccccDK
      KDCCCCCCCCCCCCCCCCCCCCCCCCCCDK
      KDccccccccccccccccccccccccccDK
      KDCCCCCCCCCCCCCCCCCCCCCCCCCCDK
      KD1111111111111D111111111111DK
      KD1111111111111D111111111111DK
      KD2222222222222D222222222222DK
      KD2222222222222D222222222222DK
      KD3332222223332D222223333222DK
      KD3433322234343D333334343322DK
      KD4444333444544D543444444433DK
      KD4544444445444D444444544444DK
      KDDDDDDDDDDDDDDDDDDDDDDDDDDDDK
      dddddddddddddddddddddddddddddd
      DDDDDDDDDDDDDDDDDDDDDDDDDDDDDD`,
  },
  // ---- doors in the back wall (no outline): they stand on the floor line.
  {
    id: "door_double",
    kind: "wall",
    desc: "double doors in a stone architrave with a fanlight (chamber)",
    grid: `
      KKKKKKKKKKKKKKKKKKKKKKKKKKKKKK
      KddddddddddddddddddddddddddddK
      KdD2222222222D2222222222222DdK
      KdD2222222222D2222222222222DdK
      KdDDDDDDDDDDDDDDDDDDDDDDDDDDdK
      KdWkkkkkkkkkkkWkkkkkkkkkkkkWdK
      KdWkWWWWWWWWwkWkWWWWWWWWWwkWdK
      KdWkWwwwwwwwkkWkWwwwwwwwwkkWdK
      KdWkWwwwwwwwkkWkWwwwwwwwwkkWdK
      KdWkWwwwwwwwkkWkWwwwwwwwwkkWdK
      KdWkWwwwwwwwkkWkWwwwwwwwwkkWdK
      KdWkWkkkkkkkkkWkWkkkkkkkkkkWdK
      KdWkwwwwwwwwwwWkwwwwwwwwwwwWdK
      KdWkwwwwwwwwwBWBwwwwwwwwwwwWdK
      KdWkwwwwwwwwwhWhwwwwwwwwwwwWdK
      KdWkwwwwwwwwwwWkwwwwwwwwwwwWdK
      KdWkWWWWWWWWwkWkWWWWWWWWWwkWdK
      KdWkWwwwwwwwkkWkWwwwwwwwwkkWdK
      KdWkWwwwwwwwkkWkWwwwwwwwwkkWdK
      KdWkWwwwwwwwkkWkWwwwwwwwwkkWdK
      KdWkWwwwwwwwkkWkWwwwwwwwwkkWdK
      KdWkWwwwwwwwkkWkWwwwwwwwwkkWdK
      KdWkWwwwwwwwkkWkWwwwwwwwwkkWdK
      KdWkWkkkkkkkkkWkWkkkkkkkkkkWdK
      KdWkwwwwwwwwwwWkwwwwwwwwwwwWdK
      KdWbbbbbbbbbbbWbbbbbbbbbbbbWdK
      KdWWWWWWWWWWWWWWWWWWWWWWWWWWdK
      KdddddddddddddddddddddddddddDK`,
  },
  {
    id: "door_office",
    kind: "wall",
    desc: "office door: a reeded-glass panel with a nameplate, brass knob",
    grid: `
      KKKKKKKKKKKKKKKK
      KdddddddddddddDK
      KdWkkkkkkkkkkWDK
      KdWkWWWWWWWWkWDK
      KdWkWnNnNnNnkWDK
      KdWkWnNnNnNnkWDK
      KdWkWnNBBBnnkWDK
      KdWkWnNnNnNnkWDK
      KdWkWnNnNnNnkWDK
      KdWkWnNnNnNnkWDK
      KdWkWnNnNnNnkWDK
      KdWkWkkkkkkkkWDK
      KdWkwwwwwwwwwWDK
      KdWkwwwwwwwwwWDK
      KdWkwwwwwwwwwWDK
      KdWkwwwwwwwBhWDK
      KdWkwwwwwwwbwWDK
      KdWkwwwwwwwwwWDK
      KdWkWWWWWWWwkWDK
      KdWkWwwwwwwkkWDK
      KdWkWwwwwwwkkWDK
      KdWkWwwwwwwkkWDK
      KdWkWwwwwwwkkWDK
      KdWkWkkkkkkkkWDK
      KdWkwwwwwwwwwWDK
      KdWWWWWWWWWWWWDK
      KddddddddddddddK`,
  },
  {
    id: "door_swing",
    kind: "wall",
    desc: "kitchen swing door: steel kick plate and a porthole",
    grid: `
      KKKKKKKKKKKKKKKK
      KDDDDDDDDDDDDDDK
      KDMnnnnnnnnnnMDK
      KDMn...nnnnnnMDK
      KDMn.MMMMnnnnMDK
      KDMnMOOOOMnnnMDK
      KDMnMO1OOMnnnMDK
      KDMnMOO1OMnnnMDK
      KDMnMOOOOMnnnMDK
      KDMnnMMMMnnnnMDK
      KDMnnnnnnnnnnMDK
      KDMnnnnnnnnnnMDK
      KDMnnnnnnnnnnMDK
      KDMnnnnnnnnnnMDK
      KDMnnnnnnnnnnMDK
      KDMnnnnnnnnnnMDK
      KDMnnnnnnnnnnMDK
      KDMnnnnnnnnnnMDK
      KDMnnnnnnnnnnMDK
      KDMNNNNNNNNNNMDK
      KDMmmmmmmmmmmMDK
      KDMmmmmmmmmmmMDK
      KDMmmmmmmmmmmMDK
      KDMMMMMMMMMMMMDK
      KDDDDDDDDDDDDDDK`,
  },
  // ---- wall fixtures (no outline)
  {
    id: "crest",
    kind: "wall",
    desc: "the town's crest: a shield in the civic colours on a brass cartouche",
    grid: `
      ....bbbbbbbbbbbbbb....
      ..bbhhhhhhhhhhhhhhbb..
      .bhhbbbbbbbbbbbbbbhhb.
      .bhbaaaaaaaaaaaaaabhb.
      bhbaaaaaaaaaaaaaaaabhb
      bhbarrrrraaaarrrrrabhb
      bhbaaaaaraBBaraaaaabhb
      bhbaeeeaaaBBaaaeeeabhb
      bhbaeeeeaBhhBaeeeeabhb
      bhbaeeeeaBhhBaeeeeabhb
      bhbaaeeeaaBBaaeeeaabhb
      .bhbaaeeeaaaaeeeaabhb.
      .bhbaaaeeeaaeeeaaabhb.
      ..bhbaaaeeeeeeaaabhb..
      ..bhbbaaaeeeeaaabbhb..
      ...bhbbaaaaaaaabbhb...
      ....bhhbbaaaabbhhb....
      .....bbhhbbbbhhbb.....
      .......bbhhhhbb.......
      .........bbbb.........`,
  },
  {
    id: "portrait",
    kind: "wall",
    desc: "a former mayor in a gilt frame",
    grid: `
      bBBBBBBBBBBBBb
      BhhhhhhhhhhhhB
      BhOOOOOOOOOObB
      BhOOOOSSOOOObB
      BhOOOStsSOOObB
      BhOOOSssSOOObB
      BhOOOOSSOOOObB
      BhOOOOSSOOOObB
      BhOOiiccciiObB
      BhOiiiicIiiibB
      BhOiiiiiiiiibB
      BhiiiiiiiiiibB
      BbbbbbbbbbbbbB
      bBBBBBBBBBBBBb`,
  },
  {
    id: "clock",
    kind: "wall",
    desc: "a round wall clock (no numerals: ticks)",
    grid: `
      ....WWWW....
      ..WWccOcWW..
      .WccccOcccW.
      .WccccOcccW.
      WcccccOccccW
      WOccccOOOcOW
      WccccccccccW
      WccccccccccW
      .WccccccccW.
      .WccccOcccW.
      ..WWccccWW..
      ....WWWW....`,
  },
  {
    id: "notice_board",
    kind: "wall",
    desc: "cork notice board in a wood frame: pinned notices, a map, a coloured pin each",
    grid: `
      WWWWWWWWWWWWWWWWWWWWWWWWWWWW
      WkkkkkkkkkkkkkkkkkkkkkkkkkkW
      WkYYYYYYYYYYYYYYYYYYYYYYYYRW
      WkYcccccxcYYYYYYYYYYYccacYRW
      WkYcCCCCcYYYeeeeeeeeYcCCcYRW
      WkYcccccCYYYeffeeffeYccccYRW
      WkYcCCCCcYYYefeeffeeYcCCcYRW
      WkYcccccCYYYeeeeeefeYccccYRW
      WkYcCCCcCYYYeeffeeeeYCCCCYRW
      WkYCCCCCCYYYeeeeeeeeYYYYYYRW
      WkYYYYYYYYYYYYYYYYYYYYYYYYRW
      WkYYYcccacYYYYccccxccYYYYYRW
      WkYYYcCCccYYYYcCCCCCcYYxYYRW
      WkYYYcccCCYYYYcccccccYcccYRW
      WkYYYCCCCCYYYYCCCCCCCYcCcYRW
      WkRRRRRRRRRRRRRRRRRRRRCCCRRW
      WWWWWWWWWWWWWWWWWWWWWWWWWWWW`,
  },
  {
    id: "poster",
    kind: "wall",
    desc: "a framed civic notice: the ballot-box pictogram over lines of print",
    grid: `
      DDDDDDDDDD
      DccccccccD
      DcceeeeccD
      DcceffeccD
      DceeeeeecD
      DceEEEEecD
      DceeeeeecD
      DccccccccD
      DcCCCCCCcD
      DccccccccD
      DcCCCCCccD
      DccccccccD
      DDDDDDDDDD`,
  },
  {
    id: "sconce",
    kind: "wall",
    lit: "Ll",
    desc: "a brass wall light with a fabric shade",
    grid: `
      .LLLL.
      LllllL
      LllllL
      .LLLL.
      ..bb..
      ..Bb..
      .bBBb.
      ..bb..`,
  },
  {
    id: "neon",
    glow: "always",
    kind: "wall",
    lit: "ar",
    desc: "a neon coffee-cup sign on a dark backing (diner)",
    grid: `
      OOOOOOOOOOOOOOOOOO
      OOOOOrOOrOOrOOOOOO
      OOOOOOaOOaOOaOOOOO
      OOOOOrOOrOOrOOOOOO
      OOarrrrrrrrrraOOOO
      OOaOOOOOOOOOOaaaOO
      OOaOOOOOOOOOOaOaOO
      OOaOOOOOOOOOOaaaOO
      OOOaOOOOOOOOaOOOOO
      OOOOaaaaaaaaOOOOOO
      OaaaaaaaaaaaaaaaOO
      OOOOOOOOOOOOOOOOOO`,
  },
  {
    id: "menu_board",
    kind: "wall",
    desc: "the menu board: pictograms (cup, burger, pie, egg) and rows of price dots, no text",
    grid: `
      WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW
      WOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOW
      WOOccccOOcOcOcOOOOOOOOYYYYOOOOcOcOcOOW
      WOOcCCcccOOOOOOOOOOOOYxxxxYOOOOOOOOOOW
      WOOcCCcOccOcOcOOOOOOOggggggOOOcOcOcOOW
      WOOOccOOOOOOOOOOOOOOOYYYYYYOOOOOOOOOOW
      WOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOW
      WOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOW
      WOOOYYYOOOcOcOcOOOOOOOcccOOOOOcOcOcOOW
      WOOYxYxYOOOOOOOOOOOOOccYccOOOOOOOOOOOW
      WOOYYYYYOOcOcOcOOOOOOOcccOOOOOcOcOcOOW
      WOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOOW
      WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW`,
  },
  {
    id: "kitchen_pass",
    glow: "always",
    kind: "wall",
    lit: "Ll",
    desc: "the kitchen pass: a steel hatch, heat lamps glowing over the shelf, plates and an order rail",
    grid: `
      MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM
      MnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnM
      MnccOcccOcccOOOOOOOOOOOOOOOOOOOOOOcccOcccM
      MnCcOCcCOcCcOOOOOOiOOOOOOOOOiOOOOOcCcOCcCM
      MnOOOOOOOOOOOOOOOOiOOOOOOOOOiOOOOOOOOOOOOM
      MnOOOOOOOOOOOOOOOLLLOOOOOOOLLLOOOOOOOOOOOM
      MnOOOOOOOOOOOOOOLlllLOOOOOLlllLOOOOOOOOOOM
      MnOOoooooooooooooooooooooooooooooooooooOOM
      MnOOoooooooooooooooooooooooooooooooooooOOM
      MnOooommmooooooooooooooooooooooooooooooOOM
      MnOoommmmmoooooooooooooooommmoooooooooooOM
      MnNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNM
      MmcccccccmmmmmmmmmcccccccmmmmmmmmmmhBhmmmM
      MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM`,
  },
  {
    id: "tv",
    glow: "always",
    kind: "wall",
    lit: "NnaeEm",
    desc: "a television on a wall bracket: the news (an anchor before a flag)",
    grid: `
      OOOOOOOOOOOOOOOOOO
      OonnnnnnnnnnnnnnoO
      OonaaaaaannnnnnnoO
      OonaeeeeannnnnnnoO
      OonaaaaaannmmnnnoO
      OonnnnnnnnmEEmnnoO
      OonnnnnnnnmEEmnnoO
      OonnnnnnnEEEEEEnoO
      OonnnnnnnEEEEEEnoO
      OoNNNNNNNNNNNNNNoO
      OOOOOOOOOOOOOOOOOO
      ........OO........
      .......OOOO.......`,
  },
  {
    id: "leaflet_rack",
    kind: "wall",
    desc: "a wire rack of leaflets in three colours (budget papers)",
    grid: `
      iIIIIIIIIIIi
      iccceeexxxxi
      iCcceefxxxxi
      icCceeexFxxi
      iIIIIIIIIIIi
      iaaacccYYYYi
      iarachcYRYYi
      iaaaccCYYYYi
      iIIIIIIIIIIi`,
  },
  // ---- the chamber
  {
    id: "chair_high",
    kind: "seat",
    sit: 17,
    front: 17,
    desc: "the chair's high-backed seat: carved crest rail, tufted upholstery, brass finials",
    grid: `
      ......bBBb......
      ....bBkkkkBb....
      ...WkkwwwwkkW...
      ..WkwUUUUUUwkW..
      ..WkUuuvvuuUkW..
      ..WkUuvuuvuUkW..
      ..WkUuuuuuuUkW..
      ..WkUuvuuvuUkW..
      ..WkUuuuuuuUkW..
      ..WkUuuuuuuUkW..
      ..WkUuvuuvuUkW..
      ..WkUuuuuuuUkW..
      ..WkUuuuuuuUkW..
      ..WkUuuuuuuUkW..
      ..WkUUUUUUUUkW..
      .kWkwwwwwwwwkWk.
      .wWuuuuuuuuuuWw.
      .wWvvvvvvvvvvWw.
      .WWUUUUUUUUUUWW.
      .W.wwwwwwwwww.W.
      .W.WWWWWWWWWW.W.
      .W.W........W.W.
      .W.W........W.W.`,
  },
  {
    id: "desk",
    kind: "furniture",
    foot: [1, 1],
    desc: "a members' desk run, leather-topped, panelled front (straight runs and the chair's bench)",
    strip: {
      dir: "h",
      start: `
        .kkkkkkkkkkkkkkk
        kwvvvvvvvvvvvvvv
        kwuuuuuuuuuuuuuu
        kwuuuuuuuuuuuuuu
        kwuuuuuuuuuuuuuu
        kwwwwwwwwwwwwwww
        WWWWWWWWWWWWWWWW
        wkkkkkkkkkkkkkkk
        wkwwwwwwwwwwwwww
        wkwWWWWWWWWWWwww
        wkwWwwwwwwwwkwww
        wkwWwwwwwwwwkwww
        wkwWwwwwwwwwkwww
        wkwWkkkkkkkkkwww
        wkwwwwwwwwwwwwww
        WWWWWWWWWWWWWWWW`,
      mid: `
        kkkkkkkkkkkkkkkk
        vvvvvvvvvvvvvvvv
        uuuuuuuuuuuuuuuu
        uuuuuuuuuuuuuuuu
        uuuuuuuuuuuuuuuu
        wwwwwwwwwwwwwwww
        WWWWWWWWWWWWWWWW
        kkkkkkkkkkkkkkkk
        wwwwwwwwwwwwwwww
        wwwWWWWWWWWWWwww
        wwwWwwwwwwwwkwww
        wwwWwwwwwwwwkwww
        wwwWwwwwwwwwkwww
        wwwWkkkkkkkkkwww
        wwwwwwwwwwwwwwww
        WWWWWWWWWWWWWWWW`,
      end: `
        kkkkkkkkkkkkkkk.
        vvvvvvvvvvvvvvwW
        uuuuuuuuuuuuuuwW
        uuuuuuuuuuuuuuwW
        uuuuuuuuuuuuuuwW
        wwwwwwwwwwwwwwwW
        WWWWWWWWWWWWWWWW
        kkkkkkkkkkkkkkwW
        wwwwwwwwwwwwwwwW
        wwwWWWWWWWWWWwwW
        wwwWwwwwwwwwkwwW
        wwwWwwwwwwwwkwwW
        wwwWwwwwwwwwkwwW
        wwwWkkkkkkkkkwwW
        wwwwwwwwwwwwwwwW
        WWWWWWWWWWWWWWWW`,
    },
  },
  {
    id: "desk_side",
    kind: "furniture",
    foot: [1, 1],
    desc: "a members' desk run seen end-on (the horseshoe's arms): the leather top, the end panel",
    strip: {
      dir: "v",
      start: `
        ...kkkkkkkkkk...
        ..kkvvvvvvvvwW..
        ..kvuuuuuuuuwW..
        ..kuuuuuuuuuwW..`,
      mid: `
        ..kuuuuuuuuuwW..
        ..kuuuuuuuuuwW..
        ..kuuuuuuuuuwW..
        ..kuuuuuuuuuwW..
        ..kuuuuuuuuuwW..
        ..kuuuuuuuuuwW..
        ..kuuuuuuuuuwW..
        ..kuuuuuuuuuwW..
        ..kuuuuuuuuuwW..
        ..kuuuuuuuuuwW..
        ..kuuuuuuuuuwW..
        ..kuuuuuuuuuwW..
        ..kuuuuuuuuuwW..
        ..kuuuuuuuuuwW..
        ..kwwwwwwwwwwW..
        ..kvuuuuuuuuwW..`,
      end: `
        ..kuuuuuuuuuwW..
        ..kwwwwwwwwwwW..
        ..WWWWWWWWWWWW..
        ..wkkkkkkkkkkW..
        ..wkwWWWWWWwwW..
        ..wkwWwwwwkwwW..
        ..wkwWwwwwkwwW..
        ..wkwWwwwwkwwW..
        ..wkwWkkkkkwwW..
        ..wkwwwwwwwwwW..
        ..wwwwwwwwwwwW..
        ..WWWWWWWWWWWW..`,
    },
  },
  {
    id: "clerk_table",
    kind: "furniture",
    foot: [1, 1],
    desc: "the clerk's table under a cloth in the civic colour, gold fringe",
    strip: {
      dir: "h",
      start: `
        .rrrrrrrrrrrrrrr
        raaaaaaaaaaaaaaa
        raaaaaaaaaaaaaaa
        raaaaaaaaaaaaaaa
        raaaaaaaaaaaaaaa
        rrrrrrrrrrrrrrrr
        AAAAAAAAAAAAAAAA
        aAaaaAaaaAaaaAaa
        aAaaaAaaaAaaaAaa
        aAaaaAaaaAaaaAaa
        aAaaaAaaaAaaaAaa
        aAaaaAaaaAaaaAaa
        BbBBBbBBBbBBBbBB
        .b.b.b.b.b.b.b.b`,
      mid: `
        rrrrrrrrrrrrrrrr
        aaaaaaaaaaaaaaaa
        aaaaaaaaaaaaaaaa
        aaaaaaaaaaaaaaaa
        aaaaaaaaaaaaaaaa
        rrrrrrrrrrrrrrrr
        AAAAAAAAAAAAAAAA
        aAaaaAaaaAaaaAaa
        aAaaaAaaaAaaaAaa
        aAaaaAaaaAaaaAaa
        aAaaaAaaaAaaaAaa
        aAaaaAaaaAaaaAaa
        BbBBBbBBBbBBBbBB
        .b.b.b.b.b.b.b.b`,
      end: `
        rrrrrrrrrrrrrrr.
        aaaaaaaaaaaaaaaA
        aaaaaaaaaaaaaaaA
        aaaaaaaaaaaaaaaA
        aaaaaaaaaaaaaaaA
        rrrrrrrrrrrrrrrA
        AAAAAAAAAAAAAAAA
        aAaaaAaaaAaaaAaA
        aAaaaAaaaAaaaAaA
        aAaaaAaaaAaaaAaA
        aAaaaAaaaAaaaAaA
        aAaaaAaaaAaaaAaA
        BbBBBbBBBbBBBbBB
        .b.b.b.b.b.b.b.b`,
    },
  },
  {
    id: "lectern",
    kind: "furniture",
    foot: [1, 1],
    desc: "the speaker's lectern with the crest on its front",
    grid: `
      ..kkkkkkkkkkkk..
      .kwwwwwwwwwwwwW.
      .WWWWWWWWWWWWWW.
      ...wkkkkkkkkW...
      ...wkwwwwwwwW...
      ...wkwwaawwwW...
      ...wkwaBBawwW...
      ...wkwaBBawwW...
      ...wkwwaawwwW...
      ...wkwwwwwwwW...
      ...wkwwwwwwwW...
      ...wkwwwwwwwW...
      ...wkwwwwwwwW...
      ...wkwwwwwwwW...
      ...wkwwwwwwwW...
      ..kwwwwwwwwwwW..
      ..WWWWWWWWWWWW..`,
  },
  {
    id: "flag_stand",
    kind: "furniture",
    foot: [1, 1],
    desc: "an indoor flag on a pole, draped, brass finial and a turned base",
    grid: `
      ....Bh..........
      ....bB..........
      ....wkaaaaaaa...
      ....wkarraaaaa..
      ....wkaraaaaaAa.
      ....wkaaaaaaaAa.
      ....wkeeeeeeeAe.
      ....wkeffeeeeEe.
      ....wkeeeeeeeEe.
      ....wkaaaaaaaAa.
      ....wkaraaaaaAa.
      ....wkaraaaaaAA.
      ....wk.aaaaaaAa.
      ....wk.aaaaaAA..
      ....wk..aaaaA...
      ....wk..aaaA....
      ....wk...aA.....
      ....wk..........
      ....wk..........
      ....wk..........
      ....wk..........
      ....wk..........
      ....wk..........
      ....wk..........
      ....wk..........
      ....wk..........
      ...WwwkW........
      ..WwwwkkW.......
      ..WWWWWWW.......`,
  },
  {
    id: "rail",
    kind: "furniture",
    foot: [1, 1],
    desc: "the gallery's balustrade: a handrail on turned balusters",
    strip: {
      dir: "h",
      start: `
        .kkkkkkkkkkkkkkk
        kwwwwwwwwwwwwwww
        WWWWWWWWWWWWWWWW
        wkW.kW.kW.kW.kW.
        wkW.kW.kW.kW.kW.
        wkWwkWwkWwkWwkWw
        wkW.kW.kW.kW.kW.
        wkW.kW.kW.kW.kW.
        wkWwwwwwwwwwwwww
        WWWWWWWWWWWWWWWW`,
      mid: `
        kkkkkkkkkkkkkkkk
        wwwwwwwwwwwwwwww
        WWWWWWWWWWWWWWWW
        .kW.kW.kW.kW.kW.
        .kW.kW.kW.kW.kW.
        wkWwkWwkWwkWwkWw
        .kW.kW.kW.kW.kW.
        .kW.kW.kW.kW.kW.
        wwwwwwwwwwwwwwww
        WWWWWWWWWWWWWWWW`,
      end: `
        kkkkkkkkkkkkkkk.
        wwwwwwwwwwwwwwkW
        WWWWWWWWWWWWWWWW
        .kW.kW.kW.kW.wkW
        .kW.kW.kW.kW.wkW
        wkWwkWwkWwkWwwkW
        .kW.kW.kW.kW.wkW
        .kW.kW.kW.kW.wkW
        wwwwwwwwwwwwwwkW
        WWWWWWWWWWWWWWWW`,
    },
  },
  {
    id: "pew",
    kind: "seat",
    foot: [1, 1],
    sit: 2,
    front: 3,
    desc: "a public-gallery pew seen from behind (the backrest hides the sitter's lower half)",
    strip: {
      dir: "h",
      start: `
        .WWWWWWWWWWWWWWW
        Wwwwwwwwwwwwwwww
        Wwwwwwwwwwwwwwww
        .kkkkkkkkkkkkkkk
        kwwwwwwwwwwwwwww
        WWWWWWWWWWWWWWWW
        wkwwwwwwwwwwwwww
        wkwWWWWWWWWWWwww
        wkwWwwwwwwwwkwww
        wkwWwwwwwwwwkwww
        wkwWkkkkkkkkkwww
        wkwwwwwwwwwwwwww
        WWWWWWWWWWWWWWWW
        .W..............`,
      mid: `
        WWWWWWWWWWWWWWWW
        wwwwwwwwwwwwwwww
        wwwwwwwwwwwwwwww
        kkkkkkkkkkkkkkkk
        wwwwwwwwwwwwwwww
        WWWWWWWWWWWWWWWW
        wwwwwwwwwwwwwwww
        wwwWWWWWWWWWWwww
        wwwWwwwwwwwwkwww
        wwwWwwwwwwwwkwww
        wwwWkkkkkkkkkwww
        wwwwwwwwwwwwwwww
        WWWWWWWWWWWWWWWW
        ................`,
      end: `
        WWWWWWWWWWWWWWW.
        wwwwwwwwwwwwwwwW
        wwwwwwwwwwwwwwwW
        kkkkkkkkkkkkkkk.
        wwwwwwwwwwwwwwkW
        WWWWWWWWWWWWWWWW
        wwwwwwwwwwwwwwkW
        wwwWWWWWWWWWWwkW
        wwwWwwwwwwwwkwkW
        wwwWwwwwwwwwkwkW
        wwwWkkkkkkkkkwkW
        wwwwwwwwwwwwwwkW
        WWWWWWWWWWWWWWWW
        ..............W.`,
    },
  },
  {
    id: "chair_front",
    kind: "seat",
    sit: 11,
    front: 12,
    desc: "an upholstered chair facing the room (the sitter sits in front of its back)",
    grid: `
      ....wkkkkkkw....
      ....wuvvvvuW....
      ....wuuuuuuW....
      ....wuuuuuuW....
      ....wuuuuuuW....
      ....wuuuuuuW....
      ....wuuuuuuW....
      ....wUUUUUUW....
      ...kwwwwwwwwW...
      ...kwwwwwwwwW...
      ...wuuuuuuuuW...
      ...wvvvvvvvvW...
      ...WUUUUUUUUW...
      ...W.W....W.W...
      ...W.W....W.W...`,
  },
  {
    id: "chair_back",
    kind: "seat",
    sit: 1,
    front: 2,
    desc: "an upholstered chair seen from behind (its back hides the sitter's lower back)",
    grid: `
      ...WwwwwwwwwW...
      ...wuuuuuuuuW...
      ....kwwwwwwk....
      ....wUUUUUUW....
      ....wUuuuuUW....
      ....wUuuuuUW....
      ....wUuuuuUW....
      ....wUuuuuUW....
      ....wUUUUUUW....
      ...W.W....W.W...
      ...W.W....W.W...`,
  },
  // ---- the town hall
  {
    id: "counter",
    kind: "furniture",
    foot: [1, 1],
    desc: "the clerk's counter: a stone top on a panelled wooden front",
    strip: {
      dir: "h",
      start: `
        .ttttttttttttttt
        tsssssssssssssss
        tsssssssssssssss
        tsssssssssssssss
        tsssssssssssssss
        SSSSSSSSSSSSSSSS
        TTTTTTTTTTTTTTTT
        wkkkkkkkkkkkkkkk
        wkwwwwwwwwwwwwww
        wkwWWWWWWWWWWwww
        wkwWwwwwwwwwkwww
        wkwWwwwwwwwwkwww
        wkwWwwwwwwwwkwww
        wkwWwwwwwwwwkwww
        wkwWwwwwwwwwkwww
        wkwWkkkkkkkkkwww
        wkwwwwwwwwwwwwww
        WWWWWWWWWWWWWWWW`,
      mid: `
        tttttttttttttttt
        ssssssssssssssss
        ssssssssssssssss
        ssssssssssssssss
        ssssssssssssssss
        SSSSSSSSSSSSSSSS
        TTTTTTTTTTTTTTTT
        kkkkkkkkkkkkkkkk
        wwwwwwwwwwwwwwww
        wwwWWWWWWWWWWwww
        wwwWwwwwwwwwkwww
        wwwWwwwwwwwwkwww
        wwwWwwwwwwwwkwww
        wwwWwwwwwwwwkwww
        wwwWwwwwwwwwkwww
        wwwWkkkkkkkkkwww
        wwwwwwwwwwwwwwww
        WWWWWWWWWWWWWWWW`,
      end: `
        ttttttttttttttt.
        ssssssssssssssSS
        ssssssssssssssSS
        ssssssssssssssSS
        ssssssssssssssSS
        SSSSSSSSSSSSSSSS
        TTTTTTTTTTTTTTTT
        kkkkkkkkkkkkkkwW
        wwwwwwwwwwwwwwwW
        wwwWWWWWWWWWWwwW
        wwwWwwwwwwwwkwwW
        wwwWwwwwwwwwkwwW
        wwwWwwwwwwwwkwwW
        wwwWwwwwwwwwkwwW
        wwwWwwwwwwwwkwwW
        wwwWkkkkkkkkkwwW
        wwwwwwwwwwwwwwwW
        WWWWWWWWWWWWWWWW`,
    },
  },
  {
    id: "counter_side",
    kind: "furniture",
    foot: [1, 1],
    desc: "the counter's return, seen end-on",
    strip: {
      dir: "v",
      start: `
        ..tttttttttttt..
        .tsssssssssssS..
        .tsssssssssssS..
        .tsssssssssssS..`,
      mid: `
        .tsssssssssssS..
        .tsssssssssssS..
        .tsssssssssssS..
        .tsssssssssssS..
        .tsssssssssssS..
        .tsssssssssssS..
        .tsssssssssssS..
        .tsssssssssssS..
        .tsssssssssssS..
        .tsssssssssssS..
        .tsssssssssssS..
        .tsssssssssssS..
        .tsssssssssssS..
        .tsssssssssssS..
        .tsssssssssssS..
        .tsssssssssssS..`,
      end: `
        .tsssssssssssS..
        .SSSSSSSSSSSSS..
        .TTTTTTTTTTTTT..
        .wkkkkkkkkkkkW..
        .wkwWWWWWWWwwW..
        .wkwWwwwwwkwwW..
        .wkwWwwwwwkwwW..
        .wkwWwwwwwkwwW..
        .wkwWwwwwwkwwW..
        .wkwWkkkkkkwwW..
        .wkwwwwwwwwwwW..
        .WWWWWWWWWWWWW..`,
    },
  },
  {
    id: "office_desk",
    kind: "furniture",
    foot: [2, 1],
    desc: "an office desk: a drawer pedestal on the left, legs on the right",
    grid: `
      .kkkkkkkkkkkkkkkkkkkkkkkkkkkkkk.
      kwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwW
      kwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwW
      kwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwW
      kwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwW
      WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW
      wkkkkkkkkkWwwwwwwwwwwwwwwwwwwkwW
      wkwwwwwwwwW..................kwW
      wkwwwbbwwwW..................kwW
      wkWWWWWWWWW..................kwW
      wkkkkkkkkkW..................kwW
      wkwwwbbwwwW..................kwW
      wkWWWWWWWWW..................kwW
      wkkkkkkkkkW..................kwW
      wkwwwbbwwwW..................kwW
      WWWWWWWWWWW..................WWW`,
  },
  {
    id: "filing_cabinet",
    kind: "furniture",
    foot: [1, 1],
    desc: "a four-drawer steel filing cabinet with brass label holders",
    grid: `
      ..NNNNNNNNNNNN..
      ..nnnnnnnnnnnM..
      ..MMMMMMMMMMMM..
      ..nNNNNNNNNNmM..
      ..nnnnhbhnnnmM..
      ..nnnnmmmnnnmM..
      ..MMMMMMMMMMMM..
      ..nNNNNNNNNNmM..
      ..nnnnhbhnnnmM..
      ..nnnnmmmnnnmM..
      ..MMMMMMMMMMMM..
      ..nNNNNNNNNNmM..
      ..nnnnhbhnnnmM..
      ..nnnnmmmnnnmM..
      ..MMMMMMMMMMMM..
      ..nNNNNNNNNNmM..
      ..nnnnhbhnnnmM..
      ..nnnnmmmnnnmM..
      ..nnnnnnnnnnmM..
      ..MMMMMMMMMMMM..`,
  },
  {
    id: "plant_tall",
    kind: "furniture",
    foot: [1, 1],
    desc: "a rubber plant in a terracotta pot",
    grid: `
      .......jg.......
      ......jgG.......
      ...gg.gG..jj....
      ..jggg.G.jggG...
      ..jgGGgG.jgGG...
      ...GG.gGjggG....
      .jj....GgGG.....
      jggGg..Gg...gg..
      jgGGG.gG...jggG.
      .GG..jgGGGjgGGG.
      .....ggG..ggGG..
      ..jgg.gG.GG.....
      .jgGGGgGgG......
      ..GG..gGG..jg...
      ......gG..jgGG..
      .....gGG...GG...
      ......gG........
      ....pppppppP....
      ....PPPPPPPP....
      .....ppppppP....
      .....ppppppP....
      .....ppppppP....
      ......PPPPP.....`,
  },
  {
    id: "plant_small",
    kind: "furniture",
    foot: [1, 1],
    desc: "a fern in a glazed pot",
    grid: `
      ....j..g..j.....
      ..jg.gjgG.gGj...
      .jgGjggGgjgGGj..
      jgGgjgGGgjgGgGG.
      .GGgGgGGGgGgGG..
      ..GG.GG.GG.GG...
      ....eeeeeeeE....
      ....EEEEEEEE....
      .....eeeeeeE....
      .....eeeeeeE....
      ......EEEEE.....`,
  },
  {
    id: "waiting_bench",
    kind: "seat",
    foot: [1, 1],
    sit: 10,
    front: 12,
    desc: "a slatted waiting bench on iron legs",
    strip: {
      dir: "h",
      start: `
        ..kkkkkkkkkkkkkk
        .kwwwwwwwwwwwwww
        .wWWWWWWWWWWWWWW
        .wkkkkkkkkkkkkkk
        .wwwwwwwwwwwwwww
        .wWWWWWWWWWWWWWW
        .ikkkkkkkkkkkkkk
        .iwwwwwwwwwwwwww
        .iWWWWWWWWWWWWWW
        ikkkkkkkkkkkkkkk
        iwwwwwwwwwwwwwww
        iwwwwwwwwwwwwwww
        iWWWWWWWWWWWWWWW
        Ii..............
        .i..............`,
      mid: `
        kkkkkkkkkkkkkkkk
        wwwwwwwwwwwwwwww
        WWWWWWWWWWWWWWWW
        kkkkkkkkkkkkkkkk
        wwwwwwwwwwwwwwww
        WWWWWWWWWWWWWWWW
        kkkkkkkkkkkkkkkk
        wwwwwwwwwwwwwwww
        WWWWWWWWWWWWWWWW
        kkkkkkkkkkkkkkkk
        wwwwwwwwwwwwwwww
        wwwwwwwwwwwwwwww
        WWWWWWWWWWWWWWWW
        ................
        ................`,
      end: `
        kkkkkkkkkkkkkk..
        wwwwwwwwwwwwwwW.
        WWWWWWWWWWWWWWW.
        kkkkkkkkkkkkkkW.
        wwwwwwwwwwwwwwW.
        WWWWWWWWWWWWWWW.
        kkkkkkkkkkkkkki.
        wwwwwwwwwwwwwwi.
        WWWWWWWWWWWWWWi.
        kkkkkkkkkkkkkkki
        wwwwwwwwwwwwwwwi
        wwwwwwwwwwwwwwwi
        WWWWWWWWWWWWWWWi
        ..............Ii
        ..............i.`,
    },
  },
  {
    id: "water_cooler",
    kind: "furniture",
    foot: [1, 1],
    desc: "a water cooler: a blue bottle on a white cabinet",
    grid: `
      ......nn........
      .....nNNn.......
      ....nNNNNn......
      ....nNNNmn......
      ....nNNNmn......
      ....nNNNmn......
      .....nnnn.......
      ...cccccccC.....
      ...ccxcOccC.....
      ...ccccccCC.....
      ...ccMMMMcC.....
      ...ccccccCC.....
      ...ccccccCC.....
      ...ccccccCC.....
      ...ccccccCC.....
      ...ccccccCC.....
      ...ccccccCC.....
      ...CCCCCCCC.....`,
  },
  {
    id: "stanchion",
    kind: "furniture",
    foot: [1, 1],
    desc: "queue posts and a sagging rope in the civic colour",
    strip: {
      dir: "h",
      start: `
        .hB.............
        .BB.............
        .bBaa.........aa
        .bB.aaa.....aaa.
        .bB....aaaaa....
        .bB.............
        .bB.............
        .bB.............
        .bB.............
        BBBB............`,
      mid: `
        .......hB.......
        .......BB.......
        aa.....bBaa.....
        .aaa...bB.aaa...
        ....aaabB....aaa
        .......bB.......
        .......bB.......
        .......bB.......
        .......bB.......
        ......BBBB......`,
      end: `
        .............hB.
        .............BB.
        aa...........bB.
        .aaa.....aaa.bB.
        ....aaaaa....bB.
        .............bB.
        .............bB.
        .............bB.
        .............bB.
        ............BBBB`,
    },
  },
  {
    id: "bin",
    kind: "furniture",
    foot: [1, 1],
    desc: "a wastepaper bin",
    grid: `
      .......nnnnnnn..
      .......mMMMMMm..
      .......mnmnmnm..
      .......mnmnmnm..
      .......mnmnmnm..
      .......MMMMMMM..`,
  },
  {
    id: "coat_stand",
    kind: "furniture",
    foot: [1, 1],
    desc: "a bentwood coat stand with a coat and a hat",
    grid: `
      ......kk........
      ....wWwwWw......
      ...uuUwwOOO.....
      ..uuuUwOOOOO....
      ..uvuUw.OO......
      ..uvuUw.........
      ..uvuUw.........
      ..uuuUw.........
      ..uuUUw.........
      ..UUU.w.........
      ......w.........
      ......w.........
      ......w.........
      ......w.........
      ......w.........
      ......w.........
      ......w.........
      ....wwWWw.......
      ...W.....W......`,
  },
  // ---- the diner
  {
    id: "booth_bench",
    kind: "seat",
    foot: [1, 2],
    sit: 21,
    front: 36,
    desc: "a booth bench, end-on: a tall vinyl back on the left, a buttoned seat (mirrored for the right side)",
    grid: `
      .kkkkk..........
      kvvvvvW.........
      kvuuuuW.........
      kvuuuuW.........
      kvuuuuW.........
      kvuuuuW.........
      kvuuuuW.........
      kvuuuuW.........
      kvuuuuW.........
      kvuuuuW.........
      kvuuuuW.........
      kvuuuuW.........
      kvuuuuWvvvvvvv..
      kvuuuuWuuuuuuuU.
      kvuuuuWuuuuuuuU.
      kvuuuuWuuuuuuuU.
      kvuuuuWuuuuuuuU.
      kvuuuuWuuuuuuuU.
      kvuuuuWuuuuuuuU.
      kvuuuuWuuuuuuuU.
      kvuuuuWUUUUUUUU.
      kvuuuuWvvvvvvv..
      kvuuuuWuuuuuuuU.
      kvuuuuWuuuuuuuU.
      kvuuuuWuuuuuuuU.
      kvuuuuWuuuuuuuU.
      kvuuuuWuuuuuuuU.
      kvuuuuWuuuuuuuU.
      kvuuuuWuuuuuuuU.
      kvuuuuWuuuuuuuU.
      kvuuuuWuuuuuuuU.
      kvuuuuWuuuuuuuU.
      kvuuuuWuuuuuuuU.
      kvuuuuWuuuuuuuU.
      kvuuuuWuuuuuuuU.
      kWWWWWWUUUUUUUU.
      kuuuuuUvvvvvvvU.
      kuUuuUUuUuuUuuU.
      kuuuuuUuuuuuuuU.
      kuUuuUUuUuuUuuU.
      nnnnnnnnnnnnnnM.
      MMMMMMMMMMMMMMM.`,
  },
  {
    id: "booth_table",
    kind: "furniture",
    foot: [1, 2],
    desc: "a booth table end-on: a formica top with a chrome rim on a pedestal",
    grid: `
      .NNNNNNNNNNNNNN.
      NffffffffffffffM
      NeeeeeeeeeeeeeeM
      NeeeeeeeeeeeeeeM
      NeeeeeeeeeeeeeeM
      NeeeeeeeeeeeeeeM
      NeeeeeeeeeeeeeeM
      NeeeeeeeeeeeeeeM
      NeeeeeeeeeeeeeeM
      NeeeeeeeeeeeeeeM
      NeeeeeeeeeeeeeeM
      NeeeeeeeeeeeeeeM
      NeeeeeeeeeeeeeeM
      NeeeeeeeeeeeeeeM
      NeeeeeeeeeeeeeeM
      NeeeeeeeeeeeeeeM
      NeeeeeeeeeeeeeeM
      NeeeeeeeeeeeeeeM
      NeeeeeeeeeeeeeeM
      NeeeeeeeeeeeeeeM
      NeeeeeeeeeeeeeeM
      NeeeeeeeeeeeeeeM
      nnnnnnnnnnnnnnnM
      MMMMMMMMMMMMMMMM
      ......nm........
      ......nm........
      ......nm........
      ......nm........
      ......nm........
      ......nm........
      ....nnnmmm......
      ....MMMMMM......`,
  },
  {
    id: "diner_counter",
    kind: "furniture",
    foot: [1, 1],
    desc: "the diner counter: a formica top with a chrome edge, a ribbed vinyl front, a steel kick plate",
    strip: {
      dir: "h",
      start: `
        .NNNNNNNNNNNNNNN
        Nfffffffffffffff
        Neeeeeeeeeeeeeee
        Neeeeeeeeeeeeeee
        Neeeeeeeeeeeeeee
        nnnnnnnnnnnnnnnn
        MMMMMMMMMMMMMMMM
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        nnnnnnnnnnnnnnnn
        mmmmmmmmmmmmmmmm
        MMMMMMMMMMMMMMMM`,
      mid: `
        NNNNNNNNNNNNNNNN
        ffffffffffffffff
        eeeeeeeeeeeeeeee
        eeeeeeeeeeeeeeee
        eeeeeeeeeeeeeeee
        nnnnnnnnnnnnnnnn
        MMMMMMMMMMMMMMMM
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        nnnnnnnnnnnnnnnn
        mmmmmmmmmmmmmmmm
        MMMMMMMMMMMMMMMM`,
      end: `
        NNNNNNNNNNNNNNN.
        fffffffffffffffM
        eeeeeeeeeeeeeeeM
        eeeeeeeeeeeeeeeM
        eeeeeeeeeeeeeeeM
        nnnnnnnnnnnnnnnM
        MMMMMMMMMMMMMMMM
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        vuuUvuuUvuuUvuuU
        nnnnnnnnnnnnnnnM
        mmmmmmmmmmmmmmmM
        MMMMMMMMMMMMMMMM`,
    },
  },
  {
    id: "back_counter",
    kind: "furniture",
    foot: [1, 1],
    desc: "the back counter against the wall: a steel worktop over wooden cupboards",
    strip: {
      dir: "h",
      start: `
        NNNNNNNNNNNNNNNN
        nnnnnnnnnnnnnnnn
        nnnnnnnnnnnnnnnn
        mmmmmmmmmmmmmmmm
        MMMMMMMMMMMMMMMM
        wkkkkkkWwkkkkkkW
        wkwwwwwWwkwwwwwW
        wkwwwwbWwbwwwwwW
        wkwwwwbWwbwwwwwW
        wkwwwwwWwkwwwwwW
        wkwwwwwWwkwwwwwW
        wkwwwwwWwkwwwwwW
        wkwwwwwWwkwwwwwW
        wkwwwwwWwkwwwwwW
        WWWWWWWWWWWWWWWW`,
      mid: `
        NNNNNNNNNNNNNNNN
        nnnnnnnnnnnnnnnn
        nnnnnnnnnnnnnnnn
        mmmmmmmmmmmmmmmm
        MMMMMMMMMMMMMMMM
        wkkkkkkWwkkkkkkW
        wkwwwwwWwkwwwwwW
        wkwwwwbWwbwwwwwW
        wkwwwwbWwbwwwwwW
        wkwwwwwWwkwwwwwW
        wkwwwwwWwkwwwwwW
        wkwwwwwWwkwwwwwW
        wkwwwwwWwkwwwwwW
        wkwwwwwWwkwwwwwW
        WWWWWWWWWWWWWWWW`,
      end: `
        NNNNNNNNNNNNNNNN
        nnnnnnnnnnnnnnnM
        nnnnnnnnnnnnnnnM
        mmmmmmmmmmmmmmmM
        MMMMMMMMMMMMMMMM
        wkkkkkkWwkkkkkkW
        wkwwwwwWwkwwwwwW
        wkwwwwbWwbwwwwwW
        wkwwwwbWwbwwwwwW
        wkwwwwwWwkwwwwwW
        wkwwwwwWwkwwwwwW
        wkwwwwwWwkwwwwwW
        wkwwwwwWwkwwwwwW
        wkwwwwwWwkwwwwwW
        WWWWWWWWWWWWWWWW`,
    },
  },
  {
    id: "stool",
    kind: "seat",
    sit: 2,
    front: 99,
    desc: "a counter stool: a vinyl seat on a chrome post",
    grid: `
      .....vvvvvv.....
      ....vuuuuuuU....
      ....uuuuuuuU....
      ....UUUUUUUU....
      .....NnnnnM.....
      .......nm.......
      .......nm.......
      .......nm.......
      .......nm.......
      .......nm.......
      .....Nnnmmm.....
      .....MMMMMM.....`,
  },
  {
    id: "jukebox",
    glow: "always",
    kind: "furniture",
    foot: [1, 1],
    lit: "Llar",
    desc: "a jukebox: a lit arch in the accent colours, a chrome grille",
    grid: `
      .....wwwwww.....
      ...wwkkkkkkww...
      ..wkrrrrrrrrkw..
      .wkraaaaaaaarkw.
      .wkraLLLLLLarkw.
      .wkraLllllLarkw.
      .wkraLlOOlLarkw.
      .wkraLlOOlLarkw.
      .wkraLllllLarkw.
      .wkraLLLLLLarkw.
      .wkraaaaaaaarkw.
      .wkWWWWWWWWWWkw.
      .wkwNnNnNnNnwkw.
      .wkwnMnMnMnMwkw.
      .wkwNnNnNnNnwkw.
      .wkwnMnMnMnMwkw.
      .wkwNnNnNnNnwkw.
      .wkwwwwwwwwwwkw.
      .wkrrrrrrrrrrkw.
      .wkaaaaaaaaaakw.
      .wkwwwwwwwwwwkw.
      .WWWWWWWWWWWWWW.`,
  },
  {
    id: "cafe_table",
    kind: "furniture",
    foot: [1, 1],
    desc: "a small round table on a pedestal",
    grid: `
      ....NNNNNNNN....
      ..NffffffffffN..
      .NeeeeeeeeeeeeM.
      .NeeeeeeeeeeeeM.
      .NeeeeeeeeeeeeM.
      ..MnnnnnnnnnnM..
      ....MMMMMMMM....
      .......nm.......
      .......nm.......
      .......nm.......
      .....nnnmmm.....
      .....MMMMMM.....`,
  },
  {
    id: "news_rack",
    kind: "furniture",
    foot: [1, 1],
    desc: "a newspaper rack: today's papers, folded (no text: headline bars and a photo block)",
    grid: `
      ..iIIIIIIIIIIi..
      ..icccccCccccI..
      ..icOOOCcOOOcI..
      ..icCCCCcCCCcI..
      ..icccccCcccci..
      ..icSSSCcSSScI..
      ..icSSSCcCCCcI..
      ..iIIIIIIIIIIi..
      ..icccccCccccI..
      ..icxxxCcOOOcI..
      ..icCCCCcCCCcI..
      ..icCCCCcSSScI..
      ..iIIIIIIIIIIi..
      ..i..........i..
      ..i..........i..
      .ii..........ii.`,
  },
  // ---- things on top of furniture (outlined; drawn with what they rest on)
  {
    id: "mic",
    kind: "top",
    rest: 2,
    desc: "a gooseneck microphone",
    grid: `
      ii.
      .i.
      .i.
      iii`,
  },
  {
    id: "papers",
    kind: "top",
    rest: 2,
    desc: "a stack of papers",
    grid: `
      .cccccc
      ccccccC
      CcCCcCC`,
  },
  {
    id: "nameplate",
    kind: "top",
    rest: 1,
    desc: "a brass nameplate",
    grid: `
      .hhhh.
      bBBBBb`,
  },
  {
    id: "banker_lamp",
    kind: "top",
    rest: 2,
    lit: "l",
    desc: "a banker's lamp: a green glass shade on a brass stem",
    grid: `
      ..jgggggG.
      .jggggggGG
      ..lllllll.
      .....B....
      .....B....
      ...bBBBb..`,
  },
  {
    id: "monitor",
    glow: "always",
    kind: "top",
    rest: 3,
    lit: "N",
    desc: "a computer monitor and keyboard",
    grid: `
      .OOOOOOOOOO.
      .ONNNNNNNNO.
      .ONnNNNnNNO.
      .ONNNNNNNNO.
      .ONNNnNNNNO.
      .OOOOOOOOOO.
      .....OO.....
      ...OOOOOO...
      ..cccccccC..
      ..CCCCCCCC..`,
  },
  {
    id: "counter_bell",
    kind: "top",
    rest: 2,
    desc: "a brass service bell",
    grid: `
      ..h..
      .hBB.
      bBBBb`,
  },
  {
    id: "counter_screen",
    kind: "top",
    rest: 4,
    desc: "a glass screen with a speaking hole on the counter",
    grid: `
      MMMMMMMMMMMMMMMM
      M....N.......N.M
      M...N.......N..M
      M..N..MMMM.N...M
      M.N..M....M....M
      MN....MMMM.....M
      M..............M
      M..............M
      MMMMMMMMMMMMMMMM`,
  },
  {
    id: "coffee_machine",
    glow: "always",
    kind: "top",
    rest: 3,
    lit: "x",
    desc: "the coffee machine: a steel body, two glass pots, a red light",
    grid: `
      NNNNNNNNNNNNNN
      nnnnnnnnnnnnnM
      nnxnnnnnnnnnnM
      MMMMMMMMMMMMMM
      nM..........nM
      nMOOO..OOO..nM
      nMOFO..OoO..nM
      nMoOo..oOo..nM
      nMMMM..MMM..nM
      NNNNNNNNNNNNNM`,
  },
  {
    id: "register",
    kind: "top",
    rest: 3,
    desc: "a cash register",
    grid: `
      ...MMMMMM.
      ...MooooM.
      ..MMMMMMM.
      .NnnnnnnnM
      NcCcCcCnnM
      NcCcCcCnnM
      MMMMMMMMMM`,
  },
  {
    id: "pie_case",
    kind: "top",
    rest: 3,
    desc: "a glass cake dome over a pie",
    grid: `
      ....NNNN....
      ..NN....NN..
      .N........N.
      N..YYYYYY..M
      N.YxxYYxxY.M
      N.RRRRRRRR.M
      nnnnnnnnnnnM
      ....MMMM....`,
  },
  {
    id: "condiments",
    kind: "top",
    rest: 2,
    desc: "ketchup and mustard bottles",
    grid: `
      .c..c.
      .x..Y.
      xxF.YR
      xxF.YR`,
  },
  {
    id: "plates",
    kind: "top",
    rest: 2,
    desc: "two plates with a meal, a mug",
    grid: `
      .cccc...cccc.
      cYYxcc.cgYYcc
      ccccC..ccccC.`,
  },
  {
    id: "mug",
    kind: "top",
    rest: 2,
    desc: "a coffee mug",
    grid: `
      cccc.
      cOOcc
      cccc.`,
  },
  {
    id: "table_lamp",
    kind: "top",
    rest: 2,
    lit: "Ll",
    desc: "a small table lamp with a warm shade",
    grid: `
      .LLLL.
      LllllL
      .LLLL.
      ..B...
      ..B...
      .bBb..`,
  },
  {
    id: "water_jug",
    kind: "top",
    rest: 2,
    desc: "a water jug and a glass",
    grid: `
      NN...
      nNn..
      nnn.N
      MMM.n`,
  },
  {
    id: "door_bell",
    kind: "wall",
    desc: "the shop bell on its spring bracket over the diner's door",
    grid: `
      iiiii..
      ....i..
      ...hB..
      ..hBBb.
      ..BBBb.
      .bbbbbb
      ...b...`,
  },
  // ---- floor decals (no outline; walkable)
  {
    id: "doormat",
    kind: "decal",
    desc: "a coir doormat inside the door",
    grid: `
      ..RRRRRRRRRRRR..
      .RYYYYYYYYYYYYR.
      .RYRYRYRYRYRYYR.
      .RYYYYYYYYYYYYR.
      .RYRYRYRYRYRYYR.
      .RYYYYYYYYYYYYR.
      ..RRRRRRRRRRRR..`,
  },
];

export const PART_BY_ID: ReadonlyMap<string, PartDef> = new Map(PARTS.map((p) => [p.id, p]));

// ---------------------------------------------------------------------------
// Rugs and runners: nine-slice from a corner, an edge and a field, in the
// accent (border) and second accent (field) ramps.

export const RUG = {
  corner: `
    AAAAAA
    Arrrrr
    ArAAAA
    ArAeee
    ArAeEe
    ArAeee`,
  edgeTop: `
    AAAA
    rrrr
    AAAA
    eeee
    eEee
    eeee`,
  field: `
    eeeeeeee
    eeeEeeee
    eeEfEeee
    eeeEeeee
    eeeeeeee
    eeeeeeeE
    eeeeeeEf
    eeeeeeeE`,
} as const;

// ---------------------------------------------------------------------------
// Parsing

/** A part's slot image(s): codes from ICODES (0 transparent). */
export interface ParsedPart {
  def: PartDef;
  img?: Indexed;
  strip?: { start: Indexed; mid: Indexed; end: Indexed };
}

const parseOne = (text: string): Indexed => {
  const f = gridFrames(text);
  if (f.length !== 1) throw new Error(`expected one frame, got ${f.length}`);
  return gridToIndexed(f[0] as string[], ICODES);
};

let parsed: Map<string, ParsedPart> | null = null;
export function parsedParts(): Map<string, ParsedPart> {
  if (parsed) return parsed;
  const out = new Map<string, ParsedPart>();
  for (const def of PARTS) {
    if (def.strip) out.set(def.id, { def, strip: { start: parseOne(def.strip.start), mid: parseOne(def.strip.mid), end: parseOne(def.strip.end) } });
    else if (def.grid) out.set(def.id, { def, img: parseOne(def.grid) });
    else throw new Error(`part ${def.id} has no drawing`);
  }
  parsed = out;
  return out;
}

/**
 * A part's slot image at a length (strips) or as drawn: a horizontal strip is
 * start, middles, end (16 px each; one tile takes the start's left half and
 * the end's right half); a vertical strip start, len - 1 middles, end.
 */
export function partSlots(id: string, len = 1, mirror = false): Indexed {
  const p = parsedParts().get(id);
  if (!p) throw new Error(`no interior part "${id}"`);
  let img: Indexed;
  if (p.img) img = p.img;
  else {
    const s = p.strip as NonNullable<ParsedPart["strip"]>;
    if (p.def.strip?.dir === "h") {
      img = blank(len * 16, s.start.h);
      if (len === 1) {
        blit(img, crop(s.start, 0, 0, 8, s.start.h), 0, 0);
        blit(img, crop(s.end, 8, 0, 8, s.end.h), 8, 0);
      } else {
        blit(img, s.start, 0, 0);
        for (let k = 1; k < len - 1; k++) blit(img, s.mid, k * 16, 0);
        blit(img, s.end, (len - 1) * 16, 0);
      }
    } else {
      img = blank(s.start.w, s.start.h + (len - 1) * s.mid.h + s.end.h);
      blit(img, s.start, 0, 0);
      for (let k = 0; k < len - 1; k++) blit(img, s.mid, 0, s.start.h + k * s.mid.h);
      blit(img, s.end, 0, s.start.h + (len - 1) * s.mid.h);
    }
  }
  return mirror ? mirrorX(img) : img;
}

/** Surface tiles parsed to surface codes (SCODES). */
let parsedSurfaces: Map<string, Indexed> | null = null;
export function surfaceSlots(id: string): Indexed {
  if (!parsedSurfaces) {
    parsedSurfaces = new Map(SURFACES.map((s) => [s.id, gridToIndexed(gridFrames(s.grid)[0] as string[], SCODES)]));
  }
  const s = parsedSurfaces.get(id);
  if (!s) throw new Error(`no interior surface "${id}"`);
  return s;
}

export const rugSlots = () => ({ corner: parseOne(RUG.corner), edgeTop: parseOne(RUG.edgeTop), field: parseOne(RUG.field) });
