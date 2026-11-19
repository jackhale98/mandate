// The town-centre building kit for the P0 spike, drawn by hand as text grids
// (D5): wall and roof textures, windows (with lit and boarded states), doors,
// a shopfront with fascia and awning, civic columns, a pediment, a clock
// turret, a dome, a chimney, rooftop clutter, and pictogram signs (no text in
// art, R14). Parts are drawn in slot colours; ./buildings recolours them per
// building from material ramps and lays them out by storeys and 16 px bays.
//
// Legend:
//   w W x m   wall base, shade, deep, light       k K j    trim base, shade, light
//   r R Z q   roof base, shade, deep, light       g G y Y  glass base, dark, reflection, bright
//   F f       frame dark, frame light (sash bars) d D e    door paint base, shade, light
//   a A c C   accent (awning, fascia) and stripe  i I      iron, iron light
//   o         deep shadow (recesses)              s S p    sign board, board shade, pictogram
//   b B n     wood planks (boarded) base, dark, light
//   l L h H   lit glass base, deep, light, core (only in lit states)

export const BSLOT = {
  wall: 1,
  wallS: 2,
  wallD: 3,
  wallL: 4,
  trim: 5,
  trimS: 6,
  trimL: 7,
  roof: 8,
  roofS: 9,
  roofD: 10,
  roofL: 11,
  glass: 12,
  glassD: 13,
  glassY: 14,
  glassB: 15,
  frame: 16,
  frameL: 17,
  door: 18,
  doorS: 19,
  doorL: 20,
  acc: 21,
  accS: 22,
  stripe: 23,
  stripeS: 24,
  iron: 25,
  ironL: 26,
  deep: 27,
  sign: 28,
  signS: 29,
  picto: 30,
  wood: 31,
  woodD: 32,
  woodL: 33,
  lit: 34,
  litD: 35,
  litL: 36,
  litC: 37,
  // Roof slots redirected for pieces with their own material: the turret's slate cap and the copper dome.
  cap: 38,
  capS: 39,
  capD: 40,
  capL: 41,
  dome: 42,
  domeS: 43,
  domeD: 44,
  domeL: 45,
  // The chimney's brick.
  stack: 46,
  stackS: 47,
  stackD: 48,
  stackL: 49,
} as const;
export const BSLOT_COUNT = 50;

export const BLEGEND: Readonly<Record<string, number>> = {
  w: BSLOT.wall,
  W: BSLOT.wallS,
  x: BSLOT.wallD,
  m: BSLOT.wallL,
  k: BSLOT.trim,
  K: BSLOT.trimS,
  j: BSLOT.trimL,
  r: BSLOT.roof,
  R: BSLOT.roofS,
  Z: BSLOT.roofD,
  q: BSLOT.roofL,
  g: BSLOT.glass,
  G: BSLOT.glassD,
  y: BSLOT.glassY,
  Y: BSLOT.glassB,
  F: BSLOT.frame,
  f: BSLOT.frameL,
  d: BSLOT.door,
  D: BSLOT.doorS,
  e: BSLOT.doorL,
  a: BSLOT.acc,
  A: BSLOT.accS,
  c: BSLOT.stripe,
  C: BSLOT.stripeS,
  i: BSLOT.iron,
  I: BSLOT.ironL,
  o: BSLOT.deep,
  s: BSLOT.sign,
  S: BSLOT.signS,
  p: BSLOT.picto,
  b: BSLOT.wood,
  B: BSLOT.woodD,
  n: BSLOT.woodL,
  l: BSLOT.lit,
  L: BSLOT.litD,
  h: BSLOT.litL,
  H: BSLOT.litC,
};

// ---------------------------------------------------------------------------
// Textures (tiled over a wall or roof)

