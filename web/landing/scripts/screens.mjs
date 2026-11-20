// Screenshots of the landing page in both themes, on a phone and a desktop,
// for review (DEC-083). Writes to ../target/screens/.
//   node web/landing/scripts/screens.mjs http://localhost:4173/mandate/   (the port PW_PORT, if set)
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { chromium } from "@playwright/test";

const url = process.argv[2] ?? "http://localhost:4173/mandate/";
const out = fileURLToPath(new URL("../../../target/screens/", import.meta.url));
await mkdir(out, { recursive: true });

const browser = await chromium.launch(
  process.env.PW_CHROMIUM_PATH ? { executablePath: process.env.PW_CHROMIUM_PATH } : {},
);
const sizes = {
  phone: { viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true },
  // The narrowest phone the page is laid out for.
  narrow: { viewport: { width: 360, height: 740 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true },
  desktop: { viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 },
};
for (const [name, size] of Object.entries(sizes)) {
  for (const colorScheme of ["dark", "light"]) {
    const ctx = await browser.newContext({ ...size, colorScheme });
    const page = await ctx.newPage();
    await page.goto(url, { waitUntil: "networkidle" });
    await page.screenshot({ path: `${out}landing-${name}-${colorScheme}-fold.png` });
    // Scroll through so lazy images load and the reveal motion settles.
    const height = await page.evaluate(() => document.documentElement.scrollHeight);
    for (let y = 0; y < height; y += 400) {
      await page.evaluate((top) => window.scrollTo(0, top), y);
      await page.waitForTimeout(60);
    }
    await page.waitForTimeout(2500);
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.screenshot({ path: `${out}landing-${name}-${colorScheme}-full.png`, fullPage: true });
    await ctx.close();
  }
}
await browser.close();
console.log(out);
