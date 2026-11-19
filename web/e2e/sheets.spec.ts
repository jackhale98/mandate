import { expect, test, type Locator, type Page } from "@playwright/test";

// UI polish (owner, 2026-11-19): sheet heads, the pocket's slide, swipe to
// close, the player's town on the map, and the pocket's agenda picker.
const start = async (page: Page, seed: string) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill(seed);
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
};

const openPocket = async (page: Page) => {
  if ((await page.getByTestId("pocket").count()) === 0) await page.getByTestId("pocket-button").click();
  await expect(page.getByTestId("pocket")).toBeVisible();
  return page.getByTestId("pocket");
};

/** After scrolling, the sticky head sits flush with the sheet's top edge and
 * nothing of the sheet's content shows above it. */
const expectHeadFlush = async (sheet: Locator) => {
  await sheet.evaluate((el) => (el.scrollTop = 400));
  await expect
    .poll(() =>
      sheet.evaluate((el) => {
        const head = el.querySelector(":scope > .sheet-head");
        if (!head || el.scrollTop === 0) return "not scrolled";
        const gap = head.getBoundingClientRect().top - (el.getBoundingClientRect().top + el.clientTop);
        const hud = document.querySelector(".hud")?.getBoundingClientRect().bottom ?? 0;
        return Math.abs(gap) <= 1 && el.getBoundingClientRect().top >= hud - 1 ? "flush" : `gap ${gap}, sheet top ${el.getBoundingClientRect().top}, hud ${hud}`;
      }),
    )
    .toBe("flush");
};

test("sheet heads stay flush with the top of the sheet when scrolled", async ({ page }) => {
  await start(page, "4242");
  const pocket = await openPocket(page);
  await page.waitForTimeout(400);
  await expectHeadFlush(pocket);
  await pocket.getByRole("button", { name: "Overview", exact: true }).click();
  await page.locator(".region-accordion > li > .row-btn").first().click();
  await page.locator(".town-list .row-btn").first().click();
  await expect(page.getByRole("heading", { name: "Resources" })).toBeVisible();
  await page.waitForTimeout(400);
  await expectHeadFlush(page.locator("aside.sheet").last());
});