export const TEX = {
  brick: `
    wwwwwwwWwwwwwwww
    wwmmwwwWwwwwwwww
    WWWWWWWWWWWWWWWW
    wwwWwwwwwwwWwmmw
    wwwWwwwwwwwWwwww
    WWWWWWWWWWWWWWWW`,
  render: `
    wwwwwwwwwwwwwwww
    wwwwwwwwwwwwwwww
    wwwwwwwwwwwwWwww
    wwwwwwwwwwwwwwww
    wwwmmwwwwwwwwwww
    wwwwwwwwwwwwwwww
    wwwwwwwwwwwwwwww
    wwwwwwwWWwwwwwww`,
  stone: `
    wwwwwwwWwwwwwwww
    wwwwwwwWwwwwwwww
    wwwwwwwWwwwwwwww
    WWWWWWWWWWWWWWWW
    wwwWwwwwwwwWwwww
    wwwWwwwwwwwWwwww
    wwwWwwwwwwwWwwww
    WWWWWWWWWWWWWWWW`,
  cladding: `
    wWwwwWwwwWwwwWww
    wWwwwWwwwWwwwWww
    wWwwwWwwwWwwwWww
    wWwwwWwwwWwwwWww`,
  tiles: `
    qqrrqqrrqqrrqqrr
    rrrRrrrRrrrRrrrR
    rrrRrrrRrrrRrrrR
    RRRRRRRRRRRRRRRR`,
  slates: `
    rrrrrRrrrrrrrRrr
    rrrrrRrrrrrrrRrr
    RRRRRRRRRRRRRRRR
    rRrrrrrrrRrrrrrr
    rRrrrrrrrRrrrrrr
    RRRRRRRRRRRRRRRR`,
  flat: `
    rrrrrrrrrrrrrrrr
    rrrrrrrrrRrrrrrr
    rrrrrrrrrrrrrrrr
    rrrRrrrrrrrrrrrr
    rrrrrrrrrrrrrrrr
    rrrrrrrrrrrrRrrr
    rrrrrrrrrrrrrrrr
    rrrrrRrrrrrrrrrr`,
};

// ---------------------------------------------------------------------------
// Windows: day state. The glass is g G y Y; the lit state recolours it.

export const WINDOWS = {
  /** Upper-floor sash window, 8×12. */
  sash: `
    kkkkkkkk
    FFFFFFFF
    FGGGGGGF
    FgyyggGF
    FgygggGF
    FffffffF
    FggyggGF
    FgyyggGF
    FgggggGF
    FFFFFFFF
    jkkkkkkK
    .WWWWWW.`,
  /** Ground-floor window, 10×14. */
  ground: `
    kkkkkkkkkk
    FFFFFFFFFF
    FGGGGGGGGF
    FgyygggGGF
    FgygggggGF
    FgggggggGF
    FffffffffF
    FgggyyggGF
    FggyygggGF
    FgggggggGF
    FgggggggGF
    FFFFFFFFFF
    jkkkkkkkkK
    .WWWWWWWW.`,
  /** Civic round-headed window, 8×15. */
  arched: `
    ..kkkk..
    .kFFFFk.
    kFGGGGFk
    FGgyygGF
    FgyyggGF
    FgygggGF
    FffffffF
    FggyggGF
    FgyyggGF
    FgggggGF
    FgggggGF
    FgggggGF
    FFFFFFFF
    jkkkkkkK
    .WWWWWW.`,
} as const;
export type WindowKind = keyof typeof WINDOWS;

/** Lit overlays drawn over the lit glass (variety at night): curtains and a half-drawn blind. */
export const LIT_OVERLAYS = {
  curtains: { sash: [1, 6], ground: [1, 8], arched: [1, 6] },
} as const;

/** Planks over a boarded window (the condition overlay, R7), drawn over the glass area. */
export const PLANKS = `
    bbbbbbbbbb
    BBBBBBBBBB
    .nbbbbbbn.
    nbbbbbbbbb
    BBBBBBBBBB
    .bbbbbbbb.
    bbbbbbbbbb
    BBBBBBBBBB
    bbbbbbbbbb
    BBBBBBBBBB`;

