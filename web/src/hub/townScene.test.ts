import { buildGrid, cellCentre, clearSegment, planRoute, type Grid, type Point } from "./pathfinding";
import { FIXTURES, fixtureTown, randomTown } from "./testTowns";
import { Terrain, buildingOfPlace, surfaceOfTerrain, townGrid, ROOF_ROWS, type TownMap } from "./townLayout";
import { ROUTE_SLACK, STEER_RADIUS, STEER_RUN, TownScene } from "./townScene";

// Layout v2 plans take tens of ms each; the route test plans all 13 real towns.
vi.setConfig({ testTimeout: 120_000 });

/** Signed heading changes along a polyline (radians), skipping changes under half a degree. */
function turnsOf(points: readonly Point[]): { angle: number; before: number; after: number; at: number }[] {
  const out: { angle: number; before: number; after: number; at: number }[] = [];
  for (let i = 1; i < points.length - 1; i++) {
    const a = points[i - 1] as Point;
    const b = points[i] as Point;
    const c = points[i + 1] as Point;
    let d = Math.atan2(c.y - b.y, c.x - b.x) - Math.atan2(b.y - a.y, b.x - a.x);
    while (d > Math.PI) d -= 2 * Math.PI;
    while (d < -Math.PI) d += 2 * Math.PI;
    if (Math.abs(d) > (0.5 * Math.PI) / 180) out.push({ angle: d, before: Math.hypot(b.x - a.x, b.y - a.y), after: Math.hypot(c.x - b.x, c.y - b.y), at: i });
  }
  return out;
}

/**
 * Unforced zig-zags (DEC-080: "no more turns than the obstacles force"): two
 * turns in opposite directions around a short leg (under two tiles: a
 * staircase step or a wiggle) where the walker would also fit along a
 * straight line past the leg, so no obstacle made the jog. A jog into a
 * one-tile alley round a fountain is forced; a step round a zebra is not.
 */
function zigzags(grid: Grid, points: readonly Point[]): number {
  const t = turnsOf(points);
  let n = 0;
  for (let i = 1; i < t.length; i++) {
    const a = t[i - 1] as { angle: number; after: number; at: number };
    const b = t[i] as { angle: number };
    if (Math.sign(a.angle) === Math.sign(b.angle) || a.after >= 2) continue;
    // The short leg runs from points[a.at] to points[a.at + 1].
    const p0 = points[a.at - 1] as Point;
    const p1 = points[a.at] as Point;
    const p2 = points[a.at + 1] as Point;
    const p3 = points[a.at + 2] as Point;
    if (clearSegment(grid, p0, p2, 0.25) || clearSegment(grid, p1, p3, 0.25)) n++;
  }
  return n;
}

/**
 * The best a walker could possibly do: the shortest walkable line, ignoring
 * pedestrian preferences (every walkable surface costs the same).
 */
function bestLength(map: TownMap, from: Point, goal: number): number {
  const flat = buildGrid(map.width, map.height, (x, y) => (map.solid[y * map.width + x] ? "blocked" : surfaceOfTerrain(map.terrain[y * map.width + x] as number) === "blocked" ? "blocked" : "pavement"));
  return planRoute(flat, from, goal)?.length ?? Infinity;
}

