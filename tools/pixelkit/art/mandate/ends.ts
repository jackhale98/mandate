// Street ends (round 6; DEC-090 D5 amended, "street ends"; PIXEL_ART_TOOL.md
// "Round 6: street ends"). Layout v2 records how every street that joins no
// other street ends (`TownMap.streetEnds`: a kind, the last road cells and the
// way the street runs into them); this draws each kind in the flat
// three-quarter style of rounds 1–5, for orthogonal and 45° streets:
//   - turning circle: the road widens into a round bulb (a disc inscribed in
//     the end's cells, joined to the street by fillets), ringed by a kerb and
//     a pavement; a bulb of 5 cells or more gets a planted island with its
//     own kerb;
//   - hammerhead: the T- or L-shaped head the generator lays, its convex
//     corners chamfered and its inner corners filled at 45°, kerb and
//     pavement following;
//   - bollards: the carriageway stops at a flush kerb line (a dropped kerb)
//     halfway along the end's last cells; paving beyond it meets the footpath
//     (the `bollards` prop stands on that line);
//   - plaza: the asphalt gives way to a band of granite setts and runs into
//     the square's paving, with no kerb across;
//   - field gate: the asphalt breaks up into an earth track with grass down
//     its middle, the pavements give way to verges, and a hedge crosses the
//     end with a gap for the gate (the `field_gate` prop);
//   - railing: the carriageway stops at a kerb, a paved apron runs on to the
//     water's edge (the `railing` prop stands along it).
// Each end is a shape in its own frame (u along the street toward its end,
// v across it), measured from the street the map draws: the carriageway's
// centre and width, and its pavements. Inside the end's zone (its cells and
// one or two cells round them) the shape decides what each map pixel is;
// pixels whose material changes, and the few pixels whose edges they reach,
// are repainted from the materials' own fills and edge profiles (round 1's
// kerbs, gutters and edgings), then the end's details are drawn. Every other
// pixel keeps what the map draws without street ends. Everything is a
// function of the map alone (integer pixel rules, a fixed hash), so cells
// join without seams and two runs draw the same.
import { hash32 } from "../../src/core/hash";
import type { Indexed } from "../../src/core/image";
import { EDGES, type TileSet } from "./tiles";

/** `StreetEndKind` (web/src/hub/townLayout.ts; a web test checks they agree). */
export const END_KINDS = ["turning_circle", "hammerhead", "bollards", "plaza", "field_gate", "railing"] as const;
export type EndKind = (typeof END_KINDS)[number];
/**
 * `TownStreetEnd.facing`: the way the street runs into its end. The props
 * that close an end (`bollards`, `field_gate`, `railing`) take this index as
 * their variant (plus 8 for the second look: lit bollards, an open gate).
 */
export const END_FACINGS = ["n", "e", "s", "w", "ne", "nw", "se", "sw"] as const;
export type EndFacing = (typeof END_FACINGS)[number];

/** A street end as the layout gives it (`TownStreetEnd`): the last road cells of the street, tiles. */
export interface StreetEndView {
  kind: string;
  x: number;
  y: number;
  w: number;
  h: number;
  facing: string;
}

/** A prop as the layout gives it (`TownProp`): the end reads where its gate stands. */
export interface EndPropView {
  kind: string;
  x: number;
  y: number;
}

/** What the end painter reads from the terrain painter. */
export interface EndHost {
  readonly width: number;
  readonly height: number;
  /** Terrain at a cell (clamped to the map). */
  t(x: number, y: number): number;
  /** The material (autotile group) at a map pixel as drawn without street ends (diagonal streets included). */
  mat(X: number, Y: number): number;
}

/** The terrain rules the end painter shares (codes come from ./terrain). */
export interface EndRules {
  group(t: number): number;
  /** Terrain codes whose cells an end may repaint. */
  claimable: ReadonlySet<number>;
  /** The tile set's terrain name per material (autotile group). */
  materialName(g: number): string;
  road: number;
  pavement: number;
  plaza: number;
  path: number;
  dirt: number;
  grass: number;
  park: number;
  quay: number;
}

const S2 = Math.SQRT1_2;
const SQ2 = Math.SQRT2;
/** Unit vectors per facing (screen coordinates: y grows down). */
const DIR: Readonly<Record<EndFacing, readonly [number, number]>> = {
  n: [0, -1],
  e: [1, 0],
  s: [0, 1],
  w: [-1, 0],
  ne: [S2, -S2],
  nw: [-S2, -S2],
  se: [S2, S2],
  sw: [-S2, S2],
};

/** Prop variant for an end's facing (0–7), plus 8 for the second look. */
export const endPropVariant = (facing: string, second = false): number => Math.max(0, END_FACINGS.indexOf(facing as EndFacing)) + (second ? 8 : 0);

const mod = (a: number, n: number) => ((a % n) + n) % n;

