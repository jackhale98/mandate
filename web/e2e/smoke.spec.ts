import { expect, test } from "./csp";
import type { Page } from "@playwright/test";
import { openAgenda, openPocket, pocketMenu, showPlaces } from "./pocket";

// The town is the main screen (DEC-088: its street, full screen); the pocket
// (a drawer from the bottom bar: Today and the Notebook, DEC-113) holds the agenda, the town card,
// every place, and links to the national views.

const readHash = async (page: Page) => {
  const menu = await pocketMenu(page);
  await menu.getByRole("button", { name: "Overview", exact: true }).click();
  return page.getByText(/state [0-9a-f]{16}/).textContent();
};

/** The state's hash once it has settled. Opening the Notebook marks what is new as seen, which is a real command (it
 *  changes the state and is saved), so whether it has happened yet depended on timing (CI, 2026-11-25). Look at the
 *  Notebook first, so every read is of a state with nothing new left unseen, then read until two reads agree. */
const stateHash = async (page: Page) => {
  await openPocket(page, "notebook");
  await page.waitForTimeout(800);
  await page.keyboard.press("Escape");
  let last = await readHash(page);
  for (let i = 0; i < 5; i++) {
    await page.keyboard.press("Escape");
    await page.waitForTimeout(400);
    const now = await readHash(page);
    if (now === last) return now;
    last = now;
  }
  return last;
};

// M0 exit gate: start a game, end a turn, save, reload offline, resume (M0-09).
test("new game, end turn, save, resume offline", async ({ page, context }) => {
  await page.goto("./");
  await expect(page.getByRole("heading", { name: "Mandate" })).toBeVisible();
  await page.getByPlaceholder("e.g. 20260").fill("4242");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByText("Turn 0")).toBeVisible();
  await expect(page.getByTestId("town")).toBeVisible();
  await page.getByRole("button", { name: "End turn" }).click();
  await expect(page.getByText("Turn 1")).toBeVisible();
  const hash = await stateHash(page);
  const menu = await pocketMenu(page);
  await menu.getByRole("button", { name: "Save & quit" }).click();
  await expect(page.getByRole("button", { name: "Continue" })).toBeVisible();

  await page.evaluate(async () => {
    await navigator.serviceWorker.ready;
  });
  await page.reload();
  await page.evaluate(async () => {
    await navigator.serviceWorker.ready;
  });
  await context.setOffline(true);
  await page.reload();
  await page.getByRole("button", { name: "Continue" }).first().click();
  await expect(page.getByText("Turn 1")).toBeVisible();
  expect(await stateHash(page)).toBe(hash);
  await context.setOffline(false);
});

// M1: the generated country renders and towns can be inspected without the
// map (accessible list path, DEC-051). Both are reached from the pocket.
test("map renders and a town can be inspected from the list", async ({ page }) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("777");
  await page.getByRole("button", { name: "Begin" }).click();
  await (await pocketMenu(page)).getByRole("button", { name: "Map", exact: true }).click();
  const canvas = page.getByRole("img", { name: /Drag to pan/ });
  await expect(canvas).toBeVisible();
  const box = await canvas.boundingBox();
  expect(box?.width ?? 0).toBeGreaterThan(200);
  // The pocket's Map opens the map overlay on the country; ✕ closes it.
  await expect(page.getByTestId("map-scope-country")).toHaveAttribute("aria-selected", "true");
  await page.getByTestId("town-map-close").click();
  await expect(page.getByTestId("town-map-overlay")).toHaveCount(0);
  await (await pocketMenu(page)).getByRole("button", { name: "Overview", exact: true }).click();
  await page.locator(".region-accordion > li > .row-btn").first().click();
  await page.locator(".town-list .row-btn").first().click();
  await expect(page.getByRole("heading", { name: "Resources" })).toBeVisible();
  await expect(page.getByText("Town hall", { exact: true })).toBeVisible();
  // The sheet is modal on phones (the screen behind is dimmed): close it first.
  await page.locator("aside.sheet").getByRole("button", { name: "Close" }).click();
  await expect(page.locator("aside.sheet")).toHaveCount(0);
  await page.getByRole("button", { name: /Back to town/ }).click();
  await expect(page.getByTestId("town")).toBeVisible();
});

