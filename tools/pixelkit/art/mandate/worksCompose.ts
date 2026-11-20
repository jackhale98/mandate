// Works on a site (art round 8, DEC-099): `composeWorks(look, site, kit)`
// places a look's parts (./works) on a concrete site in map px and draws
// what depends on the site's shape: a bridge's deck states over its
// connector, a wall or levee along its run, a building's frame and
// scaffolding over its slot, a road's resurfacing over its lane, fences
// round the site, and the decay marks on what they mark.
//
// It is pure: the same look, site and kit give the same pixels. Positions
// come from the site and fixed slots (the cabin always at yard slot 0, a
// mark always at the place its number and the seed give), so a part stays
// where it is from one stage to the next, and a mark, once made, stays put.
//
// What comes out is what a renderer draws, in order:
//   - `tints`: ground pixels remapped (fresh or planed asphalt), under everything;
//   - `decals`: images painted on the ground (deck states, foundations, puddles, weeds);
//   - `sprites`: standing things with a depth row (`base`), lit layers, idle frames;
//   - `people`: workers' feet and looks (composed on the device, D10);
//   - `replaces`: the slot whose plan building the works stands in for.
import { hash32 } from "../../src/core/hash";
import { blank, type Indexed } from "../../src/core/image";
import type { Palette } from "../../src/core/palette";
import { buildFacade, RECEIVER, type FacadeSpec } from "./buildings";
import type { Look } from "./characters";
import type { BuiltProp } from "./props";
import type { ConnectorView } from "./terrain";
import { wearLevel, workerLook, type WorksLook, type WorksPart } from "./works";
import { drawRun, rectPieces, runCentres, straightCentres, type CentrePx, type LinePiece, type LineSprite, type LineStyle } from "./worksLines";
import { buildWorksParts, Chars, charsToIndexed, worksLegend, type WorksSprite } from "./worksParts";

/** A rectangle of cells. */
export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Where a works project stands (from the plan: a connector, a run of bank, a slot, a road strip, the mine). */
export interface WorksSite {
  /** Cells where the site's furniture stands (cabin, machines, piles). */
  yard: Rect;
  /** The cell the board stands on (front 0); a sign stands one cell east (front 1). */
  front: readonly [number, number];
  /** The cell where the ribbon and the plaque go. */
  entrance: readonly [number, number];
  /** bridge: the connector whose deck is replaced. */
  connector?: ConnectorView;
  /** flood_wall, levee: the run along the bank, in order from its start. */
  pieces?: readonly LinePiece[];
  /** The perimeter fence's run, where the site's shape does not give one (walls, mines). */
  fence?: readonly LinePiece[];
  /** civic, housing, factory: the slot (cells) and its door column. */
  slot?: Rect & { doorX: number };
  /** The slot's building as finished (kind, materials, storeys). */
  building?: FacadeSpec;
  /** factory: the chimney's cell; the yard gates' left cell. */
  chimney?: readonly [number, number];
  gate?: readonly [number, number];
  /**
   * road: the strip's carriageway cells; the closed lane as a half-plane of
   * map px (a pixel is in it when nx·X + ny·Y ≥ d: a row of lanes, or one
   * side of a 45° street's centre line); the barrier line; the signs' feet
   * (map px); the axis the strip runs along.
   */
  cells?: readonly (readonly [number, number])[];
  lane?: { nx: number; ny: number; d: number };
  closure?: readonly CentrePx[];
  signs?: readonly (readonly [number, number])[];
  along?: "x" | "y";
  /** mine: the headframe's cell and the spoil heap's. */
  headframe?: readonly [number, number];
  heap?: readonly [number, number];
}

export interface WorksSpriteOut {
  id: string;
  /** Map px of the image's top-left. */
  x: number;
  y: number;
  /** Frames (one for still parts); a renderer cycles them as an idle loop. */
  frames: Indexed[];
  /** The lit layer per frame (shown at night when the part is lit). */
  emissive: (Indexed | null)[];
  /** Depth: the map row the part stands on. */
  base: number;
  shadow?: { box: readonly [number, number, number, number]; height: number; ox: number; oy: number };
}

export interface WorksDecal {
  x: number;
  y: number;
  img: Indexed;
  /** Paint only over these ground colours (palette indices); all when absent. */
  onlyOn?: ReadonlySet<number>;
}

export interface WorksTint {
  x: number;
  y: number;
  /** Which pixels of the rectangle are remapped (1) or not (0), w × h. */
  w: number;
  h: number;
  mask: Uint8Array;
  /** Palette index → palette index for those pixels (identity where unchanged). */
  table: Uint8Array;
}

export interface ComposedWorks {
  tints: WorksTint[];
  decals: WorksDecal[];
  sprites: WorksSpriteOut[];
  people: { x: number; y: number; look: Look }[];
  replaces: Rect | null;
}

/** What composing needs from the pack: the palette, lightness, the round 4 props (spoil heaps), the kit's parts. */
export interface WorksKit {
  palette: Palette;
  lightness: readonly number[];
  props: Readonly<Record<string, readonly BuiltProp[]>>;
  parts: Readonly<Record<string, WorksSprite>>;
}

export function worksKit(pack: { palette: Palette; lightness: readonly number[]; props: Readonly<Record<string, readonly BuiltProp[]>> }): WorksKit {
  return { palette: pack.palette, lightness: pack.lightness, props: pack.props, parts: buildWorksParts(pack.palette, pack.lightness) };
}

// ---------------------------------------------------------------------------

const GROUND = 24;
const UPPER = 18;
/** Front wall height of a building of so many storeys (the facade grammar's storey heights). */
const wallHeight = (storeys: number) => GROUND + UPPER * Math.max(0, storeys - 1);

/** Yard slots as fractions of the yard (x, y of the foot): 0 cabin, 1 machine, 2–5 piles and skip, 6 lorry, 7–9 goods. */
const YARD_SLOTS: readonly (readonly [number, number])[] = [
  [0.3, 0.42],
  [0.72, 0.9],
  [0.62, 0.38],
  [0.88, 0.5],
  [0.12, 0.92],
  [0.4, 0.95],
  [0.55, 0.72],
  [0.12, 0.62],
  [0.24, 0.66],
  [0.36, 0.7],
];

interface Tagged {
  out: WorksSpriteOut;
  /** What marks can land on it. */
  tags: ReadonlySet<string>;
}

class Composer {
  readonly L: Record<string, number>;
  readonly res: ComposedWorks = { tints: [], decals: [], sprites: [], people: [], replaces: null };
  readonly tagged: Tagged[] = [];
  /** Ground areas for ground marks (map px rects). */
  readonly grounds: [number, number, number, number][] = [];
  /** Where workers stand (map px feet). */
  readonly work: [number, number][] = [];
  /** Where the structure's top is (map px), for buddleia and the sheet. */
  readonly tops: { x0: number; x1: number; y: number; base: number }[] = [];
  /** Deck pixels a rust tint can mark (bridge beams), and the face rows of an east–west bridge (graffiti). */
  beams: { X: number; Y: number }[] = [];
  faceArea: [number, number, number, number] | null = null;

  readonly look: WorksLook;
  readonly site: WorksSite;
  readonly kit: WorksKit;
  readonly seed: number;

