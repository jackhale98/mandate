import { expect, test, type Page } from "@playwright/test";

// DEC-086: the town screen opens on an illustrated town map drawn from the
// versioned layout; the list of places is its text view (Map | List), with
// the same places. Runs on desktop and phone.

/** What the map exposes for tests (`window.__townMap`, web/src/components/TownMap.tsx). */
interface Pt {
  x: number;
  y: number;
}
declare global {
  interface Window {
    __townMap?: {
      view: {
        walking: boolean;
        frames: number;
        fitZoom: number;
        baked: { width: number; height: number };
        camera: { state: { x: number; y: number; zoom: number }; screenToWorld(p: Pt): Pt };
        screenCentre(key: string): Pt | null;
      } | null;
    };
  }
}
const start = async (page: Page, seed: string) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill(seed);
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  // The new run has been saved and the town loaded (the map ignores taps while the game is busy).
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
  // The town is laid out in the town worker (the loader shows meanwhile), then the map is drawn.
  await expect(page.getByTestId("town-map-canvas")).toBeVisible();
};

/** Console errors and uncaught exceptions, checked at the end of each test. */
const watchErrors = (page: Page) => {
  const errors: string[] = [];
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(String(e)));
  return errors;
};

/** Tap a point inside the map wrapper (a finger on phones, the mouse on desktop). */
const tapMap = async (page: Page, x: number, y: number, touch: boolean) => {
  const box = await page.locator(".town-map-wrap").boundingBox();
  if (!box) throw new Error("no map");
  if (touch) await page.touchscreen.tap(box.x + x, box.y + y);
  else await page.mouse.click(box.x + x, box.y + y);
};

test("the town opens on its map, and tapping a landmark goes there and opens it", async ({ page }, info) => {
  const errors = watchErrors(page);
  await start(page, "31337");
  await expect(page.getByTestId("town-tab-map")).toHaveAttribute("aria-selected", "true");
  const canvas = page.getByTestId("town-map-canvas");
  await expect(canvas).toBeVisible();
  await expect(canvas).toHaveAttribute("aria-label", /Map of .+ places to go/);
  // The map fills the width of the town column; the bottom bar stays in view.
  const map = await page.locator(".town-map-wrap").boundingBox();
  const main = await page.locator(".town-main").boundingBox();
  expect(map?.width).toBeGreaterThan((main?.width ?? 0) - 2);
  expect(map?.height).toBeGreaterThan(240);
  await expect(page.locator(".actionbar")).toBeInViewport();
  // The town card stays on the screen with the map.
  await expect(page.getByTestId("town-card")).toBeVisible();
  // Home and "you are here" are marked with words, not colour alone.
  await expect(page.getByTestId("map-home")).toHaveAttribute("aria-current", "location");
  await expect(page.getByTestId("map-home")).toHaveAttribute("aria-label", /Home.*You are here/);

  // Tap the middle of the town hall's building on the canvas.
  const hall = page.getByTestId("map-town_hall");
  const key = await hall.getAttribute("data-hotspot");
  const at = await page.evaluate((k) => window.__townMap?.view?.screenCentre(k as string) ?? null, key);
  expect(at).not.toBeNull();
  await tapMap(page, at?.x ?? 0, at?.y ?? 0, info.project.name === "phone");
  // The player is shown walking there along the streets…
  await expect.poll(() => page.evaluate(() => window.__townMap?.view?.walking ?? false), { timeout: 2000 }).toBe(true);
  // …and the town hall's panel opens, with the player there.
  const panel = page.getByTestId("place-panel");
  await expect(panel.getByRole("heading", { name: /Town hall/ })).toBeVisible();
  await expect(panel.getByTestId("you-are-here-panel")).toBeVisible();
  await expect(page.getByTestId("where")).toContainText("Town hall");
  await expect(hall).toHaveAttribute("aria-current", "location");
  await expect.poll(() => page.evaluate(() => window.__townMap?.view?.walking ?? true)).toBe(false);
  // Escape closes the panel, as elsewhere.
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
  expect(errors).toEqual([]);
});

test("the List view has the same places as the map, and the choice survives a reload", async ({ page }) => {
  const errors = watchErrors(page);
  await start(page, "2718");
  const hotspots = page.locator("[data-hotspot]");
  const mapNames = await hotspots.evaluateAll((els) => els.map((e) => ({ name: (e as HTMLElement).dataset.name ?? "", quiet: e.classList.contains("town-map-hit-quiet") })));
  expect(mapNames.length).toBeGreaterThan(5);

  await page.getByTestId("town-tab-list").click();
  await expect(page.getByTestId("town-tab-list")).toHaveAttribute("aria-selected", "true");
  await expect(page.getByTestId("town-map")).toHaveCount(0);
  const listNames = await page.locator(".spot .spot-name").allTextContents();
  // Every place you can go to on the map is a button in the list, and the other way round.
  expect(mapNames.filter((m) => !m.quiet).map((m) => m.name).sort()).toEqual([...listNames].sort());
  // Places with nothing to do yet are named in the list's "Also here".
  const quiet = (await page.locator(".hood-quiet").allTextContents()).join(" ");
  for (const m of mapNames.filter((x) => x.quiet)) expect(quiet).toContain(m.name);

  // The choice is remembered on this device.
  await page.reload();
  await page.getByRole("button", { name: "Continue" }).first().click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByTestId("town-tab-list")).toHaveAttribute("aria-selected", "true");
  await expect(page.getByTestId("spot-town_hall")).toBeVisible();
  await page.getByTestId("town-tab-map").click();
  await page.reload();
  await page.getByRole("button", { name: "Continue" }).first().click();
  await expect(page.getByTestId("town-map-canvas")).toBeVisible();
  expect(errors).toEqual([]);
});

