// Renders the 1200×630 share image (Open Graph, Twitter card) from the front-page
// map captured from the game (capture.mjs, encode.py) and the DEC-081 mark.
// `npm run capture` runs this last; on its own: node web/landing/scripts/og.mjs
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { chromium } from "@playwright/test";

const raw = fileURLToPath(new URL("../../../target/landing-capture/", import.meta.url));
const out = fileURLToPath(new URL("../public/og.png", import.meta.url));
const map = (await readFile(`${raw}map-4040-crop.png`)).toString("base64");

const html = `<!doctype html><html><head><style>
  * { box-sizing: border-box; margin: 0; }
  body { width: 1200px; height: 630px; background: #11131a; color: #e9eaf0; font-family: Georgia, "Times New Roman", serif; overflow: hidden; position: relative; }
  .map { position: absolute; right: 0; top: 0; height: 630px; width: 660px; object-fit: cover; object-position: 62% 50%; image-rendering: pixelated; }
  .fade { position: absolute; left: 540px; top: 0; width: 140px; height: 630px; background: linear-gradient(90deg, #11131a, rgba(17,19,26,0)); }
  .copy { position: absolute; left: 60px; top: 0; bottom: 0; width: 450px; display: flex; flex-direction: column; justify-content: center; gap: 22px; }
  .rule { border-top: 3px solid #e9eaf0; border-bottom: 1px solid #e9eaf0; height: 7px; width: 100%; }
  h1 { font-size: 104px; font-weight: 400; letter-spacing: 0.02em; line-height: 1; display: flex; align-items: flex-end; gap: 18px; }
  p { font-size: 29px; line-height: 1.3; }
  .kick { font: 600 20px/1 system-ui, sans-serif; letter-spacing: 0.16em; text-transform: uppercase; color: #e8b44c; }
</style></head><body>
  <img class="map" src="data:image/png;base64,${map}" alt="">
  <div class="fade"></div>
  <div class="copy">
    <p class="kick">A political career roguelike</p>
    <div class="rule"></div>
    <h1><svg viewBox="0.9 0 16 16" width="92" height="92" shape-rendering="crispEdges"><path d="M2.5 15 V7.5 H4.5 V9.5 H6.5 V11.5 H8.5 V8.5 H10.5 V5.5 H12.5 V3.5 H13.5 V15" fill="none" stroke="#e8b44c" stroke-width="1.5" stroke-linecap="square"/><rect x="13" y="0" width="1" height="3" fill="#e9eaf0"/><rect x="14" y="0" width="2" height="2" fill="#eb6834"/></svg>Mandate</h1>
    <div class="rule"></div>
    <p>Start on a town council in a country generated for your game, and see how far up you can get.</p>
    <p class="kick" style="color:#e9eaf0">Free to play in your browser</p>
  </div>
</body></html>`;

const browser = await chromium.launch(
  process.env.PW_CHROMIUM_PATH ? { executablePath: process.env.PW_CHROMIUM_PATH } : {},
);
const page = await browser.newPage({ viewport: { width: 1200, height: 630 } });
await page.setContent(html);
await page.waitForTimeout(200);
await page.screenshot({ path: out });
await browser.close();
console.log(out);
