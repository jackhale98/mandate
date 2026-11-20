// Interiors (art round 7, DEC-088): recipe validation, determinism (the same
// recipe version and seed give the same pixels), every spot and hotspot on
// walkable floor reachable from the door, a limited palette, and pinned room
// hashes (goldens for the rooms).
import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { hash32 } from "../src/core/hash";
import type { Indexed } from "../src/core/image";
import { colourCount, offPalette } from "../src/core/lint";
import { composeInterior, flattenInterior, gradeImage, gradeRoom, interiorGrade, optionCombos, placeholderPeople, reachable, renderInterior, TILE, validateRecipe } from "../art/mandate/interiors";
import { PARTS, partSlots, SURFACES, surfaceSlots } from "../art/mandate/interiorsParts";
import { readFileSync } from "node:fs";
import { catalogText, CATALOG_FILE } from "../art/mandate/interiorsCatalog";
import { COUNCIL_CHAMBER, DINER, RECIPES, TOWN_HALL } from "../art/mandate/interiorsContent";
import { recipeFor, type InteriorRecipe } from "../art/mandate/interiorsRecipes";
import { buildPack, PACK_VERSION } from "../art/mandate/pack";
import { INTERIOR_SEEDS, INTERIOR_TIMES } from "../art/mandate/samples";

const pack = buildPack();
const pal = pack.palette;
const compose = (r: InteriorRecipe, seed: number) => composeInterior(r, seed, pal, pack.lightness);
const h = (img: Indexed) => hash32(img.w, img.h, ...Array.from({ length: img.h }, (_, y) => hash32(...img.px.subarray(y * img.w, (y + 1) * img.w)))).toString(16);

describe("interior recipes", () => {
  it("are valid for every option combination", () => {
    for (const r of RECIPES) assert.deepEqual(validateRecipe(r), [], r.id);
  });
  it("cover the three M6 places, each once per version, and are plain data (JSON round-trip)", () => {
    assert.deepEqual(RECIPES.map((r) => r.place).sort(), ["council_chamber", "diner", "town_hall"]);
    for (const r of RECIPES) assert.deepEqual(JSON.parse(JSON.stringify(r)), r, r.id);
  });
  it("come from the game's content (content/base/interiors.jsonc), the highest version per place", () => {
    assert.equal(recipeFor(RECIPES, "town_hall")?.version, 2);
    assert.equal(recipeFor(RECIPES, "interior.diner")?.place, "diner");
    assert.equal(recipeFor(RECIPES, "school"), undefined);
  });
  it("keep the content check's part catalogue equal to the pack (regenerate: interiorsCatalog.ts --write)", () => {
    assert.equal(readFileSync(CATALOG_FILE, "utf8"), catalogText());
  });
  it("give each place's actions a hotspot (chamber: vote, introduce, amend, withdraw; town hall: draft, file, withdraw; diner: lobby, trade_favor, take_bribe late)", () => {
    const offered = (r: InteriorRecipe) => {
      const all = new Set<string>();
      for (const combo of optionCombos(r)) {
        const opts = r.choices.flatMap((c) => c.options.filter((o) => o.id === combo[c.id]));
        for (const hs of [...r.hotspots, ...opts.flatMap((o) => o.hotspots ?? [])]) for (const a of hs.actions ?? []) all.add(a);
      }
      return all;
    };
    for (const a of ["vote", "introduce", "amend", "withdraw"]) assert.ok(offered(COUNCIL_CHAMBER).has(a), a);
    for (const a of ["draft", "file", "withdraw"]) assert.ok(offered(TOWN_HALL).has(a), a);
    for (const a of ["lobby", "trade_favor", "take_bribe"]) assert.ok(offered(DINER).has(a), a);
    assert.ok(DINER.hotspots.some((x) => x.actions?.includes("take_bribe") && x.late));
  });
  it("catch broken recipes: unknown parts, overlaps, spots off the floor or out of reach, sitting without a seat", () => {
    const broken: InteriorRecipe = {
      ...DINER,
      pieces: [...DINER.pieces, { part: "no_such_part", x: 6, y: 6 }, { part: "plant_tall", x: 5, y: 3 }, { part: "rail", x: 0, y: 7, len: 12 }],
      spots: [...DINER.spots, { id: "inside_counter", role: "patron", x: 6, y: 3, facing: "down" }, { id: "on_floor", role: "patron", x: 6, y: 6, facing: "down", pose: "sit" }],
      hotspots: [...DINER.hotspots, { id: "far", actions: ["lobby"], x: 0, y: 0, w: 1, h: 1, stand: { x: 6, y: 6 }, facing: "up" }],
    };
    const errs = validateRecipe(broken).join("\n");
    assert.match(errs, /unknown part "no_such_part"/);
    assert.match(errs, /plant_tall overlaps diner_counter/);
    assert.match(errs, /spot inside_counter at \(6, 3\) is not on walkable floor/);
    assert.match(errs, /not reachable from the door/);
    assert.match(errs, /spot on_floor sits but \(6, 6\) has no seat/);
    assert.match(errs, /hotspot far: its stand \(6, 6\) is not next to it/);
  });
});

