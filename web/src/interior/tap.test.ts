import { describe, expect, it } from "vitest";
import { composeInterior, interiorSeed, paletteData } from "../../../tools/pixelkit/art/mandate/pack";
import { RECIPES } from "../../../tools/pixelkit/art/mandate/interiorsContent";
import { recipeFor } from "../../../tools/pixelkit/art/mandate/interiorsRecipes";
import { playerSeat, retargetHotspots } from "./seat";
import { tapTarget, TILE_PX } from "./tap";

const { palette, lightness } = paletteData();
const recipe = recipeFor(RECIPES, "interior.council_chamber");
if (!recipe) throw new Error("chamber");
const T = TILE_PX;

describe("a tap on your own desk in the chamber", () => {
  for (const seed of [1, 2, 3, 4, 5, 6]) {
    const room = composeInterior(recipe, interiorSeed(seed, 4), palette, lightness);
    const variant = room.picks["choice:benches"];
    const ps = playerSeat(room);
    if (!ps) throw new Error("player spot");
    const hotspots = retargetHotspots(room, room.hotspots);
    const vote = hotspots.find((h) => h.id === "vote");
    if (!vote) throw new Error("vote hotspot");
    // Someone standing on the tile behind the seat: their figure reaches up over the seat's lower part.
    const behind = { key: "c7", x: ps.seat.x, y: ps.seat.y + 1 };

    it(`is yours where a neighbour's figure reaches over it (${variant}, seed ${seed})`, () => {
      // The seat's tile, in its lower half where the neighbour's figure box reaches: the seat's, not theirs.
      expect(tapTarget(room, hotspots, [behind], ps.seat.x * T + 8, ps.seat.y * T + 12)).toEqual({ kind: "hotspot", id: "vote" });
      // The desk in front of the seat too.
      expect(tapTarget(room, hotspots, [behind], vote.x * T + 8, vote.y * T + 8)).toEqual({ kind: "hotspot", id: "vote" });
    });

    it(`still talks to the neighbour when the tap is clearly on them (${variant}, seed ${seed})`, () => {
      expect(tapTarget(room, hotspots, [behind], behind.x * T + 8, behind.y * T + 8)).toEqual({ kind: "person", key: "c7" });
    });

    it(`is the seat's even with no one near, and nothing else's elsewhere (${variant}, seed ${seed})`, () => {
      expect(tapTarget(room, hotspots, [], ps.seat.x * T + 8, ps.seat.y * T + 8)).toEqual({ kind: "hotspot", id: "vote" });
      // A far corner of the room is neither.
      expect(tapTarget(room, hotspots, [behind], 0.5 * T, (room.rows - 0.5) * T)).toBeNull();
    });
  }

  it("leaves scenery staff alone and a person on the desk tile itself to talk to", () => {
    const room = composeInterior(recipe, interiorSeed(1, 4), palette, lightness);
    const ps = playerSeat(room);
    if (!ps) throw new Error("player spot");
    const hotspots = retargetHotspots(room, room.hotspots);
    const staff = { key: "x:clerk", x: ps.seat.x, y: ps.seat.y + 1, extra: true };
    expect(tapTarget(room, hotspots, [staff], ps.seat.x * T + 8, ps.seat.y * T + 12)).toEqual({ kind: "hotspot", id: "vote" });
    expect(tapTarget(room, hotspots, [staff], staff.x * T + 8, staff.y * T + 8)).toBeNull();
  });
});
