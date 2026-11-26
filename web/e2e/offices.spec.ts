import { expect, test } from "@playwright/test";
import { showPlaces } from "./pocket";
import { seedFor } from "./seeds";

// R2 (DEC-098): offices and bodies come from the constitution. Vote on a
// colleague's bill in the chamber and see how the office that signs it
// leans; file for the town's executive office from the posts the engine
// offers; see the race for it at the party office. Every label here is the
// base constitution's own (its Fluent keys), not the web's.
// Medium world: a colleague's bill awaits the player's vote on the first
// turn.
test("vote, see the signer's lean, file for the executive office and see its race", async ({ page }) => {
  test.setTimeout(240_000);
  await page.goto("./");
  // A world that reaches the player's race within 30 turns (tests/seeds.json,
  // found again by `mandate-cli seeds refresh` when the world changes).
  await page.getByPlaceholder("e.g. 20260").fill(seedFor("player_race_for_the_executive_within_30_turns"));
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();

  // The chamber: the whip names the body, the signer's lean names the office.
  await showPlaces(page);
  await page.getByTestId("spot-council_chamber").first().click();
  const panel = page.getByTestId("place-panel");
  await expect(page.getByTestId("where-place")).toHaveText("Council chamber", { timeout: 30_000 });
  const whip = panel.getByTestId("whip");
  await expect(whip.getByRole("heading", { name: "Town council" })).toBeVisible();
  await expect(whip.getByTestId("signer-lean")).toHaveText(/Mayor: (will sign|leaning to sign|undecided|leaning to veto|will veto)/);
  // A colleague's bill to vote on: the policy families (F4) changed which
  // bills the council has on the first turn, so wait up to six turns for one.
  const vote = whip.getByTestId("your-vote");
  for (let turn = 0; turn < 6 && (await vote.getByRole("button", { name: "Vote yes" }).count()) === 0; turn++) {
    await panel.getByRole("button", { name: "Close" }).first().click();
    await page.getByRole("button", { name: "End turn" }).click();
    await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled({ timeout: 30_000 });
    await showPlaces(page);
    await page.getByTestId("spot-council_chamber").first().click();
    await expect(page.getByTestId("where-place")).toHaveText("Council chamber", { timeout: 30_000 });
  }
  await vote.getByRole("button", { name: "Vote yes" }).click();
  await expect(vote.getByText("You voted yes")).toBeVisible();
  await expect(panel.getByTestId("members").getByText("Mayor").first()).toBeVisible();


  // File at the town hall: one option per post the engine offers, then standing down.
  await showPlaces(page);
  await page.getByTestId("spot-town_hall").first().click();
  const calendar = panel.getByTestId("election-calendar");
  await expect(calendar).toBeVisible({ timeout: 30_000 });
  await expect(calendar.getByRole("radio")).toHaveCount(3);
  await expect(calendar.getByRole("radio", { name: /^Stand again: Ward \d+$/ })).toBeChecked();
  await calendar.getByText("Run for Mayor").click();
  await expect(calendar.getByRole("radio", { name: "Run for Mayor" })).toBeChecked();

  // The campaign: the player's race is for the office.
  await showPlaces(page);
  await page.getByTestId("spot-party_office").first().click();
  await expect(panel.getByTestId("internal-poll")).toBeVisible({ timeout: 30_000 });
  const endTurn = page.getByRole("button", { name: "End turn" });
  for (let i = 0; i < 30 && (await page.getByTestId("my-race").count()) === 0; i++) {
    await endTurn.click();
    await expect(endTurn).toBeEnabled();
  }
  const race = page.getByTestId("my-race");
  await expect(race).toBeVisible();
  await expect(race.getByRole("heading", { name: "Mayor" })).toBeVisible();
});
