import { expect, test } from "@playwright/test";
import { openPocket, showPlaces } from "./pocket";

// Places with purpose (DEC-079 note, PL1): the factory has things to do.
// Walk there from the Places tab, tour the works (a verb of the generic
// visit action, from content), and find what was learned in the notebook:
// the town's manufacturing, in words, and the conditions at the works.
// SEED's town has a factory (a medium world, as the title screen starts).
const SEED = "2";

test("walk to the factory, tour the works, and see what was learned", async ({ page }) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill(SEED);
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();

  // The factory is no longer "nothing to do here yet": its spot lists the verbs.
  const places = await showPlaces(page);
  const spot = places.getByTestId("spot-factory").first();
  await expect(spot).toContainText("Tour the works");
  await spot.click();
  const panel = page.getByTestId("place-panel");
  await expect(panel.getByTestId("you-are-here-panel")).toBeVisible();
  await expect(panel.getByText("Nothing to do here yet.")).toHaveCount(0);

  // The verbs, each with its cost; the photo op shows its risk before you commit.
  const verbs = panel.getByTestId("place-verbs");
  await expect(verbs).toBeVisible();
  await expect(verbs.getByTestId("verb-verb.photo_op_gate")).toContainText(/Risk \d+%/);
  const tour = verbs.getByTestId("verb-verb.tour_works");
  await expect(tour).toContainText("1 slot");
  await tour.getByRole("button").click();
  await expect(page.getByTestId("toast")).toHaveText(/You toured/);
  await expect(page.getByTestId("hud-slots")).toHaveAttribute("aria-label", "2 slots left this turn");

  // Something learned: the notebook's places list the town's industry and the works' conditions.
  await expect(page.getByTestId("notebook-badge")).toBeVisible();
  if (await panel.isVisible()) await panel.getByRole("button", { name: "Close" }).first().click();
  const pocket = await openPocket(page, "notebook");
  const nb = pocket.getByTestId("notebook");
  await nb.getByTestId("notebook-tab-places").click();
  await expect(nb.locator('[data-testid="notebook-entry"][data-subject="sector"]')).toContainText(/Manufacturing in .+: (taking people on|steady|letting people go)/);
  await expect(nb.locator('[data-testid="notebook-entry"][data-subject="site"]')).toContainText("Conditions at");
});
