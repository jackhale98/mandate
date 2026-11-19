// Street geometry for layout v2 (DEC-090 D5 amended: grown streets plus 45°):
// straight runs in eight directions, the band of cells each covers (a 45°
// run is a staircase of cells the art draws smooth), painting them with
// their kerbs, and the `StreetAxis` code of every street cell. Shared by
// the planner's stages and the checks. Integer maths only.
import { StreetAxis, Terrain, type StreetEndKind, type TownStreetEnd } from "../townLayout";
import { RoadClass, Use, WaterKind, isWaterBody, type PlanConnector, type SegAxis, type Segment, type Work } from "./work";

/** The eight directions, counter-clockwise from east (screen axes: y grows south). */
export const DIR8: readonly (readonly [number, number])[] = [
  [1, 0],
  [1, -1],
  [0, -1],
  [-1, -1],
  [-1, 0],
  [-1, 1],
  [0, 1],
  [1, 1],
];

export const isDiagDir = (d: number) => (d & 1) === 1;

/** The run axis of a direction (either way along it). */
export function axisOfDir(d: number): SegAxis {
  switch (d & 3) {
    case 0:
      return "h";
    case 1:
      return "ne";
    case 2:
      return "v";
    default:
      return "nw";
  }
}

/** Steps from the a end to the b end of a run along its axis. */
export function stepOf(axis: SegAxis): readonly [number, number] {
  return axis === "h" ? [1, 0] : axis === "v" ? [0, 1] : axis === "ne" ? [1, -1] : [1, 1];
}

/** Offset of the first band cell from the centre line. */
export const bandLo = (width: number) => (width - 1) >> 1;

/** A run's length in steps (cells along the centre line, minus one). */
export const segSteps = (s: Segment) => (s.axis === "v" ? s.by - s.ay : s.bx - s.ax);

/** A run from (x, y) going `steps` along direction `dir`, normalised so a ≤ b. */
export function makeSeg(x: number, y: number, dir: number, steps: number, width: number, base: Pick<Segment, "cls" | "style" | "district" | "arterial"> & { end?: StreetEndKind | null }): Segment {
  const [dx, dy] = DIR8[dir & 7] as readonly [number, number];
  const ex = x + dx * steps;
  const ey = y + dy * steps;
  const axis = axisOfDir(dir);
  const flip = axis === "v" ? ey < y : ex < x;
  const ax = flip ? ex : x;
  const ay = flip ? ey : y;
  const bx = flip ? x : ex;
  const by = flip ? y : ey;
  // An end asked for at the far end (in travel order) moves with the flip.
  const far = base.end ?? null;
  const ends: Segment["ends"] = flip ? [far, null] : [null, far];
  return withBox({ axis, ax, ay, bx, by, width, x0: 0, y0: 0, x1: 0, y1: 0, cls: base.cls, style: base.style, district: base.district, arterial: base.arterial, ends });
}

/** Set a run's bounding box from its centre line and width. */
export function withBox(s: Segment): Segment {
  const lo = bandLo(s.width);
  const hi = s.width - 1 - lo;
  if (s.axis === "h") {
    s.x0 = s.ax;
    s.x1 = s.bx;
    s.y0 = s.ay - lo;
    s.y1 = s.ay + hi;
  } else if (s.axis === "v") {
    s.y0 = s.ay;
    s.y1 = s.by;
    s.x0 = s.ax - lo;
    s.x1 = s.ax + hi;
  } else {
    s.x0 = s.ax - lo;
    s.x1 = s.bx + hi;
    s.y0 = Math.min(s.ay, s.by);
    s.y1 = Math.max(s.ay, s.by);
  }
  return s;
}

/** An orthogonal run from its band box (the arterials' coarse cells, the square's ring). */
export function boxSeg(axis: "h" | "v", x0: number, y0: number, x1: number, y1: number, base: Pick<Segment, "cls" | "style" | "district" | "arterial">): Segment {
  const width = axis === "h" ? y1 - y0 + 1 : x1 - x0 + 1;
  const lo = bandLo(width);
  const s: Segment = axis === "h" ? { axis, ax: x0, ay: y0 + lo, bx: x1, by: y0 + lo, width, x0, y0, x1, y1, ...base, ends: [null, null] } : { axis, ax: x0 + lo, ay: y0, bx: x0 + lo, by: y1, width, x0, y0, x1, y1, ...base, ends: [null, null] };
  return s;
}

