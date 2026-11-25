import { expect, test } from "./csp";
import type { Browser, Page } from "@playwright/test";
import { useWideFont } from "./wide-font";

// M6 note 11 (DEC-086, DEC-089's pattern): the town map's picker lists the
// engine's town map layers: districts with whoever holds each seat, the
// people the player has heard of where the player knows to find them, and
// what each neighbourhood worries about. Each is drawn over the map with a
// legend under it and has a list as its text alternative (DEC-051). A
// portrait opens the person's card, with their full card and "Go to them".
// Nothing the engine names is shown raw, and nothing overflows at phone
// widths with wide fonts.
//
// TOWN_LAYER_SHOTS=<dir> also saves screenshots of each layer at 390 px and
// one at desktop width, at twice the pixel density (the last test); without
// it that test is skipped.

const LAYERS = ["districts", "people", "worries"] as const;
const KIND = { districts: "boundaries", people: "markers", worries: "shaded" } as const;

const start = async (page: Page, seed: string) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill(seed);
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
  await expect(page.locator(".town-street-stage[data-ready='1']")).toBeVisible({ timeout: 30_000 });
};

/** Open the town map, with the engine's layers loaded. */
const openMap = async (page: Page) => {
  await page.getByTestId("town-map-button").click();
  await expect(page.getByTestId("town-map-overlay")).toBeVisible();
  await expect(page.getByTestId("town-map-canvas")).toBeVisible();
  await expect(page.getByTestId("town-map")).toHaveAttribute("data-layers", String(LAYERS.length), { timeout: 15_000 });
};

const choose = async (page: Page, key: (typeof LAYERS)[number]) => {
  await page.getByTestId("town-map-layer").selectOption(key);
  await expect(page.getByTestId("town-map")).toHaveAttribute("data-layer", key);
  await expect(page.getByTestId("town-map-layer-overlay")).toHaveAttribute("data-kind", KIND[key]);
  await expect(page.getByTestId("town-map-legend")).toBeVisible();
};

const watchErrors = (page: Page) => {
  const errors: string[] = [];
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(String(e)));
  return errors;
};

