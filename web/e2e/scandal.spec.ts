import { expect, test, type Page } from "@playwright/test";
import { openAgenda, openPocket, showPlaces } from "./pocket";

// M5d (DEC-034, DEC-006, DEC-073): a shady choice shows its risk before it is
// made; months later the story comes out, the newspaper runs it, and an
// investigation raises a legal-response decision, answered from the pocket.
// Seed 37 (small) is a world where the envelope's fate is to come out within
// a few months and prosecutors open a case (the engine is deterministic).

const closeSheet = async (page: Page, testId: string) => {
  const sheet = page.getByTestId(testId);
  await sheet.getByRole("button", { name: "Close" }).first().click();
  await expect(sheet).toHaveCount(0);
};

const endTurn = async (page: Page, turn: number) => {
  await page.getByTestId("end-turn").click();
  await expect(page.getByText(`Turn ${turn}`, { exact: false }).first()).toBeVisible();
  // The pocket and desk refresh before the button is free again.
  await expect(page.getByTestId("end-turn")).toBeEnabled();
};

test("a shady choice comes out later, makes the paper, and draws an investigation", async ({ page }) => {
  test.setTimeout(180_000);
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("37");
  await page.getByText("Small", { exact: true }).click();
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();

  // The envelope, from the agenda (DEC-114, B5): whoever is offering, in the
  // diner's back booth; the conversation's choice shows the risk line before acting.
  const agenda = await openAgenda(page);
  let pocket = page.getByTestId("pocket");
  await agenda.getByText("Take an envelope", { exact: true }).click();
  const card = agenda.getByTestId("verb-talk-take_bribe");
  await expect(card.getByTestId("verb-talk-with")).toBeVisible();
  const open = card.getByTestId("verb-talk-open");
  await open.evaluate((el) => el.scrollIntoView({ block: "center" }));
  await open.click();
  const box = page.getByTestId("talk-panel");
  await expect(box).toBeVisible();
  const take = box.getByTestId("talk-topic-choice.bribe.take");
  await expect(take.getByTestId("talk-costs")).toContainText("chance it comes out within a year");
  await expect(take.getByTestId("talk-costs")).toContainText("a crime");
  await take.getByRole("button").click();
  await expect(box.getByTestId("talk-outcome")).toContainText(/slipped you an envelope/);
  await box.getByTestId("talk-result-why").locator("summary").click();
  await expect(box.getByTestId("talk-result-why")).toContainText("wants something for it");
  await expect(box).not.toContainText("⟦");
  await box.getByTestId("talk-goodbye").getByRole("button").click();
  await expect(box).toHaveCount(0);

  // Months pass. The story breaks: the pocket lists it under scandals.
  let turn = 1;
  for (; turn <= 30; turn++) {
    if ((await page.getByTestId("pocket").count()) > 0) await closeSheet(page, "pocket");
    await endTurn(page, turn);
    pocket = await openPocket(page);
    if ((await pocket.getByTestId("scandal").count()) > 0) break;
  }
  await expect(pocket.getByTestId("pocket-legal").getByTestId("scandal").first()).toContainText("a cash bribe");
  await closeSheet(page, "pocket");

  // The newspaper office runs it, with the outlet and its framing (from the
  // pocket's Places tab, DEC-088).
  await showPlaces(page);
  const office = (await page.getByTestId("spot-news_office").count()) > 0 ? "spot-news_office" : "spot-town_hall";
  const spot = page.getByTestId(office).first();
  await spot.evaluate((el) => el.scrollIntoView({ block: "center" }));
  await spot.click();
  const paper = page.getByTestId("newspaper");
  await expect(paper).toBeVisible();
  const story = paper.getByTestId("article").filter({ hasText: "a cash bribe" }).first();
  await expect(story).toBeVisible();
  await expect(story).toContainText(/hostile|straight|sympathetic/);
  await expect(paper).not.toContainText("⟦");
  await closeSheet(page, "place-panel");

  // Prosecutors open a case: a legal-response decision waits in the pocket.
  for (; turn <= 40; turn++) {
    pocket = await openPocket(page);
    if ((await pocket.getByTestId("pocket-needs").getByText("The investigation").count()) > 0) break;
    await closeSheet(page, "pocket");
    await endTurn(page, turn + 1);
  }
  const item = pocket.getByTestId("pocket-needs").locator("li").filter({ hasText: "The investigation" }).first();
  const choose = item.getByRole("button", { name: "Choose" });
  await choose.evaluate((el) => el.scrollIntoView({ block: "center" }));
  await choose.click();
  const sheet = page.getByTestId("decision-sheet");
  await expect(sheet).toBeVisible();
  await expect(sheet.getByTestId("decision-situation")).toContainText("investigation into a cash bribe");
  const cooperate = sheet.getByTestId("option-cooperate");
  await cooperate.evaluate((el) => el.scrollIntoView({ block: "center" }));
  await cooperate.click();
  const after = sheet.getByTestId("decision-aftermath");
  await expect(after).toBeVisible();
  await expect(after).toContainText("You cooperated");
  await expect(sheet).not.toContainText("⟦");
  await closeSheet(page, "decision-sheet");
  // The case is on the pocket's legal card, with the answer.
  pocket = await openPocket(page);
  await expect(pocket.getByTestId("pocket-legal").getByTestId("case").first()).toContainText("under investigation");
});