/**
 * Visit the band cells of a run at positions `t0..t1` along it (0 is the a
 * end; positions past either end are allowed: the cells just beyond it).
 * `f(x, y, k, t)`: k is the index across the band (top to bottom for "h",
 * left to right otherwise). Cells off the map are visited too; callers check.
 */
export function forBand(s: Segment, t0: number, t1: number, f: (x: number, y: number, k: number, t: number) => void) {
  const [sx, sy] = stepOf(s.axis);
  const lo = bandLo(s.width);
  for (let t = t0; t <= t1; t++) {
    const cx = s.ax + sx * t;
    const cy = s.ay + sy * t;
    for (let k = 0; k < s.width; k++) {
      if (s.axis === "h") f(cx, cy - lo + k, k, t);
      else f(cx - lo + k, cy, k, t);
    }
  }
}

/** Every cell of a run's band (on the map). */
export function segCells(s: Segment, W: number, H: number, f: (c: number, k: number, t: number) => void) {
  forBand(s, 0, segSteps(s), (x, y, k, t) => {
    if (x >= 0 && y >= 0 && x < W && y < H) f(y * W + x, k, t);
  });
}

/** Terrain across a band (index k), by width, style and whether it runs at 45°. */
export function profileOf(width: number, style: Segment["style"], diagonal: boolean): number[] {
  if (style === "path") return Array.from({ length: width }, () => Terrain.Path);
  if (style === "lane") return Array.from({ length: width }, () => Terrain.Pavement);
  if (style === "dirt") return Array.from({ length: width }, (_, k) => (width >= 4 && (k === 0 || k === width - 1) ? Terrain.Grass : Terrain.Dirt));
  if (diagonal) return Array.from({ length: width }, (_, k) => (width >= 3 && (k === 0 || k === width - 1) ? Terrain.Pavement : Terrain.Road));
  if (width >= 4) return Array.from({ length: width }, (_, k) => (k === 0 || k === width - 1 ? Terrain.Pavement : Terrain.Road));
  if (width === 3) return [Terrain.Pavement, Terrain.Road, Terrain.Road];
  if (width === 2) return [Terrain.Pavement, Terrain.Pavement];
  return [Terrain.Path];
}

const RANK: Record<number, number> = { [Terrain.Grass]: 0, [Terrain.Path]: 1, [Terrain.Dirt]: 2, [Terrain.Pavement]: 3, [Terrain.Road]: 4 };

/**
 * Paint a street run. Cells on a river, lake or the sea are skipped (bridges
 * are painted by `bridgeRuns`); a creek becomes a bridge deck (or a ford for
 * lanes); a road never turns back into pavement; the railway becomes a level
 * crossing (returned, for its connector) when the run may cross it, and is
 * left alone otherwise. The cells take the level of the centre line.
 */
export function paintSegment(w: Work, s: Segment, crossRail = false): number[] {
  const { W, H } = w;
  const diag = s.axis === "ne" || s.axis === "nw";
  const prof = profileOf(s.width, s.style, diag);
  const lanes = s.style === "path" || s.style === "dirt" || s.style === "lane";
  const lo = bandLo(s.width);
  const crossings: number[] = [];
  let lv = 0;
  forBand(s, 0, segSteps(s), (x, y, k) => {
    if (k === 0) {
      // The level of the centre line here.
      const mx = s.axis === "h" ? x : x + lo;
      const my = s.axis === "h" ? y + lo : y;
      if (mx >= 0 && my >= 0 && mx < W && my < H) lv = w.level[my * W + mx] as number;
    }
    if (x < 0 || y < 0 || x >= W || y >= H) return;
    const c = y * W + x;
    if (isWaterBody(w, c) || (w.use[c] as number) & Use.Square) return;
    const t = prof[k] as number;
    const cur = w.terrain[c] as number;
    if ((w.use[c] as number) & Use.Rail) {
      if (!crossRail || cur === Terrain.Bridge) return;
      if (cur !== Terrain.Crossing) crossings.push(c);
      w.terrain[c] = Terrain.Crossing;
      w.use[c] = (w.use[c] as number) | Use.Road;
      if (!w.roadClass[c] || s.cls < (w.roadClass[c] as number)) w.roadClass[c] = s.cls;
      return;
    }
    if (w.water[c] === WaterKind.Creek) {
      w.terrain[c] = lanes ? Terrain.Ford : Terrain.Bridge;
      w.use[c] = (w.use[c] as number) | Use.Road | Use.Bridge;
      if (!w.roadClass[c] || s.cls < (w.roadClass[c] as number)) w.roadClass[c] = s.cls;
      return;
    }
    if (cur === Terrain.Bridge || cur === Terrain.Crossing || cur === Terrain.Plaza || cur === Terrain.Ford) return;
    const curRoad = (w.use[c] as number) & Use.Road;
    if (curRoad && (RANK[cur] ?? 0) >= (RANK[t] ?? 0)) {
      if (s.cls < (w.roadClass[c] as number)) w.roadClass[c] = s.cls;
      return;
    }
    w.terrain[c] = t;
    w.level[c] = lv;
    w.use[c] = (w.use[c] as number) | Use.Road;
    if (!w.roadClass[c] || s.cls < (w.roadClass[c] as number)) w.roadClass[c] = s.cls;
  });
  return crossings;
}

