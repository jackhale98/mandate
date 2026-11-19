// Diagonal streets (round 5; DEC-090 D5 amended, "A + B"; PIXEL_ART_TOOL.md
// "Round 5: diagonal streets"). Layout v2 lays a 45° street as a staircase of
// cells (Road, Crossing and Pavement codes) and marks them in
// `TownMap.streetAxis`; this draws them as smooth 45° surfaces in the flat
// three-quarter style of rounds 1–4.
//   - Corners: a cell's corner is cut by a 45° line from the middle of one
//     side to the middle of the next when both neighbours toward that corner
//     are another material and a diagonal street is there (the corner's
//     orientation matches a street axis, or a junction). The neighbours'
//     cuts dissolve the staircase's sides, so every edge of the staircase
//     becomes one straight 45° line through the middles of the cell sides;
//     a band of R cells a row is R × 16 px wide across a row (R × 11.3 px
//     square to the street). The material of a map pixel is a function of
//     the map alone, so cells join without seams.
//   - Edges: a 45° edge takes the material's own edge profile, measured
//     down (north edges) or up (south edges) as the faces of rounds 1–4 are:
//     the upper pavement's kerb shows its light top and its face, the road a
//     gutter under it; the lower pavement only its kerb top. Orthogonal
//     edges stay as the autotile draws them, so diagonal pieces meet
//     orthogonal streets, squares, yards and greens with the same kerbs.
//   - Markings: a dashed centre line on diagonal carriageways three or more
//     cells a row wide, a zebra on diagonal crossings (stripes along the
//     street, cut square to it), and the stop lines and barrier posts of a
//     level crossing set into a diagonal road.
// Everything is integer pixel rules over palette indices, deterministic for
// the map. Cells with no diagonal street near them are never drawn here.
import { autotile, canonicalMask, NB, quarterKind } from "../../src/core/autotile";
import { blank, type Indexed } from "../../src/core/image";
import { EDGES, groundTile, type TileSet } from "./tiles";

/** `TownMap.streetAxis` codes (web/src/hub/townLayout.ts `StreetAxis`; a web test checks they agree). */
export const STREET_AXIS = { Orthogonal: 0, DiagonalNE: 1, DiagonalNW: 2, Junction: 3 } as const;
const A = STREET_AXIS;

/** What the street painter reads from the terrain painter. */
export interface StreetHost {
  readonly width: number;
  readonly height: number;
  /** Terrain at a cell (clamped to the map). */
  t(x: number, y: number): number;
  /** Ground level at a cell (clamped). */
  e(x: number, y: number): number;
  /** Street axis at a cell (0 off the map). */
  axis(x: number, y: number): number;
  /** A crossing that carries rail (a level crossing: no zebra). */
  levelCrossing(x: number, y: number): boolean;
}

/** The terrain rules the painter shares (codes come from ./terrain). */
export interface StreetRules {
  group(t: number): number;
  /** Terrain codes this module draws, whose corners may be cut. */
  soft: ReadonlySet<number>;
  /** Terrain codes that may take a neighbour's corner (the soft codes, buildings and doors). */
  joins: ReadonlySet<number>;
  /** The tile set's terrain name per terrain code (what v1 draws for it). */
  tileName(t: number): string;
  /** The tile set's terrain name per material (autotile group). */
  materialName(g: number): string;
  /** Which material keeps a corner where two meet diagonally (a saddle): the higher. */
  rank(g: number): number;
  /** Group code of the carriageway; terrain codes of road and crossing. */
  roadGroup: number;
  road: number;
  crossing: number;
}

// Corners, clockwise from the north-west: the horizontal and vertical neighbour toward each, and the axis whose edges cut it.
const CORNER: readonly { hx: -1 | 1; vy: -1 | 1; axis: number }[] = [
  { hx: -1, vy: -1, axis: A.DiagonalNE },
  { hx: 1, vy: -1, axis: A.DiagonalNW },
  { hx: 1, vy: 1, axis: A.DiagonalNE },
  { hx: -1, vy: 1, axis: A.DiagonalNW },
];
const MIRROR_H = [1, 0, 3, 2];
const MIRROR_V = [3, 2, 1, 0];
const OPPOSITE = [2, 3, 0, 1];
const BITS: readonly { v: number; h: number; d: number }[] = [
  { v: NB.N, h: NB.W, d: NB.NW },
  { v: NB.N, h: NB.E, d: NB.NE },
  { v: NB.S, h: NB.E, d: NB.SE },
  { v: NB.S, h: NB.W, d: NB.SW },
];
const DIRS: readonly (readonly [number, number, number])[] = [
  [0, -1, NB.N],
  [1, 0, NB.E],
  [0, 1, NB.S],
  [-1, 0, NB.W],
  [1, -1, NB.NE],
  [1, 1, NB.SE],
  [-1, 1, NB.SW],
  [-1, -1, NB.NW],
];

