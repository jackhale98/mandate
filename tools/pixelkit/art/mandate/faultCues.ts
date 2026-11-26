// Fault and repair cues (DEC-114 §3, BEING_THERE.md §4.4–4.5; task B4): what
// a fault looks like on the street while it is open, while it is being
// repaired, and for a few months after (new). Each fault kind in
// `content/base/faults.jsonc` names three cues (`cue.open`, `cue.repairing`,
// `cue.new`); each is drawn here as a prop named `cue_<cue>` on the fault's
// spot cell, so the renderer, its depth sort, shadows and lamp light need no
// changes. Three generic cues (`cue_fault_open`, `cue_fault_repairing`,
// `cue_fault_new`) stand in for a cue that has no drawing yet.
//
//   street light (pavement): lamp_dark (unlit, a hazard tag on the post) ·
//     lamp_van (the head open, a ladder, the council van, a cone) ·
//     lamp_new (a new lantern, lit, a fresh coat and band)
//   drain (kerb):            drain_flooded (a puddle over the grate, leaves) ·
//     drain_cones (the grate lifted, a barrier and cones) · drain_new (a new
//     grate in fresh concrete)
//   road:                    pothole (a broken hole with cracks and water) ·
//     road_cones (cones round a planed patch, a road works sign) ·
//     tarmac_new (a fresh dark patch with sealed seams)
//   bus shelter (stop):      shelter_taped (a smashed pane, glass on the
//     ground, tape across) · shelter_van (a pane out, the new one leaning,
//     the van) · shelter_new (clean glass, a new roof)
//   play area (green):       swing_taped (a seat hanging by one chain, tape
//     round the frame) · swing_van (a seat off, a toolbox, the van) ·
//     swing_new (a fresh coat, new seats)
//
// Drawn by shape rules into character canvases in the works kit's legend
// (./worksParts: no colour is added), on the prop frame props.ts uses: the
// cell is 16 × 16 at column `left`, row `UP`; things rise above it. Open,
// repairing and new differ in silhouette as well as colour (a van, cones,
// tape), so they read at phone size and in the dark.
import { Chars } from "./worksParts";

/** A cue as props.ts builds it (in the works kit's legend). */
export interface CueDef {
  grid: string;
  base: number;
  left: number;
  lit?: string;
  legend: "works";
  shadow?: { box: readonly [number, number, number, number]; height: number };
}

/** Rows above the cell (the shelter's roof and the lamp's head rise into them). */
const UP = 16;

/** A canvas in cell coordinates: x 0..15 is the cell, negative x to the left, negative y above it. */
class Cue {
  readonly c: Chars;
  readonly L: number;
  readonly R: number;
  constructor(L: number, R: number) {
    this.L = L;
    this.R = R;
    this.c = new Chars(L + 16 + R, UP + 16);
  }
  set(x: number, y: number, ch: string) {
    this.c.set(x + this.L, y + UP, ch);
  }
  get(x: number, y: number): string {
    return this.c.get(x + this.L, y + UP);
  }
  rect(x: number, y: number, w: number, h: number, ch: string) {
    this.c.rect(x + this.L, y + UP, w, h, ch);
  }
  line(x0: number, y0: number, x1: number, y1: number, ch: string) {
    this.c.line(x0 + this.L, y0 + UP, x1 + this.L, y1 + UP, ch);
  }
  /** Paint a text block with its top-left at (x, y); '.' leaves the canvas as it is. */
  paste(x: number, y: number, text: string, map: Readonly<Record<string, string>> = {}) {
    const rows = text.split("\n").map((r) => r.trim()).filter((r) => r.length);
    rows.forEach((r, dy) => [...r].forEach((ch, dx) => ch !== "." && this.set(x + dx, y + dy, map[ch] ?? ch)));
  }
  /** Cut empty rows above the drawing (the frame keeps the cell at the bottom). */
  def(extra: Partial<CueDef> = {}, mirror = false): CueDef {
    const w = this.c.w;
    const rows: string[] = [];
    for (let y = 0; y < this.c.h; y++) {
      let r = "";
      for (let x = 0; x < w; x++) r += this.c.get(mirror ? w - 1 - x : x, y);
      rows.push(r);
    }
    let top = 0;
    while (top < UP && !/[^.]/.test(rows[top] as string)) top++;
    const def: CueDef = { grid: rows.slice(top).join("\n"), base: UP + 15 - top, left: mirror ? this.R : this.L, legend: "works", ...extra };
    if (mirror && def.shadow) {
      const [x0, y0, x1, y1] = def.shadow.box;
      def.shadow = { box: [15 - x1, y0, 15 - x0, y1], height: def.shadow.height };
    }
    return def;
  }
}

