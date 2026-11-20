// The interior composer (DEC-088): a room from a recipe (./interiorsRecipes)
// and a keyed seed, in the street's scale and view. Pure and integer-only on
// the art side: keyed hashes and rendezvous picks (no Math.random, no clock),
// so the same recipe version and seed give the same pixels on every device
// (DEC-085's rules, applied to rooms).
//
// What a room is:
//   - `base`: the back wall (surfaces, trims, windows, doors, things hung on
//     it), the floor (surfaces, patches, a dais, mats) and the wall tops
//     around the room, with furniture shadows baked in;
//   - `sprites`: furniture and what stands on it, each with a depth key, so
//     people (drawn by the renderer from the engine's place view) slot in
//     between them; seats split into the part drawn behind a sitter and the
//     part drawn over them (a seat's front, a pew's backrest);
//   - `outdoor`: which base pixels are the view through a window (graded as
//     the street is), `glow` and `lights`: what lights up, and where its
//     light falls; `windows`: where daylight comes in;
//   - `walk`: the tile grid (blocked, floor, seat), the door, and the
//     recipe's spots and hotspots for the options the seed chose.
//
// `renderInterior` is the reference renderer used for the sample sheets: it
// grades the room for a time of day with the pack's lighting (./lighting),
// as the web renderer would (README, "Interiors").
import { hexToRgb, type Rgb8 } from "../../src/core/color";
import { hash32, rendezvous } from "../../src/core/hash";
import { blank, blit, crop, outline, type Indexed } from "../../src/core/image";
import { ramp as rampByName, type Palette, type Ramp } from "../../src/core/palette";
import { ANCHOR, composeCharacter, faceLeft, lookFromSeed, type Look } from "./characters";
import type { Lighting } from "./lighting";
import { FAR_LIT_CHARS, FIXED_RAMPS, ICHARS, ILEGEND, OUTDOOR_CHARS, PART_BY_ID, partSlots, rugSlots, SCODES, SLEGEND, surfaceSlots, type PartDef, type VarRole } from "./interiorsParts";
import type { Facing, HotspotDef, InteriorRecipe, OptionDef, PatchDef, PieceDef, RampRef, SpotDef, SurfaceRef, WallStyle, Weighted } from "./interiorsRecipes";

/** Bump when the composer changes what a room looks like for the same recipe version and seed. */
export const INTERIOR_COMPOSER_VERSION = 1;
/** Wall-top bands around the room (px): sides, above the back wall, the front wall. */
export const BAND = { side: 8, top: 8, front: 8 } as const;
/** How far a dais lifts what stands on it (px). */
export const DAIS_LIFT = 3;

export interface InteriorSprite {
  part: string;
  x: number;
  y: number;
  img: Indexed;
  /** Depth: draw in ascending order with people (whose key is `feetY * 4`). */
  key: number;
  /** For seats: the part drawn over a sitter. */
  front?: { img: Indexed; y: number; key: number };
  /** Lit pixels (palette indices, same frame as `img`), or null. */
  emissive: Indexed | null;
  glow: "lamps" | "always";
  /** Seats: where a sitter's hips go (px), per tile of the footprint (top to bottom). */
  sitY?: number[];
  tiles: { x: number; y: number; w: number; h: number };
}

export interface InteriorLight {
  x: number;
  y: number;
  rx: number;
  ry: number;
  glow: "lamps" | "always" | "ceiling";
}

export type TileState = 0 | 1 | 2;
/** Walk grid values. */
export const TILE = { blocked: 0, floor: 1, seat: 2 } as const;

export interface InteriorRoom {
  recipe: string;
  version: number;
  composer: number;
  seed: number;
  cols: number;
  rows: number;
  w: number;
  h: number;
  /** The floor's top-left in the image (px). */
  floor: { x: number; y: number };
  wallH: number;
  base: Indexed;
  /** 1 where `base` shows the outdoors (a window's view). */
  outdoor: Uint8Array;
  /** Lit pixels of the base (wall lights, signs, far windows), palette indices. */
  baseGlow: Indexed;
  /** Far windows seen through the glass: lit at night. */
  farLit: Uint8Array;
  sprites: InteriorSprite[];
  lights: InteriorLight[];
  windows: { x: number; w: number }[];
  walk: Uint8Array;
  /** Per tile: how far a dais lifts it (px). */
  lift: Uint8Array;
  door: { x: number; y: number };
  spots: SpotDef[];
  hotspots: HotspotDef[];
  /** What the seed chose: ramps per role, the floor and wall styles, each choice's option. */
  picks: Record<string, string>;
  lamp: "warm" | "white";
  night: number;
}

// ---------------------------------------------------------------------------
// Picks

const weightOf = <T,>(w: Weighted<T>): { value: T; weight: number } => (typeof w === "object" && w !== null && "value" in w && "weight" in w ? (w as { value: T; weight: number }) : { value: w as T, weight: 1 });

/** A weighted pick by rendezvous hashing (adding an option only takes the picks it wins). */
function pickW<T>(seed: number, salt: string, list: readonly Weighted<T>[], idOf: (v: T, i: number) => string): T {
  if (list.length === 0) throw new Error(`nothing to pick for ${salt}`);
  const items = list.map((w, i) => {
    const { value, weight } = weightOf(w);
    return { id: idOf(value, i), weight, value };
  });
  return rendezvous(seed, salt, items).value;
}

const chance = (seed: number, salt: string, p: number) => hash32(seed, salt) / 4294967296 < p;

interface RampSel {
  ramp: Ramp;
  shift: number;
}

function resolveRef(pal: Palette, ref: RampRef, roles: Partial<Record<VarRole, RampSel>>): RampSel {
  if (ref.startsWith("@")) {
    const r = roles[ref.slice(1) as VarRole];
    if (!r) throw new Error(`ramp ${ref}: role not resolved yet`);
    return r;
  }
  const m = /^([a-z0-9_]+)([+-]\d+)?$/.exec(ref);
  if (!m) throw new Error(`bad ramp reference "${ref}"`);
  return { ramp: rampByName(pal, m[1] as string), shift: m[2] ? Number(m[2]) : 0 };
}

const stepOf = (s: RampSel, k: number): number => s.ramp.steps[Math.max(0, Math.min(s.ramp.steps.length - 1, s.ramp.anchorStep + s.shift + k))] as number;

/** One step darker in a colour's own ramp (shadows), or the colour itself at the ramp's end. */
function darker(pal: Palette, c: number): number {
  const ri = pal.rampOf[c] as number;
  const k = pal.stepOf[c] as number;
  const r = ri >= 0 ? pal.ramps[ri] : undefined;
  return r && k > 0 ? (r.steps[k - 1] as number) : c;
}

// ---------------------------------------------------------------------------
// Footprints

export interface PlacedPiece {
  def: PartDef;
  piece: PieceDef;
  part: string;
  /** Footprint in tiles (floor parts). */
  tiles: { x: number; y: number; w: number; h: number } | null;
}