type Metric = "euclid" | "diamond" | "square";
/** Chamfer weights (orthogonal, diagonal) and the unit of one pixel. */
const METRIC: Readonly<Record<Metric, readonly [number, number, number]>> = { euclid: [3, 4, 3], diamond: [1, 2, 1], square: [1, 1, 1] };

/** Distance (chamfer units) from every pixel to the nearest set pixel of `mask` (two passes). */
function distanceTo(mask: Uint8Array, w: number, h: number, metric: Metric): Uint16Array {
  const [a, b] = METRIC[metric];
  const INF = 60000;
  const d = new Uint16Array(w * h);
  for (let i = 0; i < d.length; i++) d[i] = mask[i] ? 0 : INF;
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = y * w + x;
      let v = d[i] as number;
      if (v === 0) continue;
      if (x > 0) v = Math.min(v, (d[i - 1] as number) + a);
      if (y > 0) {
        v = Math.min(v, (d[i - w] as number) + a);
        if (x > 0) v = Math.min(v, (d[i - w - 1] as number) + b);
        if (x < w - 1) v = Math.min(v, (d[i - w + 1] as number) + b);
      }
      d[i] = v;
    }
  }
  for (let y = h - 1; y >= 0; y--) {
    for (let x = w - 1; x >= 0; x--) {
      const i = y * w + x;
      let v = d[i] as number;
      if (v === 0) continue;
      if (x < w - 1) v = Math.min(v, (d[i + 1] as number) + a);
      if (y < h - 1) {
        v = Math.min(v, (d[i + w] as number) + a);
        if (x < w - 1) v = Math.min(v, (d[i + w + 1] as number) + b);
        if (x > 0) v = Math.min(v, (d[i + w - 1] as number) + b);
      }
      d[i] = v;
    }
  }
  return d;
}

/** Morphology on a mask: grow by r px (dilate) or keep what lies more than r px inside (erode). */
function dilate(mask: Uint8Array, w: number, h: number, r: number, metric: Metric): Uint8Array {
  const d = distanceTo(mask, w, h, metric);
  const lim = r * (METRIC[metric][2] as number);
  const out = new Uint8Array(w * h);
  for (let i = 0; i < out.length; i++) out[i] = (d[i] as number) <= lim ? 1 : 0;
  return out;
}

function erode(mask: Uint8Array, w: number, h: number, r: number, metric: Metric): Uint8Array {
  const inv = new Uint8Array(w * h);
  for (let i = 0; i < inv.length; i++) inv[i] = mask[i] ? 0 : 1;
  const grown = dilate(inv, w, h, r, metric);
  for (let i = 0; i < inv.length; i++) inv[i] = grown[i] ? 0 : 1;
  return inv;
}

/** One end, measured and laid out in its own frame. */
interface End {
  kind: EndKind;
  facing: EndFacing;
  diag: boolean;
  /** Along (toward the end) and across, unit vectors. */
  dx: number;
  dy: number;
  ax: number;
  ay: number;
  /** Origin (map px, integer): on the street's centre line, at the end line. */
  ox: number;
  oy: number;
  /** Half the carriageway's width and the pavement's width (px, square to the street). */
  hw: number;
  pw: number;
  /** The end's cells and its zone (cells, end exclusive). */
  rx0: number;
  ry0: number;
  rx1: number;
  ry1: number;
  zx0: number;
  zy0: number;
  zx1: number;
  zy1: number;
  /** Turning circle: the bulb's centre (u) and radius, the island's radius (0: none). */
  uc: number;
  R: number;
  Ri: number;
  /** Field gate: where the gate stands across the street (v). */
  gv: number;
  /** Materials per pixel of the zone and a margin (built on first use). */
  grid: Int16Array | null;
  gx: number;
  gy: number;
  gw: number;
  gh: number;
}

const MARGIN = 16;

export class StreetEnds {
  private readonly h: EndHost;
  private readonly r: EndRules;
  private readonly ts: TileSet;
  private readonly L: Record<string, number>;
  private readonly W: number;
  private readonly H: number;
  private readonly list: End[] = [];
  /** Per cell: 1 + the index of the first end whose zone (or edge reach) covers it, or 0. */
  private readonly touchedAt: Int16Array;
  private readonly profiles = new Map<string, { N: number[]; S: number[]; W: number[]; E: number[] }>();
  /** Whether any end was laid out (otherwise this draws nothing). */
  readonly active: boolean;

