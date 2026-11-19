import { SHADOW_BUCKETS, SUNRISE, SUNSET, shadowBucket } from "../dayCycle";
import { fixtureTown, randomTown } from "../testTowns";
import { ROOF_ROWS, type TownMap } from "../townLayout";
import { buildingSpec, groundCells } from "./art";
import { TownScene } from "../townScene";
import { PlaceholderArt } from "./placeholderArt";
import { newLog, stubCanvasFactory, stubContext, type StubLog } from "./stubCanvas";
import { TownRenderer, type FrameState } from "./townRenderer";

const town = (i: number): TownMap => fixtureTown(i).map;

// Layout v2 plans take tens of ms each (30 random towns below).
vi.setConfig({ testTimeout: 120_000 });

function setup(map: TownMap, opts: { maxChunks?: number } = {}) {
  const log = newLog();
  const make = stubCanvasFactory(log);
  const art = new PlaceholderArt({ createCanvas: make });
  // The on-screen context logs on its own: what each frame costs.
  const screenLog = newLog();
  const screen = stubContext(screenLog, { width: 412, height: 839 });
  const r = new TownRenderer(screen, map, art, { createCanvas: make, ...opts });
  return { log, screenLog, art, r };
}

const frame = (map: TownMap, scene: TownScene | null, minutes: number, over: Partial<FrameState> = {}): FrameState => {
  const p = scene?.player;
  return {
    width: 412,
    height: 839,
    camX: (p ? p.x : map.width / 2) * 16,
    camY: (p ? p.y : map.height / 2) * 16,
    zoom: 3,
    minutes,
    walkers: scene ? [scene.player, ...scene.folk].map((w) => ({ x: w.x, y: w.y, dir: w.dir, frame: w.moving ? 1 + (Math.floor(w.dist * 3) % 2) : 0, variant: w.variant })) : [],
    player: p ? { x: p.x, y: p.y } : null,
    target: scene?.target ? { cell: scene.target.cell, building: scene.target.kind === "building" ? scene.target.building : null } : null,
    time: 0,
    ...over,
  };
};

const calls = (log: StubLog, k: string) => log.calls[k] ?? 0;

describe("town renderer (stubbed canvas)", () => {
  it("draws a town, bakes only the visible chunks, and skips identical frames", () => {
    const map = town(0);
    const { r, screenLog: log } = setup(map);
    const scene = new TownScene(map, { townsfolk: 0 });
    expect(r.draw(frame(map, scene, 12 * 60))).toBe(true);
    const base = r.stats.baseBakes;
    const shadows = r.stats.shadowBakes;
    expect(base).toBeGreaterThan(0);
    expect(base).toBeLessThanOrEqual(6);
    expect(shadows).toBe(base);
    const drawn = calls(log, "drawImage");
    // Idle: a hundred frames, nothing redrawn or re-baked.
    for (let k = 0; k < 100; k++) expect(r.draw(frame(map, scene, 12 * 60, { time: k / 60 }))).toBe(false);
    expect(r.stats.frames).toBe(1);
    expect(r.stats.skipped).toBe(100);
    expect(r.stats.baseBakes).toBe(base);
    expect(calls(log, "drawImage")).toBe(drawn);
  });

  it("re-bakes shadows only when the sun bucket changes, never the base", () => {
    const map = town(2);
    const { r } = setup(map);
    const scene = new TownScene(map, { townsfolk: 0 });
    let m = SUNRISE + 30;
    r.draw(frame(map, scene, m));
    const base = r.stats.baseBakes;
    let bakes = r.stats.shadowBakes;
    let bucket = shadowBucket(m);
    let changes = 0;
    for (; m < SUNSET - 1; m += 0.5) {
      r.draw(frame(map, scene, m));
      const b = shadowBucket(m);
      if (b !== bucket) {
        changes++;
        expect(r.stats.shadowBakes).toBe(bakes + base);
        bucket = b;
      } else expect(r.stats.shadowBakes).toBe(bakes);
      bakes = r.stats.shadowBakes;
    }
    expect(changes).toBeGreaterThan(SHADOW_BUCKETS - 4);
    expect(r.stats.baseBakes).toBe(base);
    // Night: no shadows drawn or baked.
    r.draw(frame(map, scene, 23 * 60));
    expect(r.stats.shadowBakes).toBe(bakes);
  });

  it("a running clock redraws as the light changes, without re-baking", () => {
    const map = town(1);
    const { r } = setup(map);
    const scene = new TownScene(map, { townsfolk: 0 });
    r.draw(frame(map, scene, 18 * 60 + 30));
    const base = r.stats.baseBakes;
    let drawn = 0;
    for (let m = 18 * 60 + 30; m < 19 * 60 + 30; m += 1) if (r.draw(frame(map, scene, m))) drawn++;
    expect(drawn).toBeGreaterThan(50);
    expect(r.stats.baseBakes).toBe(base);
  });

  it("lights windows and lamps at night, not by day", () => {
    const map = town(0);
    const day = setup(map);
    const night = setup(map);
    const scene = new TownScene(map, { townsfolk: 0 });
    day.r.draw(frame(map, scene, 12 * 60));
    night.r.draw(frame(map, scene, 23 * 60));
    // Per frame on screen: the night adds window lights, lamp pools and lamp heads.
    expect(calls(night.screenLog, "drawImage")).toBeGreaterThan(calls(day.screenLog, "drawImage"));
    expect(calls(night.screenLog, "fillRect")).toBeGreaterThan(calls(day.screenLog, "fillRect"));
  });

  it("walks the player across town at every time of day without errors, culling what is off screen", () => {
    for (const i of [0, 1, 4]) {
      const map = town(i);
      const { r } = setup(map, { maxChunks: 8 });
      const scene = new TownScene(map, { townsfolk: 20, seed: 3 });
      const far = map.buildings.reduce((a, b) => (Math.abs(b.front - map.start) > Math.abs(a.front - map.start) ? b : a));
      scene.goToBuilding(far.index);
      for (let k = 0; k < 600; k++) {
        scene.step(1 / 30);
        r.draw(frame(map, scene, (k * 2.4) % 1440, { time: k / 30 }));
        expect(r.stats.chunks).toBeLessThanOrEqual(8);
      }
      expect(r.stats.frames).toBeGreaterThan(500);
    }
  });

  it("draws side-facing walkers mirrored for facing right", () => {
    const map = town(0);
    const { r, screenLog } = setup(map);
    const scene = new TownScene(map, { townsfolk: 0 });
    r.draw(frame(map, scene, 12 * 60, { walkers: [{ x: scene.player.x, y: scene.player.y, dir: 3, frame: 1, variant: -1 }] }));
    expect(calls(screenLog, "scale")).toBe(1);
    r.draw(frame(map, scene, 12 * 60, { walkers: [{ x: scene.player.x, y: scene.player.y, dir: 2, frame: 1, variant: -1 }] }));
    expect(calls(screenLog, "scale")).toBe(1);
  });
});