/** A part's footprint in tiles for a placement (null for wall parts and decals' none). */
export function footprint(def: PartDef, p: PieceDef): { x: number; y: number; w: number; h: number } | null {
  if (def.kind === "wall" || def.kind === "top") return null;
  const len = p.len ?? 1;
  const y = p.y ?? 0;
  if (def.strip) {
    const across = def.foot?.[def.strip.dir === "h" ? 1 : 0] ?? 1;
    return def.strip.dir === "h" ? { x: p.x, y, w: len, h: across } : { x: p.x, y, w: across, h: len };
  }
  const [fw, fh] = def.foot ?? [1, 1];
  return { x: p.x, y, w: fw, h: fh };
}

/** The pieces, patches, spots and hotspots of a recipe for chosen options (by choice id). */
export function arrangement(r: InteriorRecipe, options: Readonly<Record<string, string>>) {
  const chosen: OptionDef[] = r.choices.map((c) => {
    const o = c.options.find((x) => x.id === options[c.id]);
    if (!o) throw new Error(`${r.id}: no option "${options[c.id]}" for "${c.id}"`);
    return o;
  });
  return {
    pieces: [...r.pieces, ...chosen.flatMap((o) => o.pieces)],
    patches: [...r.patches, ...chosen.flatMap((o) => o.patches ?? [])],
    spots: [...r.spots, ...chosen.flatMap((o) => o.spots ?? [])],
    hotspots: [...r.hotspots, ...chosen.flatMap((o) => o.hotspots ?? [])],
  };
}

/** Which alternative and whether a piece is present, for a seed. */
function resolvePiece(seed: number, p: PieceDef, i: number): string | null {
  const salt = `piece:${i}:${p.x},${p.y ?? "w"}`;
  if (p.chance !== undefined && !chance(seed, `${salt}:chance`, p.chance)) return null;
  return typeof p.part === "string" ? p.part : pickW(seed, salt, p.part, (v) => v);
}

/**
 * The walk grid for a set of placed parts: furniture blocks its tiles, seats
 * are walkable (a sitter sits on them), everything else is floor.
 */
export function walkGrid(cols: number, rows: number, placed: readonly PlacedPiece[]): Uint8Array {
  const walk = new Uint8Array(cols * rows).fill(TILE.floor);
  for (const p of placed) {
    if (!p.tiles || p.def.kind === "decal") continue;
    for (let y = p.tiles.y; y < p.tiles.y + p.tiles.h; y++) {
      for (let x = p.tiles.x; x < p.tiles.x + p.tiles.w; x++) {
        if (x < 0 || y < 0 || x >= cols || y >= rows) continue;
        walk[y * cols + x] = p.def.kind === "seat" ? TILE.seat : TILE.blocked;
      }
    }
  }
  return walk;
}

/** Tiles reachable from `from` over floor and seats (4-neighbour). */
export function reachable(cols: number, rows: number, walk: Uint8Array, from: { x: number; y: number }): Uint8Array {
  const seen = new Uint8Array(cols * rows);
  const at = (x: number, y: number) => y * cols + x;
  if (walk[at(from.x, from.y)] === TILE.blocked) return seen;
  const queue: number[] = [at(from.x, from.y)];
  seen[at(from.x, from.y)] = 1;
  while (queue.length) {
    const i = queue.shift() as number;
    const x = i % cols;
    const y = (i - x) / cols;
    for (const [dx, dy] of [
      [1, 0],
      [-1, 0],
      [0, 1],
      [0, -1],
    ] as const) {
      const nx = x + dx;
      const ny = y + dy;
      if (nx < 0 || ny < 0 || nx >= cols || ny >= rows) continue;
      const j = at(nx, ny);
      if (seen[j] || walk[j] === TILE.blocked) continue;
      seen[j] = 1;
      queue.push(j);
    }
  }
  return seen;
}

// ---------------------------------------------------------------------------
// Composition

/** The seed a room is built from: the town's seed and the place's id (keyed, independent of the recipe version). */
export const interiorSeed = (townSeed: number, placeId: number) => hash32(townSeed, "interior", placeId);

