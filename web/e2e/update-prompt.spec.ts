import { existsSync } from "node:fs";
import { cp, mkdtemp, readdir, readFile, rename, rm, stat, writeFile } from "node:fs/promises";
import { createServer, type Server } from "node:http";
import { tmpdir } from "node:os";
import { extname, join, resolve } from "node:path";
import { expect, test, type Page } from "@playwright/test";

// Play note 18: the owner saw no "A new version is ready" bar after a deploy. This serves build A, lets
// its service worker install and take control, swaps the served files for build B (a copy of the same
// dist whose entry script has another hashed name, so the build identifier and the worker's precache
// manifest both change, as in a real deploy), asks for an update check, and checks that the bar appears
// over everything, that the page keeps running build A until Reload is clicked, and that Reload loads B.

const BASE = process.env.MANDATE_BASE ?? "/mandate/";
const DIST = resolve("dist");
const TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript",
  ".mjs": "text/javascript",
  ".css": "text/css",
  ".wasm": "application/wasm",
  ".json": "application/json",
  ".webmanifest": "application/manifest+json",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".avif": "image/avif",
  ".webp": "image/webp",
};

/** A static server for BASE whose root directory can be swapped, like a redeploy. */
async function siteServer(initialRoot: string) {
  let root = initialRoot;
  const server: Server = createServer((req, res) => {
    void (async () => {
      const path = decodeURIComponent(new URL(req.url ?? "/", "http://x").pathname);
      if (!path.startsWith(BASE)) {
        res.writeHead(404).end();
        return;
      }
      let file = join(root, path.slice(BASE.length));
      if (!file.startsWith(root)) {
        res.writeHead(403).end();
        return;
      }
      try {
        if ((await stat(file)).isDirectory()) file = join(file, "index.html");
        const body = await readFile(file);
        // Like a static host: the worker script is revalidated, never served stale.
        res.writeHead(200, { "content-type": TYPES[extname(file)] ?? "application/octet-stream", "cache-control": "no-cache" });
        res.end(body);
      } catch {
        res.writeHead(404).end();
      }
    })();
  });
  await new Promise<void>((r) => server.listen(0, "127.0.0.1", r));
  const address = server.address();
  const port = typeof address === "object" && address ? address.port : 0;
  return {
    url: `http://127.0.0.1:${port}${BASE}play/`,
    deploy: (dir: string) => {
      root = dir;
    },
    close: () =>
      new Promise<void>((r) => {
        server.close(() => r());
        server.closeAllConnections();
      }),
  };
}

/** Build B: the same app with its entry script under another hashed name, in the page, the worker's precache list and every importer. */
async function buildB(): Promise<{ dir: string; entryA: string; entryB: string }> {
  const dir = await mkdtemp(join(tmpdir(), "mandate-build-b-"));
  await cp(DIST, dir, { recursive: true });
  const assets = join(dir, "play", "assets");
  const entryA = (await readdir(assets)).find((f) => /^index-.*\.js$/.test(f));
  if (!entryA) throw new Error("no entry script in dist/play/assets");
  const entryB = "index-BuildBBBB.js";
  await rename(join(assets, entryA), join(assets, entryB));
  const textFiles: string[] = [];
  const walk = async (d: string) => {
    for (const f of await readdir(d, { withFileTypes: true })) {
      const p = join(d, f.name);
      if (f.isDirectory()) await walk(p);
      else if (/\.(js|html|css)$/.test(f.name)) textFiles.push(p);
    }
  };
  await walk(join(dir, "play"));
  for (const f of textFiles) {
    const s = await readFile(f, "utf8");
    // The worker's precache manifest: the page changed, so its revision does (as a real build's would).
    const next = s.replaceAll(entryA, entryB).replace(/(url:"index\.html",revision:")[0-9a-f]+(")/, "$1bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb$2");
    if (next !== s) await writeFile(f, next);
  }
  return { dir, entryA, entryB };
}

const controlled = (page: Page) =>
  page.evaluate(async () => {
    await navigator.serviceWorker.ready;
    return navigator.serviceWorker.controller !== null;
  });

/** The entry script the running page was loaded with (the build it runs). */
const runningEntry = (page: Page) =>
  page
    .evaluate(() => document.querySelector<HTMLScriptElement>("script[type=module][src*='index-']")?.src.split("/").pop() ?? "")
    .catch(() => ""); // a navigation is under way