  constructor(host: EndHost, rules: EndRules, ts: TileSet, legend: Record<string, number>, ends: readonly StreetEndView[], props: readonly EndPropView[] = []) {
    this.h = host;
    this.r = rules;
    this.ts = ts;
    this.L = legend;
    this.W = host.width;
    this.H = host.height;
    this.touchedAt = new Int16Array(this.W * this.H);
    for (const [name, e] of Object.entries(EDGES)) {
      const p = (s: string) => [...s].map((c) => legend[c] as number);
      this.profiles.set(name, { N: p(e.N), S: p(e.S), W: p(e.W), E: p(e.E) });
    }
    for (const e of ends) {
      const end = this.layout(e, props);
      if (end) this.list.push(end);
    }
    this.list.forEach((end, k) => {
      for (let y = end.zy0 - 1; y <= end.zy1; y++) {
        for (let x = end.zx0 - 1; x <= end.zx1; x++) {
          if (x < 0 || y < 0 || x >= this.W || y >= this.H) continue;
          if (!this.touchedAt[y * this.W + x]) this.touchedAt[y * this.W + x] = k + 1;
        }
      }
    });
    this.active = this.list.length > 0;
  }

  /** The ends laid out (for lints and tests): kind, facing, frame and measures. */
  get ends(): readonly Readonly<Pick<End, "kind" | "facing" | "diag" | "ox" | "oy" | "dx" | "dy" | "ax" | "ay" | "hw" | "pw" | "uc" | "R" | "Ri" | "gv" | "zx0" | "zy0" | "zx1" | "zy1">>[] {
    return this.list;
  }

  /** Whether a street end changes this cell or its edges (never "simple"). */
  touched(x: number, y: number): boolean {
    return this.active && x >= 0 && y >= 0 && x < this.W && y < this.H && this.touchedAt[y * this.W + x] !== 0;
  }

  // ------------------------------------------------------------ layout

  private layout(e: StreetEndView, props: readonly EndPropView[]): End | null {
    if (!(END_KINDS as readonly string[]).includes(e.kind) || !(END_FACINGS as readonly string[]).includes(e.facing)) return null;
    const kind = e.kind as EndKind;
    const facing = e.facing as EndFacing;
    const rx0 = Math.max(0, Math.floor(e.x));
    const ry0 = Math.max(0, Math.floor(e.y));
    const rx1 = Math.min(this.W, Math.floor(e.x) + Math.max(1, Math.floor(e.w)));
    const ry1 = Math.min(this.H, Math.floor(e.y) + Math.max(1, Math.floor(e.h)));
    if (rx1 <= rx0 || ry1 <= ry0) return null;
    const [dx, dy] = DIR[facing];
    const ax = -dy;
    const ay = dx;
    const diag = dx !== 0 && dy !== 0;
    const road = this.r.road;
    // The end's middle, and how far its road cells reach toward the end (the end line runs through the farthest cell's middle).
    const cx = ((rx0 + rx1) / 2) * 16;
    const cy = ((ry0 + ry1) / 2) * 16;
    let uFar = -Infinity;
    let any = false;
    for (let pass = 0; pass < 2 && !any; pass++) {
      for (let y = ry0; y < ry1; y++) {
        for (let x = rx0; x < rx1; x++) {
          if (pass === 0 && this.h.mat(x * 16 + 8, y * 16 + 8) !== road) continue;
          any = true;
          uFar = Math.max(uFar, (x * 16 + 8 - cx) * dx + (y * 16 + 8 - cy) * dy);
        }
      }
    }
    // The street just behind the end: its carriageway's centre and width, its pavements (scanning across it).
    // Several lines across, stepping back from the end's middle (outside its cells): the narrowest carriageway found
    // is the street's (a wide head, a bulb or an avenue further back measure wider).
    const depth = Math.abs((rx1 - rx0) * 8 * dx) + Math.abs((ry1 - ry0) * 8 * dy);
    const across = Math.abs((rx1 - rx0) * 16 * ax) + Math.abs((ry1 - ry0) * 16 * ay);
    let v0 = 0;
    let hw = Math.max(8, (kind === "turning_circle" ? across - 32 : across) / 2);
    let pw = 0;
    let best = Infinity;
    for (let back = 12; back <= depth + 72; back += 4) {
      const sx = Math.floor(cx - dx * back) + 0.5;
      const sy = Math.floor(cy - dy * back) + 0.5;
      const cxs = Math.floor(sx / 16);
      const cys = Math.floor(sy / 16);
      if (cxs >= rx0 && cxs < rx1 && cys >= ry0 && cys < ry1) continue;
      const at = (k: number) => this.h.mat(Math.floor(sx + ax * k), Math.floor(sy + ay * k));
      let start: number | null = null;
      for (let k = 0; k <= 40 && start === null; k++) {
        if (at(k) === road) start = k;
        else if (at(-k) === road) start = -k;
      }
      if (start === null) continue;
      let hi = start;
      let lo = start;
      while (hi - start < 120 && at(hi + 1) === road) hi++;
      while (start - lo < 120 && at(lo - 1) === road) lo--;
      const width = hi - lo + 1;
      if (width < 8 || width >= best) continue;
      best = width;
      const off = (sx - cx) * ax + (sy - cy) * ay;
      v0 = off + (lo + hi) / 2;
      hw = width / 2;
      const pav = this.r.pavement;
      let p1 = 0;
      let p2 = 0;
      while (p1 < 24 && at(hi + 1 + p1) === pav) p1++;
      while (p2 < 24 && at(lo - 1 - p2) === pav) p2++;
      pw = p1 && p2 ? Math.min(p1, p2) : Math.max(p1, p2);
    }
    if (diag) hw = Math.max(hw, 6);
    // The origin: on the centre line, at the end line; on whole pixels so lines fall on the pixel grid.
    const ox = Math.round(cx + dx * uFar + ax * v0);
    const oy = Math.round(cy + dy * uFar + ay * v0);
    // The bulb (turning circles): the disc inscribed in the end's cells, centred on the street's line.
    const uc = (cx - ox) * dx + (cy - oy) * dy;
    const R = Math.min(rx1 - rx0, ry1 - ry0) * 8;
    const ri = R - Math.max(22, hw - 4);
    const Ri = kind === "turning_circle" && ri >= 12 ? ri : 0;
    // The gate stands where a field gate prop is on or beside the end's cells (else on the street's line).
    let gv = 0;
    if (kind === "field_gate") {
      const g = props.find((p) => p.kind === "field_gate" && p.x >= rx0 - 1 && p.x <= rx1 && p.y >= ry0 - 1 && p.y <= ry1);
      if (g) gv = (g.x * 16 + 8 - ox) * ax + (g.y * 16 + 8 - oy) * ay;
    }
    // The zone: the end's cells and one cell round them (two on diagonals, and across a field gate's hedge).
    const m = diag ? 2 : 1;
    const side = kind === "field_gate" ? 2 : m;
    const mx = ax !== 0 ? Math.max(m, side) : m;
    const my = ay !== 0 ? Math.max(m, side) : m;
    return {
      kind,
      facing,
      diag,
      dx,
      dy,
      ax,
      ay,
      ox,
      oy,
      hw,
      pw,
      rx0,
      ry0,
      rx1,
      ry1,
      zx0: Math.max(0, rx0 - mx),
      zy0: Math.max(0, ry0 - my),
      zx1: Math.min(this.W, rx1 + mx),
      zy1: Math.min(this.H, ry1 + my),
      uc,
      R,
      Ri,
      gv,
      grid: null,
      gx: 0,
      gy: 0,
      gw: 0,
      gh: 0,
    };
  }

