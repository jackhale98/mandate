import { expect, test, type Locator, type Page } from "@playwright/test";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { playTo, startRun } from "./midrun";
import { openPeople, openPocket, openFold, openRow, pocketMenu } from "./pocket";
import { useWideFont } from "./wide-font";

// The pocket's length budget (POCKET_REWORK §5, DEC-113 clause 6): rules a
// test can hold, measured as the owner measured them, on a 393 × 793 phone
// whose pocket is at most 85% of the window (one "screen" is 674 px of sheet).
// Every case sets that size itself, so it runs the same on the desktop and
// phone projects (on a wide screen the sheets are the window's full height
// and the budget does not bind there).
//
// The run: until the mid-career run file of task R3 exists, a run played in
// the browser from a pinned seed with "Start with local knowledge", to turn 10
// (the budget round's positions: an arc under way, an offer, a decision), and
// once at turn 4 so an early game is held too. When R3's file is in place at
// e2e/fixtures/mid-career.mandate, the last case imports it and asserts what
// it must hold (50 people known) before measuring.

const W = 393;
const H = 793;
const SCREEN = Math.floor(H * 0.85);
const SEED = "4242";
const MID_CAREER = path.join(path.dirname(fileURLToPath(import.meta.url)), "fixtures", "mid-career.mandate");

test.describe.configure({ mode: "serial" });

async function settle(page: Page) {
  await page.evaluate(async () => {
    await document.fonts.ready;
    await Promise.all(document.getAnimations().filter((a) => a.effect?.getComputedTiming().iterations !== Infinity).map((a) => a.finished.catch(() => undefined)));
  });
}

/** How long a sheet's opening view is, in screens, and where its first item starts, as a share of a screen. */
async function lengthOf(page: Page, sheet: Locator, first: Locator): Promise<{ screens: number; firstAt: number }> {
  await sheet.evaluate((el) => (el.scrollTop = 0));
  await settle(page);
  const scroll = await sheet.evaluate((el) => el.scrollHeight);
  const top = (await sheet.boundingBox())?.y ?? 0;
  const f = (await first.boundingBox())?.y ?? top;
  return { screens: scroll / SCREEN, firstAt: (f - top) / SCREEN };
}

/** The bottom of what a sheet shows before any scroll. */
async function sheetBottom(sheet: Locator): Promise<number> {
  const b = await sheet.boundingBox();
  return (b?.y ?? 0) + (b?.height ?? 0);
}

/** L8 and "no raw keys": nothing counts knowledge, and no Fluent key, placeholder or template id shows. */
async function expectCleanText(where: Locator, what: string) {
  const text = (await where.innerText()).replace(/\s+/g, " ");
  expect(text, `${what}: a meter`).not.toMatch(/[●○]/);
  expect(text, `${what}: a placeholder`).not.toContain("Not known yet");
  expect(text, `${what}: a share of knowledge`).not.toMatch(/\d+ of \d+ (known|heard)|\d+ ?% (known|heard)|(known|heard) \d+ ?%/i);
  expect(text, `${what}: a missing string`).not.toContain("⟦");
  expect(text, `${what}: a raw key`).not.toMatch(/\b(ui|now|know|arc|ask|dilemma|role|occupation|place|hood)-[a-z0-9-]+\b/);
  expect(text, `${what}: a Fluent argument`).not.toMatch(/\{\s*\$/);
  expect(text, `${what}: a template id`).not.toMatch(/\b[a-z_]+\.[a-z_]+\.[a-z_]+\b/);
  await expect(where.locator(".nb-dots")).toHaveCount(0);
}

/** L7: no sideways scroll, and every target in the pocket at least 44 px (text links in a sentence excepted). */
async function expectTargets(page: Page, where: Locator, what: string) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), `${what}: sideways scroll`).toBe(true);
  const small = await where.evaluate((root) =>
    [...root.querySelectorAll<HTMLElement>("button, a.btn, [role='tab']")]
      .filter((el) => !el.classList.contains("link-btn") && el.offsetParent !== null)
      .map((el) => ({ el, r: el.getBoundingClientRect() }))
      .filter(({ r }) => r.width > 0 && (r.height < 43.5 || r.width < 43.5))
      .map(({ el, r }) => `${el.dataset.testid ?? el.className} ${Math.round(r.width)}×${Math.round(r.height)} "${(el.textContent ?? "").trim().slice(0, 30)}"`),
  );
  expect(small, `${what}: targets under 44 px`).toEqual([]);
}

