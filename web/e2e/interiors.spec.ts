import { mkdirSync } from "node:fs";
import { expect, test, type Page, type TestInfo } from "@playwright/test";
import { showPlaces } from "./pocket";

// Enterable places (DEC-088): in the street, the door of the town hall, the
// council chamber or the diner (a tap, or Enter at it) leads inside: a short
// fade, a single-screen room composed from the place's recipe, the people the
// engine says are here on the recipe's spots (scenery staff at the counter),
// hotspots that offer what the place view offers now, talking through the
// store's talk entry point, and out by the door to the street outside it.
// The Places tab (and the town map) still open the place's panel (DEC-051).
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
      scene: { tile: { x: number; y: number }; moving: boolean };
      setMinutes(m: number, ms?: number): void;
      tileToScreen(t: { x: number; y: number }): { x: number; y: number };
    };
    place: number;
    recipe: string;
    people: RoomPerson[];
    hotspots: string[];
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
