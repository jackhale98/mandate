// Screenshots of the pixel-art spike's sample street (DEC-084, P0-6) for the
// owner's review: the placeholder and the tool's pack (both night looks, D12)
// at several times of day, on a desktop and a phone, into target/pixelkit/street/.
// Build the site and the spike first, serve it, then run from web/:
//   npx vite preview -c landing/vite.config.ts --port 4199
//   node spikes/hub/pixelkit-capture.mjs [http://localhost:4199/mandate/spike-hub/] [seed]
// `node tools/pixelkit/bin/pixelkit.mjs street` (from the repository root) then
// lays them side by side.
import { mkdirSync } from "node:fs";
import path from "node:path";
import { chromium, devices } from "@playwright/test";

const base = process.argv[2] ?? "http://localhost:4199/mandate/spike-hub/";
const seed = process.argv[3] ?? "4242";
const out = path.resolve(process.cwd(), "..", "target", "pixelkit", "street");
mkdirSync(out, { recursive: true });

const looks = [
  ["placeholder", "art=placeholder"],
  ["multiply", "art=pixelkit&night=multiply"],
  ["moods", "art=pixelkit&night=moods"],
];
const times = [
  ["noon", 12 * 60],
  ["golden", 18 * 60 + 20],
  ["dusk", 19 * 60 + 30],
  ["night", 22 * 60 + 30],
];
const targets = [
  ["desktop", { viewport: { width: 1280, height: 800 }, deviceScaleFactor: 1 }],
  ["phone", devices["Pixel 7"]],
];

const browser = await chromium.launch(process.env.PW_CHROMIUM_PATH ? { executablePath: process.env.PW_CHROMIUM_PATH } : {});
for (const [device, opts] of targets) {
  const ctx = await browser.newContext(opts);
  for (const [look, query] of looks) {
    const page = await ctx.newPage();
    const errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    page.on("console", (m) => m.type() === "error" && errors.push(m.text()));
    await page.goto(`${base}?seed=${seed}&t=720&speed=0&n=30&preset=street&${query}`);
    await page.waitForFunction(() => window.__hub?.ready === true, undefined, { timeout: 90_000 });
    // The list of places folds away on phones; hide the status pill so the street shows.
    await page.addStyleTag({ content: "#status{display:none!important}" });
    for (const [name, minutes] of times) {
      await page.evaluate((m) => window.__hub.setClock(m), minutes);
      await page.evaluate(() => window.__hub.showStreet());
      await page.waitForTimeout(600);
      const file = path.join(out, `${device}-${look}-${name}.png`);
      await page.locator("#stage").screenshot({ path: file });
      console.log(file);
    }
    if (errors.length) console.error(look, device, errors);
    await page.close();
  }
  await ctx.close();
}
await browser.close();
