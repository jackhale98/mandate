// Ground tiles for the P0 spike (16×16), drawn by hand as text grids: seamless
// fills with variants, and edge profiles that ./src/core/edges expands into
// autotile templates. Road markings and crossings are decals chosen by the
// neighbour mask. Built ahead of time (D10); the provider assembles each
// cell's quarters from the template.
//
// Legend (ramp and step; '.' is never used: tiles are opaque):
//   a A b    asphalt base, dark, light          p P Q q  paving base, shade, deep, light
//   s S T t  stone base, shade, deep, light     g G H h  grass base, shade, deep, light
//   n N      foliage base, shade                e E      earth base, light
//   f F      field base, light                  w W v    water base, dark, light
//   d D k    wood base, dark, light              m M      chalk (markings) dark, light
//   r y c    flowers: flag, lamp light, chalk    i        iron
import { autotile, canonicalMask } from "../../src/core/autotile";
import { edgeTemplate, type EdgeProfile } from "../../src/core/edges";
import { blit, grid, type Indexed } from "../../src/core/image";
import { ramp, type Palette } from "../../src/core/palette";

/** Legend characters to (ramp, step offset from the anchor). */
export const TILE_LEGEND_RAMPS: Readonly<Record<string, readonly [string, number]>> = {
  a: ["asphalt", 0],
  A: ["asphalt", -1],
  b: ["asphalt", 1],
  p: ["paving", 0],
  P: ["paving", -1],
  Q: ["paving", -2],
  q: ["paving", 1],
  s: ["stone", 0],
  S: ["stone", -1],
  T: ["stone", -2],
  t: ["stone", 1],
  g: ["grass", 0],
  G: ["grass", -1],
  H: ["grass", -2],
  h: ["grass", 1],
  n: ["foliage", 0],
  N: ["foliage", -1],
  e: ["earth", 0],
  E: ["earth", 1],
  f: ["field", 0],
  F: ["field", 1],
  w: ["water", 0],
  W: ["water", -1],
  v: ["water", 1],
  d: ["wood", 0],
  D: ["wood", -1],
  k: ["wood", 1],
  m: ["chalk", -1],
  M: ["chalk", 0],
  r: ["flag", 1],
  y: ["lamp", 1],
  c: ["chalk", 0],
  i: ["iron", 0],
};

/** The legend as palette indices. */
export function legendFor(p: Palette, ramps: Readonly<Record<string, readonly [string, number]>>): Record<string, number> {
  const out: Record<string, number> = {};
  for (const [ch, [name, k]] of Object.entries(ramps)) {
    const r = ramp(p, name);
    out[ch] = r.steps[Math.min(r.steps.length - 1, Math.max(0, r.anchorStep + k))] as number;
  }
  return out;
}

// ---------------------------------------------------------------------------
// Fills (each a seamless 16×16; several variants per terrain)

