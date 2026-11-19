// THROWAWAY SPIKE (DEC-050): measure the hub prototype in headless Chromium.
// Usage (from web/): node spikes/hub/measure.mjs [--smoke]
// Builds the spike, serves it with `vite preview`, then for each renderer x N x
// CPU throttle opens the page with Pixel 7 emulation, runs a scripted 10 s
// route (walk + pinch zoom + drag pan + wide view) and records frame stats.
// Writes results.json and RESULTS.md (text after the interpretation marker is
// preserved) and one screenshot per renderer at N=60.
import { createRequire } from "node:module";
import { spawn, execFileSync } from "node:child_process";
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { gzipSync } from "node:zlib";
import { fileURLToPath } from "node:url";
import path from "node:path";

const require = createRequire(import.meta.url);
const { chromium, devices } = require("playwright");

const here = path.dirname(fileURLToPath(import.meta.url));
const webDir = path.resolve(here, "../..");
const dist = path.join(here, "dist");
const PORT = 5199;
const BASE = `http://localhost:${PORT}/compare.html`;
const smoke = process.argv.includes("--smoke");
const RUN_MS = smoke ? 3000 : 10000;

// ------------------------------------------------------------ build + serve
execFileSync("npx", ["vite", "build", "--config", "spikes/hub/vite.config.ts", "--logLevel", "warn"], { cwd: webDir, stdio: "inherit" });
const server = spawn("npx", ["vite", "preview", "--config", "spikes/hub/vite.config.ts", "--port", String(PORT), "--strictPort"], {
  cwd: webDir,
  stdio: "ignore",
  detached: true,
});
const stopServer = () => {
  try {
    process.kill(-server.pid);
  } catch {
    /* already gone */
  }
};
process.on("exit", stopServer);
for (let i = 0; i < 100; i++) {
  try {
    const r = await fetch(BASE);
    if (r.ok) break;
  } catch {
    /* not up yet */
  }
  await new Promise((r) => setTimeout(r, 200));
}

// ------------------------------------------------------------ helpers
const pct = (a, p) => {
  if (!a.length) return 0;
  const s = [...a].sort((x, y) => x - y);
  return s[Math.min(s.length - 1, Math.floor(p * s.length))];
};
const gz = (p) => gzipSync(readFileSync(p), { level: 9 }).length;

function frameStats(deltas, cpu, render) {
  // FPS over sliding 0.5 s windows (step 0.1 s), from frame timestamps.
  const ts = [];
  let t = 0;
  for (const d of deltas) ts.push((t += d));
  const fpsWin = [];
  for (let start = 0; start + 500 <= t; start += 100) {
    const n = ts.filter((x) => x > start && x <= start + 500).length;
    fpsWin.push(n * 2);
  }
  return {
    frames: deltas.length,
    avgFps: (1000 * deltas.length) / Math.max(1, t),
    fpsMedian: pct(fpsWin, 0.5),
    fpsP5: pct(fpsWin, 0.05),
    frameP50: pct(deltas, 0.5),
    frameP95: pct(deltas, 0.95),
    frameP99: pct(deltas, 0.99),
    frameMax: Math.max(0, ...deltas),
    over33: deltas.filter((d) => d > 34).length,
    cpuP50: pct(cpu, 0.5),
    cpuP95: pct(cpu, 0.95),
    renderP50: pct(render, 0.5),
    renderP95: pct(render, 0.95),
  };
}

const browser = await chromium.launch({
  args: ["--enable-unsafe-swiftshader", "--ignore-gpu-blocklist", "--enable-webgl"],
});

const pixel7 = devices["Pixel 7"];
const contextFor = (layout) => {
  if (layout === "desktop") return { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 };
  if (layout === "landscape") return { ...pixel7, viewport: { width: pixel7.viewport.height, height: pixel7.viewport.width } };
  return pixel7;
};

