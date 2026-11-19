import { expect, test, type Page } from "@playwright/test";

// M5c: the director spotlights a dilemma (DEC-077 §4). It is listed in the
// pocket with a badge, opened from there, answered (each option shows its
// trade-offs), and its aftermath explains what happened (DEC-073, DEC-079).
const openPocket = async (page: Page) => {
  if ((await page.getByTestId("pocket").count()) === 0) await page.getByTestId("pocket-button").click();
  await expect(page.getByTestId("pocket")).toBeVisible();
  return page.getByTestId("pocket");
};

test("a dilemma is opened from the pocket, answered, and explained", async ({ page }) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("777");
  await page.getByText("Small", { exact: true }).click();
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  // Play until the director spotlights something (a few turns).
  for (let turn = 1; turn <= 10; turn++) {
    if ((await page.getByTestId("decision-badge").count()) > 0) break;
    await page.getByRole("button", { name: "End turn" }).click();
    await expect(page.getByText(`Turn ${turn}`, { exact: false }).first()).toBeVisible();
  }
  await expect(page.getByTestId("decision-badge")).toBeVisible();
  const pocket = await openPocket(page);
  const list = pocket.getByTestId("pocket-decisions");
  await expect(list).toBeVisible();
  const open = list.getByRole("button", { name: "Choose" }).first();
  await open.evaluate((el) => el.scrollIntoView({ block: "center" }));
  await open.click();
  const sheet = page.getByTestId("decision-sheet");
  await expect(sheet).toBeVisible();
  await expect(sheet.getByTestId("decision-situation")).not.toBeEmpty();
  await expect(sheet.getByTestId("decision-deadline")).toContainText("Decide by");
  // Every option states what it costs before the player commits.
  const options = sheet.getByTestId("decision-options");
  await expect(options.getByText("Costs").first()).toBeVisible();
  await expect(options.getByText("If you don't answer")).toBeVisible();
  await expect(sheet).not.toContainText("⟦");
  const choose = options.locator("button:enabled").first();
  // Clear of the bottom bar on phones.
  await choose.evaluate((el) => el.scrollIntoView({ block: "center" }));
  await choose.click();
  const after = sheet.getByTestId("decision-aftermath");
  await expect(after).toBeVisible();
  await expect(after).toContainText("What happened");
  await expect(after.locator("li").first()).toBeVisible();
  await expect(sheet).not.toContainText("⟦");
  // The story so far records it.
  await sheet.getByRole("button", { name: "Close" }).click();
  await expect(sheet).toHaveCount(0);
  const p2 = await openPocket(page);
  await p2.getByTestId("open-story").click();
  await expect(page.getByTestId("story-sheet")).toContainText("You decided on");
});