/** No raw Fluent key, content id, placeholder or missing-key marker in what the map overlay shows. */
const expectNoRawKeys = async (page: Page, what: string) => {
  const text = await page.getByTestId("town-map-overlay").innerText();
  const labels = await page
    .getByTestId("town-map-overlay")
    .locator("[aria-label], [title]")
    .evaluateAll((els) => els.map((e) => `${e.getAttribute("aria-label") ?? ""} ${e.getAttribute("title") ?? ""}`).join("\n"));
  for (const [where, s] of [
    ["text", text],
    ["labels", labels],
  ] as const) {
    const bad = s.match(/[^\n]*(map-layer-|town_map_layer|concern\.|concern-|\{ ?\$|⟦|\bui-[a-z]|\btown-map-)[^\n]*/);
    expect(bad?.[0] ?? null, `${what}: raw key in the ${where}`).toBeNull();
  }
};

/** Nothing of the layer's controls, panel or list runs past the viewport, and the page never scrolls sideways. */
const expectFits = async (page: Page, what: string) => {
  const problems = await page.evaluate(() => {
    const out: string[] = [];
    const vw = document.documentElement.clientWidth;
    if (document.documentElement.scrollWidth > vw + 1) out.push(`the page scrolls sideways: ${document.documentElement.scrollWidth} > ${vw}`);
    for (const sel of ["[data-testid=town-map-bar]", "[data-testid=town-map-panel]", "[data-testid=town-map-list]"]) {
      const root = document.querySelector<HTMLElement>(sel);
      if (!root) continue;
      if (root.scrollWidth > root.clientWidth + 1) out.push(`${sel} scrolls sideways: ${root.scrollWidth} > ${root.clientWidth}`);
      for (const el of [root, ...root.querySelectorAll<HTMLElement>("*")]) {
        const r = el.getBoundingClientRect();
        if (r.width === 0 || r.height === 0) continue;
        if (r.right > vw + 1 || r.left < -1) out.push(`${sel} ${el.tagName.toLowerCase()}.${el.className}: ${Math.round(r.left)}–${Math.round(r.right)} of ${vw}`);
      }
    }
    // Legend entries and card lines wrap; none is cut with an ellipsis.
    for (const el of document.querySelectorAll<HTMLElement>(".tml-bins li, .tml-legend-head, .tml-note, .tml-card-text p, .tml-card-name, .tml-readout-text")) {
      if (el.scrollWidth > el.clientWidth + 1) out.push(`${el.className} is cut: ${el.scrollWidth} > ${el.clientWidth}`);
    }
    return out;
  });
  expect(problems, what).toEqual([]);
};

test("the town map's layers: districts, people and worries, a member's card, and the list", async ({ page }) => {
  test.setTimeout(180_000);
  const errors = watchErrors(page);
  await start(page, "31337");
  await openMap(page);
  const picker = page.getByTestId("town-map-layer");
  // The plain map first, then the registry's layers in its order.
  await expect(picker.locator("option")).toHaveText(["Places", "Wards", "People", "Worries"]);
  await expect(page.getByTestId("town-map")).toHaveAttribute("data-layer", "none");
  await expect(page.getByTestId("town-map-legend")).toHaveCount(0);

  // --- Districts: each named in the constitution's words, the player's own marked, a portrait per seat.
  await choose(page, "districts");
  const areas = page.getByTestId("town-map-area");
  expect(await areas.count()).toBeGreaterThan(1);
  await expect(areas.first()).toHaveText(/^Ward \d+/);
  await expect(page.locator("[data-testid=town-map-area][data-yours=true], [data-testid=town-map-area][data-home=true]")).toHaveCount(1);
  await expect(page.getByTestId("town-map-legend")).toHaveAccessibleName("Legend: Wards");
  await expectNoRawKeys(page, "districts");
  // A member's portrait (not the player's own, which offers no way to oneself), from the keyboard: their card,
  // with the way to them.
  const member = page.locator("[data-testid=town-map-person][data-member=true]:not([data-you=true])").first();
  await expect(member).toHaveAttribute("aria-label", /.+, Councillor/);
  await member.focus();
  await page.keyboard.press("Enter");
  const card = page.getByTestId("town-map-person-card");
  await expect(card).toBeVisible();
  const name = (await card.getByRole("heading").innerText()).trim();
  expect(name.length).toBeGreaterThan(1);
  await expect(card).toContainText("Councillor · Ward");
  await expect(card.getByTestId("town-map-person-go")).toBeVisible();
  await expectNoRawKeys(page, "a member's card");
  // Their full card: the map closes for the sheet.
  await card.getByTestId("town-map-person-open").click();
  await expect(page.getByTestId("town-map-overlay")).toHaveCount(0);
  const sheet = page.getByTestId("character-sheet");
  await expect(sheet).toBeVisible();
  await expect(sheet.getByRole("heading").first()).toHaveText(name);
  await sheet.getByRole("button", { name: "Close" }).click();
  await expect(sheet).toHaveCount(0);

  // The layer chosen is remembered on this device.
  await openMap(page);
  await expect(page.getByTestId("town-map")).toHaveAttribute("data-layer", "districts");
  // A district's name reads it out: its neighbourhoods and its member.
  await areas.first().focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("town-map-readout")).toContainText(/Ward \d+.*Covers /s);

  // --- People: portraits where the player knows to find them; several at one place share a mark.
  await choose(page, "people");
  await expect(page.getByTestId("town-map-legend")).toHaveAccessibleName("Legend: People");
  const marks = page.locator("[data-testid=town-map-person], [data-testid=town-map-cluster]");
  await expect(marks.first()).toBeVisible();
  const filter = page.getByTestId("town-map-filter");
  await expect(filter.locator("option").first()).toHaveText(/^Everyone \(\d+\)$/);
  await expect(filter.locator("option[value=body]")).toHaveText(/^Town council \(\d+\)$/);
  const cluster = page.getByTestId("town-map-cluster").first();
  if ((await cluster.count()) > 0) {
    await expect(cluster).toHaveAttribute("aria-label", /^\d+ people at .+/);
    await cluster.focus();
    await page.keyboard.press("Enter");
    const group = page.getByTestId("town-map-cluster-list");
    await expect(group).toBeVisible();
    await expect(group.getByTestId("town-map-row-person").first()).toBeVisible();
    await group.getByTestId("town-map-row-person").first().click();
    await expect(page.getByTestId("town-map-person-card")).toBeVisible();
    await page.getByTestId("town-map-person-close").click();
  }
  await filter.selectOption("body");
  await expect(page.getByTestId("town-map-legend")).toBeVisible();
  await expectNoRawKeys(page, "people");

  // --- Worries: every neighbourhood named with its reading; what is not known says so.
  await choose(page, "worries");
  await expect(page.getByTestId("town-map-legend")).toHaveAccessibleName("Legend: Worries");
  const hoods = page.getByTestId("town-map-hood");
  expect(await hoods.count()).toBeGreaterThan(1);
  const param = page.getByTestId("town-map-param");
  await expect(param.locator("option").first()).toHaveText("Top worry");
  expect(await param.locator("option").count()).toBeGreaterThan(2);
  await expect(page.getByTestId("town-map-readout")).toHaveText("Tap a neighbourhood to read it.");
  await page.locator("[data-testid=town-map-hood][data-shown=\"1\"]").first().focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("town-map-readout-value")).toBeVisible();
  await expectNoRawKeys(page, "worries: top");
  await param.selectOption({ index: 1 });
  for (const h of await page.locator("[data-testid=town-map-hood][data-known=false]").all()) await expect(h).toContainText("Not known yet");
  await expectNoRawKeys(page, "worries: one worry");

  // --- The list: the same data as text, for each layer.
  for (const key of LAYERS) {
    await choose(page, key);
    await page.getByTestId("town-map-list-toggle").click();
    const list = page.getByTestId("town-map-list");
    await expect(list).toBeVisible();
    await expect(list.locator(".tml-list-item").first()).toBeVisible();
    await expectNoRawKeys(page, `${key} as a list`);
    if (key === "districts") {
      // A name in the list opens the same card.
      await list.getByTestId("town-map-row-person").first().click();
      await expect(page.getByTestId("town-map-person-card")).toBeVisible();
      await page.getByTestId("town-map-person-close").click();
    }
    await page.getByTestId("town-map-list-toggle").click();
    await expect(list).toHaveCount(0);
  }

  // Back to the plain map: no layer, no legend, and the places still work.
  await picker.selectOption("none");
  await expect(page.getByTestId("town-map-layer-overlay")).toHaveCount(0);
  await expect(page.getByTestId("town-map-legend")).toHaveCount(0);
  expect(errors).toEqual([]);
});

