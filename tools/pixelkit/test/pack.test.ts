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
    for (const lint of ["palette size", "no pure black or white", "palette conformance", "outline consistency", "crowd clones", "height bands", "silhouette readability", "neighbour similarity", "flatness", "head contrast"]) {
      const bad = f.filter((x) => x.lint === lint && x.level === "error");
      assert.deepEqual(bad, [], lint);
    }
  });
});

// ---------------------------------------------------------------------------
// Golden images (§7.2): pixel hashes of pinned fixtures. A change fails here;
// UPDATE_GOLDENS=1 rewrites test/golden.json, and the commit says what changed and why.

const goldenFile = new URL("./golden.json", import.meta.url);
const h = (img: Indexed) => hash32(img.w, img.h, ...img.px).toString(16);

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
