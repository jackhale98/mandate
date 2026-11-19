// Round 4 props for town layout v2 (DEC-090): a mine's headframe (winding
// tower), spoil heaps in three sizes, reed clumps, stepping stones for a
// creek, and open-air market stalls (four awnings over different goods, and
// a closed stall). The headframe and the heaps are laid out by shape rules
// into the same text grids as round 1's hand-drawn props, so they get the
// automatic outline, the shadow and the lints like any other prop.
//
// Extra legend (on top of props.ts):
//   u U cloth green base, shade      h H cloth red base, shade      j J cloth navy base, shade
//   l L mustard base, shade          f F field base, light          e E earth base, light
//   p P Q q paving base, shade, deep, light                           d z Z asphalt light, base, shade
//   R T brick base, shade            O ink
import { hash32 } from "../../src/core/hash";

export const PROP2_LEGEND_RAMPS: Readonly<Record<string, readonly [string, number]>> = {
  u: ["cloth_green", 0],
  U: ["cloth_green", -1],
  h: ["cloth_red", 0],
  H: ["cloth_red", -1],
  j: ["cloth_navy", 0],
  J: ["cloth_navy", -1],
  l: ["cloth_mustard", 0],
  L: ["cloth_mustard", -1],
  f: ["field", 0],
  F: ["field", 1],
  e: ["earth", 0],
  E: ["earth", 1],
  p: ["paving", 0],
  P: ["paving", -1],
  Q: ["paving", -2],
  q: ["paving", 1],
  d: ["asphalt", 1],
  z: ["asphalt", 0],
  Z: ["asphalt", -1],
  R: ["brick", 0],
  T: ["brick", -1],
  O: ["ink", 0],
};

/** A prop definition as props.ts reads it. */
export interface PropDef2 {
  grid: string;
  base: number;
  shadow?: { box: readonly [number, number, number, number]; height: number };
  left?: number;
}

/** A character canvas for shape rules: '.' is empty. */
class Chars {
  readonly w: number;
  readonly h: number;
  private readonly c: string[];
  constructor(w: number, h: number) {
    this.w = w;
    this.h = h;
    this.c = new Array<string>(w * h).fill(".");
  }
  set(x: number, y: number, ch: string) {
    if (x >= 0 && y >= 0 && x < this.w && y < this.h) this.c[y * this.w + x] = ch;
  }
  get(x: number, y: number): string {
    return x >= 0 && y >= 0 && x < this.w && y < this.h ? (this.c[y * this.w + x] as string) : ".";
  }
  rect(x: number, y: number, w: number, h: number, ch: string) {
    for (let yy = y; yy < y + h; yy++) for (let xx = x; xx < x + w; xx++) this.set(xx, yy, ch);
  }
  /** A 1 px line (Bresenham). */
  line(x0: number, y0: number, x1: number, y1: number, ch: string) {
    const dx = Math.abs(x1 - x0);
    const dy = -Math.abs(y1 - y0);
    const sx = x0 < x1 ? 1 : -1;
    const sy = y0 < y1 ? 1 : -1;
    let err = dx + dy;
    let x = x0;
    let y = y0;
    for (;;) {
      this.set(x, y, ch);
      if (x === x1 && y === y1) break;
      const e2 = 2 * err;
      if (e2 >= dy) {
        err += dy;
        x += sx;
      }
      if (e2 <= dx) {
        err += dx;
        y += sy;
      }
    }
  }
  text(): string {
    const rows: string[] = [];
    for (let y = 0; y < this.h; y++) rows.push(this.c.slice(y * this.w, (y + 1) * this.w).join(""));
    return `\n${rows.join("\n")}`;
  }
}

/**
 * The headframe: two winding wheels on a head beam, a tower of red-painted
 * steel on two tapering legs braced in open panels (wide enough that the
 * automatic outline leaves sky between the members), the ropes down to the
 * shaft and its cage, and a stone banksman's house at the foot. 28 × 62,
 * standing on its cell's bottom row.
 */