/** A level crossing connector over the cells a street crossed the railway on. */
export function levelCrossing(w: Work, s: Segment, cells: number[]): PlanConnector | null {
  if (!cells.length) return null;
  const { W } = w;
  let x0 = W;
  let y0 = w.H;
  let x1 = -1;
  let y1 = -1;
  for (const c of cells) {
    const x = c % W;
    const y = (c - x) / W;
    x0 = Math.min(x0, x);
    y0 = Math.min(y0, y);
    x1 = Math.max(x1, x);
    y1 = Math.max(y1, y);
  }
  const id = w.recipe.connectors.find((k) => k.kind === "level_crossing")?.id ?? "connector.level_crossing";
  w.used.connectors.add(id);
  return { kind: "level_crossing", type: id, x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1, axis: s.axis === "v" ? "ns" : "ew", name: "", crosses: "rail", arterial: s.arterial };
}

/** Terminations that widen the road (a turning head is part of the street). */
export const isHead = (k: StreetEndKind | null | undefined) => k === "turning_circle" || k === "hammerhead";

/** The cell at position `t` along a run and `k` across its band (k may lie outside the band, either side). */
export function cellAt(s: Segment, t: number, k: number): [number, number] {
  const [sx, sy] = stepOf(s.axis);
  const lo = bandLo(s.width);
  const cx = s.ax + sx * t;
  const cy = s.ay + sy * t;
  return s.axis === "h" ? [cx, cy - lo + k] : [cx - lo + k, cy];
}

/** The carriageway across a run's band: the first and last index of its road (or track) cells (the whole band for a lane or path). */
export function carriageway(s: Segment): [number, number] {
  const prof = profileOf(s.width, s.style, s.axis === "ne" || s.axis === "nw");
  let a = -1;
  let b = -1;
  prof.forEach((t, k) => {
    if (t === Terrain.Road || t === Terrain.Dirt) {
      if (a < 0) a = k;
      b = k;
    }
  });
  return a < 0 ? [0, s.width - 1] : [a, b];
}

/**
 * A turning head at a run's end (pixel-art round 6's contract): a turning
 * circle is a block of (r + 2)² road cells beyond the street's last cells,
 * r its carriageway's width, centred on it (the art rounds it into a bulb);
 * a hammerhead is a bar two cells wider each side than the carriageway, as
 * deep as it is wide (at least two). A ring of pavement goes round either.
 * `road`: the head's road cells; `all`: with its pavement.
 */
export function headShape(s: Segment, end: 0 | 1, W: number, H: number): { road: number[]; all: number[] } {
  const kind = s.ends[end];
  const [k0, k1] = carriageway(s);
  const r = k1 - k0 + 1;
  const L = segSteps(s);
  const out = end === 0 ? -1 : 1;
  const t0 = end === 0 ? 0 : L;
  let depth: number;
  let a0: number;
  let a1: number;
  if (kind === "hammerhead") {
    depth = Math.max(2, r - 1);
    a0 = k0 - 2;
    a1 = k1 + 2;
  } else {
    depth = r + 2;
    a0 = k0 - 1;
    a1 = k1 + 1;
  }
  const road: number[] = [];
  const all: number[] = [];
  for (let d = 0; d <= depth + 1; d++) {
    for (let k = a0 - 1; k <= a1 + 1; k++) {
      const [x, y] = cellAt(s, t0 + out * d, k);
      if (x < 0 || y < 0 || x >= W || y >= H) continue;
      const c = y * W + x;
      const isRoad = d >= 1 && d <= depth && k >= a0 && k <= a1;
      // The street's own last cells stay the street's (the ring meets its pavements).
      if (d === 0 && k >= 0 && k < s.width) continue;
      if (isRoad) road.push(c);
      all.push(c);
    }
  }
  return { road, all };
}

