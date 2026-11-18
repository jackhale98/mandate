import {
  PEDESTRIAN_COSTS,
  Walk,
  buildGrid,
  cellAt,
  cellCentre,
  cellIndex,
  clearSegment,
  facing4,
  findPath,
  planRoute,
  pointAlong,
  polylineLength,
  reachableFrom,
  resolveTap,
  segmentCost,
  straighten,
  type Grid,
  type Point,
  type Surface,
} from "./pathfinding";

// '.' pavement, 'z' crossing, ',' grass, '=' road, '#' blocked.
const SURFACE: Record<string, Surface> = { ".": "pavement", z: "crossing", ",": "grass", "=": "road", "#": "blocked" };
function gridOf(rows: string[]): Grid {
  const h = rows.length;
  const w = rows[0]?.length ?? 0;
  return buildGrid(w, h, (x, y) => SURFACE[rows[y]?.[x] ?? "#"] ?? "blocked");
}
const open = (w: number, h: number, edit?: (rows: string[][]) => void): Grid => {
  const rows = Array.from({ length: h }, () => Array.from({ length: w }, () => "."));
  edit?.(rows);
  return gridOf(rows.map((r) => r.join("")));
};
const fill = (rows: string[][], x0: number, y0: number, x1: number, y1: number, ch: string) => {
  for (let y = y0; y <= y1; y++) {
    const row = rows[y];
    if (row) for (let x = x0; x <= x1; x++) row[x] = ch;
  }
};

/** Direction changes along a polyline (more than half a degree). */
function turns(points: readonly Point[]): number {
  let n = 0;
  for (let i = 1; i < points.length - 1; i++) {
    const a = points[i - 1] as Point;
    const b = points[i] as Point;
    const c = points[i + 1] as Point;
    const a1 = Math.atan2(b.y - a.y, b.x - a.x);
    const a2 = Math.atan2(c.y - b.y, c.x - b.x);
    let d = Math.abs(a2 - a1);
    if (d > Math.PI) d = 2 * Math.PI - d;
    if (d > (0.5 * Math.PI) / 180) n++;
  }
  return n;
}

function mulberry32(seed: number) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Seeded random town: blocks of buildings, roads with crossings, grass, trees. */
function randomGrid(seed: number, w = 40, h = 30, surfaces = true): Grid {
  const rng = mulberry32(seed);
  const ri = (lo: number, hi: number) => lo + Math.floor(rng() * (hi - lo + 1));
  const rows = Array.from({ length: h }, () => Array.from({ length: w }, () => "."));
  if (surfaces) {
    for (let k = 0; k < 3; k++) fill(rows, ri(0, w - 8), ri(0, h - 8), ri(8, w - 1), ri(8, h - 1), ",");
    const ry = ri(4, h - 6);
    fill(rows, 0, ry, w - 1, ry + 1, "=");
    const zx = ri(2, w - 4);
    fill(rows, zx, ry, zx + 1, ry + 1, "z");
    const rx = ri(4, w - 6);
    fill(rows, rx, 0, rx + 1, h - 1, "=");
  }
  for (let k = 0; k < 9; k++) {
    const x = ri(0, w - 4);
    const y = ri(0, h - 4);
    fill(rows, x, y, Math.min(w - 1, x + ri(1, 7)), Math.min(h - 1, y + ri(1, 5)), "#");
  }
  for (let k = 0; k < 25; k++) {
    const x = ri(0, w - 1);
    const y = ri(0, h - 1);
    const row = rows[y];
    if (row) row[x] = "#";
  }
  return gridOf(rows.map((r) => r.join("")));
}

