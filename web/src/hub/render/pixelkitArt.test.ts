import { buildPack, TERRAIN } from "../../../../tools/pixelkit/art/mandate/pack";
import { diagScene } from "../../../../tools/pixelkit/art/mandate/diagscene";
import { STREET_AXIS } from "../../../../tools/pixelkit/art/mandate/streets";
import { TerrainPainter } from "../../../../tools/pixelkit/art/mandate/terrain";
import { v2Scene, type Scene } from "../../../../tools/pixelkit/art/mandate/v2scene";
import { FIXTURES } from "../testTowns";
import { TownScene } from "../townScene";
import { layoutTown, StreetAxis, Terrain, townSeed, type TownBuilding, type TownConnector, type TownMap, type TownProp } from "../townLayout";
import { buildingSpec, groundCells } from "./art";
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

// ---------------------------------------------------------------------------
// Round 4: layout v2 (DEC-090): levels, shores, creek, rail, bridges, props, plot states.

/** The pack's hand-made v2 test scene as a town map (every v2 code, two levels, every connector kind, every plot state); or another scene (round 5: the diagonal-street town). */
function v2Map(s: Scene = v2Scene()): TownMap {
  const n = s.width * s.height;
  const buildings: TownBuilding[] = s.buildings.map((b, index) => ({
    index,
    x: b.x,
    y: b.y,
    w: b.spec.w,
    h: b.spec.h,
    doorX: b.x + b.spec.doorX,
    doorY: b.y + b.spec.h - 1,
    front: (b.y + b.spec.h) * s.width + b.x + b.spec.doorX,
    place: null,
    kind: b.spec.kind,
    name: "",
    district: 0,
    home: false,
    landmark: false,
    style: b.spec.visualSeed,
    state: b.state,
  }));
  return {
    version: 2,
    seed: 7,
    municipality: "v2-fixture",
    width: s.width,
    height: s.height,
    terrain: s.terrain,
    elevation: s.elevation,
    connectors: s.connectors.map((c) => ({ ...c, kind: c.kind as TownConnector["kind"] })),
    ...(s.streetAxis ? { streetAxis: Uint8Array.from(s.streetAxis) } : {}),
    solid: new Uint8Array(n),
    district: new Int16Array(n),
    buildings,
    districts: [{ index: 0, id: 1, name: "Test", kind: "inner", centre: { x: 20, y: 14 }, street: 0, cells: n, bounds: { x: 0, y: 0, w: s.width, h: s.height } }],
    props: s.props.map((p) => ({ kind: p.kind as TownProp["kind"], x: p.x, y: p.y, blocks: p.blocks, variant: p.variant })),
    places: [],
    square: null,
    water: null,
    homeBuilding: null,
    start: 0,
  };
}

describe("layout v2 (round 4)", () => {
  it("knows the layout's terrain codes", () => {
    expect({ ...TERRAIN }).toEqual({ ...Terrain });
  });

  it("draws every cell of a v2 map with its neighbours in view, sharing frames between identical cells", () => {
    const map = v2Map();
    const art = new PixelkitArt(pack, { createCanvas: pixelFactory() });
    const { mask, variant } = groundCells(map);
    const frames = new Set<unknown>();
    let painted = 0;
    for (let y = 0; y < map.height; y++) {
      for (let x = 0; x < map.width; x++) {
        const i = y * map.width + x;
        const f = art.groundCell(map, x, y, mask[i] as number, variant[i] as number);
        expect([f.w, f.h, f.ax, f.ay]).toEqual([16, 16, 0, 0]);
        frames.add(f);
        if (f !== art.ground(map.terrain[i] as number, mask[i] as number, variant[i] as number)) painted++;
        // Asked again, the same frame.
        expect(art.groundCell(map, x, y, mask[i] as number, variant[i] as number)).toBe(f);
      }
    }
    // Cells near rivers, rail, levels and bridges are drawn; the rest of the town keeps v1's tiles.
    expect(painted).toBeGreaterThan(300);
    expect(painted).toBeLessThan(map.width * map.height);
    expect(frames.size).toBeLessThan(map.width * map.height);
    expect(art.composeMs.terrain).toBeGreaterThan(0);
  });

  it("gives a v1 town exactly ground()'s tiles", () => {
    const map = town();
    const art = new PixelkitArt(pack, { createCanvas: stubCanvasFactory(newLog()) });
    const { mask, variant } = groundCells(map);
    for (let i = 0; i < map.width * map.height; i += 7) {
      const x = i % map.width;
      const y = Math.floor(i / map.width);
      expect(art.groundCell(map, x, y, mask[i] as number, variant[i] as number)).toBe(art.ground(map.terrain[i] as number, mask[i] as number, variant[i] as number));
    }
  });

  it("falls back gracefully: unknown terrain codes, levels out of range, connectors of unknown kinds, unknown props", () => {
    const map = v2Map();
    const elevation = (map.elevation as Uint8Array).slice();
    const odd: TownMap = { ...map, terrain: map.terrain.slice(), elevation, connectors: [...(map.connectors ?? []), { kind: "zeppelin" as TownConnector["kind"], x: 30, y: 2, w: 40, h: 1, axis: "ew", name: "" }] };
    odd.terrain[5] = 31;
    odd.terrain[6] = 200;
    elevation[7] = 9;
    const art = new PixelkitArt(pack, { createCanvas: stubCanvasFactory(newLog()) });
    for (let x = 0; x < 12; x++) expect(art.groundCell(odd, x, 0, 255, x & 7).w).toBe(16);
    for (const code of [17, 18, 19, 20, 21, 22, 23, 24, 25, 31, 255]) for (const m of [0, 15, 255]) expect(art.ground(code, m, 3).w).toBe(16);
    expect(art.prop("windmill", 2).albedo.w).toBeGreaterThan(0);
  });

  it("draws the layout v2 props from the pack, not the placeholder", () => {
    const art = new PixelkitArt(pack, { createCanvas: stubCanvasFactory(newLog()) });
    for (const k of ["headframe", "spoil_heap", "reeds", "stepping_stones", "stall"]) {
      const list = pack.props[k] ?? [];
      expect(list.length).toBeGreaterThan(0);
      for (let v = 0; v < list.length + 2; v++) expect(art.prop(k, v).albedo.w).toBe(list[v % list.length]?.albedo.w);
    }
    expect(art.prop("headframe", 0).shadow?.height).toBeGreaterThan(40);
  });

  it("passes each plot's state to the art as a condition; vacant plots and sites are never lit", () => {
    const map = v2Map();
    const art = new PixelkitArt(pack, { createCanvas: stubCanvasFactory(newLog()) });
    const states = new Set<string>();
    for (const b of map.buildings) {
      const spec = buildingSpec(map, b);
      expect(spec.conditions).toEqual(b.state && b.state !== "built" ? [b.state] : []);
      const built = art.building(spec);
      expect(built.albedo.w).toBe(b.w * 16);
      expect(built.doors.length).toBeGreaterThan(0);
      if (b.state && b.state !== "built") expect([...built.windows, ...built.doors].every((o) => o.emissive === null)).toBe(true);
      states.add(b.state ?? "built");
    }
    expect([...states].sort()).toEqual(["boarded", "built", "construction", "derelict", "reserved"]);
  });

  it("renders a v2 map through the renderer (its ground() path) by day and at night without errors", () => {
    const map = v2Map();
    const make = stubCanvasFactory(newLog());
    const art = new PixelkitArt(pack, { createCanvas: make, night: "moods" });
    const r = new TownRenderer(stubContext(newLog(), { width: 412, height: 839 }), map, art, { createCanvas: make });
    for (const minutes of [720, 1350]) {
      art.setClock(minutes);
      r.crossFade(0);
      expect(r.draw({ width: 412, height: 839, camX: 320, camY: 224, zoom: 2, minutes, walkers: [], player: null, target: null, time: 0 })).toBe(true);
    }
  });
});

