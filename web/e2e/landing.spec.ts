import { existsSync } from "node:fs";
import { expect, test, type Page } from "@playwright/test";

// DEC-083: the landing page at the site root, the game at play/. The
// playwright baseURL is the game; the landing page is one level up.
const LANDING = "../";

/** Contrast ratio between an element's text and the first opaque background behind it. */
const contrastOf = (page: Page, selector: string) =>
  page.evaluate((sel) => {
    const el = document.querySelector(sel);
    if (!el) throw new Error(`no element ${sel}`);
    const parse = (c: string) => (c.match(/[\d.]+/g) ?? []).map(Number);
    const lum = ([r, g, b]: number[]) => {
      const f = (v: number) => {
        const s = v / 255;
        return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
      };
      return 0.2126 * f(r ?? 0) + 0.7152 * f(g ?? 0) + 0.0722 * f(b ?? 0);
    };
    let bg: number[] = [];
    for (let n: Element | null = el; n; n = n.parentElement) {
      const c = parse(getComputedStyle(n).backgroundColor);
      if (c.length === 3 || (c.length === 4 && (c[3] ?? 0) > 0.95)) {
        bg = c;
        break;
      }
    }
    const fg = parse(getComputedStyle(el).color);
    const [a, b] = [lum(fg), lum(bg)].sort((x, y) => y - x);
    return ((a ?? 0) + 0.05) / ((b ?? 0) + 0.05);
  }, selector);

test("landing page: one action to the game, no errors, sound structure", async ({ page, baseURL }) => {
  const errors: string[] = [];
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(LANDING);

  await expect(page.locator("h1")).toHaveCount(1);
  await expect(page.locator("h1")).toContainText("Unknown councillor sets sights");
  await expect(page.getByRole("banner")).toHaveCount(1);
  await expect(page.getByRole("main")).toHaveCount(1);
  await expect(page.getByRole("contentinfo")).toHaveCount(1);
  expect(await page.locator("h2").count()).toBeGreaterThanOrEqual(6);
  // Every image is described; the screenshots and maps are lazy below the fold.
  for (const img of await page.locator("img").all()) {
    expect((await img.getAttribute("alt"))?.length ?? 0).toBeGreaterThan(20);
    expect(await img.getAttribute("width")).toBeTruthy();
    expect(await img.getAttribute("height")).toBeTruthy();
  }
  await expect(page.locator('img[fetchpriority="high"]')).toHaveCount(1);

  // Meta for sharing.
  await expect(page.locator('meta[property="og:image"]')).toHaveAttribute("content", /og\.png$/);
  await expect(page.locator('meta[name="description"]')).toHaveAttribute("content", /.{80,}/);

  // The primary action is a plain link to the game, and it starts the game.
  const play = page.getByRole("link", { name: "Play in your browser" }).first();
  await expect(play).toBeVisible();
  expect(new URL((await play.getAttribute("href")) ?? "", page.url()).href).toBe(baseURL);
  const box = await play.boundingBox();
  expect(box?.height ?? 0).toBeGreaterThanOrEqual(44);

  // Every visible link is a comfortable tap target.
  for (const link of await page.getByRole("link").all()) {
    if (!(await link.isVisible())) continue;
    const b = await link.boundingBox();
    if (!b || b.y < 0) continue;
    expect(b.height, (await link.textContent()) ?? "").toBeGreaterThanOrEqual(44);
  }

  // No horizontal scrolling.
  expect(await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)).toBeLessThanOrEqual(0);

  await play.click();
  await expect(page).toHaveURL(baseURL ?? /play\/$/);
  await expect(page.getByRole("heading", { name: "Mandate" })).toBeVisible();
  expect(errors).toEqual([]);
});

for (const colorScheme of ["dark", "light"] as const) {
  test(`landing page contrast, ${colorScheme} theme`, async ({ page }) => {
    await page.emulateMedia({ colorScheme });
    await page.goto(LANDING);
    for (const sel of [
      "body",
      ".lead h1",
      ".kicker",
      ".deck",
      ".standfirst",
      ".lead-art figcaption",
      ".cta-note p",
      ".button",
      ".dateline",
      ".tag-now",
      ".rungs li:not(.lit) p",
      ".listings dd",
      ".ledger td",
      ".notices dd",
      ".notices h2",
      ".button-ballot",
      ".colophon",
    ]) {
      expect(await contrastOf(page, sel), sel).toBeGreaterThanOrEqual(4.5);
    }
  });
}

test("landing page: no layout shift while loading and scrolling", async ({ page }) => {
  await page.goto(LANDING, { waitUntil: "networkidle" });
  await page.evaluate(async () => {
    for (let y = 0; y < document.documentElement.scrollHeight; y += 300) {
      window.scrollTo(0, y);
      await new Promise((r) => setTimeout(r, 30));
    }
  });
  await page.waitForLoadState("networkidle");
  const cls = await page.evaluate(
    () =>
      new Promise<number>((resolve) => {
        let sum = 0;
        new PerformanceObserver((list) => {
          for (const e of list.getEntries() as (PerformanceEntry & { value: number; hadRecentInput: boolean })[]) {
            if (!e.hadRecentInput) sum += e.value;
          }
        }).observe({ type: "layout-shift", buffered: true });
        setTimeout(() => resolve(sum), 300);
      }),
  );
  expect(cls).toBeLessThan(0.01);
});

test("landing page works without JavaScript", async ({ browser, baseURL }) => {
  const ctx = await browser.newContext({ javaScriptEnabled: false, baseURL: baseURL ?? "" });
  const page = await ctx.newPage();
  await page.goto(LANDING);
  const play = page.getByRole("link", { name: "Play in your browser" }).first();
  await expect(play).toBeVisible();
  await expect(page.locator("[data-continue]")).toBeHidden();
  // Without the reveal script, the tally and the ladder are drawn complete.
  await expect(page.locator(".tally .fill").first()).toHaveCSS("transform", "none");
  await ctx.close();
});

test("reduced motion: the tally is drawn complete", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(LANDING);
  await expect(page.locator(".tally .fill").first()).toHaveCSS("transform", "none");
  await expect(page.locator(".ladder .lit")).toBeVisible();
});

test("the landing page never creates the game's database, and offers to continue a saved run", async ({ page }) => {
  test.setTimeout(90_000);
  await page.goto(LANDING);
  await page.waitForLoadState("networkidle");
  const before = await page.evaluate(async () => (await indexedDB.databases()).map((d) => d.name));
  expect(before).not.toContain("mandate");
  await expect(page.locator("[data-continue]")).toBeHidden();

  // Start a run in the game, then come back.
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("4242");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  if ((await page.getByTestId("pocket").count()) === 0) await page.getByTestId("pocket-button").click();
  await page.getByTestId("pocket").getByRole("button", { name: "Save & quit" }).click();
  await expect(page.getByRole("button", { name: "Continue" }).first()).toBeVisible();
  await page.goto(LANDING);
  const cont = page.locator("[data-continue]");
  await expect(cont).toBeVisible();
  await expect(cont.getByRole("link", { name: "Continue your run" })).toBeVisible();
  await expect(cont).toContainText(/turn 0 · 1 January 2020/);
  await expect(page.locator("[data-free-line]")).toBeHidden();
});

test("the hub spike is still published beside the game", async ({ request, baseURL }) => {
  test.skip(!existsSync("dist/spike-hub/index.html"), "spike not built (CI builds it after the app)");
  const res = await request.get(new URL("../spike-hub/", baseURL).href);
  expect(res.status()).toBe(200);
  expect(await res.text()).toContain("Hub render spike");
});
