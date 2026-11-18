import { expect, test, type Page } from "@playwright/test";

const stateHash = async (page: Page) => {
  await page.getByRole("tab", { name: "Overview" }).click();
  return page.getByText(/state [0-9a-f]{16}/).textContent();
};

// M0 exit gate: start a game, end a turn, save, reload offline, resume (M0-09).
test("new game, end turn, save, resume offline", async ({ page, context }) => {
  await page.goto("./");
  await expect(page.getByRole("heading", { name: "Mandate" })).toBeVisible();
  await page.getByPlaceholder("e.g. 20260").fill("4242");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByText("Turn 0")).toBeVisible();
  await page.getByRole("button", { name: "End turn" }).click();
  await expect(page.getByText("Turn 1")).toBeVisible();
  const hash = await stateHash(page);
  await page.getByRole("button", { name: "Save & quit" }).click();
  await expect(page.getByRole("button", { name: "Continue" })).toBeVisible();

  await page.evaluate(async () => {
    await navigator.serviceWorker.ready;
  });
  await page.reload();
  await page.evaluate(async () => {
    await navigator.serviceWorker.ready;
  });
  await context.setOffline(true);
  await page.reload();
  await page.getByRole("button", { name: "Continue" }).first().click();
  await expect(page.getByText("Turn 1")).toBeVisible();
  expect(await stateHash(page)).toBe(hash);
  await context.setOffline(false);
});

// M1: the generated country renders and towns can be inspected without the
// map (accessible list path, DEC-051).
test("map renders and a town can be inspected from the list", async ({ page }) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("777");
  await page.getByRole("button", { name: "Begin" }).click();
  const canvas = page.getByRole("img", { name: /Drag to pan/ });
  await expect(canvas).toBeVisible();
  const box = await canvas.boundingBox();
  expect(box?.width ?? 0).toBeGreaterThan(200);
  await page.getByRole("tab", { name: "Overview" }).click();
  await page.locator(".region-accordion > li > .row-btn").first().click();
  await page.locator(".town-list .row-btn").first().click();
  await expect(page.getByRole("heading", { name: "Resources" })).toBeVisible();
  await expect(page.getByText("Town hall")).toBeVisible();
});
