import { mkdirSync } from "node:fs";
import { expect, test, type Page } from "@playwright/test";
import { openPocket, showPlaces } from "./pocket";

// Talking to people (DEC-093) and canvassing on a worry (DEC-091): the talk
// panel from the pocket's People tab, from a place's people and from the
// walkable street (walking up to someone: a tap, or Enter beside them with
// the keyboard); topics with the engine's odds and the cost; the reply as a
// dialogue, then what was learned (with a lead). The canvass form offers the
// worries the player knows of, "no particular issue" first.
// TALK_SCREENS=<dir> also saves screenshots there (light and dark).
const SCREENS = process.env.TALK_SCREENS ?? "";

interface Standing {
  key: string;
  w: { x: number; y: number };
}
interface Probes {
  __townStreet?: {
    view: {
      map: { width: number };
      scene: {
        standing: Standing[];
        player: { x: number; y: number; walk: unknown };
        reach?: ArrayLike<number>;
        placePlayer(cell: number): void;
        goToPerson(key: string): boolean;
        personNear(): string | null;
      };
      toScreen(x: number, y: number): { x: number; y: number };
      centreOn(x: number, y: number): void;
      wake(): void;
    };
  } | null;
}

const start = async (page: Page, seed: string) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill(seed);
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
};

const streetReady = (page: Page) => expect(page.locator(".town-street-stage[data-ready='1']")).toBeVisible({ timeout: 30_000 });

const shot = async (page: Page, name: string, isMobile: boolean) => {
  if (!SCREENS) return;
  mkdirSync(SCREENS, { recursive: true });
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme, reducedMotion: "reduce" });
    await page.waitForTimeout(150);
    await page.screenshot({ path: `${SCREENS}/${name}-${isMobile ? "phone" : "desktop"}-${scheme}.png` });
  }
};

/** Talk on a topic in the open panel; returns the exchange. */
const ask = async (page: Page, topic: string) => {
  const panel = page.getByTestId("talk-panel");
  const t = panel.getByTestId(`talk-topic-${topic}`);
  await expect(t).toBeVisible();
  const ask = t.getByTestId("talk-ask");
  await ((await ask.count()) > 0 ? ask : t.getByRole("button")).click();
  const x = panel.getByTestId("talk-exchange");
  await expect(x).toBeVisible();
  await expect(x.getByTestId("talk-dialogue").locator("p").first()).not.toBeEmpty();
  return x;
};

/** Go to the streets of the first neighbourhood (the Places tab), where passers-by stand. */
const toStreets = async (page: Page) => {
  const pocket = await showPlaces(page);
  await pocket.getByTestId("spot-streets").first().click();
  const panel = page.getByTestId("place-panel");
  await expect(panel.getByTestId("you-are-here-panel")).toBeVisible();
  return panel;
};

test("talk from the People tab: someone elsewhere is walked to; topics with odds and the cost; a reply in their voice", async ({ page, isMobile }) => {
  await start(page, "7311");
  const pocket = await openPocket(page, "people");
  await pocket.getByTestId("people-council").getByTestId("talk-button").first().click();
  const panel = page.getByTestId("talk-panel");
  await expect(panel).toBeVisible();
  await expect(panel.getByRole("heading", { name: /^Talking with / })).toBeVisible();
  // Not here: where they are, and a way there (free); then the topics can be asked.
  await expect(panel.getByTestId("talk-away")).toBeVisible();
  await expect(panel.getByTestId("talk-why").first()).toHaveText("They aren't here. Go to them first.");
  await expect(panel.getByTestId("talk-odds")).toHaveCount(0);
  await panel.getByTestId("talk-go").click();
  await expect(panel.getByTestId("talk-away")).toHaveCount(0);
  await expect(panel.getByTestId("talk-cost")).toHaveText(/half a slot/);
  // A pitch or an ask for support shows the engine's odds where it rolls.
  await expect(panel.getByTestId("talk-odds").first()).toHaveText(/^\d+% chance$/);
  await shot(page, "talk-topics", isMobile);
  const x = await ask(page, "topic.who_runs");
  await expect(x.getByTestId("talk-you")).toHaveText("You: Who really runs things here?");
  await expect(x.getByRole("heading", { name: "What you learned" })).toBeVisible();
  await expect(x).not.toContainText(/\b(lead|talk|notebook|ui)-[a-z]/);
  // The half slot left: the cost line says so; the next conversation uses it.
  await expect(panel.getByTestId("talk-cost")).toHaveText(/half slot left/);
  await ask(page, "topic.small_talk");
  await expect(panel.getByTestId("talk-cost").or(panel.getByTestId("talk-out-of-time"))).toBeVisible();
  // Escape closes it.
  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
});