  // ------------------------------------------------------------ materials

  private u(e: End, X: number, Y: number): number {
    return (X + 0.5 - e.ox) * e.dx + (Y + 0.5 - e.oy) * e.dy;
  }

  private v(e: End, X: number, Y: number): number {
    return (X + 0.5 - e.ox) * e.ax + (Y + 0.5 - e.oy) * e.ay;
  }

  /** Along and across the street in whole "rows" (vertical px on 45° lines, px otherwise), for bands of detail. */
  private s(e: End, X: number, Y: number): number {
    return e.diag ? Math.round(this.u(e, X, Y) * SQ2) : Math.floor(this.u(e, X, Y));
  }

  private tAcross(e: End, X: number, Y: number): number {
    return e.diag ? Math.round(this.v(e, X, Y) * SQ2) : Math.floor(this.v(e, X, Y));
  }

  private inZone(e: End, X: number, Y: number): boolean {
    const x = X >> 4;
    const y = Y >> 4;
    return x >= e.zx0 && y >= e.zy0 && x < e.zx1 && y < e.zy1;
  }

  /** The material at a map pixel as the map draws it without street ends. */
  baseMat(X: number, Y: number): number {
    return this.h.mat(X, Y);
  }

  /** The material at a map pixel, street ends included. */
  mat(X: number, Y: number): number {
    if (this.active) {
      const x = X >> 4;
      const y = Y >> 4;
      if (x >= 0 && y >= 0 && x < this.W && y < this.H) {
        const k = this.touchedAt[y * this.W + x] as number;
        if (k) {
          for (let i = k - 1; i < this.list.length; i++) {
            const e = this.list[i] as End;
            if (!this.inZone(e, X, Y)) continue;
            const g = this.grid(e);
            return g[(Y - e.gy) * e.gw + (X - e.gx)] as number;
          }
        }
      }
    }
    return this.h.mat(X, Y);
  }

  /** Whether the end may repaint a pixel: in its zone, on a cell of a terrain it may change. */
  private claimable(e: End, X: number, Y: number): boolean {
    return this.inZone(e, X, Y) && this.r.claimable.has(this.h.t(X >> 4, Y >> 4));
  }

