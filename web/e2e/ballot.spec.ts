import { expect, test } from "@playwright/test";
import { showPlaces } from "./pocket";

// Play note 20: election night always has a screen. A player on no ballot
// (here: they stood down) still sees the results where they live, and what
// became of the seat they held. The filing form names each post's holder and
// the Standing the party asks. Desktop and phone.
test("election night when the player was on no ballot", async ({ page }, info) => {
  test.setTimeout(240_000);
  if (info.project.name === "phone") await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("4040");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await showPlaces(page);
  await page.getByTestId("spot-town_hall").first().click();
  const cal = page.getByTestId("election-calendar");
  await expect(cal).toBeVisible({ timeout: 30_000 });
  // Before filing: the executive's holder, their outlook in words, and the gate against the player's Standing.
  const notes = cal.getByTestId("nomination-notes").first();
  await expect(notes).toContainText(/Held by |Nobody holds it now\./);
  await expect(notes).toContainText(/Your party asks Standing of \d+ for this\. Yours is \d+\./);
  await expect(notes).not.toContainText("%");
  if (info.project.name === "phone") {
    await notes.scrollIntoViewIfNeeded();
    await page.screenshot({ path: process.env.BALLOT_SHOTS ? `${process.env.BALLOT_SHOTS}/filing-form.png` : info.outputPath("filing-form.png") });
  }
  await cal.getByTestId("file-retire").click();
  await expect(cal.getByTestId("file-retire").locator("input")).toBeChecked();
  const endTurn = page.getByRole("button", { name: "End turn" });
  const night = page.getByTestId("election-night");
  const epilogue = page.getByTestId("open-epilogue");
  for (let i = 0; i < 40 && (await night.count()) === 0 && (await epilogue.count()) === 0; i++) {
    await endTurn.click();
    await expect(endTurn.or(epilogue)).toBeEnabled({ timeout: 30_000 });
  }
  await expect(night).toBeVisible();
  await expect(night.getByTestId("night-not-running")).toHaveText("You were not on the ballot.");
  await expect(night.getByTestId("night-seat")).toContainText("the post you held");
  await expect(night.getByTestId("night-seats")).toContainText("won by");
  if (info.project.name === "phone") {
    await page.screenshot({ path: process.env.BALLOT_SHOTS ? `${process.env.BALLOT_SHOTS}/night-no-ballot.png` : info.outputPath("night-no-ballot.png") });
  }
  await night.getByRole("button", { name: "Continue" }).last().click();
  await expect(night).toHaveCount(0);
});