const FILLS: Record<string, readonly string[]> = {
  pavement: [
    `
    PPPPPPPPPPPPPPPP
    PqqpppppPqpppppp
    Pqpppppppppppppp
    PpppppppPppppppp
    PpppppppPppppppp
    PpppppppPppppppp
    PpppppppPppppppp
    PpppppppPppppppp
    PPPPPPPPPPPPPPPP
    ppppPqqpppppPqpp
    ppppPqppppppPppp
    ppppPpppppppPppp
    ppppPpppppppPppp
    ppppPpppppppPppp
    ppppPpppppppPppp
    ppppPpppppppPppp`,
    `
    PPPPPPPPPPPPPPPP
    PqqpppppPqqqqqqq
    PqpppppppqqqqqqQ
    PpppppppPqqqqqqQ
    PpppppppPqqqqqqQ
    PpppppppPqqqqqqQ
    PpppppppPqqqqqqQ
    PpppppppPqQQQQQQ
    PPPPPPPPPPPPPPPP
    ppppPqqpppppPqpp
    ppppPqppppppPppp
    ppppPpppppppPppp
    ppppPpppppppPppp
    ppppPpppppppPppp
    ppppPpppppppPppp
    ppppPpppppppPppp`,
    `
    PPPPPPPPPPPPPPPP
    PqqpppppPqpppppp
    PqpppppppppppPpp
    PpppppppPppppPpp
    PpppppppPpppPppp
    PpppppppPpppPppp
    PpppppppPppPpppp
    PpppppppPppppppp
    PPPPPPPPPPPPPPPP
    ppppPqqpppppPqpp
    ppppPqppppppPppp
    ppppPpppppppPppp
    ppppPpppppppPppp
    ppppPpppppppPppp
    ppppPpppppppPppp
    ppppPpppppppPppp`,
  ],
  road: [
    `
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaaaaaaa
    aaaAAaaaaaaaaaaa
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaabbaaa
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaaaaaaa
    aaaaaaAAaaaaaaaa
    aaaaaaaaaaaaaaaa`,
    `
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaaaaaaa
    aaaAAAAAAaaaaaaa
    aaaAaaaaaAaaaaaa
    aaaAaaaaaAaabaaa
    aaaAaaaaaAaabbaa
    aaaAAAAAAAaaaaaa
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaaaaaaa
    aabbaaaaaaaaaaaa
    aaaaaaaaaaAAaaaa
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaaaaaaa`,
    `
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaaaaaaa
    aaaaaaaaaaaaaaaa
    aaaaaaaiiiaaaaaa
    aaaaaaibbbiaaaaa
    aaaaaiaiaibiaaaa
    aaaaaiaiaibiaaaa
    aaaaaiaaaaaiaaaa
    aaaaaaiaaaiaaaaa
    aaaaaaaiiiaaaaaa
    aaaaaaaaaaaaaaaa
    aabbaaaaaaaaaaaa
    aaaaaaaaaaaaaAaa
    aaaaaaaaaaaaAAaa
    aaaaaaaaaaaaaaaa`,
  ],
  grass: [
    `
    gggggggggggggggg
    gggggggghggggggg
    gGggggghhggggggg
    gGGggggggggggggg
    gggggggggggghggg
    ggggggggggghhggg
    gggggGgggggggggg
    ggggGGgggggggggg
    gggggggggggggGgg
    ghgggggggggggGGg
    hhgggggggggggggg
    ggggggggghgggggg
    gggggggghhgggggg
    gggGgggggggggggg
    gggGGggggggggggg
    gggggggggggggggg`,
    `
    gggggggggggggggg
    ggggggggggggGggg
    gggghgggggggGGgg
    ggghhggggggggggg
    gggggggggggggggg
    ggggggggggcggggg
    gGgggggggcGcgggg
    gGGggggggggGgggg
    gggggggggggggggg
    gggggghggggggggg
    ggggghhgggggggGg
    ggggggggggggggGG
    gggggggggggggggg
    ghgggggggggGgggg
    hhgggggggggGGggg
    gggggggggggggggg`,
    `
    gggggggggggggggg
    gggghggggggggggg
    ggghhggggggrgggg
    gggggggggggrGggg
    gggggggggggggggg
    gggggGgggggggggg
    ggggGGggggggghgg
    ggggggggggggghhg
    gggggggggggggggg
    ggyggggggggggggg
    ggyGgggggggGgggg
    gggggggggggGGggg
    gggggggggggggggg
    gggggggghggggggg
    gggggggghhgggggg
    gggggggggggggggg`,
  ],
  plaza: [
    `
    SSSSSSSSSSSSSSSS
    SttsSttsSttsStts
    SsssSsssSsssSsss
    SsssSsssSsssSsss
    SSSSSSSSSSSSSSSS
    ssSttsSttsSttsSt
    ssSsssSsssSsssSs
    ssSsssSsssSsssSs
    SSSSSSSSSSSSSSSS
    SttsSttsSttsStts
    SsssSsssSsssSsss
    SsssSsssSsssSsss
    SSSSSSSSSSSSSSSS
    ssSttsSttsSttsSt
    ssSsssSsssSsssSs
    ssSsssSsssSsssSs`,
    `
    SSSSSSSSSSSSSSSS
    SttsSttsSttsStts
    SsssSsssSsssSsss
    SsssSsssSsssSsss
    SSSSSSSSSSSSSSSS
    ssSttsSttsSttsSt
    ssSsssSTTTSsssSs
    ssSsssSTTTSsssSs
    SSSSSSSSSSSSSSSS
    SttsSttsSttsStts
    SsssSsssSsssSsss
    SsssSsssSsssSsss
    SSSSSSSSSSSSSSSS
    ssSttsSttsSttsSt
    ssSsssSsssSsssSs
    ssSsssSsssSsssSs`,
  ],
  park: [
    `
    GGGGGGGGGGGGGGGG
    GGGGGGGGgGGGGGGG
    GGGGGGGggGGGGGGG
    GGGGGGGGGGGGGGGG
    GGGHGGGGGGGGGGGG
    GGHHGGGGGGGGgGGG
    GGGGGGGGGGGggGGG
    GGGGGGGGGGGGGGGG
    GGGGGGGGGGGGGGGG
    GgGGGGGGGGHGGGGG
    ggGGGGGGGHHGGGGG
    GGGGGGGGGGGGGGGG
    GGGGGGgGGGGGGGGG
    GGGGGggGGGGGGHGG
    GGGGGGGGGGGGHHGG
    GGGGGGGGGGGGGGGG`,
  ],
  garden: [
    `
    gggggggggggggggg
    ggggggggggrggggg
    gggggggggrGrgggg
    ggcggggggggGgggg
    gcGcgggggggggggg
    gggGgggggggggggg
    gggggggghggggggg
    ggggggghhggggggg
    gggggggggggggggg
    gggggggggggggygg
    ggyggggggggGyGgg
    gyGygggggggggggg
    gggggggggggggggg
    gggggggggggggggg
    gggggggggggggggg
    gggggggggggggggg`,
  ],
  path: [
    `
    EEEEEEEEEEEEEEEE
    EEEEEEEEEEEeEEEE
    EEEeeEEEEEEEEEEE
    EEEEEEEEEEEEEEEE
    EEEEEEEEEEEEEEEE
    EEEEEEEEeEEEEEEE
    EEEEEEEEeeEEEEEE
    EEEEEEEEEEEEEEEE
    EeEEEEEEEEEEEEEE
    EEEEEEEEEEEEEeeE
    EEEEEEEEEEEEEEEE
    EEEEEEEEEEEEEEEE
    EEEEEEeeEEEEEEEE
    EEEEEEEEEEEEEEEE
    EEEEEEEEEEEEEEEE
    EEEEEEEEEEEEEEEE`,
  ],
  yard: [
    `
    PPPPPPPPPPPPPPPP
    PpppppppPppppppp
    PpppppppPppppppp
    PpppppppPppppppp
    PpppQpppPppppppp
    PpppQQppPppppppp
    PpppppppPppppppp
    PpppppppPppppppp
    PPPPPPPPPPPPPPPP
    PpppppppPppppppp
    PpppppppPppppQpp
    PpppppppPpppQQpp
    PpppppppPppppppp
    PpppppppPppppppp
    PpppppppPppppppp
    PpppppppPppppppp`,
  ],
  dirt: [
    `
    eeeeeeeeeeeeeeee
    eeeeeeeeeEeeeeee
    eeEEeeeeeeeeeeee
    eeeeeeeeeeeeeeee
    eeeeeeeeeeeeeEEe
    eeeeeEeeeeeeeeee
    eeeeeeeeeeeeeeee
    eeeeeeeeeeeeeeee
    eEeeeeeeeeEEeeee
    eeeeeeeeeeeeeeee
    eeeeeeeeeeeeeeee
    eeeeeeEEeeeeeeee
    eeeeeeeeeeeeeeEe
    eeeeeeeeeeeeeeee
    eeeEeeeeeeeeeeee
    eeeeeeeeeeeeeeee`,
  ],
  field: [
    `
    ffffffffffffffff
    FFFFFFFFFFFFFFFF
    ffffffffffffffff
    eeeeeeeeeeeeeeee
    ffffffffffffffff
    FFFFFFFFFFFFFFFF
    ffffffffffffffff
    eeeeeeeeeeeeeeee
    ffffffffffffffff
    FFFFFFFFFFFFFFFF
    ffffffffffffffff
    eeeeeeeeeeeeeeee
    ffffffffffffffff
    FFFFFFFFFFFFFFFF
    ffffffffffffffff
    eeeeeeeeeeeeeeee`,
  ],
  sand: [
    `
    tttttttttttttttt
    tttttttttttstttt
    ttsstttttttttttt
    tttttttttttttttt
    tttttttttttttttt
    tttttttttsstttts
    tttttttttttttttt
    tttttttttttttttt
    tstttttttttttttt
    tttttttttttttttt
    tttttttssttttttt
    tttttttttttttttt
    tttttttttttttttt
    ttttttttttttstts
    tttttttttttttttt
    tttttttttttttttt`,
  ],
  water: [
    `
    wwwwwwwwwwwwwwww
    wwwwwwwwwwwwwwww
    wwwvvvwwwwwwwwww
    wwwwwwwwwwwwwwww
    wwwwwwwwwwWWWwww
    wwwwwwwwwwwwwwww
    wwwwwwwwwwwwwwww
    wwwwwwwwwwwwwwww
    wwwwwwwwvvvwwwww
    wwwwwwwwwwwwwwww
    wWWWwwwwwwwwwwww
    wwwwwwwwwwwwwwww
    wwwwwwwwwwwwwwww
    wwwwwwwwwwwwvvvw
    wwwwwwwwwwwwwwww
    wwwwwwwwwwwwwwww`,
  ],
  quay: [
    `
    SSSSSSSSSSSSSSSS
    sssssssSssssssss
    sssssssSssssssss
    sssssssSssssssss
    SSSSSSSSSSSSSSSS
    sssSssssssssSsss
    sssSssssssssSsss
    sssSssssssssSsss
    SSSSSSSSSSSSSSSS
    sssssssSssssssss
    sssssssSssssssss
    sssssssSssssssss
    SSSSSSSSSSSSSSSS
    sssSssssssssSsss
    sssSssssssssSsss
    sssSssssssssSsss`,
  ],
  pier: [
    `
    kkkkkkkkkkkkkkkk
    dddddddddddddddd
    dddddddddddddddd
    DDDDDDDDDDDDDDDD
    kkkkkkkkkkkkkkkk
    dddddddddddddddd
    dddddddddddddddd
    DDDDDDDDDDDDDDDD
    kkkkkkkkkkkkkkkk
    dddddddddddddddd
    dddddddddddddddd
    DDDDDDDDDDDDDDDD
    kkkkkkkkkkkkkkkk
    dddddddddddddddd
    dddddddddddddddd
    DDDDDDDDDDDDDDDD`,
  ],
};

