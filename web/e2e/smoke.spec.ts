import { expect, test } from "@playwright/test";

// M0 exit gate: start a game, end a turn, save, reload offline, resume (M0-09).
test("new game, end turn, save, resume offline", async ({ page, context }) => {
  await page.goto("./");
  await expect(page.getByRole("heading", { name: "Mandate" })).toBeVisible();
  await page.getByPlaceholder("e.g. 20260").fill("4242");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByText("Turn 0")).toBeVisible();
  await page.getByRole("button", { name: "End turn" }).click();
  await expect(page.getByText("Turn 1")).toBeVisible();
  const hash = await page.getByText(/state [0-9a-f]{16}/).textContent();
  await page.getByRole("button", { name: "Save & quit" }).click();
  await expect(page.getByRole("button", { name: "Continue" })).toBeVisible();

  // Wait for the service worker to take control, then go offline.
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
  expect(await page.getByText(/state [0-9a-f]{16}/).textContent()).toBe(hash);
  await context.setOffline(false);
});
