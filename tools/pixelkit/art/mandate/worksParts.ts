// The works kit's drawn parts (art round 8; DEC-099 "Works you can see";
// PIXEL_ART_TOOL.md "Round 8: works"): site furniture (a portable cabin, a
// tower crane, a small excavator, material piles, a skip, a hazard sign, a
// "work in progress" board and its faded and peeling looks, cones, a road
// works sign), machines and industry (the mine's headframe turning, rusted
// and half-built, a flat-topped spoil tip, a works chimney and its smoke, a
// lorry, stacked goods, yard gates open and chained), the abandoned looks
// (a plastic sheet flapping, buddleia, rubble) and the finished works' own
// props (the sponsor's plaque on a plinth or a wall, and the ribbon on its
// stands for the opening).
//
// Parts are drawn by hand as shape rules into character canvases, as props2
// and props3 are, in the master palette's ramps (no colour is added). Each
// gets the exterior outline (D2): the 1 px line goes round the silhouette
// only, so the holes of a lattice, a mesh or a gate stay open to the ground.
// Thin things that hang in the air (ropes, ties) are drawn after the
// outline, so they stay 1 px. Every part has a foot: the pixel that stands
// on the map point the placement gives (its depth is that point's row).
//
// Legend (character → ramp, step):
//   Y y j  lamp 0, -1, +1 (yellow paint)       i I    iron 0, +1
//   g G h  glass 0, -1, +1                      c C    chalk 0, -1
//   p P Q q  paving 0, -1, -2, +1 (concrete)    s S t T  stone 0, -1, +1, -2
//   w W k  wood 0, -1, +1                       f F    field 0, +1 (sand)
//   a A r  flag 0, -1, +1 (hazard orange)       R H    cloth red 0, -1 (ribbon)
//   b B l  party blue 0, -1, +1 (council)       D      cloth navy 0
//   v u U  terracotta 0, -1, -2 (rust)          K L X V  brick 0, +1, -1, -2
//   n N m M  grass 0, -1, +1, foliage -2        o O x  ink -1, -2, 0
//   z Z d  asphalt 0, -1, +1                    ~ ^ _  water 0, +1, -1
//   3 4 5 6  teal, plum light, party green, sky light (graffiti, buddleia)
//   7 8 9  render 0, +1, -1                     e E    slate 0, +1 (faded paint)
import { hash32 } from "../../src/core/hash";
import { blank, type Indexed } from "../../src/core/image";
import type { Palette } from "../../src/core/palette";
import { round4Props } from "./props2";
import { legendFor } from "./tiles";

export const WORKS_LEGEND_RAMPS: Readonly<Record<string, readonly [string, number]>> = {
  Y: ["lamp", 0],
  y: ["lamp", -1],
  j: ["lamp", 1],
  i: ["iron", 0],
  I: ["iron", 1],
  g: ["glass", 0],
  G: ["glass", -1],
  h: ["glass", 1],
  c: ["chalk", 0],
  C: ["chalk", -1],
  p: ["paving", 0],
  P: ["paving", -1],
  Q: ["paving", -2],
  q: ["paving", 1],
  s: ["stone", 0],
  S: ["stone", -1],
  t: ["stone", 1],
  T: ["stone", -2],
  w: ["wood", 0],
  W: ["wood", -1],
  k: ["wood", 1],
  f: ["field", 0],
  F: ["field", 1],
  a: ["flag", 0],
  A: ["flag", -1],
  r: ["flag", 1],
  R: ["cloth_red", 0],
  H: ["cloth_red", -1],
  b: ["party_blue", 0],
  B: ["party_blue", -1],
  l: ["party_blue", 1],
  D: ["cloth_navy", 0],
  v: ["terracotta", 0],
  u: ["terracotta", -1],
  U: ["terracotta", -2],
  K: ["brick", 0],
  L: ["brick", 1],
  X: ["brick", -1],
  V: ["brick", -2],
  n: ["grass", 0],
  N: ["grass", -1],
  m: ["grass", 1],
  M: ["foliage", -2],
  o: ["ink", -1],
  O: ["ink", -2],
  x: ["ink", 0],
  z: ["asphalt", 0],
  Z: ["asphalt", -1],
  d: ["asphalt", 1],
  "~": ["water", 0],
  "^": ["water", 1],
  _: ["water", -1],
  "3": ["cloth_teal", 0],
  "4": ["cloth_plum", 1],
  "5": ["party_green", 0],
  "6": ["cloth_sky", 1],
  "7": ["render", 0],
  "8": ["render", 1],
  "9": ["render", -1],
  e: ["slate", 0],
  E: ["slate", 1],
};

/** Legend characters to palette indices for a palette. */
export function worksLegend(p: Palette): Record<string, number> {
  return legendFor(p, WORKS_LEGEND_RAMPS);
}

// ---------------------------------------------------------------------------
// Canvases

