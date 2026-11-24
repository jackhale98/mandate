import { expect, test } from "./csp";
import type { Locator, Page } from "@playwright/test";
import { useWideFont } from "./wide-font";
import { officeFigures, openPeople, openPocket } from "./pocket";

// DEC-088 main screen (owner, 2026-11-19): the street full screen with a slim
// bar; the pocket has tabs (Today, Town, Places, Notebook; people are the
// Notebook's People section) with badges, docks
// beside the town on wide screens, and with the skip link and landmarks
// reaches every place without the canvas (DEC-051). Desktop and phone.

const start = async (page: Page, seed: string) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill(seed);
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
  await expect(page.locator(".town-street-stage[data-ready='1']")).toBeVisible({ timeout: 30_000 });
};

/** Wait until nothing finite is animating and the fonts are loaded, then two frames, so the layout is the settled one. */
async function settled(page: Page) {
  await page.evaluate(async () => {
    await document.fonts.ready;
    await Promise.all(document.getAnimations().filter((a) => a.effect?.getComputedTiming().iterations !== Infinity).map((a) => a.finished.catch(() => undefined)));
    await new Promise<void>((done) => requestAnimationFrame(() => requestAnimationFrame(() => done())));
  });
}

/** The town card's rows (the defining industry, the crisis, ...) as one reading of the layout: any row that starts above the end of the one before, or text running out of its row. */
async function townCardOverlaps(page: Page, pocket: Locator): Promise<string[]> {
  await settled(page);
  return pocket.getByTestId("town-card").evaluate((card) => {
    const rows = [...card.querySelectorAll<HTMLElement>(".facts > div")];
    const out: string[] = [];
    rows.forEach((row, i) => {
      const r = row.getBoundingClientRect();
      const prev = rows[i - 1]?.getBoundingClientRect();
      if (prev && r.top < prev.bottom - 0.5) out.push(`row ${i} starts at ${r.top} above the end of the one before (${prev.bottom})`);
      for (const dd of row.querySelectorAll<HTMLElement>("dd")) {
        const b = dd.getBoundingClientRect();
        if (b.bottom > r.bottom + 0.5) out.push(`row ${i}: its text runs below its row (${b.bottom} > ${r.bottom})`);
        if (dd.scrollWidth > dd.clientWidth + 1) out.push(`row ${i}: its text is wider than its row`);
      }
    });
    return out;
  });
}

test("landmarks, the bar, and the skip link to Places", async ({ page }) => {
  await start(page, "4242");
  // The town is the main landmark; the bar is navigation; the canvas points to Places.
  await expect(page.getByRole("main")).toHaveAttribute("data-testid", "town");
  await expect(page.getByRole("navigation", { name: "Game" })).toBeVisible();
  await expect(page.locator(".town-street-stage canvas")).toHaveAttribute("aria-label", /Places tab/);
  const bar = page.getByTestId("gamebar");
  await expect(bar.getByRole("button", { name: "Town map" })).toBeVisible();
  await expect(bar.getByRole("button", { name: /Pocket/ })).toBeVisible();
  await expect(bar.getByTestId("hud-slots")).toHaveAttribute("aria-label", "3 slots left this turn");
  await expect(bar.getByText("Turn 0")).toBeVisible();

  // The skip link is the first stop for the keyboard, visible only when focused.
  const skip = page.getByTestId("skip-to-places");
  await expect(skip).not.toBeInViewport();
  await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
  await page.keyboard.press("Tab");
  await expect(skip).toBeFocused();
  await expect(skip).toBeInViewport();
  await page.keyboard.press("Enter");
  // It opens the pocket's Places tab and moves the keyboard into it.
  const pocket = page.getByTestId("pocket");
  await expect(pocket).toBeVisible();
  await expect(pocket).toHaveRole("complementary");
  await expect(page.getByTestId("pocket-tab-places")).toHaveAttribute("aria-selected", "true");
  const panel = page.getByRole("tabpanel", { name: /Places/ });
  await expect(panel).toBeFocused();
  // Every place is a button: Tab reaches one, Enter goes there and opens it (no canvas involved).
  await page.keyboard.press("Tab");
  await expect(page.locator(".spot:focus")).toHaveCount(1);
  const name = ((await page.locator(".spot:focus .spot-name").textContent()) ?? "").trim();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("place-panel").locator("#place-title")).toContainText(name);
  // The label at the top says where you are: the neighbourhood for its streets, else the place.
  const streets = /^Streets of (.+)$/.exec(name)?.[1];
  if (streets) await expect(page.getByTestId("where-zone")).toHaveText(streets);
  else await expect(page.getByTestId("where-place")).toHaveText(name);
});

