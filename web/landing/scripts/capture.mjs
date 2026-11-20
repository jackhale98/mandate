// Captures the landing page's images from the running game (DEC-083): the
// walkable street, a conversation, a room with people in it, the notebook,
// the country map and its layers, and a real election night. Nothing here is
// mocked up. One command does it all and is meant to be re-run whenever the
// game's screens change (the M6 follow-up in DEC-083):
//
//   npm run capture                    # build, serve, capture, encode, share image
//   npm run capture -- --no-build      # reuse web/dist
//   npm run capture -- --only=street,talk --no-encode   # while iterating
//
// It needs the engine's WASM (scripts/build-wasm.sh), Playwright's Chromium,
// and Python 3 with Pillow (AVIF and WebP) for encode.py. PW_PORT picks the
// preview port (default 4173, as in playwright.config.ts).
//
// Deterministic by construction: fixed seeds, viewports and themes, reduced
// motion, and the page's clock (Date, timers, requestAnimationFrame) paused
// at a fixed instant and only moved on by fixed steps here, so the townsfolk,
// the light and the camera are where they were last time. Raw PNGs and
// facts.json go to ../target/landing-capture/ (not committed); encode.py
// writes the committed AVIF/WebP files to landing/img/, and og.mjs the share
// image to landing/public/og.png. If the election figures in facts.json
// change (the engine changed), update the results box in landing/index.html.
import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { chromium } from "@playwright/test";

const web = fileURLToPath(new URL("../../", import.meta.url));
const out = fileURLToPath(new URL("../../../target/landing-capture/", import.meta.url));
const args = process.argv.slice(2);
const flag = (name) => args.includes(`--${name}`);
const only = args.find((a) => a.startsWith("--only="))?.slice(7).split(",") ?? null;
const wanted = (name) => only === null || only.includes(name);
const port = Number(process.env.PW_PORT ?? 4173);
const base = process.env.MANDATE_BASE ?? "/mandate/";
const app = `http://localhost:${port}${base}play/`;

/** The run the street, room, notebook and election pictures come from. */
const STORY_SEED = "4040";
/** Seeds and scenarios for the map strip; the first is the front-page map. */
const MAPS = [
  { seed: "4040", scenario: "" },
  { seed: "777", scenario: "Resource-rich frontier" },
  { seed: "20260", scenario: "Drought-prone farmland" },
  { seed: "1848", scenario: "Wealthy coastal nation" },
];
/** The data layer shown on the front-page country. */
const LAYER = "unemployment";
/** The page's clock: paused here, and moved on only by `tick`. */
const EPOCH = Date.UTC(2026, 0, 5, 9, 0, 0);

const PHONE = { viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true };
const DESKTOP = { viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2 };

const run = (cmd, argv, opts = {}) =>
  new Promise((resolve, reject) => {
    const p = spawn(cmd, argv, { cwd: web, stdio: "inherit", ...opts });
    p.on("exit", (code) => (code === 0 ? resolve() : reject(new Error(`${cmd} ${argv.join(" ")} exited ${code}`))));
  });

/** Serve the built site (the landing page and play/) as `npm run preview` does, on PW_PORT. */
async function serve() {
  const up = await fetch(app).then(
    () => true,
    () => false,
  );
  if (up) throw new Error(`something is already serving port ${port}: set PW_PORT, or pass --no-serve to use it`);
  const p = spawn(process.execPath, ["node_modules/vite/bin/vite.js", "preview", "-c", "landing/vite.config.ts", "--port", String(port), "--strictPort"], {
    cwd: web,
    stdio: ["ignore", "ignore", "inherit"],
  });
  for (let i = 0; i < 100; i++) {
    if (p.exitCode !== null) throw new Error(`vite preview exited (is port ${port} in use? set PW_PORT)`);
    try {
      if ((await fetch(app)).ok) return p;
    } catch {
      // not up yet
    }
    await new Promise((r) => setTimeout(r, 200));
  }
  p.kill();
  throw new Error("vite preview did not start");
}

/** A fresh game in a fresh context with the clock paused. */
async function begin(browser, device, seed, scenario = "") {
  const ctx = await browser.newContext({ ...device, colorScheme: "dark", reducedMotion: "reduce", locale: "en-GB" });
  const page = await ctx.newPage();
  page.on("pageerror", (e) => console.warn(`page error: ${e.message}`));
  await page.clock.install({ time: EPOCH });
  await page.clock.pauseAt(EPOCH + 1000);
  await page.goto(app);
  await page.getByPlaceholder("e.g. 20260").fill(seed);
  if (scenario) await page.getByLabel("Scenario").selectOption({ label: scenario });
  await page.getByRole("button", { name: "Begin" }).click();
  await settle(page, page.locator(".town-street-stage[data-ready='1']"));
  await tick(page, 1500);
  return { ctx, page };
}