test("Go to them walks the player to where the person is", async ({ page }) => {
  test.setTimeout(120_000);
  const errors = watchErrors(page);
  await start(page, "31337");
  await openMap(page);
  await choose(page, "districts");
  const member = page.locator("[data-testid=town-map-person][data-member=true]:not([data-you=true])").first();
  await member.focus();
  await page.keyboard.press("Enter");
  await page.getByTestId("town-map-person-go").click();
  // The map closes; the player walks there in the street, or the talk panel says where they are.
  await expect(page.getByTestId("town-map-overlay")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "End turn" })).toBeVisible();
  expect(errors).toEqual([]);
});

for (const width of [320, 360, 390, 1280] as const) {
  test(`the layers fit at ${width}px${width < 1000 ? " with wide fonts" : ""}`, async ({ page }, info) => {
    test.skip(info.project.name !== (width < 1000 ? "phone" : "desktop"), "one project per width");
    test.setTimeout(180_000);
    if (width < 1000) await useWideFont(page);
    await page.setViewportSize({ width, height: width < 1000 ? 740 : 800 });
    await start(page, "31337");
    await openMap(page);
    for (const key of LAYERS) {
      await choose(page, key);
      await expectFits(page, `${key} at ${width}`);
      await expectNoRawKeys(page, `${key} at ${width}`);
      // The map keeps most of the screen beside the legend.
      const map = await page.locator(".town-map-wrap").boundingBox();
      expect(map?.height ?? 0, `${key} at ${width}: the map's height`).toBeGreaterThan(180);
      if (key === "districts") {
        await page.locator("[data-testid=town-map-person][data-member=true]").first().focus();
        await page.keyboard.press("Enter");
        await expect(page.getByTestId("town-map-person-card")).toBeVisible();
        await expectFits(page, `a person's card at ${width}`);
        await page.getByTestId("town-map-person-close").click();
      }
      if (key === "worries") {
        await page.locator("[data-testid=town-map-hood][data-shown=\"1\"]").first().focus();
        await page.keyboard.press("Enter");
        await expect(page.getByTestId("town-map-readout-value")).toBeVisible();
        await expectFits(page, `a neighbourhood's reading at ${width}`);
      }
      await page.getByTestId("town-map-list-toggle").click();
      await expect(page.getByTestId("town-map-list")).toBeVisible();
      await expectFits(page, `${key} as a list at ${width}`);
      await page.getByTestId("town-map-list-toggle").click();
    }
  });
}

