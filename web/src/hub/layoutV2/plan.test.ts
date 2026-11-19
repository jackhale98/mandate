// Town layout v2 (DEC-090, stage A): validity, readability (§8), determinism,
// dress monotonicity, the text alternative and stage budgets, over the real
// towns dumped from the engine (../fixtures/towns-v2.json) and seeded random
// geographies.
import { townSeed } from "../townLayout";
import { checkTownV2, readability, readabilityProblems } from "./check";
import { GEO_FIXTURES, randomGeo, type GeoFixture } from "./fixtures";
import { BAND_CAPS, bandOf } from "./geo";
import { compassVec, ipow, isqrt, log2Q8, pow2Q8 } from "./imath";
import { dressHash, dressTown, geoFromInput, layoutTownV2, planHash, planTown, type TownState } from "./index";
import { snapshotRecipe, BASE_RECIPE } from "./registry";
import { squareSize, townSize } from "./terrain";
import { describeTown } from "./text";
import { WaterKind } from "./work";
import { Terrain } from "../townLayout";

// Planning a few hundred towns takes a while on a shared runner.
vi.setConfig({ testTimeout: 120_000 });

const seedOf = (f: GeoFixture) => townSeed(f.seed, f.input.municipality);
const label = (f: GeoFixture) => `${f.input.municipality} (seed ${f.seed}, ${f.source})`;

describe("layout v2: integer maths", () => {
  it("logs and powers are exact integer steps", () => {
    expect(log2Q8(1)).toBe(0);
    expect(log2Q8(2)).toBe(256);
    expect(log2Q8(1024)).toBe(2560);
    expect(Math.abs(log2Q8(3) - 405)).toBeLessThanOrEqual(1);
    expect(pow2Q8(256)).toBe(2);
    expect(pow2Q8(2560)).toBe(1024);
    // population^0.7: 1,000,000^0.7 ≈ 15,849
    expect(Math.abs(ipow(1_000_000, 7, 10) - 15849)).toBeLessThan(200);
    expect(isqrt(99)).toBe(9);
    expect(isqrt(100)).toBe(10);
  });
  it("compass vectors point where they say", () => {
    expect(compassVec(0)).toEqual({ dx: 0, dy: -1024 });
    expect(compassVec(4)).toEqual({ dx: 1024, dy: -0 });
    expect(compassVec(8).dy).toBe(1024);
    expect(compassVec(12).dx).toBe(-1024);
  });
});

describe("layout v2: size from population (§9.1, D6)", () => {
  it("grows with log(population), capped at 240 × 160", () => {
    expect(townSize(3_100_000)).toEqual({ W: 240, H: 160 });
    const village = townSize(12_000);
    expect(village.W).toBeGreaterThanOrEqual(56);
    expect(village.W).toBeLessThanOrEqual(84);
    const town = townSize(135_000);
    expect(town.W).toBeGreaterThan(village.W);
    expect(town.W).toBeLessThan(240);
    for (const p of [5_000, 30_000, 300_000, 999_999]) {
      const s = townSize(p);
      expect(s.W % 4).toBe(0);
      expect(s.H % 4).toBe(0);
      expect(s.W).toBeLessThanOrEqual(240);
      expect(s.H).toBeLessThanOrEqual(160);
    }
    expect(squareSize(3_000_000).w).toBeGreaterThan(squareSize(10_000).w);
  });
});

