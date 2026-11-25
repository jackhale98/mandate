import { expect, test, type Page } from "./csp";
import { showPlaces } from "./pocket";

// Lobbying is a conversation (M6 note 29). The chamber's whip list is the
// count (for, against, undecided, and why as far as the player knows); each
// member's row offers "Talk to them", which opens the conversation on the
// bill without a walk (DEC-051), straight at the arguments the engine
// offers, each with its chance and its cost before it is chosen. Played by
// keyboard; the phone project runs it on a phone. No raw keys anywhere.

const raw = /⟦|\{\s*\$|\b[a-z]+(-[a-z]+){2,}\b/;

const toChamberWithBill = async (page: Page) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("31337");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await showPlaces(page);
  await page.getByTestId("spot-town_hall").click();
  const panel = page.getByTestId("place-panel");
  await expect(panel.getByTestId("you-are-here-panel")).toBeVisible();
  const catalog = panel.getByTestId("catalog");
  await catalog.getByRole("button", { name: /Neighbourhood parks|Parks/ }).first().click();
  await catalog.getByRole("button", { name: "Create draft" }).click();
  await expect(page.getByTestId("toast")).toHaveText(/Draft created/);
  await panel.getByTestId("bills").getByRole("button", { name: "Introduce to council" }).click();
  await expect(page.getByTestId("toast")).toHaveText(/Bill introduced/);
  await expect(page.getByTestId("where-place")).toHaveText("Council chamber", { timeout: 30_000 });
  return panel;
};

test("lobby a member from the chamber's list, by keyboard", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  const panel = await toChamberWithBill(page);
  const whip = panel.getByTestId("whip");
  await expect(whip.getByText(/Expected yes/)).toBeVisible();
  // The count says why, as far as the player knows; nobody is one tap from a vote any more.
  await expect(whip.getByRole("button", { name: /^Lobby/ })).toHaveCount(0);
  const why = whip.getByTestId("whip-why").first();
  await expect(why).toBeVisible();
  expect(await why.textContent()).not.toMatch(raw);
  const talk = whip.getByTestId("whip-talk").first();
  await talk.focus();
  await page.keyboard.press("Enter");

  // The conversation opens on the bill, at its arguments.
  const box = page.getByTestId("talk-panel");
  await expect(box).toBeVisible();
  const args = box.getByTestId("talk-arguments");
  await expect(args).toBeVisible();
  await expect(args.getByTestId("talk-argument").first()).toBeVisible();
  // The plain pitch is always there; each shows its chance and its cost before it is made.
  await expect(args.getByRole("button", { name: /Make the case/ })).toBeVisible();
  const first = args.getByTestId("talk-argument").first();
  await expect(first.getByTestId("talk-odds")).toBeVisible();
  await expect(first.getByTestId("talk-costs")).toContainText(/half a slot/);
  await expect(args.getByRole("button", { name: /Meet after hours/ })).toBeVisible();
  await expect(args.getByText(/legal risk/).first()).toBeVisible();
  expect(await box.textContent()).not.toMatch(/⟦|\{\s*\$/);

  // The first argument has the keyboard: Enter makes it.
  await expect(first.locator("button.dlg-choice")).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("talk-bubble").locator("p").first()).not.toBeEmpty({ timeout: 15_000 });
  // The reply, page by page (Continue), then the outcome.
  for (let i = 0; i < 20 && (await box.getByTestId("talk-outcome").count()) === 0; i++) {
    const more = box.getByTestId("talk-continue");
    if ((await more.count()) > 0) await more.locator("button").click();
    else await page.waitForTimeout(300);
  }
  await expect(box.getByTestId("talk-outcome")).toHaveText(/came round|was not persuaded/);
  expect(await box.textContent()).not.toMatch(/⟦|\{\s*\$/);
  // Escape steps back to the topics, then leaves.
  await page.keyboard.press("Escape");
  await page.keyboard.press("Escape");
  await expect(box).toHaveCount(0);
  // The whip shows the talk.
  await expect(whip.getByText(/you talked this week/).first()).toBeVisible();
});
