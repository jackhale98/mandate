// Town layout v2 street rework (DEC-090 D5 amended, "A + B"; street ends): the
// checks that would have caught each fault the owner reported, shown by
// putting each fault into a valid town; and the street network's properties
// over the real towns (the 13 fixtures and the owner's seed 632, Brinerford)
// and seeded random geographies.
import { StreetAxis, Terrain, townSeed, type TownMap } from "../townLayout";
import { streetProblems } from "./check";
import { GEO_FIXTURES, randomGeo, type GeoFixture } from "./fixtures";
import { layoutTownV2, type TownPlan } from "./index";
import { boxSeg, makeSeg, segCells, segSteps } from "./net";
import owner632 from "./owner-seed632.json";
import { RoadClass, type Segment } from "./work";

vi.setConfig({ testTimeout: 120_000 });

const OWNER = owner632 as unknown as GeoFixture[];
const REAL = [...GEO_FIXTURES, ...OWNER];
const seedOf = (f: GeoFixture) => townSeed(f.seed, f.input.municipality);
const label = (f: GeoFixture) => `${f.input.municipality} (seed ${f.seed}, ${f.source})`;

/** A town and a deep copy to put a fault into. */
function town(f: GeoFixture): { plan: TownPlan; map: TownMap } {
  return layoutTownV2(f.input, f.geo, seedOf(f));
}
function copy(t: { plan: TownPlan; map: TownMap }): { plan: TownPlan; map: TownMap } {
  const plan: TownPlan = { ...t.plan, segments: t.plan.segments.map((s) => ({ ...s, ends: [...s.ends] as Segment["ends"] })), rail: t.plan.rail.map((s) => ({ ...s })), arterials: t.plan.arterials.map((a) => ({ ...a })), streetEnds: t.plan.streetEnds.map((e) => ({ ...e })) };
  const map: TownMap = { ...t.map, terrain: new Uint8Array(t.map.terrain), props: [...t.map.props], streetEnds: (t.map.streetEnds ?? []).map((e) => ({ ...e })) };
  return { plan, map };
}
/** Pave a run onto the map and add it to the plan, as a faulty generator would have. */
function lay(t: { plan: TownPlan; map: TownMap }, s: Segment) {
  segCells(s, t.map.width, t.map.height, (c, k) => (t.map.terrain[c] = k === 0 ? Terrain.Pavement : Terrain.Road));
  t.plan.segments.push(s);
}
/** Open land: a square patch of the map with no street, water, rail or building on it. */
function openPatch(t: { plan: TownPlan; map: TownMap }, size = 14): { x: number; y: number } {
  const { map } = t;
  const W = map.width;
  const free = (c: number) => [Terrain.Grass, Terrain.Field, Terrain.Park, Terrain.Garden, Terrain.Yard, Terrain.Sand, Terrain.Reeds].includes(map.terrain[c] as 0);
  for (let y = 3; y + size < map.height - 3; y++) {
    for (let x = 3; x + size < W - 3; x++) {
      let ok = true;
      for (let yy = y; yy < y + size && ok; yy++) for (let xx = x; xx < x + size && ok; xx++) if (!free(yy * W + xx)) ok = false;
      if (ok) return { x, y };
    }
  }
  throw new Error("no open patch");
}
const base = { cls: RoadClass.Street, style: "asphalt" as const, district: 0, arterial: -1 };