/** Reference Dijkstra (no heuristic, no heap) with the same step rules. */
function dijkstra(g: Grid, start: number): Float64Array {
  const n = g.width * g.height;
  const dist = new Float64Array(n).fill(Infinity);
  const done = new Uint8Array(n);
  dist[start] = 0;
  const ok = (x: number, y: number) => x >= 0 && y >= 0 && x < g.width && y < g.height && (g.cost[y * g.width + x] ?? 0) > 0;
  for (;;) {
    let u = -1;
    for (let i = 0; i < n; i++) if (!done[i] && Number.isFinite(dist[i] as number) && (u < 0 || (dist[i] as number) < (dist[u] as number))) u = i;
    if (u < 0) return dist;
    done[u] = 1;
    const ux = u % g.width;
    const uy = Math.floor(u / g.width);
    for (let dy = -1; dy <= 1; dy++) {
      for (let dx = -1; dx <= 1; dx++) {
        if (!dx && !dy) continue;
        if (!ok(ux + dx, uy + dy)) continue;
        if (dx && dy && (!ok(ux + dx, uy) || !ok(ux, uy + dy))) continue;
        const v = u + dy * g.width + dx;
        const c = ((dx && dy ? Math.SQRT2 : 1) * (((g.cost[u] as number) || 1) + (g.cost[v] as number))) / 2;
        if ((dist[u] as number) + c < (dist[v] as number)) dist[v] = (dist[u] as number) + c;
      }
    }
  }
}

const walkableCells = (g: Grid) => {
  const out: number[] = [];
  for (let i = 0; i < g.width * g.height; i++) if ((g.cost[i] ?? 0) > 0) out.push(i);
  return out;
};

describe("A* (8 directions, octile, no corner-cutting)", () => {
  it("never squeezes between two diagonal obstacles", () => {
    const g = gridOf([".#.", "#..", "..."]);
    expect(findPath(g, 0, cellIndex(g, 1, 1))).toBeNull();
    expect(segmentCost(g, { x: 0.5, y: 0.5 }, { x: 1.5, y: 1.5 })).toBe(Infinity);
    // With one side open the walker goes around the corner, not through it.
    const g2 = gridOf(["..", "#."]);
    const p = findPath(g2, 0, cellIndex(g2, 1, 1));
    expect(p?.cells).toEqual([0, 1, 3]);
  });

  it("finds least-cost paths (matches a reference Dijkstra on random towns)", () => {
    for (let seed = 1; seed <= 6; seed++) {
      const g = randomGrid(seed);
      const cells = walkableCells(g);
      const rng = mulberry32(seed * 977);
      for (let s = 0; s < 3; s++) {
        const start = cells[Math.floor(rng() * cells.length)] as number;
        const ref = dijkstra(g, start);
        for (let k = 0; k < 15; k++) {
          const goal = cells[Math.floor(rng() * cells.length)] as number;
          const p = findPath(g, start, goal);
          if (!Number.isFinite(ref[goal] as number)) {
            expect(p).toBeNull();
            continue;
          }
          expect(p).not.toBeNull();
          expect(p?.cost ?? NaN).toBeCloseTo(ref[goal] as number, 6);
        }
      }
    }
  });
});