/** A character canvas for shape rules: '.' is empty. */
export class Chars {
  readonly w: number;
  readonly h: number;
  readonly c: string[];
  constructor(w: number, h: number) {
    this.w = w;
    this.h = h;
    this.c = new Array<string>(w * h).fill(".");
  }
  static from(text: string): Chars {
    const rows = text.split("\n").map((r) => r.trim()).filter((r) => r.length);
    const out = new Chars((rows[0] as string).length, rows.length);
    rows.forEach((r, y) => [...r].forEach((ch, x) => out.set(x, y, ch)));
    return out;
  }
  clone(): Chars {
    const o = new Chars(this.w, this.h);
    for (let i = 0; i < this.c.length; i++) o.c[i] = this.c[i] as string;
    return o;
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
  /** Replace characters through a map (a recolour of the drawing). */
  swap(map: Readonly<Record<string, string>>, where: (x: number, y: number) => boolean = () => true) {
    for (let y = 0; y < this.h; y++) {
      for (let x = 0; x < this.w; x++) {
        const ch = this.get(x, y);
        const to = map[ch];
        if (to !== undefined && where(x, y)) this.set(x, y, to);
      }
    }
  }
}

/** A part as drawn: frames of the body (outlined) and of thin overlays (not outlined), the foot, lit characters, a shadow. */
export interface PartDef {
  id: string;
  frames: Chars[];
  /** Drawn after the outline (ropes, ties), per frame; same size as the body. */
  thin?: Chars[];
  /** The point that stands on the ground (canvas px). */
  foot: readonly [number, number];
  /** Characters that light up at night (emissive), as `lamp` colours. */
  lit?: string;
  /** No outline (smoke, flat marks). */
  bare?: boolean;
  /** A shadow caster: box relative to the foot (x0, y0, x1, y1, ground px) and height. */
  shadow?: { box: readonly [number, number, number, number]; height: number };
}

/** A part ready to place: palette images per frame (1 px padding for the outline), the foot in them, lit layers, a shadow. */
export interface WorksSprite {
  id: string;
  frames: Indexed[];
  emissive: (Indexed | null)[];
  fx: number;
  fy: number;
  shadow: { box: readonly [number, number, number, number]; height: number } | null;
}

const NEIGHBOURS = [
  [0, 1],
  [0, -1],
  [-1, 0],
  [1, 0],
] as const;

/**
 * The exterior outline (D2) without filling holes: only transparent pixels
 * connected to the image's border (4-connected) take the outline, so the
 * openings of a lattice or a mesh stay transparent. Where a pixel touches
 * several colours, the darkest line wins (`lightness`), as core `outline`.
 */
export function outlineExterior(img: Indexed, lineOf: ArrayLike<number>, lightness: ArrayLike<number>): Indexed {
  const { w, h, px } = img;
  const outside = new Uint8Array(w * h);
  const stack: number[] = [];
  const push = (x: number, y: number) => {
    const i = y * w + x;
    if (outside[i] || px[i] !== 0) return;
    outside[i] = 1;
    stack.push(i);
  };
  for (let x = 0; x < w; x++) {
    push(x, 0);
    push(x, h - 1);
  }
  for (let y = 0; y < h; y++) {
    push(0, y);
    push(w - 1, y);
  }
  while (stack.length) {
    const i = stack.pop() as number;
    const x = i % w;
    const y = (i - x) / w;
    for (const [dx, dy] of NEIGHBOURS) {
      const nx = x + dx;
      const ny = y + dy;
      if (nx >= 0 && ny >= 0 && nx < w && ny < h) push(nx, ny);
    }
  }
  const out: Indexed = { w, h, px: px.slice() };
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = y * w + x;
      if (!outside[i]) continue;
      let best = 0;
      let bestL = Infinity;
      for (const [dx, dy] of NEIGHBOURS) {
        const nx = x + dx;
        const ny = y + dy;
        if (nx < 0 || ny < 0 || nx >= w || ny >= h) continue;
        const v = px[ny * w + nx] as number;
        if (v === 0) continue;
        const line = lineOf[v] as number;
        const l = lightness[line] as number;
        if (l < bestL) {
          bestL = l;
          best = line;
        }
      }
      if (best) out.px[i] = best;
    }
  }
  return out;
}

/** Characters to palette indices, with a pixel of room all round. */
export function charsToIndexed(c: Chars, legend: Readonly<Record<string, number>>, pad = 1): Indexed {
  const img = blank(c.w + 2 * pad, c.h + 2 * pad);
  for (let y = 0; y < c.h; y++) {
    for (let x = 0; x < c.w; x++) {
      const ch = c.get(x, y);
      if (ch === ".") continue;
      const v = legend[ch];
      if (v === undefined) throw new Error(`works: unknown character '${ch}'`);
      img.px[(y + pad) * img.w + x + pad] = v;
    }
  }
  return img;
}

/** Build a part: outline each frame's body, lay its thin overlay on top, and its lit layer. */
export function buildPart(d: PartDef, p: Palette, lightness: readonly number[], legend: Readonly<Record<string, number>>, litLegend: Readonly<Record<string, number>>): WorksSprite {
  const frames: Indexed[] = [];
  const emissive: (Indexed | null)[] = [];
  d.frames.forEach((f, k) => {
    const body = charsToIndexed(f, legend);
    const img = d.bare ? body : outlineExterior(body, p.lineOf, lightness);
    const thin = d.thin?.[k];
    if (thin) {
      const t = charsToIndexed(thin, legend);
      for (let i = 0; i < t.px.length; i++) if (t.px[i]) img.px[i] = t.px[i] as number;
    }
    frames.push(img);
    if (d.lit) {
      const e = blank(img.w, img.h);
      let any = false;
      for (let y = 0; y < f.h; y++) {
        for (let x = 0; x < f.w; x++) {
          const ch = f.get(x, y);
          if (!d.lit.includes(ch)) continue;
          e.px[(y + 1) * e.w + x + 1] = (litLegend[ch] ?? litLegend.default) as number;
          any = true;
        }
      }
      emissive.push(any ? e : null);
    } else emissive.push(null);
  });
  return { id: d.id, frames, emissive, fx: d.foot[0] + 1, fy: d.foot[1] + 1, shadow: d.shadow ?? null };
}

// ---------------------------------------------------------------------------
// Site furniture

/** A portable site cabin: a flat roof, ribbed walls, a window (lit while the site works), a door up two steps, on blocks. */
function cabin(broken: boolean): PartDef {
  const W = 30;
  const c = new Chars(W, 20);
  c.rect(0, 0, W, 3, "c");
  c.rect(0, 3, W, 1, "C");
  c.rect(0, 4, W, 12, "C");
  for (let x = 2; x < W; x += 4) c.rect(x, 4, 1, 12, "q");
  c.rect(0, 4, 1, 12, "c");
  c.rect(W - 1, 4, 1, 12, "p");
  c.rect(0, 15, W, 1, "P");
  // The window: a light frame, glass lit from the upper left.
  c.rect(3, 6, 10, 7, "c");
  c.rect(4, 7, 8, 5, "g");
  c.rect(4, 7, 3, 2, "h");
  c.rect(4, 11, 8, 1, "G");
  c.rect(8, 7, 1, 5, "c");
  // The door: council blue, a handle, two timber steps.
  c.rect(19, 5, 6, 11, "b");
  c.rect(24, 5, 1, 11, "B");
  c.set(20, 11, "Y");
  c.rect(18, 16, 8, 1, "k");
  c.rect(18, 17, 8, 1, "W");
  // Blocks under the corners, the shade under the cabin.
  c.rect(1, 16, 16, 2, "o");
  c.rect(1, 16, 3, 3, "Q");
  c.rect(26, 16, 3, 3, "Q");
  c.rect(26, 16, 1, 1, "P");
  // Within the colour budget (round 7: 14 a part): ribs, handle, steps and the shade under it from colours already on the cabin.
  c.swap({ q: "p", Y: "c", k: "P", W: "Q", o: "Q" });
  if (broken) {
    // A pane gone: dark, a shard left in its corner; the door kicked in at the foot; a tag of rust under the roof.
    c.rect(4, 7, 4, 5, "O");
    c.set(4, 7, "h");
    c.set(5, 7, "h");
    c.set(4, 8, "h");
    c.rect(19, 13, 5, 3, "O");
    c.set(19, 13, "B");
    for (const x of [6, 14, 22]) c.rect(x, 4, 1, 2, "u");
  }
  return { id: broken ? "cabin_broken" : "cabin", frames: [c], foot: [15, 18], lit: broken ? "" : "gh", shadow: { box: [-15, -6, 15, 0], height: 16 } };
}

