import { expect, test, type Page } from "@playwright/test";
import { openPocket, pocketMenu, showPlaces } from "./pocket";

// DEC-086, DEC-088 main screen: the town map is a full-screen overlay over the
// street, drawn from the versioned layout; tapping a place there closes the
// map and walks the player there in the street. The pocket's Places tab is its
// text view, with the same places. Runs on desktop and phone.

/** What the map and the street expose for tests (web/src/components: TownMap, TownStreet). */
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
    __townStreet?: { view: { scene: { playerMoving: boolean } } } | null;
  }
}
const start = async (page: Page, seed: string) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill(seed);
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  // The new run has been saved and the town loaded (taps are ignored while the game is busy).
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
  // The town is laid out in the town worker (the loader shows meanwhile), then the street is drawn.
  await expect(page.locator(".town-street-stage[data-ready='1']")).toBeVisible({ timeout: 30_000 });
};

/** The bar's Map button opens the town map over the street. */
const openMap = async (page: Page) => {
  await page.getByTestId("town-map-button").click();
  await expect(page.getByTestId("town-map-overlay")).toBeVisible();
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

test("the Map button opens the town map full screen; tapping a landmark closes it and walks there", async ({ page }, info) => {
  test.setTimeout(120_000);
  const errors = watchErrors(page);
  await start(page, "31337");
  await openMap(page);
  const canvas = page.getByTestId("town-map-canvas");
  await expect(canvas).toHaveAttribute("aria-label", /Map of .+ places to go.+Places tab/);
  // A modal dialog over the whole screen; the keyboard starts on its close button.
  await expect(page.getByRole("dialog", { name: /tap a place/ })).toBeVisible();
  await expect(page.getByTestId("town-map-close")).toBeFocused();
  const map = await page.locator(".town-map-wrap").boundingBox();
  const vp = page.viewportSize();
  expect(map?.width).toBeGreaterThan((vp?.width ?? 0) - 2);
  expect(map?.height).toBeGreaterThan((vp?.height ?? 0) * 0.6);
  // Home and "you are here" are marked with words, not colour alone.
  await expect(page.getByTestId("map-home")).toHaveAttribute("aria-current", "location");
  await expect(page.getByTestId("map-home")).toHaveAttribute("aria-label", /Home.*You are here/);

  // Tap the middle of the town hall's building on the canvas.
  const hall = page.getByTestId("map-town_hall");
  const key = await hall.getAttribute("data-hotspot");
  const at = await page.evaluate((k) => window.__townMap?.view?.screenCentre(k as string) ?? null, key);
  expect(at).not.toBeNull();
  await tapMap(page, at?.x ?? 0, at?.y ?? 0, info.project.name === "phone");
  // The map closes and the player walks there along the streets…
  await expect(page.getByTestId("town-map-overlay")).toHaveCount(0);
  // With a mouse, focus goes back to the Map button (a finger's tap lands on the street below; the keyboard test checks the rest).
  if (info.project.name === "desktop") await expect(page.getByTestId("town-map-button")).toBeFocused();
  await expect.poll(() => page.evaluate(() => window.__townStreet?.view.scene.playerMoving ?? false), { timeout: 5000 }).toBe(true);
  // …and arrives inside the town hall (it has an interior: note 6), not at its menu; the player is there.
  await expect(page.locator("[data-testid='interior'][data-ready='1']")).toBeVisible({ timeout: 60_000 });
  await expect(page.getByTestId("place-panel")).toHaveCount(0);
  await expect(page.getByTestId("where")).toContainText("Town hall");
  // Out by the door; on the map the town hall is now where you are.
  await page.getByTestId("interior-leave").click();
  await expect(page.getByTestId("interior")).toHaveCount(0, { timeout: 15_000 });
  await openMap(page);
  await expect(page.getByTestId("map-town_hall")).toHaveAttribute("aria-current", "location");
  // Escape closes the map too.
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("town-map-overlay")).toHaveCount(0);
  expect(errors).toEqual([]);
});

