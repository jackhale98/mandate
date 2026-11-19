import { expect, test, type Page } from "@playwright/test";

// DEC-089: the country map's layer picker lists the engine's layer registry
// (terrain, population, density, unemployment, median income, rent, main
// industry, resources). Each layer has a legend with its units, a readout of
// a town's figure (a tap, or a hover with a mouse), and a sortable list of
// towns as its text alternative (DEC-051). Runs on desktop and phone.
//
// MAP_LAYER_SHOTS=<dir> also saves a screenshot of every layer in light and
// dark (the last test); without it that test is skipped.

const LAYERS = ["population", "density", "unemployment", "median_income", "rent", "industry", "resource"];

const start = async (page: Page, seed: string) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill(seed);
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
  await expect(page.locator(".town-street-stage[data-ready='1']")).toBeVisible({ timeout: 30_000 });
};

/** Open the map overlay on the country, with the engine's layers loaded. */
const openCountry = async (page: Page) => {
  await page.getByTestId("town-map-button").click();
  await expect(page.getByTestId("town-map-overlay")).toBeVisible();
  await page.getByTestId("map-scope-country").click();
  await expect(page.getByTestId("country-map")).toBeVisible();
  await expect(page.getByTestId("map-layer").locator("option")).toHaveCount(LAYERS.length + 1);
};

/** Tap (or click) the player's own town on the country map. */
const tapHome = async (page: Page, touch: boolean) => {
  const canvas = page.getByTestId("country-map").locator("canvas");
  await expect(canvas).toHaveAttribute("data-home-centre", /^\d+,\d+$/);
  // The legend under the map changes height with the layer, and the map
  // redraws for its new size: wait until the town stays put.
  let last = "";
  await expect
    .poll(async () => {
      const now = `${await canvas.getAttribute("data-home-centre")} ${JSON.stringify(await canvas.boundingBox())}`;
      const same = now === last;
      last = now;
      return same;
    }, { intervals: [150, 150, 150, 300] })
    .toBe(true);
  const [x, y] = ((await canvas.getAttribute("data-home-centre")) ?? "0,0").split(",").map(Number);
  const box = await canvas.boundingBox();
  if (!box) throw new Error("no map");
  if (touch) await page.touchscreen.tap(box.x + (x ?? 0), box.y + (y ?? 0));
  else await page.mouse.click(box.x + (x ?? 0), box.y + (y ?? 0));
};

const watchErrors = (page: Page) => {
  const errors: string[] = [];
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(String(e)));
  return errors;
};

