import { expect, test } from "./csp";
import { showPlaces } from "./pocket";
import { seedFor } from "./seeds";
import { useWideFont } from "./wide-font";

// What a bill does (DEC-111): a draft's card states the engine's estimate in
// sentences (what it does, what it costs, who gains and loses, when and how
// sure), with no raw key, and the lines fit a phone at 320, 360 and 390 px
// with a wide face. `IMPACT_SHOT=<path>` saves the 390 px view.
test.use({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2 });

test("a draft says what it will do, and the lines fit a phone", async ({ page }) => {
  await useWideFont(page);
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill(seedFor("fresh_colleague_and_bill_before_council"));
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await showPlaces(page);
  await page.getByTestId("spot-town_hall").click();
  const panel = page.getByTestId("place-panel");
  const catalog = panel.getByTestId("catalog");
  await catalog.getByRole("button", { name: /Affordable housing/ }).first().click();
  await catalog.getByRole("button", { name: "Create draft" }).click();
  await expect(page.getByTestId("toast")).toHaveText(/Draft created/);

  const impact = panel.getByTestId("bills").getByTestId("impact").first();
  await expect(impact).toBeVisible();
  await expect(impact).toContainText("What it does");
  // The bill's own count, by arithmetic; its cost; when; how sure.
  await expect(impact.getByTestId("impact-lines").locator("li").first()).toHaveText(/homes started\.$/);
  await expect(impact.getByTestId("impact-cost")).toHaveText(/^Costs .+ a year, .+% of the budget\./);
  await expect(impact.getByTestId("impact-when")).toContainText(/Fair estimate\. The statistician's figures\./);
  // No raw key and no unfilled placeholder reaches the screen.
  const text = (await impact.textContent()) ?? "";
  expect(text).not.toMatch(/⟦|\{\s*\$|impact-/);

  for (const width of [390, 360, 320]) {
    await page.setViewportSize({ width, height: 844 });
    await impact.scrollIntoViewIfNeeded();
    const problems = await impact.evaluate((el) => {
      const out: string[] = [];
      const card = el.getBoundingClientRect();
      if (card.right > window.innerWidth + 0.5) out.push(`the card ends at ${card.right} of ${window.innerWidth}`);
      if (el.scrollWidth > el.clientWidth + 1) out.push(`the card's text is ${el.scrollWidth} wide in ${el.clientWidth}`);
      for (const n of el.querySelectorAll("p, li, h4")) {
        const r = n.getBoundingClientRect();
        if (r.right > card.right + 0.5 || r.left < card.left - 0.5) out.push(`a line leaves the card: ${n.textContent?.slice(0, 40)}`);
        if (n.scrollWidth > n.clientWidth + 1) out.push(`a line is clipped: ${n.textContent?.slice(0, 40)}`);
      }
      return out;
    });
    expect(problems, `at ${width}px`).toEqual([]);
    if (width === 390 && process.env.IMPACT_SHOT) {
      // The card from its heading, with nothing over it.
      await page.getByTestId("toast").getByRole("button").click();
      await expect(page.getByTestId("toast")).toHaveCount(0);
      await impact.evaluate((el) => el.closest("article")?.scrollIntoView({ block: "start" }));
      await page.screenshot({ path: process.env.IMPACT_SHOT });
    }
  }
});
