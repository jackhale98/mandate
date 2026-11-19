// Screenshots of the pixel-art spike's sample street (DEC-084, P0-6) for the
// owner's review: the placeholder and the tool's pack (both night looks, D12)
// at several times of day, on a desktop and a phone, into target/pixelkit/street/.
// Build the site and the spike first, serve it, then run from web/:
//   npx vite preview -c landing/vite.config.ts --port 4201
//   node spikes/hub/pixelkit-capture.mjs [http://localhost:4201/mandate/spike-hub/] [seed]
// `node tools/pixelkit/bin/pixelkit.mjs street` (from the repository root) then
// lays them side by side.
// Round 2 adds close-ups of street lamps (by walls, at corners, in open
// plazas) at dusk and night into target/pixelkit/lamps/ (`lamp-light.png`),
// and a frame-rate run (30 people, dusk, the phone with the CPU slowed 4×).
// Filters (comma lists): LOOKS=placeholder,multiply,moods TIMES=noon,golden,dusk,night
// DEVICES=desktop,phone PARTS=street,lamps,fps.
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { chromium, devices } from "@playwright/test";

const base = process.argv[2] ?? "http://localhost:4201/mandate/spike-hub/";
const seed = process.argv[3] ?? "4242";
const root = path.resolve(process.cwd(), "..", "target", "pixelkit");
const out = path.join(root, "street");
mkdirSync(out, { recursive: true });
const only = (name, all) => (process.env[name] ? all.filter(([k]) => process.env[name].split(",").includes(k)) : all);
const parts = (process.env.PARTS ?? "street,lamps,fps").split(",");

const looks = only("LOOKS", [
  ["placeholder", "art=placeholder"],
  ["multiply", "art=pixelkit&night=multiply"],
  ["moods", "art=pixelkit&night=moods"],
]);
const times = only("TIMES", [
  ["noon", 12 * 60],
  ["golden", 18 * 60 + 20],
  ["dusk", 19 * 60 + 30],
  ["night", 22 * 60 + 30],
]);
const targets = only("DEVICES", [
  ["desktop", { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 }],
  ["phone", devices["Pixel 7"]],
]);

const browser = await chromium.launch(process.env.PW_CHROMIUM_PATH ? { executablePath: process.env.PW_CHROMIUM_PATH } : {});

async function open(ctx, query, clock = "t=720&speed=0") {
  const page = await ctx.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
  await page.goto(`${base}?seed=${seed}&${clock}&preset=street&${query}`);
  await page.waitForFunction(() => window.__hub?.ready === true, undefined, { timeout: 90_000 });
  // The list of places folds away on phones; hide the status pill so the street shows.
  await page.addStyleTag({ content: "#status{display:none!important}" });
  return { page, errors };
}

if (parts.includes("street")) {
  for (const [device, opts] of targets) {
    const ctx = await browser.newContext(opts);
    for (const [look, query] of looks) {
      const { page, errors } = await open(ctx, `n=30&${query}`);
      for (const [name, minutes] of times) {
        await page.evaluate((m) => window.__hub.setClock(m), minutes);
        await page.evaluate(() => window.__hub.showStreet());
        await page.waitForTimeout(1400);
        const file = path.join(out, `${device}-${look}-${name}.png`);
        await page.locator("#stage").screenshot({ path: file });
        console.log(file);
      }
      if (errors.length) console.error(look, device, errors);
      await page.close();
    }
    await ctx.close();
  }
}

if (parts.includes("lamps")) {
  // Lamps by walls, at corners and in open plazas, at zoom 4 (desktop), dusk and night, in the default look.
  const lampDir = path.join(root, "lamps");
  mkdirSync(lampDir, { recursive: true });
  // A desktop-sized page (the phone layout folds a list over the town); the sheet crops the middle.
  const ctx = await browser.newContext({ viewport: { width: 1100, height: 700 }, deviceScaleFactor: 1 });
  const { page, errors } = await open(ctx, "n=0&art=pixelkit&night=moods");
  const picks = await page.evaluate(() => window.__hub.lampSites());
  for (const [name, minutes] of [
    ["dusk", 19 * 60 + 30],
    ["night", 22 * 60 + 30],
  ]) {
    await page.evaluate((m) => window.__hub.setClock(m), minutes);
    for (const site of picks) {
      await page.evaluate(([x, y]) => window.__hub.look(x, y, 4), [site.x, site.y]);
      await page.waitForTimeout(1400);
      const file = path.join(lampDir, `${site.kind}-${site.i}-${name}.png`);
      await page.locator("#stage").screenshot({ path: file });
      console.log(file);
    }
  }
  if (errors.length) console.error("lamps", errors);
  await ctx.close();
}

if (parts.includes("fps")) {
  // Frame rate: 30 townsfolk at dusk on an emulated Pixel 7 with the CPU slowed 4× (round 1: 58–60 fps).
  const result = {};
  for (const [look, query] of looks.filter(([l]) => l !== "placeholder")) {
    const ctx = await browser.newContext(devices["Pixel 7"]);
    // The clock runs (12 game minutes a second from 19:30), so frames are never capped as ambient-only, as in round 1's runs.
    const { page } = await open(ctx, `n=30&${query}`, "t=1170&speed=12");
    const cdp = await ctx.newCDPSession(page);
    await cdp.send("Emulation.setCPUThrottlingRate", { rate: 4 });
    await page.evaluate(() => window.__hub.showStreet());
    await page.waitForTimeout(3000);
    const t = await page.evaluate(async () => {
      const v = window.__hub.view;
      v.timings.intervals.length = 0;
      v.timings.draws.length = 0;
      await new Promise((r) => setTimeout(r, 6000));
      const iv = v.timings.intervals.slice();
      const dr = v.timings.draws.slice();
      const avg = (a) => a.reduce((s, x) => s + x, 0) / Math.max(1, a.length);
      const sorted = iv.slice().sort((a, b) => a - b);
      return { fps: 1000 / avg(iv), p95ms: sorted[Math.floor(sorted.length * 0.95)] ?? 0, drawMs: avg(dr), frames: iv.length };
    });
    result[look] = t;
    console.log(look, JSON.stringify(t));
    await ctx.close();
  }
  writeFileSync(path.join(root, "fps.json"), `${JSON.stringify(result, null, 1)}\n`);
}
await browser.close();
