// Round 6 props for street ends (DEC-090 D5 amended; PIXEL_ART_TOOL.md
// "Round 6: street ends"): bollards across a road closed to traffic, a field
// gate where a road or track meets farmland, and a railing where a road meets
// water. Each stands on the end line of a street end, so each is drawn per
// facing of the end it closes (`END_FACINGS`: n, e, s, w, ne, nw, se, sw; the
// variant is the facing's index), plus a second look 8 variants on: lit
// bollards (a glowing head and a small pool of light, as the street lamp's in
// round 2) and a galvanised steel gate. They are laid out by shape rules into
// the text grids of rounds 1 and 4, so they get the automatic outline and the
// lints like any other prop.
//
// The frame is the prop's cell (16 × 16) with room above it for what stands
// up: grid row `UP + py` is the cell's pixel row py, grid column px its column.
import { END_FACINGS, type EndFacing } from "./ends";

/** A prop definition as props.ts reads it (round 6 adds a smaller light pool for lit bollards). */
export interface PropDef3 {
  grid: string;
  base: number;
  left?: number;
  lit?: string;
  pool?: "bollard";
}

const UP = 12;

class Canvas {
  private readonly c: string[];
  readonly w: number;
  readonly h: number;
  constructor(w: number, h: number) {
    this.w = w;
    this.h = h;
    this.c = new Array<string>(w * h).fill(".");
  }
  /** Set a pixel in cell coordinates (row py of the cell is grid row UP + py). */
  set(px: number, py: number, ch: string) {
    const y = py + UP;
    if (px >= 0 && y >= 0 && px < this.w && y < this.h) this.c[y * this.w + px] = ch;
  }
  text(): string {
    const rows: string[] = [];
    for (let y = 0; y < this.h; y++) rows.push(this.c.slice(y * this.w, (y + 1) * this.w).join(""));
    return rows.join("\n");
  }
}

/** Where a street end's line crosses the cell for a facing: the points props stand on, and the line's direction. */
function line(facing: EndFacing): { pts: [number, number][]; dir: "ew" | "ns" | "\\" | "/" } {
  switch (facing) {
    case "n":
      return { pts: [[4, 7], [12, 7]], dir: "ew" };
    case "s":
      return { pts: [[4, 9], [12, 9]], dir: "ew" };
    case "e":
      return { pts: [[9, 5], [9, 13]], dir: "ns" };
    case "w":
      return { pts: [[6, 5], [6, 13]], dir: "ns" };
    case "ne":
    case "sw":
      return { pts: [[4, 4], [12, 12]], dir: "\\" };
    default:
      return { pts: [[12, 4], [4, 12]], dir: "/" };
  }
}

/**
 * Bollards: two cast-iron posts a cell (8 px apart along the end line), 3 px
 * wide and 8 tall, lit from the left, a white reflective band; the lit look
 * has a glowing head instead.
 */
function bollards(facing: EndFacing, lit: boolean): PropDef3 {
  const c = new Canvas(16, UP + 16);
  const { pts } = line(facing);
  // Draw the farther (higher on screen) post first, so the nearer one overlaps it.
  for (const [gx, gy] of [...pts].sort((a, b) => a[1] - b[1])) {
    const rows = lit ? [".g.", "gyG", "GgG", "iiS", "Iio", "Iio", "Iio", "iio"] : [".i.", "Iii", "ccC", "Iio", "Iio", "Iio", "Iio", "iio"];
    rows.forEach((r, k) => {
      for (let j = 0; j < 3; j++) {
        const ch = r[j] as string;
        if (ch !== ".") c.set(gx - 1 + j, gy - 7 + k, ch === "S" ? "o" : ch);
      }
    });
  }
  const def: PropDef3 = { grid: c.text(), base: UP + 15, left: 0 };
  if (lit) {
    def.lit = "gyG";
    def.pool = "bollard";
  }
  return def;
}

/**
 * A field gate on the end line, in the gap of the hedge: two square posts
 * and a five-bar gate with its diagonal brace (timber, or galvanised steel
 * for the second look). Across a north–south line it is seen end on: a
 * strip of rails between its posts.
 */