/** Edge profiles, outside in, as legend characters ('' for none). */
export const EDGES: Readonly<Record<string, { N: string; S: string; W: string; E: string }>> = {
  // Kerbs: a light kerb top, and on the south side its dark face toward the road.
  pavement: { N: "Pq", S: "QQq", W: "Pq", E: "Pq" },
  // A gutter shadow under the kerbs.
  road: { N: "AA", S: "A", W: "A", E: "A" },
  grass: { N: "H", S: "HG", W: "G", E: "G" },
  plaza: { N: "TS", S: "TS", W: "TS", E: "TS" },
  park: { N: "H", S: "HH", W: "H", E: "H" },
  garden: { N: "G", S: "HG", W: "G", E: "G" },
  path: { N: "e", S: "e", W: "e", E: "e" },
  yard: { N: "Q", S: "QQ", W: "Q", E: "Q" },
  dirt: { N: "", S: "", W: "", E: "" },
  field: { N: "e", S: "e", W: "e", E: "e" },
  sand: { N: "s", S: "s", W: "s", E: "s" },
  // Water: a light line where it meets the shore, a dark one under the quay.
  water: { N: "WW", S: "v", W: "v", E: "v" },
  quay: { N: "t", S: "TT", W: "T", E: "T" },
  pier: { N: "D", S: "DD", W: "D", E: "D" },
};