/** Build a room. */
export function composeInterior(r: InteriorRecipe, seed: number, pal: Palette, lightness: readonly number[]): InteriorRoom {
  const key = hash32(seed, r.id, r.version);
  const picks: Record<string, string> = {};

  // Materials.
  const roles: Partial<Record<VarRole, RampSel>> = {};
  for (const role of ["wood", "uph", "acc", "acc2"] as const) {
    const ref = pickW(key, `role:${role}`, r.roles[role], (v) => v);
    roles[role] = resolveRef(pal, ref, roles);
    picks[role] = ref;
  }
  const floorStyle = pickW<SurfaceRef>(key, "floor", r.floors, (v, i) => `${v.surface}:${v.ramps.join(",")}:${i}`);
  const wallStyle = pickW<WallStyle>(key, "wall", r.walls, (v) => v.id);
  picks.floor = `${floorStyle.surface}(${floorStyle.ramps.join(",")})`;
  picks.wall = wallStyle.id;
  roles.wall = resolveRef(pal, wallStyle.upper.ramps[0] as string, roles);
  roles.trim = resolveRef(pal, wallStyle.trim, roles);

  // Arrangement.
  const options: Record<string, string> = {};
  for (const c of r.choices) {
    const o = pickW<OptionDef>(key, `choice:${c.id}`, c.options.map((x) => ({ value: x, weight: x.weight ?? 1 })), (v) => v.id);
    options[c.id] = o.id;
    picks[`choice:${c.id}`] = o.id;
  }
  const arr = arrangement(r, options);

  // The room's colour table: slot code → palette index.
  const table = new Uint8Array(256);
  ICHARS.forEach((ch, code) => {
    if (!ch) return;
    const [role, k] = ILEGEND[ch] as readonly [string, number];
    const sel = (roles as Record<string, RampSel | undefined>)[role] ?? { ramp: rampByName(pal, FIXED_RAMPS[role] as string), shift: 0 };
    table[code] = stepOf(sel, k);
  });
  const surfaceTable = (ref: SurfaceRef): Uint8Array => {
    const sels = ref.ramps.map((x) => resolveRef(pal, x, roles));
    const t = new Uint8Array(256);
    for (const [ch, [ri, k]] of Object.entries(SLEGEND)) t[SCODES[ch] as number] = stepOf((sels[ri] ?? sels[0]) as RampSel, k);
    return t;
  };

  const cols = r.cols;
  const rows = r.rows;
  const fx = BAND.side;
  const fy = BAND.top + r.wall_h;
  const W = cols * 16 + 2 * BAND.side;
  const H = fy + rows * 16 + BAND.front;
  const base = blank(W, H);
  const outdoor = new Uint8Array(W * H);
  const farLit = new Uint8Array(W * H);
  const baseGlow = blank(W, H);
  const ink = stepOf({ ramp: rampByName(pal, "ink"), shift: 0 }, -1);
  base.px.fill(ink);

  // ---- the back wall
  const fillSurface = (ref: SurfaceRef, x: number, y: number, w: number, h: number, ox = 0, oy = 0) => {
    const tile = surfaceSlots(ref.surface);
    const t = surfaceTable(ref);
    for (let yy = y; yy < y + h; yy++) {
      for (let xx = x; xx < x + w; xx++) {
        const v = tile.px[((((yy - y + oy) % 16) + 16) % 16) * 16 + ((((xx - x + ox) % 16) + 16) % 16)] as number;
        base.px[yy * W + xx] = t[v] as number;
      }
    }
  };
  const trim = roles.trim;
  const tK = stepOf(trim, 1);
  const td = stepOf(trim, 0);
  const tD = stepOf(trim, -1);
  const row = (y: number, v: number, x0 = fx, x1 = fx + cols * 16) => {
    for (let x = x0; x < x1; x++) base.px[y * W + x] = v;
  };
  const wallTop = BAND.top;
  fillSurface(wallStyle.upper, fx, wallTop, cols * 16, r.wall_h);
  if (wallStyle.lower) {
    const lh = wallStyle.lower.h;
    fillSurface(wallStyle.lower, fx, fy - lh, cols * 16, lh);
    // Dado rail.
    row(fy - lh - 2, tK);
    row(fy - lh - 1, tD);
  }
  // Cornice and skirting.
  row(wallTop, tD);
  row(wallTop + 1, tK);
  row(wallTop + 2, td);
  row(wallTop + 3, tD);
  row(fy - 3, tK);
  row(fy - 2, td);
  row(fy - 1, tD);

  // ---- the floor
  fillSurface(floorStyle, fx, fy, cols * 16, rows * 16);
  const lift = new Uint8Array(cols * rows);
  for (const p of arr.patches) drawPatch(p);
  function drawPatch(p: PatchDef) {
    const ref: SurfaceRef = { surface: p.surface ?? floorStyle.surface, ramps: p.ramps ?? floorStyle.ramps };
    const x0 = fx + p.x * 16;
    const y0 = fy + p.y * 16;
    if (p.kind === "surface") fillSurface(ref, x0, y0, p.w * 16, p.h * 16);
    else if (p.kind === "dais") {
      // The platform's top, raised: the surface lifted by DAIS_LIFT, a nosing and the step's face.
      const bottom = y0 + p.h * 16;
      fillSurface(ref, x0, y0, p.w * 16, p.h * 16 - DAIS_LIFT);
      const wood = roles.wood as RampSel;
      for (let x = x0; x < x0 + p.w * 16; x++) {
        base.px[(bottom - DAIS_LIFT - 1) * W + x] = stepOf(wood, 1);
        for (let k = 0; k < DAIS_LIFT; k++) base.px[(bottom - DAIS_LIFT + k) * W + x] = stepOf(wood, k === 0 ? 0 : -1);
      }
      for (let y = y0; y < bottom; y++) {
        base.px[y * W + x0] = stepOf(wood, 1);
        base.px[y * W + x0 + p.w * 16 - 1] = stepOf(wood, -1);
      }
      for (let ty = p.y; ty < p.y + p.h; ty++) for (let tx = p.x; tx < p.x + p.w; tx++) lift[ty * cols + tx] = DAIS_LIFT;
    } else if (p.kind === "rug") drawRug(x0, y0, p.w * 16, p.h * 16);
  }
  function drawRug(x0: number, y0: number, w: number, h: number) {
    const s = rugSlots();
    const img = blank(w - 4, h - 4);
    for (let y = 0; y < img.h; y++) for (let x = 0; x < img.w; x++) img.px[y * img.w + x] = s.field.px[(y % s.field.h) * s.field.w + (x % s.field.w)] as number;
    for (let x = s.corner.w; x < img.w - s.corner.w; x++) {
      for (let y = 0; y < s.edgeTop.h; y++) {
        const v = s.edgeTop.px[y * s.edgeTop.w + (x % s.edgeTop.w)] as number;
        img.px[y * img.w + x] = v;
        img.px[(img.h - 1 - y) * img.w + x] = v;
      }
    }
    for (let y = s.corner.h; y < img.h - s.corner.h; y++) {
      for (let x = 0; x < s.edgeTop.h; x++) {
        const v = s.edgeTop.px[x * s.edgeTop.w + (y % s.edgeTop.w)] as number;
        img.px[y * img.w + x] = v;
        img.px[y * img.w + img.w - 1 - x] = v;
      }
    }
    for (let y = 0; y < s.corner.h; y++) {
      for (let x = 0; x < s.corner.w; x++) {
        const v = s.corner.px[y * s.corner.w + x] as number;
        img.px[y * img.w + x] = v;
        img.px[y * img.w + img.w - 1 - x] = v;
        img.px[(img.h - 1 - y) * img.w + x] = v;
        img.px[(img.h - 1 - y) * img.w + img.w - 1 - x] = v;
      }
    }
    blit(base, img, x0 + 2, y0 + 2, { map: table });
  }
  // Shade under the skirting and along the left wall.
  for (let x = fx; x < fx + cols * 16; x++) for (const y of [fy, fy + 1]) base.px[y * W + x] = darker(pal, base.px[y * W + x] as number);
  for (let y = fy; y < fy + rows * 16; y++) base.px[y * W + fx] = darker(pal, base.px[y * W + fx] as number);

  // ---- pieces
  const placed: PlacedPiece[] = [];
  arr.pieces.forEach((p, i) => {
    const part = resolvePiece(key, p, i);
    if (!part) return;
    const def = PART_BY_ID.get(part);
    if (!def) throw new Error(`${r.id}: unknown part "${part}"`);
    placed.push({ def, piece: p, part, tiles: footprint(def, p) });
  });

  const recolour = (slots: Indexed): Indexed => {
    const out = blank(slots.w, slots.h);
    for (let i = 0; i < slots.px.length; i++) out.px[i] = table[slots.px[i] as number] as number;
    return out;
  };
  const litMask = (slots: Indexed, def: PartDef): Indexed | null => {
    if (!def.lit) return null;
    const m = blank(slots.w, slots.h);
    let any = false;
    for (let i = 0; i < slots.px.length; i++) {
      const ch = ICHARS[slots.px[i] as number] ?? "";
      if (ch && def.lit.includes(ch)) {
        m.px[i] = table[slots.px[i] as number] as number;
        any = true;
      }
    }
    return any ? m : null;
  };

  const lights: InteriorLight[] = [];
  const windows: { x: number; w: number }[] = [];
  const centroid = (m: Indexed, ox: number, oy: number) => {
    let sx = 0;
    let sy = 0;
    let n = 0;
    for (let y = 0; y < m.h; y++) for (let x = 0; x < m.w; x++) if (m.px[y * m.w + x]) (sx += x, sy += y, n++);
    return n ? { x: ox + Math.round(sx / n), y: oy + Math.round(sy / n) } : null;
  };

  // Wall parts and decals go into the base.
  for (const pp of placed) {
    const { def, piece } = pp;
    if (def.kind !== "wall" && def.kind !== "decal") continue;
    if (piece.front) continue;
    const slots = partSlots(pp.part, piece.len ?? 1, piece.mirror ?? false);
    const img = recolour(slots);
    let x: number;
    let y: number;
    if (def.kind === "wall") {
      x = fx + piece.x * 16 + 8 - Math.floor(slots.w / 2) + (piece.dx ?? 0);
      y = piece.floor ? fy - slots.h : wallTop + (piece.wy ?? Math.floor(r.wall_h / 2)) - Math.floor(slots.h / 2) + (piece.dy ?? 0);
    } else {
      x = fx + piece.x * 16 + 8 - Math.floor(slots.w / 2) + (piece.dx ?? 0);
      y = fy + (piece.y ?? 0) * 16 + 8 - Math.floor(slots.h / 2) + (piece.dy ?? 0);
    }
    blit(base, img, x, y);
    for (let sy = 0; sy < slots.h; sy++) {
      for (let sx = 0; sx < slots.w; sx++) {
        const ch = ICHARS[slots.px[sy * slots.w + sx] as number] ?? "";
        if (!ch) continue;
        const i = (y + sy) * W + x + sx;
        if (OUTDOOR_CHARS.includes(ch)) outdoor[i] = 1;
        if (FAR_LIT_CHARS.includes(ch)) farLit[i] = 1;
      }
    }
    const lit = litMask(slots, def);
    if (lit) {
      blit(baseGlow, lit, x, y);
      const c = centroid(lit, x, y);
      if (c) lights.push(def.glow === "always" ? { x: c.x, y: c.y + 4, rx: 22, ry: 14, glow: "always" } : { x: c.x, y: c.y + 6, rx: 30, ry: 24, glow: "lamps" });
    }
    if (pp.part.startsWith("window")) windows.push({ x: x + 2, w: slots.w - 4 });
  }

  // Furniture, seats and what stands on them become sprites.
  const sprites: InteriorSprite[] = [];
  const liftAt = (t: { x: number; y: number; w: number; h: number }) => {
    let m = 255;
    for (let y = t.y; y < t.y + t.h; y++) for (let x = t.x; x < t.x + t.w; x++) m = Math.min(m, lift[y * cols + x] ?? 0);
    return m === 255 ? 0 : m;
  };
  for (const pp of placed) {
    const { def, piece } = pp;
    if (def.kind === "wall" || def.kind === "decal") continue;
    const slots = partSlots(pp.part, piece.len ?? 1, piece.mirror ?? false);
    const padded = blank(slots.w + 2, slots.h + 2);
    blit(padded, recolour(slots), 1, 1);
    const img = outline(padded, pal.lineOf, lightness);
    const lit = litMask(slots, def);
    let emissive: Indexed | null = null;
    if (lit) {
      emissive = blank(padded.w, padded.h);
      blit(emissive, lit, 1, 1);
    }
    const ty = piece.y ?? 0;
    let gx: number;
    let gy: number;
    let k: number;
    let tiles: { x: number; y: number; w: number; h: number };
    if (def.kind === "top") {
      // Resting on the furniture under it: drawn just after it.
      const support = placed.find((q) => q.tiles && q.def.kind !== "top" && piece.x >= q.tiles.x && piece.x < q.tiles.x + q.tiles.w && ty >= q.tiles.y && ty < q.tiles.y + q.tiles.h);
      const st = support?.tiles ?? { x: piece.x, y: ty, w: 1, h: 1 };
      const l = liftAt({ x: piece.x, y: ty, w: 1, h: 1 });
      gx = fx + piece.x * 16 + (piece.dx ?? 0);
      gy = fy + ty * 16 + (def.rest ?? 1) + (piece.dy ?? 0) - l - slots.h;
      k = (fy + (st.y + st.h) * 16) * 4 + 2;
      tiles = { x: piece.x, y: ty, w: 1, h: 1 };
    } else {
      tiles = pp.tiles as NonNullable<PlacedPiece["tiles"]>;
      const l = liftAt(tiles);
      const bottom = fy + (tiles.y + tiles.h) * 16 + (piece.dy ?? 0) - l;
      gx = fx + tiles.x * 16 + (piece.dx ?? 0) + (slots.w < tiles.w * 16 ? Math.floor((tiles.w * 16 - slots.w) / 2) : 0);
      gy = bottom - slots.h;
      k = def.kind === "seat" ? (fy + tiles.y * 16) * 4 + 1 : (fy + (tiles.y + tiles.h) * 16) * 4;
    }
    const s: InteriorSprite = { part: pp.part, x: gx - 1, y: gy - 1, img, key: k, emissive, glow: def.glow ?? "lamps", tiles };
    if (def.kind === "seat") {
      const fr = def.front ?? 99;
      if (fr < slots.h) {
        const frontImg = crop(img, 0, fr + 1, img.w, img.h - fr - 1);
        const main = crop(img, 0, 0, img.w, fr + 1);
        s.img = main;
        s.front = { img: frontImg, y: gy + fr, key: (fy + (tiles.y + tiles.h) * 16) * 4 + 1 };
      }
      const sit = def.sit ?? slots.h - 4;
      s.sitY = Array.from({ length: tiles.h }, (_, i) => gy + sit + i * 16);
    }
    sprites.push(s);
    if (lit) {
      const c = centroid(lit, gx, gy);
      if (c) lights.push(def.glow === "always" ? { x: c.x, y: c.y + 6, rx: 20, ry: 12, glow: "always" } : { x: c.x, y: c.y + 4, rx: 22, ry: 14, glow: "lamps" });
    }
  }
  sprites.sort((a, b) => a.key - b.key || a.y - b.y || a.x - b.x);

  // Furniture shadows on the floor and wall: one step darker, offset down and right of each piece.
  const inRoom = (x: number, y: number) => x >= fx && x < fx + cols * 16 && y >= wallTop && y < fy + rows * 16;
  const shade = new Uint8Array(W * H);
  for (const s of sprites) {
    if (PART_BY_ID.get(s.part)?.kind === "top") continue;
    for (const layer of [{ img: s.img, y: s.y }, ...(s.front ? [{ img: s.front.img, y: s.front.y }] : [])]) {
      for (let y = 0; y < layer.img.h; y++) {
        for (let x = 0; x < layer.img.w; x++) {
          if (!layer.img.px[y * layer.img.w + x]) continue;
          const X = s.x + x + 2;
          const Y = layer.y + y + 1;
          if (inRoom(X, Y)) shade[Y * W + X] = 1;
        }
      }
    }
  }
  for (let i = 0; i < W * H; i++) if (shade[i] && !outdoor[i]) base.px[i] = darker(pal, base.px[i] as number);

  // Ceiling lights (unseen, above the room).
  for (const c of r.light.ceiling) lights.push({ x: fx + c.x * 16 + 8, y: fy + c.y * 16 + 8, rx: 44, ry: 30, glow: "ceiling" });

  // ---- the wall tops around the room, the door gap in the front wall
  // The cut tops of the walls: dark, so the room reads as a box seen from above; a lip of the trim on the room side.
  const capL = stepOf({ ramp: rampByName(pal, "asphalt"), shift: 0 }, -1);
  const capD = stepOf(roles.trim as RampSel, 0);
  const band = (x0: number, y0: number, w: number, h: number) => {
    for (let y = y0; y < y0 + h; y++) for (let x = x0; x < x0 + w; x++) base.px[y * W + x] = capL;
  };
  band(0, 0, W, BAND.top);
  band(0, 0, BAND.side, H);
  band(W - BAND.side, 0, BAND.side, H);
  band(0, H - BAND.front, W, BAND.front);
  // Inner edges (the wall's thickness reads by a dark line on the room side) and the outer ink line.
  for (let x = BAND.side - 1; x <= W - BAND.side; x++) base.px[(BAND.top - 1) * W + x] = capD;
  for (let y = BAND.top - 1; y < H - BAND.front + 1; y++) {
    base.px[y * W + BAND.side - 1] = capD;
    base.px[y * W + W - BAND.side] = capD;
  }
  for (let x = BAND.side - 1; x <= W - BAND.side; x++) base.px[(H - BAND.front) * W + x] = capD;
  for (let x = 0; x < W; x++) {
    base.px[x] = ink;
    base.px[(H - 1) * W + x] = ink;
  }
  for (let y = 0; y < H; y++) {
    base.px[y * W] = ink;
    base.px[y * W + W - 1] = ink;
  }
  // The entrance: the floor runs out through the front wall between two jambs.
  const dx0 = fx + r.door * 16;
  // Parts on the front wall's top (the bell over the door) go over the band.
  const frontParts = placed.filter((pp) => pp.def.kind === "wall" && pp.piece.front);
  for (let y = H - BAND.front; y < H; y++) {
    for (let x = dx0; x < dx0 + 16; x++) base.px[y * W + x] = base.px[(y - 16) * W + x] as number;
    base.px[y * W + dx0 - 1] = capD;
    base.px[y * W + dx0 + 16] = capD;
  }
  for (let x = dx0; x < dx0 + 16; x++) base.px[(H - 1) * W + x] = darker(pal, base.px[(H - 2) * W + x] as number);
  for (const pp of frontParts) {
    const slots = partSlots(pp.part, 1, pp.piece.mirror ?? false);
    blit(base, recolour(slots), fx + pp.piece.x * 16 + 8 - Math.floor(slots.w / 2) + (pp.piece.dx ?? 0), H - BAND.front - slots.h + 4 + (pp.piece.dy ?? 0));
  }

  // ---- walk grid, spots, hotspots
  const walk = walkGrid(cols, rows, placed);
  return {
    recipe: r.id,
    version: r.version,
    composer: INTERIOR_COMPOSER_VERSION,
    seed,
    cols,
    rows,
    w: W,
    h: H,
    floor: { x: fx, y: fy },
    wallH: r.wall_h,
    base,
    outdoor,
    baseGlow,
    farLit,
    sprites,
    lights,
    windows,
    walk,
    lift,
    door: { x: r.door, y: rows - 1 },
    spots: arr.spots.map((s) => ({ ...s })),
    hotspots: arr.hotspots.map((h) => ({ ...h })),
    picks,
    lamp: r.light.lamp,
    night: r.light.night,
  };
}