describe("layout v2: real towns from the engine", () => {
  it("has a dozen towns picked by geography", () => {
    expect(GEO_FIXTURES.length).toBeGreaterThanOrEqual(12);
    expect(GEO_FIXTURES.some((f) => f.geo.river)).toBe(true);
    expect(GEO_FIXTURES.some((f) => f.geo.coast)).toBe(true);
    expect(GEO_FIXTURES.some((f) => f.geo.class === "metro")).toBe(true);
    expect(GEO_FIXTURES.some((f) => f.geo.relief.ruggedness_pm > 600)).toBe(true);
  });

  for (const f of GEO_FIXTURES) {
    it(`${label(f)} is a valid, readable town`, () => {
      const { plan, map } = layoutTownV2(f.input, f.geo, seedOf(f));
      expect(checkTownV2(map, plan, f.input)).toEqual([]);
      expect(readabilityProblems(map, plan)).toEqual([]);
      // Every neighbourhood appears as a district (D6), with cells and its own type.
      expect(map.districts.map((d) => d.id).sort((a, b) => a - b)).toEqual(f.input.neighborhoods.map((h) => h.id).sort((a, b) => a - b));
      for (const d of plan.districts) expect(d.cells).toBeGreaterThan(30);
      expect(map.width).toBeLessThanOrEqual(240);
      expect(map.height).toBeLessThanOrEqual(160);
      // Readability caps that hold for every band (§8).
      const r = readability(map, plan);
      expect(r.levels).toBeLessThanOrEqual(4);
      expect(r.landmarks).toBeGreaterThanOrEqual(3);
      expect(r.landmarks).toBeLessThanOrEqual(12);
      expect(r.riverBridges).toBeLessThanOrEqual(BAND_CAPS[bandOf(f.geo.baseline_population)].bridges);
    });
  }

  it("a river town draws its river across the map, entering on the side the engine says, with a bridge", () => {
    const rivers = GEO_FIXTURES.filter((f) => f.geo.river && f.geo.river.class >= 2);
    expect(rivers.length).toBeGreaterThan(3);
    for (const f of rivers) {
      const { plan } = layoutTownV2(f.input, f.geo, seedOf(f));
      const W = plan.width;
      const H = plan.height;
      const g = f.geo.river as NonNullable<GeoFixture["geo"]["river"]>;
      // River water on the map edge toward `enters` (within a quarter turn).
      const v = compassVec(g.enters);
      let edge = 0;
      for (let c = 0; c < W * H; c++) {
        if (plan.water[c] !== WaterKind.River) continue;
        const x = c % W;
        const y = Math.floor(c / W);
        if (x !== 0 && y !== 0 && x !== W - 1 && y !== H - 1) continue;
        if ((x - W / 2) * v.dx + (y - H / 2) * v.dy > 0) edge++;
      }
      expect({ town: f.input.municipality, edge: edge > 0 }).toEqual({ town: f.input.municipality, edge: true });
      expect(plan.connectors.some((k) => k.kind === "bridge" && k.crosses === "river" && k.name.endsWith("Bridge"))).toBe(true);
    }
  });

  it("a coastal town has its sea on the side the engine says", () => {
    for (const f of GEO_FIXTURES.filter((x) => x.geo.coast)) {
      const { plan } = layoutTownV2(f.input, f.geo, seedOf(f));
      const v = compassVec((f.geo.coast as NonNullable<GeoFixture["geo"]["coast"]>).sea);
      let sea = 0;
      let toward = 0;
      for (let c = 0; c < plan.width * plan.height; c++) {
        if (plan.water[c] !== WaterKind.Sea) continue;
        sea++;
        if (((c % plan.width) - plan.width / 2) * v.dx + (Math.floor(c / plan.width) - plan.height / 2) * v.dy > 0) toward++;
      }
      expect(sea).toBeGreaterThan(100);
      expect(toward / sea).toBeGreaterThan(0.9);
    }
  });

  it("mines have a headframe and a spoil heap; markets stalls on the square; civic buildings face the square", () => {
    for (const f of GEO_FIXTURES) {
      const { plan, map } = layoutTownV2(f.input, f.geo, seedOf(f));
      if (f.input.places.some((p) => p.kind === "mine")) {
        expect(map.props.some((p) => p.kind === "headframe")).toBe(true);
        expect(map.props.some((p) => p.kind === "spoil_heap")).toBe(true);
      }
      if (f.input.places.some((p) => p.kind === "market")) expect(map.props.filter((p) => p.kind === "stall").length).toBeGreaterThan(2);
      const th = map.buildings.find((b) => b.kind === "town_hall");
      if (th) expect(th.y + th.h).toBe(plan.square.y);
    }
  });
});

describe("layout v2: properties over random geographies", () => {
  it("every town is valid (places, doors, roofs, water, levels, bridges, reachability), and nearly every one keeps the §8 caps", () => {
    let unreadable = 0;
    const notes: string[] = [];
    for (let s = 1; s <= 120; s++) {
      const { input, geo } = randomGeo(s);
      const { plan, map } = layoutTownV2(input, geo, s * 7919);
      expect({ s, problems: checkTownV2(map, plan, input) }).toEqual({ s, problems: [] });
      const r = readabilityProblems(map, plan);
      if (r.length) {
        unreadable++;
        notes.push(`${s}: ${r.join("; ")}`);
      }
    }
    // Stage A: some generated towns (odd geographies: a village with four parliaments) still miss a cap, mostly one
    // built block under 6 × 5 or a long walk round the river; the real towns above miss none. At most 10%.
    expect(unreadable, notes.join("\n")).toBeLessThanOrEqual(12);
  });

  it("is deterministic: the same input gives the same plan and map, and input order never matters", () => {
    for (const s of [3, 17, 42]) {
      const { input, geo } = randomGeo(s);
      const a = layoutTownV2(input, geo, 99);
      const b = layoutTownV2(structuredClone(input), structuredClone(geo), 99);
      expect(planHash(b.plan)).toBe(planHash(a.plan));
      expect(dressHash(b.map)).toBe(dressHash(a.map));
      const shuffled = { ...input, neighborhoods: [...input.neighborhoods].reverse(), places: [...input.places].reverse() };
      const g2 = { ...geo, links: [...geo.links].reverse(), neighborhoods: [...geo.neighborhoods].reverse() };
      expect(planHash(layoutTownV2(shuffled, g2, 99).plan)).toBe(planHash(a.plan));
    }
  });

  it("the recipe snapshot plans the same town as the full registry (§6.6 option C)", () => {
    const f = GEO_FIXTURES[0] as GeoFixture;
    const { plan } = layoutTownV2(f.input, f.geo, seedOf(f));
    const again = planTown({ ...f.geo, places: f.input.places.map((p) => ({ ...p })) }, seedOf(f), 2, plan.recipe);
    expect(planHash(again)).toBe(planHash(plan));
    expect(plan.recipe.districts.length).toBeLessThanOrEqual(f.input.neighborhoods.length);
    expect(snapshotRecipe(BASE_RECIPE, { districts: [], features: [], connectors: [], places: [] }).districts).toEqual([]);
  });

  it("a town with no geography (flat, inland) still lays out", () => {
    const f = GEO_FIXTURES[1] as GeoFixture;
    const { plan, map } = layoutTownV2(f.input, geoFromInput(f.input, 80_000), 5);
    expect(checkTownV2(map, plan, f.input)).toEqual([]);
    expect(map.elevation?.every((v) => v === 0)).toBe(true);
  });
});

