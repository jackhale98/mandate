import type { TownPlacesView } from "../engine/protocol/TownPlacesView";
import { checkTownV2 } from "./layoutV2/check";
import { geoFromInput, layoutTownV2 } from "./layoutV2";
import { townInputFromMunicipality, townInputFromView } from "./townInput";
import { buildingOfPlace, townSeed } from "./townLayout";

describe("adapters from the protocol views", () => {
  it("reads the player's town from TownPlacesView, and the layout puts the player where the engine says", () => {
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
        { id: 1, name: "Centre", kind: "core", is_home: false, in_ward: false, top_concern_key: null, concern_knowledge: { level: "unknown", as_of: null, stale: false, lead: null }, spots: [loc({ type: "street", neighborhood: 1 }, "streets", "", 1), loc({ type: "place", place: 7 }, "town_hall", "", 1)] },
        { id: 2, name: "Leafy", kind: "suburb", is_home: true, in_ward: true, top_concern_key: null, concern_knowledge: { level: "unknown", as_of: null, stale: false, lead: null }, spots: [loc({ type: "home" }, "home", "", 2), loc({ type: "place", place: 9 }, "diner", "Vera's", 2)] },
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
    const { plan, map } = layoutTownV2(input, geoFromInput(input), townSeed("1", input.municipality), { home: input.home, location: input.location });
    expect(checkTownV2(map, plan, input)).toEqual([]);
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
    const { plan, map } = layoutTownV2(input, geoFromInput(input), 2, { home: null, location: input.location });
    expect(checkTownV2(map, plan, input)).toEqual([]);
    expect(map.homeBuilding).toBeNull();
    expect(map.districts[map.district[map.start] as number]?.id).toBe(5);
  });
});