// ---------------------------------------------------------------------------
// Validation (the recipe's own rules; the test suite runs it on every option)

/** Every combination of a recipe's options. */
export function optionCombos(r: InteriorRecipe): Record<string, string>[] {
  let combos: Record<string, string>[] = [{}];
  for (const c of r.choices) combos = combos.flatMap((acc) => c.options.map((o) => ({ ...acc, [c.id]: o.id })));
  return combos;
}

/** Problems with a recipe (empty when it is valid), checked for every option combination and every alternative part. */
export function validateRecipe(r: InteriorRecipe): string[] {
  const errs: string[] = [];
  const inside = (x: number, y: number) => x >= 0 && y >= 0 && x < r.cols && y < r.rows;
  if (r.door < 0 || r.door >= r.cols) errs.push(`door column ${r.door} outside the room`);
  for (const combo of optionCombos(r)) {
    const tag = `[${Object.values(combo).join(",") || "base"}]`;
    const arr = arrangement(r, combo);
    // Place every piece with its first alternative for the grid; check every alternative's part exists and fits.
    const placed: PlacedPiece[] = [];
    const taken = new Map<number, string>();
    arr.pieces.forEach((p, i) => {
      const alts = typeof p.part === "string" ? [p.part] : p.part.map((w) => weightOf(w).value);
      for (const a of alts) {
        const def = PART_BY_ID.get(a);
        if (!def) {
          errs.push(`${tag} piece ${i}: unknown part "${a}"`);
          continue;
        }
        if (def.strip && (p.len ?? 1) < 1) errs.push(`${tag} ${a}: strip length ${p.len}`);
        const t = footprint(def, p);
        if (def.kind === "wall" && !p.front) {
          if (p.x < 0 || p.x >= r.cols) errs.push(`${tag} ${a}: wall column ${p.x} outside the room`);
          const slots = partSlots(a, p.len ?? 1);
          const top = p.floor ? r.wall_h - slots.h : (p.wy ?? 0) - Math.floor(slots.h / 2);
          if (top < 0 || top + slots.h > r.wall_h) errs.push(`${tag} ${a}: off the wall face (top ${top}, height ${slots.h}, wall ${r.wall_h})`);
        }
        if (def.kind === "top" && !inside(p.x, p.y ?? 0)) errs.push(`${tag} ${a}: at (${p.x}, ${p.y}) outside the room`);
        if (t) {
          if (!inside(t.x, t.y) || !inside(t.x + t.w - 1, t.y + t.h - 1)) errs.push(`${tag} ${a}: footprint (${t.x}, ${t.y}, ${t.w}×${t.h}) outside the room`);
          if (def.kind !== "decal" && a === alts[0]) {
            for (let y = t.y; y < t.y + t.h; y++) {
              for (let x = t.x; x < t.x + t.w; x++) {
                const k = y * r.cols + x;
                const other = taken.get(k);
                if (other) errs.push(`${tag} ${a} overlaps ${other} at (${x}, ${y})`);
                taken.set(k, a);
              }
            }
          }
        }
      }
      // For reachability, a piece blocks with its most blocking alternative.
      const defs = alts.map((a) => PART_BY_ID.get(a)).filter((d): d is PartDef => !!d);
      const def = defs.find((d) => d.kind === "furniture") ?? defs[0];
      if (def) placed.push({ def, piece: p, part: def.id, tiles: footprint(def, p) });
    });
    const walk = walkGrid(r.cols, r.rows, placed);
    const door = { x: r.door, y: r.rows - 1 };
    if (walk[door.y * r.cols + door.x] !== TILE.floor) errs.push(`${tag} the entrance tile is not floor`);
    const reach = reachable(r.cols, r.rows, walk, door);
    const check = (what: string, x: number, y: number) => {
      if (!inside(x, y)) {
        errs.push(`${tag} ${what} at (${x}, ${y}) outside the room`);
        return;
      }
      if (walk[y * r.cols + x] === TILE.blocked) errs.push(`${tag} ${what} at (${x}, ${y}) is not on walkable floor`);
      else if (!reach[y * r.cols + x]) errs.push(`${tag} ${what} at (${x}, ${y}) is not reachable from the door`);
    };
    const ids = new Set<string>();
    const tiles = new Map<number, string>();
    for (const s of arr.spots) {
      if (ids.has(s.id)) errs.push(`${tag} duplicate spot id ${s.id}`);
      ids.add(s.id);
      check(`spot ${s.id}`, s.x, s.y);
      const k = s.y * r.cols + s.x;
      if (tiles.has(k)) errs.push(`${tag} spots ${tiles.get(k)} and ${s.id} share (${s.x}, ${s.y})`);
      tiles.set(k, s.id);
      if (s.pose === "sit" && walk[k] !== TILE.seat) errs.push(`${tag} spot ${s.id} sits but (${s.x}, ${s.y}) has no seat`);
    }
    const hids = new Set<string>();
    for (const h of arr.hotspots) {
      if (hids.has(h.id)) errs.push(`${tag} duplicate hotspot id ${h.id}`);
      hids.add(h.id);
      if (!h.actions?.length && !h.services?.length) errs.push(`${tag} hotspot ${h.id} offers nothing`);
      if (!inside(h.x, h.y) || !inside(h.x + h.w - 1, h.y + h.h - 1)) errs.push(`${tag} hotspot ${h.id} outside the room`);
      check(`hotspot ${h.id}'s stand`, h.stand.x, h.stand.y);
      const touches = h.stand.x >= h.x - 1 && h.stand.x <= h.x + h.w && h.stand.y >= h.y - 1 && h.stand.y <= h.y + h.h;
      if (!touches) errs.push(`${tag} hotspot ${h.id}: its stand (${h.stand.x}, ${h.stand.y}) is not next to it`);
      const sk = h.stand.y * r.cols + h.stand.x;
      const spot = tiles.get(sk);
      // The player's own seat may be where the player stands to use a hotspot (the chamber's vote).
      const own = arr.spots.find((s) => s.id === spot)?.role === "player";
      if (spot && !own) errs.push(`${tag} hotspot ${h.id}'s stand is spot ${spot}'s tile`);
    }
  }
  return errs;
}

