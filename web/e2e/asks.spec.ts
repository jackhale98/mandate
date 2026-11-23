import { expect, test, type Locator, type Page } from "@playwright/test";
import { openPocket } from "./pocket";

// Asks and promises (E7; DEC-097 §2, ENGAGEMENT §3.9): offers in the Today
// tab with Accept and Decline, an accepted ask kept and marked kept; a
// campaign promise made from the picker; and the time skip's summary listing
// a lapsed law, with counts in the locale's style. Desktop and phone.
// MANDATE_CAPTURE=<dir> saves screenshots (deviceScaleFactor 2) for review.

test.use({ deviceScaleFactor: 2 });

const capture = process.env.MANDATE_CAPTURE;

async function shot(target: Locator, name: string, project: string) {
  if (capture) await target.screenshot({ path: `${capture}/e7-${name}-${project}.png` });
}

const start = async (page: Page, seed: string) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill(seed);
  await page.getByText("Small", { exact: true }).click();
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
};

const endTurn = async (page: Page) => {
  const before = await page.locator(".gamebar-date strong").textContent();
  await page.getByRole("button", { name: "End turn" }).click();
  await expect(page.locator(".gamebar-date strong")).not.toHaveText(before ?? "", { timeout: 30_000 });
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled({ timeout: 30_000 });
};

/** Bring a control in a scrolling sheet into view (clear of the phone's bottom bar), then click it. */
const press = async (l: Locator) => {
  await l.evaluate((el) => el.scrollIntoView({ block: "center" }));
  await l.click();
};

test("asks: accept one, decline one, keep one and see it kept", async ({ page }, info) => {
  await start(page, "3");
  // Two turns in, two offers wait (the badge counts them).
  for (let i = 0; i < 4 && (await page.getByTestId("asks-badge").count()) === 0; i++) await endTurn(page);
  await expect(page.getByTestId("asks-badge")).toBeVisible();
  await expect(page.getByTestId("asks-badge")).toHaveAttribute("title", /asks? or promises? waiting/);

  let pocket = await openPocket(page, "today");
  const block = pocket.getByTestId("pocket-asks");
  await expect(block).toBeVisible();
  await expect(block).not.toContainText("⟦");
  const meeting = block.locator('li[data-template="ask.town_hall_here"][data-status="offered"]');
  const other = block.locator('li[data-status="offered"]:not([data-template="ask.town_hall_here"])').first();
  await expect(meeting).toBeVisible();
  await expect(other).toBeVisible();
  await expect(meeting.getByRole("button", { name: /^Accept: / })).toBeVisible();
  await expect(meeting).toContainText(/Answer by .* left/);
  await shot(block, "today-asks", info.project.name);

  // Decline one: it is answered at once.
  const otherId = (await other.getAttribute("data-testid")) ?? "";
  await press(other.getByRole("button", { name: /^Decline: / }));
  await expect(page.getByTestId("toast")).toContainText("You said no:");
  await expect(page.getByTestId("toast")).not.toContainText("{");

  // Accept the meeting: it now shows what to do next and where.
  const meetingId = (await meeting.getAttribute("data-testid")) ?? "";
  pocket = await openPocket(page, "today");
  await press(pocket.getByTestId(meetingId).getByRole("button", { name: /^Accept: / }));
  await expect(page.getByTestId("toast")).toContainText("You said yes:");
  pocket = await openPocket(page, "today");
  const accepted = pocket.getByTestId(meetingId);
  await expect(accepted).toHaveAttribute("data-status", "accepted");
  await expect(accepted.getByTestId("ask-need")).toContainText("Next: Hold a town hall in");
  await expect(accepted).toContainText(/Due .* left/);
  await expect(pocket.getByTestId("asks-recent").getByTestId(otherId)).toHaveAttribute("data-status", "declined");

  // Keep it: go where it is done (the engine's route) and hold the meeting.
  await press(accepted.getByRole("button", { name: /^Go to .* for: / }));
  const panel = page.getByTestId("place-panel");
  await expect(panel).toBeVisible();
  const hall = panel.getByTestId("place-action-town_hall");
  await expect(hall).toBeVisible();
  await press(hall.getByRole("button", { name: /^Do it|^Go$/ }));
  await expect(page.getByTestId("toast")).toBeVisible({ timeout: 15_000 });
  await panel.getByRole("button", { name: "Close" }).click();
  await endTurn(page);
  pocket = await openPocket(page, "today");
  const kept = pocket.getByTestId("asks-recent").getByTestId(meetingId);
  await expect(kept).toHaveAttribute("data-status", "kept");
  await expect(kept.getByTestId("ask-status")).toHaveText("Kept");
  await expect(pocket.getByTestId("pocket-asks")).not.toContainText("⟦");
});

/** Talk to someone from an ask's Talk button: go to them if they are elsewhere, choose the ask's own choice, and return what they said. */
const askInConversation = async (page: Page, askId: string, choice: string, label: RegExp) => {
  const pocket = await openPocket(page, "today");
  await press(pocket.getByTestId(askId).getByTestId("talk-button"));
  const dlg = page.getByTestId("talk-panel");
  await expect(dlg).toBeVisible();
  if ((await dlg.getByTestId("talk-go").count()) > 0) await dlg.getByTestId("talk-go").locator("button.dlg-choice").click();
  const row = dlg.getByTestId(`talk-topic-${choice}`);
  await expect(row).toBeVisible();
  await expect(row).toHaveText(label);
  await expect(row).not.toContainText("{");
  await row.locator("button.dlg-choice").click();
  const live = page.getByTestId("talk-reply-live");
  await expect(live).not.toBeEmpty({ timeout: 15_000 });
  const said = (await live.textContent()) ?? "";
  await dlg.getByTestId("talk-goodbye").locator("button.dlg-choice").click();
  await expect(dlg).toHaveCount(0);
  return said;
};

