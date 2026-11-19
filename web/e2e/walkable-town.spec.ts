import { copyFileSync, mkdirSync } from "node:fs";
import path from "node:path";
import { expect, test, type Page, type TestInfo } from "@playwright/test";

// The walkable town in the game (DEC-086, DEC-088, DEC-090): the town screen
// opens on the town map, drawn from layout v2 (planned in the town worker,
// cached in memory and IndexedDB); "Walk the streets" goes down into the
// walkable street (the hub renderer, pixel-art pack), where tapping a place
// walks the player there and opens its panel. The light follows the turn's
// slots, and ending a turn passes the night. Runs on desktop and phone.
// WALK_SCREENS=<dir> also saves screenshots there.
const SCREENS = process.env.WALK_SCREENS ?? "";

interface Pt {
  x: number;
  y: number;
}
interface Building {
  index: number;
  x: number;
  y: number;
  w: number;
  h: number;
  front: number;
  place: number | null;
  kind: string;
  home: boolean;
}
/** What the town screen exposes for tests (web/src/components: useTownLayout, TownMap, TownStreet). */
interface Probes {
    __townPlan?: { loads: { source: string; ms: number; planMs: number | null }[]; dress: { ms: number } };
    __townMap?: { view: { minutes: number | null; scene: { map: { version: number; width: number; height: number } } } | null };
    __townStreet?: {
      view: {
        map: { version: number; width: number; buildings: Building[] };
        scene: { playerCell: number; playerMoving: boolean; activeFolk: number; folk: unknown[] };
        centreOn(x: number, y: number): void;
        toScreen(x: number, y: number): Pt;
      };
      minutes: number;
      target: number;
      arrivals: number;
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
  const file = info.outputPath(`${name}.png`);
  await page.screenshot({ path: file });
  copyFileSync(file, path.join(SCREENS, `${info.project.name}-${name}.png`));
};

const start = async (page: Page, seed: string) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill(seed);
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
  await expect(page.getByTestId("town-map-canvas")).toBeVisible();
};

const enterStreet = async (page: Page) => {
  await page.getByTestId("town-walk").click();
  await expect(page.getByTestId("town-street")).toBeVisible();
  await expect(page.locator(".town-street-stage[data-ready='1']")).toBeVisible({ timeout: 30_000 });
  await expect.poll(() => page.evaluate(() => (window as unknown as Probes).__townStreet?.view.map.version ?? 0)).toBe(2);
};

/**
 * Look at a building and tap the middle of it (a finger on phones, the mouse
 * on desktop): the player's house, or the nearest place with something to do
 * (the town's everyday places, which the List has as buttons).
 */
const tapBuilding = async (page: Page, info: TestInfo, which: "home" | "nearest-place") => {
  const target = await page.evaluate((mode) => {
    const s = (window as unknown as Probes).__townStreet;
    if (!s) return null;
    const v = s.view;
    const W = v.map.width;
    const p = v.scene.playerCell;
    const active = ["town_hall", "council_chamber", "diner", "party_office", "news_office", "market", "union_hall", "community_centre"];
    const score = (x: Building) => {
      if (mode === "home") return x.home ? 0 : 1e9;
      if (x.place === null || !active.includes(x.kind) || x.front === p) return 1e9;
      return Math.abs((x.front % W) - (p % W)) + Math.abs(Math.floor(x.front / W) - Math.floor(p / W));
    };
    const b = [...v.map.buildings].sort((a, c) => score(a) - score(c))[0];
    if (!b) return null;
    v.centreOn(b.x + b.w / 2, b.y + b.h / 2);
    return { b, at: v.toScreen(b.x + b.w / 2, b.y + b.h / 2) };
  }, which);
  expect(target).not.toBeNull();
  const box = await page.locator(".town-street-stage").boundingBox();
  if (!box || !target) throw new Error("no street");
  if (info.project.name === "phone") await page.touchscreen.tap(box.x + target.at.x, box.y + target.at.y);
  else await page.mouse.click(box.x + target.at.x, box.y + target.at.y);
  return target.b;
};