// ---------------------------------------------------------------------------
// Round 5: diagonal streets (DEC-090 D5 amended): `TownMap.streetAxis`.

describe("diagonal streets (round 5)", () => {
  it("knows the layout's street axis codes", () => {
    expect({ ...STREET_AXIS }).toEqual({ ...StreetAxis });
  });

  it("draws the diagonal streets of a map through groundCell, and every other cell exactly as without them", () => {
    const map = v2Map(diagScene());
    const plain: TownMap = { ...map };
    delete plain.streetAxis;
    const art = new PixelkitArt(pack, { createCanvas: stubCanvasFactory(newLog()) });
    const painter = new TerrainPainter(pack.tiles, pack.terrain, map);
    const { mask, variant } = groundCells(map);
    let diagonal = 0;
    let same = 0;
    for (let y = 0; y < map.height; y++) {
      for (let x = 0; x < map.width; x++) {
        const i = y * map.width + x;
        const f = art.groundCell(map, x, y, mask[i] as number, variant[i] as number);
        expect([f.w, f.h]).toEqual([16, 16]);
        const g = art.groundCell(plain, x, y, mask[i] as number, variant[i] as number);
        if (painter.streets?.touched(x, y)) {
          const k = map.streetAxis?.[i];
          if ((k === StreetAxis.DiagonalNE || k === StreetAxis.DiagonalNW) && map.terrain[i] === Terrain.Road && f !== g) diagonal++;
        } else {
          // Identical drawings share one frame, so a cell away from the diagonals gets the very frame it gets without them.
          expect(f).toBe(g);
          same++;
        }
      }
    }
    expect(diagonal).toBeGreaterThan(100);
    expect(same).toBeGreaterThan(800);
  });

  it("gives a v1 town with an all-orthogonal street axis exactly ground()'s tiles", () => {
    const map = { ...town() };
    map.streetAxis = new Uint8Array(map.width * map.height).fill(StreetAxis.Orthogonal);
    const art = new PixelkitArt(pack, { createCanvas: stubCanvasFactory(newLog()) });
    const { mask, variant } = groundCells(map);
    for (let i = 0; i < map.width * map.height; i += 5) {
      const x = i % map.width;
      const y = Math.floor(i / map.width);
      expect(art.groundCell(map, x, y, mask[i] as number, variant[i] as number)).toBe(art.ground(map.terrain[i] as number, mask[i] as number, variant[i] as number));
    }
  });

  it("renders a map with diagonal streets through the renderer by day and at night without errors", () => {
    const map = v2Map(diagScene());
    const make = stubCanvasFactory(newLog());
    const art = new PixelkitArt(pack, { createCanvas: make, night: "moods" });
    const r = new TownRenderer(stubContext(newLog(), { width: 412, height: 839 }), map, art, { createCanvas: make });
    for (const minutes of [720, 1350]) {
      art.setClock(minutes);
      r.crossFade(0);
      expect(r.draw({ width: 412, height: 839, camX: 360, camY: 300, zoom: 2, minutes, walkers: [], player: null, target: null, time: 0 })).toBe(true);
    }
  });
});
