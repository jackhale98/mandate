// A hand-authored town of street ends (round 6; DEC-090 D5 amended, "street
// ends"), in the layout v2 vocabulary: every kind of end (turning circle,
// hammerhead, bollards, plaza, field gate, railing) in all four orthogonal
// facings and two diagonal ones (ne on a "/" street, se on a "\" street), at
// street widths 2 to 4, recorded in `streetEnds` as the generator records
// them (the end's last road cells and the way the street runs into them).
// Streets branch off avenues and end facing each other in pairs that share
// what lies beyond their ends: a footpath through a green (bollards), a
// square (plaza), a field with a track between two gates (field gate), a
// basin (railing), houses round the turning heads. The props that close the
// ends stand on the end lines. The generator emits the same vocabulary; this
// map does not depend on it. The contact sheets render it by day, at dusk and
// at night; the lints and tests draw every cell of it.
import { endPropVariant, END_KINDS, type EndFacing, type EndKind, type StreetEndView } from "./ends";
import { STREET_AXIS as AX } from "./streets";
import { TERRAIN as K } from "./terrain";
import type { Scene, SceneBuilding, SceneProp } from "./v2scene";

export interface EndScene extends Scene {
  streetAxis: Uint8Array;
  streetEnds: (StreetEndView & { kind: EndKind; facing: EndFacing })[];
}

export const ENDS_W = 80;
export const ENDS_H = 93;

/** The street width (road cells) of each kind in each facing: 2 to 4, spread so every kind has each width. */
export const endWidth = (kind: EndKind, facing: EndFacing): number => 2 + ((END_KINDS.indexOf(kind) + ["n", "s", "e", "w", "ne", "se"].indexOf(facing)) % 3);

