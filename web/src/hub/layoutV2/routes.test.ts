// Routes on the town map at 45° (owner feedback on seed 632): a route along a
// diagonal street is drawn as one 45° leg, never a staircase of unit steps,
// and no drawn route over the real towns turns more than once in two cells.
import type { Point } from "../pathfinding";
import { buildTownMapScene, corners, routeLength, routeTree, routeTo, alongRoute, viewForInput } from "../townMapScene";
import { StreetAxis, Terrain, townSeed, type TownMap } from "../townLayout";
import { GEO_FIXTURES, type GeoFixture } from "./fixtures";
import { layoutTownV2 } from "./index";
import owner632 from "./owner-seed632.json";

vi.setConfig({ testTimeout: 120_000 });

const REAL = [...GEO_FIXTURES, ...(owner632 as unknown as GeoFixture[])];

/** A bare map: grass, with the given cells paved as road (and their street axis). */
function bareMap(W: number, H: number, road: [number, number, number][]): TownMap {
  const terrain = new Uint8Array(W * H).fill(Terrain.Grass);
  const streetAxis = new Uint8Array(W * H);
  for (const [x, y, a] of road) {
    terrain[y * W + x] = Terrain.Road;
    streetAxis[y * W + x] = a;
  }
  return { width: W, height: H, terrain, solid: new Uint8Array(W * H), streetAxis } as unknown as TownMap;
}

const legKind = (a: Point, b: Point) => (a.x === b.x || a.y === b.y ? "straight" : Math.abs(b.x - a.x) === Math.abs(b.y - a.y) ? "diagonal" : "other");
const cellsOf = (a: Point, b: Point) => Math.max(Math.abs(b.x - a.x), Math.abs(b.y - a.y));

describe("town map routes at 45°", () => {
  it("draws a route along a diagonal street as one 45° leg", () => {
    // A "\\" street three cells wide (each row: the road shifted one cell on), from (2, 2) to (16, 16).
    const road: [number, number, number][] = [];
    for (let i = 2; i <= 16; i++) for (let k = -1; k <= 1; k++) if (i + k >= 0) road.push([i + k, i, StreetAxis.DiagonalNW]);
    const map = bareMap(20, 20, road);
    const r = routeTo(map, routeTree(map, 2 * 20 + 2), 16 * 20 + 16) as Point[];
    expect(r).toEqual([
      { x: 2.5, y: 2.5 },
      { x: 16.5, y: 16.5 },
    ]);
    expect(routeLength(r)).toBeCloseTo(14 * Math.SQRT2, 6);
    expect(alongRoute(r, 7 * Math.SQRT2)).toEqual({ x: 9.5, y: 9.5 });
    // And a "/" street the same.
    const ne: [number, number, number][] = [];
    for (let i = 2; i <= 16; i++) for (let k = -1; k <= 1; k++) ne.push([18 - i + k, i, StreetAxis.DiagonalNE]);
    const m2 = bareMap(20, 20, ne);
    const r2 = routeTo(m2, routeTree(m2, 16 * 20 + 2), 2 * 20 + 16) as Point[];
    expect(r2).toHaveLength(2);
    expect(legKind(r2[0] as Point, r2[1] as Point)).toBe("diagonal");
  });

  it("folds a staircase of alternating unit steps into one 45° leg, keeping real corners", () => {
    const W = 30;
    const at = (x: number, y: number) => y * W + x;
    const map = { width: W } as TownMap;
    // East 3, then E, S, E, S, E, S, then south 3.
    const cells = [at(0, 0), at(1, 0), at(2, 0), at(3, 0), at(4, 0), at(4, 1), at(5, 1), at(5, 2), at(6, 2), at(6, 3), at(6, 4), at(6, 5), at(6, 6)];
    expect(corners(map, cells)).toEqual([
      { x: 0.5, y: 0.5 },
      { x: 3.5, y: 0.5 },
      { x: 6.5, y: 3.5 },
      { x: 6.5, y: 6.5 },
    ]);
  });

  it("over the real towns: every leg is orthogonal or 45°, and no route turns more than once in two cells", () => {
    let diagonal = 0;
    for (const f of REAL) {
      const { map } = layoutTownV2(f.input, f.geo, townSeed(f.seed, f.input.municipality));
      const scene = buildTownMapScene(map, viewForInput(f.input));
      for (const r of scene.routes) {
        for (let i = 1; i < r.length; i++) {
          const k = legKind(r[i - 1] as Point, r[i] as Point);
          expect({ town: f.input.municipality, k }).not.toEqual({ town: f.input.municipality, k: "other" });
          if (k === "diagonal") diagonal++;
        }
        // At most one direction change per two cells along the route...
        let cells = 0;
        for (let i = 1; i < r.length; i++) cells += cellsOf(r[i - 1] as Point, r[i] as Point);
        expect({ town: f.input.municipality, turns: r.length - 2, dense: (r.length - 2) * 2 > cells }).toEqual({ town: f.input.municipality, turns: r.length - 2, dense: false });
        // ...and no staircase or zig-zag: never two one-cell legs in a row (but for the last step onto a doorstep).
        for (let i = 2; i + 1 < r.length; i++) {
          const zig = cellsOf(r[i - 2] as Point, r[i - 1] as Point) === 1 && cellsOf(r[i - 1] as Point, r[i] as Point) === 1;
          expect({ town: f.input.municipality, at: r[i - 1], zig }).toEqual({ town: f.input.municipality, at: r[i - 1], zig: false });
        }
      }
    }
    expect(diagonal).toBeGreaterThan(0);
  });
});