test("the Places tab has the same places as the map, and the last tab survives a reload", async ({ page }) => {
  const errors = watchErrors(page);
  await start(page, "2718");
  await openMap(page);
  const hotspots = page.locator("[data-hotspot]");
  const mapNames = await hotspots.evaluateAll((els) => els.map((e) => ({ name: (e as HTMLElement).dataset.name ?? "", quiet: e.classList.contains("town-map-hit-quiet") })));
  expect(mapNames.length).toBeGreaterThan(5);
  await page.getByTestId("town-map-close").click();
  await expect(page.getByTestId("town-map-overlay")).toHaveCount(0);

  const pocket = await showPlaces(page);
  const listNames = await pocket.locator(".spot .spot-name").allTextContents();
  // Every place you can go to on the map is a button in the Places tab, and the other way round.
  expect(mapNames.filter((m) => !m.quiet).map((m) => m.name).sort()).toEqual([...listNames].sort());
  // Places with nothing to do yet are named under "Also here".
  const quiet = (await pocket.locator(".hood-quiet").allTextContents()).join(" ");
  for (const m of mapNames.filter((x) => x.quiet)) expect(quiet).toContain(m.name);

  // The tab is remembered on this device.
  await page.reload();
  await page.getByRole("button", { name: "Continue" }).first().click();
  await expect(page.getByTestId("town")).toBeVisible();
  await page.getByTestId("pocket-button").click();
  await expect(page.getByTestId("pocket-tab-notebook")).toHaveAttribute("data-on", "true");
  await expect(page.getByTestId("notebook-tab-places")).toHaveAttribute("aria-selected", "true");
  // The neighbourhoods opened this session stay open.
  await expect(page.getByTestId("spot-town_hall")).toBeVisible();
  await openPocket(page, "today");
  await page.reload();
  await page.getByRole("button", { name: "Continue" }).first().click();
  await page.getByTestId("pocket-button").click();
  await expect(page.getByTestId("pocket-panel-today")).toBeVisible();
  expect(errors).toEqual([]);
});

test("the keyboard moves between places on the map and Enter walks there", async ({ page }) => {
  test.setTimeout(120_000);
  const errors = watchErrors(page);
  await start(page, "4040");
  await openMap(page);
  // From the close button, Tab reaches the layer picker above the map, then enters the map at its first place (the list's order); arrows move between places.
  await page.keyboard.press("Tab");
  await expect(page.getByTestId("town-map-layer")).toBeFocused();
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
  // Zoom keys work without moving the focus off the map.
  const z0 = await page.evaluate(() => window.__townMap?.view?.camera.state.zoom ?? 0);
  await page.keyboard.press("+");
  await expect.poll(() => page.evaluate(() => window.__townMap?.view?.camera.state.zoom ?? 0)).toBeGreaterThan(z0 * 1.2);
  expect(await focused()).not.toBeNull();
  // Enter goes to the focused place (a place with something to do): the map closes, the player walks, the panel opens.
  // A building, not a street (walking to a street opens nothing); which one
  // depends on the world, so the test takes the first that is not the player's.
  const active = page.locator("[data-hotspot]:not(.town-map-hit-quiet):not([data-hotspot^='streets:'])").filter({ hasNot: page.locator("[aria-current]") });
  const target = active.first();
  const name = (await target.getAttribute("data-name")) ?? "";
  await target.focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("town-map-overlay")).toHaveCount(0);
  await expect(page.getByTestId("town-map-button")).toBeFocused();
  // A place with an interior is entered; one without opens its panel.
  await expect(page.getByTestId("place-panel").locator("#place-title").or(page.locator("[data-testid='interior'][data-ready='1']"))).toBeVisible({ timeout: 60_000 });
  await expect(page.getByTestId("where")).toContainText(name);
  expect(errors).toEqual([]);
});