  /** The end's materials over its zone and a margin (the margin keeps the base, for the shape operations). */
  private grid(e: End): Int16Array {
    if (e.grid) return e.grid;
    const gx = e.zx0 * 16 - MARGIN;
    const gy = e.zy0 * 16 - MARGIN;
    const gw = (e.zx1 - e.zx0) * 16 + 2 * MARGIN;
    const gh = (e.zy1 - e.zy0) * 16 + 2 * MARGIN;
    const n = gw * gh;
    const base = new Int16Array(n);
    const can = new Uint8Array(n);
    for (let j = 0; j < gh; j++) {
      for (let i = 0; i < gw; i++) {
        base[j * gw + i] = this.h.mat(gx + i, gy + j);
        can[j * gw + i] = this.claimable(e, gx + i, gy + j) ? 1 : 0;
      }
    }
    const out = base.slice();
    const R = this.r;
    const set = (k: number, m: number) => {
      if (can[k]) out[k] = m;
    };
    const each = (f: (k: number, u: number, v: number, b: number, X: number, Y: number) => void) => {
      for (let j = 0; j < gh; j++) {
        for (let i = 0; i < gw; i++) {
          const k = j * gw + i;
          if (!can[k]) continue;
          const X = gx + i;
          const Y = gy + j;
          f(k, this.u(e, X, Y), this.v(e, X, Y), base[k] as number, X, Y);
        }
      }
    };
    const hw = e.hw;
    const pw = e.pw;
    switch (e.kind) {
      case "turning_circle":
      case "hammerhead": {
        const circle = e.kind === "turning_circle";
        const metric: Metric = circle ? "euclid" : e.diag ? "square" : "diamond";
        // The road's shape: the bulb or the head, and the street running into it.
        const road = new Uint8Array(n);
        const head = circle ? null : this.head(e);
        for (let k = 0; k < n; k++) road[k] = base[k] === R.road ? 1 : 0;
        each((k, u, v, b) => {
          let inShape: boolean;
          if (circle) inShape = (u - e.uc) ** 2 + v * v < e.R * e.R || (u <= e.uc && u > e.uc - 32 && Math.abs(v) < hw);
          else inShape = (head as (u: number, v: number) => boolean)(u, v);
          if (inShape) road[k] = 1;
          else if (b === R.road && u > (circle ? e.uc : -16) - 32) road[k] = 0;
        });
        // Fillets where the street meets the bulb (and the inner corners of a head), chamfers on a head's outer corners.
        const closed = erode(dilate(road, gw, gh, circle ? 12 : 6, metric), gw, gh, circle ? 12 : 6, metric);
        const smooth = circle ? closed : dilate(erode(closed, gw, gh, 6, metric), gw, gh, 6, metric);
        // A ring of pavement round the new road (as wide as the street's), and the bulb's island.
        const ring = dilate(smooth, gw, gh, pw > 0 ? pw : 12, metric);
        each((k, u, v, b) => {
          if (smooth[k]) set(k, R.road);
          else if (ring[k]) set(k, R.pavement);
          else if (b === R.road) set(k, R.pavement);
          if (e.Ri > 0) {
            const d2 = (u - e.uc) ** 2 + v * v;
            if (d2 < e.Ri * e.Ri) set(k, d2 < (e.Ri - 3) * (e.Ri - 3) ? R.park : R.pavement);
          }
        });
        break;
      }
      case "bollards":
      case "plaza":
      case "railing":
      case "field_gate": {
        // Past the end line: footpath paving (bollards), the square (plaza), an apron to the water (railing), a verge (field gate).
        const beyondOf = (b: number) => (e.kind === "plaza" ? R.plaza : e.kind === "field_gate" ? R.grass : e.kind === "railing" && b === R.quay ? R.quay : R.pavement);
        const far = e.diag ? (e.kind === "railing" ? 48 : 12) : 8;
        const wide = hw + pw + (e.diag && e.kind === "railing" ? 24 : 0);
        each((k, u, v, b, X) => {
          if (u < -24) return;
          if (e.kind === "field_gate") {
            const t = this.gateTrack(e, u, v, X);
            if (t === 1) set(k, R.road);
            else if (t === 2) set(k, R.dirt);
            else if (u > -18 && (b === R.road || b === R.pavement)) set(k, R.grass);
            return;
          }
          if (u < 0) {
            if (Math.abs(v) < hw) set(k, R.road);
            else if (b === R.road && u > -16) set(k, R.pavement);
            return;
          }
          if (Math.abs(v) < wide && u < far) set(k, beyondOf(b));
          else if (b === R.road || (e.kind === "plaza" && b === R.pavement && Math.abs(v) < wide && u < 24)) set(k, beyondOf(b));
        });
        break;
      }
    }
    e.grid = out;
    e.gx = gx;
    e.gy = gy;
    e.gw = gw;
    e.gh = gh;
    return out;
  }