// ---------------------------------------------------------------------------
// Pieces

/** The street lamp of round 1 (props.ts), in the works legend: post at x 8, foot on the cell's bottom row. */
const LAMP = `
  ...iii...
  ..iIIIi..
  .iiiiiii.
  ..ghhGi..
  ..ghGGi..
  ..ggGGi..
  .iiiiiii.
  ...iIi...
  ....i....
  ....I....
  ....i....
  ....i....
  ....i....
  ....i....
  ....i....
  ....i....
  ....i....
  ....i....
  ....i....
  ....i....
  ....i....
  ...iIi...
  ...iii...
  ..iiiii..
  ..iiiii..`;

type LampHead = "dark" | "open" | "new";

function lamp(c: Cue, head: LampHead) {
  if (head === "new") {
    // A new LED lantern on a freshly painted post: a slim cap, a bright lens, a highlight down the post.
    c.paste(4, -9, LAMP.replace(/[ghG]/g, "i"));
    c.rect(5, -9, 7, 8, ".");
    c.paste(5, -8, `
      .iiiii.
      iIIIIIi
      iiiiiii
      .hhhhg.
      ..iii..
      ...i...`);
    for (let y = -2; y <= 11; y++) c.set(8, y, y % 3 === 0 ? "I" : "i");
    c.rect(7, 4, 3, 2, "b");
    c.rect(7, 4, 3, 1, "l");
    return;
  }
  c.paste(4, -9, LAMP);
  if (head === "dark") {
    // The lantern dead: black glass with a crack, a red-and-white tag tied round the post.
    c.paste(6, -6, `
      xxO
      xOO
      xxO`);
    c.set(7, -5, "C");
    c.set(6, -4, "C");
    c.paste(6, 1, `
      RcR
      cRc
      .c.`);
  } else {
    // Open for the electrician: the lantern's glass out, its door hanging open on the right.
    c.paste(6, -6, `
      OOO
      OxO
      OOO`);
    c.rect(13, -6, 1, 3, "h");
    c.set(12, -6, "i");
  }
}

/** A traffic cone with its foot centre at (x, y), as the works kit draws it. */
function cone(c: Cue, x: number, y: number) {
  c.paste(x - 2, y - 6, `
    ..a..
    ..aA.
    .caC.
    .cCC.
    .aaA.
    .aaAA
    OOOOO`);
}

/** The council's van facing right, its rear at x and its wheels on row y: white, a blue band, an amber beacon. */
function van(c: Cue, x: number, y: number) {
  const top = y - 13;
  c.rect(x, top + 3, 14, 9, "c");
  c.rect(x, top + 3, 14, 1, "C");
  c.rect(x + 1, top + 2, 12, 1, "C");
  c.rect(x + 14, top + 5, 5, 7, "c");
  c.rect(x + 14, top + 4, 3, 1, "C");
  c.rect(x + 15, top + 5, 3, 3, "g");
  c.set(x + 15, top + 5, "h");
  c.set(x + 16, top + 5, "h");
  c.rect(x + 18, top + 8, 1, 4, "C");
  c.rect(x, top + 8, 19, 1, "b");
  c.rect(x, top + 9, 19, 1, "B");
  c.rect(x + 1, top + 4, 1, 7, "C");
  c.rect(x + 13, top + 4, 1, 8, "C");
  c.rect(x + 19, top + 10, 1, 2, "I");
  c.rect(x, top + 12, 20, 1, "x");
  c.rect(x + 6, top, 2, 2, "a");
  c.set(x + 6, top, "r");
  for (const wx of [x + 4, x + 15]) {
    for (let dy = -2; dy <= 2; dy++) for (let dx = -2; dx <= 2; dx++) if (dx * dx + dy * dy <= 5) c.set(wx + dx, y + dy, Math.abs(dx) + Math.abs(dy) <= 1 ? "I" : "O");
  }
}

