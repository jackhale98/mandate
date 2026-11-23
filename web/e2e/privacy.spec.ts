import { expect, test } from "./csp";

// The privacy page (built beside the landing page) loads under the CSP, says what
// the game stores, and the game links to it from the title screen.
test("privacy page: content, and the title screen links to it", async ({ page }) => {
  await page.goto("../privacy.html");
  await expect(page.getByRole("heading", { level: 1, name: "Privacy" })).toBeVisible();
  await expect(page.getByText("does not send anything about you")).toBeVisible();
  await expect(page.getByRole("link", { name: "Third-party notices" })).toBeVisible();

  await page.goto("./");
  const link = page.getByTestId("privacy-link-title");
  await expect(link).toBeVisible();
  await expect(page.getByText("Mandate is a work of fiction.")).toBeVisible();
  const [popup] = await Promise.all([page.waitForEvent("popup"), link.click()]);
  await expect(popup.getByRole("heading", { level: 1, name: "Privacy" })).toBeVisible();
});