// Map hashes of the committed towns under the four dress states of §13. Layout v2 has not
// reached a saved run yet (stage A: the prototype only), so these pins may be refreshed until
// stage B ships it; after that a change that moves them bumps LAYOUT_VERSION instead.
describe("layout v2: pinned hashes", () => {
  const states = (f: GeoFixture, plan: ReturnType<typeof planTown>): Record<string, TownState> => ({
    start: { home: f.input.home, location: f.input.location },
    grow: { home: f.input.home, location: f.input.location, housingPm: 1300, pipelinePm: 40 },
    shrink: { home: f.input.home, location: f.input.location, hoods: plan.districts.map((d) => ({ id: d.id, population: Math.floor((d.baselinePopulation * 3) / 4) })), vacancy: 0.14 },
    closed: { home: f.input.home, location: f.input.location, closedPlaces: f.input.places.filter((p) => p.kind === "factory" || p.kind === "mine").map((p) => p.id) },
  });
  it("draws the committed towns identically", () => {
    const got: Record<string, string> = {};
    for (const f of GEO_FIXTURES) {
      const { plan } = layoutTownV2(f.input, f.geo, seedOf(f));
      const key = `${f.seed}/${f.source}`;
      got[`${key}/plan`] = planHash(plan);
      for (const [name, st] of Object.entries(states(f, plan))) got[`${key}/${name}`] = dressHash(dressTown(plan, st));
    }
    expect(got).toMatchSnapshot();
  });
});

describe("layout v2: the dress pass (D2, §9.2)", () => {
  // The largest town: enough reserved land in every district for growth and building sites
  // (the small towns' districts reserve only a few plots, see TOWN_GENERATION.md E2).
  const f = (GEO_FIXTURES.find((x) => x.source === "largest") ?? GEO_FIXTURES[0]) as GeoFixture;
  const plan = layoutTownV2(f.input, f.geo, seedOf(f)).plan;
  const base: TownState = { home: f.input.home, location: f.input.location };
  const built = (s: TownState) => new Set(dressTown(plan, s).buildings.filter((b) => b.state === "built" || b.state === "boarded" || b.state === "derelict").map((b) => b.index));

  it("never moves anything: every state has the same plots at the same places", () => {
    const a = dressTown(plan, base);
    const b = dressTown(plan, { ...base, housingPm: 1400, vacancy: 0.2 });
    expect(b.buildings.map((x) => [x.x, x.y, x.w, x.h, x.doorX])).toEqual(a.buildings.map((x) => [x.x, x.y, x.w, x.h, x.doorX]));
    expect(b.places).toEqual(a.places);
  });

  it("is monotonic: more housing only adds built plots; more vacancy only adds boarded ones", () => {
    let prev = built(base);
    for (const pm of [1050, 1150, 1300, 1500]) {
      const now = built({ ...base, housingPm: pm });
      for (const i of prev) expect(now.has(i)).toBe(true);
      expect(now.size).toBeGreaterThanOrEqual(prev.size);
      prev = now;
    }
    let boarded = new Set<number>();
    for (const v of [0.06, 0.1, 0.15, 0.25]) {
      const now = new Set(dressTown(plan, { ...base, vacancy: v }).buildings.filter((b) => b.state === "boarded").map((b) => b.index));
      for (const i of boarded) expect(now.has(i)).toBe(true);
      boarded = now;
    }
    expect(boarded.size).toBeGreaterThan(0);
  });

  it("reserves land at baseline, builds on it as the town grows, shows building sites", () => {
    const start = dressTown(plan, base);
    const grown = dressTown(plan, { ...base, housingPm: 1300, pipelinePm: 40 });
    expect(start.buildings.some((b) => b.state === "reserved")).toBe(true);
    expect(grown.buildings.filter((b) => b.state === "reserved").length).toBeLessThan(start.buildings.filter((b) => b.state === "reserved").length);
    expect(grown.buildings.some((b) => b.state === "construction")).toBe(true);
    // Reserved land is open ground: walkable, not a building.
    for (const b of start.buildings.filter((x) => x.state === "reserved")) expect(start.terrain[b.y * start.width + b.x]).not.toBe(Terrain.Building);
  });

  it("closes a works without removing it, shows derelict streets after a fall, keeps places standing", () => {
    const mine = f.input.places.find((p) => p.kind === "factory" || p.kind === "mine");
    expect(mine).toBeDefined();
    const closed = dressTown(plan, { ...base, closedPlaces: [mine?.id ?? -1] });
    const b = closed.buildings.find((x) => x.place === mine?.id);
    expect(b?.state).toBe("boarded");
    const fall = dressTown(plan, { ...base, hoods: plan.districts.map((d) => ({ id: d.id, population: Math.floor(d.baselinePopulation * 0.6) })) });
    expect(fall.buildings.some((x) => x.state === "derelict")).toBe(true);
    for (const p of fall.buildings.filter((x) => x.place !== null)) expect(p.state).toBe("built");
  });

  it("puts the player's home in their neighbourhood", () => {
    const m = dressTown(plan, base);
    if (f.input.home !== null) {
      const h = m.buildings[m.homeBuilding ?? -1];
      expect(h?.home).toBe(true);
      expect(m.districts[h?.district ?? -1]?.id).toBe(f.input.home);
    }
  });
});