export function endScene(): EndScene {
  const W = ENDS_W;
  const H = ENDS_H;
  const terrain = new Uint8Array(W * H).fill(K.Grass);
  const elevation = new Uint8Array(W * H);
  const streetAxis = new Uint8Array(W * H);
  const inside = (x: number, y: number) => x >= 0 && y >= 0 && x < W && y < H;
  const at = (x: number, y: number) => (inside(x, y) ? (terrain[y * W + x] as number) : -1);
  const set = (t: number, x: number, y: number) => {
    if (inside(x, y)) terrain[y * W + x] = t;
  };
  const rect = (t: number, x: number, y: number, w = 1, h = 1) => {
    for (let yy = y; yy < y + h; yy++) for (let xx = x; xx < x + w; xx++) set(t, xx, yy);
  };
  /** Pavement where open ground is (never over road, water or a building). */
  const pave = (x: number, y: number) => {
    const t = at(x, y);
    if (t === K.Grass || t === K.Park || t === K.Field || t === K.Plaza) set(K.Pavement, x, y);
  };
  const props: SceneProp[] = [];
  const prop = (kind: string, x: number, y: number, variant = 0, blocks = false) => {
    if (inside(x, y)) props.push({ kind, x, y, variant, blocks });
  };
  const ends: EndScene["streetEnds"] = [];

  // ---- avenues: three east–west (top, middle, bottom of the diagonal band), three north–south between them
  const avenueEW = (y: number) => {
    rect(K.Pavement, 0, y, W, 1);
    rect(K.Road, 0, y + 1, W, 2);
    rect(K.Pavement, 0, y + 3, W, 1);
  };
  const avenueNS = (x: number, y0: number, y1: number) => {
    rect(K.Pavement, x, y0, 1, y1 - y0 + 1);
    rect(K.Road, x + 1, y0, 2, y1 - y0 + 1);
    rect(K.Pavement, x + 3, y0, 1, y1 - y0 + 1);
  };
  avenueNS(0, 26, 62);
  avenueNS(38, 26, 62);
  avenueNS(76, 26, 62);
  avenueEW(0);
  avenueEW(24);
  avenueEW(61);
  avenueEW(89);
  // Where the north–south avenues meet the east–west ones: the carriageways join.
  for (const x of [1, 2, 39, 40, 77, 78]) for (const y of [27, 61]) set(K.Road, x, y);

  // ---------------------------------------------------------------- contexts shared by a pair of ends
  /** What lies between two ends of a kind facing each other (cells x, y, w, h), and the line the pair runs on. */
  const context = (kind: EndKind, x: number, y: number, w: number, h: number, vertical: boolean) => {
    switch (kind) {
      case "bollards":
        // A green with a footpath through it, from one end to the other.
        rect(K.Park, x, y, w, h);
        break;
      case "plaza":
        rect(K.Plaza, x, y, w, h);
        break;
      case "field_gate":
        rect(K.Field, x, y, w, h);
        break;
      case "railing":
        // A basin: water reaching both ends, a stone quay along its other sides.
        rect(K.Quay, x, y, w, h);
        rect(K.Water, x + (vertical ? 1 : 0), y + (vertical ? 0 : 1), w - (vertical ? 2 : 0), h - (vertical ? 0 : 2));
        break;
      default:
        break;
    }
  };

  // ---------------------------------------------------------------- orthogonal ends
  /**
   * A street of `r` road cells from an avenue (local a = along, from the
   * avenue; b = across, 0 at its first road cell) running `len` cells, then
   * its end. Returns the cells of the end's last row (where the props stand).
   */
  const orthEnd = (kind: EndKind, facing: "n" | "e" | "s" | "w", bx: number, by: number, r: number, len: number, opt: { lShape?: boolean; second?: boolean } = {}) => {
    const map = (a: number, b: number): [number, number] => (facing === "n" ? [bx + b, by - a] : facing === "s" ? [bx + b, by + a] : facing === "e" ? [bx + a, by + b] : [bx - a, by + b]);
    const put = (t: number, a: number, b: number) => {
      const [x, y] = map(a, b);
      set(t, x, y);
    };
    const paveAt = (a: number, b: number) => {
      const [x, y] = map(a, b);
      pave(x, y);
    };
    // Where the street leaves the avenue, the avenue's pavement gives way to the carriageway.
    for (let b = 0; b < r; b++) put(K.Road, -1, b);
    for (let a = 0; a < len; a++) {
      put(K.Pavement, a, -1);
      put(K.Pavement, a, r);
      for (let b = 0; b < r; b++) put(K.Road, a, b);
    }
    let cells: [number, number][] = [];
    let a0 = len;
    let a1 = len;
    let b0 = 0;
    let b1 = r - 1;
    if (kind === "turning_circle") {
      // The bulb: (r + 2)² road cells, a pavement round it.
      a1 = len + r + 1;
      b0 = -1;
      b1 = r;
      for (let a = a0 - 1; a <= a1 + 1; a++) for (let b = b0 - 1; b <= b1 + 1; b++) paveAt(a, b);
    } else if (kind === "hammerhead") {
      // A T (an arm of 2 each side) or an L (3 to one side), as deep as the street is wide.
      a1 = len + Math.max(2, r - 1) - 1;
      b0 = opt.lShape ? 0 : -2;
      b1 = opt.lShape ? r + 2 : r + 1;
      for (let a = a0 - 1; a <= a1 + 1; a++) for (let b = b0 - 1; b <= b1 + 1; b++) paveAt(a, b);
    } else {
      put(K.Pavement, len, -1);
      put(K.Pavement, len, r);
    }
    for (let a = a0; a <= a1; a++) for (let b = b0; b <= b1; b++) put(K.Road, a, b);
    for (let b = 0; b < r; b++) cells.push(map(a1, b));
    const [xa, ya] = map(a0, b0);
    const [xb, yb] = map(a1, b1);
    ends.push({ kind, facing, x: Math.min(xa, xb), y: Math.min(ya, yb), w: Math.abs(xb - xa) + 1, h: Math.abs(yb - ya) + 1 });
    // What closes the end: bollards across it, a gate on the track's cell, a railing along the water.
    if (kind === "bollards") for (const [x, y] of cells) prop("bollards", x, y, endPropVariant(facing, opt.second));
    if (kind === "field_gate") {
      const g = cells[(r - 1) >> 1] as [number, number];
      prop("field_gate", g[0], g[1], endPropVariant(facing, opt.second), true);
      cells = [g];
    }
    if (kind === "railing") {
      for (let b = -1; b <= r; b++) {
        const [x, y] = map(a1, b);
        prop("railing", x, y, endPropVariant(facing));
      }
    }
    // A lamp on each pavement near the end.
    const [lx, ly] = map(Math.max(0, len - 1), kind === "hammerhead" || kind === "turning_circle" ? -1 : r);
    prop("lamp", lx, ly);
    return { cells, a1, map };
  };

  /** A track or footpath from one end's cell straight on to the other's (cells between, exclusive). */
  const link = (t: number, x0: number, y0: number, x1: number, y1: number) => {
    const sx = Math.sign(x1 - x0);
    const sy = Math.sign(y1 - y0);
    let x = x0 + sx;
    let y = y0 + sy;
    while (x !== x1 || y !== y1) {
      set(t, x, y);
      if (x !== x1) x += sx;
      if (y !== y1) y += sy;
    }
  };

  // ---- band A (rows 4–23): streets south from the top avenue and north from the middle one, a column per kind
  END_KINDS.forEach((kind, k) => {
    const x0 = 4 + k * 12;
    const rs = endWidth(kind, "s");
    const rn = endWidth(kind, "n");
    context(kind, x0, 7, 11, 14, true);
    const bxS = x0 + 3;
    const bxN = x0 + 3 + Math.max(0, rs - rn);
    const s = orthEnd(kind, "s", bxS, 4, rs, 2, { lShape: kind === "hammerhead" });
    const n = orthEnd(kind, "n", bxN, 23, rn, 2, { second: true });
    joinPair(kind, s.cells, n.cells, true);
  });

  // ---- band B (rows 28–60): streets east and west from the north–south avenues, two pairs a slot
  END_KINDS.forEach((kind, k) => {
    const slot = Math.floor(k / 2);
    const pair = k % 2;
    const y0 = 28 + slot * 11;
    const xL = pair === 0 ? 4 : 42;
    const xR = pair === 0 ? 37 : 75;
    const re = endWidth(kind, "e");
    const rw = endWidth(kind, "w");
    context(kind, xL + 4, y0 + 1, xR - xL - 7, 9, false);
    const e = orthEnd(kind, "e", xL, y0 + 3, re, 3, { second: kind === "bollards" });
    const w = orthEnd(kind, "w", xR, y0 + 3, rw, 3, { lShape: kind === "hammerhead" });
    joinPair(kind, e.cells, w.cells, false);
  });

  /** Between a pair: the footpath (bollards) or the track (field gate) from one end to the other, through the context. */
  function joinPair(kind: EndKind, a: [number, number][], b: [number, number][], vertical: boolean) {
    if (kind !== "bollards" && kind !== "field_gate") return;
    const t = kind === "bollards" ? K.Path : K.Dirt;
    if (kind === "field_gate") {
      const [ax, ay] = a[0] as [number, number];
      const [bx, by] = b[0] as [number, number];
      // Straight on from each gate, then across to meet.
      if (vertical) {
        const mid = (ay + by) >> 1;
        link(t, ax, ay, ax, mid + 1);
        link(t, bx, by, bx, mid - 1);
        rect(t, Math.min(ax, bx), mid, Math.abs(bx - ax) + 1, 1);
      } else {
        const mid = (ax + bx) >> 1;
        link(t, ax, ay, mid + 1, ay);
        link(t, bx, by, mid - 1, by);
        rect(t, mid, Math.min(ay, by), 1, Math.abs(by - ay) + 1);
      }
      return;
    }
    // Bollards: a path two cells wide from the middle of each end to the other.
    const mA = a[Math.max(0, (a.length >> 1) - 1)] as [number, number];
    const mB = b[Math.max(0, (b.length >> 1) - 1)] as [number, number];
    if (vertical) {
      const mid = (mA[1] + mB[1]) >> 1;
      for (const dx of [0, 1]) {
        link(t, mA[0] + dx, mA[1], mA[0] + dx, mid + 1);
        link(t, mB[0] + dx, mB[1], mB[0] + dx, mid - 1);
      }
      rect(t, Math.min(mA[0], mB[0]), mid, Math.abs(mB[0] - mA[0]) + 2, 1);
    } else {
      const mid = (mA[0] + mB[0]) >> 1;
      for (const dy of [0, 1]) {
        link(t, mA[0], mA[1] + dy, mid + 1, mA[1] + dy);
        link(t, mB[0], mB[1] + dy, mid - 1, mB[1] + dy);
      }
      rect(t, mid, Math.min(mA[1], mB[1]), 1, Math.abs(mB[1] - mA[1]) + 2);
    }
  }

  // ---------------------------------------------------------------- diagonal ends (band C, rows 65–84)
  /**
   * A 45° street as the generator lays it: per row, `r` road cells and a
   * pavement cell each side, shifted one cell a row; `dir` +1 runs "\" down
   * to the right (facing se), -1 runs "/" up to the right (facing ne). Rows
   * on an avenue become a junction.
   */
  const diagonalEnd = (kind: EndKind, facing: "ne" | "se", x0: number, y0: number, rows: number, r: number, second = false) => {
    const se = facing === "se";
    const axis = se ? AX.DiagonalNW : AX.DiagonalNE;
    const step = se ? 1 : -1;
    const rowAt = (k: number) => y0 + step * k;
    const xsAt = (k: number) => x0 + k;
    const lay = (k: number, withPavement: boolean) => {
      const y = rowAt(k);
      const xs = xsAt(k);
      for (let x = xs - 1; x <= xs + r; x++) {
        if (!inside(x, y)) continue;
        const i = y * W + x;
        const t = terrain[i] as number;
        const onAvenue = t === K.Road || t === K.Crossing || (t === K.Pavement && streetAxis[i] === AX.Orthogonal && (y === 64 || y === 89 || y === 61 || y === 92));
        if (x >= xs && x < xs + r) {
          terrain[i] = K.Road;
          streetAxis[i] = onAvenue ? AX.Junction : axis;
        } else if (withPavement) {
          if (t === K.Road) streetAxis[i] = AX.Junction;
          else if (t === K.Pavement) streetAxis[i] = onAvenue ? AX.Junction : axis;
          else {
            terrain[i] = K.Pavement;
            streetAxis[i] = axis;
          }
        }
      }
    };
    for (let k = 0; k < rows; k++) lay(k, true);
    const last = rows - 1;
    const yl = rowAt(last);
    const xl = xsAt(last);
    // The end line's cells: through the farthest road cell's middle, square to the street, inside the carriageway.
    const lineCells: [number, number][] = [];
    for (let j = 0; j <= (r - 1) >> 1; j++) lineCells.push(se ? [xl + r - 1 - j, yl + j] : [xl + r - 1 - j, yl - j]);
    let rect0: [number, number, number, number] = [xl, yl, r, 1];
    if (kind === "turning_circle") {
      // The bulb: a block of (r + 2)² road cells centred on the street's line one step on, a pavement round it.
      const n = r + 2;
      const bx = xl + (r >> 1);
      const by = se ? yl : yl - n + 1;
      for (let y = by - 1; y <= by + n; y++) for (let x = bx - 1; x <= bx + n; x++) pave(x, y);
      rect(K.Road, bx, by, n, n);
      rect0 = [bx, by, n, n];
    } else if (kind === "hammerhead") {
      // A bar square to the street (a staircase the other way), as deep as the street, two cells past it each side.
      const bars: [number, number][] = [];
      for (let j = -2; j <= 2; j++) for (let d = 0; d < r; d++) bars.push(se ? [xl + r - 1 - j - d + 1, yl + j + 1] : [xl + r - 1 - j - d + 1, yl - j - 1]);
      for (const [x, y] of bars) for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) pave(x + dx, y + dy);
      let xa = 1e9;
      let ya = 1e9;
      let xb = -1e9;
      let yb = -1e9;
      for (const [x, y] of bars) {
        if (!inside(x, y) || at(x, y) === K.Water) continue;
        set(K.Road, x, y);
        xa = Math.min(xa, x);
        ya = Math.min(ya, y);
        xb = Math.max(xb, x);
        yb = Math.max(yb, y);
      }
      rect0 = [xa, ya, xb - xa + 1, yb - ya + 1];
    }
    ends.push({ kind, facing, x: rect0[0], y: rect0[1], w: rect0[2], h: rect0[3] });
    if (kind === "bollards") for (const [x, y] of lineCells) prop("bollards", x, y, endPropVariant(facing, second));
    if (kind === "field_gate") prop("field_gate", (lineCells[lineCells.length - 1] as [number, number])[0], (lineCells[lineCells.length - 1] as [number, number])[1], endPropVariant(facing, second), true);
    prop("lamp", xsAt(Math.max(0, last - 1)) - 1, rowAt(Math.max(0, last - 1)));
    return { lineCells, xl, yl };
  };

  END_KINDS.forEach((kind, k) => {
    const x0 = 2 + k * 13;
    const rse = endWidth(kind, "se");
    const rne = endWidth(kind, "ne");
    // The shared ground to the right of both ends.
    if (kind === "railing") {
      rect(K.Quay, x0 + 7, 70, 6, 14);
      rect(K.Water, x0 + 8, 71, 5, 12);
    } else context(kind, x0 + 6, 70, 7, 14, true);
    const se = diagonalEnd(kind, "se", x0, 64, 6, rse);
    const ne = diagonalEnd(kind, "ne", x0, 89, 6, rne, true);
    if (kind === "bollards" || kind === "field_gate") {
      // On from each end, one cell a step along the street, then down (or up) to meet.
      const t = kind === "bollards" ? K.Path : K.Dirt;
      const a = se.lineCells[se.lineCells.length - 1] as [number, number];
      const b = ne.lineCells[ne.lineCells.length - 1] as [number, number];
      let [x, y] = a;
      for (let i = 0; i < 3; i++) {
        x++;
        y++;
        set(t, x, y);
        if (kind === "bollards") set(t, x + 1, y);
      }
      let [x2, y2] = b;
      for (let i = 0; i < 3; i++) {
        x2++;
        y2--;
        set(t, x2, y2);
        if (kind === "bollards") set(t, x2 + 1, y2);
      }
      rect(t, x, y, kind === "bollards" ? 2 : 1, y2 - y + 1);
    }
    if (kind === "railing") {
      // The railing along the quay's edge in front of each end.
      for (let x = x0 + 7; x <= x0 + 12; x++) {
        prop("railing", x, 70, endPropVariant("s"));
        prop("railing", x, 83, endPropVariant("n"));
      }
    }
  });

  // ---------------------------------------------------------------- buildings, props and people
  const buildings: SceneBuilding[] = [];
  const base = { named: false, home: false, landmark: false, headroom: 16, ends: { left: "gap" as const, right: "gap" as const } };
  let seed = 6100;
  const put = (x: number, y: number, w: number, h: number, kind: string, hood: string, doorX = 1) => {
    for (let yy = y; yy < y + h + 1; yy++) for (let xx = x; xx < x + w; xx++) if (!inside(xx, yy) || at(xx, yy) !== K.Grass) return;
    rect(K.Building, x, y, w, h);
    rect(K.Door, x + doorX, y + h - 1, 1, 1);
    buildings.push({ x, y, state: "built", spec: { ...base, kind, hood, w, h, doorX, storeys: 1, visualSeed: seed++, conditions: [], headroom: 0 } });
  };
  // Houses round the turning heads and along the streets: wherever 2 × 2 of open ground (and a row in front) is left.
  for (let y = 5; y < H - 6; y += 3) {
    for (let x = 1; x < W - 2; x += 3) {
      const near = (t: number) => {
        for (let dy = -2; dy <= 4; dy++) for (let dx = -1; dx <= 2; dx++) if (at(x + dx, y + dy) === t) return true;
        return false;
      };
      if (!near(K.Pavement) || near(K.Water) || near(K.Field) || near(K.Plaza) || near(K.Park) || near(K.Dirt)) continue;
      put(x, y, 2, 2, (x + y) % 3 === 0 ? "terrace" : "house", (x * 7 + y) % 2 ? "suburb" : "inner");
    }
  }
  // The squares and greens: benches, trees, a stall; the fields: nothing (the hedges and gates are the ends').
  END_KINDS.forEach((kind, k) => {
    const x0 = 4 + k * 12;
    if (kind === "plaza") {
      prop("bench", x0 + 1, 13);
      prop("stall", x0 + 8, 14, 2);
      prop("planter", x0 + 2, 16);
      prop("fountain", x0 + 4, 13, 0, true);
    }
    if (kind === "bollards") {
      prop("tree", x0 + 1, 11, 0, true);
      prop("tree", x0 + 9, 16, 1, true);
      prop("bench", x0 + 8, 12);
      prop("lamp", x0 + 2, 15);
    }
  });
  const people = [
    { x: 9, y: 14, seed: 31 },
    { x: 30, y: 13, seed: 32 },
    { x: 54, y: 9, seed: 33 },
    { x: 20, y: 33, seed: 34 },
    { x: 58, y: 44, seed: 35 },
    { x: 33, y: 75, seed: 36 },
    { x: 12, y: 73, seed: 37 },
  ];
  return { width: W, height: H, terrain, elevation, streetAxis, connectors: [], buildings, props, people, streetEnds: ends };
}
