import { expect, test, type Page } from "@playwright/test";
import { startRun } from "./midrun";
import { openPocket } from "./pocket";

// The town calendar (B1, DEC-114): one date per turn, and going to an event
// from Today. The bar shows the day played with its weekday; Today's "This
// fortnight" lists the turn's events with their part and place; "Go" goes
// there (the place opens), the place is full of the people who came, and
// the row then says the player was there. CAL_SHOTS=<dir> also saves the
// screenshots for review (393 px, 2x).
const SEED = "4242";
const shots = process.env.CAL_SHOTS;

async function settle(page: Page) {
  await page.evaluate(async () => {
    await document.fonts.ready;
    await Promise.all(document.getAnimations().filter((a) => a.effect?.getComputedTiming().iterations !== Infinity).map((a) => a.finished.catch(() => undefined)));
  });
}

test.use({ deviceScaleFactor: 2 });

test("go to the fortnight's event from Today, and find it full", async ({ page }, info) => {
  test.skip(info.project.name !== "desktop", "one run, at phone width");
  test.setTimeout(120_000);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.setViewportSize({ width: 393, height: 793 });
  await startRun(page, SEED);

  // The bar names the day played, with its weekday.
  await expect(page.getByTestId("bar-day")).toHaveText(/^(Monday|Tuesday|Wednesday|Thursday|Friday|Saturday|Sunday) \d+ \w+ \d+$/);

  // Today: this fortnight's events, each with its part and place.
  const pocket = await openPocket(page, "today");
  const section = pocket.getByTestId("pocket-fortnight");
  await expect(section).toBeVisible();
  const row = section.locator("[data-cal='held'], [data-cal='expected']").first();
  await expect(row).toBeVisible();
  const id = ((await row.getAttribute("data-testid")) ?? "").replace(/^cal-/, "");
  await expect(row).toContainText(/(in the morning|in the afternoon|in the evening)/);
  if (shots) {
    await settle(page);
    await page.screenshot({ path: `${shots}/today-event.png` });
  }

  // Go: the place opens, with the people who came.
  await row.getByTestId(`cal-go-${id}`).click();
  const panel = page.getByTestId("place-panel");
  await expect(panel).toBeVisible({ timeout: 20_000 });
  await expect(panel.getByTestId("you-are-here-panel")).toBeVisible();
  if (shots) {
    await settle(page);
    await page.screenshot({ path: `${shots}/event-place-full.png` });
  }
  await panel.getByRole("button", { name: "Close" }).first().click();

  // Today says the player was there.
  const again = await openPocket(page, "today");
  await expect(again.getByTestId(`cal-${id}`)).toContainText("You were there");
});
