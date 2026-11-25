import { expect, type Locator, type Page } from "@playwright/test";

// The pocket (DEC-113): two sides under one switch, Today and the Notebook;
// the Notebook has one row of four tabs (People, Places, Town, Policies),
// which took in the old Town and Places tabs. On a wide screen Today is on
// the right and the Notebook open on the left, both inside the pocket's
// element. Helpers shared by the specs; their names and arguments are kept
// from the four-tab pocket ("town" and "places" open the Notebook there).
export type PocketTab = "today" | "town" | "places" | "notebook" | "people" | "policies";

const BOOK = new Set<PocketTab>(["town", "places", "people", "policies"]);

/** Open the pocket (if it is not open) on a side or a Notebook tab; returns the pocket. */
export async function openPocket(page: Page, tab: PocketTab = "today") {
  // A pocket that is sliding away is about to be gone.
  await expect(page.locator(".pocket.sheet-closing")).toHaveCount(0);
  if ((await page.getByTestId("pocket").count()) === 0) await page.getByTestId("pocket-button").click();
  const pocket = page.getByTestId("pocket");
  await expect(pocket).toBeVisible();
  const side = tab === "today" ? "today" : "notebook";
  const sw = pocket.getByTestId(`pocket-tab-${side}`);
  if ((await sw.getAttribute("data-on")) !== "true") await sw.click();
  await expect(sw).toHaveAttribute("data-on", "true");
  if (BOOK.has(tab)) {
    // A search typed earlier gives way to the tabs.
    const closeSearch = pocket.getByTestId("notebook-search-close");
    if ((await closeSearch.count()) > 0) await closeSearch.click();
    const t = pocket.getByTestId(`notebook-tab-${tab}`);
    if ((await t.getAttribute("aria-selected")) !== "true") await t.click();
    await expect(t).toHaveAttribute("aria-selected", "true");
  }
  return pocket;
}

/** Open a fold (a folded section, a neighbourhood) if it is closed. */
export async function openFold(fold: Locator) {
  const head = fold.locator(":scope > h3 > button.rw-fold-head").first();
  if ((await head.getAttribute("aria-expanded")) !== "true") await head.click();
  await expect(head).toHaveAttribute("aria-expanded", "true");
}

/** Open a row in place (an ask, a person, a bill) if it is closed. */
export async function openRow(row: Locator) {
  const main = row.locator(":scope > .rw-row > .rw-main").first();
  if ((await main.getAttribute("aria-expanded")) !== "true") await main.click();
  await expect(main).toHaveAttribute("aria-expanded", "true");
}

/** The Notebook's Places tab: every place in town as a button (the text path to every place, DEC-051), every neighbourhood opened. */
export async function showPlaces(page: Page) {
  const pocket = await openPocket(page, "places");
  await expect(pocket.getByRole("tabpanel", { name: /Places/ })).toBeVisible();
  // The town's places have arrived (the panel is there from the first frame).
  await expect(pocket.getByTestId("town-hood").first()).toBeVisible();
  for (const hood of await pocket.getByTestId("town-hood").all()) await openFold(hood.locator(".rw-fold"));
  return pocket;
}

/** The Notebook's People tab, its "In office" section open and its first person opened (Talk and Go to them). */
export async function openPeople(page: Page) {
  const pocket = await openPocket(page, "people");
  const office = pocket.getByTestId("notebook-key-figures");
  await expect(office).toBeVisible();
  await openFold(office);
  const first = office.locator("[data-office='true']").first();
  await openRow(first);
  return pocket;
}

/** The people in office (the executive, the members of the legislature), each a row; the first is opened by `openPeople`. */
export function officeFigures(pocket: Locator): Locator {
  return pocket.getByTestId("notebook-key-figures").locator("[data-office='true']");
}

/** The pocket's ⋯ menu, open: the maps, the story so far, skip ahead, Legacy, Save & quit, Report a problem, Privacy. */
export async function pocketMenu(page: Page) {
  await expect(page.locator(".pocket.sheet-closing")).toHaveCount(0);
  if ((await page.getByTestId("pocket").count()) === 0) await page.getByTestId("pocket-button").click();
  await expect(page.getByTestId("pocket")).toBeVisible();
  const more = page.getByTestId("pocket").getByTestId("pocket-more");
  if ((await more.getAttribute("aria-expanded")) !== "true") await more.click();
  const menu = page.getByTestId("pocket-menu");
  await expect(menu).toBeVisible();
  return menu;
}

/** Today's "Take an action" (the agenda), open. */
export async function openAgenda(page: Page) {
  const pocket = await openPocket(page, "today");
  const agenda = pocket.getByTestId("agenda");
  await openFold(agenda);
  return agenda;
}