test("where you are: a label at the top of the street names the neighbourhood and the place", async ({ page }) => {
  await start(page, "4242");
  const where = page.getByTestId("where");
  const zone = page.getByTestId("where-zone");
  // Not in the bar any more: at the top of the street, never taking taps.
  await expect(page.getByTestId("gamebar").getByTestId("where")).toHaveCount(0);
  await expect(page.getByTestId("town").getByTestId("where")).toBeVisible();
  await expect(where).toHaveCSS("pointer-events", "none");
  const box = await where.boundingBox();
  const town = await page.getByTestId("town").boundingBox();
  expect((box?.y ?? 99) - (town?.y ?? 0)).toBeLessThan(40);
  // A new run starts at home, in the home neighbourhood.
  await expect(page.getByTestId("where-place")).toHaveText("Home");
  // The district in the constitution's words follows the neighbourhood ("Old Green · Ward 2 · Home").
  await expect(page.getByTestId("where-ward")).toHaveText(/^Ward \d+$/);
  const pocket = await openPocket(page, "places");
  const home = ((await pocket.locator(".hood", { has: page.getByTestId("you-are-here") }).locator("h3").textContent()) ?? "").trim();
  await expect(zone).toHaveText(home);
  await expect(page.getByTestId("where-announce")).toContainText(home);

  // Go to a place in another neighbourhood: the zone and the place change, and the new zone is announced.
  const places = ".spot:not([data-testid='spot-streets'])";
  const other = pocket.locator(".hood").filter({ hasNot: page.getByTestId("you-are-here") }).filter({ has: page.locator(places) }).first();
  const hood = ((await other.locator("h3").textContent()) ?? "").trim();
  expect(hood).not.toBe(home);
  const spot = other.locator(places).first();
  const place = ((await spot.locator(".spot-name").textContent()) ?? "").trim();
  await spot.click();
  await expect(page.getByTestId("place-panel").locator("#place-title")).toContainText(place);
  await expect(zone).toHaveText(hood);
  await expect(page.getByTestId("where-place")).toHaveText(place);
  await expect(page.getByTestId("where-announce")).toContainText(hood);
  await expect(page.getByTestId("where-announce")).not.toContainText(place);
});

test("the pocket's tabs: a tab list with arrow keys, panels, and badges that count", async ({ page }) => {
  await start(page, "4242");
  const pocket = await openPocket(page, "today");
  const tabs = pocket.getByRole("tablist");
  await expect(tabs.getByRole("tab")).toHaveText([/Today/, /Town/, /Places/, /Notebook/]);
  // Only the open tab is in the Tab order; arrows, Home and End move between tabs.
  await pocket.getByTestId("pocket-tab-today").focus();
  await page.keyboard.press("ArrowRight");
  await expect(pocket.getByTestId("pocket-tab-town")).toBeFocused();
  await expect(pocket.getByTestId("pocket-tab-town")).toHaveAttribute("aria-selected", "true");
  await expect(pocket.getByTestId("pocket-tab-town")).toHaveAttribute("tabindex", "0");
  await expect(pocket.getByTestId("pocket-tab-today")).toHaveAttribute("tabindex", "-1");
  await expect(pocket.getByRole("tabpanel")).toHaveAttribute("aria-labelledby", "pocket-tab-town");
  await expect(pocket.getByTestId("town-card")).toBeVisible();
  await expect(pocket.getByTestId("town-hoods").getByTestId("town-hood").first()).toBeVisible();
  // Each neighbourhood says which ward it is in (note 4).
  await expect(pocket.getByTestId("town-hoods").getByTestId("town-hood-ward").first()).toHaveText(/Ward \d+/);
  // The crisis line does not run into the defining industry's (note 4): measured once the sheet has stopped
  // sliding in (two reads of a moving sheet differ by the distance it moved) and the fonts are in.
  expect(await townCardOverlaps(page, pocket)).toEqual([]);
  await page.keyboard.press("End");
  await expect(pocket.getByTestId("pocket-tab-notebook")).toBeFocused();
  await expect(pocket.getByTestId("notebook")).toBeVisible();
  await page.keyboard.press("ArrowRight");
  await expect(pocket.getByTestId("pocket-tab-today")).toBeFocused();
  await expect(pocket.getByTestId("agenda")).toBeVisible();
  await page.keyboard.press("Home");
  await expect(pocket.getByTestId("pocket-tab-today")).toBeFocused();

  // People (the Notebook's People section): a name opens their character sheet (relationships).
  await openPeople(page);
  await officeFigures(pocket).getByTestId("notebook-person-name").first().click();
  const sheet = page.getByTestId("character-sheet");
  await expect(sheet.getByTestId("character-relationship")).toBeVisible();
  await sheet.getByRole("button", { name: "Close" }).click();

  // Badges: the Notebook tab counts what was learned this turn, and says so in words.
  const learned = await pocket.getByTestId("pocket-tab-badge-notebook").count();
  if (learned > 0) {
    const n = (await pocket.getByTestId("pocket-tab-badge-notebook").textContent()) ?? "";
    await expect(pocket.getByTestId("pocket-tab-notebook")).toHaveAccessibleName(new RegExp(`Notebook ?, ${n} new in your notebook`));
    await expect(page.getByTestId("notebook-badge")).toContainText(n);
  }
  // The Today badge counts decisions, asks waiting and alerts, as the pocket button's do.
  const decisions = await page.getByTestId("decision-badge").count();
  const alerts = await page.getByTestId("alert-badge").count();
  const asks = await page.getByTestId("asks-badge").count();
  if (decisions + alerts + asks > 0) await expect(pocket.getByTestId("pocket-tab-badge-today")).toBeVisible();
  else await expect(pocket.getByTestId("pocket-tab-badge-today")).toHaveCount(0);
});

