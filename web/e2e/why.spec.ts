import { expect, test } from "./csp";
import type { Locator, Page } from "@playwright/test";
import { pocketMenu, showPlaces } from "./pocket";
import { seedFor } from "./seeds";
import { useWideFont } from "./wide-font";

// "Why?" (the causal spine's first player-facing surface): play to a real
// outcome, tap its "Why?", and read what led to it. The panel is a sheet: it
// names each step in a sentence with its date and how it led on, closes with
// Escape and its button, gives the keyboard back, and fits a narrow phone
// with wide fonts. MANDATE_CAPTURE=<dir> saves screenshots
// (deviceScaleFactor 2) for review, and plays on to an election and a
// closure for theirs.

test.use({ deviceScaleFactor: 2 });

const capture = process.env.MANDATE_CAPTURE;

const start = async (page: Page, seed: string) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill(seed);
  await page.getByText("Small", { exact: true }).click();
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
};

const endTurn = async (page: Page) => {
  const before = await page.locator(".gamebar-date strong").textContent();
  await page.getByRole("button", { name: "End turn" }).click();
  await expect(page.locator(".gamebar-date strong")).not.toHaveText(before ?? "", { timeout: 30_000 });
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled({ timeout: 30_000 });
  await expect(page.getByTestId("turn-passage")).toHaveCount(0, { timeout: 30_000 });
};

/** The overview (from the pocket): the country's recent events. */
const openOverview = async (page: Page) => {
  if ((await page.locator(".viewbar").count()) === 0) await (await pocketMenu(page)).getByRole("button", { name: "Overview", exact: true }).click();
  // On a phone the pocket covers the list: put it away.
  if ((await page.getByTestId("pocket").count()) > 0 && (page.viewportSize()?.width ?? 0) < 900) {
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("pocket")).toHaveCount(0);
  }
  await expect(page.locator(".events").first()).toBeVisible();
};

/** A line of the overview's event list, by its text. */
const eventLine = (page: Page, text: RegExp): Locator => page.locator("main.game-body .events > li", { hasText: text });

/** End turns (at most `turns`) until the overview lists a line like `text`. */
async function playUntilListed(page: Page, text: RegExp, turns: number): Promise<Locator> {
  for (let i = 0; i < turns; i++) {
    await endTurn(page);
    await openOverview(page);
    if ((await eventLine(page, text).count()) > 0) break;
  }
  // The list is newest first, and the storyteller may list an event again when it brings it to the player's notice: the
  // last match is the event itself.
  const line = eventLine(page, text).last();
  await expect(line).toBeVisible();
  return line;
}

/** Text a player must never be shown: a missing key, a raw key or id, an unfilled argument. */
const RAW = /⟦|why-|ui-|event-|concern\.|\{\s*\$/;

/** The open panel reads as sentences: at least `steps` steps, each dated, joined by a phrase, nothing raw. */
async function expectReadable(panel: Locator, steps: number) {
  const rows = panel.getByTestId("why-step");
  await expect(rows.first()).toBeVisible();
  expect(await rows.count()).toBeGreaterThanOrEqual(steps);
  const text = (await panel.innerText()).trim();
  expect(text).not.toMatch(RAW);
  for (const row of await rows.all()) {
    await expect(row.locator("time")).toHaveText(/^\d{1,2} [A-Z][a-z]+ \d{4}$/);
    await expect(row.locator(".why-text")).toHaveText(/\S.*[.!?)]$/);
  }
  // Every step but the first says how it led on.
  expect(await panel.getByTestId("why-join").count()).toBe((await rows.count()) - 1);
  for (const j of await panel.getByTestId("why-join").all()) await expect(j).toHaveText(/because of|after|prompted by|which brought to light|which ended/);
}