/** The run's preconditions, so a thin run can never pass the budget by being short. */
async function expectMidRun(page: Page, minPeople: number) {
  const pocket = await openPocket(page, "today");
  await expect(pocket.locator("[data-testid^='arc-row-']").first(), "an arc under way").toBeVisible();
  await expect(pocket.locator("[data-need='ask_offer']").first(), "an offer").toBeVisible();
  await expect(pocket.locator("[data-need='decision']").first(), "a decision").toBeVisible();
  await openPocket(page, "people");
  const counts = await pocket.locator("[data-fold^='people-'] .rw-count").evaluateAll((els) => els.map((e) => Number((e.firstChild?.textContent ?? "0").trim())));
  const now = await pocket.getByTestId("notebook-people-now").getByTestId("notebook-person").count();
  expect(now + counts.reduce((a, b) => a + b, 0), "people known").toBeGreaterThanOrEqual(minPeople);
  await openPocket(page, "places");
  expect(await pocket.locator(".spot").count() + (await pocket.getByTestId("town-hood").count()), "places and neighbourhoods").toBeGreaterThanOrEqual(10);
}

async function budget(page: Page) {
  // L1, L2: Today.
  let pocket = await openPocket(page, "today");
  const today = await lengthOf(page, pocket, pocket.getByTestId("today-strip"));
  expect(today.screens, "L1: Today").toBeLessThanOrEqual(2.0);
  expect(today.firstAt, "L2: Today").toBeLessThanOrEqual(0.25);
  await expectCleanText(pocket, "Today");
  // L4: a closed row at most 80 px; a folded section at most 60 px.
  const rows = await pocket.locator("li.rw-li:not(.rw-li-open)").evaluateAll((els) => els.map((e) => e.getBoundingClientRect().height));
  expect(Math.max(...rows), "L4: a closed row on Today").toBeLessThanOrEqual(80);
  const folds = await pocket.locator("section.rw-fold:not(.rw-fold-open)").evaluateAll((els) => els.map((e) => e.getBoundingClientRect().height));
  expect(Math.max(...folds), "L4: a folded section").toBeLessThanOrEqual(60);

  // L1, L2: each Notebook tab.
  for (const tab of ["people", "places", "town", "policies"] as const) {
    pocket = await openPocket(page, tab);
    const panel = pocket.getByRole("tabpanel", { name: new RegExp(tab, "i") });
    const m = await lengthOf(page, pocket, panel);
    expect(m.screens, `L1: Notebook, ${tab}`).toBeLessThanOrEqual(2.0);
    expect(m.firstAt, `L2: Notebook, ${tab}`).toBeLessThanOrEqual(0.25);
    await expectCleanText(pocket, `Notebook, ${tab}`);
    const closed = await pocket.locator("li.rw-li:not(.rw-li-open)").evaluateAll((els) => els.map((e) => e.getBoundingClientRect().height));
    if (closed.length > 0) expect(Math.max(...closed), `L4: a closed row, Notebook, ${tab}`).toBeLessThanOrEqual(80);
  }

  // L3: whole people on the first screen. With fewer than eight who matter now, the largest section is opened to fill it.
  pocket = await openPocket(page, "people");
  if ((await pocket.getByTestId("notebook-people-now").getByTestId("notebook-person").count()) < 8) await openFold(pocket.getByTestId("notebook-people-rest"));
  await pocket.evaluate((el) => (el.scrollTop = 0));
  await settle(page);
  const bottom = await sheetBottom(pocket);
  const whole = await pocket.getByTestId("notebook-person").evaluateAll((els, b) => els.filter((e) => e.getBoundingClientRect().bottom <= b + 0.5).length, bottom);
  expect(whole, "L3: whole people on the first screen").toBeGreaterThanOrEqual(5);
  const personRows = await pocket.locator("li.rw-li:not(.rw-li-open)[data-testid='notebook-person']").evaluateAll((els) => els.map((e) => e.getBoundingClientRect().height));
  expect(Math.max(...personRows), "L4: a closed person").toBeLessThanOrEqual(80);
  // L4: an opened person at most 0.75 of a screen.
  const first = pocket.getByTestId("notebook-person").first();
  await openRow(first);
  await settle(page);
  expect((await first.boundingBox())?.height ?? 999, "L4: an opened person").toBeLessThanOrEqual(0.75 * SCREEN);
  await first.locator(":scope > .rw-row > .rw-main").click();

  // L1, L6: the arc sheet; "What you can do" and its first three rows on its first screen.
  pocket = await openPocket(page, "today");
  await pocket.locator("[data-testid^='arc-row-'] .rw-main").first().click();
  const arc = page.getByTestId("arc-sheet");
  await expect(arc).toBeVisible();
  const a = await lengthOf(page, arc, arc.getByTestId("arc-clock").or(arc.getByTestId("arc-ended")).first());
  expect(a.screens, "L1: the arc sheet").toBeLessThanOrEqual(2.0);
  expect(a.firstAt, "L2: the arc sheet").toBeLessThanOrEqual(0.25);
  const arcBottom = await sheetBottom(arc);
  const beats = arc.getByTestId("arc-beats").locator(":scope > .rw-rows > li");
  const n = Math.min(3, await beats.count());
  expect(n, "the arc has things to do").toBeGreaterThan(0);
  for (let i = 0; i < n; i++) expect((await beats.nth(i).boundingBox())?.y ?? 9e9, `L6: row ${i + 1}`).toBeLessThanOrEqual(arcBottom - ((await beats.nth(i).boundingBox())?.height ?? 0) + 0.5);
  await expectCleanText(arc, "the arc sheet");
  await arc.getByRole("button", { name: "Close" }).click();
  await expect(arc).toHaveCount(0);
}

