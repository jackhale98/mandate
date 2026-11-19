// Mandate's P0 pack: determinism, the palette's anchors and budget, the
// character, building and prop contracts the renderer relies on (§5.1),
// the lints, and golden images (§7.2) as pinned pixel hashes.
import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { describe, it } from "node:test";
import { hash32 } from "../src/core/hash";
import type { Indexed } from "../src/core/image";
import { packToJson } from "../src/core/serial";
import { lintPack } from "../src/cli/lint";
import { sampleBuildings, sampleRows, varietyBuildings } from "../art/mandate/samples";
import { buildFacade, COPY_SIMILARITY, designOf, signatureOf, similarity, type FacadeSpec } from "../art/mandate/buildings";
import { ANCHOR, composeCharacter, composeSlots, FACINGS, FRAMES, faceLeft, HAIR_SKIN_MIN, hairSteps, lookFromSeed, peopleLines, PLAYER_LOOK, slotMaterials, slotTable } from "../art/mandate/characters";
import { hexToRgb, rgbToLab } from "../src/core/color";
import { HAIR_NAMES } from "../art/mandate/palette";
import { buildPack } from "../art/mandate/pack";
import { SKIN_ANCHORS } from "../art/mandate/palette";
import { groundTile, TERRAINS } from "../art/mandate/tiles";
import { BLOB_MASKS } from "../src/core/autotile";
import { PLOT_STATES, plotStateOf } from "../art/mandate/buildings";
import { FAMILY_SPECS } from "../art/mandate/samples";
import { CONNECTOR_KINDS, TERRAIN, TerrainPainter, tileOf, V1_NAMES, V2_CODES } from "../art/mandate/terrain";
import { allPieces, bridgePieces, railPiece } from "../art/mandate/v2pieces";
import { v2Scene } from "../art/mandate/v2scene";
import { groundImage } from "../src/cli/scene";
import { diagStats } from "../src/cli/lintDiag";
import { diagScene } from "../art/mandate/diagscene";
import { STREET_AXIS } from "../art/mandate/streets";
import { END_FACINGS, END_KINDS, endPropVariant } from "../art/mandate/ends";
import { endScene } from "../art/mandate/endscene";
import { endStats } from "../src/cli/lintEnds";

const pack = buildPack();
const pal = pack.palette;

describe("the pack", () => {
  it("builds identically twice (determinism)", () => {
    assert.equal(packToJson(buildPack()), packToJson(pack));
  });
  it("has at most 128 colours, no pure black or white, and the brand and Monk Skin Tone anchors", () => {
    assert.ok(pal.colors.length - 1 <= 128, `${pal.colors.length - 1} colours`);
    assert.ok(!pal.colors.slice(1).includes("#000000") && !pal.colors.includes("#ffffff"));
    for (const hex of ["#11131a", "#e8b44c", "#eb6834", "#2a78d6", "#1baf7a", ...SKIN_ANCHORS]) assert.ok(pal.colors.includes(hex), hex);
  });
  it("ships lighting every 15 minutes, with noon ungraded in both night looks", () => {
    assert.equal(pack.lighting.entries.length, 96);
    const noon = pack.lighting.entries[48];
    assert.ok(noon);
    assert.deepEqual(noon.mul, [255, 255, 255]);
    assert.deepEqual(noon.moodPalette, pal.colors.map((c, i) => (i === 0 ? "#000000" : c)));
    const night = pack.lighting.entries[90];
    assert.ok(night && night.spriteMul[0] > night.mul[0], "people keep more light than the static layers at night (moods)");
  });
});