/** Barrier tape between two points, red and white in turns of two pixels. */
function tape(c: Cue, x0: number, y0: number, x1: number, y1: number) {
  const n = Math.max(Math.abs(x1 - x0), Math.abs(y1 - y0));
  for (let k = 0; k <= n; k++) {
    const x = Math.round(x0 + ((x1 - x0) * k) / Math.max(1, n));
    const y = Math.round(y0 + ((y1 - y0) * k) / Math.max(1, n));
    c.set(x, y, (k >> 1) % 2 === 0 ? "R" : "c");
  }
}

/** A short red-and-white plastic barrier on two feet, its rail at row y. */
function barrier(c: Cue, x: number, y: number, w: number) {
  for (let k = 0; k < w; k++) {
    const ch = (k >> 1) % 2 === 0 ? "R" : "c";
    c.set(x + k, y, ch);
    c.set(x + k, y + 1, ch === "R" ? "H" : "C");
  }
  for (const fx of [x + 1, x + w - 2]) {
    c.rect(fx, y + 2, 1, 2, "x");
    c.rect(fx - 1, y + 4, 3, 1, "x");
  }
}

/** A gully grate on the kerb's gutter, its frame from (x, y). */
function grate(c: Cue, x: number, y: number, fresh: boolean) {
  c.rect(x, y, 9, 6, fresh ? "I" : "x");
  for (let k = 1; k < 8; k += 2) c.rect(x + k, y + 1, 1, 4, fresh ? "x" : "O");
  if (fresh) for (let k = 2; k < 8; k += 2) c.rect(x + k, y + 1, 1, 4, "i");
}

/** The bus shelter: a slate roof on posts, glass back and side, a bench, a timetable; on the cell, rising 16 px. */
function shelter(c: Cue, look: "old" | "new") {
  const roof = look === "new" ? ["l", "b", "B"] : ["E", "e", "x"];
  c.rect(-2, -16, 20, 1, roof[0] as string);
  c.rect(-2, -15, 20, 2, roof[1] as string);
  c.rect(-2, -13, 20, 1, roof[2] as string);
  // The back wall's glass and its frame.
  c.rect(-1, -12, 18, 17, "g");
  for (let y = -12; y <= 4; y++) if ((y + 12) % 6 === 0) c.rect(-1, y, 18, 1, "i");
  for (const px of [-1, 7, 16]) c.rect(px, -12, 1, 22, "i");
  c.rect(-1, 9, 1, 1, "x");
  c.rect(16, 9, 1, 1, "x");
  // The timetable panel.
  c.rect(10, -10, 5, 7, "c");
  c.rect(10, -10, 5, 1, "b");
  for (let y = -8; y < -3; y += 2) c.rect(11, y, 3, 1, "C");
  // The bench.
  c.rect(1, 3, 13, 1, "k");
  c.rect(1, 4, 13, 1, "w");
  c.rect(2, 5, 1, 3, "i");
  c.rect(12, 5, 1, 3, "i");
  // The side panel toward the viewer.
  c.rect(17, -12, 1, 20, "h");
  c.rect(17, 8, 1, 2, "i");
  if (look === "new") {
    // Clean glass catching the light, a fresh band on the frame.
    for (const s of [0, 9]) c.line(s, 2, s + 5, -9, "h");
    c.rect(-1, -12, 18, 1, "I");
  }
}

/**
 * The swings, seen from the front: two posts with splayed feet, the top bar,
 * two seats on chains, a worn patch beneath. Lines stand at least three
 * pixels apart, so the automatic outline leaves the ground showing between
 * them (the frame reads as a frame, not a cage).
 */
