import { describe, expect, it } from "vitest";
import { composeInterior, interiorSeed, paletteData } from "../../../tools/pixelkit/art/mandate/pack";
import { castRoom, floorFromDoor, type CastRoom } from "./cast";
import { InteriorScene } from "./interiorScene";
import { RECIPES } from "../../../tools/pixelkit/art/mandate/interiorsContent";
import { recipeFor } from "../../../tools/pixelkit/art/mandate/interiorsRecipes";
import { enterable, recipeFromView } from "./recipes";

const { palette, lightness } = paletteData();
const room = (id: string, seed = 23) => {
  const r = recipeFor(RECIPES, id);
  if (!r) throw new Error(id);
  return { recipe: r, room: composeInterior(r, interiorSeed(seed, 4), palette, lightness) };
};

describe("interior recipes in the game", () => {
  it("come with the place view as the content's JSON; anything else keeps the panel", () => {
    const th = recipeFor(RECIPES, "interior.town_hall");
    const json = JSON.parse(JSON.stringify(th)) as unknown;
    expect(recipeFromView(json)?.version).toBe(2);
    expect(enterable(json)).toBe("interior.town_hall");
    expect(enterable(null)).toBeNull();
    expect(enterable({ id: "interior.half" })).toBeNull();
  });
});

describe("castRoom (who stands where)", () => {
  it("puts people on spots by how they are here, in the recipe's order, and fills the clerk's window with staff", () => {
    const { recipe, room: rm } = room("interior.town_hall");
    const placed = castRoom(rm, recipe, [
      { key: "c7", hereAs: "presiding" },
      { key: "a0", hereAs: "visitor" },
      { key: "a1", hereAs: "visitor" },
    ], { fullStaff: true });
    const at = (key: string) => placed.find((p) => p.key === key);
    // Presiding: the official's spot by the office door.
    expect(at("c7")).toMatchObject({ spot: "official", role: "official", extra: false });
    // Visitors take the public spots in recipe order.
    const publicSpots = rm.spots.filter((s) => s.role === "public").map((s) => s.id);
    expect(at("a0")?.spot).toBe(publicSpots[0]);
    expect(at("a1")?.spot).toBe(publicSpots[1]);
    // Nobody present works here: the clerks and staff are scenery.
    const extras = placed.filter((p) => p.extra);
    expect(extras.map((p) => p.spot)).toEqual(["clerk1", "clerk2", "staff", "files"]);
    expect(extras.every((p) => p.key.startsWith("x:"))).toBe(true);
    // The player's own seat is never given to anyone; no two share a tile.
    expect(placed.some((p) => p.role === "player")).toBe(false);
    expect(new Set(placed.map((p) => `${p.x},${p.y}`)).size).toBe(placed.length);
  });

  it("thins the staff in the evening, and someone who works here takes a staff spot before scenery", () => {
    const { recipe, room: rm } = room("interior.diner");
    const placed = castRoom(rm, recipe, [{ key: "c3", hereAs: "works" }], { fullStaff: false });
    expect(placed.find((p) => p.key === "c3")?.spot).toBe("owner");
    // One scenery cook and one waiter at most.
    expect(placed.filter((p) => p.extra).map((p) => p.role).sort()).toEqual(["cook", "waiter"]);
  });

  it("seats the chamber: the presiding member in the chair, members at the desks, sitters on seats", () => {
    const { recipe, room: rm } = room("interior.council_chamber");
    const people = [{ key: "c1", hereAs: "presiding" }, ...Array.from({ length: 6 }, (_, i) => ({ key: `c${10 + i}`, hereAs: "member" }))];
    const placed = castRoom(rm, recipe, people);
    expect(placed.find((p) => p.key === "c1")).toMatchObject({ spot: "chair", pose: "sit" });
    for (const p of placed.filter((x) => x.key !== "c1" && !x.extra)) expect(p.role).toBe("member");
    for (const p of placed.filter((x) => x.pose === "sit")) expect(rm.walk[p.y * rm.cols + p.x]).toBe(2);
  });

  it("stands extra people on free floor nearest the door, keeping the entrance and hotspot stands clear", () => {
    const { recipe, room: rm } = room("interior.town_hall");
    const crowd = Array.from({ length: 30 }, (_, i) => ({ key: `a${i}`, hereAs: "visitor" }));
    const placed = castRoom(rm, recipe, crowd);
    const loose = placed.filter((p) => p.spot === null);
    expect(loose.length).toBeGreaterThan(0);
    const door = rm.door;
    for (const p of loose) {
      expect(rm.walk[p.y * rm.cols + p.x]).toBe(1);
      expect(p.x === door.x && (p.y === door.y || p.y === door.y - 1)).toBe(false);
      expect(rm.hotspots.some((h) => h.stand.x === p.x && h.stand.y === p.y)).toBe(false);
    }
    expect(new Set(placed.map((p) => `${p.x},${p.y}`)).size).toBe(placed.length);
  });

  it("is deterministic and keeps a tiny room's order", () => {
    const tiny: CastRoom = {
      cols: 3,
      rows: 3,
      walk: new Uint8Array([1, 1, 1, 1, 0, 1, 1, 1, 1]),
      door: { x: 1, y: 2 },
      spots: [{ id: "s1", role: "public", x: 0, y: 0, facing: "down" }],
      hotspots: [],
    };
    const cast = { cast: { visitor: ["public"] }, extras: [] };
    const a = castRoom(tiny, cast, [{ key: "a0", hereAs: "visitor" }, { key: "a1", hereAs: "visitor" }]);
    expect(a).toEqual(castRoom(tiny, cast, [{ key: "a0", hereAs: "visitor" }, { key: "a1", hereAs: "visitor" }]));
    expect(a.map((p) => [p.key, p.spot, p.x, p.y])).toEqual([
      ["a0", "s1", 0, 0],
      ["a1", null, 0, 2],
    ]);
    expect(floorFromDoor(tiny)).toEqual([7, 6, 8, 3, 5, 0, 2, 1]);
  });
});

describe("InteriorScene (walking inside)", () => {
  it("routes around furniture and people, walks there and arrives facing the hotspot", () => {
    const { room: rm } = room("interior.town_hall");
    const scene = new InteriorScene(rm);
    const h = rm.hotspots.find((x) => x.id === "window1");
    if (!h) throw new Error("window1");
    let arrived = false;
    expect(scene.goTo(h.stand, () => (arrived = true), h.facing)).toBe(true);
    for (let i = 0; i < 200 && !arrived; i++) scene.step(0.05);
    expect(arrived).toBe(true);
    expect(scene.tile).toEqual(h.stand);
    expect(scene.player.dir).toBe("up");
    // Someone on a tile blocks it; walking up to them stops beside them.
    scene.setOccupied([{ x: 1, y: 4 }]);
    expect(scene.passable(1, 4)).toBe(false);
    let met = false;
    expect(scene.goNear({ x: 1, y: 4 }, () => (met = true))).toBe(true);
    for (let i = 0; i < 200 && !met; i++) scene.step(0.05);
    const t = scene.tile;
    expect(Math.abs(t.x - 1) + Math.abs(t.y - 4)).toBe(1);
  });

  it("steers tile by tile with the keyboard and stops at walls", () => {
    const { room: rm } = room("interior.diner");
    const scene = new InteriorScene(rm);
    expect(scene.atDoor).toBe(true);
    scene.steer(0, -1);
    scene.step(0.3);
    scene.steer(0, 0);
    for (let i = 0; i < 20; i++) scene.step(0.05);
    expect(scene.tile.y).toBeLessThan(rm.door.y);
    expect(scene.moving).toBe(false);
  });
});
