import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { expect, test } from "@playwright/test";

// Measurements, not checks (run only with TOWN_TIMINGS=<dir>): the game's
// town path for the 13 committed engine towns (web/src/hub/fixtures/
// towns-v2.json, the two largest capitals among them): planned cold in the
// town worker, warm from memory and from IndexedDB (a reload), and the
// main-thread work after it (dress, the map's scene and bake). The phone
// runs with its CPU slowed 4×. Writes <dir>/town-timings-<project>.json.
const OUT = process.env.TOWN_TIMINGS ?? "";

interface Fixture {
  seed: string;
  source: string;
  input: { municipality: string };
  geo: unknown;
}

test("town timings", async ({ page, context }, info) => {
  test.skip(!OUT, "set TOWN_TIMINGS to a directory");
  test.setTimeout(600_000);
  const fixtures = JSON.parse(readFileSync(path.join(process.cwd(), "src/hub/fixtures/towns-v2.json"), "utf8")) as Fixture[];
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("4242");
  await page.getByRole("button", { name: "Begin" }).click();
  // The town map (an overlay over the street, DEC-088) carries the bench probe.
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
  await page.getByTestId("town-map-button").click();
  await expect(page.getByTestId("town-map-canvas")).toBeVisible();
  if (info.project.name === "phone") {
    const cdp = await context.newCDPSession(page);
    await cdp.send("Emulation.setCPUThrottlingRate", { rate: 4 });
  }
  const rows: Record<string, unknown>[] = [];
  for (const f of fixtures) {
    const r = await page.evaluate(
      (x) => (window as unknown as { __townMap: { bench(i: unknown, g: unknown, s: string): Promise<Record<string, number | string>> } }).__townMap.bench(x.input, x.geo, x.seed),
      f,
    );
    rows.push({ town: f.input.municipality, seed: f.seed, source: f.source, ...r });
  }
  mkdirSync(OUT, { recursive: true });
  writeFileSync(path.join(OUT, `town-timings-${info.project.name}.json`), JSON.stringify(rows, null, 1));
  for (const r of rows) expect(r.storedFrom).toBe("stored");
});
