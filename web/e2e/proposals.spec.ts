import type { Locator, Page } from "@playwright/test";
import { expect, test } from "./csp";
import { startRun, playTo } from "./midrun";
import { showPlaces } from "./pocket";

// The proposal sheet (DEC-092, F6): a run reaches an open proposal; the sheet
// opens from the desk (the town hall's panel) and from the panel of a place
// the proposal is about; an option is chosen and the engine's figures follow;
// the proposal is drafted and then amended, and the sheet says so. On a phone
// the sheet fits 390 and 360 px with no sideways scroll and 44 px targets.
// `PROPOSAL_SHOT=<dir>` saves the 390 x 844 views.
//
// Seed 4 (small world): its first month opens "Close the gap" (public, so
// heard from the start, and known once the player reads the budget papers at
// the town hall) and a mine-safety proposal bound to the mine, as
// `mandate-cli policies --seed 4 --months 2` lists them. A pinned seed:
// if the world moves, pick another from that listing (no seed requirement
// exists for it in tests/seeds.json yet).
const SEED = "4";

/** The panel of a place, opened from the Notebook's place list. */
async function openSpot(page: Page, kind: string): Promise<Locator> {
  const pocket = await showPlaces(page);
  await pocket.getByTestId(`spot-${kind}`).first().click();
  const panel = page.getByTestId("place-panel");
  await expect(panel).toBeVisible();
  return panel;
}

async function closeSheet(page: Page, sheet: Locator) {
  await sheet.getByRole("button", { name: "Close" }).first().click();
  await expect(sheet).toHaveCount(0);
}

/** Every button in the sheet is a 44 px target, and nothing scrolls sideways. */
async function fits(page: Page, sheet: Locator) {
  const problems = await sheet.evaluate((el) => {
    const out: string[] = [];
    if (document.documentElement.scrollWidth > window.innerWidth) out.push(`page is ${document.documentElement.scrollWidth} wide in ${window.innerWidth}`);
    if (el.scrollWidth > el.clientWidth + 1) out.push(`sheet is ${el.scrollWidth} wide in ${el.clientWidth}`);
    for (const b of el.querySelectorAll<HTMLElement>("button, label.pp-opt-label")) {
      const r = b.getBoundingClientRect();
      if (r.width === 0) continue;
      if (r.height < 43.5) out.push(`${b.textContent?.slice(0, 30)} is ${r.height} high`);
      if (r.right > window.innerWidth + 0.5) out.push(`${b.textContent?.slice(0, 30)} ends at ${r.right}`);
    }
    return out;
  });
  expect(problems).toEqual([]);
}

async function chooseAnother(sheet: Locator): Promise<string> {
  const other = sheet.locator('li.pp-opt[data-chosen="false"][data-available="true"]').first();
  const id = (await other.getAttribute("data-testid")) ?? "";
  await other.locator("label").click();
  const chosen = sheet.getByTestId(id);
  await expect(chosen).toHaveAttribute("data-chosen", "true");
  // The package is the engine's answer for it: the chosen option's cost is the package's.
  const cost = (await chosen.getByTestId("option-cost").textContent()) ?? "";
  await expect(sheet.getByTestId("proposal-cost")).toContainText(cost);
  return id;
}

test("a proposal's sheet: from the desk and a place, chosen, drafted and amended", async ({ page }, info) => {
  test.setTimeout(180_000);
  const shots = process.env.PROPOSAL_SHOT;
  await startRun(page, SEED);
  // Proposals open on the first of the month.
  await playTo(page, 6);

  // The desk: at the town hall, where drafting is done (and the budget papers teach it).
  const hall = await openSpot(page, "town_hall");
  if ((await hall.getByTestId("you-are-here-panel").count()) === 0) await hall.getByRole("button", { name: "Go here" }).click();
  await expect(hall.getByTestId("you-are-here-panel")).toBeVisible();
  const desk = hall.getByTestId("desk-proposals");
  await expect(desk).toBeVisible();
  const row = desk.locator('li[data-level]:not([data-level="heard"])').first();
  await expect(row).toBeVisible();
  await row.locator("button.rw-go").click();
  const sheet = page.getByTestId("proposal-sheet");
  await expect(sheet).toBeVisible();
  await expect(sheet.getByTestId("proposal-package")).toBeVisible();
  await expect(sheet.locator("fieldset.pp-axis").first()).toBeVisible();
  await expect(sheet.getByTestId("proposal-sponsors")).toBeVisible();
  await expect(sheet.getByTestId("sponsor-price").first()).toHaveText(/^Their price: You would/);
  expect(await sheet.textContent()).not.toMatch(/⟦|\{\s*\$/);
  if (shots && info.project.name === "phone") {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.screenshot({ path: `${shots}/proposal-sheet-390.png` });
    await sheet.locator("fieldset.pp-axis").first().evaluate((el) => el.scrollIntoView({ block: "start" }));
    await page.screenshot({ path: `${shots}/proposal-choices-390.png` });
    await sheet.getByTestId("proposal-sponsors").evaluate((el) => el.scrollIntoView({ block: "start" }));
    await page.screenshot({ path: `${shots}/proposal-sponsors-390.png` });
  }

  // A choice: the engine answers for the package with it.
  await chooseAnother(sheet);

  // Draft it: a draft at the town hall, and the sheet says so.
  await sheet.getByTestId("proposal-draft").click();
  await expect(page.getByTestId("toast")).toHaveText(/Draft created/);
  await expect(sheet.getByTestId("proposal-result")).toHaveAttribute("data-kind", "drafted");
  const drafted = sheet.getByTestId("proposal-bills").locator("li").first();
  await expect(drafted).toHaveAttribute("data-status", "draft");
  if (shots && info.project.name === "phone") {
    await sheet.getByTestId("proposal-result").scrollIntoViewIfNeeded();
    await page.screenshot({ path: `${shots}/proposal-drafted-390.png` });
  }

  // Amend it: the bill's choices on the sheet, another option, saved.
  await drafted.getByTestId("proposal-bill-amend").click();
  await expect(sheet.getByTestId("proposal-amending")).toBeVisible();
  await chooseAnother(sheet);
  await sheet.getByTestId("proposal-amend").click();
  await expect(page.getByTestId("toast")).toHaveText(/Bill amended/);
  await expect(sheet.getByTestId("proposal-result")).toHaveAttribute("data-kind", "amended");
  if (info.project.name === "phone") {
    for (const [w, h] of [
      [390, 844],
      [360, 640],
    ] as const) {
      await page.setViewportSize({ width: w, height: h });
      await fits(page, sheet);
    }
  }
  await closeSheet(page, sheet);

  // The bill on the desk is named with its proposal's things.
  const bills = hall.getByTestId("bills");
  await expect(bills).not.toContainText("⟦");

  // A place the proposal is about: the mine's panel lists what is on the table there.
  const mine = await openSpot(page, "mine");
  // Seeing it is how the player hears of it.
  if ((await mine.getByTestId("you-are-here-panel").count()) === 0) await mine.getByRole("button", { name: "Go here" }).click();
  await expect(mine.getByTestId("you-are-here-panel")).toBeVisible();
  const here = mine.getByTestId("proposals-here");
  await expect(here).toBeVisible();
  await here.locator("button.rw-go").first().click();
  await expect(sheet).toBeVisible();
  await expect(sheet.getByRole("heading", { level: 2 })).not.toHaveText("Proposal");
  expect(await sheet.textContent()).not.toMatch(/⟦|\{\s*\$/);
  if (shots && info.project.name === "phone") await page.screenshot({ path: `${shots}/proposal-from-place-390.png` });
  await closeSheet(page, sheet);
});