async function run(cfg) {
  const ctx = await browser.newContext(contextFor(cfg.layout));
  const page = await ctx.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(String(e)));
  page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
  const js = new Set();
  page.on("response", (r) => {
    const u = new URL(r.url());
    if (u.pathname.endsWith(".js")) js.add(u.pathname.replace(/^\//, ""));
  });
  const cdp = await ctx.newCDPSession(page);
  await cdp.send("Performance.enable");
  if (cfg.rate > 1) await cdp.send("Emulation.setCPUThrottlingRate", { rate: cfg.rate });
  const q = new URLSearchParams({ r: cfg.r, n: String(cfg.n) });
  if (cfg.res) q.set("res", cfg.res);
  if (cfg.light) q.set("light", cfg.light);
  await page.goto(`${BASE}?${q}`, { waitUntil: "load" });
  await page.waitForFunction(() => window.__spike && window.__spike.ready, null, { timeout: 120000 });
  const meta = await page.evaluate(() => ({ info: window.__spike.info, timings: window.__spike.timings }));
  await page.waitForTimeout(1000); // warm-up
  await page.evaluate(() => {
    window.__spike.autopilot();
    window.__spike.start();
  });
  await page.waitForTimeout(RUN_MS);
  const rec = await page.evaluate(() => window.__spike.stop());
  const metrics = Object.fromEntries((await cdp.send("Performance.getMetrics")).metrics.map((m) => [m.name, m.value]));
  await ctx.close();
  const stats = frameStats(rec.deltas, rec.cpu, rec.render);
  const out = {
    ...cfg,
    info: meta.info,
    ttffMs: meta.timings.firstFrameAt,
    initMs: meta.timings.initMs,
    importMs: meta.timings.importMs,
    backing: rec.backing,
    spritesP50: pct(rec.sprites, 0.5),
    spritesMax: Math.max(0, ...rec.sprites),
    longTasks: rec.longTasks.length,
    longTaskMs: rec.longTasks.reduce((a, b) => a + b.duration, 0),
    loadLongTasks: rec.loadLongTasks.length,
    loadLongTaskMs: rec.loadLongTasks.reduce((a, b) => a + b.duration, 0),
    heapUsedMB: metrics.JSHeapUsedSize / 1048576,
    heapTotalMB: metrics.JSHeapTotalSize / 1048576,
    js: [...js].sort(),
    errors,
    ...stats,
  };
  console.log(
    `${cfg.label.padEnd(34)} fps med ${String(out.fpsMedian).padStart(3)} p5 ${String(out.fpsP5).padStart(3)}  ` +
      `p95 ${out.frameP95.toFixed(1).padStart(5)} ms  cpu95 ${out.cpuP95.toFixed(1).padStart(5)} ms  long ${out.longTasks}  ` +
      `heap ${out.heapUsedMB.toFixed(1)} MB  ttff ${out.ttffMs.toFixed(0)} ms  sprites ${out.spritesP50}/${out.spritesMax}` +
      (errors.length ? `  ERRORS ${errors.join(" | ")}` : ""),
  );
  return out;
}

// ------------------------------------------------------------ matrix
const configs = [];
const add = (c) => configs.push({ layout: "portrait", res: null, light: null, ...c, label: c.label ?? `${c.r} n=${c.n} cpu x${c.rate}` });
if (smoke) {
  for (const r of ["canvas", "pixi"]) add({ r, n: 60, rate: 4 });
} else {
  for (const rate of [4, 6]) for (const r of ["canvas", "pixi"]) for (const n of [20, 60, 150]) add({ r, n, rate });
  for (const rate of [4, 6]) for (const r of ["canvas", "pixi"]) add({ r, n: 60, rate, res: "css", label: `${r} n=60 cpu x${rate} res=css` });
  for (const r of ["canvas", "pixi"]) add({ r, n: 150, rate: 6, res: "low", label: `${r} n=150 cpu x6 res=low` });
  for (const r of ["canvas", "pixi"]) add({ r, n: 60, rate: 1, label: `${r} n=60 cpu x1 (phone, unthrottled)` });
  for (const r of ["canvas", "pixi"]) add({ r, n: 60, rate: 4, light: "dusk", label: `${r} n=60 cpu x4 dusk` });
  for (const r of ["canvas", "pixi"]) add({ r, n: 60, rate: 4, layout: "landscape", label: `${r} n=60 cpu x4 landscape` });
  for (const r of ["canvas", "pixi"]) add({ r, n: 150, rate: 1, layout: "desktop", label: `${r} n=150 desktop x1` });
}

const results = [];
for (const c of configs) results.push(await run(c));

// ------------------------------------------------------------ bundle cost
const jsOf = (r) => new Set(results.filter((x) => x.r === r).flatMap((x) => x.js));
const canvasJs = jsOf("canvas");
const pixiJs = jsOf("pixi");
const shared = [...canvasJs].filter((f) => pixiJs.has(f));
const sizeOf = (files) => files.reduce((a, f) => a + (existsSync(path.join(dist, f)) ? gz(path.join(dist, f)) : 0), 0);
const bundle = {
  sharedGz: sizeOf(shared),
  canvasOnly: [...canvasJs].filter((f) => !pixiJs.has(f)),
  pixiOnly: [...pixiJs].filter((f) => !canvasJs.has(f)),
};
bundle.canvasAddsGz = sizeOf(bundle.canvasOnly);
bundle.pixiAddsGz = sizeOf(bundle.pixiOnly);

// ------------------------------------------------------------ screenshots
for (const r of ["canvas", "pixi"]) {
  const ctx = await browser.newContext({ ...pixel7, deviceScaleFactor: 1 });
  const page = await ctx.newPage();
  await page.goto(`${BASE}?r=${r}&n=60`, { waitUntil: "load" });
  await page.waitForFunction(() => window.__spike && window.__spike.ready);
  await page.waitForTimeout(2500);
  await page.screenshot({ path: path.join(here, `${r}-n60.png`) });
  if (r === "canvas") {
    await page.evaluate(() => window.__spike.enter());
    await page.waitForSelector("#interior:not([hidden])", { timeout: 15000 });
    await page.screenshot({ path: path.join(here, `interior.png`) });
  }
  await ctx.close();
}
await browser.close();
stopServer();

writeFileSync(path.join(here, "results.json"), JSON.stringify({ date: new Date().toISOString(), runMs: RUN_MS, bundle, results }, null, 2));
if (smoke) process.exit(0);

// ------------------------------------------------------------ RESULTS.md
const f1 = (x) => x.toFixed(1);
const kb = (b) => `${(b / 1024).toFixed(1)} KB`;
const row = (x) =>
  `| ${x.r} | ${x.n} | ${x.rate === 1 ? "none" : `x${x.rate}`} | ${x.layout}${x.res ? `, res=${x.res}` : ""}${x.light ? `, ${x.light}` : ""} | ${x.backing.join("×")} | ` +
  `${x.fpsMedian} | ${x.fpsP5} | ${f1(x.avgFps)} | ${f1(x.frameP50)} | ${f1(x.frameP95)} | ${f1(x.cpuP50)} | ${f1(x.cpuP95)} | ${f1(x.renderP95)} | ` +
  `${x.longTasks} (${x.longTaskMs.toFixed(0)} ms) | ${f1(x.heapUsedMB)} | ${x.ttffMs.toFixed(0)} | ${x.spritesP50}/${x.spritesMax} |`;
const header =
  "| Renderer | N | CPU throttle | Layout | Backing px | FPS median | FPS p5 | FPS avg | Frame p50 ms | Frame p95 ms | JS/frame p50 ms | JS/frame p95 ms | of which render() p95 ms | Long tasks in run | JS heap MB | First frame ms | Sprites p50/max |\n" +
  "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|";
const main = results.filter((x) => x.layout === "portrait" && !x.res && !x.light);
const extra = results.filter((x) => !main.includes(x));
const info = [...new Set(results.map((x) => x.info))];
const marker = "<!-- interpretation -->";
const resultsMd = path.join(here, "RESULTS.md");
const kept = existsSync(resultsMd) && readFileSync(resultsMd, "utf8").includes(marker) ? readFileSync(resultsMd, "utf8").split(marker)[1] : "\n\n## Interpretation\n\n(to be written)\n";
const md = `# Hub rendering spike: results

Generated by \`web/spikes/hub/measure.mjs\` on ${new Date().toISOString().slice(0, 10)} (headless Chromium via Playwright, Pixel 7 emulation
412×839 CSS px at DPR 2.625 unless noted, CDP \`Emulation.setCPUThrottlingRate\`). Each run: 1 s warm-up, then ${RUN_MS / 1000} s of a scripted
route (player walks an A* route; camera follows, pinch-zooms 1.4–4.6×, drag-pans a circle, then widens to 2×). FPS median/p5 are over sliding
0.5 s windows. "JS/frame" is the main-thread time of one frame (walker step + cull/sort + renderer submit + DOM labels);
"render()" is the renderer's share of it. Long tasks are those > 50 ms during
the measured window (load-time long tasks are in \`results.json\`).

Renderers seen: ${info.map((i) => `\`${i}\``).join(", ")}.

