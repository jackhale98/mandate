import { fixtureTown } from "./testTowns";
import { TownScene, doorPlots } from "./townScene";

// Doors on the street's houses (DEC-114 §2): slot k of a neighbourhood is its
// k-th standing house by the plan's fill rank, the same house however often
// it is asked, never a place, a landmark or the player's home; and a house
// with a door is a door the player can stand at (Enter knocks).
vi.setConfig({ testTimeout: 60_000 });

describe("door plots", () => {
  it("put each neighbourhood's door slots on its houses by fill rank, the same every time", () => {
    const { map, plan } = fixtureTown(0);
    const ranks = plan.plots.map((p) => p.fillRank);
    const slots = new Map(map.districts.map((d) => [d.id, 6]));
    const a = doorPlots(map, ranks, slots);
    expect(a).toEqual(doorPlots(map, ranks, slots));
    expect(a.size).toBeGreaterThan(0);
    for (const [i, d] of a) {
      const b = map.buildings[i];
      if (!b) throw new Error("missing building");
      expect(b.place).toBeNull();
      expect(b.home || b.landmark).toBe(false);
      expect(map.districts[b.district]?.id).toBe(d.hood);
      expect(b.state === "reserved" || b.state === "construction").toBe(false);
    }
    // Within a neighbourhood the slots run 0, 1, 2… in fill rank order.
    for (const dist of map.districts) {
      const mine = [...a].filter(([, d]) => d.hood === dist.id).sort((x, y) => x[1].door - y[1].door);
      mine.forEach(([, d], k) => expect(d.door).toBe(k));
      for (let k = 1; k < mine.length; k++) expect(ranks[(mine[k] as [number, unknown])[0]]).toBeGreaterThanOrEqual(ranks[(mine[k - 1] as [number, unknown])[0]] as number);
    }
    // More slots only add doors: the first ones stay on the same houses.
    const more = doorPlots(map, ranks, new Map(map.districts.map((d) => [d.id, 12])));
    for (const [i, d] of a) expect(more.get(i)).toEqual(d);
  });

  it("a house with a door is one the player can stand at", () => {
    const { map, plan } = fixtureTown(0);
    const scene = new TownScene(map, { townsfolk: 0 });
    const doors = doorPlots(map, plan.plots.map((p) => p.fillRank), new Map(map.districts.map((d) => [d.id, 4])));
    const [first] = [...doors.keys()];
    if (first === undefined) throw new Error("no doors");
    const b = map.buildings[first];
    if (!b) throw new Error("missing building");
    scene.placePlayer(b.front);
    expect(scene.doorHere === null || !doors.has(scene.doorHere)).toBe(true);
    scene.doors = doors;
    const at = scene.doorHere;
    expect(at !== null && (doors.has(at) || map.buildings[at]?.place !== null || map.buildings[at]?.home)).toBe(true);
  });
});
