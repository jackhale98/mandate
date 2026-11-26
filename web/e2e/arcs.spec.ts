import { expect, test, type Page } from "@playwright/test";
import { playTo, startRun } from "./midrun";
import { openPocket, openRow, pocketMenu } from "./pocket";

// The arc sheet (STAGED_ARCS A3; DEC-113 §3.9): every kind of beat the player
// takes (a visit, a talk, an ask, a decision) is taken from the arc's sheet
// through lists and sheets only: no walking in the street and no map, as quick
// mode will play (DEC-051 parity). The arc is the budget round, the standing
// arc every town's session raises (A2): on turn 9 its estimates (a visit to
// read the papers, a word with whoever proposes the budget), on turn 10 the
// positions (a colleague's ask, the player's decision). Below "known" the
// sheet shows a lead only: the arc views' fixtures hold that case
// (src/components/ArcSheet.test.tsx), as no arc the base content raises is
// hidden yet. Desktop and phone.

/** The arc's sheet, opened from its row on Today (or still open: it follows each command). */
async function arcSheet(page: Page) {
  const sheet = page.getByTestId("arc-sheet");
  if ((await sheet.count()) === 0) {
    const pocket = await openPocket(page, "today");
    // The budget round's row: other arcs (the zoning fight) may run beside it.
    await pocket.locator("[data-testid^='arc-row-']").filter({ hasText: /budget round/i }).locator(".rw-main").first().click();
  }
  await expect(sheet).toBeVisible();
  await expect(sheet).not.toContainText("⟦");
  return sheet;
}

test("every beat kind is taken from the arc's sheet, without walking or the map", async ({ page }) => {
  test.setTimeout(240_000);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await startRun(page, "4242");
  await playTo(page, 9);
  const taken = new Set<string>();
  // The map is never opened on the way.
  const map = page.getByTestId("town-map-overlay");

  // Where it stands, its clock, what can be done, who is in it.
  let sheet = await arcSheet(page);
  await expect(sheet.getByTestId("arc-clock")).toContainText(/Moves on/);
  await expect(sheet.getByTestId("arc-stage")).not.toBeEmpty();
  await expect(sheet.getByTestId("arc-cast-list").getByTestId("arc-cast").first()).toBeVisible();

  // A visit: its button goes to the place (free), whose panel offers the beat's own verb.
  const visit = sheet.locator("[data-kind='visit'][data-available='true']").first();
  const verb = (await visit.getAttribute("data-verb")) ?? "";
  expect(verb).not.toBe("");
  await visit.locator(".rw-end button").click();
  const panel = page.getByTestId("place-panel");
  await expect(panel).toBeVisible();
  const action = panel.getByTestId(`verb-${verb}`);
  await action.evaluate((el) => el.scrollIntoView({ block: "center" }));
  await action.getByRole("button").click();
  await expect(page.getByTestId("toast")).toBeVisible({ timeout: 15_000 });
  await expect(page.getByTestId("toast")).not.toContainText("⟦");
  taken.add("visit");
  await panel.getByRole("button", { name: "Close" }).first().click();

  // A talk: its button opens the conversation with them, which offers the arc's choice.
  sheet = await arcSheet(page);
  const talk = sheet.locator("[data-kind='talk']").first();
  const topic = (await talk.getAttribute("data-topic")) ?? "";
  expect(topic).not.toBe("");
  await talk.locator(".rw-end button").click();
  const dlg = page.getByTestId("talk-panel");
  await expect(dlg).toBeVisible();
  if ((await dlg.getByTestId("talk-go").count()) > 0) await dlg.getByTestId("talk-go").locator("button.dlg-choice").click();
  const choice = dlg.getByTestId(`talk-topic-${topic}`);
  await expect(choice).toBeVisible();
  await choice.locator("button.dlg-choice").click();
  await expect(page.getByTestId("talk-reply-live")).not.toBeEmpty({ timeout: 15_000 });
  taken.add("talk");
  await dlg.getByTestId("talk-goodbye").locator("button.dlg-choice").click();
  await expect(dlg).toHaveCount(0);

  // The positions: a colleague's ask, and the player's own decision.
  await playTo(page, 10);
  sheet = await arcSheet(page);
  const ask = sheet.getByTestId("arc-beats").locator("[data-status='offered']").first();
  await expect(ask).toBeVisible();
  // Opened in place, its price read first, and answered there.
  await openRow(ask);
  await ask.getByRole("button", { name: /^Accept: / }).click();
  await expect(page.getByTestId("toast")).toContainText("You said yes:");
  taken.add("ask");

  sheet = await arcSheet(page);
  const decision = sheet.locator("[data-kind='decision']").first();
  await decision.locator(".rw-end button").click();
  const dsheet = page.getByTestId("decision-sheet");
  await expect(dsheet).toBeVisible();
  await dsheet.evaluate((el) => Promise.all(el.getAnimations().map((a) => a.finished)));
  const option = dsheet.getByTestId("decision-options").locator("button:enabled").first();
  await option.evaluate((el) => el.scrollIntoView({ block: "center" }));
  await option.click();
  await expect(dsheet.getByTestId("decision-aftermath")).toBeVisible();
  taken.add("decision");
  await dsheet.getByRole("button", { name: "Close" }).click();

  expect([...taken].sort()).toEqual(["ask", "decision", "talk", "visit"]);
  await expect(map).toHaveCount(0);

  // What happened in it is kept: the sheet reopens from the story so far, with how it came to this.
  await (await pocketMenu(page)).getByTestId("open-story").click();
  const story = page.getByTestId("story-sheet");
  await expect(story).toBeVisible();
  await story.getByTestId("story-open-arc").first().click();
  sheet = page.getByTestId("arc-sheet");
  await expect(sheet).toBeVisible();
  await expect(sheet.getByRole("heading", { level: 2 })).not.toBeEmpty();
  await expect(sheet).not.toContainText("⟦");
});