/** Every cell of a turning head (its road and its pavement). */
export function headCells(s: Segment, end: 0 | 1, W: number, H: number): number[] {
  return headShape(s, end, W, H).all;
}

/** Paint a turning head: its road (a track's earth for country lanes) inside a ring of pavement. Returns false where it cannot fit. */
export function paintHead(w: Work, s: Segment, end: 0 | 1, dry = false): boolean {
  const { W, H } = w;
  const { road, all } = headShape(s, end, W, H);
  if (!road.length) return false;
  for (const c of all) {
    const x = c % W;
    const y = (c - x) / W;
    if (isWaterBody(w, c) || (w.use[c] as number) & (Use.Rail | Use.Square | Use.Plot) || w.terrain[c] === Terrain.Rock || x < 2 || y < 2 || x >= W - 2 || y >= H - 2) return false;
  }
  if (dry) return true;
  const roadSet = new Set(road);
  const [lx, ly] = cellAt(s, end === 0 ? 0 : segSteps(s), bandLo(s.width));
  const lv = w.level[ly * W + lx] as number;
  for (const c of all) {
    const isRoad = roadSet.has(c);
    const t = s.style === "dirt" ? (isRoad ? Terrain.Dirt : Terrain.Grass) : s.style === "lane" || s.style === "path" ? Terrain.Pavement : isRoad ? Terrain.Road : Terrain.Pavement;
    const cur = w.terrain[c] as number;
    if ((w.use[c] as number) & Use.Road && (RANK[cur] ?? 0) >= (RANK[t] ?? 0)) continue;
    if (w.water[c] === WaterKind.Creek) {
      w.terrain[c] = Terrain.Bridge;
      w.use[c] = (w.use[c] as number) | Use.Road | Use.Bridge;
      continue;
    }
    w.terrain[c] = t;
    w.level[c] = lv;
    if (t !== Terrain.Grass) w.use[c] = (w.use[c] as number) | Use.Road;
    if (!w.roadClass[c]) w.roadClass[c] = s.cls || RoadClass.Street;
  }
  return true;
}

const FACINGS: TownStreetEnd["facing"][] = ["n", "e", "s", "w", "ne", "nw", "se", "sw"];

/** The direction a run travels into its end. */
export function facingOf(s: Segment, end: 0 | 1): TownStreetEnd["facing"] {
  switch (s.axis) {
    case "h":
      return end === 1 ? "e" : "w";
    case "v":
      return end === 1 ? "s" : "n";
    case "ne":
      return end === 1 ? "ne" : "sw";
    default:
      return end === 1 ? "se" : "nw";
  }
}

/** The prop variant of what closes an end (pixel-art round 6): the facing's index, plus 8 for the second look. */
export const endPropVariant = (facing: TownStreetEnd["facing"], second = false) => FACINGS.indexOf(facing) + (second ? 8 : 0);

/** The last road cells of a run's end (its carriageway on the end's cells, or the whole band of a lane). */
export function endRoadCells(s: Segment, end: 0 | 1, W: number, H: number): number[] {
  const [k0, k1] = carriageway(s);
  const t = end === 0 ? 0 : segSteps(s);
  const out: number[] = [];
  for (let k = k0; k <= k1; k++) {
    const [x, y] = cellAt(s, t, k);
    if (x >= 0 && y >= 0 && x < W && y < H) out.push(y * W + x);
  }
  return out;
}

/** A recorded street end: the end's last road cells (a turning head's road block), their bounds, and the way the street runs into them. */
export function streetEndOf(s: Segment, end: 0 | 1, W: number, H: number): TownStreetEnd | null {
  const kind = s.ends[end];
  if (!kind) return null;
  const cells = isHead(kind) ? headShape(s, end, W, H).road : endRoadCells(s, end, W, H);
  if (!cells.length) return null;
  let x0 = W;
  let y0 = H;
  let x1 = -1;
  let y1 = -1;
  for (const c of cells) {
    const x = c % W;
    const y = (c - x) / W;
    x0 = Math.min(x0, x);
    y0 = Math.min(y0, y);
    x1 = Math.max(x1, x);
    y1 = Math.max(y1, y);
  }
  return { kind, x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1, facing: facingOf(s, end) };
}