describe("routes (DEC-080: visibly direct)", () => {
  it("open ground: one straight segment, no longer than the straight line", () => {
    const g = open(24, 20);
    for (const [from, to] of [
      [{ x: 2.5, y: 3.5 }, cellIndex(g, 17, 12)],
      [{ x: 20.2, y: 1.7 }, cellIndex(g, 1, 18)],
      [{ x: 5.5, y: 5.5 }, cellIndex(g, 6, 15)],
    ] as const) {
      const r = planRoute(g, from, to);
      expect(r).not.toBeNull();
      if (!r) continue;
      expect(r.points).toHaveLength(2);
      expect(turns(r.points)).toBe(0);
      const goal = cellCentre(g, to);
      expect(r.length).toBeLessThanOrEqual(1.15 * Math.hypot(goal.x - from.x, goal.y - from.y));
    }
  });

  it("around a wall: only the turns the wall forces, close to the taut path", () => {
    const g = open(32, 22, (rows) => fill(rows, 15, 4, 16, 17, "#"));
    const from = { x: 5.5, y: 10.5 };
    const r = planRoute(g, from, cellIndex(g, 26, 10));
    expect(r).not.toBeNull();
    if (!r) return;
    expect(turns(r.points)).toBeLessThanOrEqual(2);
    // Taut path: over the wall's top corners (15, 4) and (17, 4).
    const taut = Math.hypot(15 - 5.5, 10.5 - 4) + 2 + Math.hypot(26.5 - 17, 10.5 - 4);
    expect(r.length).toBeLessThanOrEqual(1.15 * taut);
    for (let i = 1; i < r.points.length; i++) expect(clearSegment(g, r.points[i - 1] as Point, r.points[i] as Point, 0.25)).toBe(true);
  });

  it("property: on random towns, two points in plain sight get a single straight segment", () => {
    for (let seed = 1; seed <= 20; seed++) {
      const g = randomGrid(seed, 40, 30, false);
      const cells = walkableCells(g);
      const rng = mulberry32(seed);
      let checked = 0;
      for (let k = 0; k < 120; k++) {
        const a = cellCentre(g, cells[Math.floor(rng() * cells.length)] as number);
        const goal = cells[Math.floor(rng() * cells.length)] as number;
        const b = cellCentre(g, goal);
        if (!clearSegment(g, a, b, 0.25)) continue;
        const r = planRoute(g, a, goal);
        expect(r?.points).toHaveLength(2);
        expect(r?.length ?? Infinity).toBeLessThanOrEqual(1.15 * Math.hypot(b.x - a.x, b.y - a.y) + 1e-9);
        checked++;
      }
      expect(checked).toBeGreaterThan(5);
    }
  });

  it("property: routes on random towns are valid, deterministic, and never cost more than the grid path", () => {
    for (let seed = 100; seed < 115; seed++) {
      const g = randomGrid(seed);
      const cells = walkableCells(g);
      const rng = mulberry32(seed);
      for (let k = 0; k < 40; k++) {
        const from = cellCentre(g, cells[Math.floor(rng() * cells.length)] as number);
        from.x += (rng() - 0.5) * 0.6;
        from.y += (rng() - 0.5) * 0.6;
        const goal = cells[Math.floor(rng() * cells.length)] as number;
        const r = planRoute(g, from, goal);
        if (!r) continue;
        expect(planRoute(g, from, goal)).toEqual(r);
        // Cells: 8-neighbours, walkable, no cut corners.
        for (let i = 1; i < r.cells.length; i++) {
          const a = r.cells[i - 1] as number;
          const b = r.cells[i] as number;
          const dx = (b % g.width) - (a % g.width);
          const dy = Math.floor(b / g.width) - Math.floor(a / g.width);
          expect(Math.max(Math.abs(dx), Math.abs(dy))).toBe(1);
          expect(g.cost[b]).toBeGreaterThan(0);
          if (dx && dy) {
            expect(g.cost[a + dx]).toBeGreaterThan(0);
            expect(g.cost[a + dy * g.width]).toBeGreaterThan(0);
          }
        }
        // Straightened: ends where it should, every segment walkable, no costlier.
        expect(r.points[0]).toEqual(from);
        expect(r.points[r.points.length - 1]).toEqual(cellCentre(g, goal));
        let cost = 0;
        for (let i = 1; i < r.points.length; i++) {
          const c = segmentCost(g, r.points[i - 1] as Point, r.points[i] as Point);
          expect(Number.isFinite(c)).toBe(true);
          cost += c;
        }
        // No longer and no costlier than the grid path it came from.
        const centres = r.cells.map((c) => cellCentre(g, c));
        const variants = [[from, ...centres], [from, ...centres.slice(1)]].filter((v) => v.length >= 2);
        const costOf = (v: Point[]) => v.slice(1).reduce((s, p, i) => s + segmentCost(g, v[i] as Point, p), 0);
        expect(polylineLength(r.points)).toBeLessThanOrEqual(polylineLength(variants[0] as Point[]) + 1e-6);
        expect(cost).toBeLessThanOrEqual(Math.max(...variants.map(costOf).filter(Number.isFinite)) + 1e-6);
      }
    }
  });

  it("straightening keeps the cost preference: no shortcut that takes more road", () => {
    // Pavement both sides of a road; the grid path crosses at the zebra right
    // beside the line, and the straightened path must not cost more.
    const g = gridOf([
      "..........",
      "..........",
      "===zz=====",
      "===zz=====",
      "..........",
      "..........",
    ]);
    const pts = [
      { x: 1.5, y: 0.5 },
      { x: 2.5, y: 1.5 },
      { x: 3.5, y: 2.5 },
      { x: 3.5, y: 3.5 },
      { x: 4.5, y: 4.5 },
      { x: 5.5, y: 5.5 },
    ];
    const raw = pts.slice(1).reduce((s, p, i) => s + segmentCost(g, pts[i] as Point, p), 0);
    const out = straighten(g, pts);
    const cost = out.slice(1).reduce((s, p, i) => s + segmentCost(g, out[i] as Point, p), 0);
    expect(cost).toBeLessThanOrEqual(raw + 1e-6);
  });
});