test("a new game opens the town map on layout v2, planned off the main thread, with no console errors", async ({ page }, info) => {
  const errors = watchErrors(page);
  await start(page, "31337");
  await expect(page.getByTestId("town-tab-map")).toHaveAttribute("aria-selected", "true");
  expect(await page.evaluate(() => (window as unknown as Probes).__townMap?.view?.scene.map.version)).toBe(2);
  const load = await page.evaluate(() => (window as unknown as Probes).__townPlan?.loads.at(-1) ?? null);
  expect(load?.source).toBe("planned");
  expect(load?.planMs).toBeGreaterThan(0);
  await shot(page, info, "town-map-v2");
  expect(errors).toEqual([]);
});

test("walk the streets: tap a place, the player walks there and its panel opens; back to the map; the List still works", async ({ page }, info) => {
  test.setTimeout(120_000);
  const errors = watchErrors(page);
  await start(page, "31337");
  await enterStreet(page);
  await expect(page.locator(".town-street-stage canvas")).toHaveAttribute("aria-label", /streets of .+ List view/);
  // Townsfolk are about.
  expect(await page.evaluate(() => (window as unknown as Probes).__townStreet?.view.scene.activeFolk ?? 0)).toBeGreaterThan(0);
  await page.waitForTimeout(500);
  await shot(page, info, "street-view");

  // The nearest place with something to do (the list's buttons): the town's everyday places.
  const b = await tapBuilding(page, info, "nearest-place");
  await expect.poll(() => page.evaluate(() => (window as unknown as Probes).__townStreet?.view.scene.playerMoving ?? false), { timeout: 5000 }).toBe(true);
  await shot(page, info, "street-walking");
  // On arrival: the same command as the List's button, and the place's panel.
  const panel = page.getByTestId("place-panel");
  await expect(panel.getByTestId("you-are-here-panel")).toBeVisible({ timeout: 60_000 });
  expect(await page.evaluate(() => (window as unknown as Probes).__townStreet?.view.scene.playerCell)).toBe(b.front);
  expect(await page.evaluate(() => (window as unknown as Probes).__townStreet?.arrivals ?? 0)).toBeGreaterThan(0);
  const name = (await panel.locator("#place-title").textContent()) ?? "";
  expect(name.length).toBeGreaterThan(0);
  await expect(page.getByTestId("where")).toContainText(name.trim());
  await shot(page, info, "street-place-panel");
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);

  // Back up to the map: the player is marked where they now are.
  await page.getByTestId("town-walk-back").click();
  await expect(page.getByTestId("town-map-canvas")).toBeVisible();
  const here = page.locator("[data-hotspot][aria-current='location']");
  await expect(here).toHaveCount(1);
  expect(name).toContain((await here.getAttribute("data-name")) ?? "-");

  // The List view still works, as before.
  await page.getByTestId("town-tab-list").click();
  await page.getByTestId("spot-town_hall").click();
  await expect(panel.getByRole("heading", { name: /Town hall/ })).toBeVisible();
  await expect(page.getByTestId("where")).toContainText("Town hall");
  await page.keyboard.press("Escape");
  await page.getByTestId("town-tab-map").click();
  await expect(page.getByTestId("town-map-canvas")).toBeVisible();
  expect(errors).toEqual([]);
});

test("a reload plans nothing: the town comes from the plan cache", async ({ page }) => {
  const errors = watchErrors(page);
  await start(page, "2026");
  expect(await page.evaluate(() => (window as unknown as Probes).__townPlan?.loads.at(-1)?.source)).toBe("planned");
  await page.reload();
  await page.getByRole("button", { name: "Continue" }).first().click();
  await expect(page.getByTestId("town-map-canvas")).toBeVisible();
  const loads = await page.evaluate(() => (window as unknown as Probes).__townPlan?.loads ?? []);
  expect(loads.map((l) => l.source)).toEqual(["stored"]);
  expect(errors).toEqual([]);
});

