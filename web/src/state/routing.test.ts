import { describe, expect, it } from "vitest";
import { findRoute, routeKey, sameLocation } from "./routing";
import type { PocketView } from "../engine/protocol/PocketView";

describe("auto-routing", () => {
  it("maps commands to action keys", () => {
    expect(routeKey({ type: "cast_vote", policy: 1, vote: "yes" }, null)?.action).toBe("vote");
    expect(routeKey({ type: "act", action: { type: "canvass", neighborhood: 4 }, late_night: false }, null)).toEqual({ action: "canvass", lateNight: false, neighborhood: 4 });
    expect(routeKey({ type: "act", action: { type: "town_hall", neighborhood: 2, policy: null }, late_night: false }, null)).toEqual({ action: "town_hall", lateNight: false, neighborhood: 2 });
    expect(routeKey({ type: "end_turn" }, null)).toBeNull();
    // Meeting someone happens where the player already is.
    expect(routeKey({ type: "meet", slot: 0 }, null)).toBeNull();
  });

  it("finds the engine's route for a command", () => {
    const to = { location: { type: "place" as const, place: 7 }, kind_key: "diner", name: "Sam's", neighborhood: 1, neighborhood_name: "Old Town" };
    const pocket = { routes: [{ action: "rest", late_night: false, neighborhood: null, to, here: false }] } as unknown as PocketView;
    expect(findRoute(pocket, null, { type: "act", action: { type: "rest" }, late_night: false })?.to).toBe(to);
    expect(findRoute(pocket, null, { type: "act", action: { type: "dip_funds" }, late_night: false })).toBeUndefined();
  });

  it("compares locations", () => {
    expect(sameLocation({ type: "home" }, { type: "home" })).toBe(true);
    expect(sameLocation({ type: "street", neighborhood: 1 }, { type: "street", neighborhood: 2 })).toBe(false);
    expect(sameLocation({ type: "place", place: 3 }, { type: "place", place: 3 })).toBe(true);
  });
});