test.beforeEach(async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: W, height: H });
});

test("L1 to L4, L6 and L8 on a run at turn 10 (an arc, an offer and a decision waiting)", async ({ page }) => {
  test.setTimeout(180_000);
  await startRun(page, SEED);
  await playTo(page, 10);
  await expectMidRun(page, 20);
  await budget(page);
});

test("L1, L2 and L8 on an early run (turn 4)", async ({ page }) => {
  test.setTimeout(120_000);
  await startRun(page, SEED);
  await playTo(page, 4);
  let pocket = await openPocket(page, "today");
  const today = await lengthOf(page, pocket, pocket.getByTestId("today-strip"));
  expect(today.screens, "L1: Today").toBeLessThanOrEqual(2.0);
  expect(today.firstAt, "L2: Today").toBeLessThanOrEqual(0.25);
  await expectCleanText(pocket, "Today");
  for (const tab of ["people", "places", "town", "policies"] as const) {
    pocket = await openPocket(page, tab);
    const m = await lengthOf(page, pocket, pocket.getByRole("tabpanel", { name: new RegExp(tab, "i") }));
    expect(m.screens, `L1: Notebook, ${tab}`).toBeLessThanOrEqual(2.0);
    expect(m.firstAt, `L2: Notebook, ${tab}`).toBeLessThanOrEqual(0.25);
    await expectCleanText(pocket, `Notebook, ${tab}`);
  }
});

test("L5: with a decision waiting, its row is wholly on the first screen at 360 × 640", async ({ page }) => {
  test.setTimeout(150_000);
  await startRun(page, SEED);
  await playTo(page, 10);
  await page.setViewportSize({ width: 360, height: 640 });
  const pocket = await openPocket(page, "today");
  await pocket.evaluate((el) => (el.scrollTop = 0));
  await settle(page);
  const row = pocket.locator("[data-need='decision']").first();
  await expect(row).toBeVisible();
  const b = await row.boundingBox();
  expect((b?.y ?? 0) + (b?.height ?? 0)).toBeLessThanOrEqual(await sheetBottom(pocket));
});

for (const width of [320, 360, 390]) {
  test(`L7: no sideways scroll and 44 px targets at ${width} px with a wide font`, async ({ page }) => {
    test.setTimeout(150_000);
    await useWideFont(page);
    await startRun(page, SEED);
    await playTo(page, 10);
    await page.setViewportSize({ width, height: 780 });
    let pocket = await openPocket(page, "today");
    await settle(page);
    await expectTargets(page, pocket, "Today");
    // The switch and the ⋯ menu stay whole.
    for (const id of ["pocket-tab-today", "pocket-tab-notebook", "pocket-more"]) {
      const box = await pocket.getByTestId(id).boundingBox();
      expect((box?.x ?? -1) >= 0 && (box?.x ?? 0) + (box?.width ?? 0) <= width + 0.5, `${id} at ${width} px`).toBe(true);
    }
    const menu = await pocketMenu(page);
    expect(((await menu.boundingBox())?.x ?? -1) >= 0, "the menu is on screen").toBe(true);
    await pocket.getByTestId("pocket-more").click();
    for (const tab of ["people", "places", "town", "policies"] as const) {
      pocket = await openPocket(page, tab);
      await settle(page);
      await expectTargets(page, pocket, `Notebook, ${tab}`);
      for (const n of await pocket.locator(".nb-tab-name").all()) expect(await n.evaluate((el) => el.scrollWidth <= el.clientWidth + 1), `a tab's name at ${width} px`).toBe(true);
    }
    await openPeople(page);
    await settle(page);
    await expectTargets(page, pocket, "a person opened");
    pocket = await openPocket(page, "today");
    await pocket.locator("[data-testid^='arc-row-'] .rw-main").first().click();
    const arc = page.getByTestId("arc-sheet");
    await expect(arc).toBeVisible();
    await settle(page);
    await expectTargets(page, arc, "the arc sheet");
  });
}