test("talk to someone at a place, from the place panel's people", async ({ page }) => {
  await start(page, "7311");
  const pocket = await showPlaces(page);
  await pocket.getByTestId("spot-council_chamber").first().click();
  const place = page.getByTestId("place-panel");
  await expect(place.getByTestId("you-are-here-panel")).toBeVisible();
  // The chamber's members are in the whip list; the town hall has the mayor. Try the diner's people instead if none.
  const talk = place.getByTestId("talk-button");
  if ((await talk.count()) === 0) {
    const p2 = await showPlaces(page);
    await p2.getByTestId("spot-diner").first().click();
    await expect(place.getByTestId("you-are-here-panel")).toBeVisible();
  }
  await place.getByTestId("talk-button").first().click();
  const panel = page.getByTestId("talk-panel");
  await expect(panel).toBeVisible();
  await expect(panel.getByTestId("talk-away")).toHaveCount(0);
  const x = await ask(page, "topic.worries");
  await expect(x.getByTestId("talk-learned").locator("li").first()).toBeVisible();
});

test("walk up to someone in the street and talk (a tap on them; Enter beside them on desktop)", async ({ page, isMobile }) => {
  await start(page, "7311");
  await streetReady(page);
  await toStreets(page);
  await page.getByTestId("place-panel").getByRole("button", { name: "Close" }).click();
  await expect(page.getByTestId("place-panel")).toHaveCount(0);
  // The engine's passers-by on these streets stand about the player.
  await expect.poll(() => page.evaluate(() => (window as unknown as Probes).__townStreet?.view.scene.standing.length ?? 0), { timeout: 10_000 }).toBeGreaterThan(0);
  await expect(page.locator(".hub-person-tag").first()).toBeVisible();
  if (isMobile) {
    // A tap on their name tag walks up to them and opens the talk panel.
    await page.locator(".hub-person-tag").first().click();
  } else {
    // Stand beside them, focus the street, and press Enter (the door/Enter pattern).
    await page.evaluate(() => {
      const v = (window as unknown as Probes).__townStreet?.view;
      const s = v?.scene.standing[0];
      if (v && s) v.scene.goToPerson(s.key);
    });
    await expect.poll(() => page.evaluate(() => (window as unknown as Probes).__townStreet?.view.scene.personNear() ?? null), { timeout: 10_000 }).not.toBeNull();
    await page.locator(".town-street-stage canvas").focus();
    await expect(page.locator(".hub-door-hint")).toHaveText(/^Enter: talk to /);
    await expect(page.getByTestId("person-announce")).toHaveText(/is here\. Press Enter to talk\./);
    await page.keyboard.press("Enter");
  }
  const panel = page.getByTestId("talk-panel");
  await expect(panel).toBeVisible();
  // A stranger until the talk starts (they are met as it does).
  await expect(panel.getByRole("heading", { name: "Talking with someone you haven't met" })).toBeVisible();
  const x = await ask(page, "topic.who_runs");
  // Met now: the heading names them; what was learned (meeting them) and where to look next (a lead).
  await expect(panel.getByRole("heading", { name: /^Talking with (?!someone)/ })).toBeVisible();
  await expect(x.getByTestId("talk-learned").locator("li").first()).toBeVisible();
  await expect(x.getByTestId("talk-leads").locator("li").first()).not.toBeEmpty();
  await expect(x).not.toContainText(/\b(lead|talk|notebook|ui)-[a-z]/);
  await shot(page, "talk-exchange", isMobile);
});

test("canvass with a chosen worry: the worries the player knows, no particular issue first", async ({ page, isMobile }) => {
  await start(page, "7311");
  const panel = await toStreets(page);
  const form = panel.getByTestId("form-canvass");
  await expect(form).toBeVisible();
  const select = form.getByTestId("canvass-worry");
  await expect(select).toHaveValue("");
  await expect(select.locator("option").first()).toHaveText("No particular issue");
  await expect(form.getByTestId("canvass-fit")).toHaveText("A general canvass, on no worry in particular.");
  const n = await select.locator("option").count();
  expect(n).toBeGreaterThan(1);
  const value = await select.locator("option").nth(1).getAttribute("value");
  await select.selectOption(value ?? "");
  await expect(form.getByTestId("canvass-fit")).not.toHaveText("A general canvass, on no worry in particular.");
  await shot(page, "canvass-worry", isMobile);
  await form.getByRole("button", { name: /^(Do it|Go)$/ }).click();
  await expect(form.getByTestId("canvass-landed")).toHaveText(/^You raised .+: it (struck a chord|went down fairly well|fell flat here)\.$/);
});