test("the light follows the turn's slots: still while idle, forward when a slot is spent", async ({ page }, info) => {
  test.setTimeout(120_000);
  const errors = watchErrors(page);
  await start(page, "4242");
  await enterStreet(page);
  const minutes = () => page.evaluate(() => (window as unknown as Probes).__townStreet?.minutes ?? -1);
  const m0 = await minutes();
  await page.waitForTimeout(1500);
  expect(await minutes()).toBe(m0);
  // Rest at home (a slot): tap the house; its panel opens; rest.
  await tapBuilding(page, info, "home");
  const panel = page.getByTestId("place-panel");
  await panel.getByTestId("place-action-rest").getByRole("button").last().click();
  await expect(page.getByTestId("hud-slots")).toHaveAttribute("aria-label", "2 slots left this turn");
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
  // The light eases forward (not a jump), then holds still again.
  await expect.poll(() => page.evaluate(() => (window as unknown as Probes).__townStreet?.target ?? -1)).toBeGreaterThan(m0);
  const target = await page.evaluate(() => (window as unknown as Probes).__townStreet?.target ?? -1);
  const seen = new Set<number>();
  for (let i = 0; i < 30; i++) {
    const m = await minutes();
    seen.add(Math.round(m));
    if (Math.abs(m - target) < 0.01) break;
    await page.waitForTimeout(60);
  }
  expect([...seen].some((m) => m > m0 + 1 && m < target - 1)).toBe(true);
  await expect.poll(minutes).toBeCloseTo(target, 1);
  await page.waitForTimeout(1200);
  expect(await minutes()).toBeCloseTo(target, 1);
  expect(errors).toEqual([]);
});

test("ending a turn passes the night: the light goes to night, the new date shows, the new turn starts in the morning", async ({ page }, info) => {
  test.setTimeout(120_000);
  const errors = watchErrors(page);
  await start(page, "4242");
  await enterStreet(page);
  const day = await page.evaluate(() => (window as unknown as Probes).__townStreet?.minutes ?? -1);
  await page.getByTestId("end-turn").click();
  const passage = page.getByTestId("turn-passage");
  await expect(passage).toBeVisible();
  expect(await passage.getAttribute("data-quick")).toBe("0");
  // Into the night (lamps lit): past midnight on the way.
  await expect.poll(() => page.evaluate(() => (window as unknown as Probes).__townStreet?.minutes ?? -1), { timeout: 5000 }).toBeLessThan(day);
  await expect(passage.getByTestId("turn-passage-date")).toBeVisible();
  await expect(passage).toContainText(/later/);
  await shot(page, info, "turn-passage");
  await expect(passage).toHaveCount(0, { timeout: 15_000 });
  await expect(page.getByText("Turn 1").first()).toBeVisible();
  // The morning of the new turn, as the map shows it.
  await expect.poll(() => page.evaluate(() => (window as unknown as Probes).__townStreet?.minutes ?? -1)).toBeCloseTo(day, 0);
  // On the map and the list it passes the same way.
  await page.getByTestId("town-walk-back").click();
  await page.getByTestId("end-turn").click();
  await expect(passage).toBeVisible();
  await expect(passage).toHaveCount(0, { timeout: 15_000 });
  await expect(page.getByText("Turn 2").first()).toBeVisible();
  expect(await page.evaluate(() => (window as unknown as Probes).__townMap?.view?.minutes ?? -1)).toBeCloseTo(day, 0);
  await page.getByTestId("town-tab-list").click();
  await page.getByTestId("end-turn").click();
  await expect(passage).toBeVisible();
  await expect(passage).toHaveCount(0, { timeout: 15_000 });
  await expect(page.getByText("Turn 3").first()).toBeVisible();
  expect(errors).toEqual([]);
});

test.describe("with reduced motion", () => {
  test.use({ reducedMotion: "reduce" });
  test("ending a turn is a quick fade of the date card", async ({ page }) => {
    const errors = watchErrors(page);
    await start(page, "4242");
    await page.getByTestId("end-turn").click();
    const passage = page.getByTestId("turn-passage");
    await expect(passage).toBeVisible();
    expect(await passage.getAttribute("data-quick")).toBe("1");
    await expect(page.getByText("Turn 1").first()).toBeVisible();
    const t0 = Date.now();
    await expect(passage).toHaveCount(0);
    expect(Date.now() - t0).toBeLessThan(2000);
    // No easing: the map shows the new morning at once.
    expect(await page.evaluate(() => (window as unknown as Probes).__townMap?.view?.minutes ?? -1)).toBeGreaterThan(0);
    expect(errors).toEqual([]);
  });
});