export const TERRAINS = Object.keys(FILLS);

/** Decals over road tiles (centre lines and zebra stripes). */
const DECALS = {
  dashH: `
    ................
    ................
    ................
    ................
    ................
    ................
    ................
    ................
    ................
    ................
    ................
    ................
    ................
    ................
    ................
    ...mMMMMMMMm....`,
  dashV: `
    ...............m
    ...............M
    ...............M
    ...............M
    ...............M
    ...............M
    ...............M
    ...............m
    ................
    ................
    ................
    ................
    ................
    ................
    ................
    ................`,
  zebraRows: `
    ................
    .MMMMMMMMMMMMMM.
    .mMMMMMMMMMMMMm.
    ................
    ................
    .MMMMMMMMMMMMMM.
    .mMMMMMMMMMMMMm.
    ................
    ................
    .MMMMMMMMMMMMMM.
    .mMMMMMMMMMMMMm.
    ................
    ................
    .MMMMMMMMMMMMMM.
    .mMMMMMMMMMMMMm.
    ................`,
  zebraCols: `
    ................
    .Mm..Mm..Mm..Mm.
    .MM..MM..MM..MM.
    .MM..MM..MM..MM.
    .MM..MM..MM..MM.
    .MM..MM..MM..MM.
    .MM..MM..MM..MM.
    .MM..MM..MM..MM.
    .MM..MM..MM..MM.
    .MM..MM..MM..MM.
    .MM..MM..MM..MM.
    .MM..MM..MM..MM.
    .MM..MM..MM..MM.
    .MM..MM..MM..MM.
    .mm..mm..mm..mm.
    ................`,
};

