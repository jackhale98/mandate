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

test("the pocket slides in from the side it is docked on", async ({ page }, info) => {
  await start(page, "4242");
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Animation.enable");
  await cdp.send("Animation.setPlaybackRate", { playbackRate: 0.1 });
  await page.getByTestId("pocket-button").click();
  const pocket = page.getByTestId("pocket");
  await expect(pocket).toBeVisible();
  const name = await pocket.evaluate((el) => getComputedStyle(el).animationName);
  // Mid-slide, the pocket is still partly off its own edge of the screen.
  await page.waitForTimeout(300);
  const box = await pocket.boundingBox();
  if (info.project.name === "desktop") {
    expect(name).toBe("sheet-from-left");
    expect(box?.x ?? 0).toBeLessThan(-10);
  } else {
    expect(name).toBe("sheet-up");
    expect(box?.x ?? 1).toBe(0);
  }
  await cdp.send("Animation.setPlaybackRate", { playbackRate: 1 });
  await pocket.getByRole("button", { name: "Close" }).click();
  if (info.project.name === "desktop") await expect(pocket).toHaveCSS("animation-name", "sheet-to-left");
  await expect(pocket).toHaveCount(0);
});

test("reduced motion: the pocket opens without sliding", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await start(page, "4242");
  await page.getByTestId("pocket-button").click();
  await expect(page.getByTestId("pocket")).toHaveCSS("animation-name", "none");
});