// ---------------------------------------------------------------------------
// Doors, 10 wide; the bottom row stands on the ground line.

export const DOORS = {
  /** Painted panel door with a fanlight, 10×21. */
  panel: `
    kkkkkkkkkk
    kFFFFFFFFK
    kFGyyggGFK
    kFFFFFFFFK
    kFedddddFK
    kFeDDDDdFK
    kFeDddDdFK
    kFeDddDdFK
    kFeDDDDdFK
    kFedddddFK
    kFedddddFK
    kFeDDDDdFK
    kFeDddDdFK
    kFeDddIdFK
    kFeDddDdFK
    kFeDDDDdFK
    kFedddddFK
    kFedddddFK
    kFDDDDDDFK
    jkkkkkkkkK
    KKKKKKKKKK`,
  /** Glazed shop door, 10×21. */
  shop: `
    kkkkkkkkkk
    kFFFFFFFFK
    kFGGGGGGFK
    kFgyygGGFK
    kFFFFFFFFK
    kFdGGGGdFK
    kFdgyygdFK
    kFdgygGdFK
    kFdggggdFK
    kFdgyggdFK
    kFdggggdFK
    kFdgyggdFK
    kFdggggdFK
    kFdgggGdFK
    kFdggggdFK
    kFdDDDDdFK
    kFddddddFK
    kFdDDDDdFK
    kFddddddFK
    jkkkkkkkkK
    KKKKKKKKKK`,
  /** Civic double door under a fanlight, 12×23. */
  civic: `
    ..kkkkkkkk..
    .kFFFFFFFFK.
    kFGGyyggGGFK
    kFgyyggggGFK
    kFFFFFFFFFFK
    kFeddDeddDFK
    kFeDdDeDdDFK
    kFeDdDeDdDFK
    kFeDdDeDdDFK
    kFeddDeddDFK
    kFeddDeddDFK
    kFeDdDeDdDFK
    kFeDdDeDdDFK
    kFeDdIIDdDFK
    kFeDdDeDdDFK
    kFeddDeddDFK
    kFeDdDeDdDFK
    kFeDdDeDdDFK
    kFeDdDeDdDFK
    kFeddDeddDFK
    kFDDDDDDDDFK
    jkkkkkkkkkkK
    KKKKKKKKKKKK`,
  /** Roller shutter (industrial), 14×20. */
  shutter: `
    KKKKKKKKKKKKKK
    KiiiiiiiiiiiiK
    KIIIIIIIIIIIIK
    KiiiiiiiiiiiiK
    KIIIIIIIIIIIIK
    KiiiiiiiiiiiiK
    KIIIIIIIIIIIIK
    KiiiiiiiiiiiiK
    KIIIIIIIIIIIIK
    KiiiiiiiiiiiiK
    KIIIIIIIIIIIIK
    KiiiiiiiiiiiiK
    KIIIIIIIIIIIIK
    KiiiiiiiiiiiiK
    KIIIIIIIIIIIIK
    KiiiiiiiiiiiiK
    KIIIIIIIIIIIIK
    KiiiiiiiiiiiiK
    KooooooooooooK
    KKKKKKKKKKKKKK`,
} as const;
export type DoorKind = keyof typeof DOORS;

// ---------------------------------------------------------------------------
// Shops: one shopfront bay (16×18: pilasters, display window, stall riser),
// the fascia (a band over all shop bays, 16×5), and the awning (16×6).

export const SHOPFRONT = `
    jkFFFFFFFFFFFFkK
    jkFGGGGGGGGGGFkK
    jkFgyyggggggGFkK
    jkFgygggggggGFkK
    jkFggggggyggGFkK
    jkFgggggyyggGFkK
    jkFgggggggggGFkK
    jkFgggggggggGFkK
    jkFgygggggggGFkK
    jkFyygggggggGFkK
    jkFgggggggggGFkK
    jkFFFFFFFFFFFFkK
    jkddddddddddddkK
    jkdDDDDDDDDDDdkK
    jkdDeeeeeeeeDdkK
    jkdDDDDDDDDDDdkK
    jkddddddddddddkK
    KKKKKKKKKKKKKKKK`;