// ---------------------------------------------------------------------------
// Placeholder people (the game puts the engine's people on the spots)

export interface Actor {
  img: Indexed;
  x: number;
  y: number;
  key: number;
  /** Feet position for a blob shadow (standing people only). */
  feet: { x: number; y: number } | null;
}

/** The row where a seated figure is cut: the hips (legs are 4 + height rows and end on row 22; characterParts.ts). */
export const sitCut = (look: Look) => 19 - look.height;

/** A placeholder look for a role: a seeded townsperson, dressed for the job. */
export function placeholderLook(role: string, seed: number): Look {
  const l = lookFromSeed(seed);
  const suit = (accent: string): Look => ({ ...l, top: "suit", topRamp: ["cloth_navy", "cloth_charcoal", "wood", "cloth_denim"][seed % 4] as string, bottom: "trousers", bottomRamp: "", accentRamp: accent, hat: "none", carry: "none" });
  switch (role) {
    case "chair":
      return { ...suit("cloth_mustard"), bottomRamp: "cloth_charcoal" };
    case "member":
    case "player": {
      const s = suit(["cloth_red", "cloth_sky", "cloth_green", "cloth_mustard"][seed % 4] as string);
      return { ...s, bottomRamp: s.topRamp };
    }
    case "clerk":
    case "staff": {
      const s = suit("cloth_sky");
      return { ...s, bottomRamp: s.topRamp, carry: "lanyard" };
    }
    case "owner":
    case "cook":
    case "waiter":
      return { ...l, top: "apron", topRamp: role === "cook" ? "cloth_shirt" : (["cloth_shirt", "cloth_sky", "cloth_mustard"][seed % 3] as string), bottom: "trousers", bottomRamp: "cloth_charcoal", accentRamp: role === "owner" ? "cloth_red" : "cloth_navy", hat: role === "cook" ? "none" : l.hat === "scarf" ? "scarf" : "none", carry: "none" };
    default:
      return l;
  }
}

