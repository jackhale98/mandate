import { expect, test, type Page } from "@playwright/test";
import { openFold, openPeople, openPocket } from "./pocket";

// The Notebook (DEC-091, DEC-113): what the player knows, under four tabs.
// Only what is known is listed: no placeholders, no meter, nothing counts how
// much has been learned (DEC-091, 2026-11-25). A search for something not
// known answers with the way to find out; following it fills the entry; the
// badge counts what is new until the Notebook is seen. "Start with local
// knowledge" is off by default. Desktop and phone.

const start = async (page: Page, seed: string, local = false) => {
  await page.goto("./");
  const toggle = page.getByTestId("local-knowledge-toggle");
  await expect(toggle).not.toBeChecked();
  await expect(page.getByTestId("local-knowledge")).toContainText("You grew up here");
  if (local) await toggle.check();
  await page.getByPlaceholder("e.g. 20260").fill(seed);
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
  await expect(page.locator(".town-street-stage[data-ready='1']")).toBeVisible({ timeout: 30_000 });
};

/** Nothing counts knowledge: no dots, no "Not known yet", no placeholder, no share of what is known. */
const expectNoChecklist = async (page: Page) => {
  const pocket = page.getByTestId("pocket");
  await expect(pocket.locator(".nb-dots, [data-testid='notebook-gap']")).toHaveCount(0);
  await expect(pocket).not.toContainText("Not known yet");
  await expect(pocket).not.toContainText(/[●○]/);
  await expect(pocket).not.toContainText(/\d+ of \d+ known/);
};

test("without local knowledge: no placeholders; a search for what is not known says how to find out; following it fills the entry; the badge rises, then clears when seen", async ({ page }, info) => {
  await start(page, "4242");
  await expect(page.getByTestId("notebook-badge")).toHaveCount(0);
  let pocket = await openPocket(page, "people");
  const nb = pocket.getByTestId("notebook");
  await expect(nb.getByTestId("notebook-new")).toHaveCount(0);
  await expectNoChecklist(page);
  // People known from the start are rows; opened, each has Talk and Go to them.
  await openPeople(page);
  const person = pocket.getByTestId("notebook-key-figures").getByTestId("notebook-person").first();
  await expect(person.getByTestId("talk-button")).toBeVisible();
  await expect(person.getByTestId("notebook-go-to")).toBeVisible();

  // What worries another neighbourhood is not known: nothing says so in the lists, but a search answers with the way to find out.
  await pocket.getByTestId("notebook-search-open").click();
  await pocket.getByTestId("notebook-search").fill("worries");
  const answer = pocket.getByTestId("notebook-gap-answer").filter({ hasText: /Canvass / }).first();
  await expect(answer).toContainText("You don't know this yet");
  const lead = ((await answer.getByTestId("notebook-lead").textContent()) ?? "").trim();
  const hood = /Canvass (.+?) to hear what worries people there/.exec(lead)?.[1] ?? "";
  expect(hood).not.toBe("");
  await pocket.getByTestId("notebook-search").fill(hood);
  await expect(pocket.getByTestId("notebook-results-gaps").getByTestId("notebook-gap-answer")).toHaveCount(1);

  // Follow it: it walks there and the street's panel offers the canvass.
  await answer.getByTestId("notebook-lead-go").click();
  const panel = page.getByTestId("place-panel");
  await expect(panel).toBeVisible();
  // Leave the pocket on Today, so what is learned is news when it next opens (on a wide screen the
  // place's panel covers the Notebook on the left while it is open).
  pocket = await openPocket(page, "today");
  await pocket.getByRole("button", { name: "Close", exact: true }).click();
  await expect(pocket).toHaveCount(0);
  await panel.getByTestId("form-canvass").locator(".btn-primary").click();
  await expect(page.getByTestId("toast")).toHaveText(/knocked on doors/);
  await expect(page.getByTestId("notebook-badge")).toBeVisible();
  if (info.project.name === "phone" && (await panel.isVisible())) await panel.getByRole("button", { name: "Close" }).first().click();

  // Today says so and links to the new entries; the switch carries the count.
  await page.getByTestId("pocket-button").click();
  pocket = page.getByTestId("pocket");
  await expect(pocket.getByTestId("pocket-tab-badge-notebook")).toBeVisible();
  await expect(pocket.getByTestId("today-notebook")).toContainText("You learned");
  await pocket.getByTestId("today-notebook-open").click();
  await expect(pocket.getByTestId("pocket-tab-notebook")).toHaveAttribute("data-on", "true");

  // It lands where the news is (the first tab with something new), and each tab with news carries its count.
  await expect(pocket.locator('.nb-tab[aria-selected="true"] .badge')).toBeVisible();
  await expect(pocket.getByTestId("notebook-tab-badge-places")).toHaveText("1");
  await pocket.getByTestId("notebook-tab-places").click();
  const row = pocket.locator(".hood").filter({ has: page.locator(".rw-fold-name", { hasText: hood }) });
  await expect(row).toContainText("Worried most about");
  await openFold(row.locator(".rw-fold"));
  await expect(row).toContainText(/Top worry|Worries/);

  // Seeing it clears the count (the engine's), on the bar and on the switch.
  await expect(page.getByTestId("notebook-badge")).toHaveCount(0);
  await expect(pocket.getByTestId("pocket-tab-badge-notebook")).toHaveCount(0);
  await expectNoChecklist(page);

  // Looked at once, it is no longer new.
  await pocket.getByRole("button", { name: "Close", exact: true }).click();
  await expect(pocket).toHaveCount(0);
  pocket = await openPocket(page, "places");
  await expect(pocket.getByTestId("notebook-tab-badge-places")).toHaveCount(0);
  await expect(pocket.getByTestId("notebook-new")).toHaveCount(0);
});

test("with local knowledge: the notebook starts full and nothing is new", async ({ page }) => {
  await start(page, "4242", true);
  await expect(page.getByTestId("notebook-badge")).toHaveCount(0);
  const pocket = await openPocket(page, "town");
  await expect(pocket.getByTestId("pocket-tab-badge-notebook")).toHaveCount(0);
  // Who runs things is known (the option gives it; no profile does).
  await expect(pocket.getByTestId("town-broker")).toBeVisible();
  // Every neighbourhood's worries, as a local knows them, on its row.
  await openPocket(page, "places");
  const hoods = pocket.getByTestId("town-hood");
  const n = await hoods.count();
  expect(n).toBeGreaterThan(1);
  await expect(hoods.filter({ hasText: "Worried most about" })).toHaveCount(n);
  await expect(pocket.getByTestId("notebook-new")).toHaveCount(0);
  await expectNoChecklist(page);
});
