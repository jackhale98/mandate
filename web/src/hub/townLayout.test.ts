import { FIXTURES, randomInput, type Fixture } from "./testTowns";
import { checkTown, distanceToSquare } from "./townCheck";
import { townInputFromMunicipality, townInputFromView, type TownInput } from "./townInput";
import { LAYOUT_VERSION, Terrain, buildingOfPlace, layoutTown, locationCell, mapHash, placeSize, townSeed, type TownMap } from "./townLayout";
import type { TownPlacesView } from "../engine/protocol/TownPlacesView";

const civicOnSquare = (map: TownMap, input: TownInput, kind: string) => {
  const p = input.places.find((x) => x.kind === kind);
  const b = p ? buildingOfPlace(map, p.id) : null;
  return b ? distanceToSquare(map, b.index) : null;
};

describe("town layout: real towns from the engine", () => {
  it("has 10+ fixtures dumped from the engine", () => {
    expect(FIXTURES.length).toBeGreaterThanOrEqual(10);
  });

  for (const f of FIXTURES) {
    it(`${f.input.municipality} (seed ${f.seed}, ${f.source}) is a valid town`, () => {
      for (const seed of [1, townSeed(f.seed, f.input.municipality)]) {
        const map = layoutTown(f.input, seed);
        expect(checkTown(map, f.input)).toEqual([]);
        // The town hall and the council chamber face the central square.
        expect(civicOnSquare(map, f.input, "town_hall")).toBeLessThanOrEqual(1);
        expect(civicOnSquare(map, f.input, "council_chamber")).toBeLessThanOrEqual(3);
        // The player starts where the engine says they are.
        expect(map.start).toBe(locationCell(map, f.input.location));
      }
    });
  }
});

describe("town layout: properties over generated towns", () => {
  it("every generated town is valid (places, doors, reachability, footprints, districts)", () => {
    for (let s = 1; s <= 120; s++) {
      const input = randomInput(s);
      const map = layoutTown(input, s * 7919);
      expect({ s, problems: checkTown(map, input) }).toEqual({ s, problems: [] });
      const th = input.places.find((p) => p.kind === "town_hall");
      const cc = input.places.find((p) => p.kind === "council_chamber");
      if (th) expect(civicOnSquare(map, input, "town_hall")).toBeLessThanOrEqual(1);
      if (cc && th && cc.neighborhood === th.neighborhood) expect(civicOnSquare(map, input, "council_chamber")).toBeLessThanOrEqual(3);
    }
  });

  it("is deterministic: same input and seed give the identical map", () => {
    for (const s of [3, 17, 42]) {
      const input = randomInput(s);
      const a = layoutTown(input, 99);
      const b = layoutTown(structuredClone(input), 99);
      expect(b).toEqual(a);
      expect(mapHash(b)).toBe(mapHash(a));
    }
  });

  it("does not depend on the order the engine lists things in", () => {
    const input = (FIXTURES[0] as Fixture).input;
    const shuffled = { ...input, neighborhoods: [...input.neighborhoods].reverse(), places: [...input.places].reverse() };
    expect(mapHash(layoutTown(shuffled, 5))).toBe(mapHash(layoutTown(input, 5)));
  });

  it("a different seed gives a different town", () => {
    for (const f of FIXTURES.slice(0, 4)) {
      const hashes = new Set([1, 2, 3, 4].map((s) => mapHash(layoutTown(f.input, s))));
      expect(hashes.size).toBe(4);
    }
  });

  it("sizes landmarks by kind and fills the town with unnamed buildings", () => {
    const f = FIXTURES[0] as Fixture;
    const map = layoutTown(f.input, 11);
    const area = (kind: string) => {
      const b = map.buildings.find((x) => x.kind === kind);
      return b ? b.w * b.h : 0;
    };
    expect(area("town_hall")).toBeGreaterThan(area("diner"));
    expect(area("council_chamber")).toBeGreaterThan(area("party_office"));
    expect(placeSize("town_hall")[0]).toBeGreaterThanOrEqual(9);
    const fillers = map.buildings.filter((b) => b.place === null && !b.home);
    expect(fillers.length).toBeGreaterThan(2 * f.input.places.length);
    expect(map.buildings.filter((b) => b.home)).toHaveLength(1);
  });

  it("puts water by a waterfront and a port on it", () => {
    let checked = 0;
    for (const f of FIXTURES) {
      const map = layoutTown(f.input, 3);
      const waterfront = f.input.neighborhoods.some((h) => h.kind === "waterfront") || f.input.places.some((p) => p.kind === "port");
      expect(map.water !== null).toBe(waterfront);
      if (!waterfront) continue;
      checked++;
      expect(map.terrain.includes(Terrain.Water)).toBe(true);
      expect(map.terrain.includes(Terrain.Quay)).toBe(true);
    }
    expect(checked).toBeGreaterThan(0);
  });
});

