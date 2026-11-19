// The tool's core: colour, hash, images, outline, autotiling, serialisation, PNG.
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { BLOB_MASKS, autotile, canonicalMask, quarterKind } from "../src/core/autotile";
import { deltaE, hexToRgb, lchToRgb, rgbToHex, rgbToLch } from "../src/core/color";
import { stretchRows } from "../src/core/compose";
import { edgeTemplate } from "../src/core/edges";
import { Prng, hash32, rendezvous } from "../src/core/hash";
import { blank, fill, grid, gridFrames, mirrorX, outline } from "../src/core/image";
import { openOutline, orphans, silhouetteIoU } from "../src/core/lint";
import { buildPalette } from "../src/core/palette";
import { fromBase64, packFromJson, packToJson, toBase64 } from "../src/core/serial";
import { decodePng, encodePng } from "../src/node/png";

describe("colour", () => {
  it("round-trips sRGB through OKLCH", () => {
    for (const hex of ["#11131a", "#e8b44c", "#eb6834", "#2a78d6", "#1baf7a", "#f6ede4", "#292420"]) {
      assert.equal(rgbToHex(lchToRgb(rgbToLch(hexToRgb(hex)))), hex);
    }
  });
  it("maps out-of-gamut colours into the gamut by reducing chroma", () => {
    const c = lchToRgb({ L: 0.7, C: 0.5, h: 150 });
    assert.ok(c.every((v) => v >= 0 && v <= 255));
  });
});

describe("palette builder", () => {
  const p = buildPalette({ ramps: [{ name: "r", anchor: "#a64b3a", darker: 2, lighter: 1, shift: 12 }] });
  it("keeps the anchor exactly and orders steps dark to light", () => {
    const r = p.ramps[0];
    assert.ok(r);
    assert.equal(p.colors[r.steps[r.anchorStep] as number], "#a64b3a");
    const L = r.steps.map((i) => rgbToLch(hexToRgb(p.colors[i] as string)).L);
    for (let i = 1; i < L.length; i++) assert.ok((L[i] as number) > (L[i - 1] as number));
  });
  it("shifts hue as well as lightness (shadows toward blue, highlights toward yellow)", () => {
    const r = p.ramps[0];
    assert.ok(r);
    const h = r.steps.map((i) => rgbToLch(hexToRgb(p.colors[i] as string)).h);
    assert.ok((h[0] as number) < (h[r.anchorStep] as number), "the shadow's hue turns from red toward violet");
    assert.ok((h[3] as number) > (h[r.anchorStep] as number), "the highlight's hue turns toward yellow");
  });
  it("gives every ramp an outline colour darker than its darkest step", () => {
    const r = p.ramps[0];
    assert.ok(r);
    const lineL = rgbToLch(hexToRgb(p.colors[r.line] as string)).L;
    assert.ok(lineL < rgbToLch(hexToRgb(p.colors[r.steps[0] as number] as string)).L);
    assert.equal(p.lineOf[r.steps[1] as number], r.line);
  });
  it("merges colours closer than a just-noticeable difference", () => {
    const q = buildPalette({ ramps: [{ name: "a", anchor: "#808080", darker: 0, lighter: 0, line: false }, { name: "b", anchor: "#818181", darker: 0, lighter: 0, line: false }] });
    assert.equal(q.colors.length, 2);
    assert.ok(deltaE([128, 128, 128], [129, 129, 129]) < 0.012);
  });
});

describe("the fixed hash", () => {
  it("matches its test vectors (shared with the engine when appearance moves to Rust)", () => {
    assert.equal(hash32(), 2654435769);
    assert.equal(hash32(0), 297802655);
    assert.equal(hash32(1), 425409545);
    assert.equal(hash32(1, 2, 3), 2884160615);
    assert.equal(hash32("look"), 286011542);
    assert.equal(hash32(4242, "hair", "crop", 0), 1493581465);
    assert.equal(hash32(-1), 2061416233);
  });
  it("rendezvous picks: adding a candidate only takes the picks it now wins", () => {
    const a = [{ id: "crop" }, { id: "long" }, { id: "bun" }];
    const b = [...a, { id: "braids" }];
    for (let s = 0; s < 2000; s++) {
      const before = rendezvous(s, "hair", a).id;
      const after = rendezvous(s, "hair", b).id;
      assert.ok(after === before || after === "braids", `seed ${s}: ${before} became ${after}`);
    }
  });
  it("rendezvous weights are honoured", () => {
    const xs = [{ id: "a", weight: 3 }, { id: "b", weight: 1 }];
    let n = 0;
    for (let s = 0; s < 4000; s++) if (rendezvous(s, "w", xs).id === "a") n++;
    assert.ok(n > 2700 && n < 3300, `a won ${n} of 4000`);
  });
  it("the sampling generator is deterministic", () => {
    const a = new Prng(7);
    const b = new Prng(7);
    for (let i = 0; i < 10; i++) assert.equal(a.next(), b.next());
  });
});

