import { existsSync } from "node:fs";
import path from "node:path";
import { expect, test, type Page } from "@playwright/test";

// The pixel-art tool's spike (DEC-084, P0) in the town prototype: ?art=pixelkit
// swaps the code-drawn placeholder for the tool's pack through the same
// ArtProvider, ?night=multiply|moods picks the night look (D12), and
// ?preset=street centres on a representative street. Build the spike first
// (see hub-town.spec.ts).
const SPIKE = "../spike-hub/";
const built = existsSync(path.join(process.cwd(), "dist", "spike-hub", "index.html"));

interface Probe {
  ready: boolean;
  art: { name: string; night: string | null };
  view: { renderer: { stats: { frames: number; baseBakes: number } } } | null;
  setClock(m: number): void;
  showStreet(): { x: number; y: number; w: number; h: number } | null;
}
const probe = (page: Page) => page.evaluate(() => (window as unknown as { __hub: Probe }).__hub.art);

async function open(page: Page, query: string) {
  const errors: string[] = [];
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(`${SPIKE}?${query}`);
  await page.waitForFunction(() => (window as unknown as { __hub?: { ready: boolean } }).__hub?.ready === true, undefined, { timeout: 90_000 });
  await page.waitForTimeout(400);
  return errors;
}

test.describe("pixel-art spike toggle", () => {
  test.skip(!built, "spike-hub is not built (see hub-town.spec.ts)");

  test("the placeholder stays the default; art=pixelkit draws the tool's pack on the street preset", async ({ page }) => {
    const errors = await open(page, "seed=4242&t=720&speed=0&n=8&preset=street");
    expect(await probe(page)).toEqual({ name: "placeholder", night: null });
    const placeholder = await page.locator("#stage canvas").screenshot();

    await open(page, "seed=4242&t=720&speed=0&n=8&preset=street&art=pixelkit");
    expect(await probe(page)).toEqual({ name: "pixelkit", night: "multiply" });
    const street = await page.evaluate(() => (window as unknown as { __hub: Probe }).__hub.showStreet());
    expect(street?.w).toBeGreaterThan(0);
    const pixelkit = await page.locator("#stage canvas").screenshot();
    expect(pixelkit.equals(placeholder)).toBe(false);
    expect(errors).toEqual([]);
  });

  test("both night looks draw through the day; palette moods re-bake the static layers", async ({ page }) => {
    for (const night of ["multiply", "moods"] as const) {
      const errors = await open(page, `seed=2026&t=720&speed=0&n=6&preset=street&art=pixelkit&night=${night}`);
      expect((await probe(page)).night).toBe(night);
      const bakes = () => page.evaluate(() => (window as unknown as { __hub: Probe }).__hub.view?.renderer.stats.baseBakes ?? 0);
      const before = await bakes();
      let last = "";
      for (const minutes of [12 * 60, 19 * 60 + 30, 23 * 60]) {
        await page.evaluate((m) => (window as unknown as { __hub: Probe }).__hub.setClock(m), minutes);
        await page.waitForTimeout(400);
        const shot = (await page.locator("#stage canvas").screenshot()).toString("base64");
        expect(shot).not.toBe(last);
        last = shot;
      }
      // Moods repaint the ground and buildings, so the chunks are baked again; multiply never re-bakes them.
      if (night === "moods") expect(await bakes()).toBeGreaterThan(before);
      else expect(await bakes()).toBe(before);
      expect(errors).toEqual([]);
    }
  });
});