test("the keyboard moves between places on the map and Enter goes there", async ({ page }) => {
  const errors = watchErrors(page);
  await start(page, "4040");
  await page.getByTestId("town-tab-map").focus();
  // Tab enters the map at its first place (the list's order); arrows move between places.
  await page.keyboard.press("Tab");
  const focused = () => page.evaluate(() => (document.activeElement as HTMLElement | null)?.dataset.hotspot ?? null);
  const first = await focused();
  expect(first).not.toBeNull();
  const seen = new Set([first]);
  for (const k of ["ArrowRight", "ArrowDown", "ArrowLeft", "ArrowUp", "ArrowRight"]) {
    await page.keyboard.press(k);
    seen.add(await focused());
  }
  expect(seen.size).toBeGreaterThan(2);
  // Enter goes to the focused place (a place with something to do).
  const active = page.locator("[data-hotspot]:not(.town-map-hit-quiet)").filter({ hasNot: page.locator("[aria-current]") });
  const target = active.nth(1);
  const name = (await target.getAttribute("data-name")) ?? "";
  await target.focus();
  await page.keyboard.press("Enter");
  const panel = page.getByTestId("place-panel");
  await expect(panel.locator("#place-title")).toContainText(name);
  await expect(page.getByTestId("where")).toContainText(name);
  // Zoom keys work without moving the focus off the map.
  const z0 = await page.evaluate(() => window.__townMap?.view?.camera.state.zoom ?? 0);
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
  await page.locator("[data-hotspot]").first().focus();
  await page.keyboard.press("+");
  await expect.poll(() => page.evaluate(() => window.__townMap?.view?.camera.state.zoom ?? 0)).toBeGreaterThan(z0 * 1.2);
  expect(errors).toEqual([]);
});

test("pinch and wheel zoom about the fingers, and zooming out frames the town", async ({ page }, info) => {
  test.skip(info.project.name === "phone", "the wheel is a desktop gesture; pinches are covered by the camera's unit tests");
  await start(page, "31337");
  const box = await page.locator(".town-map-wrap").boundingBox();
  if (!box) throw new Error("no map");
  const fit = await page.evaluate(() => window.__townMap?.view?.fitZoom ?? 0);
  // Zoom in about a point: the town point under the cursor stays under it.
  const p = { x: box.width * 0.3, y: box.height * 0.4 };
  const before = await page.evaluate((q) => window.__townMap?.view?.camera.screenToWorld(q), p);
  await page.mouse.move(box.x + p.x, box.y + p.y);
  await page.mouse.wheel(0, -400);
  await expect.poll(() => page.evaluate(() => window.__townMap?.view?.camera.state.zoom ?? 0)).toBeGreaterThan(fit * 1.5);
  const after = await page.evaluate((q) => window.__townMap?.view?.camera.screenToWorld(q), p);
  expect(Math.abs((after?.x ?? 0) - (before?.x ?? 0))).toBeLessThan(0.5);
  expect(Math.abs((after?.y ?? 0) - (before?.y ?? 0))).toBeLessThan(0.5);
  // Zooming all the way out frames the whole town again, centred.
  for (let i = 0; i < 6; i++) await page.mouse.wheel(0, 600);
  await expect.poll(() => page.evaluate(() => window.__townMap?.view?.camera.state.zoom ?? 0)).toBeLessThan(fit * 1.01);
  const centre = await page.evaluate(() => {
    const v = window.__townMap?.view;
    return v ? { x: v.camera.state.x, y: v.camera.state.y, w: v.baked.width, h: v.baked.height } : null;
  });
  await expect.poll(async () => Math.abs((await page.evaluate(() => window.__townMap?.view?.camera.state.x ?? 0)) - (centre?.w ?? 0) / 2)).toBeLessThan(0.5);
  // An idle map draws nothing (once the camera has settled).
  await page.waitForTimeout(1500);
  const frames = await page.evaluate(() => window.__townMap?.view?.frames ?? 0);
  await page.waitForTimeout(500);
  expect(await page.evaluate(() => window.__townMap?.view?.frames ?? 0)).toBe(frames);
});
