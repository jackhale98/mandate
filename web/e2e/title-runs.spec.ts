import { expect, test } from "./csp";
import type { Page } from "@playwright/test";
import { pocketMenu } from "./pocket";

// The title screen lists saved games first, each with the player's name, post and home (the pocket
// header's words), and the new career below. A run imported from a file (which carries no names)
// gains them, read from the save, without being continued.

const noShare = () => {
  Object.defineProperty(Navigator.prototype, "canShare", { value: undefined, configurable: true });
};

/** The pocket's line for the player, at the top of its ⋯ menu: "Name · Post · Home". */
async function pocketWho(page: Page): Promise<string> {
  const menu = await pocketMenu(page);
  return ((await menu.locator("p.muted").first().textContent()) ?? "").trim();
}

test("after a saved game the title screen lists it first, with the player's name, post and home; an imported run gains them", async ({ page, browser, baseURL, viewport, isMobile, hasTouch, userAgent, deviceScaleFactor }, testInfo) => {
  test.setTimeout(150_000);
  await page.addInitScript(noShare);
  await page.goto("./");
  await expect(page.getByRole("heading", { name: "New career" })).toBeVisible();
  // No saves: the new-career form is all there is (Begin is the primary action).
  await expect(page.getByRole("heading", { name: "Your runs" })).toBeVisible();
  await expect(page.getByTestId("run-row")).toHaveCount(0);
  await page.getByPlaceholder("e.g. 20260").fill("5151");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await page.getByRole("button", { name: "End turn" }).click();
  await expect(page.getByText("Turn 1")).toBeVisible();
  const who = await pocketWho(page);
  expect(who.split(" · ")).toHaveLength(3);
  await (await pocketMenu(page)).getByRole("button", { name: "Save & quit" }).click();

  // Saved runs first, the new career below; Continue is the primary action, Begin is not.
  const rows = page.getByTestId("run-row");
  await expect(rows).toHaveCount(1);
  const runsHead = await page.getByRole("heading", { name: "Your runs" }).boundingBox();
  const newHead = await page.getByRole("heading", { name: "New career" }).boundingBox();
  expect((runsHead?.y ?? 1e9) < (newHead?.y ?? 0)).toBe(true);
  await expect(rows.first().getByTestId("run-who")).toHaveText(who);
  await expect(rows.first().getByRole("button", { name: "Continue" })).toHaveClass(/btn-primary/);
  await expect(page.getByRole("button", { name: "Begin" })).not.toHaveClass(/btn-primary/);

  // A run from a file has no names in the index: it gains them from the save itself.
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: /^Export / }).click();
  const file = await download;
  const path = testInfo.outputPath(file.suggestedFilename());
  await file.saveAs(path);
  const other = await browser.newContext({ ...(baseURL ? { baseURL } : {}), viewport, isMobile, hasTouch, ...(userAgent ? { userAgent } : {}), deviceScaleFactor });
  try {
    const fresh = await other.newPage();
    await fresh.goto("./");
    await expect(fresh.getByText("No runs yet.")).toBeVisible();
    const chooser = fresh.waitForEvent("filechooser");
    await fresh.getByRole("button", { name: "Import a run" }).click();
    await (await chooser).setFiles(path);
    await expect(fresh.getByTestId("transfer-status")).toContainText("Run imported");
    await expect(fresh.getByTestId("run-row")).toHaveCount(1);
    await expect(fresh.getByTestId("run-who")).toHaveText(who, { timeout: 30_000 });
    // Saved runs now come first there too.
    const a = await fresh.getByRole("heading", { name: "Your runs" }).boundingBox();
    const b = await fresh.getByRole("heading", { name: "New career" }).boundingBox();
    expect((a?.y ?? 1e9) < (b?.y ?? 0)).toBe(true);
  } finally {
    await other.close();
  }
});
