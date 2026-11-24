import { mkdirSync } from "node:fs";
import { expect, test, type Page, type TestInfo } from "@playwright/test";
import { showPlaces } from "./pocket";

// Enterable places (DEC-088): in the street, the door of the town hall, the
// council chamber or the diner (a tap, or Enter at it) leads inside: a short
// fade, a single-screen room composed from the place's recipe, the people the
// engine says are here on the recipe's spots (scenery staff at the counter),
// hotspots that offer what the place view offers now, talking through the
// store's talk entry point, and out by the door to the street outside it.
// The Places tab still opens the place's panel (DEC-051); the town map walks in like a tap on the door.
// INTERIOR_SCREENS=<dir> also saves screenshots there.
const SCREENS = process.env.INTERIOR_SCREENS ?? "";

interface Building {
  index: number;
  x: number;
  y: number;
  w: number;
  h: number;
  front: number;
  place: number | null;
  kind: string;
}
interface RoomPerson {
  key: string;
  spot: string | null;
  role: string;
  extra: boolean;
  label: string | null;
  x: number;
  y: number;
}
interface Probes {
  __townStreet?: {
    view: {
      map: { width: number; buildings: Building[] };
      scene: { playerCell: number; playerMoving: boolean; placePlayer(cell: number): void };
      centreOn(x: number, y: number): void;
      toScreen(x: number, y: number): { x: number; y: number };
      wake(): void;
    };
  } | null;
  __interior?: {
    view: {
      stats: { frames: number; ticks: number };
      asleep: boolean;
      scene: { tile: { x: number; y: number }; moving: boolean; passable(x: number, y: number): boolean };
      talking: string | null;
      room: { cols: number; rows: number; door: { x: number; y: number } };
      crowd: { walkers: Map<string, { key: string; x: number; y: number; leaving: boolean }> };
      setPeople(people: RoomPerson[], animate?: boolean): void;
      setMinutes(m: number, ms?: number): void;
      tileToScreen(t: { x: number; y: number }): { x: number; y: number };
    };
    place: number;
    recipe: string;
    people: RoomPerson[];
    hotspots: string[];
    seat: { x: number; y: number } | null;
  } | null;
}

const watchErrors = (page: Page) => {
  const errors: string[] = [];
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(String(e)));
  return errors;
};

const shot = async (page: Page, info: TestInfo, name: string) => {
  if (!SCREENS) return;
  mkdirSync(SCREENS, { recursive: true });
  await page.screenshot({ path: `${SCREENS}/${info.project.name}-${name}.png` });
};

const start = async (page: Page, seed: string) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill(seed);
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
  await expect(page.locator(".town-street-stage[data-ready='1']")).toBeVisible({ timeout: 30_000 });
};

/** Read the test probes: `f` runs in the page with `window` (as the probes). */
const probe = <T>(page: Page, f: (w: Probes) => T) => page.evaluate(`(${f.toString()})(window)`) as Promise<T>;

/** Look at a place's building and tap it (a finger on phones, the mouse on desktop): the player walks to its door. */
const tapPlace = async (page: Page, info: TestInfo, kind: string) => {
  const target = await page.evaluate((k) => {
    const v = (window as unknown as Probes).__townStreet?.view;
    const b = v?.map.buildings.find((x) => x.kind === k && x.place !== null);
    if (!v || !b) return null;
    v.centreOn(b.x + b.w / 2, b.y + b.h / 2);
    return { b, at: v.toScreen(b.x + b.w / 2, b.y + b.h / 2) };
  }, kind);
  expect(target, kind).not.toBeNull();
  const box = await page.locator(".town-street-stage").boundingBox();
  if (!box || !target) throw new Error("no street");
  if (info.project.name === "phone") await page.touchscreen.tap(box.x + target.at.x, box.y + target.at.y);
  else await page.mouse.click(box.x + target.at.x, box.y + target.at.y);
  return target.b;
};

/** Stand at a place's door in the street and press Enter there (the keyboard way in). */
const enterByKeyboard = async (page: Page, kind: string) => {
  const b = await page.evaluate((k) => {
    const v = (window as unknown as Probes).__townStreet?.view;
    const b = v?.map.buildings.find((x) => x.kind === k && x.place !== null);
    if (!v || !b) return null;
    v.scene.placePlayer(b.front);
    v.wake();
    return b;
  }, kind);
  expect(b, kind).not.toBeNull();
  const canvas = page.locator(".town-street-stage canvas");
  await canvas.focus();
  await expect(page.locator(".hub-door-hint")).toBeVisible();
  await page.keyboard.press("Enter");
  return b as Building;
};

