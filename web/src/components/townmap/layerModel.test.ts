import type { TownMarkerView } from "../../engine/protocol/TownMarkerView";
import { GEO_FIXTURES } from "../../hub/layoutV2/fixtures";
import { layoutTownV2 } from "../../hub/layoutV2";
import { townSeed, type TownMap } from "../../hub/townLayout";
import { buildTownMapScene, viewForInput } from "../../hub/townMapScene";
import { areaAnchor, chosenOption, clusterMarkers, fillPixels, hoodAnchor, keptBy, layerKey, locationAnchor, outlinePath, parseColour, placeMarkers, shadeColours } from "./layerModel";
import { marker } from "./fixture";

vi.setConfig({ testTimeout: 60_000 });

/** A 4×3 town: neighbourhood 10 on the left two columns, 20 on the right two; the top-right cell is outside the town. */
const tiny = {
  width: 4,
  height: 3,
  district: Int16Array.from([0, 0, 1, -1, 0, 0, 1, 1, 0, 0, 1, 1]),
  districts: [
    { index: 0, id: 10, name: "West", kind: "core", centre: { x: 1, y: 1.5 }, street: 0, cells: 6, bounds: { x: 0, y: 0, w: 2, h: 3 } },
    { index: 1, id: 20, name: "East", kind: "suburb", centre: { x: 3, y: 2 }, street: 0, cells: 5, bounds: { x: 2, y: 0, w: 2, h: 3 } },
  ],
} as unknown as TownMap;

/** The length of an outline path made of `M x y H x2` and `M x y V y2` segments, tiles. */
function pathLength(d: string): number {
  let n = 0;
  for (const m of d.matchAll(/M(\d+) (\d+)([HV])(\d+)/g)) n += Math.abs(Number(m[4]) - Number(m[3] === "H" ? m[1] : m[2]));
  return n;
}

describe("town map layers: geometry", () => {
  it("outlines a set of neighbourhoods along the cell edges with the set on one side only", () => {
    // West: a 2×3 block, perimeter 10, as four merged segments.
    expect(outlinePath(tiny, [10])).toBe("M0 0H2M0 3H2M0 0V3M2 0V3");
    // East is missing its top-right cell: perimeter 10 as well, with the notch.
    expect(pathLength(outlinePath(tiny, [20]))).toBe(10);
    // Both together: the town's outer edge; the edge between them is gone.
    const both = outlinePath(tiny, [10, 20]);
    expect(pathLength(both)).toBe(14);
    expect(both).not.toContain("M2 0V3");
    // A neighbourhood the layout does not have draws nothing.
    expect(outlinePath(tiny, [99])).toBe("");
  });

  it("marks a district at the centre of its largest neighbourhood, inside it", () => {
    expect(areaAnchor(tiny, [10, 20])).toEqual({ x: 1, y: 1.5 });
    expect(areaAnchor(tiny, [20])).toEqual({ x: 3, y: 2 });
    expect(areaAnchor(tiny, [])).toBeNull();
    expect(hoodAnchor(tiny, 20)).toEqual({ x: 3, y: 2 });
    expect(hoodAnchor(tiny, 99)).toBeNull();
  });

  it("paints each neighbourhood's cells and nothing outside the town; unknown is striped, never flat", () => {
    const px = fillPixels(
      tiny,
      new Map([
        [10, { rgba: [10, 20, 30, 0.5] }],
        [20, { rgba: [72, 76, 88, 0.5], hatch: true }],
      ]),
    );
    const at = (x: number, y: number) => [...px.slice((y * 4 + x) * 4, (y * 4 + x) * 4 + 4)];
    expect(at(0, 0)).toEqual([10, 20, 30, 128]);
    expect(at(1, 2)).toEqual([10, 20, 30, 128]);
    // Outside the town: transparent.
    expect(at(3, 0)).toEqual([0, 0, 0, 0]);
    // The unknown neighbourhood alternates two opacities along the diagonals.
    const alphas = new Set([at(2, 0), at(2, 1), at(2, 2), at(3, 1), at(3, 2)].map((p) => p[3]));
    expect(alphas.size).toBe(2);
    // A neighbourhood with no fill stays clear.
    expect([...fillPixels(tiny, new Map()).slice(0, 4)]).toEqual([0, 0, 0, 0]);
  });

  it("outlines every neighbourhood of a real town, each edge between two of them drawn for both", () => {
    const f = GEO_FIXTURES[0];
    if (!f) throw new Error("no fixture");
    const map = layoutTownV2(f.input, f.geo, townSeed(f.seed, f.input.municipality)).map;
    const hoods = map.districts.map((d) => d.id);
    const whole = pathLength(outlinePath(map, hoods));
    const each = hoods.map((h) => pathLength(outlinePath(map, [h])));
    expect(each.every((n) => n > 0)).toBe(true);
    // The shared edges are counted twice in the parts and not at all in the whole.
    expect(each.reduce((a, b) => a + b, 0)).toBeGreaterThan(whole);
    expect((each.reduce((a, b) => a + b, 0) - whole) % 2).toBe(0);
    for (const h of hoods) expect(hoodAnchor(map, h)).not.toBeNull();
  });
});