describe("interior rooms", () => {
  it("are deterministic: the same recipe and seed give the same pixels, sprites, spots and render", () => {
    for (const r of RECIPES) {
      const a = compose(r, 23);
      const b = compose(r, 23);
      assert.deepEqual(a.base.px, b.base.px);
      assert.deepEqual(a.sprites.map((s) => [s.part, s.x, s.y, s.key, h(s.img)]), b.sprites.map((s) => [s.part, s.x, s.y, s.key, h(s.img)]));
      assert.deepEqual(a.spots, b.spots);
      assert.deepEqual(a.walk, b.walk);
      const ra = renderInterior(a, pal, pack.lighting, INTERIOR_TIMES.evening, placeholderPeople(a, pal, pack.lightness, 5));
      const rb = renderInterior(b, pal, pack.lighting, INTERIOR_TIMES.evening, placeholderPeople(b, pal, pack.lightness, 5));
      assert.deepEqual(ra.data, rb.data);
    }
  });
  it("vary with the seed (materials, arrangement or decoration) and every option is reachable by some seed", () => {
    for (const r of RECIPES) {
      const flats = new Set(INTERIOR_SEEDS.map((s) => h(flattenInterior(compose(r, s)))));
      assert.equal(flats.size, INTERIOR_SEEDS.length, r.id);
      const seen = new Set<string>();
      for (let s = 1; s <= 60; s++) for (const [k, v] of Object.entries(compose(r, s).picks)) seen.add(`${k}=${v}`);
      for (const c of r.choices) for (const o of c.options) assert.ok(seen.has(`choice:${c.id}=${o.id}`), `${r.id}: ${c.id}=${o.id} never picked`);
    }
  });
  it("put every spot and hotspot stand on walkable floor reachable from the door (60 seeds each, with the parts the seed chose)", () => {
    for (const r of RECIPES) {
      for (let s = 1; s <= 60; s++) {
        const rm = compose(r, s);
        const reach = reachable(rm.cols, rm.rows, rm.walk, rm.door);
        const ok = (x: number, y: number, what: string) => {
          const k = y * rm.cols + x;
          assert.notEqual(rm.walk[k], TILE.blocked, `${r.id} seed ${s}: ${what} on a blocked tile`);
          assert.equal(reach[k], 1, `${r.id} seed ${s}: ${what} unreachable`);
        };
        assert.equal(rm.walk[rm.door.y * rm.cols + rm.door.x], TILE.floor);
        for (const sp of rm.spots) {
          ok(sp.x, sp.y, `spot ${sp.id}`);
          if (sp.pose === "sit") assert.equal(rm.walk[sp.y * rm.cols + sp.x], TILE.seat, `${r.id} seed ${s}: ${sp.id} sits on no seat`);
        }
        for (const hs of rm.hotspots) ok(hs.stand.x, hs.stand.y, `hotspot ${hs.id}`);
      }
    }
  });
  it("seat sitters between a seat and its front: every seated spot has a seat sprite with a hip height", () => {
    for (const r of RECIPES) {
      const rm = compose(r, 47);
      for (const sp of rm.spots.filter((x) => x.pose === "sit")) {
        const seat = rm.sprites.find((x) => x.sitY && sp.x >= x.tiles.x && sp.x < x.tiles.x + x.tiles.w && sp.y >= x.tiles.y && sp.y < x.tiles.y + x.tiles.h);
        assert.ok(seat, `${r.id}: ${sp.id}`);
      }
    }
  });
  it("grade layer by layer as the reference renderer does (the game's canvas path: base, then sprites and people by depth)", () => {
    for (const r of RECIPES) {
      const rm = compose(r, 23);
      for (const minutes of [INTERIOR_TIMES.day, INTERIOR_TIMES.evening]) {
        // Seated people only (no blob shadows, which the game draws on the canvas).
        const actors = placeholderPeople(rm, pal, pack.lightness, 5).filter((a) => !a.feet);
        const ref = renderInterior(rm, pal, pack.lighting, minutes, actors);
        const g = interiorGrade(rm, pal, pack.lighting, minutes);
        const graded = gradeRoom(rm, g);
        const out = new Uint8Array(graded.base.data);
        const layers: { data: Uint8ClampedArray; x: number; y: number; w: number; h: number; key: number }[] = [];
        rm.sprites.forEach((s, i) => {
          const gs = graded.sprites[i] as { img: Uint8ClampedArray; front: Uint8ClampedArray | null };
          layers.push({ data: gs.img, x: s.x, y: s.y, w: s.img.w, h: s.img.h, key: s.key });
          if (s.front && gs.front) layers.push({ data: gs.front, x: s.x, y: s.front.y, w: s.front.img.w, h: s.front.img.h, key: s.front.key });
        });
        for (const a of actors) layers.push({ data: gradeImage(g, a.img, a.x, a.y), x: a.x, y: a.y, w: a.img.w, h: a.img.h, key: a.key });
        layers.sort((a, b) => a.key - b.key);
        for (const l of layers) {
          for (let y = 0; y < l.h; y++) {
            for (let x = 0; x < l.w; x++) {
              const j = (y * l.w + x) * 4;
              const X = l.x + x;
              const Y = l.y + y;
              if (!l.data[j + 3] || X < 0 || Y < 0 || X >= rm.w || Y >= rm.h) continue;
              out.set(l.data.subarray(j, j + 4), (Y * rm.w + X) * 4);
            }
          }
        }
        assert.deepEqual(out, ref.data, `${r.id} at ${minutes}`);
      }
    }
  });
  it("fit a phone: at most 14 tiles wide and 13 deep", () => {
    for (const r of RECIPES) assert.ok(r.cols <= 14 && r.rows <= 13, r.id);
  });
});

