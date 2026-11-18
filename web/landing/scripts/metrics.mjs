// Landing-page performance check (DEC-083): LCP, CLS and transfer sizes on a
// phone and a desktop, unthrottled and on an emulated slow 4G connection with
// a 4× slower CPU. Run against a preview of the built site:
//   node web/landing/scripts/metrics.mjs http://localhost:4173/mandate/
import { chromium, devices } from "@playwright/test";

const url = process.argv[2] ?? "http://localhost:4173/mandate/";
const browser = await chromium.launch(
  process.env.PW_CHROMIUM_PATH ? { executablePath: process.env.PW_CHROMIUM_PATH } : {},
);
const profiles = {
  phone: devices["Pixel 7"],
  desktop: { viewport: { width: 1440, height: 900 } },
};
const rows = [];
for (const [name, device] of Object.entries(profiles)) {
  for (const slow of [false, true]) {
    const ctx = await browser.newContext(device);
    const page = await ctx.newPage();
    if (slow) {
      const cdp = await ctx.newCDPSession(page);
      await cdp.send("Network.enable");
      // Lighthouse's "slow 4G": 150 ms RTT, 1.6 Mbps down, 750 kbps up.
      await cdp.send("Network.emulateNetworkConditions", {
        offline: false, latency: 150, downloadThroughput: (1.6 * 1024 * 1024) / 8, uploadThroughput: (750 * 1024) / 8,
      });
      await cdp.send("Emulation.setCPUThrottlingRate", { rate: 4 });
    }
    await page.addInitScript(() => {
      window.__lcp = { time: 0, element: "" };
      window.__cls = 0;
      new PerformanceObserver((l) => {
        for (const e of l.getEntries()) {
          window.__lcp = { time: e.startTime, element: e.element ? e.element.tagName + (e.element.className ? "." + e.element.className : "") : e.url };
        }
      }).observe({ type: "largest-contentful-paint", buffered: true });
      new PerformanceObserver((l) => {
        for (const e of l.getEntries()) if (!e.hadRecentInput) window.__cls += e.value;
      }).observe({ type: "layout-shift", buffered: true });
    });
    await page.goto(url, { waitUntil: "load" });
    await page.waitForTimeout(1500);
    const m = await page.evaluate(() => {
      const res = performance.getEntriesByType("resource");
      const nav = performance.getEntriesByType("navigation")[0];
      const sum = (f) => res.filter(f).reduce((a, r) => a + r.transferSize, 0);
      return {
        lcp: Math.round(window.__lcp.time),
        lcpElement: window.__lcp.element,
        cls: window.__cls,
        htmlBytes: nav.transferSize,
        cssJsBytes: sum((r) => /\.(css|js)$/.test(r.name)),
        imageBytes: sum((r) => r.initiatorType === "img" || /\.(avif|webp|png|svg)$/.test(r.name)),
        requests: res.length + 1,
      };
    });
    rows.push({ profile: `${name}${slow ? " slow-4G 4×CPU" : ""}`, ...m });
    await ctx.close();
  }
}
await browser.close();
console.table(rows);