/**
 * A tower crane: a lattice mast on a concrete base, a lattice jib to the
 * right and a counter-jib with its weights to the left, the cab at the top,
 * the apex with its ties (thin), a trolley and the hook on its rope (thin).
 * `load`: a bundle of steel beams on the hook (the crane at work), else the
 * hook drawn up. `rust`: the yellow gone to rust in patches. A red warning
 * light at the jib's tip is lit while the site works.
 */
function towerCrane(load: boolean, rust: boolean): PartDef {
  const W = 96;
  const H = 110;
  const c = new Chars(W, H);
  const t = new Chars(W, H);
  const mx = 22; // the mast's left chord
  const top = 14; // the jib's top chord
  const base = H - 1;
  // Mast: two chords 6 px apart, a zigzag of bracing every 6 px (open panels).
  for (let y = top + 6; y < base - 3; y++) {
    c.set(mx, y, "Y");
    c.set(mx + 6, y, "y");
  }
  for (let y = top + 6; y < base - 3; y += 6) {
    c.line(mx, y, mx + 6, y, "y");
    c.line(mx + 1, y + 1, mx + 5, y + 5, "Y");
  }
  // The base: a concrete footing with its bolts.
  c.rect(mx - 5, base - 3, 17, 4, "p");
  c.rect(mx - 5, base - 3, 17, 1, "q");
  c.rect(mx - 5, base, 17, 1, "P");
  c.set(mx - 3, base - 2, "i");
  c.set(mx + 9, base - 2, "i");
  // Jib: top and bottom chords, zigzag, tapering a little to the tip.
  const tip = W - 2;
  c.rect(mx - 1, top, tip - mx + 2, 1, "j");
  c.rect(mx - 1, top + 5, tip - mx + 2, 1, "y");
  for (let x = mx + 7; x < tip; x += 5) {
    c.line(x, top + 1, x + 2, top + 4, "Y");
    c.line(x + 3, top + 4, x + 5, top + 1, "Y");
  }
  c.rect(tip, top, 1, 6, "Y");
  // Counter-jib: a walkway to the left with the counterweights hanging off its end.
  c.rect(1, top + 2, mx - 1, 1, "j");
  c.rect(1, top + 5, mx - 1, 1, "y");
  for (let x = 2; x < mx - 1; x += 4) c.line(x, top + 3, x + 2, top + 4, "Y");
  c.rect(2, top + 6, 9, 9, "p");
  c.rect(2, top + 6, 9, 1, "q");
  c.rect(2, top + 9, 9, 1, "P");
  c.rect(2, top + 12, 9, 1, "P");
  c.rect(10, top + 6, 1, 9, "P");
  // The cab under the jib on the right of the mast, glazed.
  c.rect(mx + 7, top + 6, 7, 6, "Y");
  c.rect(mx + 8, top + 7, 5, 3, "g");
  c.rect(mx + 8, top + 7, 2, 1, "h");
  c.rect(mx + 7, top + 11, 7, 1, "y");
  // The apex above the mast, and its ties to the jib's tip and the counter-jib's end (thin).
  for (let y = 2; y < top; y++) {
    const k = Math.floor((top - y) / 3);
    c.set(mx + k, y, "Y");
    c.set(mx + 6 - k, y, "y");
  }
  c.set(mx + 3, 1, "y");
  t.line(mx + 3, 2, tip - 4, top - 1, "i");
  t.line(mx + 3, 2, 3, top + 1, "i");
  // The trolley on the jib and the hook.
  const tx = load ? 66 : 74;
  c.rect(tx - 2, top + 6, 6, 2, "i");
  const hookY = load ? 62 : top + 16;
  t.rect(tx, top + 8, 1, hookY - top - 8, "x");
  t.rect(tx + 1, top + 8, 1, hookY - top - 8, "x");
  c.rect(tx - 1, hookY, 4, 3, "a");
  c.rect(tx - 1, hookY + 2, 4, 1, "A");
  c.set(tx, hookY + 3, "i");
  if (load) {
    // Slings to a bundle of steel beams.
    t.line(tx, hookY + 4, tx - 7, hookY + 9, "x");
    t.line(tx + 1, hookY + 4, tx + 8, hookY + 9, "x");
    c.rect(tx - 9, hookY + 10, 20, 2, "I");
    c.rect(tx - 9, hookY + 12, 20, 1, "i");
    c.rect(tx - 8, hookY + 13, 18, 2, "I");
    c.rect(tx - 8, hookY + 15, 18, 1, "i");
  }
  // The warning light at the tip.
  c.set(tip, top - 1, "r");
  // Within the colour budget: one glass tone, the hook one orange, the load and footing in colours already used.
  c.swap({ h: "g", A: "a", q: "p", I: "i" });
  if (rust) {
    // Rust: the paint gone in patches along the members, darker where water runs.
    c.swap({ Y: "v", j: "v", y: "u" }, (x, y) => hash32(x >> 2, y >> 2, 0xc7a) % 3 === 0);
    c.swap({ Y: "y" }, (x, y) => hash32(x >> 1, y >> 3, 0x51) % 4 === 0);
    c.swap({ r: "u" });
  }
  return { id: rust ? (load ? "crane_load_rust" : "crane_rust") : load ? "crane_load" : "crane", frames: [c], thin: [t], foot: [mx + 3, base], lit: rust ? "" : "r", shadow: { box: [-8, -6, 9, 0], height: 90 } };
}

/**
 * A small tracked excavator facing left: tracks with their rollers, the
 * yellow upper body and counterweight, a glazed cab, the boom and arm with a
 * hydraulic ram, the bucket. Frames: digging (the bucket down on the ground)
 * and lifting (the arm raised). Parked is the digging frame. `rust`: faded
 * and rusted, the glass gone dark.
 */