  constructor(look: WorksLook, site: WorksSite, kit: WorksKit, seed: number) {
    this.look = look;
    this.site = site;
    this.kit = kit;
    this.seed = seed;
    this.L = worksLegend(kit.palette);
  }

  idx(ch: string): number {
    return this.L[ch] as number;
  }

  sprite(id: string, part: WorksSprite, fx: number, fy: number, tags: readonly string[], lit: boolean, frames = 1, base = fy): Tagged {
    const out: WorksSpriteOut = {
      id,
      x: fx - part.fx,
      y: fy - part.fy,
      frames: part.frames.slice(0, Math.max(1, frames)).map((f) => ({ w: f.w, h: f.h, px: f.px.slice() })),
      emissive: lit ? part.emissive.slice(0, Math.max(1, frames)) : part.frames.slice(0, Math.max(1, frames)).map(() => null),
      base,
    };
    if (part.shadow) out.shadow = { ...part.shadow, ox: fx, oy: fy };
    const t = { out, tags: new Set(tags) };
    this.tagged.push(t);
    return t;
  }

  partAt(id: string, fx: number, fy: number, tags: readonly string[] = [], lit = false, frames = 1, base = fy): Tagged | null {
    const part = this.kit.parts[id];
    if (!part) return null;
    return this.sprite(id, part, fx, fy, tags, lit, frames, base);
  }

  image(id: string, img: Indexed, x: number, y: number, base: number, tags: readonly string[], emissive: Indexed | null = null): Tagged {
    const t = { out: { id, x, y, frames: [img], emissive: [emissive], base }, tags: new Set(tags) };
    this.tagged.push(t);
    return t;
  }

  run(id: string, style: LineStyle, centres: readonly CentrePx[], part: WorksPart, tags: readonly string[]) {
    const wear = (piece: number) => (part.wear ? wearLevel(part.wear, this.seed, part.role, piece, part.decay ?? 0) : 0);
    const sprites: LineSprite[] = drawRun(this.kit.palette, this.kit.lightness, style, centres, this.seed, wear);
    for (const s of sprites) this.image(`${id}@${s.cx},${s.cy}`, s.img, s.x, s.y, s.base, tags);
  }

  yardFoot(n: number): [number, number] {
    const y = this.site.yard;
    const [fx, fy] = YARD_SLOTS[n % YARD_SLOTS.length] as readonly [number, number];
    return [Math.round(y.x * 16 + fx * y.w * 16), Math.round(y.y * 16 + fy * y.h * 16)];
  }

  decal(x: number, y: number, c: Chars, onlyOn?: readonly string[]) {
    const img = charsToIndexed(c, this.L, 0);
    const d: WorksDecal = { x, y, img };
    if (onlyOn) d.onlyOn = new Set(onlyOn.map((ch) => this.idx(ch)));
    this.res.decals.push(d);
  }
}

// ---------------------------------------------------------------------------
// Structures

/** A bridge's deck: its carriageway in the state of the works (see BRIDGE in the README). */
function bridge(cx: Composer, part: WorksPart) {
  const c = cx.site.connector;
  if (!c) return;
  const ew = c.axis !== "ns";
  const X0 = c.x * 16;
  const Y0 = c.y * 16;
  const run = (ew ? c.w : c.h) * 16;
  const span = (ew ? c.h : c.w) * 16;
  const parapet = 3;
  const lvl = part.level ?? 0;
  const deck = new Chars(ew ? run : span, ew ? span : run);
  const put = (along: number, across: number, ch: string) => deck.set(ew ? along : across, ew ? across : along, ch);
  const gapA = Math.round(run * 0.45);
  const gapB = Math.round(run * 0.8);
  const mid = Math.floor(span / 2);
  for (let along = 0; along < run; along++) {
    for (let a = parapet; a < span - parapet; a++) {
      const edge = a === parapet || a === span - parapet - 1;
      const fresh = () => (edge ? "c" : hash32(along >> 1, a >> 1, 0x5f) % 11 === 0 ? "z" : "Z");
      const stripped = () => ((along % 4 === 0 || (a - parapet) % 4 === 0) && !edge ? "Q" : hash32(along >> 1, a, 0x8a) % 7 === 0 ? "q" : "p");
      let ch: string | null = null;
      if (lvl === 1 && a >= mid) ch = stripped();
      else if (lvl === 2 || lvl === 5) {
        if (along < gapA) ch = fresh();
        else if (along < gapB) {
          if (lvl === 5) {
            // Steel road plates over the gap: plate joints every 12 px, a lit edge.
            const j = (along - gapA) % 12;
            ch = j === 0 ? "x" : j === 1 ? "I" : (a - parapet) % 10 === 0 ? "x" : "i";
          } else {
            // The deck taken out: steel beams along the way every 4 px, the river far below between them.
            // A dark shadow under the cut ends and each beam; the water bright below.
            const r = (a - parapet) % 5;
            const cut = along - gapA < 2 || gapB - along <= 2;
            ch = cut ? (along - gapA < 1 || gapB - along <= 1 ? "q" : "O") : r === 0 ? "I" : r === 1 ? "i" : r === 2 ? "O" : r === 3 ? "_" : "~";
            if (r <= 1) cx.beams.push(ew ? { X: X0 + along, Y: Y0 + a } : { X: X0 + a, Y: Y0 + along });
          }
        } else if (lvl === 5 && along < gapB + 6) ch = stripped();
      } else if (lvl === 3) ch = along < Math.round(run * 0.85) ? fresh() : stripped();
      else if (lvl === 4) ch = !edge && a === mid && along % 12 < 6 && along > 4 && along < run - 4 ? "c" : fresh();
      if (ch) put(along, a, ch);
    }
  }
  cx.decal(X0, Y0, deck);
  if (lvl === 2) {
    // Fence round the gap: Heras across the carriageway at both ends of it.
    const fence = cx.look.parts.find((p) => p.role === "closure" && p.id === "heras");
    if (fence) {
      for (const at of [gapA - 2, gapB + 1]) {
        const cs: CentrePx[] = [];
        for (let a = parapet; a < span - parapet; a++) cs.push(ew ? { X: X0 + at, Y: Y0 + a, dir: 1, piece: at < gapB ? 0 : 1 } : { X: X0 + a, Y: Y0 + at, dir: 0, piece: at < gapB ? 0 : 1 });
        cx.run("gapfence", "heras", cs, fence, ["metal"]);
      }
    }
  }
  // Ground marks go along the deck's edges and on its stripped or plated parts.
  cx.grounds.push(ew ? [X0 + 2, Y0 + parapet, run - 4, span - 2 * parapet] : [X0 + parapet, Y0 + 2, span - 2 * parapet, run - 4]);
  if (ew) cx.faceArea = [X0 + 2, (c.y + c.h) * 16 + 4, run - 4, 6];
  // Workers on the deck, clear of the gap (on the new deck before it, and the old one after it).
  for (const t of [0.12, 0.3, 0.9]) cx.work.push(ew ? [X0 + Math.round(run * t), Y0 + span - 5] : [X0 + span - 4, Y0 + Math.round(run * t)]);
}