function swings(c: Cue, look: "old" | "taped" | "off" | "new") {
  const frame = look === "new" ? "b" : "i";
  const hi = look === "new" ? "l" : "I";
  const seat = look === "new" ? "R" : "w";
  if (look === "new") {
    // A new safety surface under the new swings, edged in concrete.
    c.rect(-4, 8, 25, 8, "5");
    c.rect(-4, 15, 25, 1, "q");
    c.rect(-4, 8, 1, 8, "q");
    c.rect(20, 8, 1, 8, "q");
  } else {
    c.rect(0, 9, 16, 4, "e");
    c.rect(2, 13, 12, 1, "e");
  }
  for (const x of [-3, 18]) {
    c.rect(x, -14, 1, 27, frame);
    c.rect(x - 1, 12, 3, 1, frame);
  }
  c.rect(-3, -15, 22, 2, frame);
  c.rect(-3, -15, 22, 1, hi);
  const chains = (x: number, len: number) => c.rect(x, -13, 1, len, "I");
  // The left seat.
  chains(1, 16);
  chains(5, 16);
  c.rect(0, 3, 7, 2, seat);
  // The right seat.
  if (look === "taped") {
    // Hanging by one chain, the other snapped short.
    chains(10, 16);
    chains(14, 7);
    c.line(10, 3, 14, 8, seat);
    c.line(11, 3, 15, 8, seat);
  } else if (look === "off") {
    chains(10, 9);
    chains(14, 9);
    c.rect(9, 13, 7, 2, seat);
  } else {
    chains(10, 16);
    chains(14, 16);
    c.rect(9, 3, 7, 2, seat);
  }
  if (look === "taped") {
    tape(c, -3, 6, 18, -2);
    tape(c, -3, 7, 18, -1);
  }
}

// ---------------------------------------------------------------------------
// The cues

function lampDark(): CueDef {
  const c = new Cue(0, 0);
  lamp(c, "dark");
  return c.def({ shadow: { box: [7, 13, 9, 15], height: 24 } });
}

function lampVan(mirror: boolean): CueDef {
  const c = new Cue(0, 20);
  lamp(c, "open");
  // The ladder against the post, the van at the kerb, a cone at its tail.
  c.line(3, 15, 7, -3, "w");
  c.line(5, 15, 9, -3, "w");
  for (let k = 1; k < 6; k++) {
    const y = 15 - k * 3;
    const x = 3 + Math.round(((15 - y) * 4) / 18);
    c.rect(x + 1, y, 1, 1, "k");
  }
  van(c, 14, 13);
  cone(c, 12, 15);
  return c.def({ shadow: { box: [13, 11, 34, 15], height: 13 } }, mirror);
}

function lampNew(): CueDef {
  const c = new Cue(0, 0);
  lamp(c, "new");
  return c.def({ lit: "ghG", shadow: { box: [7, 13, 9, 15], height: 24 } });
}

function drainFlooded(): CueDef {
  const c = new Cue(0, 0);
  // A puddle spreading from the blocked grate, the grate's bars showing through, leaves caught on it.
  for (let y = 0; y < 16; y++) {
    for (let x = 0; x < 16; x++) {
      const d = ((x - 8) / 7.5) ** 2 + ((y - 9) / 5.5) ** 2 + ((x * 7 + y * 3) % 5) * 0.03;
      if (d < 1) c.set(x, y, d < 0.25 ? "_" : "~");
    }
  }
  // The grate under the water: its frame and bars a shade darker than the puddle.
  c.rect(4, 6, 9, 6, "_");
  for (let k = 1; k < 8; k += 2) c.rect(4 + k, 7, 1, 4, "x");
  c.rect(4, 6, 9, 1, "x");
  c.rect(4, 11, 9, 1, "x");
  for (const [x, y] of [
    [2, 8],
    [10, 13],
    [12, 5],
  ] as const) c.set(x, y, "^");
  for (const [x, y, ch] of [
    [6, 8, "v"],
    [7, 8, "u"],
    [10, 9, "N"],
    [11, 9, "n"],
  ] as const) c.set(x, y, ch);
  return c.def();
}

function drainCones(): CueDef {
  const c = new Cue(1, 1);
  // The grate lifted out beside an open gully, a barrier behind and cones in front.
  c.rect(4, 6, 9, 6, "O");
  c.rect(5, 7, 7, 1, "x");
  c.paste(1, 12, `
    .xixixix.
    xixixixix`);
  barrier(c, 1, 0, 14);
  cone(c, 2, 11);
  cone(c, 14, 13);
  return c.def();
}