describe("characters", () => {
  it("compose 3 facings × 6 frames of 16×24, feet on the anchor row, closed outline", () => {
    const c = composeCharacter(lookFromSeed(3), pal, pack.lightness);
    assert.equal(c.frames.length, 3);
    for (const row of c.frames) {
      assert.equal(row.length, 6);
      for (const img of row) {
        assert.equal(img.w, 16);
        assert.equal(img.h, 24);
        // The soles are on row 22; the outline under them on row 23 (the anchor row).
        const opaque = (y: number) => [...img.px.subarray(y * 16, y * 16 + 16)].some((v) => v !== 0);
        assert.ok(opaque(22) && opaque(ANCHOR.y));
      }
    }
  });
  it("the same seed gives the same person, and different seeds different people", () => {
    const a = composeCharacter(lookFromSeed(11), pal, pack.lightness).frames[0]?.[0] as Indexed;
    const b = composeCharacter(lookFromSeed(11), pal, pack.lightness).frames[0]?.[0] as Indexed;
    const c = composeCharacter(lookFromSeed(12), pal, pack.lightness).frames[0]?.[0] as Indexed;
    assert.deepEqual(a.px, b.px);
    assert.notDeepEqual(a.px, c.px);
  });
  it("are flat: at most two tones (a base and one shade) per material in every frame (round 3)", () => {
    for (let seed = 1; seed <= 60; seed++) {
      const look = lookFromSeed(seed);
      const table = slotTable(look, pal);
      const mat = slotMaterials(look);
      for (const facing of FACINGS) {
        FRAMES.forEach((_, k) => {
          const tones = new Map<string, Set<number>>();
          for (const sl of composeSlots(look, facing, k).px) {
            const m = mat[sl];
            if (sl && m) tones.set(m, (tones.get(m) ?? new Set()).add(table[sl] as number));
          }
          for (const [m, t] of tones) assert.ok(t.size <= 2, `seed ${seed} ${facing} ${k}: ${m} has ${t.size} tones`);
        });
      }
    }
  });
  it("part hair from skin on every skin tone and hair colour (round 3: no blob heads)", () => {
    const lab = pal.colors.map((c) => rgbToLab(hexToRgb(c)));
    for (let skin = 0; skin < 6; skin++) {
      const s = lab[slotTable({ ...PLAYER_LOOK, skin }, pal)[2] as number];
      for (const hc of HAIR_NAMES) {
        const h = lab[hairSteps(pal, skin, hc)[0]];
        assert.ok(s && h);
        assert.ok(Math.hypot(s.L - h.L, s.a - h.a, s.b - h.b) >= HAIR_SKIN_MIN, `skin ${skin + 1}, ${hc}`);
      }
    }
  });
  it("outline each material in a darker colour of its own: soft on light materials, never lighter than the material", () => {
    const lines = peopleLines(pal);
    const L = (i: number) => pack.lightness[i] as number;
    const at = (ramp: string) => pal.ramps.find((r) => r.name === ramp)?.steps[0] as number;
    for (const r of ["skin1", "skin3", "hair_blonde", "hair_grey", "cloth_shirt", "cloth_mustard"]) assert.ok(L(lines[at(r)] as number) > 300, `${r}: the line is near-black`);
    for (let i = 1; i < pal.colors.length; i++) if ((pal.stepOf[i] as number) >= 0) assert.ok(L(lines[i] as number) <= L(i), `${pal.colors[i]}: its line is lighter`);
  });
  it("faces left by mirroring the right-facing side view about the figure's centre column", () => {
    const side = composeCharacter(PLAYER_LOOK, pal, pack.lightness).frames[2]?.[0] as Indexed;
    const left = faceLeft(side);
    for (let y = 0; y < 24; y++) for (let x = 0; x < 15; x++) assert.equal(left.px[y * 16 + x], side.px[y * 16 + (14 - x)]);
  });
});

describe("buildings", () => {
  it("fill their lot plus one roof row and the headroom they are given, with lit openings inside the frame, shadow casters and a receiver map", () => {
    for (const s of [...sampleBuildings(), ...varietyBuildings(), ...sampleRows().flat()]) {
      const f = buildFacade(s, pal);
      assert.equal(f.albedo.w, s.w * 16);
      assert.equal(f.ay, 16 + Math.min(32, s.headroom ?? 0));
      assert.equal(f.albedo.h, s.h * 16 + f.ay);
      assert.ok(f.doors.length >= 1, `${s.kind}: no door`);
      for (const o of [...f.windows, ...f.doors]) {
        assert.ok(o.x >= 0 && o.y >= 0 && o.x + o.w <= f.albedo.w && o.y + o.h <= f.albedo.h, `${s.kind}: opening outside the frame`);
        if (o.emissive) assert.equal(o.emissive.w * o.emissive.h, o.w * o.h);
      }
      assert.ok(f.shadow.height > 0 && f.shadow.footprint.length === 4);
      assert.ok(f.shadows.length >= 1 && f.shadows.every((c) => c.height > 0));
      assert.equal(f.receivers.length, f.albedo.w * f.albedo.h);
      // Every drawn pixel has a receiver class, and nothing undrawn has one; the front wall is a face.
      for (let i = 0; i < f.receivers.length; i++) assert.equal(f.receivers[i] === 0, f.albedo.px[i] === 0);
      assert.ok([...f.receivers].some((v) => v >= 3), `${s.kind}: no wall face`);
    }
  });
  it("vary the skyline: masses of different heights, roofs that are not boxes", () => {
    const tops = varietyBuildings().map((s) => {
      const f = buildFacade(s, pal);
      let top = f.albedo.h;
      for (let i = 0; i < f.albedo.px.length && top === f.albedo.h; i++) if (f.albedo.px[i]) top = Math.floor(i / f.albedo.w);
      return f.albedo.h - top - s.h * 16;
    });
    assert.ok(new Set(tops).size >= 8, `rise above the footprint: ${tops.join(" ")}`);
    const roofs = new Set(varietyBuildings().map((s) => signatureOf(designOf(s))[1]));
    for (const r of ["gable", "front", "hip", "flat", "saw"]) assert.ok(roofs.has(r), `no ${r} roof in the variety set`);
  });
  it("never copy the building on their left", () => {
    for (const row of sampleRows()) {
      for (let i = 1; i < row.length; i++) {
        const sim = similarity(signatureOf(designOf(row[i - 1] as FacadeSpec)), signatureOf(designOf(row[i] as FacadeSpec)));
        assert.ok(sim < COPY_SIMILARITY, `neighbours share ${sim} of their features`);
      }
    }
  });
  it("look the same for the same visual seed and differ between seeds", () => {
    const [a] = sampleBuildings();
    assert.ok(a);
    assert.deepEqual(buildFacade(a, pal).albedo.px, buildFacade({ ...a }, pal).albedo.px);
    assert.notDeepEqual(buildFacade(a, pal).albedo.px, buildFacade({ ...a, visualSeed: a.visualSeed + 1 }, pal).albedo.px);
  });
});

