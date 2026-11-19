import { expect, type Page } from "@playwright/test";

// The pocket (DEC-088 main screen): a sheet with tabs (Today, Town, Places,
// People). Helpers shared by the specs.
export type PocketTab = "today" | "town" | "places" | "people";

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