/** The panel and everything in it stay inside the viewport's width; the page does not scroll sideways. */
async function expectFits(page: Page, panel: Locator) {
  const width = page.viewportSize()?.width ?? 0;
  // Once it has slid in.
  await expect.poll(() => panel.evaluate((el) => el.getAnimations().filter((a) => a.playState === "running").length)).toBe(0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  const out = await panel.evaluate((el, w) => {
    const bad: string[] = [];
    if (el.scrollWidth > el.clientWidth + 1) bad.push(`the panel scrolls sideways (${el.scrollWidth} > ${el.clientWidth})`);
    const box = el.getBoundingClientRect();
    const right = Math.min(w, box.right);
    if (box.left < -0.5 || box.right > w + 0.5) bad.push(`the panel ${Math.round(box.left)}..${Math.round(box.right)}`);
    for (const n of el.querySelectorAll<HTMLElement>("*")) {
      const r = n.getBoundingClientRect();
      if (r.width === 0) continue;
      if (r.left < box.left - 0.5 || r.right > right + 0.5) bad.push(`${String(n.className) || n.tagName} ${Math.round(r.left)}..${Math.round(r.right)}`);
    }
    return bad;
  }, width);
  expect(out).toEqual([]);
}

test("a law's Why? reads back to its introduction, closes with Escape and gives the keyboard back", async ({ page }, info) => {
  test.setTimeout(180_000);
  if (info.project.name === "phone") await page.setViewportSize({ width: 390, height: 844 });
  // A small world where a law is passed or voted down, and listed, within six idle turns.
  await start(page, seedFor("law_decided_early_with_its_chain"));
  const line = await playUntilListed(page, /passed the .* bill|voted down the .* bill/, 6);

  // By keyboard: the control is a button with the outcome in its name.
  const why = line.getByTestId("why-button");
  await expect(why).toHaveText("Why?");
  await expect(why).toHaveAccessibleName(/^Why\? \(.*bill.*\)$/);
  await why.focus();
  await page.keyboard.press("Enter");
  const panel = page.getByTestId("why-panel");
  await expect(panel).toBeVisible();
  await expect(panel.getByRole("heading", { name: "Why this happened" })).toBeFocused();
  await expectReadable(panel, 2);
  // The outcome at the top, then the bill coming before the body.
  await expect(panel.getByTestId("why-step").first().locator(".why-text")).toHaveText(/passed the .* bill|voted down the .* bill/);
  await expect(panel.getByTestId("why-chain")).toContainText(/A new bill in /);
  // What each step names as its start is said in words under it.
  await expect(panel.getByTestId("why-root").first()).toHaveText(/\S+ \S+/);
  await expectFits(page, panel);
  if (capture) await page.screenshot({ path: `${capture}/why-law-${info.project.name}.png` });

  // Nothing holds the keyboard: Tab moves on through the panel and out of it.
  await page.keyboard.press("Tab");
  await expect(panel.getByRole("button", { name: "Close" })).toBeFocused();
  // Escape closes it and the control has the keyboard again.
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
  await expect(why).toBeFocused();
  // The close button and, on a phone, a tap outside do the same.
  await why.click();
  await expect(panel).toBeVisible();
  await panel.getByRole("button", { name: "Close" }).click();
  await expect(panel).toHaveCount(0);
});

test("the same panel from a place reached by the Places list: a story in the town's paper", async ({ page }) => {
  test.setTimeout(180_000);
  // The same world: within six idle turns its paper runs a story that names an event.
  await start(page, seedFor("law_decided_early_with_its_chain"));
  const paper = page.getByTestId("newspaper");
  for (let turn = 0; turn < 6; turn++) {
    await endTurn(page);
    // To the newspaper by the list of places (no walking, no canvas): quick mode's path and the accessible one.
    await showPlaces(page);
    const office = (await page.getByTestId("spot-news_office").count()) > 0 ? "spot-news_office" : "spot-town_hall";
    const spot = page.getByTestId(office).first();
    await spot.evaluate((el) => el.scrollIntoView({ block: "center" }));
    await spot.click();
    await expect(paper).toBeVisible({ timeout: 30_000 });
    if ((await paper.getByTestId("why-button").count()) > 0) break;
    await page.getByTestId("place-panel").getByRole("button", { name: "Close" }).first().click();
    await expect(page.getByTestId("place-panel")).toHaveCount(0);
  }
  const story = paper.getByTestId("article").filter({ has: page.getByTestId("why-button") }).first();
  const why = story.getByTestId("why-button");
  await why.evaluate((el) => el.scrollIntoView({ block: "center" }));
  // The control is named after its story.
  const headline = ((await story.locator(".article-head").innerText()) ?? "").trim();
  expect(await why.getAttribute("aria-label")).toBe(`Why? (${headline})`);
  await why.click();
  const panel = page.getByTestId("why-panel");
  await expectReadable(panel, 1);
  await expectFits(page, panel);
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
  // The place is as it was under the panel.
  await expect(paper).toBeVisible();
});

test("a plant in trouble names what weighed on it", async ({ page }, info) => {
  test.setTimeout(180_000);
  if (info.project.name === "phone") await page.setViewportSize({ width: 390, height: 844 });
  // Since industries (DEC-110) a works closes by stages: short time, a dated notice, then the gates. A small
  // world where one of them is listed, with what led to it, within eight idle turns.
  const trouble = /(Short time at|Closure notice at|The gates shut at) /;
  await start(page, seedFor("plant_in_trouble_listed_early"));
  const line = await playUntilListed(page, trouble, 8);
  await line.getByTestId("why-button").click();
  const panel = page.getByTestId("why-panel");
  await expectReadable(panel, 2);
  const steps = panel.getByTestId("why-step");
  await expect(steps.first().locator(".why-text")).toHaveText(trouble);
  // What weighed on the works is named somewhere in the chain, in words.
  const pressure = panel.locator("[data-testid='why-root'][data-kind='pressure']").first();
  await expect(pressure).toBeVisible();
  await expect(pressure).not.toHaveText(/why-|\{\s*\$|[a-z]+\.[a-z_]+/);
  await expectFits(page, panel);
  if (capture) await page.screenshot({ path: `${capture}/why-closure-${info.project.name}.png` });
});

test("election night: a result's Why? shows the race's chain with the engine's factors beside it", async ({ page }, info) => {
  // One project plays the campaign through (the smoke spec plays it on both); a capture run takes both.
  test.skip(info.project.name !== "phone" && !capture, "played once");
  test.setTimeout(300_000);
  if (info.project.name === "phone") await page.setViewportSize({ width: 390, height: 844 });
  // The night between turns as a quick fade: forty turns to election day.
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("4040");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  const endTurnButton = page.getByRole("button", { name: "End turn" });
  const night = page.getByTestId("election-night");
  const epilogue = page.getByTestId("open-epilogue");
  for (let i = 0; i < 45 && (await night.count()) === 0 && (await epilogue.count()) === 0; i++) {
    await endTurnButton.click();
    await expect(endTurnButton.or(epilogue)).toBeEnabled({ timeout: 30_000 });
  }
  await expect(night).toBeVisible();
  // The player's own result: its card has the control, under the factors.
  const mine = night.getByTestId("my-result").first();
  await expect(mine).toBeVisible();
  const why = mine.getByTestId("why-button");
  await why.evaluate((el) => el.scrollIntoView({ block: "center" }));
  await why.click();
  const panel = page.getByTestId("why-panel");
  await expect(panel).toBeVisible();
  // The race, and behind it the campaign that led to election day.
  await expectReadable(panel, 2);
  await expect(panel.getByTestId("why-step").first().locator(".why-text")).toHaveText(/You (won|lost) your race|You were elected/);
  await expect(panel.getByTestId("why-chain")).toContainText("Campaign season has begun.");
  await expect(panel.getByTestId("why-chain")).toContainText("The part of every race that nobody controls.");
  // What moved the votes: the result's own factors, as its card shows them (the engine's numbers, not the page's).
  const factors = panel.getByTestId("why-factors");
  await factors.evaluate((el) => el.scrollIntoView({ block: "center" }));
  await expect(factors).toContainText("Why it went this way");
  const onCard = await mine.locator(".diverge").last().locator("li").allInnerTexts();
  expect(onCard.length).toBeGreaterThan(0);
  expect(await factors.locator(".diverge li").allInnerTexts()).toEqual(onCard);
  await expectFits(page, panel);
  await panel.evaluate((el) => (el.scrollTop = 0));
  if (capture) await page.screenshot({ path: `${capture}/why-election-${info.project.name}.png` });
  // The panel is over the night: Escape closes the panel, and the night is still there.
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
  await expect(night).toBeVisible();
  await night.getByRole("button", { name: "Continue" }).last().click();
  await expect(night).toHaveCount(0);
});

// The wide-font cases use a deliberately wide face, so the result does not depend on the machine's fonts.
for (const width of [320, 360, 390] as const) {
  test(`the panel fits a ${width}px phone with wide fonts`, async ({ page }, info) => {
    test.skip(info.project.name !== "phone", "phone widths");
    test.setTimeout(180_000);
    await useWideFont(page);
    await page.setViewportSize({ width, height: 780 });
    await start(page, seedFor("law_decided_early_with_its_chain"));
    const line = await playUntilListed(page, /passed the .* bill|voted down the .* bill/, 6);
    // The row keeps its text and its control side by side, inside the screen.
    const why = line.getByTestId("why-button");
    const box = await why.boundingBox();
    expect((box?.x ?? 0) + (box?.width ?? 0)).toBeLessThanOrEqual(width + 0.5);
    expect(box?.height ?? 0).toBeGreaterThanOrEqual(40);
    await why.click();
    const panel = page.getByTestId("why-panel");
    await expectReadable(panel, 2);
    await expectFits(page, panel);
    // A bottom sheet: it stands on the bar and leaves the top of the screen clear.
    const sheet = await panel.boundingBox();
    expect(sheet?.x ?? 1).toBe(0);
    expect(sheet?.width ?? 0).toBe(width);
    expect(sheet?.y ?? 0).toBeGreaterThan(40);
    if (capture) await page.screenshot({ path: `${capture}/why-law-wide-font-${width}.png` });
    // The scrim behind it closes it.
    await page.mouse.click(width / 2, 20);
    await expect(panel).toHaveCount(0);
  });
}