describe("town map layers: marks", () => {
  const f = GEO_FIXTURES[0];
  if (!f) throw new Error("no fixture");
  const map = layoutTownV2(f.input, f.geo, townSeed(f.seed, f.input.municipality)).map;
  const scene = buildTownMapScene(map, viewForInput(f.input));
  const places = scene.hotspots.filter((h) => h.kind === "place");
  const at = (i: number): TownMarkerView["at"] => {
    const h = places[i];
    if (!h) throw new Error("no place");
    return { location: h.location, kind_key: h.kindKey, name: h.name, neighborhood: h.neighborhood, neighborhood_name: h.neighborhoodName };
  };

  it("places a marker over its place's building, and leaves one with no known place unplaced", () => {
    const a = marker(1, { at: at(0) });
    const b = marker(2, { at: null, basis: "unknown" });
    const { placed, unplaced } = placeMarkers(scene.hotspots, [a, b]);
    expect(placed.map((p) => p.marker.person.id)).toEqual([1]);
    expect(unplaced.map((m) => m.person.id)).toEqual([2]);
    const h = places[0];
    expect(placed[0]?.spot).toBe(h?.key);
    expect(placed[0]?.at).toEqual(h ? { x: h.rect.x + h.rect.w / 2, y: h.rect.y + h.rect.h / 2 } : null);
    // Streets are marked at the neighbourhood's centre.
    const streets = scene.hotspots.find((x) => x.kind === "streets");
    expect(streets && locationAnchor(scene.hotspots, streets.location)?.at).toEqual(streets?.anchor);
  });

  it("gives people at one place one mark, and merges marks closer than a portrait at the zoom", () => {
    const { placed } = placeMarkers(scene.hotspots, [marker(1, { at: at(0) }), marker(2, { at: at(0) }), marker(3, { at: at(1) })]);
    // Zoomed far in: the two places are apart; the two people at one place still share a mark.
    const near = clusterMarkers(placed, 1000, 38);
    expect(near.map((c) => c.members.map((m) => m.marker.person.id))).toEqual([[1, 2], [3]]);
    expect(near[0]?.spot).toBe(places[0]?.key);
    // Zoomed far out: everything is within a portrait of everything else.
    const far = clusterMarkers(placed, 0.01, 38);
    expect(far).toHaveLength(1);
    expect(far[0]?.members).toHaveLength(3);
    expect(far[0]?.spot).toBeNull();
    // Every marker is in exactly one cluster at any zoom.
    for (const z of [0.5, 2, 4, 9, 28]) expect(clusterMarkers(placed, z, 38).flatMap((c) => c.members)).toHaveLength(3);
  });

  it("keeps the markers a filter lists, as the engine says", () => {
    const ms = [marker(1, { filters: ["all", "body"] }), marker(2, { filters: ["all", "met", "party:0"] }), marker(3, { filters: ["all"] })];
    expect(keptBy(ms, "all")).toHaveLength(3);
    expect(keptBy(ms, "body").map((m) => m.person.id)).toEqual([1]);
    expect(keptBy(ms, "party:0").map((m) => m.person.id)).toEqual([2]);
    expect(keptBy(ms, "party:9")).toEqual([]);
  });
});

describe("town map layers: colours and choices", () => {
  it("names a layer by its content id, colours a magnitude light to dark and categories apart", () => {
    expect(layerKey({ id: "town_map_layer.people" })).toBe("people");
    const bin = (label_key: string) => ({ label_key, party: null, count: 1 });
    const seq = shadeColours({ id: "concern.jobs", scale: "sequential", bins: [bin("a"), bin("b"), bin("c")] });
    expect(seq).toHaveLength(3);
    // One hue, lightest first: each step darker than the last.
    const light = seq.map((c) => parseColour(c).reduce((a, b) => a + b, 0));
    expect(light[0]).toBeGreaterThan(light[1] ?? 0);
    expect(light[1]).toBeGreaterThan(light[2] ?? 0);
    const cat = shadeColours({ id: "top", scale: "categorical", bins: [bin("concern-jobs"), bin("concern-housing"), bin("ui-map-legend-other")] });
    expect(new Set(cat).size).toBe(3);
    expect(parseColour("rgb(42, 120, 214)")).toEqual([42, 120, 214]);
    expect(parseColour("#2a78d6")).toEqual([42, 120, 214]);
  });

  it("falls back to a shaded layer's first choice", () => {
    const o = (id: string) => ({ id, label_key: id, scale: "sequential" as const, bins: [], cells: [], unknown: 0 });
    const options = [o("top"), o("concern.jobs")];
    expect(chosenOption(options, "concern.jobs")?.id).toBe("concern.jobs");
    expect(chosenOption(options, undefined)?.id).toBe("top");
    expect(chosenOption(options, "gone")?.id).toBe("top");
    expect(chosenOption([], "top")).toBeNull();
  });
});