/**
 * Wait for something the engine or a worker makes (real time, the clock still
 * paused); if it still waits after a few seconds, it waits on a timer of the
 * page's (the save store sometimes does), so nudge the clock by small steps.
 * The nudges can move the townsfolk, which is why the street resets them
 * (`resetTownsfolk`) before its pictures.
 */
async function settle(page, locator, ms = 60_000) {
  const end = Date.now() + ms;
  let nudges = 0;
  for (let i = 0; Date.now() < end; i++) {
    if (await locator.isVisible()) {
      if (nudges && process.env.CAPTURE_DEBUG) console.warn(`${locator}: ${nudges} nudges`);
      return;
    }
    await page.waitForTimeout(100);
    // Give real-time work a few seconds before deciding it waits on a timer.
    if (i >= 30 && i % 10 === 0) {
      nudges++;
      await tick(page, 50);
    }
  }
  throw new Error(`timed out waiting for ${locator}`);
}

/**
 * Move the page's clock on by `ms` (frames at 60 a second, timers in order).
 * Real-time events (the engine's replies, CSS animations ending, resize and
 * theme changes) are let in first, so they land at the same page time.
 */
async function tick(page, ms) {
  await page.waitForTimeout(120);
  await page.clock.runFor(ms);
}

/** Wait (real time) until a command in flight is done: End turn is enabled again when the game is not busy. */
async function quiet(page) {
  const endTurn = page.getByRole("button", { name: "End turn" });
  for (let i = 0; i < 100 && (await endTurn.isVisible().catch(() => false)) && !(await endTurn.isEnabled().catch(() => true)); i++) {
    await page.waitForTimeout(100);
  }
}

/** A screenshot in both themes (the landing page shows the one that matches its own). */
async function both(page, name, opts = {}) {
  // The townsfolk hold still while the theme changes (as they do in a
  // conversation), so both pictures show them in the same places.
  const held = await page.evaluate(() => {
    const scene = window.__townStreet?.view.scene;
    if (!scene) return null;
    const was = scene.folkPaused;
    scene.folkPaused = true;
    return was;
  });
  for (const scheme of ["dark", "light"]) {
    await quiet(page);
    await retheme(page, scheme);
    await (opts.locator ?? page).screenshot({ path: `${out}${name}-${scheme}.png`, animations: "disabled" });
  }
  await retheme(page, "dark");
  if (held !== null) {
    await page.evaluate((was) => {
      const scene = window.__townStreet?.view.scene;
      if (scene) scene.folkPaused = was;
    }, held);
  }
}

/**
 * Switch the theme and let everything repaint: React hears the change in its
 * own time (real time, not the page clock), then the canvases redraw on the
 * next frames. Two rounds, so a redraw asked for late is still drawn.
 */
async function retheme(page, scheme) {
  await page.emulateMedia({ colorScheme: scheme, reducedMotion: "reduce" });
  for (let i = 0; i < 2; i++) {
    await page.waitForTimeout(250);
    await tick(page, 300);
  }
}

async function pocketTab(page, tab) {
  if ((await page.getByTestId("pocket").count()) === 0) await page.getByTestId("pocket-button").click();
  const pocket = page.getByTestId("pocket");
  await pocket.waitFor();
  await pocket.getByTestId(`pocket-tab-${tab}`).click();
  await tick(page, 500);
  return pocket;
}

async function closePocket(page) {
  const pocket = page.getByTestId("pocket");
  if ((await pocket.count()) === 0) return;
  await pocket.getByRole("button", { name: "Close" }).first().click();
  await tick(page, 800);
  await pocket.waitFor({ state: "detached" });
}

/** To the streets of the first neighbourhood (the town centre), where passers-by stand, and look at the street. */
async function toStreets(page) {
  const pocket = await pocketTab(page, "places");
  await pocket.getByTestId("spot-streets").first().click();
  const panel = page.getByTestId("place-panel");
  await settle(page, panel.getByTestId("you-are-here-panel"));
  await tick(page, 500);
  await panel.getByRole("button", { name: "Close" }).first().click();
  await tick(page, 800);
  await panel.waitFor({ state: "detached" });
  await closePocket(page);
  await resetTownsfolk(page);
  await tick(page, 1500);
}

