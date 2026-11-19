// The layout v2 registries (DEC-090 D4, §6): JSONC on the client in stage A,
// shaped as stage B's mandate-content types will be; the validator, and the
// geography adapter that stands in for the engine's view (D3).
import type { MapView } from "../../engine/protocol/MapView";
import type { MunicipalityView } from "../../engine/protocol/MunicipalityView";
import { chooseLabels, LABEL_CAP } from "../hubView";
import { geoFromViews } from "../townGeoAdapter";
import { NO_BEARING } from "./imath";
import { parseJsonc } from "./jsonc";
import { BASE_RECIPE, recipeFromSources, validateRecipe, type DistrictType } from "./registry";

describe("layout v2 registries", () => {
  it("reads JSONC: comments, trailing commas, strings with slashes", () => {
    expect(parseJsonc('// c\n{ "a": "x//y", /* b */ "b": [1, 2,], }')).toEqual({ a: "x//y", b: [1, 2] });
  });

  it("the base registries are valid and cover every neighbourhood kind", () => {
    expect(validateRecipe(BASE_RECIPE)).toEqual([]);
    expect(BASE_RECIPE.districts.length).toBeGreaterThanOrEqual(12);
    expect(BASE_RECIPE.features.length).toBeGreaterThanOrEqual(8);
    expect(BASE_RECIPE.connectors.length).toBeGreaterThanOrEqual(6);
    // Every place kind the worldgen makes has a layout block.
    for (const k of ["town_hall", "council_chamber", "market", "mine", "port", "factory", "mill", "school", "park"]) expect(BASE_RECIPE.places.some((p) => p.key === k)).toBe(true);
  });

  it("catches broken entries", () => {
    const d = structuredClone(BASE_RECIPE.districts[0]) as DistrictType;
    const bad = {
      ...BASE_RECIPE,
      districts: [
        ...BASE_RECIPE.districts,
        { ...d, id: "district.bad", appears: { ...d.appears, hood_kinds: ["moon"], weight: 0 }, streets: { ...d.streets, pattern: "spiral" as DistrictType["streets"]["pattern"], block: [3, 4] as [number, number] }, art: {} },
        d,
      ],
    };
    const problems = validateRecipe(bad).join("\n");
    expect(problems).toMatch(/unknown neighbourhood kind moon/);
    expect(problems).toMatch(/weight must be a positive integer/);
    expect(problems).toMatch(/unknown street pattern spiral/);
    expect(problems).toMatch(/minimum block 6 × 5/);
    expect(problems).toMatch(/art.family is required/);
    expect(problems).toMatch(/duplicate id/);
    const noFallback = { ...BASE_RECIPE, districts: BASE_RECIPE.districts.filter((x) => !x.appears.hood_kinds.includes("rural")) };
    expect(validateRecipe(noFallback).join("\n")).toMatch(/no unconditional type for neighbourhood kind rural/);
  });

  it("loads a pack's own registry text", () => {
    const r = recipeFromSources({
      districts: '{ "kind": "town_districts", "items": [] }',
      features: '{ "kind": "terrain_features", "items": [] }',
      connectors: '{ "kind": "connectors", "items": [] }',
      places: '{ "kind": "place_layouts", "items": [] }',
    });
    expect(r.districts).toEqual([]);
    expect(() => recipeFromSources({ districts: '{ "kind": "nope", "items": [] }', features: "{}", connectors: "{}", places: "{}" })).toThrow(/town_districts/);
  });
});