/** Tap a tile of the room (its centre): a finger on phones, the mouse on desktop. */
const tapTile = async (page: Page, info: TestInfo, t: { x: number; y: number }) => {
  const at = await page.evaluate((tile) => (window as unknown as Probes).__interior?.view.tileToScreen(tile), t);
  const stage = await page.locator(".interior-stage").boundingBox();
  if (!at || !stage) throw new Error("no room");
  if (info.project.name === "phone") await page.touchscreen.tap(stage.x + at.x, stage.y + at.y);
  else await page.mouse.click(stage.x + at.x, stage.y + at.y);
};

const inside = (page: Page, recipe: string) => expect(page.locator(`[data-testid='interior'][data-recipe='${recipe}'][data-ready='1']`)).toBeVisible({ timeout: 30_000 });

const people = (page: Page) => probe(page, (w) => w.__interior?.people ?? []);

test("walk into the town hall, see the clerk, use a counter, talk to someone, walk out at the door; the Places tab opens the panel", async ({ page }, info) => {
  test.setTimeout(150_000);
  const errors = watchErrors(page);
  await start(page, "31337");
  // In: a tap on the building (phone), Enter at its door (desktop).
  const hall = info.project.name === "phone" ? await tapPlace(page, info, "town_hall") : await enterByKeyboard(page, "town_hall");
  await inside(page, "interior.town_hall");
  await expect(page.getByTestId("place-panel")).toHaveCount(0);
  await shot(page, info, "town-hall-entered");

  // The clerk at the counter: someone on a clerk's spot behind it (scenery staff where nobody present works here).
  const ps = await people(page);
  const clerk = ps.find((p) => p.role === "clerk");
  expect(clerk?.spot).toMatch(/^clerk/);
  // Everyone the engine says is here is in the room, each on their own tile.
  expect(new Set(ps.map((p) => `${p.x},${p.y}`)).size).toBe(ps.length);
  // The hotspots offer what the place view offers now: the counter windows, the notices, the leaflets, the offices.
  const spots = await probe(page, (w) => w.__interior?.hotspots ?? []);
  expect(spots).toEqual(expect.arrayContaining(["window1", "window2", "notices", "leaflets", "offices"]));

  // A hotspot: walk to window 2 and draft a bill there (the place panel's own forms).
  // (A tap on the counter's window itself; its marker does the same.)
  await tapTile(page, info, { x: 8, y: 3 });
  const sheet = page.getByTestId("interior-hotspot");
  await expect(sheet).toBeVisible({ timeout: 15_000 });
  await expect(sheet).toHaveAttribute("data-hotspot", "window2");
  expect(await probe(page, (w) => w.__interior?.view.scene.tile)).toEqual({ x: 8, y: 4 });
  const catalog = sheet.getByTestId("catalog");
  await catalog.locator(".row-btn").first().click();
  await catalog.getByRole("button", { name: "Create draft" }).click();
  await expect(sheet.getByTestId("hotspot-bills").locator(".bill-list > *").first()).toBeVisible({ timeout: 15_000 });
  await shot(page, info, "town-hall-hotspot");
  await sheet.getByRole("button", { name: "Close" }).click();
  await expect(sheet).toHaveCount(0);

  // Talk to someone inside: a tap on them walks up to them and opens the conversation.
  const someone = (await people(page)).find((p) => !p.extra && p.label);
  expect(someone, "someone the engine says is here").toBeDefined();
  await tapTile(page, info, { x: someone?.x ?? 0, y: someone?.y ?? 0 });
  const talk = page.getByTestId("talk-panel");
  await expect(talk).toBeVisible({ timeout: 15_000 });
  const t = await probe(page, (w) => w.__interior?.view.scene.tile);
  expect(Math.abs((t?.x ?? 0) - (someone?.x ?? 99)) + Math.abs((t?.y ?? 0) - (someone?.y ?? 99))).toBe(1);
  // Dialogue mode: the room is the talk stage: the two framed there, the rest dimmed, a bubble over the person.
  await expect(page.getByTestId("talk-layer")).toHaveAttribute("data-mode", "street");
  await expect(page.getByTestId("talk-veil")).toBeVisible();
  await expect.poll(() => probe(page, (w) => w.__interior?.view.talking ?? null)).toBe(someone?.key);
  await talk.getByTestId("talk-topic-topic.small_talk").or(talk.getByTestId("talk-topic-topic.who_runs")).first().locator("button.dlg-choice").click();
  await expect(page.getByTestId("talk-bubble").locator("p").first()).not.toBeEmpty({ timeout: 15_000 });
  const bb = await page.getByTestId("talk-bubble").boundingBox();
  const vp = page.viewportSize();
  expect(bb && vp && bb.x >= 0 && bb.x + bb.width <= vp.width && bb.y >= 0).toBe(true);
  await shot(page, info, "town-hall-talk");
  await page.keyboard.press("Escape");
  await expect(talk).toHaveCount(0);

  // Out by the door: the street outside the town hall's door.
  if (info.project.name === "phone") await page.locator(".interior-door-tag").click();
  else {
    await page.locator(".interior-stage canvas").focus();
    await page.keyboard.press("Escape");
  }
  await expect(page.getByTestId("interior")).toHaveCount(0, { timeout: 15_000 });
  expect(await probe(page, (w) => w.__townStreet?.view.scene.playerCell)).toBe(hall.front);
  await expect(page.getByTestId("where")).toContainText("Town hall");

  // The Places tab goes to the same place and opens its panel (DEC-051: nothing only by walking).
  await showPlaces(page);
  await page.getByTestId("spot-town_hall").click();
  const panel = page.getByTestId("place-panel");
  await expect(panel.getByRole("heading", { name: /Town hall/ })).toBeVisible();
  await expect(page.getByTestId("interior")).toHaveCount(0);
  await shot(page, info, "town-hall-panel");
  expect(errors).toEqual([]);
});

