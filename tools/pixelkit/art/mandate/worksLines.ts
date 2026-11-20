// Works that run along a line (art round 8, DEC-099): site fencing (Heras
// mesh panels and painted hoarding), road barriers, new bridge railings,
// flood walls, levees and sheet piling. They are laid on the street grid's
// own lines: a piece joins two of a cell's mid-sides (or a side and the
// cell's centre), so its ends sit where round 5's 45° streets put their
// edges (lines through the middles of cell sides) and every piece meets its
// neighbour at the same point. Straight runs go west–east or north–south,
// 45° runs cut a cell's corner (north to east is a "\", west to north a
// "/"), and right-angle corners go through the centre.
//
// A run is drawn as one heightfield (three-quarter view, light from the
// upper left): each footprint pixel stands a column up the screen, painted
// back to front, so a wall shows its lit coping on top and the face that
// looks at the viewer below it (a "\" run's face looks south-west into the
// light, a "/" run's south-east into the shade). Thin runs (fences, barriers,
// railings) have a 1 px footprint and a pattern along the run; thick ones
// (walls 3 px, levees 9 px with sloping sides) a band. Positions along the
// run come from map pixels, so posts and joints line up across cells. The
// image is then cut into one sprite per cell (its depth that cell's bottom
// row), so people walk in front of and behind each piece as with props.
import { hash32 } from "../../src/core/hash";
import { blank, type Indexed } from "../../src/core/image";
import type { Palette } from "../../src/core/palette";
import { outlineExterior, worksLegend } from "./worksParts";

export type Side = "n" | "e" | "s" | "w";
/** A piece of a run in one cell: from mid-side `a` to mid-side `b` (or the centre), straight across or round a sharp corner through the centre. */
export interface LinePiece {
  x: number;
  y: number;
  a: Side;
  b: Side | "c";
  /** A right-angle corner through the cell's centre instead of a 45° cut (a and b must be adjacent sides). */
  sharp?: boolean;
}

export const LINE_STYLES = ["heras", "hoarding", "barrier", "railing", "wall", "levee", "levee_bare", "piles"] as const;
export type LineStyle = (typeof LINE_STYLES)[number];

/** Direction of a run at a pixel: 0 west–east, 1 north–south, 2 "/" (rising to the east), 3 "\" (falling to the east). */
export type Dir = 0 | 1 | 2 | 3;

/** A pixel on a run's centre line (map px) and the piece it belongs to. */
export interface CentrePx {
  X: number;
  Y: number;
  dir: Dir;
  piece: number;
}

const MID: Readonly<Record<Side | "c", readonly [number, number]>> = { n: [8, 0], e: [16, 8], s: [8, 16], w: [0, 8], c: [8, 8] };

/**
 * The centre pixels of a piece, in map px. A piece owns the points of its
 * line inside its cell: a mid-side on its west or north edge is its own, one
 * on its east or south edge (x or y = 16) is the neighbour's, so two pieces
 * that meet never both draw (or both miss) the meeting point.
 */
export function pieceCentres(p: LinePiece, index = 0): CentrePx[] {
  const pts: CentrePx[] = [];
  const seen = new Set<number>();
  const seg = (a: readonly [number, number], b: readonly [number, number]) => {
    const dx = Math.sign(b[0] - a[0]);
    const dy = Math.sign(b[1] - a[1]);
    const n = Math.max(Math.abs(b[0] - a[0]), Math.abs(b[1] - a[1]));
    const dir: Dir = dy === 0 ? 0 : dx === 0 ? 1 : dx * dy < 0 ? 2 : 3;
    for (let k = 0; k <= n; k++) {
      const x = a[0] + dx * k;
      const y = a[1] + dy * k;
      if (x < 0 || y < 0 || x > 15 || y > 15 || seen.has(y * 16 + x)) continue;
      seen.add(y * 16 + x);
      pts.push({ X: p.x * 16 + x, Y: p.y * 16 + y, dir, piece: index });
    }
  };
  const A = MID[p.a];
  const B = MID[p.b];
  if (p.sharp || p.b === "c") {
    seg(A, MID.c);
    if (p.b !== "c") seg(MID.c, B);
  } else seg(A, B);
  return pts;
}

