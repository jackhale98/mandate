import { existsSync, mkdirSync } from "node:fs";
import path from "node:path";
import { expect, test, type Page } from "@playwright/test";

// The walkable town prototype (DEC-080, M6 groundwork), published unlinked at
// <base>spike-hub/ beside the game. It runs the real engine in its worker,
// lays out the player's town and lets you walk it. CI builds it before the
// Playwright run; locally, build it first:
//   npx vite build --config spikes/hub/vite.config.ts --outDir ../../dist/spike-hub --emptyOutDir
// HUB_SCREENS=1 also saves screenshots of the town through the day to
// target/screens/.
const SPIKE = "../spike-hub/";
const built = existsSync(path.join(process.cwd(), "dist", "spike-hub", "index.html"));

interface HubProbe {
  ready: boolean;
  map: { municipality: string; buildings: { index: number; place: number | null; kind: string; home: boolean }[] } | null;
  town: { location: unknown } | null;
  view: { scene: { playerCell: number; player: { walk: unknown } }; renderer: { stats: { frames: number; baseBakes: number } } } | null;
  timings: Record<string, number>;
  setClock(m: number): void;
}

const probe = <T>(page: Page, f: (h: HubProbe) => T) => page.evaluate(f as unknown as (h: unknown) => T, undefined as never);

async function open(page: Page, query: string) {
  const errors: string[] = [];
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(`${SPIKE}?${query}`);
  await page.waitForFunction(() => (window as unknown as { __hub?: { ready: boolean } }).__hub?.ready === true, undefined, { timeout: 90_000 });
  return errors;
}

test.describe("hub town prototype", () => {
  test.skip(!built, "spike-hub is not built (see the comment at the top)");

  test("loads a real town from the engine, and a place button walks the player there", async ({ page }) => {
    const errors = await open(page, "seed=4242&t=720&speed=0&n=6");
    await expect(page.locator("#loading")).toBeHidden();
    await expect(page.locator("#stage canvas")).toBeVisible();
    const town = await page.evaluate(() => (window as unknown as { __hub: HubProbe }).__hub.map?.municipality);
    expect(town).toBeTruthy();
    await expect(page.locator("#town-name")).toHaveText(town as string);
    // Every place is in the list (keyboard and screen-reader route, DEC-051).
    const places = await page.evaluate(() => (window as unknown as { __hub: HubProbe }).__hub.map?.buildings.filter((b) => b.place !== null).length ?? 0);
    expect(places).toBeGreaterThan(8);
    await expect(page.locator("#place-list button[data-testid^='go-place-']")).toHaveCount(places);
    // Labels over the canvas name the places.
    expect(await page.locator(".hub-label").count()).toBeGreaterThan(places / 2);

    // Walk to the council chamber from the list.
    const chamber = await page.evaluate(() => (window as unknown as { __hub: HubProbe }).__hub.map?.buildings.find((b) => b.kind === "council_chamber")?.place);
    expect(chamber).toBeDefined();
    const toggle = page.locator("#places-toggle");
    if ((await toggle.getAttribute("aria-expanded")) === "false") await toggle.click();
    await page.locator(`[data-testid='go-place-${chamber}']`).click();
    await expect(page.locator("#status")).toContainText("Walking to");
    // The walk ends at the door, the engine moves the player, and the place opens.
    await expect(page.locator("#panel")).toBeVisible({ timeout: 30_000 });
    await expect(page.locator("#panel-body")).toContainText("What you can do here");
    await expect(page.locator("#status")).toContainText("You are at");
    const at = await page.evaluate(() => (window as unknown as { __hub: HubProbe }).__hub.town?.location);
    expect(at).toEqual({ type: "place", place: chamber });
    await page.locator("#panel-close").click();
    await expect(page.locator("#panel")).toBeHidden();
    expect(errors).toEqual([]);
  });

  test("tapping a building on the canvas walks to its door", async ({ page }) => {
    const errors = await open(page, "seed=2026&t=600&speed=0&n=0");
    // Look at the town hall, then tap the middle of it.
    const target = await page.evaluate(() => {
      type View = { toScreen(x: number, y: number): { x: number; y: number }; centreOn(x: number, y: number): void };
      const h = (window as unknown as { __hub: { map: { buildings: { kind: string; x: number; y: number; w: number; h: number; front: number }[] }; view: View } }).__hub;
      const b = h.map.buildings.find((x) => x.kind === "town_hall");
      if (!b) return null;
      h.view.centreOn(b.x + b.w / 2, b.y + b.h / 2);
      return { front: b.front, ...h.view.toScreen(b.x + b.w / 2, b.y + b.h / 2) };
    });
    expect(target).not.toBeNull();
    if (!target) return;
    const box = await page.locator("#stage").boundingBox();
    if (!box) throw new Error("no stage");
    await page.mouse.click(box.x + target.x, box.y + target.y);
    await expect(page.locator("#panel")).toBeVisible({ timeout: 30_000 });
    const cell = await page.evaluate(() => (window as unknown as { __hub: HubProbe }).__hub.view?.scene.playerCell);
    expect(cell).toBe(target.front);
    expect(errors).toEqual([]);
  });

  test("the light changes through the day and an idle town stops redrawing", async ({ page }, info) => {
    const errors = await open(page, "seed=4242&t=720&speed=0&n=0");
    const frames = () => page.evaluate(() => (window as unknown as { __hub: HubProbe }).__hub.view?.renderer.stats.frames ?? 0);
    await page.waitForTimeout(800);
    const idle0 = await frames();
    await page.waitForTimeout(1000);
    expect((await frames()) - idle0).toBeLessThanOrEqual(1);
    const dir = path.join(process.cwd(), "..", "target", "screens");
    if (process.env.HUB_SCREENS) mkdirSync(dir, { recursive: true });
    let last = "";
    for (const [name, minutes] of [
      ["day", 12 * 60],
      ["golden-hour", 18 * 60 + 15],
      ["dusk", 19 * 60 + 40],
      ["night", 23 * 60],
    ] as const) {
      await page.evaluate((m) => (window as unknown as { __hub: HubProbe }).__hub.setClock(m), minutes);
      await page.waitForTimeout(250);
      const shot = await page.locator("#stage canvas").screenshot();
      const sig = shot.toString("base64");
      expect(sig).not.toBe(last);
      last = sig;
      if (process.env.HUB_SCREENS) await page.screenshot({ path: path.join(dir, `hub-${info.project.name}-${name}.png`) });
    }
    expect(errors).toEqual([]);
  });
});
