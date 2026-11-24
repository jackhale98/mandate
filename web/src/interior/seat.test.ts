import { describe, expect, it } from "vitest";
import { composeInterior, interiorSeed, paletteData } from "../../../tools/pixelkit/art/mandate/pack";
import { RECIPES } from "../../../tools/pixelkit/art/mandate/interiorsContent";
import { recipeFor } from "../../../tools/pixelkit/art/mandate/interiorsRecipes";
import { castRoom, type CastPerson } from "./cast";
import { InteriorScene } from "./interiorScene";
import { isSeatHotspot, playerSeat, retargetHotspots, seatRoute, seatedSpots } from "./seat";

const { palette, lightness } = paletteData();
const recipe = (() => {
  const r = recipeFor(RECIPES, "interior.council_chamber");
  if (!r) throw new Error("chamber");
  return r;
})();

/** The chamber for some seeds, covering every variant of its benches. */
const rooms = (() => {
  const out: { variant: string; seed: number; room: ReturnType<typeof composeInterior> }[] = [];
  for (let seed = 1; seed <= 60; seed++) {
    const room = composeInterior(recipe, interiorSeed(seed, 4), palette, lightness);
    out.push({ variant: room.picks["choice:benches"] ?? "?", seed, room });
  }
  return out;
})();

/** A body of `size` people: the player and the rest, one presiding. */
const body = (size: number, visitors = 0): CastPerson[] => [
  { key: "c1", hereAs: "presiding" },
  ...Array.from({ length: size - 2 }, (_, i) => ({ key: `c${10 + i}`, hereAs: "member" })),
  ...Array.from({ length: visitors }, (_, i) => ({ key: `a${i}`, hereAs: "visitor" })),
];

describe("the player's desk in the chamber", () => {
  it("is exercised in every variant of the benches", () => {
    expect(new Set(rooms.map((r) => r.variant))).toEqual(new Set(["horseshoe", "rows"]));
  });

  it("is the desk nearest the door by walking distance along clear floor, in every variant", () => {
    for (const { variant, seed, room } of rooms) {
      const ps = playerSeat(room);
      if (!ps) throw new Error("no player spot");
      const route = seatRoute(room, ps.seat);
      expect(route.length, `${variant}/${seed}: a clear route exists`).toBeGreaterThan(0);
      // Every other member desk is no nearer.
      for (const s of room.spots.filter((x) => x.role === "member" && x.pose === "sit" && x.facing === ps.home.facing)) {
        const other = seatRoute(room, s);
        if (other.length > 0) expect(other.length, `${variant}/${seed}: ${s.id}`).toBeGreaterThanOrEqual(route.length);
      }
      // The route is floor all the way (the desk itself last), never across another desk.
      route.slice(0, -1).forEach((t) => expect(room.walk[t.y * room.cols + t.x]).toBe(1));
      // Handing the home spot to the members keeps the number of desks.
      expect(seatedSpots(room).filter((s) => s.role === "player")).toHaveLength(1);
      expect(seatedSpots(room).filter((s) => s.role === "member")).toHaveLength(room.spots.filter((s) => s.role === "member" || s.role === "player").length - 1);
    }
  });

  it("is reached without passing through or going round a seated member, for bodies of 5 to 15 (and a crowd of visitors)", () => {
    for (const { variant, seed, room } of rooms) {
      const ps = playerSeat(room);
      if (!ps) throw new Error("no player spot");
      const bare = seatRoute(room, ps.seat);
      const door = room.door;
      for (let size = 5; size <= 15; size++) {
        for (const visitors of [0, 12]) {
          const placed = castRoom(room, recipe, body(size, visitors), { fullStaff: true });
          const who = `${variant}/${seed}/${size}+${visitors}`;
          // Nobody sits on the player's desk or on its route.
          const taken = new Set(placed.map((p) => `${p.x},${p.y}`));
          expect(taken.has(`${ps.seat.x},${ps.seat.y}`), who).toBe(false);
          for (const t of bare) expect(taken.has(`${t.x},${t.y}`), `${who} at ${t.x},${t.y}`).toBe(false);
          // The walker's own router finds a way no longer than the bare route (no detour).
          const scene = new InteriorScene(room);
          scene.setOccupied(placed.map((p) => ({ x: p.x, y: p.y })));
          const path = scene.route({ x: ps.seat.x, y: ps.seat.y });
          expect(path, who).not.toBeNull();
          expect(path?.length, who).toBe(bare.length);
          expect(scene.player.x === door.x && scene.player.y === door.y).toBe(true);
        }
      }
    }
  });

  it("moves the vote hotspot to the desk, which is a seat the room's walker can stand on", () => {
    for (const { variant, room } of rooms) {
      const ps = playerSeat(room);
      if (!ps) throw new Error("no player spot");
      const moved = retargetHotspots(room, room.hotspots);
      const vote = moved.find((h) => h.id === "vote");
      expect(vote, variant).toBeTruthy();
      if (!vote) continue;
      expect(vote.stand).toEqual({ x: ps.seat.x, y: ps.seat.y });
      expect(isSeatHotspot(room, vote)).toBe(true);
      // The desk the hotspot sits on is blocked furniture beside the seat, in front of it.
      expect(room.walk[vote.y * room.cols + vote.x]).toBe(0);
      expect(Math.abs(vote.x - ps.seat.x) + Math.abs(vote.y - ps.seat.y)).toBe(1);
    }
  });

  it("falls back to the recipe's own spot when no desk is on an aisle", () => {
    const { room } = rooms[0] as (typeof rooms)[number];
    // Wall in the whole room but the door: nothing is reachable.
    const walled = { ...room, walk: room.walk.map((v, i) => (v === 1 && i !== room.door.y * room.cols + room.door.x ? 0 : v)) as Uint8Array };
    const ps = playerSeat(walled);
    expect(ps?.seat).toBe(ps?.home);
  });
});