/** `StreetAxis` per cell from the runs: a diagonal's cells take its direction, where it meets another run a junction. */
export function streetAxes(W: number, H: number, segs: Segment[], isStreet: (c: number) => boolean): Uint8Array {
  const out = new Uint8Array(W * H);
  const orth = new Uint8Array(W * H);
  for (const s of segs) {
    if (s.axis === "h" || s.axis === "v") segCells(s, W, H, (c) => (orth[c] = 1));
  }
  for (const s of segs) {
    if (s.axis !== "ne" && s.axis !== "nw") continue;
    const code = s.axis === "ne" ? StreetAxis.DiagonalNE : StreetAxis.DiagonalNW;
    segCells(s, W, H, (c) => {
      if (!isStreet(c)) return;
      const cur = out[c] as number;
      out[c] = orth[c] || (cur !== 0 && cur !== code) ? StreetAxis.Junction : code;
    });
  }
  return out;
}

/** Whether two runs are parallel (the same axis). */
export const parallel = (a: Segment, b: Segment) => a.axis === b.axis;

/**
 * For two parallel runs: how far they overlap along their axis and the gap
 * between their bands across it (cells between them; negative when the
 * bands overlap). Diagonals are measured per row (the staircase's step).
 */
export function parallelGap(a: Segment, b: Segment): { overlap: number; gap: number } {
  if (a.axis === "h") return { overlap: Math.min(a.x1, b.x1) - Math.max(a.x0, b.x0) + 1, gap: Math.max(a.y0, b.y0) - Math.min(a.y1, b.y1) - 1 };
  if (a.axis === "v") return { overlap: Math.min(a.y1, b.y1) - Math.max(a.y0, b.y0) + 1, gap: Math.max(a.x0, b.x0) - Math.min(a.x1, b.x1) - 1 };
  // Diagonals: the line constant (x + y for "ne", x − y for "nw") of each centre line, and the span along x.
  const ka = a.axis === "ne" ? a.ax + a.ay : a.ax - a.ay;
  const kb = b.axis === "ne" ? b.ax + b.ay : b.ax - b.ay;
  // Along the run, project on x (one step per tile); across, the row offset between the lines.
  const overlap = Math.min(a.bx, b.bx) - Math.max(a.ax, b.ax) + 1;
  const off = Math.abs(ka - kb);
  const gap = off - ((a.width + b.width + 1) >> 1);
  return { overlap, gap };
}

/** Offset between the centre lines of two parallel runs, cells across. */
export function centreOffset(a: Segment, b: Segment): number {
  if (a.axis === "h") return Math.abs(a.ay - b.ay);
  if (a.axis === "v") return Math.abs(a.ax - b.ax);
  const ka = a.axis === "ne" ? a.ax + a.ay : a.ax - a.ay;
  const kb = b.axis === "ne" ? b.ax + b.ay : b.ax - b.ay;
  return Math.abs(ka - kb);
}

/** Whether a run would lie alongside another (or the railway) within a few cells, or jog off another's line (`self` excepted). */
export function crowded(w: Work, s: Segment, self: Segment | null): boolean {
  for (const o of w.segments) {
    if (o === self || o.axis !== s.axis) continue;
    if (o.x0 > s.x1 + 12 || o.x1 < s.x0 - 12 || o.y0 > s.y1 + 12 || o.y1 < s.y0 - 12) continue;
    const g = parallelGap(s, o);
    const off = centreOffset(s, o);
    if (off === 0) continue;
    if (g.overlap >= 4 && g.gap <= 3) return true;
    // Collinear pieces a few cells apart across: a jog.
    if (g.overlap > -7 && g.overlap < 4 && g.gap > -Math.max(s.width, o.width) && off <= 5) return true;
  }
  for (const o of w.rail) {
    if (o.axis !== s.axis) continue;
    const g = parallelGap(s, o);
    if (g.overlap >= 4 && g.gap <= 2 && g.gap >= -1) return true;
  }
  return false;
}
