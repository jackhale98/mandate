import { expect, test, type Page } from "@playwright/test";

// M5d part 2: the storyteller is chosen at new game and shown in the run
// metadata (DEC-033); the catalogue's sliders ask the engine for the cost
// (the UI never computes it); overheard talk is grounded and fully rendered.

const shots = process.env.M5D2_SHOTS;

async function showList(page: Page) {
  await page.getByTestId("town-tab-list").click();
  await expect(page.getByRole("tabpanel", { name: /List/ })).toBeVisible();
}

const openPocket = async (page: Page) => {
  if ((await page.getByTestId("pocket").count()) === 0) await page.getByTestId("pocket-button").click();
  await expect(page.getByTestId("pocket")).toBeVisible();
  return page.getByTestId("pocket");
};

test("storyteller at new game, and sliders that move the engine's estimate", async ({ page }, info) => {
  await page.goto("./");
  const picker = page.getByTestId("storyteller-picker");
  await expect(picker).toBeVisible();
  await expect(picker.getByRole("radio", { name: "Classic" })).toBeChecked();
  await picker.getByText("Steady", { exact: true }).click();
  await expect(picker).toContainText("Fewer crises");
  await page.getByPlaceholder("e.g. 20260").fill("31337");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();

  await showList(page);
  await page.getByTestId("spot-town_hall").click();
  const catalog = page.getByTestId("place-panel").getByTestId("catalog");
  await catalog.getByRole("button", { name: /Transit frequency/ }).click();
  const cost = catalog.getByTestId("policy-cost");
  const before = await cost.textContent();
  await catalog.locator("#param-level").fill("5");
  await expect(cost).not.toHaveText(before ?? "");
  await expect(catalog.getByTestId("policy-estimate")).toContainText("of the town budget");
  const high = await cost.textContent();
  const body = catalog.locator(".template-body").first();
  if (shots) {
    await body.scrollIntoViewIfNeeded();
    await body.screenshot({ path: `${shots}/slider-level5-${info.project.name}.png` });
  }
  await catalog.locator("#param-level").fill("1");
  await expect(cost).not.toHaveText(high ?? "");
  if (shots) await body.screenshot({ path: `${shots}/slider-level1-${info.project.name}.png` });

  const pocket = await openPocket(page);
  await pocket.getByRole("button", { name: "Save & quit" }).click();
  await expect(page.getByText("Steady storyteller").first()).toBeVisible();
});

test("overheard talk is grounded and rendered", async ({ page }, info) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("4242");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await showList(page);
  let found = false;
  for (const kind of ["diner", "market", "union_hall"]) {
    const spot = page.getByTestId(`spot-${kind}`);
    if ((await spot.count()) === 0) continue;
    await spot.first().click();
    const heard = page.getByTestId("overheard");
    await expect(heard).toBeVisible();
    const lines = heard.locator("li");
    expect(await lines.count()).toBeGreaterThan(0);
    // Every line is rendered (no missing Fluent keys or arguments).
    await expect(heard).not.toContainText("⟦");
    await expect(heard).not.toContainText("{");
    if (shots) await heard.screenshot({ path: `${shots}/overheard-${info.project.name}.png` });
    found = true;
    break;
  }
  expect(found).toBe(true);
});