/** People on every spot (or the first `n` of each role), for review sheets. */
export function placeholderPeople(room: InteriorRoom, pal: Palette, lightness: readonly number[], seed: number, fill = 1): Actor[] {
  const actors: Actor[] = [];
  const { x: fx, y: fy } = room.floor;
  room.spots.forEach((s) => {
    if (hash32(seed, "fill", s.id) / 4294967296 >= fill) return;
    const look = placeholderLook(s.role, hash32(seed, s.id) % 997);
    const frames = composeCharacter(look, pal, lightness).frames;
    const frameOf = (f: Facing): Indexed => (f === "down" ? frames[0]?.[0] : f === "up" ? frames[1]?.[0] : f === "right" ? frames[2]?.[0] : faceLeft(frames[2]?.[0] as Indexed)) as Indexed;
    const img = frameOf(s.facing);
    const lift = room.lift[s.y * room.cols + s.x] ?? 0;
    const tileX = fx + s.x * 16;
    const tileY = fy + s.y * 16;
    if (s.pose === "sit") {
      const seat = room.sprites.find((sp) => sp.sitY && s.x >= sp.tiles.x && s.x < sp.tiles.x + sp.tiles.w && s.y >= sp.tiles.y && s.y < sp.tiles.y + sp.tiles.h);
      const hip = seat?.sitY?.[s.y - (seat?.tiles.y ?? s.y)] ?? tileY + 9 - lift;
      // Sideways sitters sit a little toward their seat's back.
      const shift = s.facing === "right" ? 1 : s.facing === "left" ? -1 : 0;
      const cut = sitCut(look);
      actors.push({ img: crop(img, 0, 0, 16, cut), x: tileX + shift, y: hip - cut, key: (tileY + 14) * 4, feet: null });
    } else {
      const feetY = tileY + 14 - lift;
      actors.push({ img, x: tileX, y: feetY - ANCHOR.y, key: (tileY + 14) * 4, feet: { x: tileX + ANCHOR.x, y: feetY - 1 } });
    }
  });
  return actors;
}

// ---------------------------------------------------------------------------
// The reference renderer (sheets; the web renderer does the same with canvas)

export interface RgbaImage {
  w: number;
  h: number;
  data: Uint8Array;
}

/** How lit the room's lamps are (0..1) for the day's `lights` level (a smooth step from dusk). */
export const lampLevel = (lights: number) => {
  const t = Math.min(1, Math.max(0, (lights - 0.08) / 0.4));
  return t * t * (3 - 2 * t);
};

const LAMP_TINT: Record<"warm" | "white", Rgb8> = { warm: [255, 186, 104], white: [255, 222, 164] };

/**
 * The light in a room at a time of day: everything the grade needs that does
 * not depend on what is drawn. `light` multiplies the albedo of every indoor
 * pixel (ambient, daylight under the windows, lamp and ceiling pools);
 * outdoor pixels (the view through the glass) take the street's own
 * palette-moods colour for the hour. Shared by `renderInterior` (the sheets)
 * and the game's canvas renderer (`gradeRoom`, `gradeImage`), so both grade
 * alike.
 */
