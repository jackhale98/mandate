// Autotile templates from a fill and four edge profiles (§2.5: "5–6
// hand-drawn templates expanded"): the artist draws the seamless fill and
// says what each edge looks like, outside in (a kerb: light top, dark face),
// and this assembles the 32×48 template that ./autotile reads, including
// outer corners (both edges) and inner corners (the edges wrapped round the
// missing diagonal neighbour).
import { blank, tileFill, type Indexed } from "./image";

/** Codes from the outermost row or column inward (0 leaves the fill). */
export interface EdgeProfile {
  N: readonly number[];
  S: readonly number[];
  W: readonly number[];
  E: readonly number[];
}

export function edgeTemplate(fill: Indexed, e: EdgeProfile): Indexed {
  const t = blank(32, 48);
  tileFill(t, fill, 0, 0, 32, 48);
  const put = (x: number, y: number, v: number) => {
    if (v !== 0 && x >= 0 && y >= 0 && x < 32 && y < 48) t.px[y * 32 + x] = v;
  };
  // The 32×48 block's 32×32 part (rows 16..47): sides first, then top and bottom win the corners.
  e.W.forEach((v, k) => {
    for (let y = 16; y < 48; y++) put(k, y, v);
  });
  e.E.forEach((v, k) => {
    for (let y = 16; y < 48; y++) put(31 - k, y, v);
  });
  e.N.forEach((v, k) => {
    for (let x = 0; x < 32; x++) put(x, 16 + k, v);
  });
  e.S.forEach((v, k) => {
    for (let x = 0; x < 32; x++) put(x, 47 - k, v);
  });
  // Inner corners (tile (1,0): x 16..31, y 0..15): the edge wraps round the
  // missing diagonal neighbour's corner. A pixel's depth is its Chebyshev
  // distance from that corner; it takes the top/bottom profile where it is
  // further along the row than down the column, else the side profile.
  const corner = (cx: number, cy: number, sx: 1 | -1, sy: 1 | -1, vert: readonly number[], horiz: readonly number[]) => {
    const n = Math.max(vert.length, horiz.length);
    for (let dy = 0; dy < n; dy++) {
      for (let dx = 0; dx < n; dx++) {
        const k = Math.max(dx, dy);
        const prof = dx > dy ? horiz : vert;
        if (k < prof.length) put(cx + sx * dx, cy + sy * dy, prof[k] as number);
      }
    }
  };
  corner(16, 0, 1, 1, e.W, e.N);
  corner(31, 0, -1, 1, e.E, e.N);
  corner(16, 15, 1, -1, e.W, e.S);
  corner(31, 15, -1, -1, e.E, e.S);
  return t;
}