/** A page of its own at twice the pixel density (screenshots). */
const sharp = async (browser: Browser, baseURL: string | undefined, width: number, height: number) => {
  const context = await browser.newContext({ viewport: { width, height }, deviceScaleFactor: 2, ...(baseURL ? { baseURL } : {}) });
  return { context, page: await context.newPage() };
};

test("screenshots of each layer", async ({ browser, baseURL }, info) => {
  const dir = process.env.TOWN_LAYER_SHOTS;
  test.skip(!dir, "set TOWN_LAYER_SHOTS to a directory to save them");
  test.skip(info.project.name !== "desktop", "one run");
  test.setTimeout(300_000);
  const seed = process.env.TOWN_LAYER_SEED ?? "31337";
  const phone = await sharp(browser, baseURL, 390, 844);
  await start(phone.page, seed);
  // A few turns in, so there is something learned and someone seen.
  await openMap(phone.page);
  for (const key of LAYERS) {
    await choose(phone.page, key);
    if (key === "worries") {
      await phone.page.locator("[data-testid=town-map-hood][data-shown=\"1\"]").first().focus();
      await phone.page.keyboard.press("Enter");
    }
    await phone.page.waitForTimeout(900);
    await phone.page.screenshot({ path: `${dir}/town-${key}-390.png` });
  }
  await choose(phone.page, "districts");
  await phone.page.locator("[data-testid=town-map-person][data-member=true]:not([data-you=true])").first().focus();
  await phone.page.keyboard.press("Enter");
  await phone.page.waitForTimeout(600);
  await phone.page.screenshot({ path: `${dir}/town-districts-card-390.png` });
  await choose(phone.page, "worries");
  await phone.page.getByTestId("town-map-param").selectOption({ index: 1 });
  await phone.page.waitForTimeout(600);
  await phone.page.screenshot({ path: `${dir}/town-worries-one-390.png` });
  await choose(phone.page, "people");
  await phone.page.getByTestId("town-map-list-toggle").click();
  await phone.page.waitForTimeout(400);
  await phone.page.screenshot({ path: `${dir}/town-people-list-390.png` });
  await phone.context.close();

  // Someone who grew up in the town ("Start with local knowledge"): people across the town, worries known.
  const local = await sharp(browser, baseURL, 390, 844);
  await local.page.goto("./");
  await local.page.getByPlaceholder("e.g. 20260").fill(seed);
  await local.page.getByTestId("local-knowledge-toggle").check();
  await local.page.getByRole("button", { name: "Begin" }).click();
  await expect(local.page.locator(".town-street-stage[data-ready='1']")).toBeVisible({ timeout: 30_000 });
  await expect(local.page.getByRole("button", { name: "End turn" })).toBeEnabled();
  await openMap(local.page);
  for (const key of ["people", "worries"] as const) {
    await choose(local.page, key);
    await local.page.waitForTimeout(900);
    await local.page.screenshot({ path: `${dir}/town-${key}-local-390.png` });
  }
  await choose(local.page, "people");
  await local.page.getByRole("button", { name: "Zoom in" }).click();
  await local.page.waitForTimeout(900);
  await local.page.screenshot({ path: `${dir}/town-people-local-zoomed-390.png` });
  await choose(local.page, "worries");
  await local.page.getByTestId("town-map-param").selectOption({ index: 1 });
  await local.page.waitForTimeout(700);
  await local.page.screenshot({ path: `${dir}/town-worries-one-local-390.png` });
  await local.context.close();

  // The narrowest phone with a wide face, as the fit test sees it.
  const narrow = await sharp(browser, baseURL, 320, 740);
  await useWideFont(narrow.page);
  await start(narrow.page, seed);
  await openMap(narrow.page);
  for (const key of LAYERS) {
    await choose(narrow.page, key);
    await narrow.page.waitForTimeout(700);
    await narrow.page.screenshot({ path: `${dir}/town-${key}-320-wide.png` });
  }
  await narrow.context.close();

  const desk = await sharp(browser, baseURL, 1280, 800);
  await start(desk.page, seed);
  await openMap(desk.page);
  for (const key of LAYERS) {
    await choose(desk.page, key);
    await desk.page.waitForTimeout(900);
    await desk.page.screenshot({ path: `${dir}/town-${key}-1280.png` });
  }
  await desk.context.close();
});