function headframe(): PropDef2 {
  const W = 28;
  const H = 62;
  const c = new Chars(W, H);
  const baseY = H - 1;
  const footY = baseY - 6;
  const headY = 17;
  // Legs: 2 px, from x 2 and 24 at the foot to x 7 and 19 under the head beam.
  const legX = (y: number, left: boolean) => {
    const t = (footY - y) / (footY - headY);
    return Math.round(left ? 2 + 5 * t : 24 - 5 * t);
  };
  for (let y = headY; y <= footY; y++) {
    c.set(legX(y, true), y, "R");
    c.set(legX(y, true) + 1, y, "T");
    c.set(legX(y, false), y, "R");
    c.set(legX(y, false) + 1, y, "T");
  }
  // Girders, and one diagonal per panel, alternating (a Warren truss): panels stay open.
  const girders = [headY, 27, 37, 47, footY];
  for (const gy of girders) for (let x = legX(gy, true) + 2; x < legX(gy, false); x++) c.set(x, gy, "R");
  for (let k = 0; k < girders.length - 1; k++) {
    const y0 = (girders[k] as number) + 1;
    const y1 = (girders[k + 1] as number) - 1;
    if (k % 2 === 0) c.line(legX(y0, true) + 2, y0, legX(y1, false) - 1, y1, "T");
    else c.line(legX(y0, false) - 1, y0, legX(y1, true) + 2, y1, "T");
  }
  // The head: a beam (a light top) on the legs, and two wheels on bearings above it.
  c.rect(1, headY - 2, 26, 2, "T");
  c.rect(1, headY - 2, 26, 1, "R");
  for (const cx of [7, 20]) {
    const cy = 8;
    const r = 6;
    for (let y = -r; y <= r; y++) {
      for (let x = -r; x <= r; x++) {
        const d = x * x + y * y;
        if (d <= r * r + 2 && d >= (r - 1) * (r - 1) + 1) c.set(cx + x, cy + y, y + x < 0 ? "I" : "i");
      }
    }
    c.line(cx - r + 1, cy, cx + r - 1, cy, "i");
    c.line(cx, cy - r + 1, cx, cy + r - 1, "i");
    c.set(cx, cy, "c");
    c.rect(cx - 1, cy + r, 3, headY - 2 - cy - r, "T");
  }
  // Ropes from the wheels' far sides down into the shaft, and the cage hanging in the tower.
  c.line(1, 8, 11, 44, "O");
  c.line(26, 8, 16, 44, "O");
  c.rect(11, 44, 6, 5, "j");
  c.rect(11, 44, 6, 1, "c");
  c.rect(12, 46, 4, 2, "J");
  // The banksman's house at the foot: stone, a dark roof edge, a door and a window.
  c.rect(0, footY + 1, 28, 1, "Q");
  c.rect(0, footY + 2, 28, 5, "p");
  c.rect(0, footY + 2, 28, 1, "q");
  for (let x = 2; x < 28; x += 6) c.rect(x, footY + 4, 1, 3, "P");
  c.rect(4, footY + 3, 3, 4, "w");
  c.rect(19, footY + 3, 4, 2, "g");
  c.set(19, footY + 3, "y");
  return { grid: c.text(), base: baseY, shadow: { box: [2, 54, 26, 62], height: 56 } };
}

/** A spoil heap: a cone of dark shale, lit on its left, gullies down from the ridge, scrub at its foot. */
function spoilHeap(size: 0 | 1 | 2): PropDef2 {
  const R = [15, 22, 30][size] as number;
  const Hh = [13, 20, 28][size] as number;
  const W = 2 * R + 2;
  const H = Hh + 3;
  const c = new Chars(W, H);
  const cx = R;
  const baseY = H - 1;
  const top = (x: number) => {
    const u = Math.abs(x - cx) / R;
    if (u > 1) return null;
    // A flattened cone with a slight wobble on the ridge line.
    const n = (hash32(x, size, 0x5e) % 3) - 1;
    return Math.round(baseY - Hh * (1 - Math.pow(u, 1.35)) + (u > 0.15 ? n * 0.5 : 0));
  };
  for (let x = 0; x < W; x++) {
    const ty = top(x);
    if (ty === null) continue;
    for (let y = ty; y <= baseY; y++) {
      const left = x < cx;
      const depth = y - ty;
      let ch = left ? (depth < 2 ? "d" : "z") : x < cx + R * 0.45 ? "z" : "Z";
      // Gullies: thin dark runnels wandering down from near the ridge, a lit lip on their left.
      for (let g = 0; g < 2 + size * 2; g++) {
        const gx = Math.round(cx - R * 0.75 + (hash32(g, size, 0x91) % Math.max(1, Math.round(R * 1.5)))) + ((y >> 2) & 1);
        const start = (top(gx) ?? baseY) + 2 + (g % 3);
        if (y < start || y >= baseY) continue;
        if (x === gx) ch = left ? "z" : "O";
        else if (x === gx - 1 && left) ch = "d";
      }
      // Lumps of shale: 2 px clusters of the darkest and the lightest.
      const lump = hash32(x >> 1, y, size, 0x1b) % 17;
      if (depth > 1 && lump === 0) ch = "O";
      else if (depth > 1 && lump === 1 && left) ch = "d";
      c.set(x, y, ch);
    }
  }
  // Scrub and grass at the foot, and the bare earth where it meets the ground.
  for (let x = 1; x < W - 1; x++) {
    if (c.get(x, baseY) === ".") continue;
    if (hash32(x, size, 0x3c) % 4 === 0) {
      c.set(x, baseY, "N");
      c.set(x, baseY - 1, "n");
    } else c.set(x, baseY, "Z");
  }
  return { grid: c.text(), base: baseY, left: Math.max(0, R - 8), shadow: { box: [Math.max(0, cx - R + 2), baseY - 6, cx + R - 2, baseY], height: Math.round(Hh * 0.7) } };
}

