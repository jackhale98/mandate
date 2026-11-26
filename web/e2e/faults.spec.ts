import { expect, test, type Page } from "@playwright/test";
import { openPocket, showPlaces } from "./pocket";

// Faults on the streets (DEC-114, task B2): a neighbourhood's streets list
// what is wrong there and how it stands in the repair queue, with "Why?",
// all in the engine's words. B4 draws the cue on the spot; this checks the
// place view only. MANDATE_CAPTURE=<dir> saves a screenshot for review.

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
};

/** Open each neighbourhood's streets in turn until one lists a fault; returns its section, or null. */
async function streetWithFaults(page: Page) {
  const pocket = await showPlaces(page);
  const n = await pocket.getByTestId("spot-streets").count();
  for (let i = 0; i < n; i++) {
    const p = i === 0 ? pocket : await showPlaces(page);
    await p.getByTestId("spot-streets").nth(i).click();
    const panel = page.getByTestId("place-panel");
    await expect(panel).toBeVisible();
    await expect(panel.locator("#place-title")).toContainText("Streets of");
    const faults = panel.getByTestId("faults-here");
    if ((await faults.count()) > 0) return faults;
    await panel.getByRole("button", { name: "Close" }).first().click();
    await expect(panel).toHaveCount(0);
  }
  return null;
}

test("a neighbourhood's streets show their faults in words, with Why", async ({ page }, info) => {
  test.setTimeout(240_000);
  await start(page, "4");
  let faults = null;
  for (let turn = 0; turn < 12 && !faults; turn++) {
    await endTurn(page);
    if (turn % 3 === 2) {
      faults = await streetWithFaults(page);
      if (!faults) await openPocket(page, "today");
    }
  }
  expect(faults, "a fault on some street within six months").not.toBeNull();
  if (!faults) return;
  await expect(faults).not.toContainText("⟦");
  await expect(faults).toContainText("On these streets");
  const first = faults.locator(".fault").first();
  await expect(first).toHaveAttribute("data-status", /^(open|repairing|fixed)$/);
  await expect(first).toHaveAttribute("data-cue", /.+/);
  await expect(first.getByTestId("fault-status")).toHaveText(/Waiting for repair|Being repaired|Repaired/);
  await first.scrollIntoViewIfNeeded();
  if (capture) await page.getByTestId("place-panel").screenshot({ path: `${capture}/b2-street-faults-${info.project.name}.png` });
  // "Why?" on it walks back to the streets wearing out.
  await first.getByTestId("why-button").click();
  await expect(page.getByTestId("why-panel")).toBeVisible();
  await expect(page.getByTestId("why-panel")).not.toContainText("⟦");
  if (capture) await page.getByTestId("why-panel").screenshot({ path: `${capture}/b2-fault-why-${info.project.name}.png` });
});