test("the diner: scenery staff, no late booth by day, an idle room sleeps; a place without a recipe keeps its panel", async ({ page }, info) => {
  test.setTimeout(120_000);
  const errors = watchErrors(page);
  await start(page, "31337");
  // Into the diner from its door, then check the room goes to sleep when nothing moves (DEC-080).
  await tapPlace(page, info, "diner");
  await inside(page, "interior.diner");
  await expect.poll(() => probe(page, (w) => w.__interior?.view.asleep ?? false), { timeout: 10_000 }).toBe(true);
  const before = await probe(page, (w) => w.__interior?.view.stats.frames ?? 0);
  await page.waitForTimeout(1200);
  expect(await probe(page, (w) => w.__interior?.view.stats.frames ?? 0)).toBe(before);
  // The diner's owner works here; the cook and the waiter are scenery by day.
  const ps = await people(page);
  expect(ps.some((p) => p.extra && (p.role === "cook" || p.role === "waiter"))).toBe(true);
  // The late-only back booth is not offered before the night.
  expect(await probe(page, (w) => w.__interior?.hotspots ?? [])).not.toContain("booth_back");
  await page.getByTestId("interior-leave").click();
  await expect(page.getByTestId("interior")).toHaveCount(0, { timeout: 10_000 });
  // A place with no recipe (the market, a party office...) opens its panel from the street, as before.
  const kinds = await page.evaluate(() => (window as unknown as Probes).__townStreet?.view.map.buildings.filter((b) => b.place !== null).map((b) => b.kind) ?? []);
  const plain = ["market", "party_office", "news_office", "union_hall", "community_centre"].find((k) => kinds.includes(k));
  if (plain) {
    await tapPlace(page, info, plain);
    await expect(page.getByTestId("place-panel").getByTestId("you-are-here-panel")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("interior")).toHaveCount(0);
  }
  expect(errors).toEqual([]);
});

/** The engine's next view of the room (as the room's effect hands it on): one person gone, a newcomer at a free tile. */
const swapOne = (page: Page) =>
  page.evaluate(() => {
    const w = window as unknown as Probes;
    const i = w.__interior;
    if (!i) return null;
    const { view, people } = i;
    const gone = people.find((p) => !p.extra && p.label);
    if (!gone) return null;
    const taken = new Set(people.map((p) => `${p.x},${p.y}`));
    let spot: { x: number; y: number } | null = null;
    let best = 0;
    for (let y = 0; y < view.room.rows; y++)
      for (let x = 0; x < view.room.cols; x++) {
        const d = Math.abs(x - view.room.door.x) + Math.abs(y - view.room.door.y);
        if (view.scene.passable(x, y) && !taken.has(`${x},${y}`) && d > best) {
          best = d;
          spot = { x, y };
        }
      }
    if (!spot) return null;
    const rest = people.filter((p) => p !== gone);
    const newcomer = { ...gone, key: "c999999", spot: null, role: "public", x: spot.x, y: spot.y, label: null };
    view.setPeople([...rest, newcomer], true);
    return { gone: { key: gone.key, x: gone.x, y: gone.y }, to: spot, door: view.room.door };
  });
