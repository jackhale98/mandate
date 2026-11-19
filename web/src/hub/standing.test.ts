// The talk hook in the street (DEC-093): people the engine lists on the
// player's street stand a few steps away, the player walks up to them, and
// Enter beside them (or a tap) talks. Presentation only.
import { fixtureTown } from "./testTowns";
import { TALK_REACH, TownScene } from "./townScene";

vi.setConfig({ testTimeout: 60_000 });

function stepUntilStill(s: TownScene) {
  for (let i = 0; i < 4000 && s.player.walk; i++) s.step(1 / 30);
}

describe("people standing in the street", () => {
  const { map } = fixtureTown(0);

  it("stand near the player, apart, off doorsteps, and stay put", () => {
    const s = new TownScene(map, { townsfolk: 0 });
    s.setStanding([
      { key: "c1", variant: 101 },
      { key: "a2", variant: 102 },
    ]);
    expect(s.standing.map((x) => x.key)).toEqual(["c1", "a2"]);
    const fronts = new Set(map.buildings.map((b) => b.front));
    const cells = s.standing.map(({ w }) => Math.floor(w.y) * map.width + Math.floor(w.x));
    for (const c of cells) {
      expect(fronts.has(c)).toBe(false);
      const d = Math.hypot((c % map.width) - Math.floor(s.player.x), Math.floor(c / map.width) - Math.floor(s.player.y));
      expect(d).toBeGreaterThanOrEqual(2);
      expect(d).toBeLessThan(9);
    }
    const [a, b] = cells as [number, number];
    expect(Math.max(Math.abs((a % map.width) - (b % map.width)), Math.abs(Math.floor(a / map.width) - Math.floor(b / map.width)))).toBeGreaterThanOrEqual(2);
    const before = s.standing.map(({ w }) => [w.x, w.y]);
    for (let i = 0; i < 60; i++) s.step(1 / 30);
    expect(s.standing.map(({ w }) => [w.x, w.y])).toEqual(before);
    // Listed again: they stay where they stand; someone gone goes.
    s.setStanding([{ key: "c1", variant: 101 }]);
    expect(s.standing.map(({ w }) => [w.x, w.y])).toEqual([before[0]]);
  });

  it("a passer-by met (a new key for the same person) stands where they stood", () => {
    const s = new TownScene(map, { townsfolk: 0 });
    s.setStanding([{ key: "a1", variant: 101 }]);
    const at = [s.standing[0]?.w.x, s.standing[0]?.w.y];
    s.setStanding([{ key: "c40", variant: 555 }]);
    expect(s.standing.map((x) => x.key)).toEqual(["c40"]);
    expect([s.standing[0]?.w.x, s.standing[0]?.w.y]).toEqual(at);
  });

  it("the player walks up to someone and is then beside them; a tap on them finds them", () => {
    const s = new TownScene(map, { townsfolk: 0 });
    s.setStanding([{ key: "c1", variant: 101 }]);
    expect(s.personNear()).toBeNull();
    const who = s.standing[0]?.w;
    if (!who) throw new Error("someone stands");
    expect(s.personAt({ x: who.x, y: who.y - 0.4 })).toBe("c1");
    expect(s.personAt({ x: who.x + 3, y: who.y })).toBeNull();
    let arrived = 0;
    expect(s.goToPerson("c1", () => arrived++)).toBe(true);
    stepUntilStill(s);
    expect(arrived).toBe(1);
    expect(s.personNear()).toBe("c1");
    expect(Math.hypot(who.x - s.player.x, who.y - s.player.y)).toBeLessThanOrEqual(TALK_REACH);
    // Already beside them: at once.
    expect(s.goToPerson("c1", () => arrived++)).toBe(true);
    expect(arrived).toBe(2);
  });

  it("dialogue mode: the player stands beside them (face to face), and the townsfolk stand still", () => {
    const s = new TownScene(map, { townsfolk: 6 });
    s.setStanding([{ key: "c1", variant: 101 }]);
    const who = s.standing[0]?.w;
    if (!who) throw new Error("someone stands");
    expect(s.goToPerson("c1")).toBe(true);
    stepUntilStill(s);
    // Beside them, on their row (people stand on pavements, which run both ways).
    expect(Math.floor(s.player.y)).toBe(Math.floor(who.y));
    expect(Math.abs(Math.floor(s.player.x) - Math.floor(who.x))).toBe(1);
    s.folkPaused = true;
    const before = s.folk.map((w) => [w.x, w.y]);
    for (let i = 0; i < 90; i++) s.step(1 / 30);
    expect(s.folk.map((w) => [w.x, w.y])).toEqual(before);
    s.folkPaused = false;
    for (let i = 0; i < 300; i++) s.step(1 / 30);
    expect(s.folk.map((w) => [w.x, w.y])).not.toEqual(before);
  });
});
