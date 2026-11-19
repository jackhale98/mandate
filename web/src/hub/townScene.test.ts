import { buildGrid, cellCentre, clearSegment, planRoute, type Grid, type Point } from "./pathfinding";
import { FIXTURES, randomInput } from "./testTowns";
import { buildingOfPlace, layoutTown, surfaceOfTerrain, townGrid, townSeed, ROOF_ROWS, type TownMap } from "./townLayout";
import { ROUTE_SLACK, TownScene } from "./townScene";

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
  it("routes between doors are within 1.2x of the best possible path and never zig-zag", () => {
    let pairs = 0;
    let worst = 0;
    for (const f of FIXTURES.slice(0, 6)) {
      const map = layoutTown(f.input, townSeed(f.seed, f.input.municipality));
      const grid = townGrid(map);
      const doors = map.buildings.filter((b) => b.place !== null || b.home);
      for (let i = 0; i < doors.length; i++) {
        for (let j = i + 1; j < doors.length; j += 2) {
          const a = doors[i] as (typeof doors)[number];
          const b = doors[j] as (typeof doors)[number];
          const from = cellCentre(grid, a.front);
          const r = planRoute(grid, from, b.front, 0.25, ROUTE_SLACK);
          expect(r).not.toBeNull();
          if (!r) continue;
          const best = bestLength(map, from, b.front);
          // Within 1.2x of the best, plus a tile of slack for very short hops.
          expect(r.length).toBeLessThanOrEqual(1.2 * best + 1);
          const z = zigzags(grid, r.points);
          // On failure, show the route.
          expect({ from: a.kind, to: b.kind, zigzags: z, route: z ? r.points.map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(" ") : "" }).toEqual({ from: a.kind, to: b.kind, zigzags: 0, route: "" });
          if (best >= 8) worst = Math.max(worst, r.length / best);
          pairs++;
        }
      }
    }
    expect(pairs).toBeGreaterThan(200);
    // Routes of 8+ tiles: strictly within 1.2x of the best possible path.
    expect(worst).toBeLessThanOrEqual(1.2);
    console.info(`door-to-door routes: ${pairs} pairs, worst length ratio ${worst.toFixed(3)}`);
  });

  it("tapping a building, or its roof, walks to its door", () => {
    for (const f of FIXTURES.slice(0, 3)) {
      const map = layoutTown(f.input, 7);
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
    const f = FIXTURES[1] as (typeof FIXTURES)[number];
    const map = layoutTown(f.input, 9);
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
    const map = layoutTown(randomInput(5), 5);
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
