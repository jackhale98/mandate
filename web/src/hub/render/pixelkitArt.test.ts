import { buildPack } from "../../../../tools/pixelkit/art/mandate/pack";
import { FIXTURES } from "../testTowns";
import { TownScene } from "../townScene";
import { layoutTown, townSeed, type TownMap } from "../townLayout";
import { buildingSpec } from "./art";
import { PixelkitArt } from "./pixelkitArt";
import { newLog, stubCanvasFactory, stubContext } from "./stubCanvas";
import { TownRenderer } from "./townRenderer";

const pack = buildPack();
const town = (): TownMap => {
  const f = FIXTURES[0] as (typeof FIXTURES)[number];
  return layoutTown(f.input, townSeed(f.seed, f.input.municipality));
};

describe("the pixel-art tool's provider (stubbed canvas)", () => {
  it("draws a town through the renderer without changes, at noon and at night", () => {
    const map = town();
    const make = stubCanvasFactory(newLog());
    const art = new PixelkitArt(pack, { createCanvas: make });
    const r = new TownRenderer(stubContext(newLog(), { width: 412, height: 839 }), map, art, { createCanvas: make });
    const scene = new TownScene(map, { townsfolk: 12 });
    const walkers = [scene.player, ...scene.folk].map((w) => ({ x: w.x, y: w.y, dir: w.dir, frame: 1, variant: w.variant }));
    for (const minutes of [720, 1170, 1350]) {
      expect(r.draw({ width: 412, height: 839, camX: scene.player.x * 16, camY: scene.player.y * 16, zoom: 3, minutes, walkers, player: { x: scene.player.x, y: scene.player.y }, target: null, time: 0 })).toBe(true);
    }
    expect(r.stats.baseBakes).toBeGreaterThan(0);
  });

  it("gives buildings their footprint plus a roof row (and headroom where nothing stands behind), with lit openings, shadow casters and a receiver map", () => {
    const map = town();
    const art = new PixelkitArt(pack, { createCanvas: stubCanvasFactory(newLog()) });
    let tall = 0;
    for (const b of map.buildings.slice(0, 30)) {
      const spec = buildingSpec(map, b);
      const built = art.building(spec);
      expect(built.albedo.w).toBe(b.w * 16);
      // The frame rises one roof row above the footprint, plus the headroom the layout leaves free behind it.
      expect(built.albedo.ay).toBe(16 + (spec.headroom ?? 0));
      expect(built.albedo.h).toBe(b.h * 16 + built.albedo.ay);
      expect(built.albedo.ay).toBeLessThanOrEqual(art.maxRoofRows * 16);
      if (built.albedo.ay > 16) tall++;
      expect(built.doors.length).toBeGreaterThan(0);
      expect(built.shadow.height).toBeGreaterThan(0);
      expect(built.shadows?.length).toBeGreaterThan(0);
      expect(built.receivers?.length).toBe(built.albedo.w * built.albedo.h);
    }
    expect(tall).toBeGreaterThan(0);
  });

  it("knows each building's left neighbour and street corners from the layout", () => {
    const map = town();
    const specs = map.buildings.map((b) => buildingSpec(map, b));
    expect(specs.some((s) => s.left)).toBe(true);
    expect(specs.some((s) => s.ends?.left === "street" || s.ends?.right === "street")).toBe(true);
    // A spec is made once per building: neighbours share the same object.
    for (const s of specs) if (s.left) expect(specs).toContain(s.left);
  });

  it("serves 16×24 characters anchored at the feet, composed once per person", () => {
    const art = new PixelkitArt(pack, { createCanvas: stubCanvasFactory(newLog()) });
    const f = art.character({ seed: 5 }, "walk", "side", 1);
    expect([f.w, f.h, f.ax, f.ay]).toEqual([16, 24, 7, 23]);
    const made = art.made;
    art.character({ seed: 5 }, "idle", "down", 0);
    art.character({ seed: 5 }, "walk", "up", 2);
    expect(art.made).toBe(made);
    art.character({ seed: 6 }, "idle", "down", 0);
    expect(art.made).toBe(made + 1);
  });

  it("keeps at most the configured number of characters", () => {
    const art = new PixelkitArt(pack, { createCanvas: stubCanvasFactory(newLog()), maxCharacters: 4 });
    for (let s = 0; s < 10; s++) art.character({ seed: s }, "idle", "down", 0);
    expect(art.composeMs.count).toBe(10);
    art.character({ seed: 9 }, "idle", "down", 0);
    expect(art.composeMs.count).toBe(10);
    art.character({ seed: 0 }, "idle", "down", 0);
    expect(art.composeMs.count).toBe(11);
  });

  it("palette moods: the static palette changes with the clock, and only in the moods look", () => {
    const moods = new PixelkitArt(pack, { createCanvas: stubCanvasFactory(newLog()), night: "moods" });
    expect(moods.setClock(720)).toBe(true);
    expect(moods.setClock(725)).toBe(false);
    expect(moods.setClock(23 * 60)).toBe(true);
    // At night the people keep more light than the multiply look gives them.
    const multiply = new PixelkitArt(pack, { createCanvas: stubCanvasFactory(newLog()) });
    expect(multiply.setClock(23 * 60)).toBe(false);
    expect(moods.grade(23 * 60).mul[0]).toBeGreaterThan(multiply.grade(23 * 60).mul[0]);
    expect(multiply.grade(720).mul).toEqual([255, 255, 255]);
  });

  it("falls back to the placeholder only for what the tool plan leaves out (markers, icons)", () => {
    const art = new PixelkitArt(pack, { createCanvas: stubCanvasFactory(newLog()) });
    expect(art.icon("diner")).toMatch(/^hub-icon-/);
    expect(art.marker("pin").w).toBeGreaterThan(0);
    for (const k of ["lamp", "bench", "tree", "pine", "planter", "fountain", "flag", "stall", "boat", "crane"]) expect(pack.props[k]?.length).toBeGreaterThan(0);
  });
});
