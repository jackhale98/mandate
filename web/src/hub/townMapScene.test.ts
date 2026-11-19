import { FIXTURES, randomInput } from "./testTowns";
import { layoutTown, townSeed, type TownMap } from "./townLayout";
import { buildTownMapScene, hotspotOf, mapHotspots, routeLength, alongRoute, RouteCache, viewForInput } from "./townMapScene";
import { bakeTownMap, cellColours, pixelHash } from "./render/townMapRenderer";
import { newLog, stubCanvasFactory } from "./render/stubCanvas";
import type { TownInput } from "./townInput";
import type { TownPlacesView } from "../engine/protocol/TownPlacesView";

const layout = (input: TownInput, seed: number): TownMap => layoutTown(input, seed);

/** The list's entries (Town.tsx): every spot of every neighbourhood, by key, in order. */
function listKeys(view: TownPlacesView): string[] {
  const out: string[] = [];
  for (const h of view.neighborhoods)
    for (const s of h.spots) {
      const l = s.location.location;
      out.push(l.type === "home" ? "home" : l.type === "street" ? `streets:${l.neighborhood}` : `place:${l.place}`);
    }
  return out;
}

/** A view like the engine's: some kinds offer nothing yet (the list's "Also here"). */
function engineLikeView(input: TownInput): TownPlacesView {
  const v = viewForInput(input);
  const quiet = new Set(["school", "clinic", "park", "hotel", "farm", "mine", "mill"]);
  for (const h of v.neighborhoods) for (const s of h.spots) if (quiet.has(s.location.kind_key)) s.actions = [];
  return v;
}