/** Pieces along a rectangle of cell mid-lines (x0, y0)–(x1, y1) inclusive, clockwise from the north-west corner; `gaps` are cells left open. */
export function rectPieces(x0: number, y0: number, x1: number, y1: number, gaps: readonly (readonly [number, number])[] = []): LinePiece[] {
  const out: LinePiece[] = [];
  const open = (x: number, y: number) => gaps.some(([gx, gy]) => gx === x && gy === y);
  const add = (p: LinePiece) => {
    if (!open(p.x, p.y)) out.push(p);
  };
  add({ x: x0, y: y0, a: "e", b: "s", sharp: true });
  for (let x = x0 + 1; x < x1; x++) add({ x, y: y0, a: "w", b: "e" });
  add({ x: x1, y: y0, a: "w", b: "s", sharp: true });
  for (let y = y0 + 1; y < y1; y++) add({ x: x1, y, a: "n", b: "s" });
  add({ x: x1, y: y1, a: "n", b: "w", sharp: true });
  for (let x = x1 - 1; x > x0; x--) add({ x, y: y1, a: "e", b: "w" });
  add({ x: x0, y: y1, a: "e", b: "n", sharp: true });
  for (let y = y1 - 1; y > y0; y--) add({ x: x0, y, a: "s", b: "n" });
  return out;
}

/** One cell's share of a drawn run: an image at map px (x, y), and the row its depth sorts by. */
export interface LineSprite {
  cx: number;
  cy: number;
  x: number;
  y: number;
  img: Indexed;
  base: number;
}

/** How each piece has worn: 0 standing, 1 sagging (leaning, the mesh drooping), 2 down (lying on the ground). Walls and levees ignore it. */
export type WearOf = (piece: number) => number;

interface StyleDef {
  /** Footprint half-width (Chebyshev px). */
  r: number;
  /** Column height at a footprint pixel `d` px from the centre line. */
  height(d: number, along: number, seed: number): number;
  outline: boolean;
  /** The character at height k of a column (k = 0 the ground row), or '.' for none. */
  px(k: number, h: number, along: number, dir: Dir, d: number, south: boolean, front: boolean): string;
}

const mod = (a: number, n: number) => ((a % n) + n) % n;

/** The shaded east side of a thin run seen end on. */
const NS_SHADE: Readonly<Record<string, string>> = { c: "C", C: "p", d: "z", q: "p", l: "b", b: "B", R: "H", I: "i", i: "x", P: "Q" };

const STYLES: Readonly<Record<LineStyle, StyleDef>> = {
  // Heras panels: a galvanised tube frame round a wire mesh, posts every 16 px of the run (8 on 45°), concrete feet.
  heras: {
    r: 0,
    height: () => 13,
    outline: true,
    px(k, h, along, dir) {
      const period = dir >= 2 ? 8 : 16;
      const post = mod(along, period);
      const near = Math.min(post, period - post);
      if (k <= 1) return near <= 2 && dir !== 1 ? (k === 1 ? "q" : "P") : k === 0 && dir === 1 && near === 0 ? "P" : ".";
      if (near === 0) return k === h ? "c" : "C";
      if (k === h) return "c";
      if (k === h - 1 || k === 2) return "C";
      return mod(along, 2) === 0 || k % 3 === 0 ? "d" : ".";
    },
  },
  // Site hoarding: painted plywood panels in council blue, a white band, joints every 16 px, a dark foot.
  hoarding: {
    r: 0,
    height: () => 15,
    outline: true,
    px(k, h, along) {
      if (k === h) return "l";
      if (k <= 1) return "B";
      if (k === 11) return "c";
      return mod(along, 16) === 0 ? "B" : "b";
    },
  },
  // A road barrier: a red-and-white plank on A-frame legs with weighted feet.
  barrier: {
    r: 0,
    height: () => 8,
    outline: true,
    px(k, h, along, dir) {
      const period = dir >= 2 ? 8 : 16;
      const leg = mod(along, period);
      const isLeg = leg === 2 || leg === period - 3;
      if (k <= 1) return isLeg ? (k === 0 ? "x" : "i") : ".";
      if (k <= 4) return isLeg ? "i" : ".";
      const red = (Math.floor(mod(along, 8) / 4) & 1) === 1;
      if (k === h) return red ? "R" : "c";
      if (k === 5) return red ? "H" : "C";
      return red ? "R" : "c";
    },
  },
  // New railings (a bridge's): galvanised posts every 4 px, a mid rail and a bright top rail.
  railing: {
    r: 0,
    height: () => 6,
    outline: true,
    px(k, h, along) {
      const post = mod(along, 4) === 0;
      if (k === h) return "C";
      if (k === 3) return "p";
      if (k === 0) return post ? "P" : ".";
      return post ? "p" : ".";
    },
  },
  // A concrete flood wall: a lit coping, the face toward the viewer in the light (\), square on (west–east) or in shade (/), panel joints every 16 px, a dark foot.
  wall: {
    r: 1,
    height: () => 10,
    outline: false,
    px(k, h, along, dir, _d, _south, front) {
      if (k === h) return front ? "c" : "C";
      const tone = dir === 3 ? 2 : dir === 2 ? 0 : 1;
      if (k === 0) return tone === 0 ? "x" : "Q";
      const face = ["Q", "P", "p"][tone] as string;
      const joint = ["P", "Q", "P"][tone] as string;
      if (k === h - 1) return tone === 0 ? "P" : "q";
      return mod(along, 16) === 0 ? joint : face;
    },
  },
  // A grass levee: a crest path between grassed slopes, the slope toward the viewer lit, a dark toe.
  levee: {
    r: 4,
    height: (d) => [7, 7, 5, 3, 1][Math.min(4, d)] as number,
    outline: false,
    px(k, h, along, _dir, d, south) {
      if (d <= 1) return k === h ? (mod(along, 7) === 0 ? "F" : "f") : south ? "n" : "N";
      if (!south) return d === 2 && k === h ? "n" : "N";
      if (k === 0 && d === 4) return "N";
      return d === 2 ? "m" : "n";
    },
  },
  // The same bank going up: bare earth, the crest rolled, no grass yet.
  levee_bare: {
    r: 4,
    height: (d) => [7, 7, 5, 3, 1][Math.min(4, d)] as number,
    outline: false,
    px(k, h, along, _dir, d, south) {
      if (d <= 1) return k === h ? (mod(along, 5) === 0 ? "W" : "w") : south ? "w" : "W";
      if (!south) return d === 2 && k === h ? "w" : "W";
      if (k === 0 && d === 4) return "W";
      return d === 2 ? "k" : "w";
    },
  },
  // Sheet piling driven ahead of a wall: corrugated steel standing to ragged heights.
  piles: {
    r: 0,
    height: (_d, along, seed) => 5 + (hash32(seed, along >> 1, 0x5e7) % 4),
    outline: true,
    px(k, h, along) {
      if (k === h) return "I";
      return mod(along, 4) < 2 ? "i" : "x";
    },
  },
};