describe("images and text grids", () => {
  it("parses frames side by side and rejects unknown characters", () => {
    const f = gridFrames(`
      ab. .ba
      .a. .a.`);
    assert.deepEqual(f, [
      ["ab.", ".a."],
      [".ba", ".a."],
    ]);
    assert.throws(() => grid("az", { a: 1 }), /unknown grid character/);
    assert.throws(() => grid("aa\na", { a: 1 }), /ragged/);
  });
  it("mirrors left to right", () => {
    const g = grid("ab.", { a: 1, b: 2 });
    assert.deepEqual([...mirrorX(g).px], [0, 2, 1]);
  });
  it("stretches a drawing by repeating or removing its stretch row", () => {
    const g = grid("a\nb\nc", { a: 1, b: 2, c: 3 });
    assert.deepEqual([...stretchRows(g, 1, 1).px], [1, 2, 2, 3]);
    assert.deepEqual([...stretchRows(g, 1, -1).px], [1, 3]);
  });
  it("adds a closed 1 px outline in the darkest touching line colour", () => {
    const img = blank(5, 5);
    fill(img, 1, 1, 3, 3, 1);
    img.px[2 * 5 + 2] = 2;
    const lineOf = [0, 3, 4, 3, 4];
    const L = [1000, 500, 400, 100, 50];
    const o = outline(img, lineOf, L);
    assert.equal(openOutline(o, (v) => v === 3 || v === 4), 0);
    assert.equal(o.px[0], 0, "corners stay empty (side neighbours only)");
    assert.equal(o.px[1 * 5 + 0], 3);
  });
  it("counts orphan pixels and silhouette overlap", () => {
    const g = grid("a.a\n...\naab", { a: 1, b: 2 });
    assert.equal(orphans(g), 3);
    assert.equal(orphans(g, new Set([2])), 2);
    assert.equal(silhouetteIoU(g, g), 1);
  });
});

describe("autotiling", () => {
  it("has the 47 blob tiles", () => assert.equal(BLOB_MASKS.length, 47));
  it("ignores corners whose sides are not both joined", () => {
    assert.equal(canonicalMask(16), 0);
    assert.equal(canonicalMask(1 | 2 | 16), 1 | 2 | 16);
    assert.equal(quarterKind(1 | 8, 0, 0), "inner");
    assert.equal(quarterKind(1 | 8 | 128, 0, 0), "fill");
    assert.equal(quarterKind(0, 0, 0), "outer");
  });
  it("expands a fill and edge profiles into a template whose tiles meet seamlessly", () => {
    const fillImg = blank(16, 16);
    fill(fillImg, 0, 0, 16, 16, 1);
    const t = edgeTemplate(fillImg, { N: [2], S: [3, 3], W: [4], E: [5] });
    const full = autotile(t, 255);
    assert.ok([...full.px].every((v) => v === 1), "an inside tile is pure fill");
    const north = autotile(t, 255 & ~1 & ~16 & ~128);
    assert.ok([...north.px.subarray(0, 16)].every((v) => v === 2), "the top edge carries the north profile");
    // Two horizontally adjacent edge tiles: the right column of one meets the left of the next in the same profile rows.
    const a = autotile(t, 2 | 8 | 4 | 32 | 64);
    for (let y = 0; y < 16; y++) assert.equal(a.px[y * 16 + 15], a.px[y * 16 + 0]);
  });
});

describe("serialisation and PNG", () => {
  it("round-trips bytes through base64 and packs through JSON", () => {
    for (const n of [0, 1, 2, 3, 4, 255, 1000]) {
      const u = new Uint8Array(n).map((_, i) => (i * 37) & 255);
      assert.deepEqual(fromBase64(toBase64(u)), u);
    }
    const v = { a: new Uint8Array([1, 2, 3]), b: [{ c: new Uint8Array([9]) }] };
    assert.deepEqual(packFromJson(packToJson(v)), v);
  });
  it("encodes PNGs deterministically and decodes them back", () => {
    const img = { w: 3, h: 2, data: new Uint8Array(24).map((_, i) => (i * 11) & 255) };
    const a = encodePng(img);
    assert.deepEqual(encodePng(img), a);
    assert.deepEqual(decodePng(a), img);
  });
});
