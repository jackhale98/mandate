import { expect, type Page } from "@playwright/test";

// A run some turns in, played in the browser from a pinned seed: what the
// pocket's length budget and the arc sheet's test stand on until the
// mid-career run file (POCKET_REWORK §6.4 R3) exists. Small world; "Start
// with local knowledge" gives the player the town's people and worries, as a
// later career would. Every seed so far raises the budget round (a standing
// arc, A2) on turn 9: its estimates (a visit, a talk), then on turn 10 its
// positions (an ask, a decision, a talk).

export async function startRun(page: Page, seed: string, local = true) {
  await page.goto("./");
  if (local) await page.getByTestId("local-knowledge-toggle").check();
  await page.getByPlaceholder("e.g. 20260").fill(seed);
  await page.getByText("Small", { exact: true }).click();
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
}

/** End turns until the bar shows turn `n`. */
export async function playTo(page: Page, n: number) {
  for (;;) {
    const sub = (await page.locator(".gamebar-sub").first().textContent()) ?? "";
    const turn = Number(/Turn (\d+)/.exec(sub)?.[1] ?? "0");
    if (turn >= n) return;
    if ((await page.getByTestId("pocket").count()) > 0) await page.getByTestId("pocket").getByRole("button", { name: "Close" }).first().click();
    await page.getByRole("button", { name: "End turn" }).click();
    await expect(page.locator(".gamebar-sub").first()).toContainText(`Turn ${turn + 1}`, { timeout: 30_000 });
    await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled({ timeout: 30_000 });
  }
}