/** New railings along both parapets (a share of the length while they are being fitted). */
function bridgeRailings(cx: Composer, part: WorksPart) {
  const c = cx.site.connector;
  if (!c) return;
  const ew = c.axis !== "ns";
  const run = (ew ? c.w : c.h) * 16;
  const span = (ew ? c.h : c.w) * 16;
  const len = Math.round((run * (part.level ?? 100)) / 100);
  for (const a of [1, span - 2]) {
    const cs = ew ? straightCentres(c.x * 16, c.y * 16 + a, len, 0) : straightCentres(c.x * 16 + a, c.y * 16, len, 1);
    cx.run(`railing${a}`, "railing", cs, part, ["metal"]);
  }
}

/** Scaffolding: on a bridge's south face (east–west) or along both sides (north–south); over a building's front. */
function scaffold(cx: Composer, part: WorksPart) {
  const lvl = (part.level ?? 100) / 100;
  const c = cx.site.connector;
  if (c) {
    const ew = c.axis !== "ns";
    if (ew) {
      const W = Math.max(8, Math.round(c.w * 16 * lvl));
      const Yd = (c.y + c.h) * 16;
      const s = scaffoldChars(W, 18, false, part.id === "scaffold_bare");
      cx.image("scaffold", charsToIndexed(s, cx.L, 0), c.x * 16, Yd - 2, Yd + 16, ["metal", "scaffold"]);
    } else {
      const H = Math.max(8, Math.round(c.h * 16 * lvl));
      for (const side of [-1, 1]) {
        const s = new Chars(4, H + 6);
        for (let y = 0; y < H + 6; y++) {
          s.set(0, y, y % 6 === 0 ? "I" : ".");
          s.set(side < 0 ? 1 : 2, y, "k");
          s.set(side < 0 ? 2 : 1, y, y % 3 === 0 ? "W" : "w");
          s.set(3, y, y % 6 === 0 ? "I" : ".");
        }
        const x = side < 0 ? c.x * 16 - 4 : (c.x + c.w) * 16;
        cx.image(`scaffold${side}`, charsToIndexed(s, cx.L, 0), x, c.y * 16 - 3, (c.y + c.h) * 16, ["metal", "scaffold"]);
      }
    }
    return;
  }
  const slot = cx.site.slot;
  const b = cx.site.building;
  if (!slot || !b) return;
  const wallH = wallHeight(b.storeys);
  const W = Math.max(12, Math.round(slot.w * 16 * lvl));
  const H = wallH + 8;
  const s = scaffoldChars(W, H, true, part.id === "scaffold_bare");
  const base = (slot.y + slot.h) * 16;
  const t = cx.image("scaffold", charsToIndexed(s, cx.L, 0), slot.x * 16, base - H, base + 1, ["metal", "scaffold"]);
  cx.tops.push({ x0: t.out.x + 2, x1: t.out.x + W - 3, y: t.out.y + 1, base: base + 1 });
}

/** Tube-and-fitting scaffolding W × H: standards every 12 px, a ledger and boards every lift, a brace, a ladder, netting on the top lift (buildings). */
function scaffoldChars(W: number, H: number, net: boolean, bare = false): Chars {
  const s = new Chars(W, H);
  for (let x = 1; x < W - 1; x += 12) s.rect(x, 0, 1, H, bare ? "u" : "I");
  s.rect(W - 2, 0, 1, H, "I");
  for (let y = H - 3; y >= 2; y -= 9) {
    s.rect(0, y, W, 1, "i");
    // Left for years: most boards gone (taken or fallen), the rest warped dark.
    for (let x = 0; x < W; x += 6) {
      if (bare && hash32(x, y, 0xb0a) % 3 !== 0) continue;
      s.rect(x, y - 2, Math.min(6, W - x), 2, bare ? "W" : "w");
      if (!bare) s.rect(x, y - 2, Math.min(6, W - x), 1, "k");
    }
  }
  for (let y = 0; y < H - 3; y++) s.set(2 + Math.floor((y * Math.min(10, W - 4)) / Math.max(1, H - 3)), y, "I");
  if (W > 30) {
    const lx = 26;
    for (let y = 2; y < H - 1; y++) {
      s.set(lx, y, "W");
      s.set(lx + 3, y, "W");
      if (y % 3 === 0) s.rect(lx, y, 4, 1, "k");
    }
  }
  // Netting on the top lift; torn to rags when the site is left.
  if (net) for (let y = 0; y < Math.min(7, H); y++) for (let x = 0; x < W; x++) {
    if (x % 12 === 1 || (hash32(x >> 2, 0x4e7) + y) % 9 === 0) continue;
    if (bare && (hash32(x >> 2, 0x4e8) % 3 !== 0 || y > 2 + (hash32(x, 0x4e9) % 6))) continue;
    s.set(x, y, (x + (y >> 1)) % 5 === 0 ? "e" : "E");
  }
  return s;
}

