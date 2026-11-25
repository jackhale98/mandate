import { expect, test, type Page } from "@playwright/test";
import { startRun, playTo } from "./midrun";
import { openPeople, openPocket, openRow } from "./pocket";

// Screenshots of the reworked pocket for review (DEC-113): Today, People, a
// person opened and an arc's sheet at 393 px wide (2x), light and dark, and
// the desktop layout at 1280 px. Not a check: runs only with
// POCKET_SHOTS=<dir>, on the desktop project (it sets its own sizes).

const dir = process.env.POCKET_SHOTS;
test.skip(!dir, "screenshots only with POCKET_SHOTS=<dir>");
test.use({ deviceScaleFactor: 2 });

async function settle(page: Page) {
  await page.evaluate(async () => {
    await document.fonts.ready;
    await Promise.all(document.getAnimations().filter((a) => a.effect?.getComputedTiming().iterations !== Infinity).map((a) => a.finished.catch(() => undefined)));
  });
}

test("the pocket at 393 px, light and dark, and at 1280 px", async ({ page }, info) => {
  test.skip(info.project.name !== "desktop", "one run makes every size");
  test.setTimeout(240_000);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 393, height: 793 });
  await startRun(page, "4242");
  await playTo(page, 10);
  for (const scheme of ["dark", "light"] as const) {
    await page.emulateMedia({ colorScheme: scheme, reducedMotion: "reduce" });
    await page.setViewportSize({ width: 393, height: 793 });
    await openPocket(page, "today");
    await settle(page);
    await page.screenshot({ path: `${dir}/today-${scheme}.png` });
    let pocket = await openPocket(page, "people");
    await settle(page);
    await page.screenshot({ path: `${dir}/people-${scheme}.png` });
    await openRow(pocket.getByTestId("notebook-people-now").getByTestId("notebook-person").first());
    await settle(page);
    await page.screenshot({ path: `${dir}/person-${scheme}.png` });
    await openPocket(page, "places");
    await settle(page);
    await page.screenshot({ path: `${dir}/places-${scheme}.png` });
    pocket = await openPocket(page, "today");
    await pocket.locator("[data-testid^='arc-row-'] .rw-main").first().click();
    await expect(page.getByTestId("arc-sheet")).toBeVisible();
    await settle(page);
    await page.screenshot({ path: `${dir}/arc-${scheme}.png` });
    await page.getByTestId("arc-sheet").getByRole("button", { name: "Close" }).click();
    await page.getByTestId("pocket").getByRole("button", { name: "Close" }).first().click();
  }
  await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
  await page.setViewportSize({ width: 1280, height: 800 });
  await openPeople(page);
  await settle(page);
  await page.screenshot({ path: `${dir}/desktop-1280.png` });
});
