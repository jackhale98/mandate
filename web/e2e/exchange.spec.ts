import { expect, test, type Locator, type Page } from "@playwright/test";
import { playTo, startRun } from "./midrun";
import { openPocket } from "./pocket";
import { useWideFont } from "./wide-font";

// The debate's screen (DEC-112; EXCHANGE_SCENES §7, task X2): a scene of an
// arc, played in the conversation view, and in quick mode as a plain list.
// The rival's showdown in the pinned world opens a debate on the turn below
// (crates/mandate-wasm/tests/scene_views.rs plays the same seed natively).
// A first seat is the player's own neighbourhood, whose worries they know
// from the start, so the hall's groups are known here and a line's range is
// what the other side may answer; a group the player does not know ("?" on
// the strip, ranges on every line) is held by the native test with the seat
// forgotten and by the screen's unit test on that view.
//   - From Today, by keyboard alone, in twelve activations or fewer: open
//     the pocket, the row, quick mode, turn up, three lines, "Why?".
//   - A line shown as a range for a group: how that group moved lies inside
//     the range (shown = rolled).
//   - "Why?" on the result reads the three rounds back.
//   - At 360 px (and 390) with CI's wider fonts every line on offer shows
//     without scrolling; at 320 nothing runs off the side; 1280 too.
//   - No raw key or id is ever on screen; the structure a screen reader
//     needs is there (the accessibility check below).
const SEED = "4242";
const TURN = 22;

/** Nothing raw on screen: no unresolved Fluent key, no engine id. */
async function noRaw(l: Locator) {
  const text = (await l.innerText()).replace(/\s+/g, " ");
  expect(text).not.toContain("⟦");
  expect(text).not.toMatch(/\b(exchange|ui|now|know|alert)-[a-z]/);
  expect(text).not.toMatch(/\b(showdown|hood|person)\.\d|\b[a-z]+:[a-z]+\.\d+/);
}

/** The accessibility check: a dialog with a name (as the conversation box is: the sheets over it keep Escape); every control named; the strip a list of named chips; the
 *  live region polite; a real table with headers on the result; focus inside the scene; decorations hidden. */
async function a11y(page: Page) {
  const scene = page.getByTestId("scene");
  const dialog = scene.locator("[role='dialog']");
  await expect(dialog).toHaveCount(1);
  const named = await dialog.evaluate((d) => {
    const label = d.getAttribute("aria-label") ?? document.getElementById(d.getAttribute("aria-labelledby") ?? "")?.textContent ?? "";
    const unnamed = [...d.querySelectorAll("button, a, input")].filter((b) => {
      const el = b as HTMLElement;
      if (el.closest("[aria-hidden='true']")) return false;
      const name = el.getAttribute("aria-label") ?? el.textContent ?? "";
      return name.trim() === "";
    }).length;
    const chips = [...d.querySelectorAll("[data-testid='scene-chip']")];
    const chipsNamed = chips.every((c) => (c.getAttribute("aria-label") ?? "").trim().length > 0);
    const live = d.querySelector("[aria-live]")?.getAttribute("aria-live");
    const inside = d.contains(document.activeElement) || scene(d).contains(document.activeElement);
    function scene(x: Element) {
      return x.closest("[data-testid='scene']") ?? x;
    }
    return { label: label.trim(), unnamed, chips: chips.length, chipsNamed, live, inside };
  });
  expect(named.label).not.toBe("");
  expect(named.unnamed).toBe(0);
  expect(named.chips).toBeGreaterThan(0);
  expect(named.chipsNamed).toBe(true);
  expect(named.live).toBe("polite");
  expect(named.inside).toBe(true);
  await expect(dialog.getByRole("list", { name: "Who is listening" })).toBeVisible();
}

async function toDebate(page: Page) {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await startRun(page, SEED, false);
  await playTo(page, TURN);
  const pocket = await openPocket(page, "today");
  const row = pocket.locator("[data-need='scene']").first();
  await expect(row).toBeVisible();
  return row;
}

type Change = { group: string; lo: number; hi: number; lo_steps: number; hi_steps: number; exact: boolean };

