import { expect, test } from "@playwright/test";
import { openAgenda } from "./pocket";

// An action chosen from the pocket somewhere else walks the player there first; what happened
// shows only once they have arrived (owner report: fundraising's result appeared before the
// player finished walking). The engine resolves the action at once; the UI holds the outcome.

interface StreetProbe {
  __townStreet?: { view: { scene: { playerMoving: boolean } } } | null;
}

test("an action from the pocket shows its result only after the walk to where it happens", async ({ page }, info) => {
  test.skip(info.project.name !== "desktop", "one profile is enough");
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("4242");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
  await expect(page.locator(".town-street-stage[data-ready='1']")).toBeVisible({ timeout: 30_000 });

  const agenda = await openAgenda(page);
  await agenda.getByText("Fundraise", { exact: true }).first().click();
  await agenda.getByRole("button", { name: "Go" }).click();

  // On the way: walking, and no result yet.
  await expect.poll(() => page.evaluate(() => (window as unknown as StreetProbe).__townStreet?.view.scene.playerMoving ?? false)).toBe(true);
  await expect(page.getByTestId("toast")).toHaveCount(0);
  // Arrived: the result, and the player is no longer walking.
  await expect(page.getByTestId("toast")).toHaveText(/raised/, { timeout: 20_000 });
  expect(await page.evaluate(() => (window as unknown as StreetProbe).__townStreet?.view.scene.playerMoving ?? true)).toBe(false);
});