/** Reed clumps: a dense body of green stems lit with gold, a few seed heads standing out of the top. */
const REEDS: PropDef2[] = [
  {
    grid: `
      ......W....W..
      ..W...W.W..W..
      ..W..fW.W.fF..
      ..fF.fn.f.fF..
      .nfFnfnNfnfFn.
      .nfnNfnNfnNfn.
      nnfnNNfnNnNfnn
      NnNnNNnNnNNnNN
      .NNNNNNNNNNNN.`,
    base: 8,
  },
  {
    grid: `
      ...W....W.
      ...W..W.W.
      .f.F..fFn.
      .fnFnffFn.
      nfnNnfnNnn
      NnNNnNnNNN
      .NNNNNNNN.`,
    base: 6,
  },
];

const STEPPING: PropDef2[] = [
  {
    grid: `
      ........qqp.....
      .......qppP.....
      ........PPV.....
      ..qqpp..........
      .qppppP.........
      ..PPPPV....qqp..
      ..........qpppP.
      ...........PPPV.`,
    base: 11,
  },
  {
    grid: `
      ....qqp.........
      ...qpppP........
      ....PPPV..qqp...
      .........qppP...
      ..........PPV...
      ...qqpp.........
      ..qpppPP........
      ...PPPPV........`,
    base: 11,
  },
];

/** Market stalls: an awning in two stripes (a muted cloth and chalk), a counter of goods, posts. */
function stall(awning: string, shade: string, goods: readonly string[]): PropDef2 {
  const a = awning;
  const s = shade;
  const top = `${a}${a}cc${a}${a}cc${a}${a}cc${a}${a}c`;
  const under = `${s}${s}CC${s}${s}CC${s}${s}CC${s}${s}C`;
  const g = goods;
  return {
    grid: `
      ${top}
      ${top}
      ${under}
      .i...........i.
      .i.${g[0]}${g[1]}${g[0]}.${g[2]}${g[3]}.${g[1]}${g[0]}${g[1]}i.
      .i${g[1]}${g[0]}${g[1]}${g[0]}${g[3]}${g[2]}${g[3]}${g[0]}${g[1]}${g[0]}.i.
      kkkkkkkkkkkkkkk
      wwwwwwwwwwwwwww
      WWWWWWWWWWWWWWW
      .w...........w.
      .w...........w.`,
    base: 10,
  };
}

/** A closed stall: the goods put away, a canvas sheet tied down over the counter. */
const CLOSED_STALL: PropDef2 = {
  grid: `
    ...............
    ...............
    ...............
    .i...........i.
    .i...........i.
    .pqqqqqqqqqqqp.
    pqqqpqqqqqpqqqP
    pPqqPqqqqqPqqPP
    PPPPPPPPPPPPPPP
    .w...........w.
    .w...........w.`,
  base: 10,
};

export function round4Props(): Record<string, PropDef2[]> {
  return {
    headframe: [headframe()],
    spoil_heap: [spoilHeap(0), spoilHeap(1), spoilHeap(2)],
    reeds: REEDS,
    stepping_stones: STEPPING,
  };
}

/** Stall variants after round 1's (fruit, vegetables, fish, bread), and a closed stall. */
export function marketStalls(): PropDef2[] {
  return [stall("U", "U", ["r", "Y", "n", "m"]), stall("H", "H", ["n", "m", "N", "e"]), stall("j", "J", ["v", "V", "c", "C"]), stall("L", "L", ["k", "w", "E", "e"]), CLOSED_STALL];
}