// M3 + M5a: draft at the town hall, introduce (auto-routed to the council
// chamber), lobby from the exact whip list, and survive a reload mid-turn.
test("draft, introduce, lobby and resume mid-turn", async ({ page }) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("31337");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByTestId("hud-slots")).toHaveAttribute("aria-label", "3 slots left this turn");
  await showPlaces(page);
  await expect(page.getByTestId("you-are-here")).toContainText("You are here");

  // Tap the town hall: the player walks there (free) and its panel opens.
  await page.getByTestId("spot-town_hall").click();
  const panel = page.getByTestId("place-panel");
  await expect(panel.getByTestId("you-are-here-panel")).toBeVisible();
  await expect(panel.getByTestId("town-data")).toBeVisible();
  const catalog = panel.getByTestId("catalog");
  await catalog.getByRole("button", { name: /Neighbourhood parks|Parks/ }).first().click();
  await catalog.getByRole("button", { name: "Create draft" }).click();
  await expect(page.getByTestId("toast")).toHaveText(/Draft created/);
  // Introducing is done in the council chamber: one tap walks there and acts.
  await panel.getByTestId("bills").getByRole("button", { name: "Introduce to council" }).click();
  await expect(page.getByTestId("toast")).toHaveText(/Bill introduced/);
  // The street walks the player across; the label at the top names the place on arrival.
  await expect(page.getByTestId("where-place")).toHaveText("Council chamber", { timeout: 30_000 });

  // In the chamber the whip count is exact, member by member.
  const whip = panel.getByTestId("whip");
  await expect(whip.getByText(/Expected yes/)).toBeVisible();
  // Lobbying is a conversation (M6 note 29): "Talk to them" opens it on the bill.
  await whip.getByTestId("whip-talk").first().click();
  const talk = page.getByTestId("talk-panel");
  await talk.getByRole("button", { name: /Make the case/ }).click();
  // The reply, page by page (Continue), then the outcome.
  for (let i = 0; i < 20 && (await talk.getByTestId("talk-outcome").count()) === 0; i++) {
    const more = talk.getByTestId("talk-continue");
    if ((await more.count()) > 0) await more.locator("button").click();
    else await page.waitForTimeout(300);
  }
  await expect(talk.getByTestId("talk-outcome")).toHaveText(/came round|not persuaded/);
  await talk.getByRole("button", { name: "Close" }).click();
  await expect(panel.getByTestId("slots")).toHaveText("1 slot left this turn");

  const hash = await stateHash(page);
  await page.reload();
  await page.getByRole("button", { name: "Continue" }).first().click();
  await expect(page.getByTestId("hud-slots")).toHaveAttribute("aria-label", "1 slot left this turn");
  await expect(page.getByTestId("where-place")).toHaveText("Council chamber");
  expect(await stateHash(page)).toBe(hash);
});