test("L9: activations from the street: a decision answered in 3, a person who matters gone to in 3, an arc's actions in 2, Save in 3", async ({ page }) => {
  test.setTimeout(180_000);
  await startRun(page, SEED);
  await playTo(page, 10);
  let taps = 0;
  const tap = async (l: Locator) => {
    taps++;
    await l.click();
  };
  const closeAll = async () => {
    for (const id of ["arc-sheet", "decision-sheet", "place-panel", "pocket"]) {
      const s = page.getByTestId(id);
      if ((await s.count()) > 0) await s.getByRole("button", { name: "Close", exact: true }).first().click();
    }
  };
  // Leave the pocket on Today, its Notebook on People (how a player last left it).
  await openPocket(page, "people");
  await openPocket(page, "today");
  await closeAll();

  // An arc's actions: the pocket, the arc's row.
  taps = 0;
  await tap(page.getByTestId("pocket-button"));
  await tap(page.getByTestId("pocket").locator("[data-testid^='arc-row-'] .rw-main").first());
  await expect(page.getByTestId("arc-sheet").getByTestId("arc-beats")).toBeVisible();
  expect(taps, "an arc's actions").toBe(2);
  await closeAll();

  // A person who matters now: the pocket, the Notebook, Go on their row.
  taps = 0;
  await tap(page.getByTestId("pocket-button"));
  const sw = page.getByTestId("pocket").getByTestId("pocket-tab-notebook");
  if ((await sw.getAttribute("data-on")) !== "true") await tap(sw);
  await tap(page.getByTestId("pocket").getByTestId("notebook-people-now").locator("[data-testid='notebook-person'] .rw-end button").first());
  await expect(page.getByTestId("place-panel").or(page.getByTestId("talk-panel")).first()).toBeVisible({ timeout: 15_000 });
  expect(taps, "a person who matters now").toBeLessThanOrEqual(3);
  await closeAll();
  await openPocket(page, "today");
  await closeAll();

  // A decision, answered by the keyboard: Enter on the pocket button, on the decision's row, on an option.
  taps = 0;
  await page.getByTestId("pocket-button").focus();
  await page.keyboard.press("Enter");
  taps++;
  const row = page.getByTestId("pocket").locator("[data-need='decision'] .rw-main").first();
  // "Needs you" is one Tab stop; Down moves along it (moves, not activations).
  const inNeeds = () => page.evaluate(() => document.activeElement?.closest("[data-testid='pocket-needs-list']") !== null);
  for (let i = 0; i < 40 && !(await inNeeds()); i++) await page.keyboard.press("Tab");
  for (let i = 0; i < 10 && !(await row.evaluate((el) => el === document.activeElement)); i++) await page.keyboard.press("ArrowDown");
  await expect(row).toBeFocused();
  await page.keyboard.press("Enter");
  taps++;
  const sheet = page.getByTestId("decision-sheet");
  await expect(sheet).toBeVisible();
  const option = sheet.getByTestId("decision-options").locator("button:enabled").first();
  await option.focus();
  await page.keyboard.press("Enter");
  taps++;
  await expect(sheet.getByTestId("decision-aftermath")).toBeVisible();
  expect(taps, "a decision").toBe(3);
  await closeAll();

  // Save: the pocket, ⋯, Save & quit.
  taps = 0;
  await tap(page.getByTestId("pocket-button"));
  await tap(page.getByTestId("pocket").getByTestId("pocket-more"));
  await tap(page.getByTestId("pocket-menu").getByRole("button", { name: "Save & quit" }));
  await expect(page.getByRole("button", { name: "Continue" }).first()).toBeVisible();
  expect(taps, "Save").toBe(3);
});

test("a mid-career run (R3's file): at least 50 people known, and the budget holds", async ({ page }) => {
  test.skip(!existsSync(MID_CAREER), "the mid-career run file (POCKET_REWORK §6.4 R3) is not made yet");
  test.setTimeout(180_000);
  await page.goto("./");
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Import a run" }).click();
  await (await chooser).setFiles(MID_CAREER);
  await page.getByRole("button", { name: "Continue" }).first().click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expectMidRun(page, 50);
  await budget(page);
});