/** A building's frame: concrete columns and slab edges, dark bays with the back columns showing, the top slab with its starter bars. */
function frame(cx: Composer, worn: number) {
  const slot = cx.site.slot;
  const b = cx.site.building;
  if (!slot || !b) return;
  const W = slot.w * 16;
  const HR = Math.max(0, Math.min(32, b.headroom ?? 0));
  const IH = slot.h * 16 + 16 + HR;
  const wallH = wallHeight(b.storeys);
  const wallTop = IH - wallH;
  const roofTop = Math.max(2, wallTop - (slot.h * 16 - 6));
  const c = new Chars(W, IH);
  // The top slab, seen from above, and its front edge.
  c.rect(1, roofTop, W - 2, wallTop - roofTop, "p");
  for (let x = 1; x < W - 1; x += 16) c.rect(x, roofTop, 1, wallTop - roofTop, "P");
  c.rect(1, roofTop, W - 2, 1, "q");
  // The stair core standing a storey proud at the back, its shuttering marks; columns' starter bars along the back edge.
  const coreW = Math.min(14, W - 8);
  const coreX = W - coreW - 4;
  const coreTop = Math.max(0, roofTop - 10);
  c.rect(coreX, coreTop, coreW, roofTop + 6 - coreTop, "P");
  c.rect(coreX, coreTop, coreW, 3, "q");
  c.rect(coreX, roofTop + 3, coreW, 3, "Q");
  for (let x = coreX + 3; x < coreX + coreW; x += 4) c.rect(x, coreTop + 4, 1, roofTop - coreTop - 2, "Q");
  for (let x = 2; x < W - 2; x += 16) c.rect(x, roofTop - 2, 1, 2, "u");
  // Starter bars on the top slab at the column lines, rusting.
  for (let x = 2; x < W - 2; x += 16) for (const dx of [0, 2]) c.rect(x + dx, wallTop - 4, 1, 3, "u");
  // Bays: dark inside, the back columns, the shadow under each slab.
  const floors: number[] = [IH - 1];
  for (let k = 0; k < b.storeys - 1; k++) floors.push(IH - GROUND - k * UPPER);
  floors.push(wallTop);
  c.rect(1, wallTop, W - 2, wallH, "x");
  for (let x = 9; x < W - 3; x += 16) c.rect(x, wallTop, 2, wallH, "o");
  for (const fy of floors) {
    c.rect(1, fy, W - 2, 3, "p");
    c.rect(1, fy, W - 2, 1, "q");
    c.rect(1, fy + 2, W - 2, 1, "P");
    if (fy + 3 < IH) c.rect(1, fy + 3, W - 2, 2, "O");
  }
  // Columns in front: lit left, shaded right.
  for (let x = 1; x < W - 3; x += 16) {
    c.rect(x, wallTop, 3, wallH, "p");
    c.rect(x, wallTop, 1, wallH, "q");
    c.rect(x + 2, wallTop, 1, wallH, "P");
  }
  c.rect(W - 4, wallTop, 3, wallH, "p");
  c.rect(W - 4, wallTop, 1, wallH, "q");
  c.rect(W - 2, wallTop, 1, wallH, "P");
  c.rect(1, IH - 1, W - 2, 1, "Q");
  if (worn >= 1) {
    // Weathered: rust run down from the starter bars, dark streaks of rain under every slab edge and down the columns.
    for (let x = 2; x < W - 2; x += 16) for (let y = wallTop - 1; y < wallTop + 3; y++) c.set(x + 1, y, "U");
    for (const fy of floors) for (let x = 1; x < W - 1; x++) {
      if (hash32(cx.seed, x, fy, 0xd1) % 4 !== 0 || c.get(x, fy + 3) === "O") continue;
      const len = 2 + (hash32(cx.seed, x, fy, 0xd2) % 4);
      for (let k = 3; k < 3 + len; k++) if (c.get(x, fy + k) === "p" || c.get(x, fy + k) === "q") c.set(x, fy + k, "P");
    }
  }
  if (worn >= 2) {
    // Overgrown: ivy up the end columns from the ground, moss along the slab edges, a corner of a slab broken away.
    for (const x0 of [1, W - 4]) for (let y = IH - 1; y > IH - 1 - Math.min(wallH, 30); y--) {
      const reach = 4 - Math.floor((IH - 1 - y) / 9);
      for (let x = x0 - 1; x < x0 - 1 + reach + 2; x++) if (hash32(cx.seed, x, y, 0x1f) % 3 !== 0 && c.get(x, y) !== ".") c.set(x, y, (x + y) % 3 ? "N" : "n");
    }
    for (const fy of floors.slice(1)) for (let x = 1; x < W - 1; x += 3) if (hash32(cx.seed, x, fy, 0x3d) % 2 === 0) c.set(x, fy, "N");
    const bx = 8 + (hash32(cx.seed, 0xb7) % Math.max(1, W - 20));
    c.rect(bx, wallTop, 6, 2, "x");
    c.rect(bx + 1, wallTop + 2, 4, 1, "x");
  }
  const img = charsToIndexed(c, cx.L, 0);
  const x = slot.x * 16;
  const y = slot.y * 16 - 16 - HR;
  const base = (slot.y + slot.h) * 16;
  cx.image("frame", img, x, y, base, ["concrete", "frame"]);
  cx.tops.push({ x0: x + 3, x1: x + W - 4, y: y + wallTop - 4, base });
}

/** The building in a works state: built (lit), unlit, reserved, construction, boarded, derelict. */
function facade(cx: Composer, part: WorksPart) {
  const slot = cx.site.slot;
  const b = cx.site.building;
  if (!slot || !b) return;
  const state = part.id === "facade" || part.id === "facade.unlit" ? null : part.id.slice("facade.".length);
  const spec: FacadeSpec = { ...b, conditions: state ? [state] : [] };
  const f = buildFacade(spec, cx.kit.palette);
  const x = slot.x * 16;
  const y = slot.y * 16 - f.ay;
  const base = (slot.y + slot.h) * 16;
  // Lit windows: all of them when open, a share of them by the works' activity, none while building or shut.
  let em: Indexed | null = null;
  if (part.id === "facade") {
    const share = part.level ?? 100;
    em = blank(f.albedo.w, f.albedo.h);
    [...f.windows, ...f.doors].forEach((o, i) => {
      if (!o.emissive || hash32(cx.seed, "lit", i) % 100 >= share) return;
      for (let yy = 0; yy < o.emissive.h; yy++) for (let xx = 0; xx < o.emissive.w; xx++) {
        const v = o.emissive.px[yy * o.emissive.w + xx] as number;
        if (v) em!.px[(o.y + yy) * em!.w + o.x + xx] = v;
      }
    });
  }
  const t = cx.image(part.id, { w: f.albedo.w, h: f.albedo.h, px: f.albedo.px.slice() }, x, y, base, ["facade"], em);
  // Faces low enough to tag: the receivers' wall faces 2–14 px above their base.
  (t as Tagged & { faces?: Uint8Array }).faces = f.receivers;
  const roof = f.albedo.px.findIndex((v) => v !== 0);
  cx.tops.push({ x0: x + 4, x1: x + f.albedo.w - 5, y: y + Math.max(0, Math.floor(roof / f.albedo.w)) + 6, base });
}

/** A run of wall or levee pieces: the built share in the finished style, then the rest in the style it is going up in. */
function lineStructure(cx: Composer, part: WorksPart) {
  const pieces = cx.site.pieces ?? [];
  if (!pieces.length) return;
  const level = part.level ?? 100;
  const n = Math.round((pieces.length * level) / 100);
  // The other line part on this site (piles beyond a wall; the bare bank beyond the grassed levee) starts where this one stops.
  const style = part.id as LineStyle;
  let from = 0;
  if (part.id === "piles" || part.id === "levee_bare") {
    const done = cx.look.parts.find((p) => p.role === "structure" && (p.id === "wall" || p.id === "levee"));
    from = done ? Math.round((pieces.length * (done.level ?? 0)) / 100) : 0;
  }
  const use = pieces.slice(from, Math.max(from, n));
  if (!use.length) return;
  const tags = style === "wall" ? ["concrete", "wall"] : style === "piles" ? ["metal"] : style === "levee_bare" ? ["bare"] : ["levee"];
  cx.run(part.id, style, runCentres(use), part, tags);
  for (const p of use) {
    cx.grounds.push([p.x * 16 + 1, p.y * 16 + 10, 14, 5]);
    cx.work.push([p.x * 16 + 8, p.y * 16 + 14]);
  }
}

