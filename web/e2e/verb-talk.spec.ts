import { expect, test } from "./csp";
import { openAgenda } from "./pocket";

// Fundraising, the press, the bribe and the leak are conversations (DEC-114,
// B5). From the agenda, the press names whom it is had with and where;
// "Talk to them" goes there and opens the talk panel at the interview, with
// its chance and its costs before it is chosen; the result is the exchange,
// with a Why. No toast, and no raw keys anywhere.

const raw = /⟦|\{\s*\$|\b(press|verb|why|fundraise|talk|ui)-[a-z]+(-[a-z]+)*\b/;

test("the press is an interview: whom with, the odds and costs shown, the result with its Why", async ({ page }, info) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("4242");
  await page.getByText("Small", { exact: true }).click();
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible({ timeout: 60_000 });

  const agenda = await openAgenda(page);
  await agenda.getByText("Talk to the press", { exact: true }).first().click();
  const card = agenda.getByTestId("verb-talk-press");
  await expect(card.getByTestId("verb-talk-with")).toBeVisible();
  expect(await card.textContent()).not.toMatch(raw);
  const open = card.getByTestId("verb-talk-open");
  await open.evaluate((el) => el.scrollIntoView({ block: "center" }));
  await open.click();

  const box = page.getByTestId("talk-panel");
  await expect(box).toBeVisible();
  const interview = box.getByTestId("talk-topic-choice.press.general");
  await expect(interview.getByTestId("talk-odds")).toHaveText(/\d+%/);
  await expect(interview.getByTestId("talk-costs")).toContainText("chance of a gaffe");
  expect(await box.textContent()).not.toMatch(raw);
  if (info.project.name === "phone" && process.env.B5_SHOT) {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.screenshot({ path: process.env.B5_SHOT });
  }

  await interview.getByRole("button").click();
  await expect(box.getByTestId("talk-outcome")).toContainText(/statement ran|taken badly/);
  await expect(page.getByTestId("toast")).toHaveCount(0);
  const why = box.getByTestId("talk-result-why");
  await why.locator("summary").click();
  await expect(why).toContainText("The chance of a gaffe was");
  expect(await box.textContent()).not.toMatch(raw);
  if (info.project.name === "phone" && process.env.B5_SHOT) await page.screenshot({ path: process.env.B5_SHOT.replace(/\.png$/, "-result.png") });
});
