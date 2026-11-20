// Works in street context (art round 8, DEC-099): the sample sites in the
// round 4 and round 5 test towns, and a small compositor that draws a window
// of a town with works on it (the ground through the terrain painter, the
// works' tints and decals on it, then buildings, props, works sprites and
// people in depth order, day shadows, and the palette-moods evening with lit
// windows, lamps and the sites' lit cabins). It follows src/cli/scene.ts
// (which approximates the hub renderer) and adds the works layers; the game
// draws with web/src/hub/render.
import { hexToRgb, type Rgb8 } from "../../src/core/color";
import { blank, type Indexed } from "../../src/core/image";
import { groundImage } from "../../src/cli/scene";
import { buildFacade } from "./buildings";
import { composeCharacter, lookFromSeed, type Look } from "./characters";
import { diagScene } from "./diagscene";
import type { ArtPack } from "./pack";
import { TERRAIN as K } from "./terrain";
import { v2Scene, type Scene } from "./v2scene";
import type { WorksKind } from "./works";
import type { ComposedWorks, Rect, WorksSite } from "./worksCompose";
import { pieceCentres as pieceCentresOf, type LinePiece } from "./worksLines";

export type Moment = "day" | "evening" | "night";
/** Lighting table entries (15-minute steps): 13:00, 20:00, 22:30. */
export const MOMENT_ENTRY: Readonly<Record<Moment, number>> = { day: 52, evening: 80, night: 90 };

/** A sample: a town (a copy of a test town, edited where the works stand), the site, the window to show (cells), what of the town the works replace. */
export interface WorksSample {
  kind: WorksKind;
  label: string;
  scene: Scene;
  site: WorksSite;
  window: Rect;
  seed: number;
}

const groundCache = new WeakMap<object, Indexed>();
function ground(pack: ArtPack, s: Scene): Indexed {
  let g = groundCache.get(s);
  if (!g) groundCache.set(s, (g = groundImage(pack, s)));
  return g;
}

interface Item {
  img: Indexed;
  x: number;
  y: number;
  base: number;
  person?: boolean;
  lit?: { img: Indexed; x: number; y: number }[];
  pool?: [number, number];
  shadows?: { box: readonly [number, number, number, number]; height: number; ox: number; oy: number }[];
}

const inRect = (r: Rect, x: number, y: number) => x >= r.x && y >= r.y && x < r.x + r.w && y < r.y + r.h;