/** A road strip: planed, resurfaced (unmarked, then marked) over the closed lane or the whole strip, or patched after a stall. */
function surface(cx: Composer, part: WorksPart) {
  const cells = cx.site.cells ?? [];
  const lvl = part.level ?? 0;
  if (!cells.length) return;
  const lane = cx.site.lane;
  const inLane = (X: number, Y: number) => !lane || lane.nx * X + lane.ny * Y >= lane.d;
  const along = cx.site.along ?? "x";
  const pos = (X: number, Y: number) => (along === "x" ? X : Y);
  const lo = Math.min(...cells.map((c) => pos(c[0], c[1]))) * 16;
  const hi = (Math.max(...cells.map((c) => pos(c[0], c[1]))) + 1) * 16;
  const half = lo + Math.round((hi - lo) / 2);
  for (const c of cells) {
    for (let k = 0; k < 3; k++) {
      const X = c[0] * 16 + 3 + (hash32(cx.seed, c[0], c[1], k, 0x77) % 10);
      const Y = c[1] * 16 + 3 + (hash32(cx.seed, c[0], c[1], k, 0x78) % 10);
      if (inLane(X, Y)) cx.work.push([X, Y]);
    }
  }
  if (lvl === 0) return;
  const pal = cx.kit.palette;
  const asphalt = pal.ramps.find((r) => r.name === "asphalt")?.steps ?? [];
  const chalk = pal.ramps.find((r) => r.name === "chalk")?.steps ?? [];
  const n = pal.colors.length;
  const table = (to: "fresh" | "fresh_unmarked" | "planed") => {
    const t = new Uint8Array(256);
    for (let i = 0; i < 256; i++) t[i] = i < n ? i : 0;
    for (const s of asphalt) t[s] = to === "planed" ? cx.idx("d") : s === asphalt[asphalt.length - 1] ? cx.idx("z") : cx.idx("Z");
    for (const s of chalk) if (to !== "fresh") t[s] = to === "planed" ? cx.idx("d") : cx.idx("Z");
    return t;
  };
  const tint = (to: "fresh" | "fresh_unmarked" | "planed", which: (X: number, Y: number) => boolean) => {
    const tab = table(to);
    for (const c of cells) {
      const mask = new Uint8Array(256);
      let any = false;
      for (let y = 0; y < 16; y++) for (let x = 0; x < 16; x++) {
        const X = c[0] * 16 + x;
        const Y = c[1] * 16 + y;
        if (which(X, Y)) {
          mask[y * 16 + x] = 1;
          any = true;
        }
      }
      if (!any) continue;
      cx.res.tints.push({ x: c[0] * 16, y: c[1] * 16, w: 16, h: 16, mask, table: tab });
      if (to === "planed") {
        // Grooves along the strip left by the planer, on the planed asphalt.
        const g = new Chars(16, 16);
        for (let y = 0; y < 16; y++) for (let x = 0; x < 16; x++) {
          const X = c[0] * 16 + x;
          const Y = c[1] * 16 + y;
          if (mask[y * 16 + x] && (along === "x" ? Y : X) % 3 === 0) g.set(x, y, "z");
        }
        cx.decal(c[0] * 16, c[1] * 16, g, ["d"]);
      }
    }
  };
  if (lvl === 1) tint("planed", inLane);
  else if (lvl === 2) {
    tint("fresh_unmarked", (X, Y) => inLane(X, Y) && pos(X, Y) < half);
    tint("planed", (X, Y) => inLane(X, Y) && pos(X, Y) >= half);
  } else if (lvl === 3) tint("fresh_unmarked", inLane);
  else if (lvl === 4) tint("fresh", () => true);
  else if (lvl === 5) {
    // Patched after the works were dropped: the planed lane, square patches of new asphalt over it.
    tint("planed", inLane);
    for (const c of cells) {
      if (hash32(cx.seed, c[0], c[1], 0x9a) % 2) continue;
      const p = new Chars(16, 16);
      const x0 = 2 + (hash32(cx.seed, c[0], c[1], 1) % 5);
      const y0 = 2 + (hash32(cx.seed, c[0], c[1], 2) % 5);
      p.rect(x0, y0, 8, 7, "Z");
      p.rect(x0, y0, 8, 1, "z");
      cx.decal(c[0] * 16, c[1] * 16, p, ["d", "z"]);
    }
  }
  for (const c of cells) if (inLane(c[0] * 16 + 8, c[1] * 16 + 8)) cx.grounds.push([c[0] * 16 + 2, c[1] * 16 + 2, 12, 12]);
}

/** The lot of a slot: cleared, foundations, the slab, the slab left after a stall was cleared. */
function lot(cx: Composer, part: WorksPart) {
  const slot = cx.site.slot;
  if (!slot) return;
  const W = slot.w * 16;
  const H = slot.h * 16;
  const k = part.level ?? 0;
  const g = new Chars(W, H);
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const h = hash32(cx.seed, x >> 1, y >> 1, 0x10) % 9;
      g.set(x, y, h === 0 ? "k" : h < 3 ? "W" : "w");
    }
  }
  // Tyre tracks across the lot.
  for (let x = 0; x < W; x++) for (const ty of [H - 9, H - 5]) if ((x >> 1) % 3 !== 0) g.set(x, ty, "W");
  if (k === 1) {
    // Strip footings round the footprint and along its bays, a trench being dug at the back, starter bars.
    const fx0 = 2;
    const fy0 = 3;
    const fx1 = W - 3;
    const fy1 = H - 3;
    for (let x = fx0; x <= fx1; x++) for (const y of [fy0, fy1]) for (let d = 0; d < 3; d++) g.set(x, y - 1 + d, d === 0 ? "q" : "p");
    for (let y = fy0; y <= fy1; y++) for (let bx = fx0; bx <= fx1; bx += 16) for (let d = 0; d < 3; d++) g.set(Math.min(fx1, bx) - 1 + d, y, d === 0 ? "q" : "p");
    for (let x = fx0 + 2; x < fx1 - 2; x++) g.set(x, fy0 + 3, "O");
    for (let x = fx0; x <= fx1; x += 4) {
      g.set(x, fy1 - 1, "i");
      g.set(x, fy0, "i");
    }
    // Half the ground slab poured.
    for (let y = fy0 + 2; y < fy1 - 1; y++) for (let x = fx0 + 2; x < fx0 + Math.floor((fx1 - fx0) / 2); x++) g.set(x, y, (x - fx0) % 16 === 0 ? "P" : "p");
  } else if (k >= 2) {
    for (let y = 2; y < H - 1; y++) for (let x = 1; x < W - 1; x++) g.set(x, y, (x - 1) % 16 === 0 || (y - 2) % 16 === 0 ? "P" : "p");
    for (let x = 1; x < W - 1; x++) g.set(x, 2, "q");
    if (k === 3) {
      // Left to the weather: cracks across the slab.
      let y = 4 + (hash32(cx.seed, 0xc1) % Math.max(1, H - 8));
      for (let x = 1; x < W - 1; x++) {
        g.set(x, y, "Q");
        if (hash32(cx.seed, x, 0xc2) % 4 === 0) y += hash32(cx.seed, x, 0xc3) % 2 ? 1 : -1;
        y = Math.max(3, Math.min(H - 3, y));
      }
    }
  }
  cx.decal(slot.x * 16, slot.y * 16, g);
  cx.grounds.push([slot.x * 16 + 2, slot.y * 16 + 3, W - 4, H - 5]);
  for (let k2 = 0; k2 < 4; k2++) cx.work.push([slot.x * 16 + 10 + ((k2 * (W - 20)) >> 2), (slot.y + slot.h) * 16 - 4 - (k2 % 2) * 10]);
}

/** A mine's headframe (turning, still, rusted, half-built) on its cell, as round 4 stands it. */
function mineStructure(cx: Composer, part: WorksPart) {
  const at = cx.site.headframe;
  if (!at) return;
  const fx = at[0] * 16 + 8;
  const fy = at[1] * 16 + 15;
  if (part.id.startsWith("headframe")) {
    const t = cx.partAt(part.id, fx, fy, part.id === "headframe_half" ? ["metal"] : ["metal", "headframe"], false, part.frames ?? 1);
    if (t) cx.tops.push({ x0: fx - 12, x1: fx + 11, y: fy - 7, base: fy });
    cx.grounds.push([fx - 22, fy + 2, 44, 12]);
    cx.work.push([fx - 18, fy + 6], [fx + 16, fy + 8]);
  } else if (part.id === "heap") {
    const h = cx.site.heap;
    if (!h || !part.level) return;
    if (part.level <= 3) {
      const sp = cx.kit.props.spoil_heap?.[part.level - 1];
      if (!sp) return;
      cx.image("heap", { w: sp.albedo.w, h: sp.albedo.h, px: sp.albedo.px.slice() }, h[0] * 16 - sp.ax, h[1] * 16 - sp.ay, h[1] * 16 + 16, ["heap"]);
    } else cx.partAt("spoil_tip", h[0] * 16 + 8, h[1] * 16 + 15, ["heap"]);
  }
}

