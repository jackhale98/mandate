// THROWAWAY PROTOTYPE (DEC-050): measure the town prototype (index.html) in
// headless Chromium. Usage (from web/): node spikes/hub/measure-town.mjs
// Builds the spike, serves it with `vite preview` on port 4197, and for a
// phone (Pixel 7, CPU slowed 4x) and a desktop (no throttle) records: layout
// time of the player's town and of the largest committed capital fixture,
// time to the first frame, and frame intervals and draw times while the
// player walks across town with 30 townsfolk and the clock running.
// Prints a Markdown table (copied into RESULTS.md by hand).
import { createRequire } from "node:module";
import { spawn, execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const require = createRequire(import.meta.url);
const { chromium, devices } = require("playwright");

const here = path.dirname(fileURLToPath(import.meta.url));
const webDir = path.resolve(here, "../..");
const PORT = 4197;
const URL0 = `http://localhost:${PORT}/`;
const fixtures = JSON.parse(readFileSync(path.join(webDir, "src/hub/fixtures/towns.json"), "utf8"));
const capital = fixtures.find((f) => f.source === "largest");

execFileSync("npx", ["vite", "build", "--config", "spikes/hub/vite.config.ts", "--logLevel", "warn"], { cwd: webDir, stdio: "inherit" });
// Run vite directly (not through npx) so stopping the server stops it.
const server = spawn(process.execPath, [path.join(webDir, "node_modules/vite/bin/vite.js"), "preview", "--config", "spikes/hub/vite.config.ts", "--port", String(PORT), "--strictPort"], { cwd: webDir, stdio: "ignore" });
const stop = () => server.kill("SIGTERM");
process.on("exit", stop);
for (let i = 0; i < 100; i++) {
  try {
    if ((await fetch(URL0)).ok) break;
  } catch {
    /* not up yet */
  }
  await new Promise((r) => setTimeout(r, 200));
}

const pct = (a, p) => {
  const s = [...a].sort((x, y) => x - y);
  return s.length ? s[Math.min(s.length - 1, Math.floor(p * s.length))] : 0;
};

const browser = await chromium.launch(process.env.PW_CHROMIUM_PATH ? { executablePath: process.env.PW_CHROMIUM_PATH } : {});
const rows = [];
for (const [name, device, throttle] of [
  ["phone (Pixel 7, CSS px)", devices["Pixel 7"], 4],
  ["desktop 1280×720", { viewport: { width: 1280, height: 720 }, deviceScaleFactor: 1 }, 1],
]) {
  for (const seed of ["4242", "2026"]) {
    const ctx = await browser.newContext({ ...device });
    const page = await ctx.newPage();
    const cdp = await ctx.newCDPSession(page);
    await cdp.send("Emulation.setCPUThrottlingRate", { rate: throttle });
    await page.goto(`${URL0}?seed=${seed}&t=1110&speed=12&n=30`);
    await page.waitForFunction(() => window.__hub?.ready === true, undefined, { timeout: 180_000 });
    const t = await page.evaluate(() => window.__hub.timings);
    const capitalMs = await page.evaluate((input) => window.__hub.layoutMs(input, 3), capital.input);
    // Walk to the farthest named place.
    await page.evaluate(() => {
      const h = window.__hub;
      const v = h.view;
      const start = v.scene.playerCell;
      const W = h.map.width;
      let best = null;
      let bestD = -1;
      for (const b of h.map.buildings) {
        if (b.place === null) continue;
        const d = Math.abs((b.front % W) - (start % W)) + Math.abs(Math.floor(b.front / W) - Math.floor(start / W));
        if (d > bestD) {
          bestD = d;
          best = b;
        }
      }
      v.timings.intervals.length = 0;
      v.timings.draws.length = 0;
      v.goToBuilding(best.index);
    });
    await page.waitForTimeout(8000);
    const run = await page.evaluate(() => ({ iv: [...window.__hub.view.timings.intervals], dr: [...window.__hub.view.timings.draws], st: { ...window.__hub.view.renderer.stats } }));
    const fps = run.iv.map((x) => 1000 / x);
    rows.push({
      name,
      seed,
      town: await page.evaluate(() => `${window.__hub.map.municipality} ${window.__hub.map.width}×${window.__hub.map.height}`),
      layout: t.layoutMs,
      capital: capitalMs,
      first: t.firstFrameMs,
      ready: t.ready,
      fpsMed: pct(fps, 0.5),
      fpsP5: pct(fps, 0.05),
      drawP50: pct(run.dr, 0.5),
      drawP95: pct(run.dr, 0.95),
      bakes: `${run.st.baseBakes}/${run.st.shadowBakes}`,
    });
    await ctx.close();
  }
}
await browser.close();
stop();

const f = (n, d = 1) => Number(n).toFixed(d);
console.log("| Device | Seed | Town (tiles) | Layout ms | Capital layout ms | First frame ms | Ready ms | FPS median | FPS p5 | Draw p50 ms | Draw p95 ms | Base/shadow bakes |");
console.log("|---|---|---|---|---|---|---|---|---|---|---|---|");
for (const r of rows) console.log(`| ${r.name} | ${r.seed} | ${r.town} | ${f(r.layout)} | ${f(r.capital)} | ${f(r.first)} | ${f(r.ready, 0)} | ${f(r.fpsMed, 0)} | ${f(r.fpsP5, 0)} | ${f(r.drawP50, 2)} | ${f(r.drawP95, 2)} | ${r.bakes} |`);
process.exit(0);
