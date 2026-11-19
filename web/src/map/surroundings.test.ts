import { extendWorld, paintSurroundings, seedHash, surroundingsMargin } from "./surroundings";

const b64 = (a: Uint8Array) => btoa(String.fromCharCode(...a));

/** A 60×40 grid: open sea on the left third, foreign land on the right, the
 * country in between (reaching the bottom edge), with varied biomes and relief. */
function grid() {
  const w = 60;
  const h = 40;
  const cover = new Uint8Array(w * h);
  const biome = new Uint8Array(w * h);
  const shade = new Uint8Array(w * h);
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = y * w + x;
      cover[i] = x < 20 ? 0 : x < 40 && y > 8 ? 2 : 1;
      biome[i] = cover[i] === 0 ? 0 : 4 + ((x + y) % 6);
      shade[i] = cover[i] === 0 ? 128 : 100 + ((x * 7 + y * 3) % 60);
    }
  }
  return { width: w, height: h, cover, biome, shade, view: { width: w, height: h, cover_b64: b64(cover), biome_b64: b64(biome), shade_b64: b64(shade) } };
}

describe("surroundings", () => {
  it("is deterministic for a seed and differs between seeds", () => {
    const g = grid();
    const a = extendWorld(g.view, "4242");
    const b = extendWorld(g.view, "4242");
    expect(a.cover).toEqual(b.cover);
    expect(a.biome).toEqual(b.biome);
    expect(a.shade).toEqual(b.shade);
    const c = extendWorld(g.view, "777");
    expect(c.cover).not.toEqual(a.cover);
    expect(seedHash("4242")).toBe(seedHash("4242"));
    expect(seedHash("4242")).not.toBe(seedHash("4243"));
  });

  it("copies every real cell unchanged and paints none of them", () => {
    const g = grid();
    const s = extendWorld(g.view, "1");
    const m = s.margin;
    expect(m).toBe(surroundingsMargin(g.width, g.height));
    expect(s.width).toBe(g.width + 2 * m);
    expect(s.height).toBe(g.height + 2 * m);
    const img = paintSurroundings(s);
    for (let y = 0; y < g.height; y++) {
      for (let x = 0; x < g.width; x++) {
        const i = y * g.width + x;
        const I = (y + m) * s.width + x + m;
        expect(s.cover[I]).toBe(g.cover[i]);
        expect(s.biome[I]).toBe(g.biome[i]);
        expect(s.shade[I]).toBe(g.shade[i]);
        expect(img[I * 4 + 3]).toBe(0);
      }
    }
  });

  it("continues land beyond land edges, sea beyond sea edges, and ends in open sea", () => {
    const g = grid();
    const s = extendWorld(g.view, "4242");
    const m = s.margin;
    const at = (x: number, y: number) => s.cover[(y + m) * s.width + x + m];
    // Just beyond the right (foreign) edge: land. Beyond the left (sea) edge: sea.
    let land = 0;
    for (let y = 4; y < g.height - 4; y++) {
      if (at(g.width, y) === 1) land++;
      expect(at(-1, y)).toBe(0);
      expect(at(-3, y)).toBe(0);
    }
    expect(land).toBeGreaterThan((g.height - 8) * 0.8);
    // Only foreign land or sea is generated, never the country or a lake.
    for (let I = 0; I < s.cover.length; I++) {
      const x = (I % s.width) - m;
      const y = Math.floor(I / s.width) - m;
      const outside = x < 0 || y < 0 || x >= g.width || y >= g.height;
      if (outside) expect([0, 1]).toContain(s.cover[I]);
    }
    // The outer band of the ring is all open sea, matching the map's background.
    const band = Math.ceil(m * 0.15);
    const img = paintSurroundings(s);
    for (let Y = 0; Y < s.height; Y++) {
      for (let X = 0; X < s.width; X++) {
        if (X >= band && Y >= band && X < s.width - band && Y < s.height - band) continue;
        const I = Y * s.width + X;
        expect(s.cover[I]).toBe(0);
        expect([img[I * 4], img[I * 4 + 1], img[I * 4 + 2], img[I * 4 + 3]]).toEqual([38, 70, 112, 255]);
      }
    }
  });

  it("draws a border where the country reaches the edge of the grid", () => {
    const g = grid();
    const s = extendWorld(g.view, "4242");
    const img = paintSurroundings(s);
    const m = s.margin;
    // Below the country's stretch of the bottom edge.
    const I = (g.height + m) * s.width + 30 + m;
    expect(s.cover[I]).toBe(1);
    expect([img[I * 4], img[I * 4 + 1], img[I * 4 + 2]]).toEqual([36, 28, 30]);
  });
});