/**
 * Put the ambient townsfolk back where a new street starts them (the scene's
 * own seeded stream), so where they stand in the picture does not depend on
 * how long the engine took to answer on the way here. Presentation only: the
 * townsfolk are decoration, the engine never sees them.
 */
async function resetTownsfolk(page) {
  await page.evaluate(() => {
    const scene = window.__townStreet?.view.scene;
    if (!scene) throw new Error("no street");
    const fresh = new scene.constructor(scene.map, { townsfolk: scene.folk.length });
    scene.folk.splice(0, scene.folk.length, ...fresh.folk);
    scene.rng = fresh.rng;
  });
}

/** Talk to the first passer-by here: the camera frames the two of them; ask who runs things and show the reply. */
async function talkInStreet(page, desktop) {
  if (desktop) {
    // Stand beside them and press Enter (the keyboard way).
    await page.evaluate(() => {
      const v = window.__townStreet?.view;
      const s = v?.scene.standing[0];
      if (v && s) v.scene.goToPerson(s.key);
    });
    await tick(page, 4000);
    await page.locator(".town-street-stage canvas").focus();
    await tick(page, 200);
    await page.keyboard.press("Enter");
  } else {
    await page.locator(".hub-person-tag").first().click();
    await tick(page, 4000);
  }
  const box = page.getByTestId("talk-panel");
  await settle(page, box);
  await tick(page, 1500);
  await box.getByTestId("talk-topic-topic.who_runs").locator("button.dlg-choice").click();
  // The question holds the stage, then the engine's reply takes it.
  await tick(page, 1200);
  const live = page.getByTestId("talk-reply-live");
  for (let i = 0; i < 300 && !(await live.textContent())?.trim(); i++) await page.waitForTimeout(50);
  await tick(page, 1500);
  return box;
}

/** Look at a place's building and tap it: the player walks to its door and goes in. */
async function enterRoom(page, kind, desktop) {
  const at = await page.evaluate((k) => {
    const v = window.__townStreet?.view;
    const b = v?.map.buildings.find((x) => x.kind === k && x.place !== null);
    if (!v || !b) return null;
    v.centreOn(b.x + b.w / 2, b.y + b.h / 2);
    return v.toScreen(b.x + b.w / 2, b.y + b.h / 2);
  }, kind);
  if (!at) throw new Error(`no ${kind} in the street`);
  await tick(page, 300);
  const box = await page.locator(".town-street-stage").boundingBox();
  if (desktop) await page.mouse.click(box.x + at.x, box.y + at.y);
  else await page.touchscreen.tap(box.x + at.x, box.y + at.y);
  await tick(page, 8000);
  await settle(page, page.locator(`[data-testid='interior'][data-recipe='interior.${kind}'][data-ready='1']`));
  await tick(page, 2000);
}

async function leaveRoom(page) {
  await page.getByTestId("interior-leave").click();
  await tick(page, 2000);
  await page.getByTestId("interior").waitFor({ state: "detached" });
  await tick(page, 1000);
}

async function openCountry(page) {
  await page.getByTestId("town-map-button").click();
  await page.getByTestId("town-map-overlay").waitFor();
  await tick(page, 300);
  await page.getByTestId("map-scope-country").click();
  await page.getByTestId("country-map").waitFor();
  const canvas = page.getByTestId("country-map").locator("canvas");
  await canvas.waitFor();
  await tick(page, 1500);
  // The whole country: zoom all the way out (the map then centres it), then
  // in to the fitted scale (the wheel's step is exp(0.0015 per unit)).
  const box = await canvas.boundingBox();
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.wheel(0, 2000);
  await tick(page, 300);
  await page.mouse.wheel(0, -Math.log(1 / 0.8) / 0.0015);
  await tick(page, 1000);
  return page.getByTestId("country-map");
}

/** Tap the player's own town on the country map: the legend reads out its figure. */
async function tapHome(page, desktop) {
  const canvas = page.getByTestId("country-map").locator("canvas");
  await tick(page, 1000);
  const [x, y] = ((await canvas.getAttribute("data-home-centre")) ?? "0,0").split(",").map(Number);
  const box = await canvas.boundingBox();
  if (desktop) await page.mouse.click(box.x + x, box.y + y);
  else await page.touchscreen.tap(box.x + x, box.y + y);
  await tick(page, 800);
}