/** Draw a window of a town with works on it, at a moment, an integer scale and an animation frame. */
export function renderWorks(pack: ArtPack, sample: WorksSample, works: ComposedWorks | null, moment: Moment, scale: number, frame = 0): { w: number; h: number; data: Uint8Array } {
  const pal = pack.palette;
  const s = sample.scene;
  const win = sample.window;
  const X0 = win.x * 16;
  const Y0 = win.y * 16;
  const W = win.w * 16;
  const H = win.h * 16;
  const full = ground(pack, s);
  const g = blank(W, H);
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) g.px[y * W + x] = full.px[(Y0 + y) * full.w + X0 + x] as number;
  if (works) {
    for (const t of works.tints) {
      for (let y = 0; y < t.h; y++) for (let x = 0; x < t.w; x++) {
        if (!t.mask[y * t.w + x]) continue;
        const X = t.x + x - X0;
        const Y = t.y + y - Y0;
        if (X < 0 || Y < 0 || X >= W || Y >= H) continue;
        g.px[Y * W + X] = t.table[g.px[Y * W + X] as number] as number;
      }
    }
    for (const d of works.decals) {
      for (let y = 0; y < d.img.h; y++) for (let x = 0; x < d.img.w; x++) {
        const v = d.img.px[y * d.img.w + x] as number;
        if (!v) continue;
        const X = d.x + x - X0;
        const Y = d.y + y - Y0;
        if (X < 0 || Y < 0 || X >= W || Y >= H) continue;
        if (d.onlyOn && !d.onlyOn.has(g.px[Y * W + X] as number)) continue;
        g.px[Y * W + X] = v;
      }
    }
  }
  const items: Item[] = [];
  const replaced = works?.replaces ?? null;
  for (const b of s.buildings) {
    if (replaced && inRect(replaced, b.x, b.y)) continue;
    const f = buildFacade(b.spec, pal);
    const x = b.x * 16;
    const y = b.y * 16 - f.ay;
    items.push({
      img: f.albedo,
      x,
      y,
      base: (b.y + b.spec.h) * 16,
      lit: [...f.windows, ...f.doors].filter((o) => o.emissive).map((o) => ({ img: o.emissive as Indexed, x: x + o.x, y: y + o.y })),
      shadows: f.shadows.map((c) => {
        const xs = c.footprint.map((p) => p[0]);
        const ys = c.footprint.map((p) => p[1]);
        return { box: [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)] as const, height: c.height, ox: b.x * 16, oy: b.y * 16 };
      }),
    });
  }
  for (const p of s.props) {
    const list = pack.props[p.kind];
    const sp = list?.[p.variant % (list?.length || 1)];
    if (!sp) continue;
    const x = p.x * 16 - sp.ax;
    const y = p.y * 16 - sp.ay;
    const it: Item = { img: sp.albedo, x, y, base: p.y * 16 + Math.max(16, sp.albedo.h - sp.ay) };
    if (sp.emissive) {
      it.lit = [{ img: sp.emissive, x, y }];
      it.pool = [p.x * 16 + 8, p.y * 16 + 14];
    }
    if (sp.shadow) {
      const xs = sp.shadow.footprint.map((q) => q[0]);
      const ys = sp.shadow.footprint.map((q) => q[1]);
      it.shadows = [{ box: [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)], height: sp.shadow.height, ox: x, oy: y }];
    }
    items.push(it);
  }
  const person = (look: Look, fx: number, fy: number) => {
    const c = composeCharacter(look, pal, pack.lightness);
    items.push({ img: c.frames[0]?.[frame % 2] as Indexed, x: fx - 7, y: fy - 23, base: fy, person: true });
  };
  for (const p of s.people) person(lookFromSeed(p.seed), p.x * 16 + 8, p.y * 16 + 13);
  if (works) {
    for (const sp of works.sprites) {
      const k = frame % sp.frames.length;
      const it: Item = { img: sp.frames[k] as Indexed, x: sp.x, y: sp.y, base: sp.base };
      const e = sp.emissive[k] ?? null;
      if (e) it.lit = [{ img: e, x: sp.x, y: sp.y }];
      if (sp.shadow) it.shadows = [{ box: sp.shadow.box, height: sp.shadow.height, ox: sp.shadow.ox, oy: sp.shadow.oy }];
      items.push(it);
    }
    for (const p of works.people) person(p.look, p.x, p.y);
  }
  items.sort((a, b) => a.base - b.base || a.x - b.x);

  const base = pal.colors.map(hexToRgb);
  const entry = pack.lighting.entries[MOMENT_ENTRY[moment]];
  const dark = moment !== "day" && entry;
  const mul = dark ? entry.spriteMul : ([255, 255, 255] as const);
  const mood = dark ? entry.moodPalette.map(hexToRgb) : base;
  const statics: Rgb8[] = mood.map((c) => [(c[0] * mul[0]) / 255, (c[1] * mul[1]) / 255, (c[2] * mul[2]) / 255]);
  const people: Rgb8[] = base.map((c) => [(c[0] * mul[0]) / 255, (c[1] * mul[1]) / 255, (c[2] * mul[2]) / 255]);
  const rgb = new Float32Array(W * H * 3);
  const put = (img: Indexed, x0: number, y0: number, table: readonly Rgb8[]) => {
    for (let y = 0; y < img.h; y++) {
      const Y = y0 + y - Y0;
      if (Y < 0 || Y >= H) continue;
      for (let x = 0; x < img.w; x++) {
        const X = x0 + x - X0;
        const v = img.px[y * img.w + x] as number;
        if (!v || X < 0 || X >= W) continue;
        const c = table[v] as Rgb8;
        const i = (Y * W + X) * 3;
        rgb[i] = c[0];
        rgb[i + 1] = c[1];
        rgb[i + 2] = c[2];
      }
    }
  };
  put(g, X0, Y0, statics);
  // Day shadows: each caster's footprint swept along the sun, one shade.
  if (moment === "day") {
    const mask = new Uint8Array(W * H);
    const sun = [0.42, 0.24] as const;
    for (const it of items) {
      for (const c of it.shadows ?? []) {
        const [bx0, by0, bx1, by1] = c.box;
        const steps = Math.ceil(c.height);
        for (let k = 0; k <= steps; k += 2) {
          const ox = Math.round((sun[0] * c.height * k) / Math.max(1, steps) / 2) * 2;
          const oy = Math.round((sun[1] * c.height * k) / Math.max(1, steps) / 2) * 2;
          for (let y = c.oy + by0 + oy; y < c.oy + by1 + oy; y++) for (let x = c.ox + bx0 + ox; x < c.ox + bx1 + ox; x++) {
            const X = x - X0;
            const Y = y - Y0;
            if (X >= 0 && Y >= 0 && X < W && Y < H) mask[Y * W + X] = 1;
          }
        }
      }
    }
    const sc = hexToRgb(pack.lighting.shadowColor);
    for (let i = 0; i < W * H; i++) if (mask[i]) for (let k = 0; k < 3; k++) rgb[i * 3 + k] = (rgb[i * 3 + k] as number) * 0.66 + (sc[k] as number) * 0.34;
  }
  if (dark) {
    const pool = pack.pool;
    for (const it of items) {
      if (!it.pool) continue;
      const [cx, cy] = it.pool;
      for (let y = 0; y < pool.h; y++) for (let x = 0; x < pool.w; x++) {
        const band = pool.bands[y * pool.w + x] as number;
        const c = pool.colours[band];
        if (!band || !c || (band === 1 && (x + y) % 2)) continue;
        const X = cx - pool.w / 2 + x - X0;
        const Y = cy - pool.h / 2 + y - Y0;
        if (X < 0 || Y < 0 || X >= W || Y >= H) continue;
        const i = (Y * W + X) * 3;
        for (let k = 0; k < 3; k++) rgb[i + k] = Math.min(255, (rgb[i + k] as number) + (c[k] as number) * c[3] * entry.lights);
      }
    }
  }
  for (const it of items) put(it.img, it.x, it.y, it.person ? people : statics);
  if (dark) for (const it of items) for (const l of it.lit ?? []) put(l.img, l.x, l.y, base);
  const data = new Uint8Array(W * scale * H * scale * 4);
  for (let y = 0; y < H * scale; y++) for (let x = 0; x < W * scale; x++) {
    const i = (Math.floor(y / scale) * W + Math.floor(x / scale)) * 3;
    const o = (y * W * scale + x) * 4;
    data[o] = Math.round(rgb[i] as number);
    data[o + 1] = Math.round(rgb[i + 1] as number);
    data[o + 2] = Math.round(rgb[i + 2] as number);
    data[o + 3] = 255;
  }
  return { w: W * scale, h: H * scale, data };
}

