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

test("the pocket's agenda starts with no action chosen", async ({ page }) => {
  await start(page, "4242");
  const agenda = (await openPocket(page)).getByTestId("agenda");
  await expect(agenda.getByTestId("agenda-choose")).toBeVisible();
  await expect(agenda.locator(".agenda-kinds .seg-on")).toHaveCount(0);
  await expect(agenda.locator('.agenda-kinds input[type="radio"]:checked')).toHaveCount(0);
  await expect(agenda.getByRole("button", { name: "Go" })).toHaveCount(0);
  await agenda.getByText("Canvass a neighbourhood", { exact: true }).click();
  await expect(agenda.locator(".agenda-kinds .seg-on")).toHaveCount(1);
  await expect(agenda.getByTestId("route")).toContainText("Streets of");
  await expect(agenda.getByTestId("agenda-choose")).toHaveCount(0);
});

/** A real touch drag (through the browser's input pipeline, so scrolling and
 * pointercancel behave as on a phone). */
const touchDrag = async (page: Page, x: number, y0: number, y1: number, steps: number, stepMs: number, holdMs = 0) => {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x, y: y0 }] });
  for (let i = 1; i <= steps; i++) {
    await cdp.send("Input.dispatchTouchEvent", { type: "touchMove", touchPoints: [{ x, y: y0 + ((y1 - y0) * i) / steps }] });
    await page.waitForTimeout(stepMs);
  }
  if (holdMs) await page.waitForTimeout(holdMs);
  await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
  await cdp.detach();
};

test("phone: swiping a sheet down closes it, a short drag springs back", async ({ page }, info) => {
  test.skip(info.project.name !== "phone", "bottom sheets are a phone layout");
  await start(page, "4242");
  let pocket = await openPocket(page);
  await page.waitForTimeout(400);
  const head = await pocket.locator(".sheet-head").boundingBox();
  if (!head) throw new Error("no head");
  const x = head.x + head.width / 3;
  // A short, slow drag: the sheet follows, then springs back.
  await touchDrag(page, x, head.y + 12, head.y + 70, 6, 30, 150);
  await expect(pocket).toBeVisible();
  await expect.poll(() => pocket.evaluate((el) => getComputedStyle(el).transform)).toMatch(/^(none|matrix\(1, 0, 0, 1, 0, 0\))$/);
  // Dragging the head well down closes it.
  await touchDrag(page, x, head.y + 12, head.y + 400, 12, 20);
  await expect(pocket).toHaveCount(0);

  // From the body: scrolled down, a downward drag scrolls the content back up instead.
  pocket = await openPocket(page);
  await page.waitForTimeout(400);
  await pocket.evaluate((el) => (el.scrollTop = 600));
  const box = await pocket.boundingBox();
  if (!box) throw new Error("no sheet");
  await touchDrag(page, x, box.y + 300, box.y + 450, 8, 20);
  await expect(pocket).toBeVisible();
  expect(await pocket.evaluate((el) => el.scrollTop)).toBeLessThan(600);
  // At the top of the body, a quick downward flick (shorter than the close distance) closes it.
  await pocket.evaluate((el) => (el.scrollTop = 0));
  await touchDrag(page, x, box.y + 300, box.y + 435, 3, 0);
  await expect(pocket).toHaveCount(0);
});

test("the player's town is marked on the map and in the town list", async ({ page }) => {
  await start(page, "4242");
  const town = await openPocket(page);
  const home = (await town.locator(".sheet-head p").textContent())?.split(" · ").pop()?.trim() ?? "";
  expect(home.length).toBeGreaterThan(0);
  await town.getByRole("button", { name: "Map", exact: true }).click();
  const canvas = page.getByRole("img", { name: new RegExp(`Drag to pan.*You: ${home}`) });
  await expect(canvas).toBeVisible();
  await expect(page.getByTestId("map-home")).toContainText(`You: ${home}`);
  // The map opens on the player's town: its flag is drawn inside the view.
  await expect(canvas).toHaveAttribute("data-home-at", /^\d+,\d+$/);
  const flag = await canvas.evaluate((el: HTMLCanvasElement) => {
    const [x, y] = (el.dataset.homeAt ?? "0,0").split(",").map(Number);
    const dpr = window.devicePixelRatio || 1;
    const inView = (x ?? -1) >= 0 && (y ?? -1) >= 0 && (x ?? 0) < el.clientWidth && (y ?? 0) < el.clientHeight;
    const px = el.getContext("2d")?.getImageData(Math.round((x ?? 0) * dpr), Math.round((y ?? 0) * dpr), 1, 1).data;
    return { inView, rgb: px ? [px[0], px[1], px[2]] : [] };
  });
  expect(flag.inView).toBe(true);
  expect(flag.rgb).toEqual([235, 104, 52]);
  // The town list flags it too, with text.
  await (await openPocket(page)).getByRole("button", { name: "Overview", exact: true }).click();
  await page.locator(".region-accordion > li > .row-btn", { has: page.locator(".you-chip") }).click();
  await expect(page.getByTestId("town-list-home")).toBeVisible();
  await expect(page.locator(".town-home")).toContainText(home);
});

test("Escape closes the sheet on top", async ({ page }) => {
  await start(page, "4242");
  const pocket = await openPocket(page);
  await pocket.getByRole("button", { name: "Overview", exact: true }).click();
  await page.locator(".region-accordion > li > .row-btn").first().click();
  await page.locator(".town-list .row-btn").first().click();
  await expect(page.getByRole("heading", { name: "Resources" })).toBeVisible();
  await openPocket(page);
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("pocket")).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Resources" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.locator("aside.sheet")).toHaveCount(0);
});