describe("lints", () => {
  const f = lintPack(pack);
  it("palette, outline and crowd-clone lints pass", () => {
    for (const lint of ["palette size", "no pure black or white", "palette conformance", "outline consistency", "crowd clones", "height bands", "silhouette readability", "neighbour similarity", "flatness", "head contrast", "v2 coverage", "connector kits", "plot states", "v2 seams", "diagonal coverage", "diagonal seams", "diagonal isolation", "diagonal palette"]) {
      const bad = f.filter((x) => x.lint === lint && x.level === "error");
      assert.deepEqual(bad, [], lint);
    }
  });
});

describe("layout v2 (round 4)", () => {
  it("keeps at least 10 colours of headroom in the 128 budget (D11)", () => {
    assert.ok(pal.colors.length - 1 <= 118, `${pal.colors.length - 1} colours`);
  });
  it("draws every v2 terrain code from its mask alone (the plain ground path): 16×16, opaque, deterministic", () => {
    for (const code of V2_CODES) {
      for (const m of [0, 1 | 4, 2 | 8, 15, 255, 1 | 2 | 16]) {
        const a = tileOf(pack.tiles, pack.terrain, code, m, 3);
        assert.equal(a.w * a.h, 256);
        assert.ok(a.px.every((v) => v > 0 && v < pal.colors.length), `code ${code} mask ${m}`);
        assert.deepEqual(a.px, tileOf(pack.tiles, pack.terrain, code, m, 3).px);
      }
    }
  });
  it("draws a v1 cell exactly as v1 does where nothing v2 is near it", () => {
    const scene = v2Scene();
    const p = new TerrainPainter(pack.tiles, pack.terrain, scene);
    let simple = 0;
    for (let y = 0; y < scene.height; y++) {
      for (let x = 0; x < scene.width; x++) {
        if (!p.simple(x, y)) continue;
        simple++;
        const t = scene.terrain[y * scene.width + x] as number;
        assert.deepEqual(p.draw(x, y, 5).px, groundTile(pack.tiles, V1_NAMES[t] ?? "grass", p.v1Mask(x, y), 5).px, `cell ${x},${y}`);
      }
    }
    assert.ok(simple > 100, `${simple} simple cells`);
    // A flat map with only v1 codes and no water is simple everywhere.
    const flat = new TerrainPainter(pack.tiles, pack.terrain, { width: 4, height: 3, terrain: new Uint8Array([0, 1, 1, 2, 0, 1, 1, 2, 8, 8, 9, 9]) });
    for (let y = 0; y < 3; y++) for (let x = 0; x < 4; x++) assert.ok(flat.simple(x, y));
  });
  it("never throws on unknown codes, levels out of range or connectors off the map", () => {
    const view = { width: 3, height: 3, terrain: new Uint8Array([31, 255, 17, 22, 40, 20, 5, 5, 99]), elevation: new Uint8Array([9, 0, 2, 3, 200, 1, 0, 0, 0]), connectors: [{ kind: "zeppelin", x: -2, y: 1, w: 9, h: 1, axis: "ew" as const }] };
    const p = new TerrainPainter(pack.tiles, pack.terrain, view);
    for (let y = 0; y < 3; y++) for (let x = 0; x < 3; x++) assert.equal(p.draw(x, y, 0).px.length, 256);
    assert.equal(tileOf(pack.tiles, pack.terrain, 77, 255, 0).px.length, 256);
  });
  it("finishes on bridge cells that reach the map's edge with no connector (seed 672 froze here)", () => {
    // Bridge (22) runs touching the right and bottom edges, no connectors: the run scan used to loop forever.
    const B = 22;
    const view = { width: 4, height: 4, terrain: new Uint8Array([0, 0, B, B, 0, 0, 0, B, 5, 5, 5, B, 0, B, 0, B]) };
    const p = new TerrainPainter(pack.tiles, pack.terrain, view);
    for (let y = 0; y < 4; y++) for (let x = 0; x < 4; x++) assert.equal(p.draw(x, y, 0).px.length, 256);
  });
  it("is deterministic for a map and seamless across cells (creek water, rail heads)", () => {
    for (const piece of allPieces()) assert.deepEqual(groundImage(pack, piece).px, groundImage(pack, piece).px, piece.label);
    const rail = groundImage(pack, railPiece());
    // The straight run at row 4 (cells 6..11): both rail heads (rows 4 and 11 of the cell) unbroken over 6 cells.
    const head = pack.terrain.legend.m as number;
    for (const row of [4 * 16 + 4, 4 * 16 + 11]) for (let x = 7 * 16; x < 11 * 16; x++) assert.equal(rail.px[row * rail.w + x], head, `rail head at ${x},${row}`);
  });
  it("has a kit for every connector kind in both axes, and the kits differ", () => {
    assert.deepEqual([...CONNECTOR_KINDS], ["bridge", "footbridge", "ford", "viaduct", "level_crossing"]);
    const decks = bridgePieces().map((p) => {
      const c = p.connectors[0];
      assert.ok(c);
      const img = groundImage(pack, p);
      const cx = c.x + (c.axis === "ew" ? 1 : 0);
      const cy = c.y + (c.axis === "ns" ? 1 : 0);
      return [...img.px.subarray(cy * 16 * img.w, (cy * 16 + 16) * img.w)].filter((_, i) => Math.floor((i % img.w) / 16) === cx).join(",");
    });
    assert.equal(new Set(decks).size, decks.length, "two kits draw the same deck");
  });
  it("draws every plot state in every family in the built building's frame, dark when vacant", () => {
    assert.equal(plotStateOf([]), "built");
    assert.equal(plotStateOf(["flooded", "derelict"]), "derelict");
    for (const [fam, spec] of FAMILY_SPECS) {
      const built = buildFacade(spec, pal);
      for (const state of PLOT_STATES) {
        const f = buildFacade({ ...spec, conditions: state === "built" ? [] : [state] }, pal);
        assert.equal(f.albedo.w, built.albedo.w, `${fam} ${state}`);
        assert.equal(f.albedo.h, built.albedo.h, `${fam} ${state}`);
        assert.equal(f.ay, built.ay);
        assert.ok(f.doors.length >= 1 && f.shadows.length >= 1 && f.receivers.length === f.albedo.w * f.albedo.h);
        for (let i = 0; i < f.receivers.length; i++) assert.equal(f.receivers[i] === 0, f.albedo.px[i] === 0);
        if (state !== "built") {
          assert.ok([...f.windows, ...f.doors].every((o) => !o.emissive), `${fam} ${state} is lit`);
          assert.notDeepEqual(f.albedo.px, built.albedo.px);
        }
      }
    }
  });
  it("has the layout v2 props: a headframe, three spoil heaps, reeds, stepping stones, and market stalls with round 1's first", () => {
    assert.equal(pack.props.headframe?.length, 1);
    assert.equal(pack.props.spoil_heap?.length, 3);
    assert.ok((pack.props.reeds?.length ?? 0) >= 2 && (pack.props.stepping_stones?.length ?? 0) >= 2);
    assert.equal(pack.props.stall?.length, 6);
    assert.ok((pack.props.headframe?.[0]?.albedo.h ?? 0) >= 56, "a headframe stands tall");
  });
  it("names the terrain codes as the layout does", () => {
    assert.equal(TERRAIN.Creek, 17);
    assert.equal(TERRAIN.Steps, 25);
    assert.deepEqual([...V2_CODES], [17, 18, 19, 20, 21, 22, 23, 24, 25]);
  });
});