const walkers = (page: Page) => probe(page, (w) => [...(w.__interior?.view.crowd.walkers.values() ?? [])].map((x) => ({ key: x.key, x: x.x, y: x.y, leaving: x.leaving })));

test("time passes: a newcomer walks in from the door to their spot and someone gone walks out through it", async ({ page }, info) => {
  test.setTimeout(120_000);
  const errors = watchErrors(page);
  await start(page, "31337");
  await tapPlace(page, info, "diner");
  await inside(page, "interior.diner");
  await expect.poll(() => probe(page, (w) => w.__interior?.view.asleep ?? false), { timeout: 10_000 }).toBe(true);
  const change = await swapOne(page);
  expect(change, "someone to send out and a free tile to come in to").not.toBeNull();
  if (!change) return;
  // Right away: the newcomer is at the door, the leaver where they stood; both on their way (the room wakes).
  const first = await walkers(page);
  const comer = first.find((x) => x.key === "c999999");
  const goer = first.find((x) => x.key === change.gone.key);
  expect(comer && !comer.leaving).toBe(true);
  expect(goer?.leaving).toBe(true);
  expect(Math.abs((comer?.x ?? 0) - change.door.x) + Math.abs((comer?.y ?? 0) - change.door.y)).toBeLessThan(2);
  // Over time the newcomer's position changes towards their spot, and the leaver's towards the door.
  await expect
    .poll(async () => {
      const c = (await walkers(page)).find((x) => x.key === "c999999");
      return c ? Math.abs(c.x - change.door.x) + Math.abs(c.y - change.door.y) : 99;
    })
    .toBeGreaterThan(2);
  // Everyone is where they belong in the end (nobody walking), and the room goes back to sleep.
  await expect.poll(async () => (await walkers(page)).length, { timeout: 20_000 }).toBe(0);
  await expect.poll(() => probe(page, (w) => w.__interior?.view.asleep ?? false), { timeout: 10_000 }).toBe(true);
  const now = await people(page);
  expect(now.some((p) => p.key === change.gone.key)).toBe(true);
  expect(errors).toEqual([]);
});

test("reduced motion: nobody walks; they are simply there", async ({ page }, info) => {
  test.setTimeout(120_000);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await start(page, "31337");
  await tapPlace(page, info, "diner");
  await inside(page, "interior.diner");
  const change = await swapOne(page);
  expect(change).not.toBeNull();
  expect(await walkers(page)).toEqual([]);
});

test("each room in the game, by day and in the evening, with the people present (screenshots)", async ({ page }, info) => {
  test.skip(!SCREENS, "INTERIOR_SCREENS not set");
  test.setTimeout(240_000);
  await start(page, "31337");
  for (const kind of ["council_chamber", "town_hall", "diner"]) {
    // The door transition, slowed down for the picture only: the street fading into the room.
    const slow = kind === "town_hall" ? await page.addStyleTag({ content: ".interior-veil { animation-duration: 2400ms !important; }" }) : null;
    await tapPlace(page, info, kind);
    // The door transition: the street fading into the room.
    await expect(page.getByTestId("interior")).toBeVisible({ timeout: 60_000 });
    if (slow) {
      await page.waitForTimeout(700);
      await shot(page, info, "door-transition");
      await slow.evaluate((el) => (el as HTMLElement).remove());
    }
    await inside(page, `interior.${kind}`);
    await page.waitForTimeout(400);
    await shot(page, info, `${kind}-day`);
    await page.evaluate(() => (window as unknown as Probes).__interior?.view.setMinutes(20 * 60 + 30, 0));
    await page.waitForTimeout(300);
    await shot(page, info, `${kind}-evening`);
    await page.getByTestId("interior-leave").click();
    await expect(page.getByTestId("interior")).toHaveCount(0, { timeout: 10_000 });
    await shot(page, info, `${kind}-out`);
  }
});

