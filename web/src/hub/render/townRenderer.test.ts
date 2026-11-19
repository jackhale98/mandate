import { SHADOW_BUCKETS, SUNRISE, SUNSET, shadowBucket } from "../dayCycle";
import { FIXTURES, randomInput } from "../testTowns";
import { layoutTown, townSeed, type TownMap } from "../townLayout";
import { TownScene } from "../townScene";
import { PlaceholderArt } from "./placeholderArt";
import { newLog, stubCanvasFactory, stubContext, type StubLog } from "./stubCanvas";
import { TownRenderer, type FrameState } from "./townRenderer";

const town = (i: number): TownMap => {
  const f = FIXTURES[i] as (typeof FIXTURES)[number];
  return layoutTown(f.input, townSeed(f.seed, f.input.municipality));
};

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

  it("the placeholder art draws every place kind, filler and walker, once each", () => {
    const maps = [town(1), ...Array.from({ length: 30 }, (_, s) => layoutTown(randomInput(s + 1), s + 1))];
    const log = newLog();
    const art = new PlaceholderArt({ createCanvas: stubCanvasFactory(log) });
    const ctx = stubContext(log, { width: 16, height: 16 });
    for (const map of maps) {
      for (const b of map.buildings) {
        const s = art.building(map, b);
        expect(s.w).toBe(b.w * 16);
        expect(art.building(map, b)).toBe(s);
        art.buildingLights(map, b);
      }
      for (const p of map.props) art.prop(map, p);
      for (let y = 0; y < map.height; y += 3) for (let x = 0; x < map.width; x += 3) art.ground(ctx, map, x, y, 0, 0);
    }
    for (let v = -1; v < 70; v++) for (let d = 0; d < 4; d++) for (let f = 0; f < 3; f++) art.walker(v, d, f);
    const before = art.made;
    art.walker(5, 2, 1);
    art.lightPool();
    art.lightPool();
    for (const k of ["here", "pin", "target", "door"] as const) art.marker(k);
    expect(art.made).toBeLessThanOrEqual(before + 5);
  });
});