// ---------------------------------------------------------------------------
// Marks

const GRAFFITI_COLOURS = ["3", "4", "5", "R", "6", "c", "a"] as const;

/** A tag: a looping squiggle in one colour with a dark drop shadow (no letters, R14). */
function tag(seed: number, n: number): Chars {
  const w = 9 + (hash32(seed, "tw", n) % 6);
  const c = new Chars(w + 1, 6);
  const col = GRAFFITI_COLOURS[hash32(seed, "tc", n) % GRAFFITI_COLOURS.length] as string;
  const phase = hash32(seed, "tp", n) % 4;
  const pts: [number, number][] = [];
  for (let x = 0; x < w; x++) {
    const k = (x + phase) % 4;
    const y = k === 0 ? 1 : k === 1 ? 0 : k === 2 ? 2 : 3;
    pts.push([x, y], [x, y + 1]);
  }
  for (const [x, y] of pts) if (c.get(x + 1, y + 1) === ".") c.set(x + 1, y + 1, "x");
  for (const [x, y] of pts) c.set(x, y, col);
  return c;
}

const WEEDS = [
  `
  .m.n.
  nNmNn`,
  `
  ..j..
  .mNm.
  nNNNn`,
  `
  n...m
  Nn.nN
  .NnN.`,
  `
  .4.
  mNm
  NnN`,
] as const;

const BIG_WEEDS = [
  `
  ..m..n.
  .mNm.Nn
  nNNMNNn
  NNMNMNN`,
  `
  .4..4..
  .4m.4m.
  mNNmNNm
  NNMNNMN`,
  `
  n..j..n
  Nm.N.mN
  NNmNmNN`,
] as const;

/** Weeds: the first few tufts small, later ones (only there at later stages) grown into clumps. */
function weeds(cx: Composer, n: number, X: number, Y: number) {
  const list = n >= 8 ? BIG_WEEDS : WEEDS;
  const c = Chars.from(list[hash32(cx.seed, "wv", n) % list.length] as string);
  cx.decal(X - (c.w >> 1), Y - c.h + 1, c);
}

function puddle(cx: Composer, n: number, X: number, Y: number, onlyOn?: readonly string[]) {
  const w = 8 + (hash32(cx.seed, "pw", n) % 6);
  const c = new Chars(w, 4);
  for (let y = 0; y < 4; y++) for (let x = 0; x < w; x++) {
    const u = (x - (w - 1) / 2) / (w / 2);
    const v = (y - 1.5) / 2;
    if (u * u + v * v > 1) continue;
    c.set(x, y, y === 0 && x < w / 2 ? "^" : u * u + v * v > 0.6 ? "Z" : "_");
  }
  cx.decal(X - (w >> 1), Y - 2, c, onlyOn);
}

function litter(cx: Composer, n: number, X: number, Y: number) {
  const k = hash32(cx.seed, "lv", n) % 4;
  const c = k === 0 ? Chars.from("cC") : k === 1 ? Chars.from("a\nA") : k === 2 ? Chars.from(".c\nCc") : Chars.from("bB");
  cx.decal(X, Y, c);
}

function pothole(cx: Composer, n: number, X: number, Y: number) {
  const w = 5 + (hash32(cx.seed, "hw", n) % 4);
  const c = new Chars(w, 4);
  for (let y = 0; y < 4; y++) for (let x = 0; x < w; x++) {
    const u = (x - (w - 1) / 2) / (w / 2);
    const v = (y - 1.5) / 2;
    const d = u * u + v * v;
    if (d > 1) continue;
    c.set(x, y, d > 0.55 ? (y >= 2 ? "d" : "O") : y === 0 ? "O" : "x");
  }
  // A crumb of loose stone beside it.
  c.set(w - 1, 3, "d");
  cx.decal(X - (w >> 1), Y - 2, c, ["z", "Z", "d", "c", "C", "O"]);
}

/** A point in one of the site's ground areas, from the mark's number. */
function groundPoint(cx: Composer, id: string, n: number): [number, number] | null {
  if (!cx.grounds.length) return null;
  const g = cx.grounds[hash32(cx.seed, id, n, "area") % cx.grounds.length] as [number, number, number, number];
  return [g[0] + (hash32(cx.seed, id, n, "x") % Math.max(1, g[2])), g[1] + (hash32(cx.seed, id, n, "y") % Math.max(1, g[3]))];
}

/** Paint a small image onto a sprite, only over its pixels that pass `ok`. */
function paintOn(t: Tagged, c: Chars, L: Readonly<Record<string, number>>, x0: number, y0: number, ok: (i: number) => boolean) {
  for (const f of t.out.frames) {
    for (let y = 0; y < c.h; y++) {
      for (let x = 0; x < c.w; x++) {
        const ch = c.get(x, y);
        if (ch === ".") continue;
        const X = x0 + x;
        const Y = y0 + y;
        if (X < 0 || Y < 0 || X >= f.w || Y >= f.h) continue;
        const i = Y * f.w + X;
        if (!f.px[i] || !ok(i)) continue;
        f.px[i] = L[ch] as number;
      }
    }
  }
}