/** Along-run coordinate of a centre pixel: map x for west–east and 45° runs, map y for north–south. */
const alongOf = (c: CentrePx) => (c.dir === 1 ? c.Y : c.X);

/**
 * Draw a run of pieces (or an explicit list of centre pixels) in a style and
 * cut it into per-cell sprites. `wear` gives each piece's wear (fences and
 * barriers sag, then fall); `seed` varies sheet piles' ragged tops.
 */
export function drawRun(p: Palette, lightness: readonly number[], style: LineStyle, centres: readonly CentrePx[], seed = 0, wear: WearOf = () => 0): LineSprite[] {
  if (centres.length === 0) return [];
  const S = STYLES[style];
  const L = worksLegend(p);
  // The footprint: every pixel within r (Chebyshev) of a centre pixel takes the nearest one's data.
  const foot = new Map<number, { X: number; Y: number; c: CentrePx; d: number }>();
  const key = (X: number, Y: number) => (Y + 4096) * 16384 + (X + 4096);
  for (const c of centres) {
    for (let dy = -S.r; dy <= S.r; dy++) {
      for (let dx = -S.r; dx <= S.r; dx++) {
        const X = c.X + dx;
        const Y = c.Y + dy;
        const d = Math.max(Math.abs(dx), Math.abs(dy));
        const k = key(X, Y);
        const prev = foot.get(k);
        if (!prev || d < prev.d || (d === prev.d && (c.Y < prev.c.Y || (c.Y === prev.c.Y && c.X < prev.c.X)))) foot.set(k, { X, Y, c, d });
      }
    }
  }
  const cells = [...foot.values()];
  const maxH = 16;
  const minX = Math.min(...cells.map((f) => f.X)) - 10;
  const maxX = Math.max(...cells.map((f) => f.X)) + 10;
  const minY = Math.min(...cells.map((f) => f.Y)) - maxH - 2;
  const maxY = Math.max(...cells.map((f) => f.Y)) + 10;
  const W = maxX - minX + 1;
  const H = maxY - minY + 1;
  const img = blank(W, H);
  const owner = new Int32Array(W * H).fill(-1);
  const put = (X: number, Y: number, ch: string, cell: number) => {
    if (ch === ".") return;
    const x = X - minX;
    const y = Y - minY;
    if (x < 0 || y < 0 || x >= W || y >= H) return;
    img.px[y * W + x] = L[ch] as number;
    owner[y * W + x] = cell;
  };
  const cellOf = (X: number, Y: number) => (Y >> 4) * 65536 + (X >> 4) + 32768;
  // Back to front, west to east.
  cells.sort((a, b) => a.Y - b.Y || a.X - b.X);
  for (const f of cells) {
    const along = alongOf({ ...f.c, X: f.X, Y: f.Y });
    const w = wear(f.c.piece);
    const h0 = S.height(f.d, along, seed);
    const cell = cellOf(f.X, f.Y);
    // Which side of the centre line (the half toward the viewer is lit on levees), and whether this pixel is the band's front edge.
    const south = f.c.dir === 1 ? f.X > f.c.X : f.Y > f.c.Y;
    const front = !foot.has(key(f.X, f.Y + 1));
    if (S.r === 0 && w >= 2) {
      // Down: the panel lying flat on the ground beside its line (south, or east of a north–south run).
      const len = Math.min(8, h0);
      for (let j = 0; j < len; j++) {
        const [X, Y] = f.c.dir === 1 ? [f.X + j, f.Y] : [f.X, f.Y + j];
        const k = Math.round(((j + 1) * h0) / len);
        const ch = S.px(k, h0, along, f.c.dir, 0, true, true);
        put(X, Y, ch === "." && j % 2 === 0 && style === "heras" ? "." : ch, cell);
      }
      continue;
    }
    // Sagging: the top drooping toward the middle of a panel, the panel leaning over.
    let h = h0;
    if (S.r === 0 && w === 1) {
      const period = f.c.dir >= 2 ? 8 : 16;
      const m = mod(along, period);
      h = h0 - Math.round((Math.min(m, period - m) * 4) / period);
    }
    for (let k = 0; k <= h; k++) {
      const lean = S.r === 0 && w === 1 ? Math.floor((k * 3) / Math.max(1, h)) : 0;
      const ch = S.px(k, h, along, f.c.dir, f.d, south, front);
      put(f.X + lean, f.Y - k, ch, cell);
      // A thin run seen end on (north–south) shows its east side in shade too, 2 px wide, as round 6's railings.
      if (S.r === 0 && f.c.dir === 1 && ch !== ".") put(f.X + lean + 1, f.Y - k, NS_SHADE[ch] ?? ch, cell);
    }
  }
  const out = S.outline ? outlineExterior(img, p.lineOf, lightness) : img;
  // Outline pixels belong to the cell of the drawn pixel they border.
  if (S.outline) {
    for (let y = 0; y < H; y++) {
      for (let x = 0; x < W; x++) {
        const i = y * W + x;
        if (owner[i] !== -1 || !out.px[i]) continue;
        for (const [dx, dy] of [
          [0, 1],
          [0, -1],
          [-1, 0],
          [1, 0],
        ] as const) {
          const j = (y + dy) * W + x + dx;
          if (x + dx >= 0 && y + dy >= 0 && x + dx < W && y + dy < H && owner[j] !== -1 && img.px[j]) {
            owner[i] = owner[j] as number;
            break;
          }
        }
      }
    }
  }
  // Cut into per-cell sprites.
  const byCell = new Map<number, number[]>();
  for (let i = 0; i < owner.length; i++) {
    const o = owner[i] as number;
    if (o === -1 || !out.px[i]) continue;
    let list = byCell.get(o);
    if (!list) byCell.set(o, (list = []));
    list.push(i);
  }
  const sprites: LineSprite[] = [];
  for (const [o, list] of [...byCell.entries()].sort((a, b) => a[0] - b[0])) {
    const cy = Math.floor(o / 65536);
    const cx = o - cy * 65536 - 32768;
    let x0 = W;
    let y0 = H;
    let x1 = 0;
    let y1 = 0;
    for (const i of list) {
      const x = i % W;
      const y = (i - x) / W;
      x0 = Math.min(x0, x);
      y0 = Math.min(y0, y);
      x1 = Math.max(x1, x);
      y1 = Math.max(y1, y);
    }
    const part = blank(x1 - x0 + 1, y1 - y0 + 1);
    for (const i of list) {
      const x = i % W;
      const y = (i - x) / W;
      part.px[(y - y0) * part.w + (x - x0)] = out.px[i] as number;
    }
    sprites.push({ cx, cy, x: minX + x0, y: minY + y0, img: part, base: (cy + 1) * 16 });
  }
  return sprites;
}

/** Centre pixels of a run of pieces. */
export function runCentres(pieces: readonly LinePiece[]): CentrePx[] {
  return pieces.flatMap((p, i) => pieceCentres(p, i));
}

/** Centre pixels of a straight run in map px (a row or a column), as one piece per cell it crosses. */
export function straightCentres(x0: number, y0: number, len: number, dir: 0 | 1): CentrePx[] {
  const out: CentrePx[] = [];
  for (let k = 0; k < len; k++) {
    const X = dir === 0 ? x0 + k : x0;
    const Y = dir === 0 ? y0 : y0 + k;
    out.push({ X, Y, dir, piece: dir === 0 ? X >> 4 : Y >> 4 });
  }
  return out;
}
