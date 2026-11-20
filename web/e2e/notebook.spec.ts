import { expect, test, type Page } from "@playwright/test";
import { openPocket } from "./pocket";

// The notebook (DEC-091, M6 K5): what the player knows, by tab, with
// placeholders and the leads that would reveal them. Following a lead fills
// an entry; the badge counts what is new until the notebook is seen. "Start
// with local knowledge" is off by default. Desktop and phone.

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

test("without local knowledge: placeholders with leads; following one fills the entry; the badge rises, then clears when seen", async ({ page }) => {
  await start(page, "4242");
  await expect(page.getByTestId("notebook-badge")).toHaveCount(0);
  let pocket = await openPocket(page, "notebook");
  const nb = pocket.getByTestId("notebook");
  await expect(nb.getByRole("heading", { level: 3 }).first()).toContainText("What you know about");
  await expect(nb.getByTestId("notebook-new")).toHaveCount(0);

  // People: who really runs the town is not known; a lead says where to ask.
  await expect(nb.getByTestId("notebook-tab-people")).toHaveAttribute("aria-selected", "true");
  const broker = nb.locator('[data-testid="notebook-gap"][data-subject="broker"]');
  await expect(broker).toContainText("Not known yet");
  await expect(broker.getByTestId("notebook-lead")).toBeVisible();
  // People known from the start are cards (the key figures pinned first), with Talk and Go to them.
  const person = nb.getByTestId("notebook-person").first();
  await expect(person).toBeVisible();
  await expect(person.getByTestId("talk-button")).toBeVisible();
  await expect(person.getByTestId("notebook-go-to")).toBeVisible();

  // Worries by neighbourhood: the ward is known from the start; another is a placeholder.
  await nb.getByTestId("notebook-tab-issues").click();
  const gap = nb.getByTestId("notebook-hoods").getByTestId("notebook-gap").first();
  await expect(gap).toContainText("Not known yet");
  await expect(gap.getByTestId("notebook-lead")).toContainText("Canvass ");
  const title = ((await gap.getByRole("heading").textContent()) ?? "").trim();
  expect(title).toMatch(/^What .+ worries about$/);
  const hoodGaps = await nb.getByTestId("notebook-hoods").getByTestId("notebook-gap").count();

  // The search finds it and nothing else in this tab.
  const hood = title.replace(/^What (.+) worries about$/, "$1");
  await nb.getByTestId("notebook-search").fill(hood);
  await expect(nb.getByTestId("notebook-hoods").locator("li.nb-item")).toHaveCount(1);
  await nb.getByTestId("notebook-search").fill("");

  // Follow the lead: it walks there and the street's panel offers the canvass.
  await gap.getByTestId("notebook-lead-go").click();
  const panel = page.getByTestId("place-panel");
  await expect(panel).toBeVisible();
  await panel.getByTestId("form-canvass").locator(".btn-primary").click();
  await expect(page.getByTestId("toast")).toHaveText(/knocked on doors/);
  await expect(page.getByTestId("notebook-badge")).toBeVisible();
  if ((await page.getByTestId("pocket").count()) > 0) await page.getByTestId("pocket").getByRole("button", { name: "Close" }).first().click();
  if (await panel.isVisible()) await panel.getByRole("button", { name: "Close" }).first().click();

  // Today says so and links to the new entries; the notebook's tab carries the badge.
  // (The pocket button reopens the tab it was left on, the notebook, which
  // would see them at once; come in by the skip link to Places instead.)
  await page.getByTestId("skip-to-places").focus();
  await page.keyboard.press("Enter");
  pocket = page.getByTestId("pocket");
  await expect(pocket.getByTestId("pocket-tab-places")).toHaveAttribute("aria-selected", "true");
  await pocket.getByTestId("pocket-tab-today").click();
  await expect(pocket.getByTestId("pocket-tab-badge-notebook")).toBeVisible();
  await expect(pocket.getByTestId("today-notebook")).toContainText("You learned");
  await pocket.getByTestId("today-notebook-open").click();
  await expect(pocket.getByTestId("pocket-tab-notebook")).toHaveAttribute("aria-selected", "true");

  // It lands where the news is (the first tab with something new: the
  // resident met at the door, or the neighbourhood's worries); each tab with
  // news carries its count.
  await expect(nb.locator('.nb-tab[aria-selected="true"] .nb-tab-badge')).toBeVisible();
  await expect(nb.getByTestId("notebook-tab-badge-issues")).toHaveText("1");
  await nb.getByTestId("notebook-tab-issues").click();
  const filled = nb.getByTestId("notebook-entry").filter({ has: page.getByRole("heading", { name: title, exact: true }) });
  await expect(filled).toBeVisible();
  await expect(filled.getByTestId("notebook-new")).toBeVisible();
  await expect(filled).toContainText(/Top worry|Worries/);
  await expect(filled.getByTestId("notebook-from")).toContainText("Canvass");
  await expect(nb.getByTestId("notebook-hoods").getByTestId("notebook-gap")).toHaveCount(hoodGaps - 1);

  // Seeing it clears the badge (the engine's count), on the bar and on the tab.
  await expect(page.getByTestId("notebook-badge")).toHaveCount(0);
  await expect(pocket.getByTestId("pocket-tab-badge-notebook")).toHaveCount(0);
  await expect(pocket.getByTestId("today-notebook")).toHaveCount(0);

  // Looked at once, it is no longer new.
  await pocket.getByTestId("pocket-tab-today").click();
  await pocket.getByTestId("pocket-tab-notebook").click();
  await nb.getByTestId("notebook-tab-issues").click();
  await expect(filled).toBeVisible();
  await expect(nb.getByTestId("notebook-new")).toHaveCount(0);
});

test("with local knowledge: the notebook starts full and nothing is new", async ({ page }) => {
  await start(page, "4242", true);
  await expect(page.getByTestId("notebook-badge")).toHaveCount(0);
  const pocket = await openPocket(page, "notebook");
  await expect(pocket.getByTestId("pocket-tab-badge-notebook")).toHaveCount(0);
  const nb = pocket.getByTestId("notebook");
  // Who runs things is known (the option gives it; no profile does).
  const broker = nb.locator('[data-testid="notebook-entry"][data-subject="broker"]');
  await expect(broker).toBeVisible();
  await expect(broker).toHaveAttribute("data-level", "known");
  await expect(broker.getByTestId("notebook-from")).toContainText("Local knowledge");
  await expect(nb.locator('[data-testid="notebook-gap"][data-subject="broker"]')).toHaveCount(0);
  // Every neighbourhood's worries, as a local knows them.
  await nb.getByTestId("notebook-tab-issues").click();
  await expect(nb.getByTestId("notebook-hoods").getByTestId("notebook-entry").first()).toBeVisible();
  await expect(nb.getByTestId("notebook-hoods").getByTestId("notebook-gap")).toHaveCount(0);
  await expect(nb.getByTestId("notebook-new")).toHaveCount(0);
});