/** The quarter of a cell a pixel is in (the corner index). */
const quarterOf = (px: number, py: number) => (py < 8 ? (px < 8 ? 0 : 1) : px < 8 ? 3 : 2);

/**
 * Whether a pixel of a cell lies in its corner's cut: the triangle beyond the
 * 45° line through the middles of the two sides. Pixels on a line belong to
 * its upper side (north-west of "/" lines, north-east of "\" lines), so the
 * two cells a line crosses agree on them.
 */
export function inCut(corner: number, px: number, py: number): boolean {
  switch (corner) {
    case 0:
      return px + py <= 7;
    case 1:
      return px - py >= 8;
    case 2:
      return px + py >= 24;
    default:
      return py - px >= 9;
  }
}

const mod = (a: number, n: number) => ((a % n) + n) % n;

interface Zebra {
  axis: number;
  /** Mean along-street coordinate of the crossing's cells (map px). */
  along: number;
}

export class DiagonalStreets {
  private readonly h: StreetHost;
  private readonly r: StreetRules;
  private readonly ts: TileSet;
  private readonly L: Record<string, number>;
  private readonly W: number;
  private readonly H: number;
  /** Per cell and corner: the material its cut takes, or -1. */
  private readonly cut: Int16Array;
  /** Cells this module draws (diagonal cells, cells beside a cut, zebra zones). */
  private readonly touchedAt: Uint8Array;
  private readonly zebraAt: Int16Array;
  private readonly zebras: Zebra[] = [];
  private readonly profiles = new Map<string, { N: number[]; S: number[] }>();
  /** Whether any cell is marked diagonal (otherwise this draws nothing). */
  readonly active: boolean;