## Main matrix (portrait, full device resolution)

${header}
${main.map(row).join("\n")}

## Extra runs

\`res=css\`: backing store at CSS resolution (DPR 1), upscaled by the compositor with \`image-rendering: pixelated\`.
\`res=low\`: backing store at 1/3 of CSS px (≈ one backing pixel per art pixel at the default zoom), upscaled the same way.
\`x1 (phone, unthrottled)\`: Pixel 7 viewport and DPR with no CPU throttle.
\`dusk\`: day-part lighting (full-screen multiply tint + ${"additive"} lamp glows). \`landscape\`: Pixel 7 rotated. \`desktop\`: 1280×800 at DPR 1, no throttle.

${header}
${extra.map(row).join("\n")}

## Bundle cost (gzip -9 of the JS chunks each renderer actually loads)

| | gzipped |
|---|---|
| Shared spike code (scene, art, loop, UI) | ${kb(bundle.sharedGz)} |
| Canvas 2D renderer adds | ${kb(bundle.canvasAddsGz)} (${bundle.canvasOnly.join(", ")}) |
| PixiJS renderer adds (pixi.js 8.22.0 + adapter) | ${kb(bundle.pixiAddsGz)} (${bundle.pixiOnly.length} chunks) |

For comparison the app's main bundle is ~122 KB gzipped (\`npm run build\`) and the WASM engine ~3.4 MB raw.

![Canvas 2D, N=60](canvas-n60.png) ![PixiJS, N=60](pixi-n60.png) ![Interior overlay](interior.png)

${marker}${kept}`;
writeFileSync(resultsMd, md);
console.log("wrote", resultsMd);