describe("interior palette", () => {
  it("uses only master palette colours, adds none, and stays within budgets (room ≤ 56, furniture ≤ 14, surfaces ≤ 6)", () => {
    assert.equal(pal.colors.length - 1, 114, "the interiors add no palette colours");
    const n = pal.colors.length - 1;
    for (const r of RECIPES) {
      for (let s = 1; s <= 40; s++) {
        const rm = compose(r, s);
        const flat = flattenInterior(rm, placeholderPeople(rm, pal, pack.lightness, s));
        assert.equal(offPalette(flat, n), 0);
        assert.ok(colourCount(flattenInterior(rm)) <= 56, `${r.id} seed ${s}: ${colourCount(flattenInterior(rm))} colours`);
        for (const sp of rm.sprites) assert.ok(colourCount(sp.img) <= 14, `${r.id} seed ${s}: ${sp.part} has ${colourCount(sp.img)} colours`);
      }
    }
    for (const sf of SURFACES) assert.ok(colourCount(surfaceSlots(sf.id)) <= 6, sf.id);
  });
  it("draws every part (30–60 interior parts, as round 7 asks) and every part parses at its strip lengths", () => {
    assert.ok(PARTS.length + SURFACES.length >= 30 && PARTS.length <= 70, `${PARTS.length} parts`);
    for (const p of PARTS) for (const len of p.strip ? [1, 2, 5] : [1]) assert.ok(partSlots(p.id, len).px.some((v) => v !== 0), p.id);
  });
});

describe("interior goldens", () => {
  // Pinned rooms (flattened, no people): a change fails here. Update deliberately, with the pack version.
  const PINNED: Record<string, string> = {
    "interior.council_chamber@1 11": "282d70bb",
    "interior.council_chamber@1 23": "3da1754d",
    "interior.council_chamber@1 47": "fba731dc",
    "interior.council_chamber@1 90": "c53c8745",
    "interior.town_hall@2 11": "33aa7ccf",
    "interior.town_hall@2 23": "b2b8aed",
    "interior.town_hall@2 47": "a48a15a7",
    "interior.town_hall@2 90": "d48c7bb3",
    "interior.diner@1 11": "c881d673",
    "interior.diner@1 23": "2855748c",
    "interior.diner@1 47": "9f2bb95d",
    "interior.diner@1 90": "49519df",
  };
  it("match the pinned rooms", () => {
    assert.equal(PACK_VERSION, "mandate-p0.9");
    const now: Record<string, string> = {};
    for (const r of RECIPES) for (const s of INTERIOR_SEEDS) now[`${r.id}@${r.version} ${s}`] = h(flattenInterior(compose(r, s)));
    if (process.env.UPDATE_GOLDENS) {
      console.log(JSON.stringify(now, null, 1));
      return;
    }
    assert.deepEqual(now, PINNED);
  });
});