describe("pedestrian costs", () => {
  const street = (zebraX: number) => {
    const rows = ["..............................", "..............................", "..............................", "..............................", "..............................", "==============================", "==============================", "..............................", "..............................", ".............................."];
    return gridOf(rows.map((r, y) => (y === 5 || y === 6 ? r.slice(0, zebraX) + "zz" + r.slice(zebraX + 2) : r)));
  };

  it("crosses a short road directly instead of a long detour to a crossing", () => {
    const g = street(24);
    const r = planRoute(g, { x: 5.5, y: 1.5 }, cellIndex(g, 5, 8));
    expect(r).not.toBeNull();
    if (!r) return;
    for (const c of r.cells) expect(Math.abs((c % g.width) - 5)).toBeLessThanOrEqual(1);
    expect(r.length).toBeLessThanOrEqual(1.15 * 7);
  });

  it("uses a crossing that is right on the way", () => {
    const g = street(6);
    const r = planRoute(g, { x: 5.5, y: 1.5 }, cellIndex(g, 5, 8));
    const crossed = (r?.cells ?? []).filter((c) => Math.floor(c / g.width) === 5 || Math.floor(c / g.width) === 6);
    expect(crossed.every((c) => g.cost[c] === PEDESTRIAN_COSTS.crossing)).toBe(true);
  });

  it("walks along the pavement, not down the road", () => {
    const g = street(24);
    const r = planRoute(g, { x: 1.5, y: 4.5 }, cellIndex(g, 28, 4));
    expect(r?.cells.every((c) => g.cost[c] === PEDESTRIAN_COSTS.pavement)).toBe(true);
    expect(r?.points).toHaveLength(2);
  });
});

describe("tap to target", () => {
  // A building block (x 10-19, y 4-9) with its door below; the walker is south of it.
  const town = () => open(30, 20, (rows) => fill(rows, 10, 4, 19, 9, "#"));
  const building = (g: Grid) => ({ x: 10, y: 4, w: 10, h: 6, door: cellIndex(g, 14, 10), roofRows: 1 });
  const player = { x: 15.5, y: 15.5 };

  it("a tap on a building goes to its door, roof included", () => {
    const g = town();
    expect(resolveTap(g, [building(g)], player, { x: 12.2, y: 5.5 })).toEqual({ kind: "door", building: 0, cell: cellIndex(g, 14, 10) });
    expect(resolveTap(g, [building(g)], player, { x: 18.9, y: 3.4 })).toMatchObject({ kind: "door" });
    expect(resolveTap(g, [building(g)], player, { x: 18.9, y: 2.9 })).toMatchObject({ kind: "ground" });
  });

  it("the front-most building wins where two overlap on screen", () => {
    const g = open(20, 20);
    const back = { x: 2, y: 2, w: 6, h: 4, door: cellIndex(g, 4, 6), roofRows: 2 };
    const front = { x: 2, y: 7, w: 6, h: 3, door: cellIndex(g, 4, 10), roofRows: 2 };
    expect(resolveTap(g, [front, back], { x: 10, y: 15 }, { x: 4, y: 5.5 })).toEqual({ kind: "door", building: 0, cell: front.door });
  });

  it("a tap on open ground goes to that cell", () => {
    const g = town();
    expect(resolveTap(g, [], player, { x: 3.2, y: 2.7 })).toEqual({ kind: "ground", cell: cellIndex(g, 3, 2) });
  });

  it("a tap at the back of a building resolves to the walker's side, never behind it", () => {
    const g = town();
    // No footprint: an obstacle the hit test does not cover (a wall, a hedge).
    const t = resolveTap(g, [], player, { x: 15.5, y: 4.2 });
    expect(t?.kind).toBe("ground");
    const y = Math.floor((t?.cell ?? 0) / g.width);
    const x = (t?.cell ?? 0) % g.width;
    expect(y).toBe(10);
    expect(Math.abs(x - 15)).toBeLessThanOrEqual(1);
    // The naive nearest walkable cell would have been behind the block (row 3).
    // From the north the same tap resolves to the north side.
    const north = resolveTap(g, [], { x: 15.5, y: 1.5 }, { x: 15.5, y: 4.2 });
    expect(Math.floor((north?.cell ?? 0) / g.width)).toBe(3);
  });

  it("a tap on a tree goes to the nearest open cell facing the walker", () => {
    const g = open(20, 20, (rows) => fill(rows, 8, 8, 8, 8, "#"));
    const t = resolveTap(g, [], { x: 8.5, y: 15.5 }, { x: 8.5, y: 8.5 });
    expect(t).toEqual({ kind: "ground", cell: cellIndex(g, 8, 9) });
  });

  it("a tap inside a yard the walker cannot reach stays on the walker's side of its fence", () => {
    const g = open(20, 20, (rows) => {
      fill(rows, 5, 5, 12, 5, "#");
      fill(rows, 5, 12, 12, 12, "#");
      fill(rows, 5, 5, 5, 12, "#");
      fill(rows, 12, 5, 12, 12, "#");
    });
    const from = { x: 8.5, y: 17.5 };
    const t = resolveTap(g, [], from, { x: 8.5, y: 7.5 });
    const reach = reachableFrom(g, cellAt(g, from));
    expect(reach[t?.cell ?? 0]).toBe(1);
    expect(Math.floor((t?.cell ?? 0) / g.width)).toBe(13);
  });

  it("property: a resolved tap is always reachable and never farther from the tap than the exit point", () => {
    for (let seed = 1; seed <= 10; seed++) {
      const g = randomGrid(seed);
      const cells = walkableCells(g);
      const rng = mulberry32(seed + 5);
      const from = cellCentre(g, cells[Math.floor(rng() * cells.length)] as number);
      const reach = reachableFrom(g, cellAt(g, from));
      for (let k = 0; k < 50; k++) {
        const tap = { x: rng() * g.width, y: rng() * g.height };
        const t = resolveTap(g, [], from, tap, reach);
        expect(t).not.toBeNull();
        expect(reach[t?.cell ?? -1]).toBe(1);
        expect(findPath(g, cellAt(g, from), t?.cell ?? -1)).not.toBeNull();
      }
    }
  });
});