describe("street rework: the checks catch every fault the owner reported", () => {
  const f = REAL.find((x) => x.seed === "632") as GeoFixture;
  const good = town(f);

  it("the owner's town (seed 632, Brinerford) has none of them now", () => {
    expect(streetProblems(good.map, good.plan)).toEqual([]);
  });

  it("a stub that ends in nothing", () => {
    const t = copy(good);
    const p = openPatch(t);
    lay(t, makeSeg(p.x + 1, p.y + 6, 0, 10, 3, base));
    expect(streetProblems(t.map, t.plan).some((m) => m.includes("ends in nothing"))).toBe(true);
  });

  it("a road starting from the wrong side, joining nothing", () => {
    const t = copy(good);
    const p = openPatch(t);
    lay(t, makeSeg(p.x + 1, p.y + 6, 0, 10, 3, base));
    expect(streetProblems(t.map, t.plan).some((m) => m.includes("does not join the town's streets"))).toBe(true);
  });

  it("near-parallel duplicate roads", () => {
    const t = copy(good);
    const s = t.plan.segments.find((x) => x.axis === "h" && segSteps(x) >= 10 && x.district >= 0) as Segment;
    lay(t, makeSeg(s.ax, s.ay + s.width + 2, 0, segSteps(s), s.width, base));
    expect(streetProblems(t.map, t.plan).some((m) => m.includes("run alongside"))).toBe(true);
  });

  it("roads that should meet but jog a few cells across a junction", () => {
    const t = copy(good);
    const p = openPatch(t);
    // A cross street, a street ending on it from the north, and one leaving it to the south three cells aside.
    lay(t, boxSeg("v", p.x + 6, p.y + 1, p.x + 9, p.y + 13, base));
    lay(t, makeSeg(p.x + 1, p.y + 5, 0, 4, 3, base));
    lay(t, makeSeg(p.x + 10, p.y + 8, 0, 3, 3, base));
    expect(streetProblems(t.map, t.plan).some((m) => m.includes(" jog "))).toBe(true);
  });

  it("a road meeting the railway with no crossing (stopping at it, or broken by it)", () => {
    const t = copy(good);
    const r = t.plan.rail.find((x) => segSteps(x) > 30) as Segment;
    const W = t.map.width;
    // Up to the line from beside it, square-on, and stopping short of the track.
    const mid = r.axis === "h" ? { x: r.ax + (segSteps(r) >> 1), y: r.ay } : { x: r.ax, y: r.ay + (segSteps(r) >> 1) };
    const into = r.axis === "h" ? makeSeg(mid.x, mid.y + 12, 2, 10, 3, base) : makeSeg(mid.x + 12, mid.y, 4, 10, 3, base);
    lay(t, into);
    const across = r.axis === "h" ? makeSeg(mid.x + 20, mid.y - 6, 6, 12, 3, base) : makeSeg(mid.x - 6, mid.y + 20, 0, 12, 3, base);
    t.plan.segments.push(across);
    segCells(across, W, t.map.height, (c) => {
      if (t.map.terrain[c] !== Terrain.Rail) t.map.terrain[c] = Terrain.Road;
    });
    const probs = streetProblems(t.map, t.plan);
    expect(probs.some((m) => m.includes("stops at the railway"))).toBe(true);
    expect(probs.some((m) => m.includes("meets the railway with no crossing"))).toBe(true);
  });

  it("the railway looping round the centre", () => {
    const t = copy(good);
    const sq = t.plan.square;
    const rb = { cls: RoadClass.None, style: "path" as const, district: -1, arterial: -1 };
    const W = t.map.width;
    t.plan.rail = [boxSeg("h", 0, sq.y - 20, sq.x + sq.w + 12, sq.y - 19, rb), boxSeg("v", sq.x + sq.w + 11, sq.y - 20, sq.x + sq.w + 12, sq.y + sq.h + 12, rb), boxSeg("h", 0, sq.y + sq.h + 11, sq.x + sq.w + 12, sq.y + sq.h + 12, rb)];
    for (const r of t.plan.rail) segCells(r, W, t.map.height, (c) => (t.map.terrain[c] = Terrain.Rail));
    expect(streetProblems(t.map, t.plan).some((m) => m.includes("the railway loops") || m.includes("does not run edge to edge"))).toBe(true);
  });

  it("an arterial in staircase zigzags", () => {
    const t = copy(good);
    const a = t.plan.arterials.length;
    t.plan.arterials.push({ index: a, name: "Zigzag Road", mode: "road", bearing: 0, toName: "", length: 0 });
    const p = openPatch(t);
    const ab = { ...base, cls: RoadClass.Arterial, arterial: a };
    for (let k = 0; k < 3; k++) {
      t.plan.segments.push(boxSeg("h", p.x + k * 3, p.y + k * 3, p.x + k * 3 + 4, p.y + k * 3 + 3, ab));
      t.plan.segments.push(boxSeg("v", p.x + k * 3 + 4, p.y + k * 3, p.x + k * 3 + 7, p.y + k * 3 + 4, ab));
    }
    expect(streetProblems(t.map, t.plan).some((m) => m.includes("zigzags"))).toBe(true);
  });

  it("a town street running out to the map's edge (only the roads to other towns may)", () => {
    const t = copy(good);
    const s = t.plan.segments.find((x) => x.district >= 0 && x.axis === "h" && segSteps(x) > 6) as Segment;
    lay(t, makeSeg(0, s.ay, 0, s.ax, s.width, base));
    expect(streetProblems(t.map, t.plan).some((m) => m.includes("reaches the map's edge") || m.includes("leaves the map"))).toBe(true);
  });

  it("a street end that is not recorded, or a termination missing what makes it one", () => {
    const t = copy(good);
    const i = t.plan.segments.findIndex((x) => x.ends.some((e) => e !== null));
    expect(i).toBeGreaterThanOrEqual(0);
    t.map.streetEnds = [];
    expect(streetProblems(t.map, t.plan).some((m) => m.includes("is not recorded"))).toBe(true);
    const u = copy(good);
    u.map.props = u.map.props.filter((p) => p.kind !== "field_gate" && p.kind !== "bollards" && p.kind !== "railing");
    const kinds = new Set(u.plan.streetEnds.map((e) => e.kind));
    if (kinds.has("field_gate") || kinds.has("bollards") || kinds.has("railing")) expect(streetProblems(u.map, u.plan).some((m) => / has no (gate|bollards|railing)/.test(m))).toBe(true);
  });
});