describe("the geography adapter (stage A stand-in for TownGeographyView)", () => {
  const W = 20;
  const H = 20;
  const raster = (f: (x: number, y: number) => number) => {
    let s = "";
    for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) s += String.fromCharCode(f(x, y));
    return btoa(s);
  };
  const map = (river: (x: number, y: number) => number, cover: (x: number, y: number) => number): MapView =>
    ({
      width: W,
      height: H,
      km_per_cell: 2.5,
      biome_b64: raster(() => 3),
      cover_b64: raster(cover),
      shade_b64: raster(() => 128),
      river_b64: raster(river),
      owner_b64: "",
      municipalities: [
        { id: 7, name: "Testford", region: 0, class: "town", x: 10, y: 10, population: 120000, unemployment: 0.05, port: false, regional_capital: false, national_capital: false },
        { id: 8, name: "Elsewhere", region: 0, class: "town", x: 10, y: 0, population: 50000, unemployment: 0.05, port: false, regional_capital: false, national_capital: false },
      ],
      regions: [],
      links: [{ mode: "road", path: [10 * W + 10, 9 * W + 10, 8 * W + 10, 7 * W + 10, 6 * W + 10, 0 * W + 10] }],
    }) as MapView;
  const muni = (over: Partial<MunicipalityView> = {}): MunicipalityView =>
    ({
      id: 7,
      name: "Testford",
      region: "R",
      class: "town",
      population: 120000,
      coastal: false,
      port: false,
      on_river: true,
      elevation_m: 80,
      temperature_c: 9,
      precipitation_mm: 700,
      biome: "temperate_forest",
      resources: [{ key: "minerals", value: 0.6 }, { key: "timber", value: 0.3 }],
      hazards: [{ key: "flood", value: 0.4 }],
      neighborhoods: [{ id: 1, name: "Core", kind: "core", population: 60000 }],
      places: [],
      economy: null,
      communities: [],
      top_concerns: [],
      ...over,
    }) as MunicipalityView;
  const input = { municipality: "Testford", neighborhoods: [{ id: 1, name: "Core", kind: "core" }], places: [], home: null, location: { type: "home" as const } };

  it("reads a river's axis, a link's bearing and the resources", () => {
    // A class-2 river running west–east through the centre.
    const g = geoFromViews({ municipality: muni(), map: map((_, y) => (y === 10 ? 2 : 0), () => 2), input });
    expect(g.river?.class).toBe(2);
    expect([4, 12]).toContain(g.river?.enters);
    expect(g.river?.through_centre).toBe(true);
    expect(g.links).toEqual([{ mode: "road", bearing: 0, to: 8, to_name: "Elsewhere", foreign: false }]);
    expect(g.minerals.value_pm).toBe(600);
    expect(g.hazards_pm.flood).toBe(400);
    expect(g.coast).toBeNull();
    expect(g.relief.uphill).toBe(NO_BEARING);
  });

  it("finds the sea's side", () => {
    // Sea to the east of the centre.
    const g = geoFromViews({ municipality: muni({ coastal: true, on_river: false }), map: map(() => 0, (x) => (x >= 12 ? 0 : 2)), input });
    expect(g.coast?.sea).toBe(4);
    expect(g.river).toBeNull();
  });
});

describe("hub labels (zoomed out they do not pile up)", () => {
  it("shows labels by priority, skipping any that overlap one shown, at most the cap", () => {
    const boxes = [
      { index: 0, x: 0, y: 0, w: 50, h: 10, priority: 1 },
      { index: 1, x: 10, y: 5, w: 50, h: 10, priority: 0 },
      { index: 2, x: 100, y: 0, w: 20, h: 10, priority: 2 },
    ];
    expect([...chooseLabels(boxes)].sort()).toEqual([1, 2]);
    const many = Array.from({ length: 40 }, (_, i) => ({ index: i, x: i * 100, y: 0, w: 50, h: 10, priority: i }));
    expect(chooseLabels(many, LABEL_CAP).size).toBe(12);
    // Unmeasured labels (no layout) never collide.
    expect(chooseLabels([{ index: 0, x: 0, y: 0, w: 0, h: 0, priority: 0 }, { index: 1, x: 0, y: 0, w: 0, h: 0, priority: 1 }]).size).toBe(2);
  });
});