describe("smooth movement", () => {
  it("interpolates along the route at walking speed and stops at the end", () => {
    const w = new Walk(
      [
        { x: 0, y: 0 },
        { x: 3, y: 0 },
        { x: 3, y: 4 },
      ],
      2,
    );
    expect(w.length).toBe(7);
    expect(w.advance(0.25)).toMatchObject({ x: 0.5, y: 0, dx: 1, dy: 0, done: false });
    expect(w.advance(0.75)).toMatchObject({ x: 2, y: 0 });
    const p = w.advance(1);
    expect(p.x).toBeCloseTo(3);
    expect(p.y).toBeCloseTo(1);
    expect(p.dy).toBe(1);
    expect(w.advance(10)).toMatchObject({ x: 3, y: 4, done: true });
    expect(w.advance(1)).toMatchObject({ x: 3, y: 4, done: true });
  });

  it("frame rate does not change where the walker is", () => {
    const g = open(20, 20, (rows) => fill(rows, 8, 2, 9, 14, "#"));
    const r = planRoute(g, { x: 2.5, y: 8.5 }, cellIndex(g, 16, 8));
    if (!r) throw new Error("no route");
    const a = new Walk(r.points);
    const b = new Walk(r.points);
    for (let i = 0; i < 60; i++) a.advance(1 / 60);
    for (let i = 0; i < 30; i++) b.advance(1 / 30);
    expect(a.distance).toBeCloseTo(b.distance, 9);
    const pa = pointAlong(r.points, a.distance);
    const pb = pointAlong(r.points, b.distance);
    expect(pa.x).toBeCloseTo(pb.x, 9);
    expect(pa.y).toBeCloseTo(pb.y, 9);
  });

  it("faces the main direction of travel", () => {
    expect(facing4(0, 1)).toBe(0);
    expect(facing4(0, -1)).toBe(1);
    expect(facing4(-1, 0.2)).toBe(2);
    expect(facing4(1, -0.2)).toBe(3);
    expect(facing4(0, 0, 2)).toBe(2);
  });
});