function drainNew(): CueDef {
  const c = new Cue(0, 0);
  // A new grate set in fresh concrete.
  c.rect(2, 4, 13, 10, "q");
  c.rect(2, 13, 13, 1, "p");
  c.rect(14, 4, 1, 10, "p");
  grate(c, 4, 6, true);
  c.set(3, 5, "c");
  c.set(13, 12, "c");
  return c.def();
}

function pothole(): CueDef {
  const c = new Cue(0, 0);
  // A broken hole in the asphalt: a dark pit with water at the bottom, a rim of loose chips and cracks running off.
  for (let y = 2; y < 15; y++) {
    for (let x = 1; x < 16; x++) {
      const d = ((x - 8) / 6) ** 2 + ((y - 8) / 4.5) ** 2 + ((x * 5 + y * 11) % 7) * 0.035;
      if (d < 0.45) c.set(x, y, d < 0.18 ? "_" : "O");
      else if (d < 0.75) c.set(x, y, "x");
    }
  }
  c.line(2, 4, 4, 6, "Z");
  c.line(13, 12, 15, 14, "Z");
  c.line(12, 3, 14, 2, "Z");
  for (const [x, y] of [
    [3, 11],
    [12, 13],
    [14, 6],
    [5, 3],
  ] as const) c.set(x, y, "d");
  c.set(7, 7, "^");
  return c.def();
}

function roadCones(): CueDef {
  const c = new Cue(2, 2);
  // The patch planed off, cones round it and the road works sign at its head.
  c.rect(2, 4, 13, 10, "d");
  c.rect(4, 6, 9, 6, "Z");
  for (const [x, y] of [
    [1, 9],
    [15, 9],
    [5, 15],
    [12, 15],
  ] as const) cone(c, x, y);
  c.paste(5, -8, `
    ......A......
    .....AAA.....
    .....AcA.....
    ....AcccA....
    ....AcOcA....
    ...AccOOcA...
    ...AcOcOcA...
    ..AcccOOccA..
    ..AAAAAAAAA..
    .....i.i.....
    ....i...i....
    ...i.....i...`);
  return c.def();
}

function tarmacNew(): CueDef {
  const c = new Cue(0, 0);
  // A fresh patch: smooth dark tarmac with sealed seams round it, a few chips catching the light.
  c.rect(1, 3, 15, 11, "x");
  c.rect(2, 4, 13, 9, "Z");
  for (const [x, y] of [
    [4, 6],
    [9, 5],
    [12, 9],
    [6, 11],
    [10, 11],
  ] as const) c.set(x, y, "z");
  c.rect(2, 4, 13, 1, "z");
  return c.def();
}

function shelterTaped(): CueDef {
  const c = new Cue(2, 2);
  shelter(c, "old");
  // A pane smashed out: the hole, the crazed edges, the glass on the pavement, and tape across the front.
  c.rect(0, -5, 7, 6, ".");
  c.rect(0, -5, 7, 6, "O");
  for (const [x, y] of [
    [1, -5],
    [3, -4],
    [5, -2],
    [2, 0],
    [6, -5],
  ] as const) c.set(x, y, "h");
  c.line(0, -6, 3, -9, "c");
  c.line(6, -6, 4, -10, "c");
  c.rect(1, 11, 3, 2, "h");
  c.rect(6, 12, 2, 1, "h");
  tape(c, -2, 1, 18, -3);
  tape(c, -2, 2, 18, -2);
  return c.def({ shadow: { box: [-2, 7, 17, 10], height: 26 } });
}

function shelterVan(mirror: boolean): CueDef {
  const c = new Cue(2, 26);
  shelter(c, "old");
  // A pane taken out, the new one leaning against the post, the van alongside, a cone.
  c.rect(0, -11, 7, 14, ".");
  c.rect(0, -11, 7, 1, "i");
  c.paste(17, -6, `
    hgg
    hgg
    hgg
    hgg
    hgg
    hgg
    hgg
    hgg
    hgg
    hgg
    iii`);
  van(c, 21, 13);
  cone(c, 18, 15);
  return c.def({ shadow: { box: [-2, 7, 40, 11], height: 20 } }, mirror);
}