  /** A hammerhead's head and the street into it, in the end's frame. */
  private head(e: End): (u: number, v: number) => boolean {
    const hw = e.hw;
    if (!e.diag) {
      // The head is the end's cells (a T, or an L when they lie to one side of the street).
      const corners = [
        [e.rx0 * 16, e.ry0 * 16],
        [e.rx1 * 16, e.ry0 * 16],
        [e.rx0 * 16, e.ry1 * 16],
        [e.rx1 * 16, e.ry1 * 16],
      ].map(([X, Y]) => [this.u(e, (X as number) - 0.5, (Y as number) - 0.5), this.v(e, (X as number) - 0.5, (Y as number) - 0.5)] as const);
      const u0 = Math.min(...corners.map((c) => c[0]));
      const u1 = Math.max(...corners.map((c) => c[0]));
      const v0 = Math.min(...corners.map((c) => c[1]));
      const v1 = Math.max(...corners.map((c) => c[1]));
      return (u, v) => (u > u0 && u < u1 && v > v0 && v < v1) || (u <= u0 + 1 && u > u0 - 32 && Math.abs(v) < hw);
    }
    // On a 45° street: a bar square to the street, as deep as the street is wide, reaching the end's cells either side.
    // Arms of 20 px past the carriageway (a cell and a bit, square to the street), longer where the end's cells reach on (up to 32).
    let vlo = -(hw + 20);
    let vhi = hw + 20;
    for (let y = e.ry0; y < e.ry1; y++) {
      for (let x = e.rx0; x < e.rx1; x++) {
        if (this.h.mat(x * 16 + 8, y * 16 + 8) !== this.r.road) continue;
        const v = this.v(e, x * 16 + 7.5, y * 16 + 7.5);
        vlo = Math.max(-(hw + 32), Math.min(vlo, v - 8));
        vhi = Math.min(hw + 32, Math.max(vhi, v + 8));
      }
    }
    const u1 = 8;
    const u0 = u1 - Math.max(24, 2 * hw + 4);
    return (u, v) => (u > u0 && u < u1 && v > vlo && v < vhi) || (u <= u0 + 1 && u > u0 - 32 && Math.abs(v) < hw);
  }

  /** A field gate's ragged asphalt edge (u of the edge at this point across the street). */
  private fadeAt(e: End, v: number): number {
    const k = Math.floor((v + 256) / 2);
    return -14 + (hash32(k, e.ox, e.oy, 0x9a7e) % 5) - 2;
  }

  /** What a field gate's end is at (u, v): 1 asphalt, 2 the earth track, 0 neither. */
  private gateTrack(e: End, u: number, v: number, _X: number): number {
    const fade = this.fadeAt(e, v);
    if (u < fade) return Math.abs(v) < e.hw ? 1 : 0;
    const t = Math.max(0, Math.min(1, (u + 14) / 10));
    const centre = e.gv * t;
    const half = e.hw + (7 - e.hw) * t;
    return Math.abs(v - centre) < half && u < (e.diag ? 40 : 24) ? 2 : 0;
  }

  // ------------------------------------------------------------ drawing

  private fillPx(name: string, X: number, Y: number, own: boolean, variant: number): number {
    const t = this.ts.terrains[name] ?? (this.ts.terrains.grass as { template: Indexed; fills: Indexed[] });
    const f = (own && t.fills.length > 1 && variant >> 1 === 3 ? t.fills[1 + ((variant & 1) % (t.fills.length - 1))] : t.fills[0]) as Indexed;
    return f.px[(Y & 15) * 16 + (X & 15)] as number;
  }

  /**
   * A material's edge at a pixel, as its autotile template draws it: the
   * north or south profile where another material lies within its reach
   * above or below (the nearer; south on a tie), else the west or east one;
   * at an inner corner (another material only diagonally), the corner's.
   */
  private edgePx(M: number, X: number, Y: number, at: (X: number, Y: number) => number = (a, b) => this.mat(a, b)): number {
    const prof = this.profiles.get(this.r.materialName(M));
    if (!prof) return 0;
    // How far another material is along a profile's direction (99: not within its reach).
    const reach = (p: readonly number[], sx: number, sy: number): number => {
      for (let k = 1; k <= p.length; k++) if (at(X + sx * k, Y + sy * k) !== M) return k;
      return 99;
    };
    const n = reach(prof.N, 0, -1);
    const s = reach(prof.S, 0, 1);
    if (n < 99 || s < 99) return s <= n ? (prof.S[s - 1] as number) : (prof.N[n - 1] as number);
    const w = reach(prof.W, -1, 0);
    const e = reach(prof.E, 1, 0);
    if (w < 99 || e < 99) return e <= w ? (prof.E[e - 1] as number) : (prof.W[w - 1] as number);
    // Inner corners: the nearest other material diagonally (Chebyshev depth), along the row or down the column.
    const lim = Math.max(prof.N.length, prof.S.length, prof.W.length, prof.E.length);
    for (let d = 1; d <= lim; d++) {
      for (const [sx, sy] of [
        [-1, -1],
        [1, -1],
        [-1, 1],
        [1, 1],
      ] as const) {
        for (let a = 1; a <= d; a++) {
          for (let b = 1; b <= d; b++) {
            if (Math.max(a, b) !== d || at(X + sx * a, Y + sy * b) === M) continue;
            // Only a corner: the row and the column toward it are this material.
            if (at(X + sx * a, Y) !== M || at(X, Y + sy * b) !== M) continue;
            // As the templates wrap an edge round a missing corner: further along the row than down the column takes the row's (north or south) profile.
            const p = a > b ? (sy < 0 ? prof.N : prof.S) : sx < 0 ? prof.W : prof.E;
            if (d <= p.length) return p[d - 1] as number;
          }
        }
      }
    }
    return 0;
  }