describe("placeholder art: the ArtProvider contract (PIXEL_ART_TOOL.md §5.1)", () => {
  const maps = [town(1), ...Array.from({ length: 30 }, (_, s) => randomTown(s + 1).map)];

  it("buildings: albedo of footprint plus roof, windows and doors with emissive frames and use tags, a shadow footprint and height", () => {
    const art = new PlaceholderArt({ createCanvas: stubCanvasFactory(newLog()) });
    const uses = new Set<string>();
    for (const map of maps) {
      for (const b of map.buildings) {
        const spec = buildingSpec(map, b);
        const a = art.building(spec);
        // Reserved land and building sites are lots, not buildings: no walls to shade, no door (DEC-090 D2).
        if (b.state === "reserved" || b.state === "construction") {
          expect(a.doors).toHaveLength(0);
          continue;
        }
        expect(a.albedo.w).toBe(b.w * 16);
        expect(a.albedo.h).toBe((b.h + ROOF_ROWS) * 16);
        expect(a.albedo.ay).toBe(ROOF_ROWS * 16);
        expect(a.shadow.footprint.length).toBeGreaterThanOrEqual(3);
        expect(a.shadow.height).toBeGreaterThan(0);
        expect(a.doors).toHaveLength(1);
        for (const o of [...a.windows, ...a.doors]) {
          expect(o.x).toBeGreaterThanOrEqual(0);
          expect(o.x + o.w).toBeLessThanOrEqual(a.albedo.w);
          expect(o.y + o.h).toBeLessThanOrEqual(a.albedo.h);
          uses.add(o.use);
        }
        if (b.place !== null) expect(a.doors[0]?.emissive).not.toBeNull();
        // The same place looks the same wherever the layout puts it.
        if (b.place !== null) expect(spec.visualSeed).toBe(buildingSpec({ ...map }, { ...b, x: b.x + 5 }).visualSeed);
      }
    }
    expect([...uses].sort()).toEqual(["civic", "home", "industrial", "office", "shop"]);
  });

  it("characters are 16×24, anchored at the feet, in three facings; frames are cached", () => {
    const log = newLog();
    const art = new PlaceholderArt({ createCanvas: stubCanvasFactory(log) });
    for (let v = -1; v < 70; v++) {
      for (const facing of ["down", "up", "side"] as const) {
        for (let f = 0; f <= art.walkFrames; f++) {
          const c = art.character({ seed: v, player: v < 0 }, f ? "walk" : "idle", facing, f);
          expect([c.w, c.h, c.ax, c.ay]).toEqual([16, 24, 8, 23]);
        }
      }
    }
    const before = art.made;
    art.character({ seed: 5 }, "walk", "side", 1);
    expect(art.made).toBe(before);
  });

  it("ground tiles, props, grade and icons", () => {
    const art = new PlaceholderArt({ createCanvas: stubCanvasFactory(newLog()) });
    for (const map of maps) {
      const g = groundCells(map);
      for (let i = 0; i < map.terrain.length; i += 7) {
        const f = art.ground(map.terrain[i] as number, g.mask[i] as number, g.variant[i] as number);
        expect([f.w, f.h]).toEqual([16, 16]);
      }
      for (const p of map.props) art.prop(p.kind, p.variant);
    }
    // Ground tiles share sheets: a town draws from a handful of canvases.
    expect(art.made).toBeLessThan(maps.reduce((a, m) => a + m.buildings.length, 0) / 10 + 40);
    const lamp = art.prop("lamp", 0);
    expect(lamp.emissive).toBeDefined();
    expect(lamp.pool).toBeDefined();
    expect(art.prop("tree", 1).shadow?.height).toBeGreaterThan(0);
    // Grading: shadows by day only, lights by night only.
    expect(art.grade(12 * 60).shadow.alpha).toBeGreaterThan(0.2);
    expect(art.grade(12 * 60).emissive).toBe(0);
    expect(art.grade(23 * 60).shadow.alpha).toBe(0);
    expect(art.grade(23 * 60).emissive).toBe(1);
    expect(art.icon("council_chamber")).toBe("hub-icon-dome");
    expect(art.iconSprite()).toContain('id="hub-icon-dome"');
    for (const k of ["here", "pin", "target", "door"] as const) expect(art.marker(k).w).toBeGreaterThan(0);
  });
});