/** A grass map with one straight diagonal street: `r` road cells a row, pavements unless `bare`. */
function oneDiagonal(r: number, axis: 1 | 2, bare = false) {
  const W = 24;
  const H = 20;
  const terrain = new Uint8Array(W * H).fill(TERRAIN.Grass);
  const streetAxis = new Uint8Array(W * H);
  for (let y = 0; y < H; y++) {
    const xs = axis === STREET_AXIS.DiagonalNW ? 2 + y : W - 3 - r - y + 1;
    for (let x = xs - 1; x <= xs + r; x++) {
      if (x < 0 || x >= W) continue;
      const road = x >= xs && x < xs + r;
      if (!road && bare) continue;
      terrain[y * W + x] = road ? TERRAIN.Road : TERRAIN.Pavement;
      streetAxis[y * W + x] = axis;
    }
  }
  return { width: W, height: H, terrain, streetAxis };
}

describe("diagonal streets (round 5)", () => {
  const scene = diagScene();
  it("names the street axis codes as the layout does", () => {
    assert.deepEqual({ ...STREET_AXIS }, { Orthogonal: 0, DiagonalNE: 1, DiagonalNW: 2, Junction: 3 });
  });
  it("changes nothing where no street is diagonal: an all-orthogonal axis, and cells away from the diagonals", () => {
    const v2 = v2Scene();
    assert.deepEqual(groundImage(pack, { ...v2, streetAxis: new Uint8Array(v2.width * v2.height) }).px, groundImage(pack, v2).px);
    const p = new TerrainPainter(pack.tiles, pack.terrain, scene);
    const a = groundImage(pack, scene);
    const b = groundImage(pack, { ...scene, streetAxis: undefined });
    let away = 0;
    for (let y = 0; y < scene.height; y++) {
      for (let x = 0; x < scene.width; x++) {
        if (p.streets?.touched(x, y)) continue;
        away++;
        for (let k = 0; k < 16; k++) {
          const i = (y * 16 + k) * a.w + x * 16;
          assert.deepEqual(a.px.subarray(i, i + 16), b.px.subarray(i, i + 16), `cell ${x},${y}`);
        }
      }
    }
    assert.ok(away > 800, `${away} cells away from the diagonals`);
  });
  it("draws every width on both axes as a smooth 45° band, R × 16 px across a row, with its kerbs", () => {
    for (const axis of [1, 2] as const) {
      for (const r of [1, 2, 3, 4, 6]) {
        for (const bare of [false, true]) {
          const view = oneDiagonal(r, axis, bare);
          const st = diagStats(pack, view);
          assert.equal(st.stairs, 0, `axis ${axis} r ${r}: ${st.worstStair}`);
          assert.equal(st.kerbBad, 0, `axis ${axis} r ${r} kerbs`);
          // A bare street one cell a row has no cells side by side (they meet at corners).
          if (r > 1 || !bare) assert.ok(st.edges > 20, `axis ${axis} r ${r}: ${st.edges} edges`);
          if (!bare) assert.ok(st.kerbs > 100, `axis ${axis} r ${r}: ${st.kerbs} kerb pixels`);
          // Across a row in the middle of the map the carriageway is exactly r cells wide (the staircase's area).
          const p = new TerrainPainter(pack.tiles, pack.terrain, view);
          for (const Y of [160, 161, 167, 175]) {
            let w = 0;
            for (let X = 0; X < view.width * 16; X++) if (p.streets?.mat(X, Y) === TERRAIN.Road) w++;
            assert.equal(w, r * 16, `axis ${axis} r ${r} row ${Y}`);
          }
        }
      }
    }
  });
  it("is seamless in the test town: no staircase across a cell edge along a street, the kerbs drawn alike, no new colours", () => {
    const st = diagStats(pack, scene);
    assert.equal(st.stairs, 0, st.worstStair);
    assert.ok(st.edges > 150);
    assert.equal(st.kerbBad, 0);
    assert.ok(st.kerbs > 1000);
    assert.deepEqual(st.newColours, []);
  });
  it("marks the streets: whole centre dashes on carriageways of three or more a row, none on two, zebras on both axes, a level crossing", () => {
    const img = groundImage(pack, scene);
    const L = pack.terrain.legend;
    const W = scene.width;
    const cell = (X: number, Y: number) => (Y >> 4) * W + (X >> 4);
    const nearCrossing = (X: number, Y: number) => {
      for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) if (scene.terrain[((Y >> 4) + dy) * W + (X >> 4) + dx] === TERRAIN.Crossing) return true;
      return false;
    };
    const chalk = (X: number, Y: number) => img.px[Y * img.w + X] === L.M || img.px[Y * img.w + X] === L.m;
    const dash = (X: number, Y: number) => {
      const c = cell(X, Y);
      return chalk(X, Y) && scene.terrain[c] === TERRAIN.Road && (scene.streetAxis[c] === 1 || scene.streetAxis[c] === 2);
    };
    const seen = new Uint8Array(img.w * img.h);
    let dashes = 0;
    for (let Y = 0; Y < img.h; Y++) {
      for (let X = 0; X < img.w; X++) {
        if (seen[Y * img.w + X] || !dash(X, Y)) continue;
        let n = 0;
        let zebra = false;
        const stack: [number, number][] = [[X, Y]];
        seen[Y * img.w + X] = 1;
        while (stack.length) {
          const [x, y] = stack.pop() as [number, number];
          n++;
          zebra ||= nearCrossing(x, y);
          for (let dy = -1; dy <= 1; dy++) {
            for (let dx = -1; dx <= 1; dx++) {
              const nx = x + dx;
              const ny = y + dy;
              if (nx < 0 || ny < 0 || nx >= img.w || ny >= img.h || seen[ny * img.w + nx] || !dash(nx, ny)) continue;
              seen[ny * img.w + nx] = 1;
              stack.push([nx, ny]);
            }
          }
        }
        // Stripes of a zebra (beside a crossing cell) are not dashes.
        if (zebra && n !== 14) continue;
        assert.equal(n, 14, `a broken dash at ${X},${Y}`);
        dashes++;
      }
    }
    assert.ok(dashes >= 8, `${dashes} dashes`);
    // The narrow street (2 a row, north of North Road): no centre line.
    for (let y = 1; y < 9; y++) for (let x = 0; x < W; x++) {
      const i = y * W + x;
      if (scene.streetAxis[i] !== 1 || scene.terrain[i] !== TERRAIN.Road || x > 40) continue;
      for (let k = 0; k < 256; k++) assert.ok(!chalk(x * 16 + (k & 15), y * 16 + (k >> 4)), `a centre line on the narrow street at ${x},${y}`);
    }
    // Zebras on both axes; the level crossing has rails, posts and no zebra.
    const p = new TerrainPainter(pack.tiles, pack.terrain, scene);
    const stripes: Record<number, number> = { 1: 0, 2: 0 };
    let rails = 0;
    let posts = 0;
    for (let y = 0; y < scene.height; y++) {
      for (let x = 0; x < W; x++) {
        const i = y * W + x;
        if (scene.terrain[i] !== TERRAIN.Crossing) continue;
        for (let k = 0; k < 256; k++) {
          const v = img.px[(y * 16 + (k >> 4)) * img.w + x * 16 + (k & 15)];
          if (p.isRail(x, y)) {
            if (v === L.m) rails++;
            if (v === L.R) posts++;
          } else if (v === L.M) stripes[scene.streetAxis[i] as number] = (stripes[scene.streetAxis[i] as number] ?? 0) + 1;
        }
      }
    }
    assert.ok((stripes[1] ?? 0) > 50 && (stripes[2] ?? 0) > 50, JSON.stringify(stripes));
    assert.ok(rails > 30 && posts >= 4, `rails ${rails}, posts ${posts}`);
  });
  it("is deterministic and never throws on odd axes (unknown codes, diagonal marks off streets, tiny maps)", () => {
    assert.deepEqual(groundImage(pack, scene).px, groundImage(pack, diagScene()).px);
    const odd = { width: 4, height: 3, terrain: new Uint8Array([1, 2, 0, 6, 1, 1, 2, 5, 3, 2, 22, 20]), streetAxis: new Uint8Array([7, 255, 1, 2, 3, 1, 2, 2, 1, 3, 2, 1]) };
    const p = new TerrainPainter(pack.tiles, pack.terrain, odd);
    for (let y = 0; y < 3; y++) for (let x = 0; x < 4; x++) assert.equal(p.draw(x, y, 0).px.length, 256);
    const one = new TerrainPainter(pack.tiles, pack.terrain, { width: 1, height: 1, terrain: new Uint8Array([1]), streetAxis: new Uint8Array([2]) });
    assert.equal(one.draw(0, 0, 0).px.length, 256);
  });
});