  /**
   * Draw the street ends over a cell the map has drawn (`img`, 16×16, changed
   * in place): pixels whose material the ends change, or whose edges reach
   * such a pixel, are repainted; then the ends' details.
   */
  apply(img: Indexed, x: number, y: number, variant: number): void {
    if (!this.touched(x, y) || !this.r.claimable.has(this.h.t(x, y))) return;
    const X0 = x * 16;
    const Y0 = y * 16;
    const own = this.r.group(this.h.t(x, y));
    // Pixels within 4 px of the cell whose material the ends change.
    const S = 24;
    const changed = new Uint8Array(S * S);
    const win = new Int16Array(S * S);
    for (let j = 0; j < S; j++) {
      for (let i = 0; i < S; i++) {
        const m = this.mat(X0 - 4 + i, Y0 - 4 + j);
        win[j * S + i] = m;
        if (m !== this.h.mat(X0 - 4 + i, Y0 - 4 + j)) changed[j * S + i] = 1;
      }
    }
    // Materials near the cell from the window (edges reach at most 4 px).
    const at = (XX: number, YY: number) => {
      const i = XX - X0 + 4;
      const j = YY - Y0 + 4;
      return i >= 0 && j >= 0 && i < S && j < S ? (win[j * S + i] as number) : this.mat(XX, YY);
    };
    // Which of the cell's pixels have a change within 4 px (the window grown by 4 px, rows then columns).
    const rows = new Uint8Array(S * 16);
    for (let j = 0; j < S; j++) {
      for (let px = 0; px < 16; px++) {
        let c = 0;
        for (let i = px; i <= px + 8 && !c; i++) c = changed[j * S + i] as number;
        rows[j * 16 + px] = c;
      }
    }
    // No centre dash or other marking runs into an end.
    const inEnd = this.list.some((e) => x >= e.zx0 && y >= e.zy0 && x < e.zx1 && y < e.zy1);
    const chalkM = this.L.M as number;
    const chalkm = this.L.m as number;
    for (let py = 0; py < 16; py++) {
      for (let px = 0; px < 16; px++) {
        const X = X0 + px;
        const Y = Y0 + py;
        let near = false;
        for (let j = py; j <= py + 8 && !near; j++) near = rows[j * 16 + px] === 1;
        const M = win[(py + 4) * S + px + 4] as number;
        if (near) {
          // Repainted: the material's fill, or its edge where another material is within the edge's reach.
          const edge = this.edgePx(M, X, Y, at);
          img.px[py * 16 + px] = edge > 0 ? edge : this.fillPx(this.r.materialName(M), X, Y, M === own, variant);
        } else if (inEnd && M === this.r.road && (img.px[py * 16 + px] === chalkM || img.px[py * 16 + px] === chalkm)) img.px[py * 16 + px] = this.fillPx(this.r.materialName(M), X, Y, M === own, variant);
      }
    }
    this.details(img, X0, Y0);
  }

  /** Each end's drawn details over the cell: a dropped kerb, setts, a track and hedge, an island's planting. */
  private details(img: Indexed, X0: number, Y0: number) {
    // Only the ends whose zone holds this cell (a cell is in at most a few).
    const x = X0 >> 4;
    const y = Y0 >> 4;
    const here = this.list.filter((e) => x >= e.zx0 && y >= e.zy0 && x < e.zx1 && y < e.zy1);
    if (here.length === 0 || !this.r.claimable.has(this.h.t(x, y))) return;
    for (let py = 0; py < 16; py++) {
      for (let px = 0; px < 16; px++) {
        for (const e of here) {
          const d = this.detail(e, X0 + px, Y0 + py);
          if (d > 0) {
            img.px[py * 16 + px] = d;
            break;
          }
        }
      }
    }
  }

  /** The detail an end draws at a map pixel (a palette index), or 0 (the pixel shows its material). */
  detailAt(X: number, Y: number): number {
    if (!this.active) return 0;
    for (const e of this.list) {
      if (!this.claimable(e, X, Y)) continue;
      const d = this.detail(e, X, Y);
      if (d > 0) return d;
    }
    return 0;
  }