describe("layout v2: the text alternative (DEC-051, §8)", () => {
  it("names every district, its edges and tags, the landmarks and a route to every place, as Fluent keys", () => {
    for (const f of GEO_FIXTURES.slice(0, 4)) {
      const { plan, map } = layoutTownV2(f.input, f.geo, seedOf(f));
      const t = describeTown(plan, map);
      expect(t.summary.key).toBe("town-layout-summary");
      expect(t.districts).toHaveLength(plan.districts.length);
      for (const d of t.districts) expect(d.lines[0]?.key).toBe("town-district");
      // Districts are described by what they border.
      expect(t.districts.filter((d) => d.lines.some((l) => l.key.startsWith("town-edge-"))).length).toBeGreaterThan(t.districts.length / 2);
      expect(t.routes).toHaveLength(map.places.length);
      for (const r of t.routes) {
        expect(r.key).toBe("town-route");
        expect(["short", "walk", "long"]).toContain(r.args.length);
      }
      for (const l of [t.summary, ...t.landmarks, ...t.routes, ...t.districts.flatMap((d) => d.lines)]) {
        expect(l.key).toMatch(/^[a-z][a-z0-9-]*$/);
        for (const v of Object.values(l.args)) expect(["string", "number"]).toContain(typeof v);
      }
      // The text and the map agree: a route over a bridge names one the map has.
      const names = new Set(plan.connectors.map((k) => k.name));
      for (const r of t.routes) if (r.args.bridge) expect(names.has(String(r.args.bridge))).toBe(true);
    }
  });
});

describe("layout v2: budgets (§13)", () => {
  it("plans the largest capital and dresses it within budget, stage by stage", () => {
    const f = GEO_FIXTURES.find((x) => x.source === "largest") as GeoFixture;
    // Warm up once (JIT), then measure the best of three: stage timings are reported, totals bounded.
    layoutTownV2(f.input, f.geo, 1);
    let best = Infinity;
    let stages: Record<string, number> = {};
    let dressMs = Infinity;
    for (let k = 0; k < 3; k++) {
      const t0 = performance.now();
      const plan = planTown({ ...f.geo, places: f.input.places.map((p) => ({ ...p })) }, seedOf(f));
      const t1 = performance.now();
      dressTown(plan, { home: null, location: f.input.location });
      dressMs = Math.min(dressMs, performance.now() - t1);
      if (t1 - t0 < best) {
        best = t1 - t0;
        stages = plan.stages;
      }
    }
    // Desktop target 60 ms (§13); this bound is generous for shared CI runners.
    expect(best).toBeLessThan(1500);
    for (const v of Object.values(stages)) expect(v).toBeLessThan(500);
    expect(dressMs).toBeLessThan(60);
    expect(Object.keys(stages)).toEqual(expect.arrayContaining(["terrain", "water", "centre", "arterials", "districts", "streets", "plots", "places", "fillers", "props", "checks"]));
  });
});