describe("street rework: the real towns", () => {
  for (const f of REAL) {
    it(`${label(f)}: grown streets, 45° runs, clean arterials, recorded ends`, () => {
      const { plan, map } = town(f);
      expect(streetProblems(map, plan)).toEqual([]);
      const W = map.width;
      // Streets of several widths (lanes, streets, high streets).
      expect(new Set(plan.segments.map((s) => (s.axis === "h" || s.axis === "v" ? s.width : s.width - 1))).size).toBeGreaterThan(1);
      // The diagonal streets' cells carry their direction; the art reads it.
      const diag = plan.segments.filter((s) => s.axis === "ne" || s.axis === "nw");
      for (const s of diag) {
        let marked = 0;
        segCells(s, W, map.height, (c) => {
          if ((map.streetAxis?.[c] ?? 0) !== StreetAxis.Orthogonal) marked++;
        });
        expect(marked).toBeGreaterThan(0);
      }
      // Every recorded end is a kind the art knows, on the map.
      for (const e of map.streetEnds ?? []) {
        expect(["turning_circle", "hammerhead", "bollards", "plaza", "field_gate", "railing"]).toContain(e.kind);
        expect(e.x).toBeGreaterThanOrEqual(0);
        expect(e.x + e.w).toBeLessThanOrEqual(W);
      }
      // Only the roads to the neighbouring towns reach the map's edge, each from a real link.
      for (const a of plan.arterials) if (a.mode === "local") expect(plan.segments.filter((s) => s.arterial === a.index).every((s) => s.x0 > 0 && s.y0 > 0 && s.x1 < W - 1 && s.y1 < map.height - 1)).toBe(true);
    });
  }

  it("most real towns have 45° streets (radials, along the river or shore, on the contours)", () => {
    let with45 = 0;
    for (const f of REAL) if (town(f).plan.segments.some((s) => s.axis === "ne" || s.axis === "nw")) with45++;
    expect(with45).toBeGreaterThan(REAL.length * 0.6);
  });

  it("the railway passes the town on a plausible line, with the station beside it", () => {
    let towns = 0;
    for (const f of [...OWNER, ...Array.from({ length: 40 }, (_, k) => randomGeo(400 + k)).filter((g) => g.geo.links.some((l) => l.mode === "rail")).map((g, k) => ({ ...g, seed: String(9000 + k), source: "town" as const, state: { vacancy: 0, unemployment: 0, hoods: [] } }))]) {
      const { plan, map } = town(f);
      if (!plan.rail.length) continue;
      towns++;
      expect(plan.rail.length).toBeLessThanOrEqual(3);
      expect({ seed: f.seed, problems: streetProblems(map, plan).filter((m) => m.includes("railway") || m.includes("station")) }).toEqual({ seed: f.seed, problems: [] });
    }
    expect(towns).toBeGreaterThan(5);
  });
});

describe("street rework: random geographies", () => {
  it("no connection fault anywhere (zero tolerance)", () => {
    for (let s = 1; s <= 120; s++) {
      const { input, geo } = randomGeo(s);
      const { plan, map } = layoutTownV2(input, geo, s * 7919);
      expect({ s, problems: streetProblems(map, plan) }).toEqual({ s, problems: [] });
    }
  });
});