test("every map layer has a legend with units and a town readout, and a sortable list", async ({ page }, info) => {
  test.setTimeout(120_000);
  const errors = watchErrors(page);
  const touch = info.project.name === "phone";
  await start(page, "4242");
  await openCountry(page);
  const picker = page.getByTestId("map-layer");
  // A dropdown: terrain, then the registry's layers in its order.
  expect(await picker.locator("option").evaluateAll((os) => os.map((o) => (o as HTMLOptionElement).value))).toEqual(["terrain", ...LAYERS]);
  await expect(page.getByTestId("map-legend")).toHaveCount(0);

  const legend = page.getByTestId("map-legend");
  const readout = page.getByTestId("map-readout-value");
  const heads = new Set<string>();
  const values = new Set<string>();
  let home = "";
  for (const key of LAYERS) {
    await picker.selectOption(key);
    await expect(legend).toHaveAttribute("data-layer", `map_layer.${key}`);
    // The canvas names the layer, and still says where the player is.
    await expect(page.getByTestId("country-map").getByRole("img", { name: /You: .+Map layer: / })).toBeVisible();
    const head = (await legend.locator(".map-legend-head").textContent()) ?? "";
    expect(head).toMatch(/·/); // the layer's name, then its units
    heads.add(head);
    // Numeric layers write the engine's breaks; categorical ones name their categories.
    if (key === "industry" || key === "resource") {
      expect(await legend.locator(".map-cats li").count()).toBeGreaterThanOrEqual(2);
      expect(await legend.locator(".map-cats li").count()).toBeLessThanOrEqual(4);
    } else {
      expect(await legend.locator(".map-scale-seg").count()).toBeGreaterThanOrEqual(2);
      expect(await legend.locator(".map-scale-edges span").count()).toBe((await legend.locator(".map-scale-seg").count()) - 1);
    }
    // A tap on the player's town reads its figure into the legend (no panel).
    await tapHome(page, touch);
    await expect(readout).toBeVisible();
    await expect(page.locator("aside.sheet")).toHaveCount(0);
    values.add((await readout.textContent()) ?? "");
    home = (await legend.locator(".map-readout-town").textContent())?.replace(/You$/, "") ?? "";
    await expect(legend.locator(".map-readout-town .you-chip")).toBeVisible();
    // Its class is marked in the legend by an outline.
    await expect(legend.locator(".map-scale-seg.on, .map-cats li.on")).toHaveCount(1);
  }
  // Each layer had its own legend and its own reading of the town.
  expect(heads.size).toBe(LAYERS.length);
  expect(values.size).toBe(LAYERS.length);
  expect(home.length).toBeGreaterThan(0);

  // Details opens the town's panel; Escape closes it, not the map.
  await page.getByTestId("map-readout-details").click();
  const panel = page.getByTestId("town-map-overlay").locator("aside.sheet");
  await expect(panel.getByRole("heading", { name: "Resources" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);

  // The list: every town with its figure, sortable, each a way to its panel.
  await picker.selectOption("unemployment");
  await page.getByTestId("map-list-toggle").click();
  await expect(page.getByTestId("map-list-toggle")).toHaveAttribute("aria-pressed", "true");
  const table = page.getByTestId("map-table");
  await expect(table).toBeVisible();
  const towns = await page.getByTestId("country-map").evaluate(() => document.querySelectorAll("[data-testid='map-table'] tbody tr").length);
  expect(towns).toBeGreaterThanOrEqual(20);
  const valueHead = table.getByRole("columnheader", { name: /Unemployment/ });
  await expect(valueHead).toHaveAttribute("aria-sort", "descending");
  const firstPct = async () => Number(((await table.locator("tbody tr").first().locator("td").textContent()) ?? "").replace(/[^0-9.]/g, ""));
  const lastPct = async () => Number(((await table.locator("tbody tr").last().locator("td").textContent()) ?? "").replace(/[^0-9.]/g, ""));
  expect(await firstPct()).toBeGreaterThanOrEqual(await lastPct());
  await page.getByTestId("map-table-sort-value").click();
  await expect(valueHead).toHaveAttribute("aria-sort", "ascending");
  expect(await firstPct()).toBeLessThanOrEqual(await lastPct());
  await page.getByTestId("map-table-sort-town").click();
  await expect(table.getByRole("columnheader", { name: /Town/ })).toHaveAttribute("aria-sort", "ascending");
  const names = await table.locator("tbody th button").allTextContents();
  expect(names).toEqual([...names].sort((a, b) => a.localeCompare(b, "en")));
  // The legend stays with the list; the player's town is flagged in it.
  await expect(page.getByTestId("map-legend")).toBeVisible();
  await expect(table.locator("tbody tr", { has: page.locator(".you-chip") })).toContainText(home);
  // A town's name opens its panel.
  await table.getByRole("button", { name: `Open ${home}` }).click();
  await expect(panel.getByRole("heading", { name: "Resources" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
  // Back to the map; terrain only takes the list away.
  await page.getByTestId("map-list-toggle").click();
  await expect(page.getByTestId("country-map").locator("canvas")).toBeVisible();
  await picker.selectOption("terrain");
  await expect(page.getByTestId("map-list-toggle")).toHaveCount(0);
  await expect(page.getByTestId("map-legend")).toHaveCount(0);

  // The choice is remembered on this device.
  await picker.selectOption("rent");
  await page.getByTestId("town-map-close").click();
  await page.getByTestId("town-map-button").click();
  await expect(page.getByTestId("map-legend")).toHaveAttribute("data-layer", "map_layer.rent");
  expect(errors).toEqual([]);
});

test("the layers follow the turns: each turn's figures are fetched again", async ({ page }) => {
  test.setTimeout(120_000);
  await start(page, "777");
  await openCountry(page);
  const map = page.getByTestId("country-map");
  await expect(map).toHaveAttribute("data-figures-day", /^\d+$/);
  const before = Number(await map.getAttribute("data-figures-day"));
  await page.getByTestId("town-map-close").click();
  await page.getByRole("button", { name: "End turn" }).click();
  const skip = page.getByText("Tap to carry on");
  if (await skip.isVisible().catch(() => false)) await skip.click();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled({ timeout: 30_000 });
  await page.getByTestId("town-map-button").click();
  await expect.poll(async () => Number(await map.getAttribute("data-figures-day")), { timeout: 15_000 }).toBeGreaterThan(before);
});

test("screenshots of every layer, light and dark", async ({ page }, info) => {
  const dir = process.env.MAP_LAYER_SHOTS;
  test.skip(!dir, "set MAP_LAYER_SHOTS to a directory to save them");
  test.setTimeout(240_000);
  const touch = info.project.name === "phone";
  await start(page, "4242");
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme });
    await openCountry(page);
    for (const key of ["terrain", ...LAYERS]) {
      await page.getByTestId("map-layer").selectOption(key);
      if (key !== "terrain") await tapHome(page, touch);
      await page.waitForTimeout(300);
      await page.screenshot({ path: `${dir}/${info.project.name}-${scheme}-${key}.png` });
    }
    await page.getByTestId("map-layer").selectOption("median_income");
    await page.getByTestId("map-list-toggle").click();
    await page.waitForTimeout(200);
    await page.screenshot({ path: `${dir}/${info.project.name}-${scheme}-list.png` });
    await page.getByTestId("map-list-toggle").click();
    await page.getByTestId("town-map-close").click();
  }
});