// Play note 21: with every member in the chamber the player's desk is the one nearest the door along a clear
// aisle, "Take your seat" and the vote are real buttons, and a tap on your own desk is yours even where a
// neighbour's figure reaches over it.
test("the chamber full of members: your desk is on the aisle, the buttons take you there", async ({ page }, info) => {
  test.setTimeout(150_000);
  const errors = watchErrors(page);
  await start(page, "31337");
  await tapPlace(page, info, "council_chamber");
  await inside(page, "interior.council_chamber");

  // Every desk taken but the player's: members on each seated spot and the sides (what a full body looks like).
  const seat = await probe(page, (w) => w.__interior?.seat ?? null);
  expect(seat, "the chamber has a player desk").not.toBeNull();
  const full = await page.evaluate(() => {
    const i = (window as unknown as Probes).__interior;
    if (!i || !i.seat) return null;
    const { view, people } = i;
    const seatTile = i.seat;
    const template = people.find((p) => !p.extra && p.label) ?? people[0];
    if (!template) return null;
    const rm = view.room as unknown as { spots: { id: string; role: string; x: number; y: number; facing: string; pose?: string }[]; door: { x: number; y: number } };
    const taken = new Set(people.map((p) => `${p.x},${p.y}`));
    const added = rm.spots
      .filter((s) => (s.role === "member" || s.role === "player") && !(s.x === seatTile.x && s.y === seatTile.y) && !taken.has(`${s.x},${s.y}`))
      .map((s, k) => ({ ...template, key: `cfull${k}`, spot: s.id, role: "member", x: s.x, y: s.y, facing: s.facing, pose: s.pose ?? "stand", label: `Member ${k}` }));
    view.setPeople([...people, ...added]);
    return { members: added.length + people.length, door: rm.door };
  });
  expect(full?.members ?? 0).toBeGreaterThan(10);
  // The way from the door to the desk exists past all of them, and goes straight (no detour round a seated member).
  const route = await probe(page, (w) => {
    const i = w.__interior;
    const sc = i?.view.scene as unknown as { route(t: { x: number; y: number }): { x: number; y: number }[] | null } | undefined;
    return i?.seat && sc ? (sc.route(i.seat)?.length ?? null) : null;
  });
  expect(route).not.toBeNull();
  expect(route ?? 99).toBeLessThanOrEqual(Math.abs((seat?.x ?? 0) - (full?.door.x ?? 0)) + Math.abs((seat?.y ?? 0) - (full?.door.y ?? 0)) + 1);
  await shot(page, info, "chamber-full");

  // The buttons: real, reachable by keyboard, at least 44 px.
  const takeSeat = page.getByTestId("interior-seat");
  await expect(takeSeat).toBeVisible();
  await expect(takeSeat).toHaveText("Take your seat");
  const box = await takeSeat.boundingBox();
  expect(box?.height ?? 0).toBeGreaterThanOrEqual(44);
  expect(box?.width ?? 0).toBeGreaterThanOrEqual(44);
  await takeSeat.focus();
  await expect(takeSeat).toBeFocused();
  await page.keyboard.press("Enter");
  await expect.poll(() => probe(page, (w) => w.__interior?.view.scene.tile), { timeout: 15_000 }).toEqual(seat);
  await expect.poll(() => probe(page, (w) => w.__interior?.view.scene.moving ?? true)).toBe(false);
  await shot(page, info, "chamber-seated");

  // At your desk, Enter opens the vote's sheet even with a member in the chair beside you.
  await page.locator(".interior-stage canvas").focus();
  await page.keyboard.press("Enter");
  const sheet = page.getByTestId("interior-hotspot");
  await expect(sheet).toHaveAttribute("data-hotspot", "vote");
  await sheet.getByRole("button", { name: "Close" }).click();
  await expect(sheet).toHaveCount(0);

  // The vote's own button, when one is open, goes there and opens it.
  const vote = page.getByTestId("interior-vote");
  if ((await vote.count()) > 0) {
    const vb = await vote.boundingBox();
    expect(vb?.height ?? 0).toBeGreaterThanOrEqual(44);
    await vote.click();
    await expect(sheet).toHaveAttribute("data-hotspot", "vote");
    await sheet.getByRole("button", { name: "Close" }).click();
    await expect(sheet).toHaveCount(0);
  }

  // A tap on your desk itself (the desk tile in front of the seat) is the vote too (the overlap with a
  // neighbour's figure is unit-tested in src/interior/tap.test.ts, where the figures can be placed exactly).
  await expect(sheet).toHaveCount(0);
  await tapTile(page, info, { x: seat?.x ?? 0, y: (seat?.y ?? 0) - 1 });
  await expect(sheet).toHaveAttribute("data-hotspot", "vote", { timeout: 15_000 });
  await expect(page.getByTestId("talk-panel")).toHaveCount(0);
  expect(errors).toEqual([]);
});
