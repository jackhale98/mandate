import { devices, expect, test, type Page } from "@playwright/test";

// The in-game street view zooms through many crisp steps (owner report: only
// about two zoom levels). The street draws at device resolution, so a phone
// at DPR 3 zooms in thirds of a CSS pixel per art pixel, from one device pixel
// per art pixel up to the maximum. A small wheel delta or a small pinch always
// takes one step in its direction. Phone profile at DPR 3 (run once, from the
// phone project).

const DPR = 3;
const MAX_ZOOM = 6;

interface StreetProbe {
  __townStreet?: { view: { camera: { state: { zoom: number } } } } | null;
}

const zoom = (page: Page) => page.evaluate(() => (window as unknown as StreetProbe).__townStreet?.view.camera.state.zoom ?? -1);

/** The zoom once the camera has settled (easing done): two equal reads in a row. */
async function settled(page: Page): Promise<number> {
  let last = await zoom(page);
  for (let i = 0; i < 40; i++) {
    await page.waitForTimeout(60);
    const z = await zoom(page);
    if (Math.abs(z - last) < 1e-6) return z;
    last = z;
  }
  return last;
}

/** Whole steps of 1/DPR. */
const onStep = (z: number) => Math.abs(z * DPR - Math.round(z * DPR)) < 1e-3;

/** A two-finger pinch about a point, through the browser's touch input (fingers `from` → `to` px apart). */
async function pinch(page: Page, cx: number, cy: number, from: number, to: number) {
  const cdp = await page.context().newCDPSession(page);
  const pts = (d: number) => [
    { x: cx - d / 2, y: cy, id: 1 },
    { x: cx + d / 2, y: cy, id: 2 },
  ];
  await cdp.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: pts(from) });
  for (let i = 1; i <= 6; i++) {
    await cdp.send("Input.dispatchTouchEvent", { type: "touchMove", touchPoints: pts(from + ((to - from) * i) / 6) });
    await page.waitForTimeout(16);
  }
  await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
  await cdp.detach();
}

// The Pixel 7 profile with a DPR of 3 (its browser type stays the project's).
const { defaultBrowserType: _browser, ...pixel7 } = devices["Pixel 7"];

test.describe("street zoom on a phone at DPR 3", () => {
  test.use({ ...pixel7, deviceScaleFactor: DPR });

  test("wheel and pinch zoom through many crisp steps, one step per small delta", async ({ page }, info) => {
    test.skip(info.project.name !== "phone", "a phone profile; run once");
    test.setTimeout(120_000);
    await page.goto("./");
    await page.getByPlaceholder("e.g. 20260").fill("4242");
    await page.getByRole("button", { name: "Begin" }).click();
    await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
    const stage = page.locator(".town-street-stage[data-ready='1']");
    await expect(stage).toBeVisible({ timeout: 30_000 });
    // Drawn at device resolution.
    expect(await page.evaluate(() => window.devicePixelRatio)).toBe(DPR);
    const canvas = await page.locator(".town-street-stage canvas").evaluate((c: HTMLCanvasElement) => ({ w: c.width, css: c.clientWidth }));
    expect(canvas.w).toBe(Math.round(canvas.css * DPR));

    const box = await stage.boundingBox();
    if (!box) throw new Error("no street");
    const cx = box.x + box.width / 2;
    // Below the player and their house label (the labels are buttons of their own).
  const cy = box.y + box.height * 0.75;
    await page.mouse.move(cx, cy);
    const levels = new Set<number>();
    let z = await settled(page);
    expect(onStep(z)).toBe(true);
    levels.add(z);

    // Small wheel deltas in: each takes one step up, to the maximum.
    for (let i = 0; i < 40 && z < MAX_ZOOM - 1e-6; i++) {
      await page.mouse.wheel(0, -40);
      const next = await settled(page);
      expect(next, `wheel in from ${z}`).toBeGreaterThan(z + 1e-6);
      expect(onStep(next)).toBe(true);
      z = next;
      levels.add(z);
    }
    expect(z).toBeCloseTo(MAX_ZOOM, 5);
    // Small wheel deltas out: each takes one step down, to one device pixel per art pixel.
    for (let i = 0; i < 40 && z > 1 / DPR + 1e-6; i++) {
      await page.mouse.wheel(0, 40);
      const next = await settled(page);
      expect(next, `wheel out from ${z}`).toBeLessThan(z - 1e-6);
      expect(onStep(next)).toBe(true);
      z = next;
      levels.add(z);
    }
    expect(z).toBeCloseTo(1 / DPR, 5);
    // Many distinct levels between the limits (thirds at DPR 3: 18).
    expect(levels.size).toBeGreaterThanOrEqual(10);

    // A small pinch out takes a step in, a small pinch in a step out.
    await pinch(page, cx, cy, 120, 150);
    const pinchedIn = await settled(page);
    expect(pinchedIn).toBeGreaterThan(z + 1e-6);
    expect(onStep(pinchedIn)).toBe(true);
    await pinch(page, cx, cy, 150, 120);
    expect(await settled(page)).toBeLessThan(pinchedIn - 1e-6);
  });
});
