import { expect, test } from "./csp";
import type { Page } from "@playwright/test";
import { openPocket } from "./pocket";

// Owner, 2026-11-25: on a phone, a drag at the bottom of the screen or on a short sheet scrolled the whole game like
// a web page. While the game is shown the page never scrolls; sheets and lists scroll inside themselves and stop at
// their ends. (Touch rubber-banding cannot be emulated here; the rules that stop it are asserted instead.)

const pageScroll = (page: Page) =>
  page.evaluate(() => {
    const el = document.scrollingElement as HTMLElement;
    window.scrollTo(0, 10_000);
    el.scrollTop = 10_000;
    return {
      y: window.scrollY + el.scrollTop,
      overflow: getComputedStyle(document.documentElement).overflowY,
      bodyOverflow: getComputedStyle(document.body).overflowY,
      overscroll: getComputedStyle(document.documentElement).overscrollBehaviorY,
    };
  });

test("the game screen never scrolls as a page, with or without a sheet open", async ({ page }) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("4242");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();

  for (const where of ["the street", "the pocket", "a short sheet"]) {
    if (where === "the pocket") await openPocket(page);
    if (where === "a short sheet") {
      await page.keyboard.press("Escape");
      await expect(page.getByTestId("pocket")).toHaveCount(0);
      await page.getByTestId("pocket-button").click();
      await page.getByTestId("pocket").getByTestId("pocket-more").click();
    }
    const s = await pageScroll(page);
    expect(s.y, `the page scrolled with ${where} shown`).toBe(0);
    expect(s.overflow).toBe("hidden");
    expect(s.bodyOverflow).toBe("hidden");
    expect(s.overscroll).toBe("none");
  }
  // A sheet's own scroll stops at its end instead of passing to the page.
  const contain = await page
    .getByTestId("pocket")
    .evaluate((el) => getComputedStyle(el).overscrollBehaviorY);
  expect(contain).toBe("contain");
});

test("the title screen still scrolls as a page", async ({ page }) => {
  await page.goto("./");
  await expect(page.getByRole("heading", { name: "Mandate" })).toBeVisible();
  const overflow = await page.evaluate(() => getComputedStyle(document.documentElement).overflowY);
  expect(overflow).not.toBe("hidden");
});