test("pinch and wheel zoom about the fingers, and zooming out frames the town", async ({ page }, info) => {
  test.skip(info.project.name === "phone", "the wheel is a desktop gesture; pinches are covered by the camera's unit tests");
  await start(page, "31337");
  await openMap(page);
  const box = await page.locator(".town-map-wrap").boundingBox();
  if (!box) throw new Error("no map");
  const fit = await page.evaluate(() => window.__townMap?.view?.fitZoom ?? 0);
  // Zoom in about a point: the town point under the cursor stays under it (a point far enough in that the
  // zoomed view stays inside the town; at its edge the camera eases back, which moves the point).
  const p = { x: box.width * 0.4, y: box.height * 0.4 };
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

test("the map overlay: Town | Country at its top; the country map, its layers and a town's panel; the choice survives a reload", async ({ page }) => {
  test.setTimeout(90_000);
  const errors = watchErrors(page);
  await start(page, "4242");
  await openMap(page);
  const scopes = page.getByRole("tablist", { name: "Which map" });
  const town = scopes.getByRole("tab", { name: "Town" });
  const country = scopes.getByRole("tab", { name: "Country" });
  await expect(town).toHaveAttribute("aria-selected", "true");
  await expect(page.getByRole("tabpanel")).toHaveAttribute("aria-labelledby", "map-scope-town");

  // The keyboard reaches the toggle; an arrow switches to the country.
  await town.focus();
  await page.keyboard.press("ArrowRight");
  await expect(country).toBeFocused();
  await expect(country).toHaveAttribute("aria-selected", "true");
  const canvas = page.getByTestId("town-map-overlay").getByRole("img", { name: /Drag to pan.*You: / });
  await expect(canvas).toBeVisible();
  await expect(page.getByTestId("town-map-canvas")).toHaveCount(0);
  expect((await canvas.boundingBox())?.width ?? 0).toBeGreaterThan(200);
  // The layer picker works.
  const layer = page.getByTestId("map-layer");
  await layer.selectOption("unemployment");
  await expect(layer).toHaveValue("unemployment");
  await expect(page.getByTestId("country-map").locator(".legend")).toBeVisible();

  // "You: <town>" selects the player's town and opens its panel, as a tap on it does; Escape closes the panel first, not the map.
  await page.getByTestId("country-map").getByTestId("map-home").click();
  const panel = page.getByTestId("town-map-overlay").locator("aside.sheet");
  await expect(panel.getByRole("heading", { name: "Resources" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
  await expect(page.getByTestId("town-map-overlay")).toBeVisible();

  // Back to the town map.
  await town.click();
  await expect(page.getByTestId("town-map-canvas")).toBeVisible();
  await country.click();
  await expect(canvas).toBeVisible();
  // ✕ closes it from the country view, and the keyboard goes back to the bar's Map button.
  await page.getByTestId("town-map-close").click();
  await expect(page.getByTestId("town-map-overlay")).toHaveCount(0);
  await expect(page.getByTestId("town-map-button")).toBeFocused();

  // Remembered on this device: after a reload the map opens on the country.
  await page.reload();
  await page.getByRole("button", { name: "Continue" }).first().click();
  await expect(page.locator(".town-street-stage[data-ready='1']")).toBeVisible({ timeout: 30_000 });
  await page.getByTestId("town-map-button").click();
  await expect(page.getByTestId("map-scope-country")).toHaveAttribute("aria-selected", "true");
  await expect(page.getByTestId("country-map")).toBeVisible();
  // Escape closes it from here too.
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("town-map-overlay")).toHaveCount(0);
  // The pocket's Map opens the overlay on the country as well.
  await page.getByTestId("town-map-button").click();
  await page.getByTestId("map-scope-town").click();
  await page.getByTestId("town-map-close").click();
  await (await pocketMenu(page)).getByTestId("pocket-country-map").click();
  await expect(page.getByTestId("map-scope-country")).toHaveAttribute("aria-selected", "true");
  expect(errors).toEqual([]);
});