test("phone: the pocket is a bottom sheet with its tabs at the top", async ({ page }, info) => {
  test.skip(info.project.name !== "phone", "bottom sheets are a phone layout");
  await start(page, "4242");
  const pocket = await openPocket(page, "places");
  await page.waitForTimeout(400);
  const box = await pocket.boundingBox();
  const tabs = await pocket.getByRole("tablist").boundingBox();
  const bar = await page.getByTestId("gamebar").boundingBox();
  // It stands on the bar (End turn stays reachable) and its tabs are in its head.
  expect((box?.y ?? 0) + (box?.height ?? 0)).toBeLessThanOrEqual((bar?.y ?? 0) + 1);
  expect((tabs?.y ?? 0) - (box?.y ?? 0)).toBeLessThan(160);
  // The tabs stay at the top while the panel scrolls.
  await pocket.evaluate((el) => (el.scrollTop = 500));
  await expect.poll(async () => ((await pocket.getByRole("tablist").boundingBox())?.y ?? 0) - ((await pocket.boundingBox())?.y ?? 0)).toBeLessThan(160);
  await expect(pocket.getByTestId("pocket-dock")).toBeHidden();
});

test("desktop: the pocket docks beside the town; the street makes room and the pocket stays open", async ({ page }, info) => {
  test.skip(info.project.name !== "desktop", "docking is a wide-screen layout");
  await start(page, "4242");
  const wide = (await page.getByTestId("town").boundingBox())?.width ?? 0;
  const pocket = await openPocket(page, "places");
  // Not docked: the pocket lies over the town, and going somewhere closes it.
  expect((await page.getByTestId("town").boundingBox())?.width).toBeCloseTo(wide, 0);
  const dock = pocket.getByTestId("pocket-dock");
  await expect(dock).toHaveAttribute("aria-pressed", "false");
  await dock.click();
  await expect(dock).toHaveAttribute("aria-pressed", "true");
  // Docked: the street resizes to the space beside it…
  await expect.poll(async () => { const b = await page.getByTestId("town").boundingBox(); return b ? b.x + b.width : wide; }).toBeLessThan(wide - 390);
  await expect.poll(async () => (await page.locator(".town-street-stage canvas").boundingBox())?.width ?? 0).toBeLessThan(wide - 390);
  // …and the pocket stays open when a place is chosen.
  await pocket.getByTestId("spot-town_hall").click();
  await expect(page.getByTestId("place-panel")).toBeVisible();
  await expect(page.getByTestId("where")).toContainText("Town hall");
  await expect(pocket).toBeVisible();
  // The choice is remembered on this device.
  await page.reload();
  await page.getByRole("button", { name: "Continue" }).first().click();
  await expect(page.getByTestId("town")).toBeVisible();
  await page.getByTestId("pocket-button").click();
  await expect(page.getByTestId("pocket-dock")).toHaveAttribute("aria-pressed", "true");
  await expect.poll(async () => { const b = await page.getByTestId("town").boundingBox(); return b ? b.x + b.width : wide; }).toBeLessThan(wide - 390);
  // Undocked again, the town takes the whole width back once the pocket closes.
  await page.getByTestId("pocket-dock").click();
  await page.getByTestId("pocket").getByRole("button", { name: "Close" }).click();
  await expect(page.getByTestId("pocket")).toHaveCount(0);
  await expect.poll(async () => (await page.getByTestId("town").boundingBox())?.width ?? 0).toBeCloseTo(wide, 0);
});