function marks(cx: Composer) {
  const pal = cx.kit.palette;
  const lineSet = new Set(pal.lineOf);
  const L = cx.L;
  const set = (chs: string) => new Set([...chs].map((ch) => L[ch] as number));
  const paintable = set("bBlcCqpPQ78KXLsS9E");
  const metal = set("iIYyjcCq");
  const concrete = set("pPQq");
  const targets = (tag: string) => cx.tagged.filter((t) => t.tags.has(tag));
  for (const part of cx.look.parts) {
    if (!part.mark) continue;
    const n = part.n;
    const h = (salt: string, k = 0) => hash32(cx.seed, part.id, n, salt, k);
    switch (part.id) {
      case "weeds":
      case "litter": {
        const p = groundPoint(cx, part.id, n);
        if (!p) break;
        if (part.id === "weeds") weeds(cx, n, p[0], p[1]);
        else litter(cx, n, p[0], p[1]);
        break;
      }
      case "puddle": {
        const p = groundPoint(cx, part.id, n);
        // Puddles stand on hard or bare ground, not on grass.
        if (p) puddle(cx, n, p[0], p[1], ["p", "P", "Q", "q", "z", "Z", "d", "w", "W", "k", "s", "S", "t", "T", "x", "i", "I", "c", "C"]);
        break;
      }
      case "pothole": {
        // In the closed lane (the planed or old surface), where the asphalt breaks up first.
        const cells = cx.site.cells ?? [];
        const lane = cx.site.lane;
        for (let k = 0; k < 8 && cells.length; k++) {
          const c = cells[h("cell", k) % cells.length] as readonly [number, number];
          const X = c[0] * 16 + 3 + (h("x", k) % 10);
          const Y = c[1] * 16 + 3 + (h("y", k) % 10);
          if (lane && lane.nx * X + lane.ny * Y < lane.d + 3) continue;
          pothole(cx, n, X, Y);
          break;
        }
        break;
      }
      case "graffiti":
      case "rust":
      case "moss":
      case "scrub":
      case "gully": {
        const kinds: Readonly<Record<string, readonly string[]>> = {
          graffiti: ["facade", "frame", "wall", "paint", "cabin", "board", "gates"],
          rust: ["metal", "frame"],
          moss: ["wall", "concrete"],
          scrub: ["heap", "bare"],
          gully: ["bare"],
        };
        let list = (kinds[part.id] as readonly string[]).flatMap(targets);
        // Tags go on what stands in front first (hoarding, a cabin, a wall), most of the time.
        if (part.id === "graffiti" && h("front") % 4 !== 0) {
          const front = ["paint", "wall", "gates"].flatMap(targets);
          if (front.length) list = front;
        }
        // The face of an east–west bridge takes tags too (it is ground: a decal).
        if (part.id === "graffiti" && cx.faceArea && h("face") % 2 === 0) {
          const [fx, fy, fw, fh] = cx.faceArea;
          cx.decal(fx + (h("fx") % Math.max(1, fw - 12)), fy + (h("fy") % Math.max(1, fh - 4)), tag(cx.seed, n), ["s", "S", "t", "T", "K", "B", "L"]);
          break;
        }
        if (part.id === "rust" && cx.beams.length && h("beam") % 2 === 0) {
          // Rust bleeding along the exposed beams of a bridge's gap.
          const b = cx.beams[h("b") % cx.beams.length] as { X: number; Y: number };
          const c = new Chars(6, 1);
          for (let x = 0; x < 6; x++) c.set(x, 0, x % 3 === 0 ? "U" : "u");
          cx.decal(b.X, b.Y, c, ["i", "I"]);
          break;
        }
        if (!list.length) break;
        const t = list[h("t") % list.length] as Tagged;
        const f = t.out.frames[0] as Indexed;
        const faces = (t as Tagged & { faces?: Uint8Array }).faces;
        const above = (i: number) => t.out.base - (t.out.y + Math.floor(i / f.w));
        const eligible = (i: number): boolean => {
          const v = f.px[i] as number;
          if (!v || lineSet.has(v)) return false;
          if (part.id === "graffiti") {
            if (faces) return (faces[i] as number) >= RECEIVER.face + 2 && (faces[i] as number) <= RECEIVER.face + 14;
            return paintable.has(v) && above(i) >= 2 && above(i) <= 16;
          }
          if (part.id === "rust") return metal.has(v) || concrete.has(v);
          if (part.id === "moss") return concrete.has(v) && above(i) <= 6;
          return true;
        };
        const cand: number[] = [];
        for (let i = 0; i < f.px.length; i++) if (eligible(i)) cand.push(i);
        if (!cand.length) break;
        const i0 = cand[h("px") % cand.length] as number;
        const x0 = i0 % f.w;
        const y0 = (i0 - x0) / f.w;
        let c: Chars;
        if (part.id === "graffiti") c = tag(cx.seed, n);
        else if (part.id === "rust") {
          const len = 3 + (h("len") % 5);
          c = new Chars(1, len + 1);
          c.set(0, 0, "v");
          for (let k = 1; k <= len; k++) c.set(0, k, k > len / 2 ? "U" : "u");
        } else if (part.id === "moss") c = Chars.from(h("mv") % 2 ? "nN.\nNNn" : ".n.\nNnN");
        else if (part.id === "scrub") c = Chars.from(h("sv") % 3 === 0 ? ".m.\nnNn" : h("sv") % 3 === 1 ? "mn\nNN" : ".4\nmN");
        else {
          c = new Chars(1, 4);
          for (let k = 0; k < 4; k++) c.set(0, k, "W");
        }
        const ok = part.id === "graffiti" ? (i: number) => eligible(i) || paintable.has(f.px[i] as number) : (i: number) => !lineSet.has(f.px[i] as number);
        paintOn(t, c, L, x0 - (part.id === "graffiti" ? 2 : 0), y0, ok);
        break;
      }
      case "sheet":
      case "buddleia": {
        if (!cx.tops.length) break;
        const top = cx.tops[h("top") % cx.tops.length] as (typeof cx.tops)[number];
        const X = top.x0 + (h("x") % Math.max(1, top.x1 - top.x0));
        if (part.id === "sheet") cx.partAt("sheet", X, top.y + 2, [], false, 2, top.base + 1);
        else cx.partAt("buddleia", X, top.y + 1, [], false, 1, top.base + 1);
        break;
      }
    }
  }
}

// ---------------------------------------------------------------------------