export interface TileSet {
  /** Per terrain: the autotile template (32×48) and its fill variants (16×16), palette indices. */
  terrains: Record<string, { template: Indexed; fills: Indexed[] }>;
  decals: Record<keyof typeof DECALS, Indexed>;
}

/** Build every template, fill and decal (ahead of time). */
export function buildTiles(p: Palette): TileSet {
  const legend = { ...legendFor(p, TILE_LEGEND_RAMPS) };
  const legendDots = { ...legend };
  const terrains: TileSet["terrains"] = {};
  for (const name of TERRAINS) {
    const fills = (FILLS[name] as readonly string[]).map((t) => grid(t, legend));
    const e = EDGES[name] as { N: string; S: string; W: string; E: string };
    const prof = (s: string) => [...s].map((ch) => legend[ch] as number);
    const profile: EdgeProfile = { N: prof(e.N), S: prof(e.S), W: prof(e.W), E: prof(e.E) };
    terrains[name] = { template: edgeTemplate(fills[0] as Indexed, profile), fills };
  }
  const decals = Object.fromEntries(Object.entries(DECALS).map(([k, t]) => [k, grid(t, legendDots)])) as TileSet["decals"];
  return { terrains, decals };
}

const N = 1;
const E = 2;
const S = 4;
const W = 8;

/**
 * One ground tile: the terrain's template for the neighbour mask, the
 * variant's fill where the tile is plain, and road decals by the mask.
 * `crossing` and `road` share the road template.
 */
export function groundTile(ts: TileSet, kind: string, mask: number, variant: number): Indexed {
  const base = kind === "crossing" ? "road" : kind;
  const t = ts.terrains[base] ?? (ts.terrains.grass as { template: Indexed; fills: Indexed[] });
  // Most cells use the first fill; a quarter use the others, so a street is varied without looking busy.
  const pickFill = (t.fills.length > 1 && variant >> 1 === 3 ? t.fills[1 + ((variant & 1) % (t.fills.length - 1))] : t.fills[0]) as Indexed;
  const img = autotile(t.template, canonicalMask(mask), pickFill);
  if (base === "road") {
    const horizontal = (mask & (E | W)) !== 0 && !((mask & N) !== 0 && (mask & S) !== 0 && !((mask & E) !== 0 && (mask & W) !== 0));
    if (kind === "crossing") {
      const vertical = (mask & N) !== 0 && (mask & S) !== 0 && !((mask & E) !== 0 && (mask & W) !== 0);
      blit(img, vertical ? ts.decals.zebraCols : ts.decals.zebraRows, 0, 0);
    } else if (horizontal && mask & S && !(mask & N)) {
      // The centre line runs along the bottom of the upper lane, every other tile.
      if (variant % 2 === 0) blit(img, ts.decals.dashH, 0, 0);
    } else if (horizontal && mask & N && !(mask & S)) {
      if (variant % 2 === 1) blit(img, ts.decals.dashH, 0, -15);
    } else if (!horizontal && mask & E && !(mask & W)) {
      if (variant % 2 === 0) blit(img, ts.decals.dashV, 0, 0);
    } else if (!horizontal && mask & W && !(mask & E)) {
      if (variant % 2 === 1) blit(img, ts.decals.dashV, -15, 0);
    }
  }
  return img;
}