test.describe("the update prompt", () => {
  test.use({ actionTimeout: 15_000, navigationTimeout: 20_000 });
  test.skip(({ browserName }) => browserName !== "chromium", "service workers: Chromium only here");
  test.skip(!existsSync(join(DIST, "play", "sw.js")), "build first: npm run build");

  test("a deploy while the game is open: the bar appears over the pocket, the page keeps its build until Reload, and Reload loads the new one", async ({ page }, info) => {
    test.skip(info.project.name !== "desktop", "one device is enough");
    test.setTimeout(120_000);
    // A phone: the bar must clear the bottom bar and sit above the pocket sheet there.
    await page.setViewportSize({ width: 390, height: 844 });
    const b = await buildB();
    const site = await siteServer(DIST);
    try {
      await page.goto(site.url);
      await expect(page.getByTestId("build-id")).toContainText(b.entryA);
      // The worker installs and takes control (the first install claims the page at once).
      await expect.poll(() => controlled(page)).toBe(true);
      await expect(page.getByText("A new version is ready.")).toHaveCount(0);

      // Start a run, open the pocket (the bar must sit above it), then deploy B and ask for an update check.
      await page.getByPlaceholder("e.g. 20260").fill("4242");
      await page.getByRole("button", { name: "Begin" }).click();
      await expect(page.getByTestId("town")).toBeVisible();
      await page.getByTestId("pocket-button").click();
      await expect(page.getByTestId("pocket")).toBeVisible();
      site.deploy(b.dir);
      // The app checks every hour and when it returns to the foreground; a test asks the browser the same way.
      await page.evaluate(async () => {
        const reg = await navigator.serviceWorker.getRegistration();
        await reg?.update();
      });

      const bar = page.locator(".banner-info");
      await expect(bar).toBeVisible({ timeout: 30_000 });
      await expect(bar).toContainText("A new version is ready.");
      // Above the pocket and clear of the bottom bar: the element at the bar's centre is the bar, and it ends above the game bar.
      const box = await bar.boundingBox();
      const gamebar = await page.getByTestId("gamebar").boundingBox();
      expect(box && gamebar && box.y + box.height <= gamebar.y).toBe(true);
      const top = await page.evaluate(([x, y]) => document.elementFromPoint(x as number, y as number)?.closest(".banner-info") !== null, [(box?.x ?? 0) + (box?.width ?? 0) / 2, (box?.y ?? 0) + (box?.height ?? 0) / 2]);
      expect(top).toBe(true);

      if (process.env.UPDATE_SHOT) await page.screenshot({ path: process.env.UPDATE_SHOT });
      // Without a click the page keeps running build A (the run is not disturbed).
      await page.waitForTimeout(1500);
      expect(await runningEntry(page)).toBe(b.entryA);
      await expect(page.getByTestId("pocket")).toBeVisible();
      await expect(bar).toBeVisible();

      // Reload: the new build.
      await bar.getByRole("button", { name: "Reload" }).click();
      await expect.poll(() => runningEntry(page), { timeout: 30_000 }).toBe(b.entryB);
      await expect(page.locator(".banner-info")).toHaveCount(0);
    } finally {
      await site.close();
      await rm(b.dir, { recursive: true, force: true });
    }
  });

  test("the first load after a deploy: a page loaded from the old build's cache is offered the new one", async ({ page }, info) => {
    test.skip(info.project.name !== "desktop", "one device is enough");
    test.setTimeout(120_000);
    const b = await buildB();
    const site = await siteServer(DIST);
    try {
      await page.goto(site.url);
      await expect.poll(() => controlled(page)).toBe(true);
      // A deploy, and the player just comes back to the app (a reload): the old worker answers with the old shell,
      // and finds the new worker while it does.
      site.deploy(b.dir);
      await page.reload();
      await expect(page.getByTestId("build-id")).toContainText(b.entryA);
      await expect(page.locator(".banner-info")).toContainText("A new version is ready.", { timeout: 30_000 });
      await page.locator(".banner-info").getByRole("button", { name: "Reload" }).click();
      await expect.poll(() => runningEntry(page), { timeout: 30_000 }).toBe(b.entryB);
      await expect(page.getByTestId("build-id")).toContainText(b.entryB);
    } finally {
      await site.close();
      await rm(b.dir, { recursive: true, force: true });
    }
  });

  test("the build identifier shows on the title screen and in the report dialog", async ({ page }, info) => {
    test.skip(info.project.name !== "desktop", "one device is enough");
    await page.goto("./");
    const id = page.getByTestId("build-id");
    await expect(id).toHaveText(/^Build \d+\.\d+\.\d+\+index-[\w-]+\.js$/);
    // Small type (not the body size).
    const [size, body] = await page.evaluate(() => [parseFloat(getComputedStyle(document.querySelector("[data-testid=build-id]") as Element).fontSize), parseFloat(getComputedStyle(document.body).fontSize)] as const);
    expect(size).toBeLessThan(body);
    await page.getByTestId("report-problem-title").click();
    await expect(page.getByTestId("report-build-id")).toHaveText((await id.textContent()) ?? "");
  });
});
