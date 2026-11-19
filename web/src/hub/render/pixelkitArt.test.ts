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

// ---------------------------------------------------------------------------
// Round 2: light that respects buildings, and the mood cross-fade.

/** A stub canvas factory whose image data is real, so baked lights can be read back. */
function pixelFactory(log = newLog()) {
  const base = stubCanvasFactory(log);
  return (w: number, h: number) => {
    const l = base(w, h);
    const store: { data: Uint8ClampedArray | null } = { data: null };
    const ctx = l.ctx as unknown as Record<string, unknown>;
    ctx.createImageData = (cw: number, ch: number) => ({ width: cw, height: ch, data: new Uint8ClampedArray(cw * ch * 4) });
    ctx.putImageData = (id: { data: Uint8ClampedArray }) => {
      store.data = id.data;
    };
    (l.canvas as unknown as { pixels: typeof store }).pixels = store;
    return l;
  };
}

interface LightProbe {
  frame: { image: { width: number; height: number; pixels?: { data: Uint8ClampedArray | null } }; w: number; h: number; ax: number; ay: number };
  x: number;
  y: number;
}
interface RendererProbe {
  lamps: { pool: LightProbe | null; shape: unknown; shaped: boolean }[];
  statics: unknown[];
  receiverAt(px: number, py: number, cands: readonly unknown[]): number;
  spillOf(b: unknown): LightProbe | null;
}

describe("light that respects buildings (round 2)", () => {
  const map = town();
  const make = pixelFactory();
  const art = new PixelkitArt(pack, { createCanvas: make });
  const r = new TownRenderer(stubContext(newLog(), { width: 412, height: 839 }), map, art, { createCanvas: make });
  const probe = r as unknown as RendererProbe;
  // Every lamp's pool, fitted to the town.
  for (const lamp of probe.lamps) (r as unknown as { shapeLamp(l: unknown): void }).shapeLamp(lamp);
  const lit = (l: LightProbe, fn: (wx: number, wy: number) => void) => {
    const d = l.frame.image.pixels?.data;
    if (!d) return;
    for (let y = 0; y < l.frame.h; y++) for (let x = 0; x < l.frame.w; x++) if ((d[(y * l.frame.w + x) * 4 + 3] as number) > 0) fn(l.x - l.frame.ax + x, l.y - l.frame.ay + y);
  };

  it("never lights a roof, and is not a plain ellipse where buildings stand", () => {
    let pools = 0;
    let onRoof = 0;
    let onWalls = 0;
    let clipped = 0;
    const full = (pack.pool.w / 2) * (pack.pool.h / 2) * Math.PI;
    for (const lamp of probe.lamps) {
      if (!lamp.pool) continue;
      pools++;
      let n = 0;
      lit(lamp.pool, (wx, wy) => {
        n++;
        const v = probe.receiverAt(wx, wy, probe.statics);
        if (v === 2) onRoof++;
        if (v >= 3) onWalls++;
      });
      if (n < full * 0.6) clipped++;
    }
    expect(pools).toBeGreaterThan(5);
    expect(onRoof).toBe(0);
    // Lamps by a wall splash light up its facade; some pools are cut short by buildings.
    expect(onWalls).toBeGreaterThan(0);
    expect(clipped).toBeGreaterThan(0);
  });

  it("throws window light on the ground in front of lit shopfronts, never on a building", () => {
    let spills = 0;
    for (const b of map.buildings) {
      const sp = probe.spillOf(b);
      if (!sp) continue;
      spills++;
      lit(sp, (wx, wy) => expect(probe.receiverAt(wx, wy, probe.statics)).toBe(0));
    }
    expect(spills).toBeGreaterThan(0);
  });

  it("cross-fades from the old mood's layers instead of stepping", () => {
    let now = 0;
    const moods = new PixelkitArt(pack, { createCanvas: stubCanvasFactory(newLog()), night: "moods" });
    const rr = new TownRenderer(stubContext(newLog(), { width: 412, height: 839 }), map, moods, { createCanvas: stubCanvasFactory(newLog()), now: () => now });
    const f = (minutes: number) => ({ width: 412, height: 839, camX: (map.width / 2) * 16, camY: (map.height / 2) * 16, zoom: 3, minutes, walkers: [], player: null, target: null, time: 0 });
    moods.setClock(19 * 60);
    expect(rr.draw(f(19 * 60))).toBe(true);
    expect(moods.setClock(19 * 60 + 30)).toBe(true);
    rr.crossFade(900);
    expect(rr.animating).toBe(true);
    // Frames keep changing while the fade runs, then it ends and idle frames are skipped again.
    let drawn = 0;
    for (now = 0; now <= 1000; now += 100) if (rr.draw(f(19 * 60 + 30))) drawn++;
    expect(drawn).toBeGreaterThan(4);
    expect(rr.animating).toBe(false);
    expect(rr.draw(f(19 * 60 + 30))).toBe(false);
  });
});