describe("town map scene (DEC-086)", () => {
  it("has exactly one hotspot per entry of the place list, in the list's order, for real towns", () => {
    for (const f of FIXTURES) {
      const map = layout(f.input, townSeed(f.seed, f.input.municipality));
      const view = engineLikeView(f.input);
      const hs = mapHotspots(map, view);
      expect(hs.map((h) => h.key)).toEqual(listKeys(view));
      expect(new Set(hs.map((h) => h.key)).size).toBe(hs.length);
      // Every named place has exactly one hotspot, on its own building.
      for (const p of f.input.places) {
        const mine = hs.filter((h) => h.location.type === "place" && h.location.place === p.id);
        expect(mine).toHaveLength(1);
        const b = map.buildings[mine[0]?.building ?? -1];
        expect(b?.place).toBe(p.id);
        expect(mine[0]?.cell).toBe(b?.front);
      }
      // Quiet places are hotspots too, marked so the map says "nothing to do here yet" as the list does.
      for (const h of hs) {
        const spot = view.neighborhoods.flatMap((n) => n.spots).find((s) => JSON.stringify(s.location.location) === JSON.stringify(h.location));
        expect(h.active).toBe((spot?.actions.length ?? 0) + (spot?.services.length ?? 0) > 0);
        expect(hotspotOf(hs, h.location)).toBe(h);
      }
    }
  });

  it("covers generated towns: one hotspot per place, home and neighbourhood; every one reachable by a route", () => {
    for (let s = 1; s <= 60; s++) {
      const input = randomInput(s);
      const map = layout(input, s * 31);
      const view = viewForInput(input);
      const scene = buildTownMapScene(map, view);
      expect({ s, keys: scene.hotspots.map((h) => h.key) }).toEqual({ s, keys: listKeys(view) });
      const cache = new RouteCache(map);
      for (const h of scene.hotspots) {
        const r = cache.route(scene.hub, h.cell);
        expect({ s, key: h.key, ok: r !== null }).toEqual({ s, key: h.key, ok: true });
        // Routes run along the grid: every leg is horizontal, vertical or at 45° (a staircase is drawn as one diagonal).
        for (let i = 1; i < (r?.length ?? 0); i++) {
          const a = (r as { x: number; y: number }[])[i - 1] as { x: number; y: number };
          const b = (r as { x: number; y: number }[])[i] as { x: number; y: number };
          expect(a.x === b.x || a.y === b.y || Math.abs(b.x - a.x) === Math.abs(b.y - a.y)).toBe(true);
        }
      }
      // Hit areas of buildings never overlap (one building per tap).
      const rects = scene.hotspots.filter((h) => h.kind !== "streets").map((h) => h.rect);
      for (let i = 0; i < rects.length; i++)
        for (let j = i + 1; j < rects.length; j++) {
          const a = rects[i] as { x: number; y: number; w: number; h: number };
          const b = rects[j] as { x: number; y: number; w: number; h: number };
          const overlap = a.x < b.x + b.w && b.x < a.x + a.w && a.y + 1 < b.y + b.h && b.y + 1 < a.y + a.h;
          expect({ s, i, j, overlap }).toEqual({ s, i, j, overlap: false });
        }
    }
  });

  it("is deterministic: the same layout gives the same scene, routes and ground pixels", () => {
    for (const f of FIXTURES.slice(0, 4)) {
      const seed = townSeed(f.seed, f.input.municipality);
      const a = buildTownMapScene(layout(f.input, seed), viewForInput(f.input));
      const b = buildTownMapScene(layout(structuredClone(f.input), seed), viewForInput(structuredClone(f.input)));
      expect(b.hotspots).toEqual(a.hotspots);
      expect(b.routes).toEqual(a.routes);
      expect(b.hub).toBe(a.hub);
      expect(pixelHash(cellColours(b.map))).toBe(pixelHash(cellColours(a.map)));
    }
  });

  it("does not depend on where the player is (moving never re-bakes the map)", () => {
    const f = FIXTURES[0] as (typeof FIXTURES)[number];
    const seed = townSeed(f.seed, f.input.municipality);
    const elsewhere: TownInput = { ...f.input, location: { type: "street", neighborhood: (f.input.neighborhoods[1] ?? f.input.neighborhoods[0])?.id ?? 0 } };
    const a = buildTownMapScene(layout(f.input, seed), viewForInput(f.input));
    const b = buildTownMapScene(layout(elsewhere, seed), viewForInput(elsewhere));
    expect(b.hotspots).toEqual(a.hotspots);
    expect(pixelHash(cellColours(b.map))).toBe(pixelHash(cellColours(a.map)));
  });

  it("walks along a route at a steady pace and ends at the door", () => {
    const f = FIXTURES[1] as (typeof FIXTURES)[number];
    const map = layout(f.input, 7);
    const scene = buildTownMapScene(map, viewForInput(f.input));
    const far = scene.hotspots[scene.hotspots.length - 1];
    const r = new RouteCache(map).route(scene.hub, far?.cell ?? 0) ?? [];
    const len = routeLength(r);
    expect(len).toBeGreaterThan(0);
    expect(alongRoute(r, 0)).toEqual(r[0]);
    expect(alongRoute(r, len + 5)).toEqual(r[r.length - 1]);
    const end = r[r.length - 1] as { x: number; y: number };
    expect(Math.floor(end.y) * map.width + Math.floor(end.x)).toBe(far?.cell);
  });

  it("bakes every fixture with finite drawing calls, within memory bounds", () => {
    for (const f of FIXTURES) {
      const scene = buildTownMapScene(layout(f.input, townSeed(f.seed, f.input.municipality)), viewForInput(f.input));
      const log = newLog();
      const baked = bakeTownMap(scene, { createCanvas: stubCanvasFactory(log) });
      expect(baked.px).toBeGreaterThanOrEqual(4);
      expect(Math.max(baked.width, baked.height) * baked.px).toBeLessThanOrEqual(1400);
      // The map and its lights, plus the one-pixel-per-tile ground and the small texture tiles.
      expect(log.canvases).toBeLessThanOrEqual(12);
      expect(log.calls.fillRect ?? 0).toBeGreaterThan(scene.map.buildings.length);
    }
  });
});