describe("street ends (round 6)", () => {
  const scene = endScene();
  const painter = new TerrainPainter(pack.tiles, pack.terrain, scene);
  const ends = painter.ends ?? assert.fail("the test town lays out no street ends");
  const img = groundImage(pack, scene);
  const L = pack.terrain.legend;
  const px = (X: number, Y: number) => img.px[Y * img.w + X] as number;
  type Laid = (typeof ends)["ends"][number];
  const u = (e: Laid, X: number, Y: number) => (X + 0.5 - e.ox) * e.dx + (Y + 0.5 - e.oy) * e.dy;
  const v = (e: Laid, X: number, Y: number) => (X + 0.5 - e.ox) * e.ax + (Y + 0.5 - e.oy) * e.ay;
  /** Every pixel of an end's zone, with its frame coordinates. */
  const zone = (e: Laid, f: (X: number, Y: number, uu: number, vv: number) => void) => {
    for (let Y = e.zy0 * 16; Y < e.zy1 * 16; Y++) for (let X = e.zx0 * 16; X < e.zx1 * 16; X++) f(X, Y, u(e, X, Y), v(e, X, Y));
  };
  const laid = (kind: string) => ends.ends.filter((e) => e.kind === kind);

  it("names the kinds and facings as the layout does, and lays out every end of the test town", () => {
    assert.deepEqual([...END_KINDS], ["turning_circle", "hammerhead", "bollards", "plaza", "field_gate", "railing"]);
    assert.deepEqual([...END_FACINGS], ["n", "e", "s", "w", "ne", "nw", "se", "sw"]);
    assert.ok(ends);
    assert.equal(ends.ends.length, scene.streetEnds.length);
    for (const kind of END_KINDS) for (const facing of ["n", "e", "s", "w", "ne", "se"]) assert.ok(scene.streetEnds.some((e) => e.kind === kind && e.facing === facing), `${kind} ${facing}`);
    const widths = new Set(ends.ends.map((e) => Math.round((e.hw * 2) / (e.diag ? 11.31 : 16))));
    assert.deepEqual([...widths].sort(), [2, 3, 4]);
  });

  it("changes nothing without street ends, and nothing away from them", () => {
    for (const s of [v2Scene(), diagScene()]) assert.deepEqual(groundImage(pack, { ...s, streetEnds: [] }).px, groundImage(pack, s).px);
    const st = endStats(pack, scene);
    assert.equal(st.awayChanged, 0);
    assert.ok(st.recorded === st.laidOut && st.laidOut === 36);
  });

  it("is seamless, keeps its kerbs round every bulb and head, and adds no colour", () => {
    const st = endStats(pack, scene);
    assert.equal(st.kerbBad, 0, st.worstKerb);
    assert.ok(st.kerbs > 2000, `${st.kerbs} kerb pairs`);
    assert.equal(st.seams, 0, st.worstSeam);
    assert.equal(st.mismatches, 0, st.worstMismatch);
    assert.deepEqual(st.newColours, []);
  });

  it("draws turning circles round, with an island in the larger ones", () => {
    const circles = laid("turning_circle");
    assert.equal(circles.length, 6);
    for (const e of circles) {
      const cx = e.ox + e.dx * e.uc;
      const cy = e.oy + e.dy * e.uc;
      // Round: carriageway just inside the rim on every side away from the street (and the island's kerb, or the
      // island, at the middle); pavement just outside it, where the generator's square of road cells had its corners.
      for (let k = 0; k < 16; k++) {
        const a = (k * Math.PI) / 8;
        const ca = Math.round(Math.cos(a) * 1000) / 1000;
        const sa = Math.round(Math.sin(a) * 1000) / 1000;
        // Toward the street (behind the bulb) the carriageway runs on: skip that side.
        if (ca * -e.dx + sa * -e.dy > 0.5) continue;
        const inX = Math.floor(cx + ca * (e.R - 3));
        const inY = Math.floor(cy + sa * (e.R - 3));
        const outX = Math.floor(cx + ca * (e.R + 4));
        const outY = Math.floor(cy + sa * (e.R + 4));
        assert.equal(ends.mat(inX, inY), TERRAIN.Road, `${e.facing} inside at ${k}`);
        assert.equal(ends.mat(outX, outY), TERRAIN.Pavement, `${e.facing} outside at ${k}`);
      }
      const mid = ends.mat(Math.floor(cx), Math.floor(cy));
      if (e.R >= 40) {
        assert.ok(e.Ri > 0, `${e.facing}: an island in a bulb of ${e.R}`);
        assert.equal(mid, TERRAIN.Park);
      } else assert.equal(mid, TERRAIN.Road);
    }
  });

  it("chamfers a hammerhead's corners and fills its inner corners at 45°", () => {
    const heads = laid("hammerhead").filter((e) => !e.diag);
    assert.equal(heads.length, 4);
    for (const e of heads) {
      let convex = 0;
      zone(e, (X, Y) => {
        // A convex corner of the carriageway: road with pavement on two sides at right angles is chamfered away,
        // so no road pixel has pavement both straight ahead and to a side within 2 px of a corner of the head.
        if (ends.mat(X, Y) !== TERRAIN.Road) return;
        const p = (dx: number, dy: number) => ends.mat(X + dx, Y + dy) !== TERRAIN.Road;
        if ((p(1, 0) || p(-1, 0)) && (p(0, 1) || p(0, -1)) && (p(2, 0) || p(-2, 0)) && (p(0, 2) || p(0, -2)) && p(1, 1) && p(-1, -1) && p(1, -1) && p(-1, 1)) convex++;
      });
      assert.equal(convex, 0, `${e.facing}: square corners`);
      // The head's far corners are pavement; 8 px in from them, carriageway.
      // The head reaches past the street on one side (an L) or both (a T).
      const far = (sv: number) => [e.ox + e.dx * 7 + e.ax * sv, e.oy + e.dy * 7 + e.ay * sv] as const;
      const reach = (side: 1 | -1) => {
        let k = 0;
        while (k < 200 && ends.mat(Math.floor(far(side * (k + 1))[0]), Math.floor(far(side * (k + 1))[1])) === TERRAIN.Road) k++;
        return k;
      };
      assert.ok(Math.max(reach(1), reach(-1)) > e.hw + 12, `${e.facing}: the head reaches past the street (${reach(1)}, ${reach(-1)}; ${e.hw})`);
    }
  });

  it("stops the carriageway at bollards on a dropped kerb, a kerb before a railing, setts into a square, a track at a field gate", () => {
    for (const e of [...laid("bollards"), ...laid("railing"), ...laid("plaza"), ...laid("field_gate")]) {
      let roadPast = 0;
      let roadBefore = 0;
      let flush = 0;
      let raised = 0;
      let setts = 0;
      let dirt = 0;
      let hedge = 0;
      zone(e, (X, Y, uu, vv) => {
        if (Math.abs(vv) >= e.hw - 2) return;
        const m = ends.mat(X, Y);
        if (m === TERRAIN.Road && uu > (e.kind === "field_gate" ? -10 : 0)) roadPast++;
        if (m === TERRAIN.Road && uu < -1 && uu > -6) roadBefore++;
        if (uu >= 0 && uu < 2 && px(X, Y) === L.t) flush++;
        if (uu >= 0 && uu < 2 && (px(X, Y) === L.Q || px(X, Y) === L.q)) raised++;
        if (uu > -6 && uu < 0 && px(X, Y) === L.q) setts++;
        if (m === TERRAIN.Dirt) dirt++;
        if (uu > -2 && uu < 5 && px(X, Y) === L.H) hedge++;
      });
      const name = `${e.kind} ${e.facing}`;
      assert.equal(roadPast, 0, `${name}: carriageway past the end line`);
      if (e.kind !== "field_gate") assert.ok(roadBefore > 0, `${name}: carriageway up to it`);
      if (e.kind === "bollards") {
        assert.ok(flush > 10, `${name}: a dropped kerb (${flush})`);
        assert.equal(raised, 0, `${name}: a raised kerb across`);
      }
      if (e.kind === "plaza") {
        assert.equal(raised, 0, `${name}: a kerb across`);
        assert.ok(setts > 10, `${name}: setts (${setts})`);
      }
      if (e.kind === "railing") assert.ok(raised > 0, `${name}: a kerb before the railing`);
      if (e.kind === "field_gate") {
        assert.ok(dirt > 40, `${name}: a track (${dirt})`);
        assert.ok(hedge > 20, `${name}: a hedge (${hedge})`);
      }
    }
  });

  it("has the props that close the ends: a variant per facing, a second look, lit bollards with a small pool", () => {
    for (const [kind, n] of [
      ["bollards", 16],
      ["field_gate", 16],
      ["railing", 8],
    ] as const) {
      const list = pack.props[kind] ?? [];
      assert.equal(list.length, n, kind);
      for (const p of list) {
        assert.equal(p.ax, 1, `${kind} anchored on its cell's left`);
        assert.ok(p.albedo.px.some((c) => c > 0));
      }
      // Every facing draws differently (north and south bollards stand on the kerb line's two sides).
      assert.ok(new Set(list.slice(0, 8).map((p) => p.albedo.px.join(","))).size >= 4, `${kind} per facing`);
    }
    assert.equal(endPropVariant("w"), 3);
    assert.equal(endPropVariant("se", true), 14);
    const lit = (pack.props.bollards ?? []).slice(8);
    for (const p of lit) {
      assert.ok(p.emissive && p.emissive.px.some((c) => c > 0), "a lit head");
      assert.ok(p.pool && p.pool.w < pack.pool.w && p.pool.rz < pack.pool.rz, "a smaller pool than the lamp's");
    }
    for (const p of (pack.props.bollards ?? []).slice(0, 8)) assert.equal(p.emissive, null);
  });

  it("is deterministic and never throws on odd ends (unknown kinds and facings, off the map, empty, no street)", () => {
    assert.deepEqual(groundImage(pack, endScene()).px, img.px);
    const t = new Uint8Array(25).fill(TERRAIN.Grass);
    for (let y = 0; y < 5; y++) t[y * 5 + 2] = TERRAIN.Road;
    const odd = [
      { kind: "moat", x: 2, y: 0, w: 1, h: 1, facing: "n" },
      { kind: "bollards", x: 2, y: 0, w: 1, h: 1, facing: "up" },
      { kind: "turning_circle", x: 40, y: 40, w: 3, h: 3, facing: "s" },
      { kind: "hammerhead", x: 2, y: 0, w: 0, h: 0, facing: "n" },
      { kind: "railing", x: -3, y: -3, w: 4, h: 4, facing: "ne" },
      ...END_KINDS.map((kind) => ({ kind, x: 0, y: 0, w: 5, h: 5, facing: "sw" })),
    ];
    const p = new TerrainPainter(pack.tiles, pack.terrain, { width: 5, height: 5, terrain: t, streetEnds: odd });
    for (let y = 0; y < 5; y++) for (let x = 0; x < 5; x++) assert.equal(p.draw(x, y, 0).px.length, 256);
    const one = new TerrainPainter(pack.tiles, pack.terrain, { width: 1, height: 1, terrain: new Uint8Array([1]), streetEnds: END_KINDS.map((kind) => ({ kind, x: 0, y: 0, w: 1, h: 1, facing: "e" })) });
    assert.equal(one.draw(0, 0, 0).px.length, 256);
  });
});

