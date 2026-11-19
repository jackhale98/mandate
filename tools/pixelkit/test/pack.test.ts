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
    for (const lint of ["palette size", "no pure black or white", "palette conformance", "outline consistency", "crowd clones", "height bands", "silhouette readability", "neighbour similarity", "flatness", "head contrast", "v2 coverage", "connector kits", "plot states", "v2 seams"]) {
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
