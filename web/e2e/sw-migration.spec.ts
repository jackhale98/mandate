import { existsSync } from "node:fs";
import { mkdtemp, readFile, stat, writeFile } from "node:fs/promises";
import { createServer, type Server } from "node:http";
import { tmpdir } from "node:os";
import { extname, join, resolve } from "node:path";
import { expect, test, type Page } from "@playwright/test";

// DEC-083: the game moved from the site root to play/. A browser that ran the
// old build has a service worker at <base>sw.js, scoped to the whole site,
// that answers every navigation with the cached old app. The new build ships
// a self-unregistering sw.js at that path. These tests serve one origin whose
// files switch from an old build to the new dist/, as a deploy does.
//
// The first test uses a minimal stand-in for the old worker and always runs.
// The second uses a real old build when MANDATE_OLD_DIST points to one (a
// dist/ built before DEC-083 with the same MANDATE_BASE), and also checks that
// saved runs survive.

const BASE = process.env.MANDATE_BASE ?? "/mandate/";
const NEW_DIST = resolve("dist");
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
    url: `http://127.0.0.1:${port}${BASE}`,
    deploy: (dir: string) => {
      root = dir;
    },
    // The browser keeps connections alive; close them or close() waits for them.
    close: () =>
      new Promise<void>((r) => {
        server.close(() => r());
        server.closeAllConnections();
      }),
  };
}

/** The old layout's behaviour in miniature: precache the shell, answer every navigation with it. */
async function legacyStandIn(): Promise<string> {
  const dir = await mkdtemp(join(tmpdir(), "mandate-legacy-"));
  await writeFile(
    join(dir, "index.html"),
    `<!doctype html><title>Old app</title><h1>OLD APP SHELL</h1>
     <script>navigator.serviceWorker.register("sw.js").then((r) => r.update());</script>`,
  );
  await writeFile(
    join(dir, "sw.js"),
    `const shell = new URL("index.html", self.registration.scope).href;
     const name = "workbox-precache-v2-" + self.registration.scope;
     self.addEventListener("install", (e) => {
       e.waitUntil((async () => {
         await (await caches.open(name)).add(shell);
         // A cache that belongs to the new game at play/: it must survive.
         await (await caches.open(name + "play/")).put(shell, new Response("kept"));
         await self.skipWaiting();
       })());
     });
     self.addEventListener("activate", (e) => e.waitUntil(self.clients.claim()));
     self.addEventListener("fetch", (e) => {
       if (e.request.mode === "navigate") e.respondWith(caches.match(shell, { cacheName: name }));
     });`,
  );
  return dir;
}

const controlled = (page: Page) =>
  page.evaluate(async () => {
    await navigator.serviceWorker.ready;
    return navigator.serviceWorker.controller !== null;
  });

test.describe("service-worker migration to play/", () => {
  test.use({ actionTimeout: 15_000, navigationTimeout: 20_000 });
  test.skip(({ browserName }) => browserName !== "chromium", "service workers: Chromium only here");
  test.skip(!existsSync(join(NEW_DIST, "sw.js")), "build first: npm run build");

  test("an old root-scoped worker gives way to the landing page", async ({ page }, testInfo) => {
    test.skip(testInfo.project.name !== "desktop", "one device is enough");
    const site = await siteServer(await legacyStandIn());
    try {
      await page.goto(site.url);
      await expect(page.getByRole("heading", { name: "OLD APP SHELL" })).toBeVisible();
      await controlled(page);
      await page.reload();
      expect(await controlled(page)).toBe(true);

      // Deploy the new build. The old worker still answers this navigation
      // with the old shell; its update check installs the retiring worker,
      // which clears the old cache and reloads the page from the network.
      site.deploy(NEW_DIST);
      await page.reload();
      await expect(page.getByRole("heading", { level: 1 })).toContainText("Unknown councillor sets sights", { timeout: 20_000 });
      const state = await page.evaluate(async () => ({
        caches: await caches.keys(),
        scopes: (await navigator.serviceWorker.getRegistrations()).map((r) => r.scope),
      }));
      expect(state.scopes).toEqual([]);
      expect(state.caches.filter((c) => c.endsWith(site.url))).toEqual([]);
      expect(state.caches.some((c) => c.endsWith("/play/"))).toBe(true);

      // The game works at play/ and installs its own worker there.
      await page.getByRole("link", { name: "Play in your browser" }).first().click();
      await expect(page.getByRole("heading", { name: "Mandate" })).toBeVisible();
      await expect
        .poll(() => page.evaluate(async () => (await navigator.serviceWorker.getRegistrations()).map((r) => r.scope)))
        .toEqual([`${site.url}play/`]);
      // A later visit to the landing page is not taken over by anything.
      await page.goto(site.url);
      await expect(page.getByRole("heading", { level: 1 })).toContainText("Unknown councillor sets sights");
    } finally {
      await site.close();
    }
  });

  test("a real old build: landing page after the update, game at play/, saved runs kept", async ({ page }, testInfo) => {
    const oldDist = process.env.MANDATE_OLD_DIST;
    test.skip(!oldDist, "set MANDATE_OLD_DIST to a dist/ built before the move to play/");
    test.skip(testInfo.project.name !== "desktop", "one device is enough");
    test.setTimeout(120_000);
    const site = await siteServer(resolve(oldDist ?? ""));
    try {
      // The old app at the root: start a run and save it.
      await page.goto(site.url);
      await page.getByPlaceholder("e.g. 20260").fill("4242");
      await page.getByRole("button", { name: "Begin" }).click();
      await expect(page.getByTestId("town")).toBeVisible();
      if ((await page.getByTestId("pocket").count()) === 0) await page.getByTestId("pocket-button").click();
      await page.getByTestId("pocket").getByRole("button", { name: "Save & quit" }).click();
      await expect(page.getByRole("button", { name: "Continue" }).first()).toBeVisible();
      // From the next load the old worker serves the app, as on a returning visit.
      await controlled(page);
      await page.reload();
      expect(await controlled(page)).toBe(true);
      await expect(page.getByRole("button", { name: "Continue" }).first()).toBeVisible();

      site.deploy(NEW_DIST);
      await page.reload();
      await expect(page.getByRole("heading", { level: 1 })).toContainText("Unknown councillor sets sights", { timeout: 30_000 });
      await expect(page.locator("[data-continue]")).toBeVisible();

      await page.getByRole("link", { name: "Continue your run" }).click();
      await expect(page).toHaveURL(`${site.url}play/`);
      await expect(page.getByRole("button", { name: "Continue" }).first()).toBeVisible();
      await page.getByRole("button", { name: "Continue" }).first().click();
      await expect(page.getByText("Turn 0")).toBeVisible();
    } finally {
      await site.close();
    }
  });
});