test("a debate from Today, in quick mode, by keyboard alone in twelve activations or fewer", async ({ page }) => {
  test.setTimeout(300_000);
  const row = await toDebate(page);
  // Back to the town with the pocket shut: the keyboard opens it again.
  await page.getByTestId("pocket").getByRole("button", { name: "Close" }).first().click();
  await expect(page.getByTestId("pocket")).toHaveCount(0);
  let presses = 0;
  const press = async (key: string) => {
    presses += 1;
    await page.keyboard.press(key);
  };
  await page.getByTestId("pocket-button").focus();
  await press("Enter");
  await expect(row).toBeVisible();
  await row.locator(".rw-main").focus();
  await press("Enter");
  const scene = page.getByTestId("scene");
  await expect(scene).toBeVisible();
  await expect(scene).toHaveAttribute("data-phase", "open");
  await noRaw(scene);
  // Quick mode: the same scene as a plain list.
  if ((await scene.getAttribute("data-mode")) !== "plain") {
    await scene.getByTestId("scene-plain-toggle").focus();
    await press("Enter");
  }
  await expect(scene).toHaveAttribute("data-mode", "plain");
  // Turn up (the scene focuses its first choice).
  await expect(scene.getByTestId("scene-begin")).toBeFocused();
  await press("Enter");
  await expect(scene).toHaveAttribute("data-phase", "under_way", { timeout: 15_000 });

  for (let round = 1; round <= 3; round++) {
    await expect(scene).toHaveAttribute("data-round", String(round));
    const options = scene.getByTestId("scene-option");
    const n = await options.count();
    expect(n).toBeGreaterThanOrEqual(4);
    expect(n).toBeLessThanOrEqual(5);
    await expect(scene.locator("[data-safe='true']")).toHaveCount(1);
    expect(await scene.getByTestId("scene-locked").count()).toBeLessThanOrEqual(1);
    await expect(scene.getByTestId("scene-prompt")).not.toBeEmpty();
    await noRaw(scene);
    await a11y(page);
    // A line shown as a range for a group: how that group moved is inside it (round 1; else the first line).
    const all = await options.evaluateAll((els) => els.map((e) => ({ stands: e.getAttribute("data-stands") ?? "[]", fails: e.getAttribute("data-fails") ?? "[]" })));
    const parsed = all.map((x) => [...(JSON.parse(x.stands) as Change[]), ...(JSON.parse(x.fails) as Change[])]);
    const ranged = round === 1 ? parsed.findIndex((cs) => cs.some((c) => !c.exact)) : -1;
    const pick = Math.max(0, ranged);
    const shown = parsed[pick] ?? [];
    const group = ranged >= 0 ? (shown.find((c) => !c.exact)?.group ?? null) : null;
    await expect(scene.locator("[data-testid='scene-option'] [data-choice]").first()).toBeFocused();
    await press(String(pick + 1));
    if (round < 3) await expect(scene).toHaveAttribute("data-round", String(round + 1), { timeout: 15_000 });
    else await expect(scene).toHaveAttribute("data-phase", "done", { timeout: 15_000 });
    if (group) {
      const mine = shown.filter((c) => c.group === group);
      const lo = Math.min(...mine.map((c) => c.lo_steps));
      const hi = Math.max(...mine.map((c) => c.hi_steps));
      const chip = scene.locator(`[data-testid='scene-chip'][data-group='${group}']`);
      const moved = Number((await chip.getAttribute("data-moved")) ?? "NaN");
      expect(moved).toBeGreaterThanOrEqual(lo);
      expect(moved).toBeLessThanOrEqual(hi);
    }
  }
  // The result card: the outcome, how each group moved, what the paper will say.
  const result = scene.getByTestId("scene-result");
  await expect(result).toBeVisible();
  await expect(result).toBeFocused();
  expect(["won", "split", "lost"]).toContain(await result.getAttribute("data-outcome"));
  await expect(result.getByRole("table")).toBeVisible();
  await expect(result.getByRole("columnheader")).toHaveCount(4);
  expect(await result.locator("tbody tr").count()).toBe(await scene.getByTestId("scene-chip").count());
  await expect(scene.getByTestId("scene-paper")).not.toBeEmpty();
  await noRaw(scene);
  // "Why?" reads the three rounds back.
  await result.getByTestId("why-button").focus();
  await press("Enter");
  const why = page.locator(".why-sheet");
  await expect(why).toBeVisible();
  for (const r of [1, 2, 3]) await expect(why.getByText(new RegExp(`Round ${r}`)).first()).toBeVisible();
  expect(presses).toBeLessThanOrEqual(12);
  await page.keyboard.press("Escape");
  await expect(why).toHaveCount(0);
  await result.getByTestId("scene-close").focus();
  await page.keyboard.press("Enter");
  await expect(scene).toHaveCount(0);
  // It needs the player no more.
  const pocket = await openPocket(page, "today");
  await expect(pocket.locator("[data-need='scene']")).toHaveCount(0);
});