function excavator(rust: boolean): PartDef {
  const frames: Chars[] = [];
  for (const lift of [false, true]) {
    const c = new Chars(32, 24);
    // Tracks: a dark belt, light top run, rollers.
    c.rect(9, 18, 22, 5, "x");
    c.rect(10, 17, 20, 1, "I");
    c.rect(9, 23, 22, 1, "O");
    for (let x = 11; x < 30; x += 5) {
      c.rect(x, 19, 3, 3, "i");
      c.set(x + 1, 20, "I");
    }
    // The upper body on its turntable, the counterweight at the back (right).
    c.rect(12, 15, 16, 2, "i");
    c.rect(11, 9, 19, 6, "Y");
    c.rect(11, 9, 19, 1, "j");
    c.rect(11, 13, 19, 2, "y");
    c.rect(26, 8, 5, 7, "y");
    c.rect(26, 8, 5, 1, "Y");
    c.rect(28, 10, 2, 1, "O");
    // The cab: a frame of yellow, glazing lit from the upper left.
    c.rect(12, 2, 9, 8, "Y");
    c.rect(12, 2, 9, 1, "j");
    c.rect(13, 3, 7, 5, "g");
    c.rect(13, 3, 3, 2, "h");
    c.rect(13, 7, 7, 1, "G");
    c.rect(20, 3, 1, 6, "y");
    // Boom from the body's front up and out to the left, then the arm and bucket.
    if (!lift) {
      c.line(12, 10, 5, 3, "Y");
      c.line(12, 11, 5, 4, "y");
      c.line(11, 11, 4, 4, "y");
      c.line(5, 3, 3, 15, "Y");
      c.line(4, 3, 2, 15, "y");
      c.line(9, 7, 4, 10, "I");
      // The bucket on the ground, its teeth biting in.
      c.rect(0, 15, 6, 4, "i");
      c.rect(1, 15, 4, 1, "I");
      c.rect(0, 19, 6, 1, "o");
      c.set(0, 20, "i");
      c.set(2, 20, "i");
      c.set(4, 20, "i");
    } else {
      c.line(12, 10, 6, 1, "Y");
      c.line(12, 11, 6, 2, "y");
      c.line(11, 11, 5, 2, "y");
      c.line(6, 1, 1, 7, "Y");
      c.line(5, 1, 0, 7, "y");
      c.line(9, 6, 4, 4, "I");
      c.rect(0, 8, 5, 4, "i");
      c.rect(0, 8, 5, 1, "I");
      c.set(1, 12, "i");
      c.set(3, 12, "i");
      c.rect(1, 9, 3, 1, "e");
    }
    if (rust) {
      c.swap({ Y: "v", j: "y" }, (x, y) => hash32(x >> 1, y >> 1, 0xe7) % 3 === 0);
      c.swap({ y: "u" }, (x, y) => hash32(x, y >> 1, 0x3e) % 3 === 0);
      c.swap({ g: "x", h: "G" });
    }
    frames.push(c);
  }
  return { id: rust ? "excavator_rust" : "excavator", frames: rust ? [frames[0] as Chars] : frames, foot: [18, 23], shadow: { box: [-18, -6, 13, 0], height: 14 } };
}

/** A skip: a hazard-orange tapered steel box, its lip, rubbish piled in it. */
function skip(): PartDef {
  const c = new Chars(18, 10);
  c.rect(1, 3, 16, 6, "a");
  c.rect(0, 3, 18, 1, "r");
  c.rect(1, 8, 16, 1, "A");
  c.rect(16, 3, 1, 6, "A");
  c.set(0, 8, ".");
  c.set(17, 8, ".");
  c.rect(4, 5, 2, 2, "A");
  c.rect(12, 5, 2, 2, "A");
  c.rect(2, 1, 5, 2, "p");
  c.rect(7, 0, 4, 3, "w");
  c.rect(11, 1, 4, 2, "P");
  c.set(8, 0, "k");
  c.rect(2, 9, 14, 1, "o");
  return { id: "skip", frames: [c], foot: [9, 9] };
}

/** Material piles: a sand heap, concrete pipes, a pallet of bricks, timber, sheet piles, rusty rebar on chocks, rubble. */
function piles(): PartDef[] {
  const sand = new Chars(18, 8);
  for (let y = 0; y < 8; y++) {
    const half = Math.round(2 + (y * 7) / 7);
    for (let x = 9 - half; x < 9 + half; x++) sand.set(x, y, x < 9 - 1 ? (y < 2 ? "F" : "f") : x < 11 ? "f" : "S");
  }
  sand.rect(4, 5, 2, 1, "F");
  sand.rect(13, 6, 2, 1, "T");

  const pipes = new Chars(20, 13);
  const ring = (cx: number, cy: number) => {
    for (let y = -3; y <= 3; y++) {
      for (let x = -3; x <= 3; x++) {
        const d = x * x + y * y;
        if (d > 11) continue;
        pipes.set(cx + x, cy + y, d <= 3 ? (y < 0 ? "x" : "O") : x + y < 0 ? "q" : x + y > 2 ? "P" : "p");
      }
    }
  };
  ring(4, 9);
  ring(11, 9);
  ring(8, 3);
  pipes.rect(15, 7, 5, 6, "p");
  pipes.rect(15, 7, 5, 1, "q");
  pipes.rect(19, 7, 1, 6, "P");

  const bricks = new Chars(14, 11);
  bricks.rect(0, 9, 14, 2, "w");
  bricks.rect(0, 10, 14, 1, "W");
  for (const x of [3, 7, 11]) bricks.set(x, 10, "O");
  bricks.rect(1, 1, 12, 8, "K");
  for (let y = 1; y < 9; y += 2) bricks.rect(1, y, 12, 1, "X");
  for (let y = 2; y < 9; y += 2) for (let x = y % 4 === 0 ? 3 : 1; x < 13; x += 4) bricks.set(x, y, "X");
  bricks.rect(1, 1, 12, 1, "L");
  bricks.rect(1, 1, 1, 8, "h");
  bricks.rect(12, 1, 1, 8, "V");

  const timber = new Chars(20, 8);
  for (let r = 0; r < 3; r++) {
    const y = 2 + r * 2;
    timber.rect(r === 0 ? 2 : 0, y, r === 0 ? 16 : 20, 2, "w");
    timber.rect(r === 0 ? 2 : 0, y, r === 0 ? 16 : 20, 1, "k");
    for (let x = r === 0 ? 2 : 0; x < 20; x += 4) timber.set(x, y + 1, "W");
  }
  timber.rect(0, 7, 20, 1, "W");

  const sheets = new Chars(20, 7);
  for (let y = 1; y < 7; y++) for (let x = 0; x < 20; x++) sheets.set(x, y, y === 1 ? "I" : (x + (y >> 1)) % 4 < 2 ? "i" : "x");
  for (let x = 0; x < 20; x += 2) sheets.set(x, 0, x % 4 === 0 ? "I" : ".");

  const rebar = new Chars(22, 6);
  rebar.rect(2, 4, 3, 2, "w");
  rebar.rect(16, 4, 3, 2, "w");
  for (let r = 0; r < 4; r++) rebar.rect(r % 2, r, 22 - (r % 2), 1, r % 2 ? "U" : "u");
  rebar.rect(0, 3, 22, 1, "U");

  const rubble = new Chars(20, 9);
  for (let x = 0; x < 20; x++) {
    const top = Math.round(8 - 7 * Math.max(0, 1 - Math.abs(x - 9.5) / 10) - (hash32(x, 0x7b) % 2));
    for (let y = Math.max(0, top); y < 9; y++) {
      const hsh = hash32(x >> 1, y >> 1, 0xb1) % 7;
      rubble.set(x, y, hsh === 0 ? "K" : hsh === 1 ? "X" : hsh === 2 ? "q" : x < 9 ? "p" : "P");
    }
  }
  rubble.rect(3, 8, 15, 1, "Q");

  return [
    { id: "pile_sand", frames: [sand], foot: [9, 7] },
    { id: "pile_pipes", frames: [pipes], foot: [10, 12] },
    { id: "pile_bricks", frames: [bricks], foot: [7, 10] },
    { id: "pile_timber", frames: [timber], foot: [10, 7] },
    { id: "pile_sheets", frames: [sheets], foot: [10, 6] },
    { id: "pile_rebar", frames: [rebar], foot: [11, 5] },
    { id: "rubble", frames: [rubble], foot: [10, 8] },
  ];
}