/** Canvass a neighbourhood from the agenda: what the doors say is new in the notebook. */
const learnSomething = async (page: Page) => {
  const agenda = (await openPocket(page, "today")).getByTestId("agenda");
  await agenda.getByText("Canvass a neighbourhood", { exact: true }).click();
  await agenda.getByRole("button", { name: "Go" }).click();
  await expect(page.getByTestId("toast")).toHaveText(/knocked on doors/);
  if ((await page.getByTestId("pocket").count()) > 0) await page.getByTestId("pocket").getByRole("button", { name: "Close" }).first().click();
  await expect(page.getByTestId("pocket")).toHaveCount(0);
};

test("the notebook badge: none on a new game (what you start with is not news); learning something adds one", async ({ page }) => {
  await start(page, "4242");
  await expect(page.getByTestId("notebook-badge")).toHaveCount(0);
  const pocket = await openPocket(page, "town");
  await expect(pocket.getByTestId("pocket-tab-badge-notebook")).toHaveCount(0);
  await learnSomething(page);
  await expect(page.getByTestId("notebook-badge")).toBeVisible();
  expect(Number(((await page.getByTestId("notebook-badge").textContent()) ?? "").replace(/\D/g, ""))).toBeGreaterThan(0);
});

test("phone: the pocket's four tabs and their badges fit at 360 and 412 px", async ({ page }, info) => {
  test.skip(info.project.name !== "phone", "a phone layout");
  await start(page, "4242");
  await learnSomething(page);
  await expect(page.getByTestId("notebook-badge")).toBeVisible();
  const tabs = ["today", "town", "places", "notebook"];
  for (const width of [360, 412]) {
    await page.setViewportSize({ width, height: 780 });
    const pocket = await openPocket(page, "today");
    // People are the Notebook's People section: no tab of their own.
    await expect(pocket.getByRole("tablist").first().getByRole("tab")).toHaveCount(tabs.length);
    await page.waitForTimeout(300);
    const list = pocket.getByRole("tablist");
    expect(await list.evaluate((el) => el.scrollWidth <= el.clientWidth + 1)).toBe(true);
    const lb = await list.boundingBox();
    const right = Math.min(width, (lb?.x ?? 0) + (lb?.width ?? 0)) + 0.5;
    for (const el of [...tabs.map((t) => pocket.getByTestId(`pocket-tab-${t}`)), pocket.getByTestId("pocket-tab-badge-notebook")]) {
      await expect(el).toBeVisible();
      const b = await el.boundingBox();
      expect(b?.x ?? -1, `${width} px`).toBeGreaterThanOrEqual((lb?.x ?? 0) - 0.5);
      expect((b?.x ?? 0) + (b?.width ?? 0), `${width} px`).toBeLessThanOrEqual(right);
    }
    // Every label whole, none cut short.
    for (const t of tabs) {
      expect(await pocket.getByTestId(`pocket-tab-${t}`).locator(".pocket-tab-name").evaluate((el) => el.scrollWidth <= el.clientWidth + 1), `${t} at ${width} px`).toBe(true);
    }
    await pocket.getByRole("button", { name: "Close" }).first().click();
    await expect(pocket).toHaveCount(0);
  }
});

// Play note 21: the crisis line ran into the defining industry's. The card is one flow of full-width rows, so
// at any width from 320 px with the wide face a long crisis wraps inside its own row.
for (const width of [320, 360, 390]) {
  test(`the town card's crisis line stays below the industry line at ${width}px (wide font)`, async ({ page }) => {
    await useWideFont(page);
    await page.setViewportSize({ width, height: 800 });
    await start(page, "4242");
    const pocket = await openPocket(page, "town");
    await expect(pocket.getByTestId("town-card")).toBeVisible();
    // A crisis as the card prints it (a long name, its band and trend, since when), whether or not this run has one.
    await pocket.getByTestId("town-crisis").evaluate((dd) => {
      dd.innerHTML = '<strong>Flooding along the whole lower river</strong> · severe, worsening <span class="muted small">(since March of year 3)</span>';
    });
    await pocket.getByTestId("town-industry").evaluate((dd) => {
      dd.textContent = "Agricultural processing and packing · 38% of the town's work";
    });
    expect(await townCardOverlaps(page, pocket)).toEqual([]);
    if (process.env.TOWN_SHOTS) await page.screenshot({ path: `${process.env.TOWN_SHOTS}/town-card-${width}.png` });
    const [ind, cri] = await Promise.all([pocket.getByTestId("town-industry").boundingBox(), pocket.getByTestId("town-crisis").boundingBox()]);
    expect(cri?.y ?? 0).toBeGreaterThanOrEqual((ind?.y ?? 0) + (ind?.height ?? 0) - 1);
  });
}
