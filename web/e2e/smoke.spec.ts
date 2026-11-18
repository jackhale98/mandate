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
  await page.getByRole("tab", { name: "Map" }).click();
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

// M3: propose a policy, put it before the council, lobby, and survive a reload
// mid-turn (Ironman journal replays player commands deterministically, DEC-044).
test("draft, introduce, lobby and resume mid-turn", async ({ page }) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("31337");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("player-card")).toBeVisible();
  await expect(page.getByTestId("slots")).toHaveText("3 slots left this turn");

  const catalog = page.getByTestId("catalog");
  await catalog.getByRole("button", { name: /Neighbourhood parks|Parks/ }).first().click();
  await catalog.getByRole("button", { name: "Create draft" }).click();
  await expect(page.getByTestId("toast")).toHaveText(/Draft created/);
  await page.getByTestId("bills").getByRole("button", { name: "Introduce to council" }).click();
  await expect(page.getByTestId("toast")).toHaveText(/Bill introduced/);

  await page.getByRole("tab", { name: "Council" }).click();
  const whip = page.getByTestId("whip");
  await expect(whip.getByText(/Expected yes/)).toBeVisible();
  await whip.getByRole("button", { name: /^Lobby/ }).first().click();
  await expect(page.getByTestId("toast")).toHaveText(/persuaded|not convinced/);
  await expect(whip.getByText("1 slot left this turn")).toBeVisible();

  const hash = await stateHash(page);
  await page.reload();
  await page.getByRole("button", { name: "Continue" }).first().click();
  await expect(page.getByTestId("slots")).toHaveText("1 slot left this turn");
  expect(await stateHash(page)).toBe(hash);
});

// M4 owner checkpoint: play a full local election (filing, campaign, polls,
// election night with an explanation).
test("a full local election", async ({ page }) => {
  test.setTimeout(180_000);
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("4040");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("player-card")).toBeVisible();
  await page.getByRole("tab", { name: "Campaign" }).click();
  await expect(page.getByTestId("election-calendar")).toContainText("3 November 2020");
  const endTurn = page.getByRole("button", { name: "End turn" });
  for (let i = 0; i < 30 && (await page.getByTestId("my-race").count()) === 0; i++) {
    await endTurn.click();
    await expect(endTurn).toBeEnabled();
  }
  await expect(page.getByTestId("my-race")).toBeVisible();
  await expect(page.getByText("Campaign season")).toBeVisible();
  // Campaign: one canvass, then let the weeks run.
  await page.getByTestId("agenda").getByRole("button", { name: "Go" }).click();
  await expect(page.getByTestId("toast")).toHaveText(/knocked on doors/);
  for (let i = 0; i < 12 && (await page.getByTestId("election-night").count()) === 0; i++) {
    await endTurn.click();
    await expect(endTurn).toBeEnabled();
  }
  const night = page.getByTestId("election-night");
  await expect(night).toBeVisible();
  await expect(night.getByText(/You (won|lost)\./)).toBeVisible();
  await expect(night.getByText("Why it went this way").first()).toBeVisible();
  await night.getByRole("button", { name: "Continue" }).last().click();
  await expect(night).toHaveCount(0);
});