function fieldGate(facing: EndFacing, steel: boolean): PropDef3 {
  const c = new Canvas(16, UP + 16);
  const [lite, mid, dark] = steel ? ["c", "I", "i"] : ["k", "w", "W"];
  const post = (x: number, gy: number, w = 2) => {
    for (let y = gy - 11; y <= gy; y++) for (let k = 0; k < w; k++) c.set(x + k, y, y === gy - 11 ? "k" : k === 0 ? "w" : "W");
  };
  const { dir } = line(facing);
  if (dir === "ew") {
    // The gate's line: the middle of the hedge, a little beyond the end line.
    const gy = facing === "n" ? 9 : 12;
    post(0, gy);
    post(14, gy);
    for (let x = 2; x < 14; x++) {
      for (const h of [2, 4, 6, 8, 10]) c.set(x, gy - h, h === 10 ? lite : mid);
      // The brace, from the hanging post's foot to the top of the far end.
      const by = gy - 2 - Math.round(((x - 2) * 8) / 11);
      c.set(x, by, dark);
    }
    // The upright at the hanging end and the middle.
    for (const x of [2, 8]) for (let h = 2; h <= 10; h++) c.set(x, gy - h, mid);
  } else if (dir === "ns") {
    // End on: the rails run up the screen between the posts at the cell's top and bottom.
    const gx = facing === "e" ? 9 : 6;
    for (let y = -9; y <= 13; y++) {
      c.set(gx, y, lite);
      c.set(gx + 1, y, y % 2 === 0 ? dark : mid);
    }
    for (const gy of [1, 15]) {
      for (let y = gy - 11; y <= gy; y++) {
        c.set(gx - 1, y, y === gy - 11 ? "k" : "w");
        c.set(gx, y, y === gy - 11 ? "k" : "w");
        c.set(gx + 1, y, "W");
      }
    }
  } else {
    // Along a 45° line through the cell's middle, a little beyond the end line: the gate's face seen slantwise.
    const shift = facing === "ne" || facing === "se" ? -2 : 2;
    const ground = (x: number) => (dir === "\\" ? x + shift : 15 - x + shift);
    for (let x = 2; x < 14; x++) {
      const g = ground(x);
      for (const h of [2, 4, 6, 8, 10]) c.set(x, g - h, h === 10 ? lite : mid);
      c.set(x, g - 2 - Math.round(((x - 2) * 8) / 11), dark);
    }
    for (const x of [0, 14]) {
      const g = ground(x + 1);
      for (let y = g - 11; y <= g; y++) for (let k = 0; k < 2; k++) c.set(x + k, y, y === g - 11 ? "k" : k === 0 ? "w" : "W");
    }
  }
  return { grid: c.text(), base: UP + 15, left: 0 };
}

/**
 * A railing along the water's edge: two iron rails on posts every 4 px,
 * 7 px tall, the top rail catching the light. It runs along the side of its
 * cell the water is on (n, e, s, w), or along a 45° end line.
 */
function railing(facing: EndFacing): PropDef3 {
  const c = new Canvas(16, UP + 16);
  const along = (pts: [number, number][]) => {
    for (const [x, g] of pts) {
      c.set(x, g - 7, "I");
      c.set(x, g - 4, "i");
      if (x % 4 === 1) for (let h = 1; h <= 6; h++) c.set(x, g - h, h === 6 ? "I" : "i");
      c.set(x, g, x % 4 === 1 ? "o" : ".");
    }
  };
  if (facing === "n" || facing === "s") {
    const g = facing === "n" ? 3 : 14;
    along(Array.from({ length: 16 }, (_, x) => [x, g] as [number, number]));
  } else if (facing === "e" || facing === "w") {
    // Seen end on: the rails run up the screen, a post every 4 px.
    const x = facing === "e" ? 13 : 2;
    for (let y = -7; y <= 15; y++) {
      c.set(x, y, "I");
      c.set(x + 1, y, "i");
    }
    for (let gy = 3; gy <= 15; gy += 4) for (let h = 0; h <= 7; h++) c.set(x + 1, gy - h, h === 7 ? "I" : "o");
  } else {
    const shift = facing === "ne" || facing === "se" ? -3 : 3;
    along(Array.from({ length: 16 }, (_, x) => [x, facing === "ne" || facing === "sw" ? x + shift : 15 - x + shift] as [number, number]));
  }
  return { grid: c.text(), base: UP + 15, left: 0 };
}

/** The street-end props: per kind, a variant per facing (`END_FACINGS` order), then the second look. */
export function endProps(): Record<string, PropDef3[]> {
  return {
    bollards: [...END_FACINGS.map((f) => bollards(f, false)), ...END_FACINGS.map((f) => bollards(f, true))],
    field_gate: [...END_FACINGS.map((f) => fieldGate(f, false)), ...END_FACINGS.map((f) => fieldGate(f, true))],
    railing: END_FACINGS.map((f) => railing(f)),
  };
}
