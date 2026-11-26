import { expect, test } from "@playwright/test";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { openPocket } from "./pocket";

// The draft of openings (DEC-101 §3; STAGED_ARCS A8): new game → the engine
// auditions the world (progress shown, one fork a worker message) → three real
// towns, each with its line, the seat and its tag, and no score → choose one →
// turn 1 in that town shows the opening's arc on Today with something to do in
// it (the first-beat guarantee). The world's details come from the engine's own
// fixture of the same seed (crates/mandate-wasm/tests/draft.rs). On a phone the
// screen is 390×844: no sideways scroll, every button at least 44 px tall.

interface Opening {
  town: number;
  town_name: string;
  template: string;
}
const here = path.dirname(fileURLToPath(import.meta.url));
const fixture = JSON.parse(readFileSync(path.resolve(here, "../src/components/fixtures/draft/draft_3.json"), "utf8")) as { openings: Opening[] };

test("new game → the draft → choose → turn 1 shows the opening's beat", async ({ page }, info) => {
  test.setTimeout(180_000);
  const phone = info.project.name === "phone";
  if (phone) await page.setViewportSize({ width: 390, height: 844 });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("3");
  await page.getByText("Small", { exact: true }).click();
  await page.getByRole("button", { name: "Choose an opening" }).click();

  // The audition, then the openings, best first, as the engine drafted them.
  const draft = page.getByTestId("draft");
  await expect(draft).toBeVisible({ timeout: 60_000 });
  const cards = page.getByTestId("draft-opening");
  await expect(cards).toHaveCount(fixture.openings.length, { timeout: 90_000 });
  for (const [i, o] of fixture.openings.entries()) {
    const card = cards.nth(i);
    await expect(card.getByRole("heading", { name: o.town_name })).toBeVisible();
    await expect(card.getByTestId("draft-pitch")).not.toBeEmpty();
    await expect(card.getByTestId("draft-seat")).not.toBeEmpty();
  }
  await expect(draft).not.toContainText("⟦");
  await expect(draft).not.toContainText(/score|agency|escalation|visibility|pacing/i);

  // A phone: no sideways scroll, and every target 44 px or more.
  const wide = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(wide).toBeLessThanOrEqual(0);
  for (const b of await draft.getByRole("button").all()) {
    const box = await b.boundingBox();
    expect(box?.height ?? 0).toBeGreaterThanOrEqual(44);
  }
  if (phone) await page.screenshot({ path: info.outputPath("draft-phone.png"), fullPage: true });

  // Choose the second (the first is "Surprise me"'s).
  const pick = fixture.openings[1] ?? fixture.openings[0];
  if (!pick) throw new Error("the fixture drafts nothing");
  await page.getByRole("button", { name: `Begin in ${pick.town_name}` }).click();
  await expect(page.getByTestId("town")).toBeVisible({ timeout: 60_000 });
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
  // The first turn played (the bar counts turns ended: "Turn 0").
  await expect(page.locator(".gamebar-sub").first()).toContainText("Turn 0");

  // Turn 1: the opening's arc is on Today, in the chosen town, with something to do in it.
  const pocket = await openPocket(page, "today");
  const row = pocket.locator("[data-testid^='arc-row-']").filter({ hasText: pick.town_name }).filter({ hasText: /to do/i });
  await expect(row.first()).toBeVisible();
  if (phone) await page.screenshot({ path: info.outputPath("turn1-phone.png") });
  await row.first().locator(".rw-main").click();
  const sheet = page.getByTestId("arc-sheet");
  await expect(sheet).toBeVisible();
  await expect(sheet.getByTestId("arc-beats")).toBeVisible();
  await expect(sheet).not.toContainText("⟦");
});

test("the draft can be left, and Begin still starts at once (the quick start)", async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("3");
  await page.getByText("Small", { exact: true }).click();
  await page.getByRole("button", { name: "Choose an opening" }).click();
  await expect(page.getByTestId("draft")).toBeVisible({ timeout: 60_000 });
  await page.getByRole("button", { name: "Back" }).click();
  await expect(page.getByRole("button", { name: "Begin" })).toBeVisible();
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible({ timeout: 60_000 });
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
});
