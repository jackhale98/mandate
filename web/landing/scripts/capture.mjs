// Captures the landing page's images from the running game (DEC-083): real
// maps, real screens, a real election night. Nothing here is mocked up.
//
//   npm run build && npx vite preview -c landing/vite.config.ts --port 4173 &
//   node landing/scripts/capture.mjs http://localhost:4173/mandate/play/
//   python3 landing/scripts/encode.py
//
// Raw PNGs and facts.json go to ../target/landing-capture/ (not committed);
// encode.py writes the committed AVIF/WebP/PNG files to landing/img/ and
// landing/public/og.png. If the election numbers in facts.json change (the
// engine changed), update the results box in landing/index.html to match.
import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { chromium } from "@playwright/test";

const app = process.argv[2] ?? "http://localhost:4173/mandate/play/";
const out = fileURLToPath(new URL("../../../target/landing-capture/", import.meta.url));
await mkdir(out, { recursive: true });

/** Seeds and scenarios for the map strip; the first is the front-page map. */
const MAPS = [
  { seed: "4040", scenario: "" },
  { seed: "777", scenario: "Resource-rich frontier" },
  { seed: "20260", scenario: "Drought-prone farmland" },
  { seed: "1848", scenario: "Wealthy coastal nation" },
];
/** The run whose first election the page reports. */
const STORY_SEED = "4040";

const browser = await chromium.launch(
  process.env.PW_CHROMIUM_PATH ? { executablePath: process.env.PW_CHROMIUM_PATH } : {},
);
const facts = { maps: [], election: null };

async function begin(page, seed, scenario) {
  await page.goto(app);
  await page.getByPlaceholder("e.g. 20260").fill(seed);
  if (scenario) await page.getByLabel("Scenario").selectOption({ label: scenario });
  await page.getByRole("button", { name: "Begin" }).click();
  await page.getByTestId("town").waitFor({ timeout: 60_000 });
}

async function openPocket(page) {
  if ((await page.getByTestId("pocket").count()) === 0) await page.getByTestId("pocket-button").click();
  await page.getByTestId("pocket").waitFor();
  return page.getByTestId("pocket");
}

// Maps: a landscape desktop window fits the whole country.
for (const m of MAPS) {
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 1000 }, deviceScaleFactor: 2, colorScheme: "dark" });
  const page = await ctx.newPage();
  await begin(page, m.seed, m.scenario);
  const town = (await page.getByTestId("town").locator("h1, h2").first().textContent())?.trim();
  const pocket = await openPocket(page);
  await pocket.getByRole("button", { name: "Map", exact: true }).click();
  const canvas = page.getByRole("img", { name: /Drag to pan/ });
  await canvas.waitFor();
  await page.waitForTimeout(1000);
  await canvas.screenshot({ path: `${out}map-${m.seed}.png` });
  facts.maps.push({ ...m, town });
  await ctx.close();
}

// Phone screens, each in both themes (the page shows the one that matches).
const phone = await browser.newContext({
  viewport: { width: 390, height: 844 },
  deviceScaleFactor: 2,
  isMobile: true,
  hasTouch: true,
  colorScheme: "dark",
});
const page = await phone.newPage();
const both = async (name) => {
  for (const scheme of ["dark", "light"]) {
    await page.emulateMedia({ colorScheme: scheme });
    await page.waitForTimeout(250);
    await page.screenshot({ path: `${out}${name}-${scheme}.png` });
  }
  await page.emulateMedia({ colorScheme: "dark" });
};
await begin(page, STORY_SEED, "");
await page.waitForTimeout(500);
await both("town");
await page.getByTestId("spot-council_chamber").click();
await page.getByTestId("place-panel").waitFor();
await page.waitForTimeout(800);
await both("chamber");
await page.getByTestId("place-panel").getByRole("button", { name: "Close" }).first().click();
await page.waitForTimeout(400);
await page.getByTestId("spot-party_office").click();
await page.waitForTimeout(400);
await page.getByTestId("place-panel").getByRole("button", { name: "Close" }).first().click();
const endTurn = page.getByRole("button", { name: "End turn" });
for (let i = 0; i < 400 && (await page.getByTestId("election-night").count()) === 0; i++) {
  if (await endTurn.isEnabled()) await endTurn.click();
  await page.waitForTimeout(250);
}
const night = page.getByTestId("election-night");
await night.waitFor();
await page.waitForTimeout(800);
await both("night");
facts.election = (await night.innerText()).split("\n").filter(Boolean);
await phone.close();
await browser.close();

await writeFile(`${out}facts.json`, JSON.stringify(facts, null, 2));
console.log(JSON.stringify(facts, null, 2));