/** A hazard sign: a yellow triangle with a dark border and mark, on a post. */
function hazardSign(): PartDef {
  const c = new Chars(11, 20);
  for (let y = 0; y < 9; y++) {
    const half = Math.floor((y + 1) / 2) + (y > 0 ? 0 : 0);
    for (let x = 5 - half; x <= 5 + half; x++) c.set(x, y, x === 5 - half || x === 5 + half || y === 8 ? "O" : "Y");
  }
  c.rect(5, 3, 1, 3, "O");
  c.set(5, 7, "O");
  c.rect(5, 9, 1, 11, "I");
  c.rect(4, 19, 3, 1, "i");
  return { id: "hazard_sign", frames: [c], foot: [5, 19] };
}

/** A temporary road works sign: a red-bordered white triangle with the digging-man mark, on an A-frame weighted with a sandbag. */
function roadSign(faded: boolean): PartDef {
  const c = new Chars(13, 20);
  for (let y = 0; y < 11; y++) {
    const half = Math.floor((y + 1) / 2) + 1;
    for (let x = 6 - half; x <= 6 + half; x++) c.set(x, y, Math.abs(x - 6) >= half - 1 || y >= 9 ? "A" : "c");
  }
  c.rect(6, 0, 1, 1, "A");
  // The digging man: head, body, the spade and its heap.
  c.set(5, 4, "O");
  c.line(5, 5, 6, 7, "O");
  c.line(7, 5, 8, 6, "O");
  c.rect(7, 7, 2, 1, "O");
  c.line(1, 11, 4, 19, "i");
  c.line(11, 11, 8, 19, "i");
  c.rect(4, 18, 5, 2, "w");
  c.rect(4, 18, 5, 1, "k");
  if (faded) {
    c.swap({ A: "u", c: "s" });
    c.swap({ s: "S" }, (x, y) => (x + y) % 5 === 0);
  }
  return { id: faded ? "road_sign_faded" : "road_sign", frames: [c], foot: [6, 19] };
}

/** A traffic cone: orange with a white band, on its square foot. Down: knocked over, lying on its side. */
function cones(): PartDef[] {
  const up = Chars.from(`
    ..a..
    ..aA.
    .caC.
    .cCC.
    .aaA.
    .aaAA
    OOOOO`);
  const down = Chars.from(`
    ...O....
    .aacaa..
    OaacCaA.
    OAAcCAAA
    ...O....`);
  return [
    { id: "cone", frames: [up], foot: [2, 6] },
    { id: "cone_down", frames: [down], foot: [4, 4] },
  ];
}

/**
 * The "work in progress" board on two posts: a council-blue header with a
 * crane mark, an artist's impression of the finished works and lines of
 * text as bars (no letters, R14). Faded: the blue gone to slate, the white
 * yellowed and streaked. Peeling: as faded, a corner of the face torn away
 * and hanging, one post leaning.
 */
function board(state: 0 | 1 | 2): PartDef {
  const W = 24;
  const c = new Chars(W, 24);
  c.rect(4, 12, 2, 12, "w");
  c.rect(18, 12, 2, 12, "w");
  c.rect(5, 12, 1, 12, "W");
  c.rect(19, 12, 1, 12, "W");
  c.rect(0, 0, W, 15, "c");
  c.rect(0, 0, W, 4, "b");
  c.rect(0, 3, W, 1, "B");
  // The crane mark in the header.
  c.rect(2, 1, 5, 1, "c");
  c.rect(3, 1, 1, 2, "c");
  // The artist's impression: sky, a building, grass.
  c.rect(2, 6, 8, 6, "h");
  c.rect(3, 8, 6, 3, "K");
  c.rect(3, 8, 6, 1, "L");
  c.rect(5, 9, 2, 2, "g");
  c.rect(2, 11, 8, 1, "n");
  for (const [y, w] of [[6, 10], [8, 9], [10, 11], [12, 7]] as const) c.rect(12, y, w, 1, "C");
  c.rect(0, 14, W, 1, "C");
  c.rect(W - 1, 0, 1, 15, "C");
  if (state >= 1) {
    c.swap({ b: "E", B: "e", c: "s", h: "C", K: "v", L: "v", g: "e", n: "m" });
    c.swap({ s: "S" }, (x, y) => (x * 3 + y) % 7 === 0 || (y > 4 && (x + 2 * y) % 11 === 0));
    c.swap({ C: "S" });
  }
  if (state >= 2) {
    // The top right corner torn away (the post behind shows), a flap hanging off the edge.
    for (let y = 0; y < 8; y++) for (let x = W - 9 + y; x < W; x++) c.set(x, y, ".");
    c.rect(19, 0, 1, 2, "w");
    c.line(W - 9, 0, W - 2, 7, "S");
    c.rect(W - 4, 8, 3, 4, "s");
    c.rect(W - 2, 9, 2, 4, "S");
    // The left post leaning out at its foot.
    c.rect(4, 18, 2, 6, ".");
    c.rect(5, 12, 2, 1, "w");
    c.rect(3, 18, 2, 6, "w");
    c.rect(4, 18, 1, 6, "W");
  }
  return { id: ["board", "board_faded", "board_peeling"][state] as string, frames: [c], foot: [12, 23] };
}

/** A plastic sheet hanging from a slab edge, flapping in the wind (two frames). */
function sheet(): PartDef {
  const a = Chars.from(`
    hhhhhhhhhh
    hCCChhhChh
    hCChhhCChh
    .hCChhCCh.
    .hhCChhCh.
    .hhCCchCh.
    ..hhCChh..
    ..hhhCCh..
    ...hhCh...
    ...hChh...
    ....hh....`);
  const b = Chars.from(`
    hhhhhhhhhh
    hhCChhhCCh
    .hhCChhhCh
    ..hhCChhCh
    ...hhCChhh
    ....hhCChh
    .....hhCCh
    .....hhhCh
    ......hhhC
    .......hhh
    ........hh`);
  return { id: "sheet", frames: [a, b], foot: [5, 0] };
}