test("an introduction: put in a word with the official, tell the asker, and it is kept at once", async ({ page }, info) => {
  test.setTimeout(120_000);
  await start(page, "61");
  await openPocket(page, "today");
  const offer = () => page.getByTestId("pocket").locator('li[data-template="ask.introduce_me"][data-status="offered"]');
  for (let i = 0; i < 4 && (await offer().count()) === 0; i++) {
    await endTurn(page);
    await openPocket(page, "today");
  }
  await expect(offer()).toBeVisible();
  const askId = (await offer().getAttribute("data-testid")) ?? "";
  const choice = `choice.ask.${askId.replace(/^ask-/, "")}`;
  await press(offer().getByRole("button", { name: /^Accept: / }));
  await expect(page.getByTestId("toast")).toContainText("You said yes:");
  let pocket = await openPocket(page, "today");
  const first = (await pocket.getByTestId(askId).getByTestId("talk-button").getAttribute("aria-label")) ?? "";

  // The official: the ask's own choice, named for the asker, and a real answer.
  const said = await askInConversation(page, askId, choice, /^Put in a word for \S/);
  expect(said).toMatch(/Of course|Ten minutes/);
  // The step is met in that command: the ask now points at the asker.
  pocket = await openPocket(page, "today");
  const accepted = pocket.getByTestId(askId);
  await expect(accepted).toHaveAttribute("data-status", "accepted");
  await expect(accepted.getByTestId("talk-button")).not.toHaveAttribute("aria-label", first);
  await shot(accepted, "ask-after-official", info.project.name);

  // The asker: told it is arranged, and the ask is kept there and then.
  const thanks = await askInConversation(page, askId, choice, /^Tell \S.* it is arranged$/);
  expect(thanks).toContain("Thank you");
  pocket = await openPocket(page, "today");
  const kept = pocket.getByTestId("asks-recent").getByTestId(askId);
  await expect(kept).toHaveAttribute("data-status", "kept");
  await expect(kept.getByTestId("ask-status")).toHaveText("Kept");
  await expect(pocket.getByTestId("pocket-asks")).not.toContainText("⟦");
});

test("a campaign promise is made from the picker", async ({ page }, info) => {
  test.setTimeout(180_000);
  await start(page, "2");
  // Skip ahead to the campaign (interrupts may stop short: go on until promises can be made).
  for (let round = 0; round < 6; round++) {
    const pocket = await openPocket(page, "today");
    if ((await pocket.getByTestId("open-promises").count()) > 0) break;
    await pocket.getByTestId("open-term-review").click();
    const review = page.getByTestId("term-review");
    await expect(review).toBeVisible();
    const target = review.getByTestId("skip-target-campaign");
    if ((await target.count()) === 0) {
      await review.getByRole("button", { name: "Close" }).click();
      await endTurn(page);
      continue;
    }
    await target.check();
    await review.getByTestId("skip-go").click();
    const summary = page.getByTestId("skip-summary");
    await expect(summary).toBeVisible({ timeout: 90_000 });
    await press(summary.getByTestId("skip-summary-close"));
    await expect(summary).toHaveCount(0);
  }
  let pocket = await openPocket(page, "today");
  await press(pocket.getByTestId("open-promises"));
  const picker = page.getByTestId("promise-picker");
  await expect(picker).toBeVisible();
  await expect(picker.getByTestId("promise-made")).toContainText("0 of 3 promises made in this race");
  await expect(picker).not.toContainText("⟦");
  await shot(picker, "promise-picker", info.project.name);
  const make = picker.getByRole("button", { name: /^Promise: / }).first();
  const label = (await make.getAttribute("aria-label")) ?? "";
  await press(make);
  await expect(picker.getByTestId("promise-made")).toContainText("1 of 3 promises made in this race");
  await picker.getByRole("button", { name: "Close" }).click();
  await expect(picker).toHaveCount(0);
  // The promise stands in the Today block, and in the story so far.
  pocket = await openPocket(page, "today");
  const promised = pocket.getByTestId("pocket-asks").locator('li[data-template^="promise."]');
  await expect(promised.first()).toBeVisible();
  await expect(promised.first()).toContainText(label.replace(/^Promise: /, ""));
  await expect(promised.first()).toContainText("Campaign promise");
  await pocket.getByTestId("open-story").click();
  const story = page.getByTestId("story-sheet");
  await expect(story.getByTestId("story-promises")).toContainText("Made 1");
});

test("the skip summary lists a lapsed law, with counts in the locale's style", async ({ page }, info) => {
  test.setTimeout(120_000);
  // Seed 5: its next-session skip lapses a law (seed 2's did until the party
  // line changed which laws pass in the burn-in); the native twin is
  // `laws_in_force_never_fall_unexplained` in crates/mandate-wasm/tests/asks.rs.
  await start(page, "5");
  await endTurn(page);
  const pocket = await openPocket(page, "today");
  await pocket.getByTestId("open-term-review").click();
  const review = page.getByTestId("term-review");
  await review.getByTestId("skip-target-next_session").check();
  await review.getByTestId("skip-go").click();
  const summary = page.getByTestId("skip-summary");
  await expect(summary).toBeVisible({ timeout: 90_000 });
  await expect(summary.getByTestId("skip-items")).toContainText("A law lapsed:");
  await expect(summary).not.toContainText("Nothing of note");
  await expect(summary.getByTestId("skip-changes")).toContainText(/Residents: \d{1,3}(,\d{3})+ → \d{1,3}(,\d{3})+/);
  await expect(summary).not.toContainText("⟦");
  await shot(summary, "skip-summary", info.project.name);
});