export const FASCIA = `
    kkkkkkkkkkkkkkkk
    KKKKKKKKKKKKKKKK
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaaaaaaa
    AAAAAAAAAAAAAAAA`;

export const AWNING = `
    ccccaaaaccccaaaa
    ccccaaaaccccaaaa
    CCCCAAAACCCCAAAA
    ccccaaaaccccaaaa
    ccccaaaaccccaaaa
    .cc..aa..cc..aa.`;

// ---------------------------------------------------------------------------
// Civic pieces

/** A column for a portico, 4 wide, tiled down from its capital (4×3) and down its shaft (4×1). */
export const COLUMN = {
  capital: `
    kkkk
    jkkK
    KKKK`,
  shaft: `jkkK`,
  base: `
    jkkK
    kkkk`,
};

/** A pediment end (the left half, 16×10); the right half is its mirror in the shade colours. */
export const PEDIMENT_HALF = `
    ...............k
    .............kkk
    ...........kkjjj
    .........kkjjwww
    .......kkjjwwwww
    .....kkjjwwwwwww
    ...kkjjwwwwwwwww
    .kkjjwwwwwwwwwww
    kkkkkkkkkkkkkkkk
    KKKKKKKKKKKKKKKK`;

/** A clock turret for the town hall's roof, 14×24 (the bottom row sits on the roof). */
export const TURRET = `
    ......ii......
    ......II......
    .....rrrr.....
    ....rrrrRR....
    ...rrrrrrRR...
    ..qqqqqqqqqq..
    ..kkkkkkkkkK..
    ..jwwwwwwwwK..
    ..jwwssssswK..
    ..jwssspsswK..
    ..jwsssppswK..
    ..jwssssswwK..
    ..jwwssswwwK..
    ..jwwwwwwwwK..
    ..kkkkkkkkkK..
    ..jwwoooowwK..
    ..jwoooooowK..
    ..jwoooooowK..
    ..jwoooooowK..
    ..jwwwwwwwwK..
    .kkkkkkkkkkkK.
    .KKKKKKKKKKKK.
    ..wwwwwwwwwW..
    ..WWWWWWWWWW..`;

/** A dome for the council chamber, 24×18 (the bottom row sits on the roof), drawn in the dome's roof ramp. */
export const DOME = `
    ...........ii...........
    ...........II...........
    ..........qqqq..........
    ........qqrrrrrr........
    ......qqrrrrrrrrRR......
    .....qrrrrrrrrrrrRR.....
    ....qrrrrrrrrrrrrrRR....
    ...qrrrrrrrrrrrrrrrRR...
    ...qrrrrrrrrrrrrrrrRR...
    ..qrrrrrrrrrrrrrrrrrRR..
    ..RRRRRRRRRRRRRRRRRRRR..
    ..kkkkkkkkkkkkkkkkkkkK..
    ..jwoowwoowwoowwoowwwK..
    ..jwoowwoowwoowwoowwwK..
    ..jwwwwwwwwwwwwwwwwwwK..
    ..kkkkkkkkkkkkkkkkkkkK..
    ..KKKKKKKKKKKKKKKKKKKK..
    ..WWWWWWWWWWWWWWWWWWWW..`;

// ---------------------------------------------------------------------------
// Roof clutter

export const CHIMNEY = `
    .iI.Ii.
    .ii.ii.
    kkkkkkK
    KKKKKKK
    wwwwwwW
    wWwwwWW
    wwwwwwW
    wwwWwwW
    WWWWWWW`;

export const VENT = `
    iiiiI
    IIIII
    iiiii
    ooooo`;

