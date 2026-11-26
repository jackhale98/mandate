import { expect, test, type Page } from "@playwright/test";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { pocketMenu } from "./pocket";

// The career card and stepping away (M6 note 23): Begin → the ⋯ menu → Step
// away from politics → one confirmation → the epilogue with the career card,
// marked unfinished → Copy (the clipboard holds the text block) → Share where
// the device has no share sheet (the image is saved, the text copied) → back
// to the title → paste the card's world code → read it → begin that world (the
// same character in the same town). The world's details come from the engine's
// own card of the same seed (crates/mandate-wasm/tests/career_card.rs). On a
// phone the screen is 390×844: no sideways scroll, every button at least 44 px.

interface Card {
  name: string;
  town: string;
  code: string;
  build: string;
}
const here = path.dirname(fileURLToPath(import.meta.url));
const fixture = JSON.parse(readFileSync(path.resolve(here, "../src/components/fixtures/card/first_turn.json"), "utf8")) as Card;

async function noSidewaysScroll(page: Page) {
  const wide = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(wide).toBeLessThanOrEqual(0);
}

async function bigEnough(page: Page, scope: ReturnType<Page["getByTestId"]>) {
  for (const b of await scope.getByRole("button").all()) {
    if (!(await b.isVisible())) continue;
    const box = await b.boundingBox();
    expect(box?.height ?? 0, (await b.textContent()) ?? "").toBeGreaterThanOrEqual(44);
  }
}

test("step away → the career card → Copy, Share's fallback → start from its code", async ({ page, context }, info) => {
  test.setTimeout(240_000);
  const phone = info.project.name === "phone";
  if (phone) await page.setViewportSize({ width: 390, height: 844 });
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  // No share sheet on this device: Share falls back to saving the image and copying the text.
  await page.addInitScript(() => {
    Object.defineProperty(Navigator.prototype, "share", { value: undefined, configurable: true });
    Object.defineProperty(Navigator.prototype, "canShare", { value: undefined, configurable: true });
  });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("3");
  await page.getByText("Small", { exact: true }).click();
  await page.getByRole("button", { name: "Begin", exact: true }).click();
  await expect(page.getByTestId("town")).toBeVisible({ timeout: 60_000 });
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();

  // Step away: one confirmation ("Keep going" leaves everything as it was).
  await (await pocketMenu(page)).getByTestId("open-step-away").click();
  const dialog = page.getByTestId("step-away-dialog");
  await expect(dialog).toBeVisible();
  await expect(dialog).toContainText("unfinished");
  await bigEnough(page, dialog);
  await dialog.getByTestId("step-away-no").click();
  await expect(dialog).toHaveCount(0);
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
  await (await pocketMenu(page)).getByTestId("open-step-away").click();
  await page.getByTestId("step-away-yes").click();

  // The epilogue, with its short text and the card, marked unfinished.
  const epilogue = page.getByTestId("epilogue");
  await expect(epilogue).toBeVisible({ timeout: 30_000 });
  await expect(epilogue.getByTestId("epilogue-ending")).toHaveAttribute("data-ending", "ending.stepped_away");
  const card = epilogue.getByTestId("career-card");
  await expect(card).toBeVisible({ timeout: 90_000 });
  await expect(card).toHaveAttribute("data-unfinished", "true");
  await expect(card.getByRole("heading", { name: fixture.name })).toBeVisible();
  await expect(card).toContainText(fixture.town);
  await expect(card.getByTestId("card-opening-rating")).toContainText(/rated \d+%/);
  await expect(card.getByTestId("card-code")).toHaveText(fixture.code);
  await expect(card).not.toContainText("⟦");
  // No more turns: the bar offers the epilogue.
  await expect(page.getByTestId("open-epilogue")).toBeVisible();
  await noSidewaysScroll(page);
  await bigEnough(page, epilogue);
  if (phone) {
    // The card's head, then its foot (the code and the buttons).
    await card.locator(".career-card-head").evaluate((e) => e.scrollIntoView({ block: "start" }));
    await page.screenshot({ path: info.outputPath("career-card-phone.png") });
    await epilogue.getByTestId("card-share").evaluate((e) => e.scrollIntoView({ block: "end" }));
    await page.screenshot({ path: info.outputPath("career-card-phone-foot.png") });
  }

  // Copy: the clipboard holds the text block.
  await epilogue.getByTestId("card-copy").click();
  await expect(epilogue.getByTestId("card-status")).toHaveText("Copied.");
  const copied = await page.evaluate(() => navigator.clipboard.readText());
  expect(copied.split("\n")[0]).toBe("Mandate career card (unfinished)");
  expect(copied).toContain(fixture.name);
  expect(copied).toContain(fixture.code);
  expect(copied).toContain(fixture.build);
  expect(copied).not.toContain("⟦");
  await info.attach("career-card.txt", { body: copied, contentType: "text/plain" });

  // Share with no share sheet: the image is saved and the text copied.
  await page.evaluate(() => navigator.clipboard.writeText(""));
  const download = page.waitForEvent("download");
  await epilogue.getByTestId("card-share").click();
  const file = await download;
  expect(file.suggestedFilename()).toBe("mandate-career-card.png");
  await expect(epilogue.getByTestId("card-status")).toHaveAttribute("data-status", "saved");
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(copied);

  // Back to the title, and the same opening from the card's code.
  await epilogue.getByTestId("epilogue-new-game").click();
  const code = page.getByTestId("code-start");
  await expect(code).toBeVisible();
  await code.getByTestId("code-input").fill(`  ${fixture.code.toLowerCase()} `);
  await code.getByTestId("code-read").click();
  await expect(code.getByTestId("code-result")).toContainText("A small world");
  await expect(code.getByTestId("code-other-build")).toHaveCount(0);
  await noSidewaysScroll(page);
  await bigEnough(page, code);
  await code.getByTestId("code-begin").click();
  await expect(page.getByTestId("town")).toBeVisible({ timeout: 60_000 });
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
  // The same character in the same town (the pocket's header).
  const menu = await pocketMenu(page);
  await expect(menu).toContainText(fixture.name);
  await expect(menu).toContainText(fixture.town);
});

test("a mistyped code says so and starts nothing", async ({ page }) => {
  await page.goto("./");
  const code = page.getByTestId("code-start");
  const typo = fixture.code.slice(0, -1) + (fixture.code.endsWith("0") ? "1" : "0");
  await code.getByTestId("code-input").fill(typo);
  await code.getByTestId("code-read").click();
  await expect(code.getByTestId("code-result")).toContainText("typo");
  await expect(code.getByTestId("code-begin")).toBeDisabled();
});