describe("routes in generated towns (DEC-080: obviously direct)", () => {
  it("routes between doors are within 1.2x of the best possible path, and almost never jog", () => {
    let pairs = 0;
    let worst = 0;
    const jogs: string[] = [];
    for (let i = 0; i < FIXTURES.length; i++) {
      const { map } = fixtureTown(i);
      const grid = townGrid(map);
      const doors = map.buildings.filter((b) => b.place !== null || b.home);
      for (let a0 = 0; a0 < doors.length; a0++) {
        for (let j = a0 + 1; j < doors.length; j += 2) {
          const a = doors[a0] as (typeof doors)[number];
          const b = doors[j] as (typeof doors)[number];
          const from = cellCentre(grid, a.front);
          const r = planRoute(grid, from, b.front, 0.25, ROUTE_SLACK);
          expect(r).not.toBeNull();
          if (!r) continue;
          const best = bestLength(map, from, b.front);
          // Within 1.2x of the best, plus a tile of slack for very short hops.
          expect(r.length).toBeLessThanOrEqual(1.2 * best + 1);
          if (zigzags(grid, r.points)) jogs.push(`${map.municipality}: ${a.kind} → ${b.kind}: ${r.points.map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(" ")}`);
          if (best >= 8) worst = Math.max(worst, r.length / best);
          pairs++;
        }
      }
    }
    expect(pairs).toBeGreaterThan(700);
    // Routes of 8+ tiles: strictly within 1.2x of the best possible path.
    expect(worst).toBeLessThanOrEqual(1.2);
    // Layout v2 leaves a few one-cell jogs (about 5% of door-to-door routes on the 13 real towns, from
    // 6% before `dejog` folded short legs): most where a 45° street's one-cell pavement band steps
    // sideways at a junction (TOWN_GENERATION.md "Street rework", not yet: two pavement cells per
    // row), the rest half-tile steps round a building's corner. Lower this bound as those are fixed.
    expect(jogs.length, jogs.slice(0, 5).join("\n")).toBeLessThanOrEqual(Math.ceil(pairs * 0.06));
    console.info(`door-to-door routes: ${pairs} pairs, worst length ratio ${worst.toFixed(3)}, ${jogs.length} with a jog`);
  });

  it("tapping a building, or its roof, walks to its door", () => {
    for (let i = 0; i < 3; i++) {
      const { map, fixture: f } = fixtureTown(i);
      const scene = new TownScene(map, { townsfolk: 0 });
      for (const p of f.input.places.slice(0, 6)) {
        const b = buildingOfPlace(map, p.id);
        if (!b) throw new Error("missing building");
        for (const tap of [
          { x: b.x + b.w / 2, y: b.y + b.h / 2 },
          { x: b.x + 0.2, y: b.y - ROOF_ROWS + 0.3 },
        ]) {
          const t = scene.tap(tap);
          expect(t).toEqual({ kind: "door", building: b.index, cell: b.front });
          expect(scene.target).toEqual({ kind: "building", building: b.index, cell: b.front });
        }
        // Walk there and arrive on the doorstep.
        let arrived = false;
        scene.goToBuilding(b.index, () => (arrived = true));
        for (let k = 0; k < 2000 && !arrived; k++) scene.step(1 / 30);
        expect(arrived).toBe(true);
        expect(scene.playerCell).toBe(b.front);
        expect(scene.atBuilding).toBe(b.index);
      }
    }
  });

  it("tapping open ground walks there; the place list does the same as tapping a building", () => {
    const { map, fixture: f } = fixtureTown(1);
    const a = new TownScene(map, { townsfolk: 0 });
    const b = new TownScene(map, { townsfolk: 0 });
    const p = f.input.places[2] as (typeof f.input.places)[number];
    const bi = buildingOfPlace(map, p.id);
    if (!bi) throw new Error("missing building");
    a.tap({ x: bi.x + 1, y: bi.y + 1 });
    b.goToLocation({ type: "place", place: p.id });
    expect(a.target).toEqual(b.target);
    expect(a.player.walk?.points).toEqual(b.player.walk?.points);
    // Ground: the square.
    const sq = map.square;
    if (!sq) throw new Error("no square");
    const t = a.tap({ x: sq.x + 1.5, y: sq.y + 1.5 });
    expect(t).toEqual({ kind: "ground", cell: (sq.y + 1) * map.width + sq.x + 1 });
  });

  it("townsfolk wander deterministically and stay on walkable ground", () => {
    const { map } = randomTown(5);
    const run = () => {
      const s = new TownScene(map, { townsfolk: 8 });
      for (let k = 0; k < 600; k++) {
        s.step(1 / 30);
        for (const w of s.folk) expect(map.solid[Math.floor(w.y) * map.width + Math.floor(w.x)]).toBe(0);
      }
      return s.folk.map((w) => [w.x, w.y]);
    };
    expect(run()).toEqual(run());
  });
});

/** A bare town for steering: `rows` of "#" (solid) and "." (pavement); the player starts at "@". */
function sketchTown(rows: readonly string[]): TownMap {
  const height = rows.length;
  const width = (rows[0] ?? "").length;
  const terrain = new Uint8Array(width * height);
  const solid = new Uint8Array(width * height);
  let start = 0;
  rows.forEach((row, y) =>
    [...row].forEach((ch, x) => {
      const i = y * width + x;
      terrain[i] = ch === "#" ? Terrain.Building : Terrain.Pavement;
      solid[i] = ch === "#" ? 1 : 0;
      if (ch === "@") start = i;
    }),
  );
  return { seed: 1, width, height, terrain, solid, start, buildings: [] } as unknown as TownMap;
}

const onGround = (s: TownScene) => s.map.solid[s.playerCell] === 0;