describe("layout stability across app updates", () => {
  // If this fails, the layout algorithm changed what a saved town looks like.
  // Either undo the change or bump LAYOUT_VERSION and keep the old version
  // callable for towns already laid out with it (DEC-084 proposal), then
  // update these hashes.
  const PINNED: Record<string, string> = {
    "4242/town": "9e660a8a",
    "4242/largest": "ffd0a413",
    "2026/town": "697f4c95",
    "777/town": "5020d629",
  };
  it(`layout version ${LAYOUT_VERSION} still draws the committed towns identically`, () => {
    const got: Record<string, string> = {};
    for (const key of Object.keys(PINNED)) {
      const f = FIXTURES.find((x) => `${x.seed}/${x.source}` === key) as Fixture;
      got[key] = mapHash(layoutTown(f.input, townSeed(f.seed, f.input.municipality)));
    }
    expect(got).toEqual(PINNED);
  });

  it("refuses a layout version this build does not have", () => {
    const f = FIXTURES[0] as Fixture;
    expect(() => layoutTown(f.input, 1, LAYOUT_VERSION + 1)).toThrow(/unknown town layout version/);
    expect(layoutTown(f.input, 1, LAYOUT_VERSION).version).toBe(LAYOUT_VERSION);
  });
});

describe("adapters from the protocol views", () => {
  it("reads the player's town from TownPlacesView", () => {
    const loc = (location: TownPlacesView["location"], kind_key: string, name: string, hood: number) => ({
      location: { location, kind_key, name, neighborhood: hood, neighborhood_name: "" },
      actions: [],
      late_night: [],
      services: [],
      here: false,
    });
    const view: TownPlacesView = {
      municipality: "Ashford",
      location: { type: "place", place: 7 },
      home_neighborhood: 2,
      character: null,
      neighborhoods: [
        { id: 1, name: "Centre", kind: "core", is_home: false, in_ward: false, spots: [loc({ type: "street", neighborhood: 1 }, "streets", "", 1), loc({ type: "place", place: 7 }, "town_hall", "", 1)] },
        { id: 2, name: "Leafy", kind: "suburb", is_home: true, in_ward: true, spots: [loc({ type: "home" }, "home", "", 2), loc({ type: "place", place: 9 }, "diner", "Vera's", 2)] },
      ],
    };
    const input = townInputFromView(view);
    expect(input).toEqual({
      municipality: "Ashford",
      neighborhoods: [
        { id: 1, name: "Centre", kind: "core" },
        { id: 2, name: "Leafy", kind: "suburb" },
      ],
      places: [
        { id: 7, kind: "town_hall", name: "", neighborhood: 1 },
        { id: 9, kind: "diner", name: "Vera's", neighborhood: 2 },
      ],
      home: 2,
      location: { type: "place", place: 7 },
    });
    const map = layoutTown(input, 1);
    expect(checkTown(map, input)).toEqual([]);
    expect(map.start).toBe(buildingOfPlace(map, 7)?.front);
  });

  it("reads any town from MunicipalityView (a visit: no home, start on the central streets)", () => {
    const input = townInputFromMunicipality({
      id: 3,
      name: "Belstead",
      neighborhoods: [
        { id: 4, name: "Docks", kind: "waterfront", population: 1 },
        { id: 5, name: "Middle", kind: "core", population: 1 },
      ],
      places: [{ id: 8, kind: "port", name: "", neighborhood: 4 }],
    } as never);
    expect(input.home).toBeNull();
    expect(input.location).toEqual({ type: "street", neighborhood: 5 });
    const map = layoutTown(input, 2);
    expect(checkTown(map, input)).toEqual([]);
    expect(map.homeBuilding).toBeNull();
    expect(map.district[map.start]).toBe(1);
  });
});