/** Compose a look on a site. */
export function composeWorks(look: WorksLook, site: WorksSite, kit: WorksKit, seed: number): ComposedWorks {
  const cx = new Composer(look, site, kit, seed);
  const yardRect = site.yard;
  cx.grounds.push([yardRect.x * 16 + 2, yardRect.y * 16 + 4, yardRect.w * 16 - 4, yardRect.h * 16 - 6]);
  const kind = look.kind;
  const slot = site.slot;
  // The slot's plan building is drawn by the works while it has a structure there.
  if (slot && look.parts.some((p) => p.role === "structure" || p.role === "ground")) cx.res.replaces = { x: slot.x, y: slot.y, w: slot.w, h: slot.h };
  for (const part of look.parts) {
    const id = part.id;
    switch (part.role) {
      case "ground":
        if (id === "lot") lot(cx, part);
        else if (id === "surface") surface(cx, part);
        else if (id === "collar" || id === "cap") {
          const at = site.headframe;
          if (!at) break;
          if (id === "cap") {
            cx.partAt("shaft_cap", at[0] * 16 + 8, at[1] * 16 + 12, ["concrete"]);
            cx.grounds.push([at[0] * 16 - 8, at[1] * 16, 32, 14]);
          } else {
            const c = Chars.from(`
              ..qqqqqqqq..
              .qpppppppppP
              qpPOOOOOOxpP
              qpOOOOOOOOpP
              qpOOOOOOOOpP
              .pPxxxxxxPP.
              ..PPPPPPPP..`);
            cx.decal(at[0] * 16 + 2, at[1] * 16 + 6, c);
            cx.work.push([at[0] * 16 - 4, at[1] * 16 + 14]);
          }
        }
        break;
      case "structure":
        if (kind === "bridge") {
          if (id === "deck") bridge(cx, part);
          else if (id === "railing") bridgeRailings(cx, part);
          else if (id.startsWith("scaffold")) scaffold(cx, part);
        } else if (kind === "flood_wall" || kind === "levee") lineStructure(cx, part);
        else if (kind === "mine") mineStructure(cx, part);
        else if (id.startsWith("frame")) frame(cx, part.worn ?? 0);
        else if (id.startsWith("facade")) facade(cx, part);
        else if (id.startsWith("scaffold")) scaffold(cx, part);
        else if (id === "chimney" || id === "smoke") {
          const at = site.chimney ?? (slot ? ([slot.x + slot.w - 1, slot.y] as const) : null);
          if (!at) break;
          const fx = at[0] * 16 + 8;
          const fy = at[1] * 16 + 6;
          if (id === "chimney") cx.partAt("chimney", fx, fy, ["chimney"]);
          else cx.partAt("smoke", fx - 1, fy - 61, [], false, part.frames ?? 1, fy + 1);
        }
        break;
      case "perimeter": {
        const style = id as LineStyle;
        let pieces: readonly LinePiece[] | undefined = site.fence;
        if (!pieces && slot) {
          // Along the pavement in front of the slot and its yard (the sides stand behind the buildings either side), two cells left open as the gate into the yard.
          const x0 = Math.min(slot.x, site.yard.x);
          const x1 = Math.max(slot.x + slot.w, site.yard.x + site.yard.w) - 1;
          const gx = site.gate ? site.gate[0] : site.yard.x + 1;
          const ps: LinePiece[] = [];
          for (let x = x0; x <= x1; x++) if (x !== gx && x !== gx + 1) ps.push({ x, y: slot.y + slot.h, a: "w", b: "e" });
          pieces = ps;
        }
        if (!pieces && site.headframe) {
          const [hx, hy] = site.headframe;
          pieces = rectPieces(hx - 1, hy - 1, hx + 1, hy + 1, [[hx, hy + 1]]);
        }
        if (pieces) cx.run(id, style, runCentres(pieces), part, style === "hoarding" ? ["paint"] : ["metal"]);
        break;
      }
      case "closure": {
        const c = site.connector;
        if (c && id === "barrier") {
          const ew = c.axis !== "ns";
          const run = (ew ? c.w : c.h) * 16;
          const span = (ew ? c.h : c.w) * 16;
          if (part.n === 0) {
            // One lane closed: a barrier along the middle of the deck.
            const mid = Math.floor(span / 2);
            cx.run("lane", "barrier", ew ? straightCentres(c.x * 16 + 4, c.y * 16 + mid, run - 8, 0) : straightCentres(c.x * 16 + mid, c.y * 16 + 4, run - 8, 1), part, ["paint"]);
          } else {
            // The bridge closed: barriers across the way at both ends.
            for (const at of [2, run - 3]) {
              const cs: CentrePx[] = [];
              for (let a = 3; a < span - 3; a++) cs.push(ew ? { X: c.x * 16 + at, Y: c.y * 16 + a, dir: 1, piece: at } : { X: c.x * 16 + a, Y: c.y * 16 + at, dir: 0, piece: at });
              cx.run(`closed${at}`, "barrier", cs, part, ["paint"]);
            }
          }
        } else if (site.closure && id === "barrier") cx.run("closure", "barrier", site.closure, part, ["paint"]);
        break;
      }
      case "yard": {
        const [fx, fy] = cx.yardFoot(part.n);
        const tags = id.startsWith("cabin") ? ["cabin", "paint"] : id.startsWith("excavator") ? ["metal"] : id === "rubble" || id.startsWith("pile") ? ["pile"] : [];
        cx.partAt(id, fx, fy, tags, part.lit ?? false, part.frames ?? 1);
        if (id.startsWith("cabin")) cx.work.push([fx + 18, fy + 4]);
        break;
      }
      case "front": {
        const [bx, by] = site.front;
        if (id === "gates_open" || id.startsWith("gates_shut")) {
          const g = site.gate ?? site.front;
          cx.partAt(id, g[0] * 16 + 16, g[1] * 16 + 15, ["gates", "metal"]);
        } else if (part.n === 0) cx.partAt(id, bx * 16 + 8, by * 16 + 13, ["board"]);
        else if ((id === "road_sign" || id === "road_sign_faded") && site.signs?.[part.n - 1]) {
          const s = site.signs[part.n - 1] as readonly [number, number];
          cx.partAt(id, s[0], s[1], ["metal"]);
        } else if (part.n <= 2) cx.partAt(id, bx * 16 + 8 + 16 * part.n, by * 16 + 13, id === "hazard_sign" ? ["metal"] : []);
        else {
          // Cones: along the closure line (a road, a bridge's lane), else by the front.
          const line = site.closure ?? (site.connector ? bridgeLine(site.connector) : null);
          if (line && line.length) {
            const k = part.n - 3;
            const at = line[Math.min(line.length - 1, Math.floor(((k + 0.5) * line.length) / 4))] as CentrePx;
            cx.partAt(id, at.X + (at.dir === 1 ? 4 : 0), at.Y + (at.dir === 1 ? 0 : 5), []);
          } else cx.partAt(id, bx * 16 + 4 + 6 * (part.n - 3), by * 16 + 15, []);
        }
        break;
      }
      case "entrance": {
        const [ex, ey] = site.entrance;
        const c = site.connector;
        if (id === "ribbon" && c) {
          const ew = c.axis !== "ns";
          // Across the deck at its far end (east or south), where the first walk over begins.
          if (ew) cx.partAt("ribbon_ns", (c.x + c.w) * 16 - 6, (c.y + c.h) * 16 - 2, []);
          else cx.partAt("ribbon", c.x * 16 + (c.w * 16) / 2, (c.y + c.h) * 16 - 4, []);
        } else if (id === "ribbon" && slot) cx.partAt("ribbon", (slot.x + slot.doorX) * 16 + 8, (slot.y + slot.h) * 16 + 7, []);
        else if (id === "plaque" && slot && cx.look.parts.some((p) => p.id === "facade")) cx.partAt(id, (slot.x + slot.doorX) * 16 + 24, (slot.y + slot.h) * 16 + 5, []);
        else cx.partAt(id, ex * 16 + 8, ey * 16 + 12, []);
        break;
      }
      case "crane": {
        const at: [number, number] = slot ? [slot.x * 16 + 12, slot.y * 16 + 6] : (() => {
          const [x, y] = cx.yardFoot(3);
          return [x, y - 10];
        })();
        cx.partAt(id, at[0], at[1], ["metal"], part.lit ?? false);
        break;
      }
      default:
        break;
    }
  }
  // Workers stand at the work: along the structure, by the cabin.
  const people = look.parts.filter((p) => p.role === "people");
  if (people.length) {
    const spots = cx.work.length ? cx.work : [cx.yardFoot(6)];
    people.forEach((p, i) => {
      const s = spots[(hash32(seed, "ws", p.n) + i * 2) % spots.length] as [number, number];
      cx.res.people.push({ x: s[0] + ((p.n * 7) % 9) - 4, y: s[1], look: workerLook(seed, p.n) });
    });
  }
  marks(cx);
  cx.res.sprites = cx.tagged.map((t) => t.out);
  return cx.res;
}

/** A bridge's lane-closure line (for its cones). */
function bridgeLine(c: ConnectorView): CentrePx[] {
  const ew = c.axis !== "ns";
  const run = (ew ? c.w : c.h) * 16;
  const span = (ew ? c.h : c.w) * 16;
  const mid = Math.floor(span / 2);
  return ew ? straightCentres(c.x * 16 + 4, c.y * 16 + mid, run - 8, 0) : straightCentres(c.x * 16 + mid, c.y * 16 + 4, run - 8, 1);
}
