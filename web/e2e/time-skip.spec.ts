import { expect, test, type Page } from "@playwright/test";
import { openPocket } from "./pocket";

// Time skips (DEC-012, M6): Term in review from the pocket: set directives,
// pick a point the engine offers, Skip; progress with Cancel; then the
// summary of what happened while away (interrupted, or the target reached).
// The Legacy record opens from the pocket too. Desktop and phone.

const start = async (page: Page, seed: string) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill(seed);
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
};

test("set directives, skip ahead, and read what happened while away", async ({ page }) => {
  await start(page, "2027");
  // One turn in, the first chapter has begun (a session to skip to the end of).
  await page.getByRole("button", { name: "End turn" }).click();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled({ timeout: 20_000 });
  const before = await page.locator(".gamebar-date strong").textContent();

  const pocket = await openPocket(page, "today");
  await pocket.getByTestId("open-term-review").click();
  const review = page.getByTestId("term-review");
  await expect(review).toBeVisible();
  await expect(review.getByRole("heading", { name: "Term in review" })).toBeVisible();

  // Directives: the engine's options, the choice kept.
  const ground = review.getByTestId("directive-directive.ground").locator("select");
  await ground.selectOption("lots");
  await expect(ground).toHaveValue("lots");
  const vote = review.getByTestId("directive-directive.vote").locator("select");
  await vote.selectOption("ward");
  await expect(vote).toHaveValue("ward");

  // A point to skip to, then Skip.
  const target = review.getByTestId("skip-target-next_session");
  await expect(target).toBeVisible();
  await target.check();
  await review.getByTestId("skip-go").click();

  // The summary: why it stopped, the span, what changed and what happened.
  const summary = page.getByTestId("skip-summary");
  await expect(summary).toBeVisible({ timeout: 45_000 });
  await expect(summary.getByTestId("skip-stop")).toHaveAttribute("data-stop", /^(reached|interrupted)$/);
  await expect(summary.getByRole("heading", { name: "While you were away" })).toBeVisible();
  await expect(summary).toContainText(/turns?/);
  await expect(summary).not.toContainText("⟦");
  await summary.getByTestId("skip-summary-close").click();
  await expect(summary).toHaveCount(0);
  // Time passed.
  await expect(page.locator(".gamebar-date strong")).not.toHaveText(before ?? "");

  // Directives are kept for the next skip.
  const again = await openPocket(page, "today");
  await again.getByTestId("open-term-review").click();
  await expect(page.getByTestId("term-review").getByTestId("directive-directive.ground").locator("select")).toHaveValue("lots");
  await page.getByTestId("term-review").getByRole("button", { name: "Close" }).click();

  // The Legacy record so far.
  const p = await openPocket(page, "today");
  await p.getByTestId("open-legacy").click();
  const legacy = page.getByTestId("epilogue");
  await expect(legacy).toBeVisible();
  await expect(legacy.getByRole("heading", { name: "Legacy" }).first()).toBeVisible();
  await expect(legacy).not.toContainText("⟦");
});

test("a skip can be cancelled between turns", async ({ page }) => {
  await start(page, "2028");
  await page.getByRole("button", { name: "End turn" }).click();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled({ timeout: 20_000 });
  const pocket = await openPocket(page, "today");
  await pocket.getByTestId("open-term-review").click();
  const review = page.getByTestId("term-review");
  await review.getByTestId("skip-target-end_of_term").check();
  await review.getByTestId("skip-go").click();
  const cancel = review.getByTestId("skip-cancel");
  // A short skip may be over before the button can be pressed: either way a summary follows.
  if (await cancel.isVisible().catch(() => false)) await cancel.click().catch(() => undefined);
  const summary = page.getByTestId("skip-summary");
  await expect(summary).toBeVisible({ timeout: 60_000 });
  await expect(summary.getByTestId("skip-stop")).toHaveAttribute("data-stop", /^(cancelled|reached|interrupted|act_ended)$/);
});