// ---------------------------------------------------------------------------
// The sample sites

function editV2(edit: (s: Scene) => void): Scene {
  const s = v2Scene();
  edit(s);
  return s;
}

const without = (s: Scene, pred: (b: { x: number; y: number }) => boolean) => {
  s.buildings = s.buildings.filter((b) => !pred(b));
};

/** A 45° run down one side of a "\" street: per row, the pieces north to east and west to south (the street grid's own line). */
export function diagonalRun(cells: readonly (readonly [number, number])[]): LinePiece[] {
  return cells.flatMap(([x, y]) => [
    { x, y, a: "n", b: "e" } as LinePiece,
    { x: x + 1, y, a: "w", b: "s" } as LinePiece,
  ]);
}

export function worksSamples(): WorksSample[] {
  const out: WorksSample[] = [];

  // The Old Bridge (round 4's stone bridge, east–west, three spans): the yard on the quay, the board by the bridge's west end.
  const bridgeScene = editV2((s) => {
    s.people = s.people.filter((p) => !(p.y === 15 && p.x >= 25 && p.x <= 29));
  });
  out.push({
    kind: "bridge",
    label: "the Old Bridge",
    scene: bridgeScene,
    site: { connector: { kind: "bridge", x: 26, y: 15, w: 3, h: 1, axis: "ew" }, yard: { x: 21, y: 16, w: 4, h: 1 }, front: [20, 14], entrance: [25, 16] },
    window: { x: 20, y: 11, w: 13, h: 8 },
    seed: 0x0b1d,
  });

  // A flood wall along the quay of the round 5 town, turning inland at 45° up the far side of a "\" street.
  const wallScene = diagScene();
  const quay: LinePiece[] = [
    { x: 55, y: 31, a: "e", b: "w" },
    { x: 54, y: 31, a: "e", b: "w" },
    { x: 53, y: 31, a: "e", b: "n" },
    { x: 53, y: 30, a: "s", b: "w" },
    { x: 52, y: 30, a: "e", b: "n" },
    { x: 52, y: 29, a: "s", b: "w" },
    { x: 51, y: 29, a: "e", b: "n" },
    { x: 51, y: 28, a: "s", b: "w" },
    { x: 50, y: 28, a: "e", b: "n" },
    { x: 50, y: 27, a: "s", b: "c" },
  ];
  wallScene.props = wallScene.props.filter((p) => !(p.x >= 50 && p.y >= 26 && p.y <= 31));
  out.push({
    kind: "flood_wall",
    label: "the quay wall",
    scene: wallScene,
    site: { pieces: quay, yard: { x: 52, y: 26, w: 4, h: 2 }, front: [54, 28], entrance: [55, 29] },
    window: { x: 44, y: 23, w: 12, h: 11 },
    seed: 0xf100d,
  });

  // A levee on the north bank of the round 4 river, west of the creek, stepping up at 45° to the path.
  const leveeScene = editV2((s) => {
    s.props = s.props.filter((p) => p.kind !== "reeds" || p.y !== 17);
    s.people = s.people.filter((p) => p.y < 12 || p.y > 18 || p.x > 12);
  });
  out.push({
    kind: "levee",
    label: "the river levee",
    scene: leveeScene,
    site: {
      pieces: [
        { x: 0, y: 16, a: "w", b: "e" },
        { x: 1, y: 16, a: "w", b: "e" },
        { x: 2, y: 16, a: "w", b: "e" },
        { x: 3, y: 16, a: "w", b: "e" },
        { x: 4, y: 16, a: "w", b: "n" },
        { x: 4, y: 15, a: "s", b: "e" },
        { x: 5, y: 15, a: "w", b: "e" },
        { x: 6, y: 15, a: "w", b: "c" },
      ],
      yard: { x: 0, y: 12, w: 4, h: 2 },
      front: [5, 13],
      entrance: [6, 13],
    },
    window: { x: 0, y: 10, w: 12, h: 9 },
    seed: 0x1e7ee,
  });

  // A clinic in the slot of round 4's reserved plot; the terrace site beside it is the works yard.
  const civicScene = editV2((s) => {
    without(s, (b) => b.y === 22 && (b.x === 25 || b.x === 29));
    for (let y = 22; y < 25; y++) for (let x = 25; x < 29; x++) s.terrain[y * s.width + x] = K.Yard;
  });
  const civicSpec = { kind: "clinic", hood: "core", w: 3, h: 3, doorX: 1, storeys: 2, visualSeed: 8801, named: true, home: false, landmark: false, conditions: [] as string[], headroom: 16, ends: { left: "gap" as const, right: "joined" as const } };
  out.push({
    kind: "civic",
    label: "the new clinic",
    scene: civicScene,
    site: { slot: { x: 29, y: 22, w: 3, h: 3, doorX: 1 }, building: civicSpec, yard: { x: 25, y: 22, w: 4, h: 3 }, front: [27, 25], entrance: [31, 25] },
    window: { x: 23, y: 14, w: 12, h: 13 },
    seed: 0xc11c,
  });

  // Housing infill on round 4's construction plot; the reserved plot beside it is the yard.
  const housingScene = editV2((s) => {
    without(s, (b) => b.y === 22 && (b.x === 25 || b.x === 29));
    for (let y = 22; y < 25; y++) for (let x = 29; x < 32; x++) s.terrain[y * s.width + x] = K.Yard;
  });
  out.push({
    kind: "housing",
    label: "infill terrace",
    scene: housingScene,
    site: { slot: { x: 25, y: 22, w: 4, h: 3, doorX: 1 }, building: { kind: "terrace", hood: "inner", w: 4, h: 3, doorX: 1, storeys: 2, visualSeed: 4108, named: false, home: false, landmark: false, conditions: [], headroom: 16, ends: { left: "joined", right: "gap" } }, yard: { x: 29, y: 22, w: 3, h: 3 }, front: [30, 25], entrance: [26, 25] },
    window: { x: 23, y: 18, w: 11, h: 9 },
    seed: 0x4005e,
  });

  // Road works on round 4's southern road: the south lane closed between the signs.
  const roadScene = editV2((s) => {
    s.people = s.people.filter((p) => p.y < 25);
  });
  const roadCells: [number, number][] = [];
  for (let x = 17; x < 29; x++) for (const y of [26, 27]) roadCells.push([x, y]);
  const closure = [];
  for (let X = 18 * 16; X < 28 * 16; X++) closure.push({ X, Y: 27 * 16, dir: 0 as const, piece: X >> 4 });
  out.push({
    kind: "road",
    label: "the south road",
    scene: roadScene,
    site: { cells: roadCells, lane: { nx: 0, ny: 1, d: 27 * 16 + 1 }, closure, signs: [[17 * 16 + 4, 27 * 16 + 13], [28 * 16 + 10, 27 * 16 + 13]], along: "x", yard: { x: 20, y: 25, w: 4, h: 1 }, front: [20, 25], entrance: [20, 25] },
    window: { x: 15, y: 21, w: 15, h: 7 },
    seed: 0x60ad,
  });

  // Road works on a 45° street of the round 5 town: the west lane of the "\" street closed along its centre line.
  const diagRoad = diagScene();
  const dcells: [number, number][] = [];
  const dpieces: LinePiece[] = [];
  for (let r = 23; r <= 28; r++) {
    const xs = 41 + (r - 22);
    dcells.push([xs, r], [xs + 1, r]);
    dpieces.push({ x: xs, y: r, a: "n", b: "e" }, { x: xs + 1, y: r, a: "w", b: "s" });
  }
  diagRoad.people = diagRoad.people.filter((p) => !(p.x >= 40 && p.y >= 22));
  out.push({
    kind: "road",
    label: "the diagonal street",
    scene: diagRoad,
    // The centre line runs through (xs + 1) · 16 at each row's middle, where X − Y = 312; the west lane is where X − Y is less.
    site: { cells: dcells, lane: { nx: -1, ny: 1, d: -(19 * 16 + 8) + 1 }, closure: dpieces.flatMap((p, i) => pieceCentresOf(p, i)), signs: [[42 * 16 + 2, 23 * 16 + 4], [48 * 16 + 4, 29 * 16 + 2]], along: "x", yard: { x: 37, y: 27, w: 3, h: 2 }, front: [38, 27], entrance: [38, 27] },
    window: { x: 38, y: 21, w: 14, h: 10 },
    seed: 0xd1a6,
  });

  // The mine on round 4's upland: the headframe's cell, the heap's, the yard by the pit.
  const mineScene = editV2((s) => {
    s.props = s.props.filter((p) => !(p.kind === "headframe" || (p.kind === "spoil_heap" && p.x >= 30)));
  });
  out.push({
    kind: "mine",
    label: "the pit",
    scene: mineScene,
    site: { headframe: [37, 3], heap: [30, 7], yard: { x: 31, y: 3, w: 4, h: 2 }, front: [35, 6], entrance: [36, 6] },
    window: { x: 27, y: 0, w: 13, h: 10 },
    seed: 0x313e,
  });

  // A works on round 4's warehouse plot, its yard on the grass beside it, gates onto the south pavement.
  const factoryScene = editV2((s) => {
    without(s, (b) => b.x === 9 && b.y === 22);
    for (let y = 22; y < 25; y++) for (let x = 5; x < 9; x++) s.terrain[y * s.width + x] = K.Yard;
  });
  out.push({
    kind: "factory",
    label: "the works",
    scene: factoryScene,
    site: { slot: { x: 9, y: 22, w: 4, h: 3, doorX: 2 }, building: { kind: "factory", hood: "industrial", w: 4, h: 3, doorX: 2, storeys: 2, visualSeed: 4114, named: false, home: false, landmark: false, conditions: [], headroom: 16, ends: { left: "gap", right: "gap" } }, yard: { x: 5, y: 22, w: 4, h: 3 }, gate: [6, 24], chimney: [12, 22], front: [5, 25], entrance: [8, 25] },
    window: { x: 3, y: 14, w: 12, h: 13 },
    seed: 0xfac7,
  });
  return out;
}