export interface InteriorGrade {
  w: number;
  h: number;
  minutes: number;
  rgb: Rgb8[];
  /** Per pixel and channel. */
  light: Float32Array;
  /** How lit the lamps are (0..1). */
  lamp: number;
  mood: Rgb8[];
  spriteMul: number[];
  lampRgb: Rgb8;
  seed: number;
}

export function interiorGrade(room: InteriorRoom, pal: Palette, lighting: Lighting, minutes: number): InteriorGrade {
  const { w: W, h: H } = room;
  const rgb = pal.colors.map(hexToRgb);
  const n = lighting.entries.length;
  const pos = (((minutes % 1440) + 1440) % 1440) / lighting.stepMinutes;
  const e0 = lighting.entries[Math.floor(pos) % n] as Lighting["entries"][number];
  const e1 = lighting.entries[(Math.floor(pos) + 1) % n] as Lighting["entries"][number];
  const t = pos - Math.floor(pos);
  const mix = (a: number, b: number, k: number) => a + (b - a) * k;
  const mul = [0, 1, 2].map((c) => mix(e0.mul[c] as number, e1.mul[c] as number, t) / 255);
  const lights = mix(e0.lights, e1.lights, t);
  const lamp = lampLevel(lights);
  const tint = LAMP_TINT[room.lamp].map((v) => v / 255);
  // Ambient light indoors: a third of the street's grade by day. Once the lamps are on, the room is lit
  // by them, not by the sky: the night grade's brightness (the recipe's share of it) in the lamps' colour,
  // so the blue stays outside the windows.
  const nightL = [0, 1, 2].reduce((s, c) => s + mix(1, mul[c] as number, room.night) * ([0.3, 0.59, 0.11][c] as number), 0);
  const ambient = [0, 1, 2].map((c) => mix(mix(1, mul[c] as number, 0.3), nightL * mix(1, tint[c] as number, 0.45), lamp));
  const light = new Float32Array(W * H * 3);
  for (let i = 0; i < W * H; i++) for (let c = 0; c < 3; c++) light[i * 3 + c] = ambient[c] as number;
  const add = (x: number, y: number, c: readonly number[], a: number) => {
    if (x < 0 || y < 0 || x >= W || y >= H || a <= 0) return;
    const i = y * W + x;
    for (let k = 0; k < 3; k++) light[i * 3 + k] = (light[i * 3 + k] as number) + (c[k] as number) * a;
  };
  // Daylight falling in under the windows: two stepped bands, leaning with the hour.
  const sun = (1 - lights) * (minutes > 360 && minutes < 1260 ? 1 : 0);
  if (sun > 0) {
    const lean = Math.max(-1, Math.min(1, (780 - minutes) / 300));
    const fy = room.floor.y;
    const warm = [1, 0.96, 0.86].map((v, c) => v * (mul[c] as number));
    for (const win of room.windows) {
      for (let r = -2; r < 26; r++) {
        const off = Math.round(r * lean * 0.6);
        for (let x = win.x; x < win.x + win.w; x++) {
          const edge = r > 18 || x === win.x || x === win.x + win.w - 1;
          add(x + off, fy + r, warm, sun * (edge ? 0.07 : 0.14));
        }
      }
    }
  }
  // Lamp pools: banded ellipses (as the street's pools: stepped bands, the outer one dithered).
  for (const l of room.lights) {
    const level = l.glow === "always" ? 0.5 + 0.5 * lamp : lamp;
    if (level <= 0) continue;
    const bands = l.glow === "ceiling" ? [0.08, 0.12, 0.16] : l.glow === "always" ? [0.06, 0.1, 0.14] : [0.18, 0.32, 0.46];
    const c = l.glow === "always" ? [0.85, 0.92, 1] : tint;
    for (let y = -l.ry; y <= l.ry; y++) {
      for (let x = -l.rx; x <= l.rx; x++) {
        const d = (x / l.rx) ** 2 + (y / l.ry) ** 2;
        const b = d < 0.18 ? 2 : d < 0.55 ? 1 : d < 1 ? 0 : -1;
        if (b < 0) continue;
        if (b === 0 && l.glow !== "ceiling" && ((x + y) & 1) === 1) continue;
        add(l.x + x, l.y + y, c, (bands[b] as number) * level);
      }
    }
  }
  return {
    w: W,
    h: H,
    minutes,
    rgb,
    light,
    lamp,
    mood: (t < 0.5 ? e0 : e1).moodPalette.map(hexToRgb),
    spriteMul: (t < 0.5 ? e0 : e1).spriteMul.map((v) => v / 255),
    lampRgb: rgb[(rampByName(pal, "lamp").steps[2] ?? 1) as number] as Rgb8,
    seed: room.seed,
  };
}

const mixN = (a: number, b: number, k: number) => a + (b - a) * k;
const clamp8 = (v: number) => Math.round(Math.min(255, Math.max(0, v)));

/** An outdoor base pixel's colour (the view through the glass; far windows lit in clusters at night). */
function outdoorColour(room: InteriorRoom, g: InteriorGrade, i: number): [number, number, number] {
  const W = room.w;
  const v = room.base.px[i] as number;
  const lit = room.farLit[i] && g.lamp > 0.5 && hash32(room.seed, "far", Math.floor((i % W) / 3), Math.floor(i / W / 3)) % 3 !== 0;
  const c = lit ? g.lampRgb : (g.mood[v] as Rgb8);
  const k = lit ? [0.95, 0.88, 0.7] : g.spriteMul;
  return [c[0] * (k[0] as number), c[1] * (k[1] as number), c[2] * (k[2] as number)];
}

/** One indoor pixel, graded: the albedo under the light, then any glow over it. */
function indoorPixel(g: InteriorGrade, i: number, v: number, glow: number, k: number, out: Uint8Array | Uint8ClampedArray, o: number) {
  const c = g.rgb[v] as Rgb8;
  for (let ch = 0; ch < 3; ch++) {
    let px = (c[ch] as number) * Math.min(1.12, g.light[i * 3 + ch] as number);
    if (glow && k > 0) px = mixN(px, (g.rgb[glow] as Rgb8)[ch] as number, k);
    out[o + ch] = clamp8(px);
  }
  out[o + 3] = 255;
}

/**
 * An indexed image (a sprite, a person) placed at (x, y) in the room, graded
 * by the room's light where each pixel falls: RGBA, transparent where the
 * image is. Glowing pixels (`glow`, same frame) light by the lamps, or always.
 */
export function gradeImage(g: InteriorGrade, img: Indexed, x: number, y: number, glow: Indexed | null = null, always = false, out?: Uint8ClampedArray): Uint8ClampedArray {
  const o = out ?? new Uint8ClampedArray(img.w * img.h * 4);
  for (let yy = 0; yy < img.h; yy++) {
    for (let xx = 0; xx < img.w; xx++) {
      const j = yy * img.w + xx;
      const v = img.px[j] as number;
      if (!v) {
        o[j * 4 + 3] = 0;
        continue;
      }
      const X = Math.min(g.w - 1, Math.max(0, x + xx));
      const Y = Math.min(g.h - 1, Math.max(0, y + yy));
      const gl = glow?.px[j] ?? 0;
      indoorPixel(g, Y * g.w + X, v, gl, gl ? (always ? 1 : g.lamp) : 0, o, j * 4);
    }
  }
  return o;
}