/** Buddleia: a bush seeded in a ruin, purple flower spikes over grey-green leaves. */
function buddleia(): PartDef {
  const c = Chars.from(`
    ..4.....4...
    .44..4..44..
    .4.m.44.4...
    ..mmN.4m....
    .mmNNmmNm...
    mNmNNmNNmm..
    .NNMNNNMNm..
    ..NNM.NNN...
    ....M.M.....
    ....M.......`);
  return { id: "buddleia", frames: [c], foot: [4, 9] };
}

// ---------------------------------------------------------------------------
// Machines and industry

/** The round 4 headframe's grid as a canvas (its own legend characters map into this kit's legend; see HEADFRAME_MAP). */
function headframeChars(): Chars {
  const def = round4Props().headframe?.[0];
  if (!def) throw new Error("works: the round 4 headframe is missing");
  const c = Chars.from(def.grid);
  // Props legend → works legend (the same ramps): R brick, T brick shade, O ink; P p q c i I are the same characters here.
  c.swap({ R: "K", T: "X", O: "x" });
  return c;
}

/** The headframe's wheels: centres and radius (props2.ts round 4). */
const WHEELS = [
  { cx: 7, cy: 8, dir: 1 },
  { cx: 20, cy: 8, dir: -1 },
] as const;
/** Spoke vectors for three turning frames (0°, about 27° and 63°: a quarter turn repeats). */
const SPOKES: readonly (readonly [number, number])[] = [
  [5, 0],
  [4, 2],
  [2, 4],
];

/**
 * The mine's headframe (round 4's, extended): three frames of its winding
 * wheels turning (the left and right wheel turn opposite ways, as the two
 * ropes run); rusted (the red paint gone to rust); and half-built (only the
 * legs and the banksman's house up to a girder, for a shaft being sunk).
 */
function headframes(): PartDef[] {
  const base = headframeChars();
  const turning = SPOKES.map(([vx, vy]) => {
    const c = base.clone();
    for (const w of WHEELS) {
      for (let y = -5; y <= 5; y++) for (let x = -5; x <= 5; x++) if (x * x + y * y <= 25) c.set(w.cx + x, w.cy + y, ".");
      const v: [number, number] = [vx, vy * w.dir];
      const spokes: [number, number][] = [v, [-v[1], v[0]], [-v[0], -v[1]], [v[1], -v[0]]];
      for (const [sx, sy] of spokes) c.line(w.cx, w.cy, w.cx + sx, w.cy + sy, "i");
      c.set(w.cx, w.cy, "c");
    }
    return c;
  });
  const rust = (turning[0] as Chars).clone();
  // Rust: the red paint gone brown and orange in patches, streaked darker down the legs.
  rust.swap({ K: "v", X: "U" }, (x, y) => hash32(x >> 1, y >> 2, 0x4ea) % 4 === 0);
  rust.swap({ K: "w", X: "W" });
  // Half-built: everything above the third girder (row 37) not yet up; a gin pole on the stub.
  const half = (turning[0] as Chars).clone();
  for (let y = 0; y < 37; y++) for (let x = 0; x < half.w; x++) half.set(x, y, ".");
  half.rect(13, 22, 1, 15, "I");
  half.rect(12, 22, 3, 1, "i");
  const foot: [number, number] = [14, base.h - 1];
  const shadow = { box: [-12, -8, 12, 0] as const, height: 56 };
  return [
    { id: "headframe", frames: turning, foot, shadow },
    { id: "headframe_rust", frames: [rust], foot, shadow },
    { id: "headframe_half", frames: [half], foot, shadow: { box: [-12, -8, 12, 0], height: 30 } },
  ];
}

/**
 * A spoil tip grown past round 4's three heaps: wide and flat-topped, as tips
 * are built up by tipping along the top, with the tub track along its crest
 * and its buffer at the end. Lit on the left, gullies down its face.
 */
function spoilTip(): PartDef {
  const W = 84;
  const H = 34;
  const c = new Chars(W, H);
  const baseY = H - 1;
  const crest = 6;
  const topOf = (x: number) => {
    const edge = Math.min(x, W - 1 - x);
    if (edge < 0) return null;
    const slope = Math.max(crest, baseY - Math.round(edge * 0.95));
    return slope + ((hash32(x >> 1, 0x71) % 3 === 0 ? 1 : 0) as number);
  };
  for (let x = 0; x < W; x++) {
    const ty = topOf(x);
    if (ty === null) continue;
    for (let y = ty; y <= baseY; y++) {
      const flat = ty <= crest + 1 && y <= crest + 2;
      let ch = flat ? (y === ty ? "d" : "z") : x < W * 0.4 ? (y - ty < 2 ? "d" : "z") : x < W * 0.7 ? "z" : "Z";
      const g = hash32(x, 0x6a) % 9;
      if (!flat && g === 0 && y > ty + 2) ch = x < W * 0.4 ? "Z" : "O";
      if (hash32(x >> 1, y, 0x2b) % 19 === 0 && y > ty + 1) ch = "O";
      c.set(x, y, ch);
    }
  }
  // The tub track along the crest: sleepers and a rail, a buffer at its end.
  for (let x = 18; x < W - 18; x++) {
    c.set(x, crest, x % 3 === 0 ? "w" : "I");
  }
  c.rect(W - 19, crest - 3, 2, 3, "a");
  c.rect(W - 19, crest - 3, 2, 1, "r");
  // Scrub at the foot.
  for (let x = 2; x < W - 2; x++) if (hash32(x, 0x3c) % 4 === 0 && c.get(x, baseY) !== ".") {
    c.set(x, baseY, "N");
    c.set(x, baseY - 1, "n");
  }
  return { id: "spoil_tip", frames: [c], foot: [W >> 1, baseY], shadow: { box: [-36, -10, 36, 0], height: 22 } };
}

/** A shaft capped when a mine is cleared: a low concrete cap, its vent pipe with a hood. */
function shaftCap(): PartDef {
  const c = Chars.from(`
    ........iI.....
    .......iiII....
    ........IO.....
    ........IO.....
    .qqqqqqqIOqqqq.
    qpppppppppppppP
    qppPpppppPppppP
    PPPPPPPPPPPPPPQ
    .QQQQQQQQQQQQQ.`);
  return { id: "shaft_cap", frames: [c], foot: [7, 8] };
}