  constructor(host: StreetHost, rules: StreetRules, ts: TileSet, legend: Record<string, number>) {
    this.h = host;
    this.r = rules;
    this.ts = ts;
    this.L = legend;
    this.W = host.width;
    this.H = host.height;
    const n = this.W * this.H;
    this.cut = new Int16Array(n * 4).fill(-1);
    this.touchedAt = new Uint8Array(n);
    this.zebraAt = new Int16Array(n).fill(-1);
    for (const [name, e] of Object.entries(EDGES)) this.profiles.set(name, { N: [...e.N].map((c) => legend[c] as number), S: [...e.S].map((c) => legend[c] as number) });
    let any = false;
    for (let y = 0; y < this.H && !any; y++) for (let x = 0; x < this.W; x++) if (host.axis(x, y) !== 0) any = true;
    this.active = any;
    if (!any) return;
    for (let y = 0; y < this.H; y++) for (let x = 0; x < this.W; x++) for (let k = 0; k < 4; k++) this.cut[(y * this.W + x) * 4 + k] = this.cutOf(x, y, k);
    const touch = (x: number, y: number) => {
      if (x >= 0 && y >= 0 && x < this.W && y < this.H) this.touchedAt[y * this.W + x] = 1;
    };
    for (let y = 0; y < this.H; y++) {
      for (let x = 0; x < this.W; x++) {
        const i = y * this.W + x;
        if (host.axis(x, y) !== 0) touch(x, y);
        if (!this.cut.subarray(i * 4, i * 4 + 4).some((c) => c >= 0)) continue;
        // A cut's edge profile reaches up to 3 px into the cells above and below it.
        for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) touch(x + dx, y + dy);
      }
    }
    this.findZebras(touch);
  }

  /**
   * The cell standing for (x, y): itself on the map; off the map, a diagonal
   * street at the border runs on past it (a "/" street is the same one row up
   * and one column right, a "\\" street one row up and one column left), so a
   * street leaving the map keeps its 45° edges to the border. Anything else
   * clamps to the nearest cell, as the terrain painter does.
   */
  private ext(x: number, y: number): [number, number] {
    const W = this.W;
    const H = this.H;
    if (x >= 0 && y >= 0 && x < W && y < H) return [x, y];
    const cx = Math.max(0, Math.min(W - 1, x));
    const cy = Math.max(0, Math.min(H - 1, y));
    const a = this.h.axis(cx, cy);
    if (a !== A.DiagonalNE && a !== A.DiagonalNW) return [cx, cy];
    // Translate along the street (dx = s * dy) until on the map, then clamp.
    const s = a === A.DiagonalNE ? -1 : 1;
    let k = 0;
    if (y < 0) k = -y;
    else if (y >= H) k = H - 1 - y;
    else if (x < 0) k = -x * s;
    else k = (W - 1 - x) * s;
    return [Math.max(0, Math.min(W - 1, x + s * k)), Math.max(0, Math.min(H - 1, y + k))];
  }

  /** Terrain, ground level and street axis at a cell, off the map as `ext` says. */
  private tt(x: number, y: number): number {
    const [cx, cy] = this.ext(x, y);
    return this.h.t(cx, cy);
  }

  private ee(x: number, y: number): number {
    const [cx, cy] = this.ext(x, y);
    return this.h.e(cx, cy);
  }

  private aa(x: number, y: number): number {
    const [cx, cy] = this.ext(x, y);
    return this.h.axis(cx, cy);
  }

  /** The material a corner's cut takes, or -1 when the corner stays square. */
  private cutOf(x: number, y: number, k: number): number {
    const r = this.r;
    const t = this.tt(x, y);
    if (!r.soft.has(t)) return -1;
    const c = CORNER[k] as (typeof CORNER)[number];
    const hx = x + c.hx;
    const vy = y + c.vy;
    const th = this.tt(hx, y);
    const tv = this.tt(x, vy);
    if (!r.joins.has(th) || !r.joins.has(tv)) return -1;
    const g = r.group(t);
    const gh = r.group(th);
    if (gh !== r.group(tv) || gh === g) return -1;
    const e0 = this.ee(x, y);
    if (this.ee(hx, y) !== e0 || this.ee(x, vy) !== e0) return -1;
    // A diagonal street of this corner's orientation (or a junction) at the corner.
    const fits = (a: number) => a === c.axis || a === A.Junction;
    if (!fits(this.aa(x, y)) && !fits(this.aa(hx, y)) && !fits(this.aa(x, vy)) && !fits(this.aa(hx, vy))) return -1;
    // A saddle (this material again across the corner): the higher-ranked material keeps the corner.
    if (r.group(this.tt(hx, vy)) === g && r.rank(gh) <= r.rank(g)) return -1;
    return gh;
  }

  /** A cell's corner cut (off the map, the cell `ext` stands it for). */
  private cutAt(x: number, y: number, k: number): number {
    const [cx, cy] = this.ext(x, y);
    if (cx !== x || cy !== y) return this.cutOf(x, y, k);
    return this.cut[(y * this.W + x) * 4 + k] as number;
  }

  /** Zebra crossings on diagonal streets: each group of crossing cells, its axis and where along the street it lies. */
  private findZebras(touch: (x: number, y: number) => void) {
    const h = this.h;
    const seen = new Uint8Array(this.W * this.H);
    const isZ = (x: number, y: number) => x >= 0 && y >= 0 && x < this.W && y < this.H && h.t(x, y) === this.r.crossing && h.axis(x, y) !== 0 && !h.levelCrossing(x, y);
    for (let y = 0; y < this.H; y++) {
      for (let x = 0; x < this.W; x++) {
        if (seen[y * this.W + x] || !isZ(x, y)) continue;
        const cells: [number, number][] = [];
        const stack: [number, number][] = [[x, y]];
        seen[y * this.W + x] = 1;
        while (stack.length) {
          const [cx, cy] = stack.pop() as [number, number];
          cells.push([cx, cy]);
          for (const [dx, dy] of DIRS) {
            const nx = cx + dx;
            const ny = cy + dy;
            if (isZ(nx, ny) && !seen[ny * this.W + nx]) {
              seen[ny * this.W + nx] = 1;
              stack.push([nx, ny]);
            }
          }
        }
        let ne = 0;
        let nw = 0;
        for (const [cx, cy] of cells) {
          const a = h.axis(cx, cy);
          if (a === A.DiagonalNE) ne++;
          else if (a === A.DiagonalNW) nw++;
        }
        // A crossing only of junction cells takes the axis of the street beside it.
        if (ne === 0 && nw === 0) {
          for (const [cx, cy] of cells) {
            for (const [dx, dy] of DIRS) {
              const a = h.axis(cx + dx, cy + dy);
              if (a === A.DiagonalNE) ne++;
              else if (a === A.DiagonalNW) nw++;
            }
          }
        }
        if (ne === 0 && nw === 0) continue;
        const axis = nw >= ne ? A.DiagonalNW : A.DiagonalNE;
        let sum = 0;
        for (const [cx, cy] of cells) sum += axis === A.DiagonalNW ? 16 * (cx + cy) + 16 : 16 * (cx - cy);
        const id = this.zebras.length;
        this.zebras.push({ axis, along: Math.round(sum / cells.length) });
        for (const [cx, cy] of cells) {
          for (let dy = -1; dy <= 1; dy++) {
            for (let dx = -1; dx <= 1; dx++) {
              const nx = cx + dx;
              const ny = cy + dy;
              if (nx < 0 || ny < 0 || nx >= this.W || ny >= this.H) continue;
              if ((this.zebraAt[ny * this.W + nx] as number) < 0) this.zebraAt[ny * this.W + nx] = id;
              touch(nx, ny);
            }
          }
        }
      }
    }
  }

  /** Whether this module draws the cell (a soft terrain with a diagonal street at or beside it). */
  draws(x: number, y: number): boolean {
    return this.active && this.touched(x, y) && this.r.soft.has(this.h.t(x, y));
  }

  /** Whether a diagonal street changes this cell or its edges (never "simple"). */
  touched(x: number, y: number): boolean {
    return this.active && x >= 0 && y >= 0 && x < this.W && y < this.H && this.touchedAt[y * this.W + x] === 1;
  }

  /** The material (autotile group) at a map pixel. */
  mat(X: number, Y: number): number {
    const [cx, cy] = this.ext(Math.floor(X / 16), Math.floor(Y / 16));
    const px = mod(X, 16);
    const py = mod(Y, 16);
    const k = quarterOf(px, py);
    const c = this.cut[(cy * this.W + cx) * 4 + k] as number;
    if (c >= 0 && inCut(k, px, py)) return c;
    return this.r.group(this.h.t(cx, cy));
  }

  /** Whether a map pixel is in a quarter whose corner is cut (where 45° edges run). */
  cutQuarter(X: number, Y: number): boolean {
    const [cx, cy] = this.ext(Math.floor(X / 16), Math.floor(Y / 16));
    return (this.cut[(cy * this.W + cx) * 4 + quarterOf(mod(X, 16), mod(Y, 16))] as number) >= 0;
  }

  private mask(x: number, y: number, g: number): number {
    let m = 0;
    for (const [dx, dy, bit] of DIRS) if (this.r.group(this.tt(x + dx, y + dy)) === g) m |= bit;
    return m;
  }

  /** The cell's neighbour mask for one quarter, counting a neighbour whose touching corner is cut to this material as joined. */
  private quarterMask(x: number, y: number, g: number, real: number, k: number): number {
    const c = CORNER[k] as (typeof CORNER)[number];
    const b = BITS[k] as (typeof BITS)[number];
    let m = real;
    if (!(m & b.h) && this.cutAt(x + c.hx, y, MIRROR_H[k] as number) === g) m |= b.h;
    if (!(m & b.v) && this.cutAt(x, y + c.vy, MIRROR_V[k] as number) === g) m |= b.v;
    if (!(m & b.d) && this.cutAt(x + c.hx, y + c.vy, OPPOSITE[k] as number) === g) m |= b.d;
    return m;
  }

  private fillOf(name: string, variant: number): Indexed {
    const t = this.ts.terrains[name] ?? (this.ts.terrains.grass as { template: Indexed; fills: Indexed[] });
    return (t.fills.length > 1 && variant >> 1 === 3 ? t.fills[1 + ((variant & 1) % (t.fills.length - 1))] : t.fills[0]) as Indexed;
  }

  /** The cell (16×16 palette indices). Only for cells `draws()` accepts. */
  draw(x: number, y: number, variant: number): Indexed {
    const h = this.h;
    const r = this.r;
    const t = h.t(x, y);
    const g = r.group(t);
    const name = r.materialName(g);
    const tset = this.ts.terrains[name] ?? (this.ts.terrains.grass as { template: Indexed; fills: Indexed[] });
    const pick = this.fillOf(name, variant);
    const real = this.mask(x, y, g);
    const axis = h.axis(x, y);
    // Orthogonal cells keep their tile (decals and all); diagonal cells take the plain autotile (no orthogonal markings).
    // A level crossing's road carries no zebra.
    const tile = t === r.crossing && h.levelCrossing(x, y) ? name : r.tileName(t);
    const base = axis === A.Orthogonal ? groundTile(this.ts, tile, real, variant) : autotile(tset.template, canonicalMask(real), pick);
    const out = blank(16, 16);
    out.px.set(base.px);
    const X0 = x * 16;
    const Y0 = y * 16;
    for (let k = 0; k < 4; k++) {
      const qx = k === 1 || k === 2 ? 1 : 0;
      const qy = k >= 2 ? 1 : 0;
      if (this.cutAt(x, y, k) >= 0) {
        // A cut quarter: this material's fill on its side of the line, the other material's beyond it.
        for (let py = qy * 8; py < qy * 8 + 8; py++) {
          for (let px = qx * 8; px < qx * 8 + 8; px++) {
            const m = this.mat(X0 + px, Y0 + py);
            const f = m === g ? pick : this.fillOf(r.materialName(m), 0);
            out.px[py * 16 + px] = f.px[py * 16 + px] as number;
          }
        }
        continue;
      }
      const q = this.quarterMask(x, y, g, real, k);
      if (quarterKind(canonicalMask(q), qx as 0 | 1, qy as 0 | 1) === quarterKind(canonicalMask(real), qx as 0 | 1, qy as 0 | 1)) continue;
      // A side the neighbours' cuts dissolved: the quarter as if that neighbour were this material.
      const alt = autotile(tset.template, canonicalMask(q), pick);
      for (let py = qy * 8; py < qy * 8 + 8; py++) for (let px = qx * 8; px < qx * 8 + 8; px++) out.px[py * 16 + px] = alt.px[py * 16 + px] as number;
    }
    this.edges(out, X0, Y0);
    this.markings(out, x, y, axis);
    return out;
  }

  /** 45° edges: each material's north profile under a 45° edge above it, its south profile over one below (south wins, as in the templates). */
  private edges(out: Indexed, X0: number, Y0: number) {
    for (let py = 0; py < 16; py++) {
      for (let px = 0; px < 16; px++) {
        const X = X0 + px;
        const Y = Y0 + py;
        const M = this.mat(X, Y);
        const prof = this.profiles.get(this.r.materialName(M));
        if (!prof) continue;
        let col = -1;
        for (let k = 1; k <= prof.N.length; k++) {
          if (this.mat(X, Y - k) === M) continue;
          if (this.cutQuarter(X, Y - k) || this.cutQuarter(X, Y - k + 1)) col = prof.N[k - 1] as number;
          break;
        }
        for (let k = 1; k <= prof.S.length; k++) {
          if (this.mat(X, Y + k) === M) continue;
          if (this.cutQuarter(X, Y + k) || this.cutQuarter(X, Y + k - 1)) col = prof.S[k - 1] as number;
          break;
        }
        if (col > 0) out.px[py * 16 + px] = col;
      }
    }
  }

  /** The run of carriageway along a pixel row through X: [first, last] map px (scanning at most `reach` px each way). */
  private run(X: number, Y: number, reach = 128): [number, number] {
    const road = this.r.roadGroup;
    let a = X;
    while (a > X - reach && this.mat(a - 1, Y) === road) a--;
    let b = X;
    while (b < X + reach && this.mat(b + 1, Y) === road) b++;
    return [a, b];
  }

  /** Centre dashes on diagonal carriageways, zebras on diagonal crossings. */
  private markings(out: Indexed, x: number, y: number, axis: number) {
    const L = this.L;
    const road = this.r.roadGroup;
    const t = this.h.t(x, y);
    const zi = this.zebraAt[y * this.W + x] as number;
    const z = zi >= 0 ? this.zebras[zi] : undefined;
    const diagonal = axis === A.DiagonalNE || axis === A.DiagonalNW;
    if (!z && !(diagonal && t === this.r.road)) return;
    const X0 = x * 16;
    const Y0 = y * 16;
    for (let py = 0; py < 16; py++) {
      const Y = Y0 + py;
      let span: [number, number] | null = null;
      for (let px = 0; px < 16; px++) {
        const X = X0 + px;
        if (this.mat(X, Y) !== road) continue;
        if (!span || X < span[0] || X > span[1]) span = this.run(X, Y);
        const [a, b] = span;
        if (z) {
          const along = z.axis === A.DiagonalNW ? X + Y : X - Y;
          const d = Math.abs(along - z.along);
          if (d <= 11) {
            // Stripes along the street, 3 px a row apart, kept 2 px off the kerbs; cut square to the street.
            const across = z.axis === A.DiagonalNW ? X - Y : X + Y;
            const clear = this.mat(X - 2, Y) === road && this.mat(X + 2, Y) === road && this.mat(X, Y - 2) === road && this.mat(X, Y + 2) === road;
            if (clear && mod(across, 6) < 3) out.px[py * 16 + px] = (d >= 10 ? L.m : L.M) as number;
            continue;
          }
          if (d <= 24) continue;
        }
        if (!diagonal || t !== this.r.road) continue;
        // The centre line: carriageways three or more cells a row wide (two lanes), dashed along the street.
        const w = b - a + 1;
        if (w < 44 || w > 112 || Math.abs(2 * X - (a + b)) > 1) continue;
        // Only where the carriageway runs on at 45° above and below (not at its ends, at junctions or bridges).
        const s = axis === A.DiagonalNW ? 1 : -1;
        const up = this.run(X - s, Y - 1);
        const down = this.run(X + s, Y + 1);
        if (up[0] !== a - s || up[1] !== b - s || down[0] !== a + s || down[1] !== b + s) continue;
        const along = mod(axis === A.DiagonalNW ? X + Y : X - Y, 64);
        if (along < 14) out.px[py * 16 + px] = (along <= 1 || along >= 12 ? L.m : L.M) as number;
      }
    }
  }

  /** A level crossing set into a diagonal road: stop lines only where carriageway meets it, posts at the verge's corners. */
  levelCrossingMarks(img: Indexed, x: number, y: number, railEW: boolean) {
    const L = this.L;
    const h = this.h;
    const road = this.r.roadGroup;
    const put = (px: number, py: number, c: string) => (img.px[py * 16 + px] = L[c] as number);
    const X0 = x * 16;
    const Y0 = y * 16;
    for (let k = 0; k < 16; k++) {
      if (k % 4 === 3) continue;
      if (railEW) {
        if (h.t(x, y - 1) !== this.r.crossing && this.mat(X0 + k, Y0 - 1) === road) put(k, 0, "M");
        if (h.t(x, y + 1) !== this.r.crossing && this.mat(X0 + k, Y0 + 16) === road) put(k, 15, "M");
      } else {
        if (h.t(x - 1, y) !== this.r.crossing && this.mat(X0 - 1, Y0 + k) === road) put(0, k, "M");
        if (h.t(x + 1, y) !== this.r.crossing && this.mat(X0 + 16, Y0 + k) === road) put(15, k, "M");
      }
    }
    // Barrier posts where the crossing's end meets the verge on the side the road does not continue.
    const notCrossing = (dx: number, dy: number) => h.t(x + dx, y + dy) !== this.r.crossing;
    const verge = (dx: number, dy: number) => this.r.group(h.t(x + dx, y + dy)) !== road;
    const corners: [number, number, boolean][] = railEW
      ? [
          [0, 0, notCrossing(-1, 0) && verge(0, -1)],
          [14, 0, notCrossing(1, 0) && verge(0, -1)],
          [0, 12, notCrossing(-1, 0) && verge(0, 1)],
          [14, 12, notCrossing(1, 0) && verge(0, 1)],
        ]
      : [
          [0, 0, notCrossing(0, -1) && verge(-1, 0)],
          [14, 0, notCrossing(0, -1) && verge(1, 0)],
          [0, 12, notCrossing(0, 1) && verge(-1, 0)],
          [14, 12, notCrossing(0, 1) && verge(1, 0)],
        ];
    for (const [cx, cy, on] of corners) {
      if (!on) continue;
      put(cx, cy, "R");
      put(cx + 1, cy, "R");
      put(cx, cy + 1, "c");
      put(cx + 1, cy + 1, "c");
      put(cx, cy + 2, "R");
      put(cx + 1, cy + 2, "R");
      put(cx, cy + 3, "O");
      put(cx + 1, cy + 3, "O");
    }
  }
}