describe("keyboard steering (desktop)", () => {
  it("a held direction walks the player; Shift runs; letting go stops", () => {
    const s = new TownScene(sketchTown(["....................", "....................", ".........@..........", "....................", "...................."]), { townsfolk: 0 });
    const x0 = s.player.x;
    s.steer(1, 0);
    for (let k = 0; k < 15; k++) s.step(1 / 60);
    const walked = s.player.x - x0;
    expect(walked).toBeCloseTo(s.player.speed * 0.25, 5);
    expect(s.playerMoving).toBe(true);
    expect(s.player.dir).toBe(3);
    s.steer(-1, 0, true);
    const x1 = s.player.x;
    for (let k = 0; k < 15; k++) s.step(1 / 60);
    expect(x1 - s.player.x).toBeCloseTo(walked * STEER_RUN, 5);
    s.steer(0, 0);
    const x2 = s.player.x;
    s.step(0.1);
    expect(s.player.x).toBe(x2);
    expect(s.playerMoving).toBe(false);
  });

  it("buildings block; held diagonally against a wall the player slides along it", () => {
    const s = new TownScene(sketchTown(["##########", "..........", "....@.....", ".........."]), { townsfolk: 0 });
    s.steer(0, -1);
    for (let k = 0; k < 60; k++) s.step(1 / 60);
    // Stopped by the wall, within its margin.
    expect(s.player.y).toBeGreaterThanOrEqual(1 + STEER_RADIUS - 1e-9);
    expect(s.player.y).toBeLessThan(1 + STEER_RADIUS + 0.11);
    const x0 = s.player.x;
    s.steer(1, -1);
    for (let k = 0; k < 20; k++) s.step(1 / 60);
    expect(s.player.x).toBeGreaterThan(x0 + 0.5);
    expect(onGround(s)).toBe(true);
  });

  it("walks a 45° street: smoothly with a diagonal held, and up its steps with one arrow", () => {
    // A staircase street three cells wide, rising to the right (cells with x + y from 12 to 14).
    const rows: string[] = [];
    for (let y = 0; y < 14; y++) rows.push([...Array(16).keys()].map((x) => (x + y >= 12 && x + y <= 14 ? "." : "#")).join(""));
    const town = sketchTown(rows);
    const start = 10 * 16 + 3;
    // Diagonal keys: straight along the street, every frame, never snagging on a step.
    const a = new TownScene({ ...town, start }, { townsfolk: 0 });
    a.steer(1, -1);
    for (let k = 0; k < 90; k++) {
      const { x, y } = a.player;
      a.step(1 / 60);
      expect(a.player.x).toBeGreaterThan(x);
      expect(a.player.y).toBeLessThan(y);
      expect(a.player.x + a.player.y).toBeCloseTo(14, 9);
    }
    expect(a.player.y).toBeLessThan(10.5 - 4);
    // One arrow: up the street step by step, never back.
    const b = new TownScene({ ...town, start }, { townsfolk: 0 });
    b.steer(0, -1);
    let y = b.player.y;
    for (let k = 0; k < 180; k++) {
      b.step(1 / 60);
      expect(onGround(b)).toBe(true);
      expect(b.player.y).toBeLessThanOrEqual(y + 1e-9);
      y = b.player.y;
    }
    expect(10.5 - b.player.y).toBeGreaterThan(4);
  });

  it("stays on walkable ground in generated towns, and steering cancels a walk", () => {
    const { map } = fixtureTown(2);
    const s = new TownScene(map, { townsfolk: 0 });
    const b = map.buildings.find((x) => x.place !== null);
    if (!b) throw new Error("no place");
    expect(s.goToBuilding(b.index)).toBe(true);
    s.step(0.2);
    s.steer(1, 0);
    expect(s.player.walk).toBeNull();
    expect(s.target).toBeNull();
    const dirs = [[1, 0], [1, 1], [0, 1], [-1, 1], [-1, 0], [-1, -1], [0, -1], [1, -1]] as const;
    let seed = 7;
    for (let k = 0; k < 3000; k++) {
      if (k % 40 === 0) {
        seed = (seed * 1103515245 + 12345) >>> 0;
        const d = dirs[seed % 8] as readonly [number, number];
        s.steer(d[0], d[1], seed % 3 === 0);
      }
      s.step(1 / 30);
      expect(onGround(s)).toBe(true);
    }
  });

  it("knows the place whose door the player stands at", () => {
    const { map } = fixtureTown(1);
    const s = new TownScene(map, { townsfolk: 0 });
    const b = map.buildings.find((x) => x.place !== null);
    if (!b) throw new Error("no place");
    s.placePlayer(b.front);
    const at = s.doorHere;
    expect(at).not.toBeNull();
    expect(map.buildings[at as number]?.front).toBe(b.front);
    // Walking a route, there is no door to enter.
    s.goToCell(map.start);
    if (s.player.walk) expect(s.doorHere).toBeNull();
  });
});