/** A works chimney: a tapering brick stack with a stone band and a dark mouth. */
function chimney(): PartDef {
  const H = 60;
  const c = new Chars(12, H);
  for (let y = 0; y < H; y++) {
    const inset = Math.floor((H - 1 - y) / 22);
    for (let x = inset; x < 12 - inset; x++) {
      const edgeL = x === inset;
      const edgeR = x === 11 - inset;
      let ch = edgeL ? "L" : edgeR || x === 10 - inset ? "X" : y % 4 === 3 ? "X" : "K";
      if (y < 2) ch = y === 0 ? "O" : "Q";
      else if (y < 5) ch = y === 2 ? "q" : edgeR ? "P" : "p";
      if (y >= H - 4) ch = y === H - 4 ? "q" : edgeR ? "P" : "p";
      c.set(x, y, ch);
    }
  }
  c.rect(3, 0, 6, 1, "O");
  return { id: "chimney", frames: [c], foot: [6, H - 1], shadow: { box: [-4, -4, 4, 0], height: 60 } };
}

/** Smoke from a stack: round puffs growing as they rise and drift right, lit on their upper left, the oldest thinning out; two frames; no outline (it is not solid). */
function smoke(): PartDef {
  const frames = [0, 1].map((f) => {
    const c = new Chars(34, 40);
    const puffs = [
      [4, 35, 2],
      [6, 29, 3],
      [10, 22, 4],
      [16, 14, 5],
      [25, 6, 5],
    ] as const;
    puffs.forEach(([px, py, r0], k) => {
      const r = r0 + (f && k > 1 ? 1 : 0);
      const x0 = px + (f ? 1 + (k >> 1) : 0);
      const y0 = py - f * 2;
      for (let y = -r; y <= r; y++) {
        for (let x = -r; x <= r; x++) {
          const d = x * x + y * y;
          if (d > r * r + 1) continue;
          // The last puff breaks up: its outer ring in a 2 px checker.
          if (k === puffs.length - 1 && d > (r - 2) * (r - 2) && ((x >> 1) + (y >> 1) + f) % 2 === 0) continue;
          const ch = x + y <= -r / 2 ? "c" : x + y >= r ? "p" : "C";
          c.set(x0 + x, y0 + y, ch);
        }
      }
    });
    return c;
  });
  return { id: "smoke", frames, foot: [4, 37], bare: true };
}

/** A lorry facing right: a council-blue cab, a flatbed of wrapped pallets or a yellow tipper body heaped with ore, three wheels. */
function lorry(tipper: boolean): PartDef {
  const c = new Chars(42, 24);
  // Cab.
  c.rect(30, 4, 11, 13, "b");
  c.rect(30, 4, 11, 1, "l");
  c.rect(40, 5, 1, 12, "B");
  c.rect(32, 6, 7, 5, "g");
  c.rect(32, 6, 3, 2, "h");
  c.rect(30, 12, 11, 1, "B");
  c.rect(41, 13, 1, 4, "I");
  c.set(36, 13, "c");
  // Chassis.
  c.rect(1, 16, 40, 2, "x");
  // Body.
  if (tipper) {
    c.rect(1, 7, 28, 9, "Y");
    c.rect(1, 7, 28, 1, "j");
    c.rect(1, 14, 28, 2, "y");
    for (let x = 4; x < 28; x += 6) c.rect(x, 8, 1, 6, "y");
    for (let x = 2; x < 28; x++) {
      const hgt = Math.round(4 - Math.abs(x - 15) / 5) + (hash32(x, 0x0e) % 2);
      for (let y = 7 - hgt; y < 7; y++) c.set(x, y, y === 7 - hgt ? "d" : "z");
    }
  } else {
    c.rect(0, 14, 30, 2, "w");
    c.rect(0, 14, 30, 1, "k");
    for (const x0 of [1, 15]) {
      c.rect(x0, 12, 13, 2, "w");
      for (const x of [x0 + 1, x0 + 6, x0 + 11]) c.set(x, 13, "W");
      c.rect(x0, 4, 13, 8, "C");
      c.rect(x0, 4, 13, 1, "c");
      c.rect(x0, 4, 1, 8, "c");
      c.rect(x0 + 12, 5, 1, 7, "p");
      c.rect(x0 + 4, 4, 1, 8, "b");
      c.rect(x0, 8, 13, 1, "b");
    }
  }
  // Wheels.
  for (const cx of [6, 14, 34]) {
    for (let y = -2; y <= 2; y++) for (let x = -2; x <= 2; x++) if (x * x + y * y <= 5) c.set(cx + x, 20 + y, Math.abs(x) + Math.abs(y) <= 1 ? "I" : "O");
  }
  c.rect(1, 23, 40, 1, "O");
  c.swap({ l: "b", h: "g", p: "C" });
  return { id: tipper ? "lorry_tipper" : "lorry", frames: [c], foot: [21, 22], shadow: { box: [-20, -6, 20, 0], height: 16 } };
}

/** Stacked goods in a yard: pallets of wrapped boxes, two high. */
function goods(): PartDef {
  const c = new Chars(16, 18);
  for (const [x0, y0] of [
    [0, 9],
    [0, 0],
  ] as const) {
    c.rect(x0, y0 + 7, 16, 2, "w");
    for (const x of [x0 + 2, x0 + 8, x0 + 13]) c.set(x, y0 + 8, "O");
    c.rect(x0 + 1, y0, 14, 7, "k");
    c.rect(x0 + 1, y0, 14, 1, "F");
    c.rect(x0 + 8, y0, 1, 7, "w");
    c.rect(x0 + 14, y0 + 1, 1, 6, "w");
    c.rect(x0 + 1, y0 + 3, 14, 1, "b");
  }
  return { id: "goods", frames: [c], foot: [8, 17] };
}

/**
 * Yard gates (two cells wide) on their brick piers: open (the leaves swung
 * back against the piers, seen end on) or shut with a chain and padlock;
 * rusted (a shut gate left for years).
 */
