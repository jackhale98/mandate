import { expect, type Locator, type Page } from "@playwright/test";

// The pocket (DEC-088 main screen): a sheet with tabs (Today, Town, Places,
// Notebook; people are the Notebook's People section). Helpers shared by the specs.
export type PocketTab = "today" | "town" | "places" | "notebook";

/** Open the pocket (if it is not open) on a tab; returns the pocket. */
export async function openPocket(page: Page, tab: PocketTab = "today") {
  // A pocket that is sliding away is about to be gone.
  await expect(page.locator(".pocket.sheet-closing")).toHaveCount(0);
  if ((await page.getByTestId("pocket").count()) === 0) await page.getByTestId("pocket-button").click();
  const pocket = page.getByTestId("pocket");
  await expect(pocket).toBeVisible();
  const t = pocket.getByTestId(`pocket-tab-${tab}`);
  if ((await t.getAttribute("aria-selected")) !== "true") await t.click();
  await expect(t).toHaveAttribute("aria-selected", "true");
  return pocket;
}

/** The Places tab: every place in town as a button (the text path to every place, DEC-051). */
export async function showPlaces(page: Page) {
  const pocket = await openPocket(page, "places");
  await expect(pocket.getByRole("tabpanel", { name: /Places/ })).toBeVisible();
  return pocket;
}

/** The Notebook's People section, the single place for people: the key figures pinned, then everyone else you know. */
export async function openPeople(page: Page) {
  const pocket = await openPocket(page, "notebook");
  const t = pocket.getByTestId("notebook-tab-people");
  if ((await t.getAttribute("aria-selected")) !== "true") await t.click();
  await expect(t).toHaveAttribute("aria-selected", "true");
  await expect(pocket.getByTestId("notebook-key-figures")).toBeVisible();
  return pocket;
}

/** The key figures in office (the executive, then the members of the legislature), each a card with Talk and Go to them. */
export function officeFigures(pocket: Locator): Locator {
  return pocket.getByTestId("notebook-key-figures").locator("[data-office='true']");
}
