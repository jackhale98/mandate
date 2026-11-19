import { expect, test } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

// The content workbench's designer page is dev only: a release build drops
// it, so this test serves the app with Vite's dev server (dev mode) and
// opens `?dev=1`. With an engine built with `dev-tools` it lists the
// content and explains an item; with the release engine (CI's) the page
// says the engine lacks the dev tools. Either way the page opens.

const WEB = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const PORT = Number(process.env.PW_DEV_PORT ?? Number(process.env.PW_PORT ?? 4173) + 17);
const BASE = process.env.MANDATE_BASE ?? "/mandate/";
const URL = `http://localhost:${PORT}${BASE}play/?dev=1&seed=2026`;

let server: ChildProcess | null = null;

test.describe("designer page (dev mode)", () => {
  // One dev server, for the desktop project only.
  test.beforeAll(async ({ browserName: _ }, info) => {
    if (info.project.name !== "desktop") return;
    server = spawn("npx", ["vite", "--port", String(PORT), "--strictPort"], {
      cwd: WEB,
      stdio: "ignore",
      env: { ...process.env, BROWSER: "none" },
    });
    const deadline = Date.now() + 60_000;
    for (;;) {
      try {
        const r = await fetch(`http://localhost:${PORT}${BASE}play/`);
        if (r.ok) break;
      } catch {
        // not up yet
      }
      if (Date.now() > deadline) throw new Error("vite dev server did not start");
      await new Promise((r) => setTimeout(r, 500));
    }
  });

  test.afterAll(() => {
    server?.kill();
  });

  test("opens in dev mode and explains an item", async ({ page }, info) => {
    test.skip(info.project.name !== "desktop", "desktop only (one dev server)");
    test.setTimeout(180_000);
    await page.goto(URL);
    await expect(page.getByRole("heading", { name: "Content workbench" })).toBeVisible({ timeout: 60_000 });
    const devless = page.getByRole("heading", { name: "This engine was built without the dev tools" });
    const list = page.getByRole("complementary", { name: "content items" });
    await expect(devless.or(list)).toBeVisible({ timeout: 120_000 });
    if (await devless.isVisible()) return;
    // The catalogue: every kind is listed; filter to dilemmas and pick one.
    await page.getByLabel("kind").selectOption("dilemma");
    const first = list.getByRole("button").first();
    await first.click();
    const detail = page.getByRole("region", { name: "explanation of the item" });
    await expect(detail.getByRole("list", { name: "explanation" })).toBeVisible({ timeout: 30_000 });
    await expect(detail.getByText(/could fire now|cannot fire now/)).toBeVisible();
    // A blocked item can still be previewed without changing anything.
    const force = detail.getByRole("button", { name: /Force preview/ });
    if (await force.isVisible()) await force.click();
    await expect(detail.getByRole("region", { name: "preview" })).toBeVisible();
    await expect(detail.getByRole("heading", { name: "Options" })).toBeVisible();
  });
});