function gates(): PartDef[] {
  const make = (shut: boolean, rust: boolean): PartDef => {
    const W = 36;
    const c = new Chars(W, 20);
    for (const x0 of [0, W - 4]) {
      c.rect(x0, 2, 4, 18, "K");
      c.rect(x0, 2, 1, 18, "L");
      c.rect(x0 + 3, 2, 1, 18, "X");
      c.rect(x0, 0, 4, 2, "q");
      c.rect(x0, 1, 4, 1, "P");
      for (let y = 5; y < 20; y += 4) c.rect(x0 + 1, y, 2, 1, "X");
    }
    if (shut) {
      for (let x = 4; x < W - 4; x++) {
        for (let y = 5; y < 19; y++) {
          const rail = y === 5 || y === 6 || y === 17 || y === 18 || y === 11;
          const bar = x % 3 === 0;
          if (rail || bar) c.set(x, y, rail && y % 6 === 5 ? "I" : "i");
        }
        if (x % 3 === 0) c.set(x, 4, "I");
      }
      c.rect(W / 2 - 1, 5, 2, 14, "i");
      // The chain looped round the meeting stiles, the padlock.
      c.line(W / 2 - 4, 10, W / 2 + 3, 12, "I");
      c.line(W / 2 - 4, 12, W / 2 + 3, 10, "I");
      c.rect(W / 2 - 1, 13, 3, 3, "y");
      c.set(W / 2, 13, "Y");
      if (rust) {
        c.swap({ i: "u", I: "v" }, (x, y) => hash32(x >> 1, y >> 1, 0x9a7) % 2 === 0);
        c.swap({ i: "U" }, (x, y) => y > 14 && hash32(x, y, 0x1f) % 2 === 0);
        c.swap({ y: "u", Y: "v" });
      }
    } else {
      // The leaves folded back against the piers, seen end on as strips of bars.
      for (const x0 of [4, W - 8]) {
        for (let y = 5; y < 19; y++) {
          c.set(x0, y, "I");
          c.set(x0 + 1, y, y % 3 === 0 ? "I" : "i");
          c.set(x0 + 2, y, "i");
          c.set(x0 + 3, y, y % 3 === 0 ? "i" : ".");
        }
      }
    }
    return { id: shut ? (rust ? "gates_shut_rust" : "gates_shut") : "gates_open", frames: [c], foot: [W >> 1, 19] };
  };
  return [make(false, false), make(true, false), make(true, true)];
}

// ---------------------------------------------------------------------------
// Finished works: the plaque and the ribbon

/**
 * The sponsor's plaque on a stone plinth (about waist high): a lit coping, a
 * bronze plate with a gold rim and lines of lettering as light dashes (no
 * letters, R14), a plinth step. It reads as a plaque at street zoom by its
 * dark plate on light stone. The name is a label the UI draws (DEC-054).
 */
function plaques(): PartDef[] {
  const plinth = Chars.from(`
    .tttttttttt.
    .tttttttttt.
    .sssssssssS.
    .sjjjjjjjjS.
    .sjWWWWWWjS.
    .sjWkkkkWjS.
    .sjWWWWWWjS.
    .sjWkkkWWjS.
    .sjjjjjjjjS.
    .sssssssssS.
    .sssssssssS.
    SSSSSSSSSSSS
    TTTTTTTTTTTT`);
  const wall = Chars.from(`
    jjjjjjjj
    jWWWWWWj
    jWkkkkWj
    jWWWWWWj
    jWkkWWWj
    yyyyyyyy`);
  return [
    { id: "plaque", frames: [plinth], foot: [6, 12] },
    { id: "plaque_wall", frames: [wall], foot: [4, 5] },
  ];
}

/**
 * The opening ceremony: a red ribbon on two chrome stands, with a bow in the
 * middle. Across a way that runs north–south it stretches left to right
 * (`ribbon`); across one that runs east–west the stands stand one behind the
 * other and the ribbon runs up the screen (`ribbon_ns`).
 */
function ribbons(): PartDef[] {
  const W = 34;
  const a = new Chars(W, 16);
  const stand = (c: Chars, x: number, y: number) => {
    c.rect(x, y, 2, 1, "c");
    c.rect(x, y + 1, 1, 11, "I");
    c.rect(x + 1, y + 1, 1, 11, "i");
    c.rect(x - 1, y + 12, 4, 1, "i");
    c.rect(x - 1, y + 12, 1, 1, "I");
  };
  stand(a, 0, 3);
  stand(a, W - 2, 3);
  for (let x = 2; x < W - 2; x++) {
    const sag = Math.round(2 - Math.abs(x - W / 2) / 8);
    a.set(x, 4 + sag, "R");
    a.set(x, 5 + sag, "H");
  }
  // The bow and its tails.
  const bx = W >> 1;
  a.rect(bx - 4, 4, 3, 3, "R");
  a.rect(bx + 2, 4, 3, 3, "R");
  a.set(bx - 3, 5, "H");
  a.set(bx + 3, 5, "H");
  a.rect(bx - 1, 5, 3, 3, "H");
  a.line(bx - 1, 8, bx - 3, 12, "R");
  a.line(bx + 1, 8, bx + 3, 12, "R");

  // Across an east–west way the stands stand one behind the other (feet 11 px apart), the near one a little to the right, so the ribbon reads as it slants down between them.
  const b = new Chars(14, 24);
  stand(b, 1, 1);
  stand(b, 9, 11);
  b.line(2, 2, 10, 12, "R");
  b.line(3, 2, 11, 12, "H");
  b.rect(4, 5, 3, 3, "R");
  b.rect(7, 7, 3, 3, "R");
  b.rect(6, 7, 2, 2, "H");
  b.line(6, 9, 4, 13, "R");
  b.line(8, 10, 9, 14, "R");
  return [
    { id: "ribbon", frames: [a], foot: [bx, 15] },
    { id: "ribbon_ns", frames: [b], foot: [10, 23] },
  ];
}

/** Every drawn part of the works kit. */
export function worksPartDefs(): PartDef[] {
  return [
    cabin(false),
    cabin(true),
    towerCrane(true, false),
    towerCrane(false, false),
    towerCrane(false, true),
    excavator(false),
    excavator(true),
    skip(),
    ...piles(),
    hazardSign(),
    roadSign(false),
    roadSign(true),
    ...cones(),
    board(0),
    board(1),
    board(2),
    sheet(),
    buddleia(),
    ...headframes(),
    spoilTip(),
    shaftCap(),
    chimney(),
    smoke(),
    lorry(false),
    lorry(true),
    goods(),
    ...gates(),
    ...plaques(),
    ...ribbons(),
  ];
}

/** Lit parts glow in the lamp's colours; the crane's warning light stays red. */
export function worksLitLegend(p: Palette): Record<string, number> {
  const lamp = legendFor(p, { a: ["lamp", 1], b: ["lamp", 2], c: ["lamp", 0], r: ["flag", 1] });
  return { g: lamp.a as number, h: lamp.b as number, G: lamp.c as number, r: lamp.r as number, default: lamp.a as number };
}

/** Build every part for a palette. */
export function buildWorksParts(p: Palette, lightness: readonly number[]): Record<string, WorksSprite> {
  const legend = worksLegend(p);
  const lit = worksLitLegend(p);
  const out: Record<string, WorksSprite> = {};
  for (const d of worksPartDefs()) {
    if (out[d.id]) throw new Error(`works: duplicate part ${d.id}`);
    out[d.id] = buildPart(d, p, lightness, legend, lit);
  }
  return out;
}