// ---------------------------------------------------------------------------

const facts = { maps: [], election: null, street: null, talk: null, room: null };
await mkdir(out, { recursive: true });
if (!flag("no-build")) await run("npm", ["run", "build"]);
const server = flag("no-serve") ? null : await serve();
const browser = await chromium.launch(process.env.PW_CHROMIUM_PATH ? { executablePath: process.env.PW_CHROMIUM_PATH } : {});
try {
  // Phone: the street, a conversation in it, a room with people, the notebook.
  if (["street", "talk", "room", "notebook"].some(wanted)) {
    const { ctx, page } = await begin(browser, PHONE, STORY_SEED);
    await toStreets(page);
    facts.street = (await page.getByTestId("where").innerText()).replace(/\s+/g, " ").trim();
    if (wanted("street")) await both(page, "street");
    const box = await talkInStreet(page, false);
    facts.talk = (await page.getByTestId("talk-reply-live").textContent())?.trim() ?? null;
    if (wanted("talk")) await both(page, "talk");
    await box.getByTestId("talk-goodbye").locator("button.dlg-choice").click();
    await tick(page, 1500);
    await box.waitFor({ state: "detached" });
    await enterRoom(page, "council_chamber", false);
    facts.room = await page.evaluate(() => window.__interior?.people.map((p) => p.label ?? p.role) ?? []);
    if (wanted("room")) await both(page, "room");
    await leaveRoom(page);
    if (wanted("notebook")) {
      await pocketTab(page, "notebook");
      await tick(page, 800);
      await both(page, "notebook");
    }
    await ctx.close();
  }

  // Desktop: the street with the slim bar and the pocket docked beside it.
  if (wanted("desktop")) {
    const { ctx, page } = await begin(browser, DESKTOP, STORY_SEED);
    await toStreets(page);
    await pocketTab(page, "today");
    await both(page, "desktop");
    await ctx.close();
  }

  // The country with a data layer, in a small computer window (so the legend
  // can still be read in a tile on the landing page).
  if (wanted("layer")) {
    const { ctx, page } = await begin(browser, { viewport: { width: 1024, height: 700 }, deviceScaleFactor: 2 }, STORY_SEED);
    const map = await openCountry(page);
    await page.getByTestId("map-layer").selectOption(LAYER);
    await tapHome(page, true);
    await both(page, "layer", { locator: map });
    await ctx.close();
  }

  // Maps: terrain, a landscape window that fits the whole country.
  if (wanted("maps")) {
    for (const m of MAPS) {
      const { ctx, page } = await begin(browser, { viewport: { width: 1440, height: 1000 }, deviceScaleFactor: 2 }, m.seed, m.scenario);
      const town = (await page.getByTestId("where-zone").textContent().catch(() => ""))?.trim();
      const map = await openCountry(page);
      await map.locator("canvas").screenshot({ path: `${out}map-${m.seed}.png` });
      facts.maps.push({ ...m, town });
      await ctx.close();
    }
  }

  // A real election night, from a run where the player does nothing: End
  // turn until the count, each turn's passage moved on by the same steps.
  if (wanted("night")) {
    const { ctx, page } = await begin(browser, PHONE, STORY_SEED);
    const endTurn = page.getByRole("button", { name: "End turn" });
    const skip = page.getByText("Tap to carry on");
    const night = page.getByTestId("election-night");
    let nudges = 0;
    for (let i = 0; i < 20_000 && (await night.count()) === 0; i++) {
      if (await skip.isVisible().catch(() => false)) {
        await skip.click();
        await tick(page, 300);
      } else if (await endTurn.isEnabled().catch(() => false)) {
        await endTurn.click();
        await tick(page, 400);
      } else {
        await page.waitForTimeout(50);
        if (i % 40 === 39) {
          nudges++;
          await tick(page, 50);
        }
      }
    }
    if (nudges && process.env.CAPTURE_DEBUG) console.warn(`election night: ${nudges} nudges`);
    await night.waitFor();
    await tick(page, 1000);
    await both(page, "night");
    facts.election = (await night.innerText()).split("\n").filter(Boolean);
    await ctx.close();
  }
} finally {
  await browser.close();
  server?.kill();
}

await writeFile(`${out}facts.json`, JSON.stringify(facts, null, 2));
console.log(JSON.stringify(facts, null, 2));
if (!flag("no-encode")) {
  await run("python3", ["landing/scripts/encode.py"]);
  await run("node", ["landing/scripts/og.mjs"]);
}