// M4 owner checkpoint: play a full local election (filing, campaign, polls,
// election night with an explanation). The race is followed at the party
// office; canvassing is chosen from the pocket's agenda and auto-routed.
test("a full local election", async ({ page }) => {
  test.setTimeout(180_000);
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("4040");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  const pocket = await openPocket(page);
  await expect(pocket.getByTestId("pocket-election")).toContainText("3 November 2020");
  await pocket.getByRole("button", { name: "Close" }).click();
  await showPlaces(page);
  await page.getByTestId("spot-party_office").click();
  await expect(page.getByTestId("internal-poll")).toBeVisible();
  const endTurn = page.getByRole("button", { name: "End turn" });
  // Whatever opens after a turn (election night) waits for the night between turns to pass, never covers it.
  await page.evaluate(() => {
    const w = window as unknown as { __overlap?: boolean };
    w.__overlap = false;
    new MutationObserver(() => {
      if (document.querySelector("[data-testid='election-night']") && document.querySelector("[data-testid='turn-passage']")) w.__overlap = true;
    }).observe(document.body, { childList: true, subtree: true });
  });
  for (let i = 0; i < 30 && (await page.getByTestId("my-race").count()) === 0; i++) {
    await endTurn.click();
    await expect(endTurn).toBeEnabled();
  }
  await expect(page.getByTestId("my-race")).toBeVisible();
  await expect(page.getByText("Campaign season", { exact: true })).toBeVisible();
  // Campaign: one canvass from the agenda (it walks to the streets), then let the weeks run.
  const agenda = await openAgenda(page);
  await agenda.getByText("Canvass a neighbourhood", { exact: true }).click();
  await expect(agenda.getByTestId("route")).toContainText("Streets of");
  const route = (await agenda.getByTestId("route").textContent()) ?? "";
  await agenda.getByRole("button", { name: "Go" }).click();
  await expect(page.getByTestId("toast")).toHaveText(/knocked on doors/);
  // On the streets the label names the neighbourhood alone (no place).
  await expect(page.getByTestId("where-place")).toHaveCount(0, { timeout: 30_000 });
  await expect.poll(async () => route.includes(((await page.getByTestId("where-zone").textContent()) ?? "-").trim()), { timeout: 30_000 }).toBe(true);
  // A lost seat ends Act 1 (DEC-006, `endings.jsonc`): the bar then offers the epilogue instead of another turn.
  const epilogue = page.getByTestId("open-epilogue");
  for (let i = 0; i < 12 && (await page.getByTestId("election-night").count()) === 0 && (await epilogue.count()) === 0; i++) {
    await endTurn.click();
    await expect(endTurn.or(epilogue)).toBeEnabled();
  }
  const night = page.getByTestId("election-night");
  await expect(night).toBeVisible();
  expect(await page.evaluate(() => (window as unknown as { __overlap?: boolean }).__overlap)).toBe(false);
  await expect(night.getByText(/You (won|lost)\./)).toBeVisible();
  const lost = (await night.getByText("You lost.").count()) > 0;
  await expect(night.getByText("Why it went this way").first()).toBeVisible();
  await night.getByRole("button", { name: "Continue" }).last().click();
  await expect(night).toHaveCount(0);
  if (lost) {
    await epilogue.click();
    await expect(page.getByTestId("epilogue-ending")).toHaveAttribute("data-ending", "ending.lost_seat");
  } else await expect(endTurn).toBeEnabled();
});

// M5b: the town card shows the town's character (DEC-077 §1; in the pocket's
// Town tab, DEC-088); a place lists who is there; meeting a passer-by opens
// their character sheet (DEC-031).
test("town character, people at a place, and a character sheet", async ({ page }) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("2718");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  const card = (await openPocket(page, "town")).getByTestId("town-card");
  await expect(card).toBeVisible();
  await expect(card.getByTestId("town-industry")).not.toHaveText("—");
  await expect(card.getByTestId("town-rival")).not.toHaveText("—");

  await showPlaces(page);
  await page.getByTestId("spot-market").first().click();
  const panel = page.getByTestId("place-panel");
  await expect(panel.getByTestId("you-are-here-panel")).toBeVisible();
  const people = panel.getByTestId("people-here");
  await expect(people).toBeVisible();
  // Someone who works here (the shopkeeper) has a sheet already.
  await people.getByTestId("person").first().click();
  const sheet = page.getByTestId("character-sheet");
  await expect(sheet).toBeVisible();
  await expect(sheet.getByTestId("character-relationship")).toBeVisible();
  await sheet.getByRole("button", { name: "Close" }).click();
  await expect(sheet).toHaveCount(0);
  // A passer-by becomes someone you know.
  const meet = people.getByTestId("meet");
  if ((await meet.count()) > 0) {
    await meet.first().click();
    await expect(page.getByTestId("toast")).toHaveText(/You introduced yourself/);
    await expect(sheet).toBeVisible();
    await expect(sheet.getByTestId("character-memories")).toContainText("first met");
  }
});