  private detail(e: End, X: number, Y: number): number {
    const L = this.L;
    const R = this.r;
    const M = this.mat(X, Y);
    const u = this.u(e, X, Y);
    const v = this.v(e, X, Y);
    const s = this.s(e, X, Y);
    const t = this.tAcross(e, X, Y);
    let ch: string | null = null;
    switch (e.kind) {
      case "bollards":
        // A dropped kerb across the carriageway: a gutter line, then flush kerb stones (light top, joints every 8 px).
        if (Math.abs(v) < e.hw) {
          if (s === -1 && M === R.road) ch = "A";
          else if ((s === 0 || s === 1) && M === R.pavement) ch = mod(t, 8) === 0 ? "S" : s === 0 ? "t" : "s";
          // Flush: the paving beyond shows no kerb edge.
          else if (s === 2 && M === R.pavement) return this.fillPx("pavement", X, Y, false, 0);
        }
        break;
      case "plaza":
        // The asphalt gives way to two courses of grey granite setts (staggered, lit on top), then the square's paving.
        if (Math.abs(v) < e.hw && M === R.road && s >= -6) {
          const row = Math.floor(s / 3);
          const inRow = mod(s, 3);
          const off = mod(row, 2) * 2;
          ch = inRow === 0 || mod(t + off, 4) === 0 ? "P" : inRow === 1 ? "q" : "p";
        } else if (M === R.plaza && s >= 0 && s < 3 && Math.abs(v) < e.hw + e.pw) {
          // No kerb or edging across: the square's paving begins at once.
          return this.fillPx("plaza", X, Y, false, 0);
        }
        break;
      case "field_gate": {
        const fade = this.fadeAt(e, v);
        // Broken asphalt in the first few rows of the track; grass down the middle further on.
        if (M === R.dirt) {
          const tt = Math.max(0, Math.min(1, (u + 14) / 10));
          const c = e.gv * tt;
          if (u < fade + 5 && hash32(X >> 1, Y >> 1, 0xa5f) % 4 === 0) ch = "A";
          else if (u < 0 && Math.abs(v - c) >= 4.5) ch = "e";
        }
        // The hedge across the end, with a gap where the gate stands.
        const hedge = (XX: number, YY: number) => {
          const uu = this.u(e, XX, YY);
          const vv = this.v(e, XX, YY);
          // A band 8 px deep, its edges uneven in clumps of 3 px.
          const clump = hash32(Math.floor((vv + 256) / 3), e.ox, e.oy, 0x4ed9);
          if (uu < -2.5 + (clump & 1) || uu > 5.5 - ((clump >> 1) & 1) || Math.abs(vv - e.gv) < 8) return false;
          const reach = e.hw + e.pw + 22 + (hash32(e.ox, e.oy, vv < 0 ? 1 : 2) % 6);
          return Math.abs(vv) < reach && this.claimable(e, XX, YY);
        };
        if (hedge(X, Y)) {
          const up = hedge(X, Y - 1);
          const down = hedge(X, Y + 1);
          const down2 = hedge(X, Y + 2);
          const left = hedge(X - 1, Y);
          const right = hedge(X + 1, Y);
          if (!down) ch = "H";
          else if (!up) ch = hash32(X >> 1, 0x4ed) % 3 === 0 ? "h" : "G";
          else if (!down2) ch = "H";
          else if (!left) ch = "G";
          else if (!right) ch = "H";
          else ch = hash32(X >> 1, Y >> 1, 0x4e1) % 3 === 0 ? "G" : "H";
        } else if (hedge(X, Y - 1) && !this.claimableAndHedgeGap(e, X, Y)) {
          // The hedge's shadow on the ground below it.
          if (M === R.grass || M === R.dirt) ch = "H";
        }
        break;
      }
      case "turning_circle":
        // The island: shrubs in a ring inside its kerb, flowers in the middle.
        if (e.Ri > 0 && M === R.park) {
          const d2 = (u - e.uc) ** 2 + v * v;
          if (d2 >= (e.Ri - 8) ** 2) ch = v < 0 === u - e.uc > 0 ? "G" : hash32(X >> 1, Y >> 1, 0x15) % 3 === 0 ? "h" : "g";
          else if (d2 < (e.Ri - 10) ** 2) {
            const hh = hash32(X >> 1, Y >> 1, 0xf10);
            ch = hh % 7 === 0 ? "r" : hh % 11 === 1 ? "y" : hh % 13 === 2 ? "c" : hh % 3 === 0 ? "h" : "g";
          } else ch = "H";
        }
        break;
      default:
        break;
    }

    return ch ? (L[ch] as number) : 0;
  }

  /** The gap in a field gate's hedge (no shadow drawn under the gate's opening). */
  private claimableAndHedgeGap(e: End, X: number, Y: number): boolean {
    return Math.abs(this.v(e, X, Y) - e.gv) < 8;
  }
}
