import type { Placement } from "./cast";
import { RoomCrowd, routeTiles } from "./crowd";
import type { SceneRoom } from "./interiorScene";

// A 7 x 6 room: floor inside a wall, the door in the bottom row, a table of two blocked tiles.
const cols = 7;
const rows = 6;
const walk = new Uint8Array(cols * rows);
for (let y = 1; y < rows - 1; y++) for (let x = 1; x < cols - 1; x++) walk[y * cols + x] = 1;
walk[5 * cols + 3] = 1; // the door tile
walk[2 * cols + 3] = 0; // furniture
walk[2 * cols + 4] = 0;
const room: SceneRoom = { cols, rows, walk, door: { x: 3, y: 5 } };

const at = (key: string, x: number, y: number, extra = false): Placement => ({ key, spot: `s${x}${y}`, role: "public", x, y, facing: "down", pose: "stand", extra });

describe("people coming and going in a room", () => {
  it("routes round furniture and other people", () => {
    const direct = routeTiles(room, { x: 3, y: 5 }, { x: 3, y: 1 }, new Set());
    expect(direct?.length).toBeGreaterThanOrEqual(6);
    for (const t of direct ?? []) expect(walk[t.y * cols + t.x]).toBe(1);
    // Someone standing in the way is walked round.
    const around = routeTiles(room, { x: 3, y: 5 }, { x: 3, y: 1 }, new Set([3 * cols + 3]));
    expect(around).not.toBeNull();
    expect(around?.some((t) => t.x === 3 && t.y === 3)).toBe(false);
    // Boxed in: no route.
    expect(routeTiles(room, { x: 3, y: 5 }, { x: 1, y: 1 }, new Set([4 * cols + 3]))).toBeNull();
  });

  it("a newcomer walks in from the door to their spot, and then stands there", () => {
    const crowd = new RoomCrowd(room);
    const here = [at("c1", 1, 1)];
    const change = crowd.sync(here, [...here, at("c2", 5, 1)]);
    expect(change.entering).toEqual([{ key: "c2", from: { x: 3, y: 5 }, to: { x: 5, y: 1 } }]);
    expect(change.leaving).toEqual([]);
    const w = crowd.walkers.get("c2");
    expect(w).toBeTruthy();
    // They start at the door, not at the spot, and their position changes over time.
    expect([w?.x, w?.y]).toEqual([3, 5]);
    expect(crowd.moving).toBe(true);
    crowd.step(0.5);
    expect(Math.hypot((w?.x ?? 0) - 3, (w?.y ?? 0) - 5)).toBeGreaterThan(0.5);
    for (let i = 0; i < 600 && crowd.moving; i++) crowd.step(1 / 30);
    expect(crowd.moving).toBe(false);
    expect([w?.x, w?.y]).toEqual([5, 1]);
    // The person already there did not move.
    expect(crowd.walkers.has("c1")).toBe(false);
  });

  it("someone gone walks out through the door from where they stood", () => {
    const crowd = new RoomCrowd(room);
    const before = [at("c1", 1, 1), at("c2", 5, 1)];
    const change = crowd.sync(before, [at("c1", 1, 1)]);
    expect(change.leaving).toEqual([{ key: "c2", from: { x: 5, y: 1 }, to: { x: 3, y: 5 } }]);
    expect(change.entering).toEqual([]);
    const w = crowd.walkers.get("c2");
    expect(w?.leaving).toBe(true);
    crowd.step(0.4);
    expect(Math.hypot((w?.x ?? 0) - 5, (w?.y ?? 0) - 1)).toBeGreaterThan(0.5);
    for (let i = 0; i < 600 && crowd.moving; i++) crowd.step(1 / 30);
    // Gone once at the door.
    expect(crowd.walkers.has("c2")).toBe(false);
    expect([w?.x, w?.y]).toEqual([3, 5]);
  });

  it("scenery staff and an unchanged cast do not walk; a leaver who comes back stays; a turn-round mid-entry leaves from where they are", () => {
    const crowd = new RoomCrowd(room);
    const cast = [at("c1", 1, 1), at("x:cook", 5, 1, true)];
    expect(crowd.sync(cast, cast)).toEqual({ entering: [], leaving: [] });
    expect(crowd.moving).toBe(false);
    crowd.sync(cast, [cast[1] as Placement]);
    expect(crowd.walkers.get("c1")?.leaving).toBe(true);
    // They come back before reaching the door: on from where they are, towards their spot.
    crowd.step(0.2);
    const there = { x: Math.round(crowd.walkers.get("c1")?.x ?? 0), y: Math.round(crowd.walkers.get("c1")?.y ?? 0) };
    expect(crowd.sync([cast[1] as Placement], cast).entering).toEqual([{ key: "c1", from: there, to: { x: 1, y: 1 } }]);
    expect(crowd.walkers.get("c1")?.leaving).toBe(false);
    // Someone arrives, and is gone again before they are in.
    const c = new RoomCrowd(room);
    c.sync([], [at("c3", 5, 1)]);
    c.step(0.6);
    const mid = { x: Math.round(c.walkers.get("c3")?.x ?? 0), y: Math.round(c.walkers.get("c3")?.y ?? 0) };
    const out = c.sync([at("c3", 5, 1)], []);
    expect(out.leaving).toEqual([{ key: "c3", from: mid, to: { x: 3, y: 5 } }]);
  });
});