export const SKYLIGHT = `
    FFFFFFFF
    FGGyyGGF
    FGyyGGGF
    FFFFFFFF
    oooooooo`;

// ---------------------------------------------------------------------------
// Sign plates: an 11×11 board with a 7×7 pictogram (no letters, R14).

export const SIGN_BOARD = `
    .SSSSSSSSS.
    SsssssssssS
    SsssssssssS
    SsssssssssS
    SsssssssssS
    SsssssssssS
    SsssssssssS
    SsssssssssS
    SsssssssssS
    SsssssssssS
    .SSSSSSSSS.`;

export const PICTOGRAMS: Readonly<Record<string, string>> = {
  civic: `
    ...p...
    ..ppp..
    ppppppp
    .p.p.p.
    .p.p.p.
    .p.p.p.
    ppppppp`,
  dome: `
    ...p...
    ..ppp..
    .ppppp.
    ppppppp
    p.p.p.p
    p.p.p.p
    ppppppp`,
  diner: `
    .p.p...
    p.p....
    .......
    ppppp..
    ppppp.p
    pppppp.
    .ppp...`,
  market: `
    ..ppp..
    .p...p.
    ppppppp
    p.p.p.p
    ppppppp
    .p.p.p.
    .ppppp.`,
  news_office: `
    ppppppp
    p.....p
    p.ppp.p
    p.....p
    p.ppp.p
    p.....p
    ppppppp`,
  party_office: `
    ..ppp..
    .ppppp.
    ppp.ppp
    .ppppp.
    ..ppp..
    .pp.pp.
    pp...pp`,
  union_hall: `
    .pppp..
    ppppp..
    pppppp.
    pppppp.
    .ppppp.
    ..ppp..
    ..ppp..`,
  business_association: `
    ..ppp..
    ..p.p..
    ppppppp
    p.....p
    ppppppp
    p.....p
    ppppppp`,
  clinic: `
    .pp.pp.
    ppppppp
    ppppppp
    ppppppp
    .ppppp.
    ..ppp..
    ...p...`,
  school: `
    .......
    ppp.ppp
    p..p..p
    p..p..p
    p..p..p
    ppppppp
    ...p...`,
  hotel: `
    .......
    p......
    pp.....
    ppppppp
    ppppppp
    p.....p
    p.....p`,
  factory: `
    p......
    p..p..p
    p.pp.pp
    ppppppp
    p.p.p.p
    ppppppp
    ppppppp`,
  home: `
    ...p...
    ..ppp..
    .ppppp.
    ppppppp
    .p...p.
    .p.p.p.
    .ppppp.`,
  park: `
    ..ppp..
    .ppppp.
    ppppppp
    .ppppp.
    ...p...
    ...p...
    .ppppp.`,
  port: `
    ...p...
    ..p.p..
    ...p...
    ppppppp
    ...p...
    p..p..p
    .ppppp.`,
  community_centre: `
    .p...p.
    ppp.ppp
    .p...p.
    ppppppp
    p.....p
    p.ppp.p
    ppppppp`,
  shop: `
    ..ppp..
    .p...p.
    ppppppp
    p.....p
    p.....p
    p.....p
    ppppppp`,
  university: `
    ...p...
    .ppppp.
    ppppppp
    .ppppp.
    ..p.p.p
    ..ppp.p
    ......p`,
  tech_campus: `
    .p.p.p.
    ppppppp
    .p...p.
    pp.p.pp
    .p...p.
    ppppppp
    .p.p.p.`,
  mine: `
    .ppppp.
    p..p..p
    ...p...
    ...p...
    ...p...
    ...p...
    ...p...`,
  mill: `
    p..p..p
    .p.p.p.
    ..ppp..
    ppp.ppp
    ..ppp..
    .p.p.p.
    p..p..p`,
  farm: `
    p.p.p.p
    .ppppp.
    ..ppp..
    ...p...
    ..ppp..
    .p.p.p.
    ...p...`,
};