/** The room's static layers graded for a time of day: the base, and each sprite (and the part of a seat over a sitter). */
export function gradeRoom(room: InteriorRoom, g: InteriorGrade): { base: RgbaImage; sprites: { img: Uint8ClampedArray; front: Uint8ClampedArray | null }[] } {
  const { w: W, h: H } = room;
  const data = new Uint8Array(W * H * 4);
  for (let i = 0; i < W * H; i++) {
    if (room.outdoor[i]) {
      const c = outdoorColour(room, g, i);
      for (let ch = 0; ch < 3; ch++) data[i * 4 + ch] = clamp8(c[ch] as number);
      data[i * 4 + 3] = 255;
    } else {
      const gl = room.baseGlow.px[i] as number;
      indoorPixel(g, i, room.base.px[i] as number, gl, gl ? g.lamp : 0, data, i * 4);
    }
  }
  const sprites = room.sprites.map((s) => {
    const always = s.glow === "always";
    const img = gradeImage(g, s.img, s.x, s.y, s.emissive, always);
    const front = s.front ? gradeImage(g, s.front.img, s.x, s.front.y, s.emissive ? crop(s.emissive, 0, s.front.y - s.y, s.emissive.w, s.front.img.h) : null, always) : null;
    return { img, front };
  });
  return { base: { w: W, h: H, data }, sprites };
}

/**
 * Render a room at a time of day (minutes after midnight), with actors, to RGBA.
 * Indoors: a softened share of the street's grade by day; by night the
 * recipe's share of it under the lamp tint, and the lamps' banded pools.
 * Through the windows: the street's own palette-moods colour for that time.
 */
export function renderInterior(room: InteriorRoom, pal: Palette, lighting: Lighting, minutes: number, actors: readonly Actor[] = []): RgbaImage {
  const { w: W, h: H } = room;
  const g = interiorGrade(room, pal, lighting, minutes);
  const { rgb, lamp, light } = g;

  // Albedo (indoor), final colour (outdoor); the light buffer multiplies the albedo.
  const alb = new Float32Array(W * H * 3);
  const fixed = new Uint8Array(W * H);
  const set = (i: number, c: readonly number[]) => {
    alb[i * 3] = c[0] as number;
    alb[i * 3 + 1] = c[1] as number;
    alb[i * 3 + 2] = c[2] as number;
  };
  for (let i = 0; i < W * H; i++) {
    if (room.outdoor[i]) {
      fixed[i] = 1;
      set(i, outdoorColour(room, g, i));
    } else set(i, rgb[room.base.px[i] as number] as Rgb8);
  }
  // Glow from the base (wall lights, signs) where nothing covers it.
  const glowAt = new Float32Array(W * H);
  const glowCol = new Uint8Array(W * H);
  for (let i = 0; i < W * H; i++) {
    const gl = room.baseGlow.px[i] as number;
    if (gl) {
      glowCol[i] = gl;
      glowAt[i] = lamp;
    }
  }
  // Sprites and actors by depth.
  type Layer = { img: Indexed; x: number; y: number; key: number; glow: Indexed | null; always: boolean };
  const layers: Layer[] = [];
  for (const s of room.sprites) {
    layers.push({ img: s.img, x: s.x, y: s.y, key: s.key, glow: s.emissive, always: s.glow === "always" });
    if (s.front) layers.push({ img: s.front.img, x: s.x, y: s.front.y, key: s.front.key, glow: s.emissive ? crop(s.emissive, 0, s.front.y - s.y, s.emissive.w, s.front.img.h) : null, always: s.glow === "always" });
  }
  for (const a of actors) layers.push({ img: a.img, x: a.x, y: a.y, key: a.key, glow: null, always: false });
  layers.sort((a, b) => a.key - b.key);
  // Blob shadows under standing people (on the floor, before anything is drawn over it).
  for (const a of actors) {
    if (!a.feet) continue;
    for (const [dx, dy] of BLOB) {
      const x = a.feet.x + dx;
      const y = a.feet.y + dy;
      if (x < 0 || y < 0 || x >= W || y >= H) continue;
      const i = y * W + x;
      for (let c = 0; c < 3; c++) alb[i * 3 + c] = (alb[i * 3 + c] as number) * 0.7;
    }
  }
  for (const l of layers) {
    for (let y = 0; y < l.img.h; y++) {
      for (let x = 0; x < l.img.w; x++) {
        const v = l.img.px[y * l.img.w + x] as number;
        if (!v) continue;
        const X = l.x + x;
        const Y = l.y + y;
        if (X < 0 || Y < 0 || X >= W || Y >= H) continue;
        const i = Y * W + X;
        set(i, rgb[v] as Rgb8);
        fixed[i] = 0;
        const gl = l.glow?.px[y * l.glow.w + x] ?? 0;
        glowAt[i] = gl ? (l.always ? 1 : lamp) : 0;
        glowCol[i] = gl;
      }
    }
  }
  const data = new Uint8Array(W * H * 4);
  for (let i = 0; i < W * H; i++) {
    const px = [0, 1, 2].map((c) => (fixed[i] ? (alb[i * 3 + c] as number) : (alb[i * 3 + c] as number) * Math.min(1.12, light[i * 3 + c] as number)));
    const k = glowAt[i] as number;
    if (k > 0) {
      const c = rgb[glowCol[i] as number] as Rgb8;
      for (let ch = 0; ch < 3; ch++) px[ch] = mixN(px[ch] as number, c[ch] as number, k);
    }
    for (let ch = 0; ch < 3; ch++) data[i * 4 + ch] = clamp8(px[ch] as number);
    data[i * 4 + 3] = 255;
  }
  return { w: W, h: H, data };
}

/** The blob shadow under a standing person's feet: offsets from the feet pixel (darkened to 70 %). */
export const BLOB: readonly (readonly [number, number])[] = (() => {
  const out: [number, number][] = [];
  for (let dy = -1; dy <= 1; dy++) for (let dx = -4; dx <= 4; dx++) if (dy === 0 || Math.abs(dx) <= 2) out.push([dx, dy]);
  return out;
})();

/** The room flat (no grading), palette indices: base, then sprites and actors by depth (goldens and determinism). */
export function flattenInterior(room: InteriorRoom, actors: readonly Actor[] = []): Indexed {
  const img = blank(room.w, room.h);
  img.px.set(room.base.px);
  const layers: { img: Indexed; x: number; y: number; key: number }[] = [];
  for (const s of room.sprites) {
    layers.push({ img: s.img, x: s.x, y: s.y, key: s.key });
    if (s.front) layers.push({ img: s.front.img, x: s.x, y: s.front.y, key: s.front.key });
  }
  for (const a of actors) layers.push(a);
  layers.sort((a, b) => a.key - b.key);
  for (const l of layers) blit(img, l.img, l.x, l.y);
  return img;
}

export { PARTS, SURFACES } from "./interiorsParts";
export { parseInteriorRecipes, recipeFor } from "./interiorsRecipes";