function shelterNew(): CueDef {
  const c = new Cue(2, 6);
  // A fresh paving pad in front, the new shelter on it, and a new stop flag on its pole beside it.
  c.rect(-2, 10, 24, 6, "q");
  c.rect(-2, 15, 24, 1, "p");
  for (const x of [4, 10, 16]) c.rect(x, 10, 1, 5, "p");
  shelter(c, "new");
  c.rect(20, -12, 1, 26, "i");
  c.rect(20, -12, 1, 1, "I");
  c.paste(18, -16, `
    .RRR.
    RcccR
    RcbcR
    RcccR
    .RRR.`);
  c.rect(19, -10, 3, 3, "c");
  c.rect(19, -10, 3, 1, "b");
  return c.def({ shadow: { box: [-2, 7, 21, 10], height: 28 } });
}

function swingTaped(): CueDef {
  const c = new Cue(5, 5);
  swings(c, "taped");
  return c.def({ shadow: { box: [0, 9, 17, 13], height: 18 } });
}

function swingVan(mirror: boolean): CueDef {
  const c = new Cue(4, 26);
  swings(c, "off");
  // The seat off and on the grass, a toolbox beside it, the van on the green, a cone.
  c.rect(1, 13, 5, 2, "b");
  c.rect(1, 13, 5, 1, "l");
  c.rect(3, 12, 1, 1, "i");
  van(c, 22, 12);
  cone(c, 18, 15);
  return c.def({ shadow: { box: [0, 9, 40, 13], height: 18 } }, mirror);
}

function swingNew(): CueDef {
  const c = new Cue(5, 5);
  swings(c, "new");
  return c.def({ shadow: { box: [0, 9, 17, 13], height: 18 } });
}

function faultOpen(): CueDef {
  const c = new Cue(0, 0);
  // Two pins with tape between them, and a hazard sign.
  for (const x of [2, 13]) {
    c.rect(x, 3, 1, 11, "i");
    c.set(x, 2, "I");
  }
  tape(c, 2, 5, 13, 5);
  tape(c, 2, 6, 13, 6);
  c.paste(6, 7, `
    ..A..
    .AYA.
    AYxYA
    AAAAA
    ..i..
    ..i..`);
  return c.def();
}

function faultRepairing(): CueDef {
  const c = new Cue(0, 0);
  barrier(c, 1, 2, 14);
  cone(c, 3, 14);
  cone(c, 12, 14);
  return c.def();
}

function faultNew(): CueDef {
  const c = new Cue(0, 0);
  // Fresh paving: clean slabs with crisp joints.
  c.rect(2, 4, 12, 10, "q");
  c.rect(2, 8, 12, 1, "p");
  c.rect(8, 4, 1, 10, "p");
  c.rect(2, 13, 12, 1, "P");
  c.set(4, 5, "c");
  c.set(11, 10, "c");
  return c.def();
}

/**
 * Every cue the kit draws, by the name content uses (`faults.jsonc` `cue`);
 * the prop is `cue_<name>`. Variants: the cues with a van have a second,
 * mirrored look (the van on the other side), which the street picks to keep
 * the van off a house front.
 */
export function faultCueProps(): Record<string, readonly CueDef[]> {
  const cues: Record<string, readonly CueDef[]> = {
    lamp_dark: [lampDark()],
    lamp_van: [lampVan(false), lampVan(true)],
    lamp_new: [lampNew()],
    drain_flooded: [drainFlooded()],
    drain_cones: [drainCones()],
    drain_new: [drainNew()],
    pothole: [pothole()],
    road_cones: [roadCones()],
    tarmac_new: [tarmacNew()],
    shelter_taped: [shelterTaped()],
    shelter_van: [shelterVan(false), shelterVan(true)],
    shelter_new: [shelterNew()],
    swing_taped: [swingTaped()],
    swing_van: [swingVan(false), swingVan(true)],
    swing_new: [swingNew()],
    fault_open: [faultOpen()],
    fault_repairing: [faultRepairing()],
    fault_new: [faultNew()],
  };
  return Object.fromEntries(Object.entries(cues).map(([k, v]) => [`cue_${k}`, v]));
}

/** The cue names drawn (without the `cue_` prefix). */
export const FAULT_CUES: readonly string[] = Object.keys(faultCueProps()).map((k) => k.slice(4));
/** The stand-ins for a cue with no drawing, by status. */
export const FALLBACK_CUES = { open: "fault_open", repairing: "fault_repairing", fixed: "fault_new" } as const;
