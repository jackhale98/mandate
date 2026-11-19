// A hand-authored town with 45° streets (round 5; DEC-090 D5 amended), in the
// layout v2 vocabulary: diagonal streets in both directions as staircases of
// cells marked in `streetAxis`, two to four cells a row wide, meeting
// orthogonal streets at T, offset and X junctions; a radial leaving a square
// at its corner; a high street running down to a bridge; a zebra on a
// diagonal and a level crossing where one crosses the railway; wedges at the
// junctions as a green, a paved yard and a small square; square buildings
// stepping along the diagonals; lamps and benches on diagonal pavements. The
// generator emits the same vocabulary; this map does not depend on it. The
// contact sheets render it by day, at dusk and at night; the lints and tests
// draw every cell of it.
import { STREET_AXIS as AX } from "./streets";
import { TERRAIN as K } from "./terrain";
import type { Scene, SceneBuilding, SceneProp } from "./v2scene";

export interface DiagScene extends Scene {
  streetAxis: Uint8Array;
}

export const DIAG_W = 56;
export const DIAG_H = 36;

export function diagScene(): DiagScene {
  const W = DIAG_W;
  const H = DIAG_H;
  const terrain = new Uint8Array(W * H).fill(K.Grass);
  const elevation = new Uint8Array(W * H);
  const streetAxis = new Uint8Array(W * H);
  const inside = (x: number, y: number) => x >= 0 && y >= 0 && x < W && y < H;
  const at = (x: number, y: number) => terrain[y * W + x] as number;
  const rect = (t: number, x: number, y: number, w = 1, h = 1) => {
    for (let yy = y; yy < y + h; yy++) for (let xx = x; xx < x + w; xx++) if (inside(xx, yy)) terrain[yy * W + xx] = t;
  };
  /** An orthogonal street running east–west: pavement, `lanes` rows of road, pavement. */
  const streetEW = (y: number, x0: number, x1: number, lanes = 2) => {
    rect(K.Pavement, x0, y, x1 - x0 + 1, 1);
    rect(K.Road, x0, y + 1, x1 - x0 + 1, lanes);
    rect(K.Pavement, x0, y + lanes + 1, x1 - x0 + 1, 1);
  };
  /** Pavement cells laid by each diagonal, west side then east side, per row (for lamps). */
  const sides: [number, number][][] = [];
  /**
   * A 45° street as the generator lays it: per row, `r` road cells and a
   * pavement cell each side, shifted one cell a row (`dir` +1 runs "\" down to
   * the right, -1 runs "/" down to the left), from row `y0` down. Where it
   * crosses another street the cells become a junction; it never paves over
   * water, bridges or buildings. `crossingRows` (counted from `y0`) are zebras.
   */
  const diagonal = (x0: number, y0: number, rows: number, dir: 1 | -1, r: number, crossingRows: readonly number[] = [], pavements = true) => {
    const axis = dir === 1 ? AX.DiagonalNW : AX.DiagonalNE;
    const pav: [number, number][] = [];
    for (let k = 0; k < rows; k++) {
      const y = y0 + k;
      const xs = x0 + dir * k;
      for (let x = xs - 1; x <= xs + r; x++) {
        if (!inside(x, y)) continue;
        const i = y * W + x;
        const t = at(x, y);
        if (t === K.Water || t === K.Bridge || t === K.Building || t === K.Door) continue;
        const road = x >= xs && x < xs + r;
        const street = t === K.Road || t === K.Crossing || t === K.Pavement || t === K.Plaza;
        if (road) {
          terrain[i] = t === K.Rail || crossingRows.includes(k) ? K.Crossing : K.Road;
          streetAxis[i] = street ? AX.Junction : axis;
        } else if (!pavements) continue;
        else if (t === K.Grass || t === K.Park || t === K.Yard) {
          terrain[i] = K.Pavement;
          streetAxis[i] = axis;
          pav.push([x, y]);
        } else if (t === K.Pavement && streetAxis[i] === AX.Orthogonal) streetAxis[i] = AX.Junction;
      }
    }
    sides.push(pav);
  };

  // ---- the river along the south, a quay road above it
  rect(K.Water, 0, 32, W, 4);
  rect(K.Quay, 0, 31, W, 1);
  streetEW(27, 0, 22);

  // ---- the railway across the west, over a street and into a siding in the goods yard
  rect(K.Rail, 0, 25, 16, 1);

  // ---- Market Street and North Road (orthogonal, east–west)
  streetEW(18, 0, W - 1);
  streetEW(10, 26, W - 1);

  // ---- the square (north-west): paved, edged by pavement
  rect(K.Pavement, 1, 1, 10, 9);
  rect(K.Plaza, 2, 2, 8, 7);
  // A small square in the acute wedge where the radial meets Market Street (west side).
  for (let y = 14; y <= 17; y++) rect(K.Plaza, 10, y, y - 12, 1);

  // ---- greens and yards that the diagonals cut into wedges
  // The acute wedge east of the narrow street, between Market Street and North Road (its west side follows the street).
  for (let y = 14; y <= 17; y++) rect(K.Park, 42 - y, y, y - 8, 1);
  rect(K.Yard, 15, 22, 7, 4); // the paved yard in the V between the southern diagonals (the siding ends in it)

  // ---- diagonals
  // The radial: "\" three cells a row, leaving the square's south-east corner, to Market Street (a Y at the square, a T at the street).
  diagonal(9, 10, 9, 1, 3);
  // A narrow street: "/" two cells a row, from the north edge down to Market Street, across North Road (an X).
  diagonal(40, 0, 18, -1, 2);
  // A wide street: "/" four cells a row, from the north edge down into North Road (a T), with a zebra.
  diagonal(54, 0, 11, -1, 4, [6]);
  // The high street: "\" four cells a row, from Market Street (offset from the radial) down to the bridge, with a zebra.
  diagonal(19, 21, 10, 1, 4, [4]);
  // A back lane: "\\" one cell a row, no pavements, from the square's side down to Market Street.
  diagonal(0, 11, 8, 1, 1, [], false);
  // A street "/" three cells a row, from Market Street down over the railway (a level crossing) to the quay road, with a zebra.
  diagonal(16, 21, 7, -1, 3, [2]);

  // The siding: east of the level crossing into the goods yard (laid after the streets: it takes the pavement it crosses).
  rect(K.Rail, 15, 25, 5, 1);

  // A side street: "\\" two cells a row, from Market Street down to the river's edge.
  diagonal(40, 21, 10, 1, 2);

  // ---- the bridge at the high street's foot
  rect(K.Bridge, 28, 31, 4, 5);
  const connectors: Scene["connectors"] = [
    { kind: "bridge", x: 28, y: 31, w: 4, h: 5, axis: "ns", name: "the Mill Bridge" },
    { kind: "level_crossing", x: 12, y: 25, w: 3, h: 1, axis: "ns", name: "" },
  ];

  // ---- buildings, square, stepping along the diagonals (placed only on open ground)
  const buildings: SceneBuilding[] = [];
  const base = { named: false, home: false, landmark: false, headroom: 16, ends: { left: "gap" as const, right: "gap" as const } };
  let seed = 5100;
  const skipped: string[] = [];
  const put = (x: number, y: number, w: number, h: number, kind: string, hood: string, doorX = 1) => {
    for (let yy = y; yy < y + h; yy++) {
      for (let xx = x; xx < x + w; xx++) {
        if (!inside(xx, yy) || at(xx, yy) !== K.Grass) {
          skipped.push(`${kind} ${x},${y}`);
          return;
        }
      }
    }
    rect(K.Building, x, y, w, h);
    rect(K.Door, x + doorX, y + h - 1, 1, 1);
    buildings.push({ x, y, state: "built", spec: { ...base, kind, hood, w, h, doorX, storeys: h >= 3 ? 2 : 1, visualSeed: seed++, conditions: [], headroom: 0 } });
  };
  // North-east of the radial: a terrace stepping down beside it.
  put(13, 9, 2, 2, "house", "inner");
  put(15, 11, 2, 2, "terrace", "inner");
  put(17, 13, 2, 2, "terrace", "inner");
  // Beside the narrow street.
  put(34, 2, 2, 2, "house", "suburb");
  put(32, 4, 2, 2, "house", "suburb");
  put(30, 6, 2, 2, "shop", "inner");
  put(41, 3, 2, 2, "house", "suburb");
  put(39, 5, 3, 2, "shop", "inner");
  // Along the high street, both sides.
  put(26, 22, 2, 2, "shop", "core");
  put(28, 24, 2, 2, "shop", "core");
  put(30, 26, 2, 2, "terrace", "inner");
  put(32, 28, 3, 2, "warehouse", "industrial");
  put(23, 29, 2, 2, "house", "inner");
  // Beside the side street.
  put(37, 23, 2, 2, "house", "suburb");
  put(40, 26, 2, 2, "house", "suburb");
  put(46, 23, 2, 2, "terrace", "inner");
  put(48, 25, 3, 2, "shop", "inner");
  // Beside the wide street.
  put(45, 2, 3, 2, "shop", "inner");
  put(43, 5, 3, 3, "terrace", "inner");
  put(52, 7, 3, 2, "house", "suburb");
  if (skipped.length) throw new Error(`diagScene: buildings on taken ground: ${skipped.join("; ")}`);

  const props: SceneProp[] = [];
  const prop = (kind: string, x: number, y: number, variant = 0, blocks = false) => props.push({ kind, x, y, variant, blocks });
  // Lamps on diagonal pavements: every fourth row, alternating sides, where the cell is still pavement.
  sides.forEach((pav, si) => {
    pav.forEach(([x, y], k) => {
      if ((y + si) % 4 === 0 && at(x, y) === K.Pavement && (k + y) % 2 === 0) prop("lamp", x, y);
    });
  });
  prop("bench", 28, 16);
  prop("bench", 11, 16);
  prop("bench", 17, 24);
  prop("tree", 29, 15, 0, true);
  prop("tree", 32, 16, 1, true);
  prop("planter", 12, 17);
  prop("planter", 18, 23);

  const people = [
    { x: 14, y: 12, seed: 21 },
    { x: 20, y: 19, seed: 22 },
    { x: 24, y: 25, seed: 23 },
    { x: 12, y: 24, seed: 24 },
    { x: 29, y: 33, seed: 25 },
    { x: 36, y: 8, seed: 26 },
    { x: 6, y: 5, seed: 27 },
  ];

  return { width: W, height: H, terrain, elevation, streetAxis, connectors, buildings, props, people };
}