// ---------------------------------------------------------------------------
// Golden images (§7.2): pixel hashes of pinned fixtures. A change fails here;
// UPDATE_GOLDENS=1 rewrites test/golden.json, and the commit says what changed and why.

const goldenFile = new URL("./golden.json", import.meta.url);
const h = (img: Indexed) => hash32(img.w, img.h, ...img.px).toString(16);
/** A large image's hash, row by row (argument lists stay short). */
const hBig = (img: Indexed) => hash32(img.w, img.h, ...Array.from({ length: img.h }, (_, y) => hash32(...img.px.subarray(y * img.w, (y + 1) * img.w)))).toString(16);

function goldens(): Record<string, string> {
  const g: Record<string, string> = {};
  for (let s = 1; s <= 40; s++) {
    const c = composeCharacter(lookFromSeed(s), pal, pack.lightness);
    g[`character ${s}`] = hash32(...c.frames.flat().map((img) => Number.parseInt(h(img), 16))).toString(16);
  }
  g.player = hash32(...composeCharacter(PLAYER_LOOK, pal, pack.lightness).frames.flat().map((img) => Number.parseInt(h(img), 16))).toString(16);
  [...sampleBuildings(), ...varietyBuildings()].forEach((s, i) => {
    const f = buildFacade(s, pal);
    g[`building ${i} ${s.kind}`] = hash32(Number.parseInt(h(f.albedo), 16), ...f.windows.map((o) => (o.emissive ? Number.parseInt(h(o.emissive), 16) : 0))).toString(16);
  });
  for (const t of TERRAINS) g[`tiles ${t}`] = hash32(...BLOB_MASKS.map((m) => Number.parseInt(h(groundTile(pack.tiles, t, m, 0)), 16))).toString(16);
  for (const [k, list] of Object.entries(pack.props)) list.forEach((p, i) => (g[`prop ${k} ${i}`] = h(p.albedo)));
  // Round 4: every v2 piece's ground, the v2 scene's ground, and every plot state of every family.
  for (const piece of allPieces()) g[`v2 ${piece.label}`] = hBig(groundImage(pack, piece));
  g["v2 scene ground"] = hBig(groundImage(pack, v2Scene()));
  // Round 5: the diagonal-street test town's ground, and one street of each width on each axis.
  g["diag scene ground"] = hBig(groundImage(pack, diagScene()));
  for (const axis of [1, 2] as const) for (const r of [1, 2, 3, 4]) g[`diag street ${axis === 1 ? "ne" : "nw"} ${r}`] = hBig(groundImage(pack, oneDiagonal(r, axis)));
  // Round 6: the street-end test town's ground (the new props are pinned with every prop above).
  g["ends scene ground"] = hBig(groundImage(pack, endScene()));
  for (const code of V2_CODES) g[`v2 tiles ${code}`] = hash32(...BLOB_MASKS.map((m) => Number.parseInt(h(tileOf(pack.tiles, pack.terrain, code, m, 0)), 16))).toString(16);
  for (const [fam, spec] of FAMILY_SPECS) for (const state of PLOT_STATES) g[`plot ${fam} ${state}`] = hBig(buildFacade({ ...spec, conditions: state === "built" ? [] : [state] }, pal).albedo);
  g.palette = hash32(...pal.colors).toString(16);
  return g;
}

describe("golden images", () => {
  it("match the pinned fixtures", () => {
    const now = goldens();
    if (process.env.UPDATE_GOLDENS) {
      writeFileSync(goldenFile, `${JSON.stringify(now, null, 1)}\n`);
      return;
    }
    const pinned = JSON.parse(readFileSync(goldenFile, "utf8")) as Record<string, string>;
    const changed = Object.keys({ ...pinned, ...now }).filter((k) => pinned[k] !== now[k]);
    assert.deepEqual(changed, [], "art changed: review the contact sheets, then UPDATE_GOLDENS=1");
  });
});