test("the staged scene at phone widths with wide fonts, and on a desktop", async ({ page }, info) => {
  test.skip(info.project.name !== "phone", "it sets its own sizes");
  test.setTimeout(300_000);
  await useWideFont(page);
  await page.setViewportSize({ width: 360, height: 740 });
  const row = await toDebate(page);
  await row.locator(".rw-main").click();
  const scene = page.getByTestId("scene");
  await expect(scene).toBeVisible();
  if ((await scene.getAttribute("data-mode")) === "plain") await scene.getByTestId("scene-plain-toggle").click();
  await expect(scene).toHaveAttribute("data-mode", "stage");
  await scene.getByTestId("scene-begin").click();
  await expect(scene).toHaveAttribute("data-phase", "under_way", { timeout: 15_000 });
  for (const width of [320, 360, 390, 1280]) {
    await page.setViewportSize({ width, height: width === 1280 ? 800 : 740 });
    await expect(scene.getByTestId("scene-option").first()).toBeVisible();
    // Nothing runs off the side.
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    if (width === 320) continue;
    // Every line on offer is in view without scrolling the box.
    const fit = await scene.evaluate((s) => {
      const body = s.querySelector(".scn-body") as HTMLElement;
      const box = body.getBoundingClientRect();
      return [...s.querySelectorAll("[data-testid='scene-option']")].map((o) => {
        const r = o.getBoundingClientRect();
        return r.top >= box.top - 1 && r.bottom <= box.bottom + 1 && r.bottom <= window.innerHeight;
      });
    });
    expect(fit.length).toBeGreaterThanOrEqual(4);
    expect(fit.every(Boolean), `${width} px: ${JSON.stringify(fit)}`).toBe(true);
    await noRaw(scene);
  }
  await page.setViewportSize({ width: 360, height: 740 });
  await a11y(page);
  // The keyboard: Right arrow opens a line's detail, Left arrow leaves it; Escape asks before walking out.
  await scene.locator("[data-testid='scene-option'] [data-choice]").first().focus();
  await page.keyboard.press("ArrowRight");
  await expect(scene.getByTestId("scene-detail")).toBeVisible();
  await noRaw(scene);
  await page.keyboard.press("ArrowLeft");
  await expect(scene.getByTestId("scene-detail")).toHaveCount(0);
  await page.keyboard.press("Escape");
  await expect(scene.getByTestId("scene-leave")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(scene.getByTestId("scene-options")).toBeVisible();
  // Three rounds: a line, how the room took it, Continue.
  for (let round = 1; round <= 3; round++) {
    await scene.getByTestId("scene-option").last().locator("[data-choice]").click();
    await expect(scene.getByTestId("scene-continue")).toBeVisible({ timeout: 15_000 });
    await expect(scene.getByTestId("scene-room")).not.toBeEmpty();
    await noRaw(scene);
    await scene.getByTestId("scene-continue").click();
  }
  await expect(scene.getByTestId("scene-result")).toBeVisible();
  await noRaw(scene);
  await a11y(page);
});

// Screenshots for review: a round and the result card at 393 px (2x), light and dark, and both at 1280 px.
// Not a check: only with SCENE_SHOTS=<dir>, on the desktop project (it sets its own sizes).
test.describe("screenshots", () => {
  const dir = process.env.SCENE_SHOTS;
  test.skip(!dir, "screenshots only with SCENE_SHOTS=<dir>");
  test.use({ deviceScaleFactor: 2 });
  test("a round and the result card", async ({ page }, info) => {
    test.skip(info.project.name !== "desktop", "one run makes every size");
    test.setTimeout(300_000);
    await page.setViewportSize({ width: 393, height: 851 });
    const row = await toDebate(page);
    await row.locator(".rw-main").click();
    const scene = page.getByTestId("scene");
    if ((await scene.getAttribute("data-mode")) === "plain") await scene.getByTestId("scene-plain-toggle").click();
    await scene.getByTestId("scene-begin").click();
    await expect(scene).toHaveAttribute("data-phase", "under_way", { timeout: 15_000 });
    const settle = () => page.evaluate(() => document.fonts.ready);
    for (const scheme of ["light", "dark"] as const) {
      await page.emulateMedia({ colorScheme: scheme, reducedMotion: "reduce" });
      await settle();
      await page.screenshot({ path: `${dir}/round-393-${scheme}.png` });
    }
    await page.setViewportSize({ width: 1280, height: 800 });
    await settle();
    await page.screenshot({ path: `${dir}/round-1280-dark.png` });
    await page.setViewportSize({ width: 393, height: 851 });
    // Quick mode too.
    await scene.getByTestId("scene-plain-toggle").click();
    await settle();
    await page.screenshot({ path: `${dir}/round-393-plain-dark.png` });
    await scene.getByTestId("scene-plain-toggle").click();
    for (let round = 1; round <= 3; round++) {
      await scene.getByTestId("scene-option").first().locator("[data-choice]").click();
      await expect(scene.getByTestId("scene-continue")).toBeVisible({ timeout: 15_000 });
      if (round === 1) await page.screenshot({ path: `${dir}/reaction-393-dark.png` });
      await scene.getByTestId("scene-continue").click();
    }
    await expect(scene.getByTestId("scene-result")).toBeVisible();
    for (const scheme of ["light", "dark"] as const) {
      await page.emulateMedia({ colorScheme: scheme, reducedMotion: "reduce" });
      await settle();
      await page.screenshot({ path: `${dir}/result-393-${scheme}.png` });
    }
    await page.setViewportSize({ width: 1280, height: 800 });
    await settle();
    await page.screenshot({ path: `${dir}/result-1280-dark.png` });
  });
});
