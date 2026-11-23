import { mkdirSync } from "node:fs";
import { expect, test, type Locator, type Page } from "@playwright/test";
import { officeFigures, openPeople, showPlaces } from "./pocket";

// Talking to people (DEC-093) in dialogue mode, and canvassing on a worry
// (DEC-091). A conversation is a dialogue box at the bottom of the screen:
// from the pocket's People tab and a place's people it stands over a pixel
// portrait of the person; in the walkable street (walking up to someone: a
// tap, or Enter beside them) the camera frames the two speakers, the street
// dims and pauses, and the reply is a speech bubble over the person. Topics
// are one-tap choices with the engine's odds and the cost; a topic about
// something opens a subject step in the same box; the reply comes page by
// page; what was learned is a notebook toast and a lead a choice. The
// keyboard: arrow keys move, Enter chooses, Escape steps back or leaves.
// TALK_SCREENS=<dir> also saves screenshots there (light and dark).
const SCREENS = process.env.TALK_SCREENS ?? "";

interface Standing {
  key: string;
  w: { x: number; y: number };
}
interface Probes {
  __townStreet?: {
    view: {
      stats: { frames: number; ticks: number };
      paused: boolean;
      talking: string | null;
      scene: {
        standing: Standing[];
        goToPerson(key: string): boolean;
        personNear(): string | null;
      };
    };
  } | null;
}
/** What the street shows now (tests read the view through `window.__townStreet`). */
const street = (page: Page) =>
  page.evaluate(() => {
    const v = (window as unknown as Probes).__townStreet?.view;
    return v ? { standing: v.scene.standing.length, near: v.scene.personNear(), talking: v.talking, paused: v.paused, frames: v.stats.frames } : null;
  });

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
    await page.waitForTimeout(200);
    await page.screenshot({ path: `${SCREENS}/${name}-${isMobile ? "phone" : "desktop"}-${scheme}.png` });
  }
  await page.emulateMedia({ colorScheme: null, reducedMotion: null });
};

/** The dialogue box. */
const box = (page: Page) => page.getByTestId("talk-panel");
/** A choice button in the box by its test id (the row's). */
const choiceIn = (row: Locator) => row.locator("button.dlg-choice");

/** The reply to the last question: waits for it, reads every page (Continue), and returns what was said, page by page. */
const readReply = async (page: Page): Promise<string[]> => {
  const bubble = page.getByTestId("talk-bubble");
  const live = page.getByTestId("talk-reply-live");
  await expect(bubble.locator("p").first()).not.toBeEmpty({ timeout: 15_000 });
  await expect(live).not.toBeEmpty();
  const said: string[] = [];
  for (let i = 0; i < 12; i++) {
    said.push((await live.textContent()) ?? "");
    const more = box(page).getByTestId("talk-continue");
    if ((await more.count()) === 0) break;
    // The first press shows the whole page (the typewriter), the next turns it.
    await choiceIn(more).click();
    if ((await more.count()) > 0) await choiceIn(more).click();
  }
  await expect(box(page).getByTestId("talk-continue")).toHaveCount(0);
  // A tap on the last page shows all of it at once (the typewriter).
  await bubble.click();
  await expect(bubble.locator(".dlg-untyped")).toHaveCount(0);
  return said;
};

/** Raise a topic: a plain topic at once; a topic about something on its first subject that can be raised. */
const raise = async (page: Page, topic: string) => {
  const row = box(page).getByTestId(`talk-topic-${topic}`);
  await expect(row).toBeVisible();
  await choiceIn(row).click();
  const subjects = box(page).getByTestId("talk-subjects");
  if (await subjects.isVisible()) {
    await choiceIn(subjects.getByTestId("talk-subject").filter({ has: page.locator("button:not([aria-disabled])") }).first()).click();
  }
  return readReply(page);
};

/** Go to the streets of the first neighbourhood (the Places tab), where passers-by stand. */
const toStreets = async (page: Page) => {
  const pocket = await showPlaces(page);
  await pocket.getByTestId("spot-streets").first().click();
  const panel = page.getByTestId("place-panel");
  await expect(panel.getByTestId("you-are-here-panel")).toBeVisible();
  return panel;
};

test("talk from the Notebook's People: over a portrait; someone elsewhere is gone to; topics with odds and the cost; a subject step; a reply in their voice", async ({ page, isMobile }) => {
  await start(page, "7311");
  const pocket = await openPeople(page);
  await officeFigures(pocket).getByTestId("talk-button").first().click();
  const dlg = box(page);
  await expect(dlg).toBeVisible();
  await expect(page.getByRole("dialog", { name: /^Talking with / })).toBeVisible();
  // Away from the street: the same box over a portrait.
  await expect(page.getByTestId("talk-layer")).toHaveAttribute("data-mode", "portrait");
  await expect(page.getByTestId("talk-portrait")).toBeVisible();
  // Not here: where they are, and a way there first (free); the topics wait, without odds.
  await expect(dlg.getByTestId("talk-away")).toHaveText(/^They're at /);
  await expect(choiceIn(dlg.getByTestId("talk-go"))).toBeFocused();
  await expect(choiceIn(dlg.getByTestId("talk-topic-topic.who_runs"))).toHaveAttribute("aria-disabled", "true");
  await expect(dlg.getByTestId("talk-odds")).toHaveCount(0);
  await choiceIn(dlg.getByTestId("talk-go")).click();
  await expect(dlg.getByTestId("talk-away")).toHaveCount(0);
  await expect(dlg.getByTestId("talk-cost")).toHaveText(/^½ slot · \d+ left/);
  await shot(page, "portrait-topics", isMobile);
  // A topic about something opens its subjects in the same box, each with the engine's odds where it rolls.
  const support = dlg.getByTestId("talk-topic-topic.support").or(dlg.getByTestId("talk-topic-topic.pitch")).first();
  await expect(support.locator(".dlg-opens")).toHaveText("›");
  await choiceIn(support).click();
  const subjects = dlg.getByTestId("talk-subjects");
  await expect(subjects).toBeVisible();
  await expect(dlg.getByTestId("talk-back")).toBeVisible();
  await expect(subjects.getByTestId("talk-odds").first().locator("span").first()).toHaveText(/^\d+%$/);
  await shot(page, "portrait-subjects", isMobile);
  // Back to the topics.
  await dlg.getByTestId("talk-back").click();
  await expect(dlg.getByTestId("talk-topics")).toBeVisible();
  const said = await raise(page, "topic.who_runs");
  // In their voice (the engine's lines), never a raw key.
  expect(said.join(" ")).not.toMatch(/\b(lead|talk|notebook|ui)-[a-z]/);
  // The bubble's text is in the DOM too, the same as the live region reads.
  const live = (await page.getByTestId("talk-reply-live").textContent()) ?? "";
  const bubble = (await page.getByTestId("talk-bubble").textContent()) ?? "";
  expect(live.endsWith(bubble.replace(/▼$/, "").replace(/\s+/g, " ").trim().slice(-20))).toBe(true);
  await expect(dlg.getByTestId("talk-outcome")).not.toBeEmpty();
  await shot(page, "portrait-reply", isMobile);
  if (SCREENS && isMobile) {
    // The narrowest phone the game is laid out for.
    const vp = page.viewportSize();
    await page.setViewportSize({ width: 360, height: 740 });
    await shot(page, "portrait-reply-360", isMobile);
    if (vp) await page.setViewportSize(vp);
  }
  // The half slot left: the next conversation uses it.
  await expect(dlg.getByTestId("talk-cost")).toHaveText(/^½ slot · \d*½ left/);
  // Goodbye closes it.
  await choiceIn(dlg.getByTestId("talk-goodbye")).click();
  await expect(dlg).toHaveCount(0);
});

test("talk to someone at a place, from the place panel's people: what was learned is a notebook toast", async ({ page, isMobile }) => {
  await start(page, "7311");
  const pocket = await showPlaces(page);
  await pocket.getByTestId("spot-council_chamber").first().click();
  const place = page.getByTestId("place-panel");
  await expect(place.getByTestId("you-are-here-panel")).toBeVisible();
  // The chamber's members are in the whip list; try the diner's people instead if none.
  const talk = place.getByTestId("talk-button");
  if ((await talk.count()) === 0) {
    const p2 = await showPlaces(page);
    await p2.getByTestId("spot-diner").first().click();
    await expect(place.getByTestId("you-are-here-panel")).toBeVisible();
  }
  await place.getByTestId("talk-button").first().click();
  const dlg = box(page);
  await expect(dlg).toBeVisible();
  await expect(page.getByTestId("talk-layer")).toHaveAttribute("data-mode", "portrait");
  await expect(dlg.getByTestId("talk-away")).toHaveCount(0);
  await raise(page, "topic.worries");
  const noted = page.getByTestId("talk-noted");
  await expect(noted).toHaveText(/Noted: /);
  await shot(page, "portrait-noted", isMobile);
  // Tapping it leaves the conversation for the notebook.
  await noted.getByTestId("talk-noted-open").click();
  await expect(dlg).toHaveCount(0);
  await expect(page.getByTestId("pocket").getByTestId("pocket-tab-notebook")).toHaveAttribute("aria-selected", "true");
});

test("walk up to someone in the street and talk (a tap on them; Enter beside them on desktop): the street frames the two, a bubble over the person", async ({ page, isMobile }) => {
  await start(page, "7311");
  await streetReady(page);
  await toStreets(page);
  await page.getByTestId("place-panel").getByRole("button", { name: "Close" }).click();
  await expect(page.getByTestId("place-panel")).toHaveCount(0);
  // The engine's passers-by on these streets stand about the player.
  await expect.poll(() => street(page).then((v) => v?.standing ?? 0), { timeout: 10_000 }).toBeGreaterThan(0);
  await expect(page.locator(".hub-person-tag").first()).toBeVisible();
  // A passer-by (`a<slot>`): the townsfolk whose routines have them on these
  // streets now stand about too, and are people the player may know.
  await expect(page.locator(".hub-person-tag[data-person^='a']").first()).toBeVisible();
  if (isMobile) {
    // A tap on their name tag walks up to them and opens the conversation.
    await page.locator(".hub-person-tag[data-person^='a']").first().click();
  } else {
    // Stand beside them, focus the street, and press Enter (the door/Enter pattern).
    await page.evaluate(() => {
      const v = (window as unknown as Probes).__townStreet?.view;
      const s = v?.scene.standing.find((x) => x.key.startsWith("a"));
      if (v && s) v.scene.goToPerson(s.key);
    });
    await expect.poll(() => street(page).then((v) => v?.near ?? null), { timeout: 10_000 }).not.toBeNull();
    await page.locator(".town-street-stage canvas").focus();
    await expect(page.locator(".hub-door-hint")).toHaveText(/^Enter: talk to /);
    await expect(page.getByTestId("person-announce")).toHaveText(/is here\. Press Enter to talk\./);
    await page.keyboard.press("Enter");
  }
  const dlg = box(page);
  await expect(dlg).toBeVisible();
  // In the street: framed there, the rest dimmed, walking paused (the view is live, not paused behind a sheet).
  await expect(page.getByTestId("talk-layer")).toHaveAttribute("data-mode", "street");
  await expect(page.getByTestId("talk-veil")).toBeVisible();
  await expect.poll(() => street(page).then((v) => v?.talking ?? null)).not.toBeNull();
  expect((await street(page))?.paused).toBe(false);
  // A stranger until the talk starts (they are met as it does).
  await expect(page.getByRole("dialog", { name: "Talking with someone you haven't met" })).toBeVisible();
  await expect(dlg.locator(".dlg-sub")).toContainText("Someone you haven't met");
  await shot(page, "street-topics", isMobile);
  // The person's opener comes first (note 2), then the question goes up over the player, then the reply over the person.
  await choiceIn(dlg.getByTestId("talk-topic-topic.who_runs")).click();
  await expect(page.getByTestId("talk-bubble").locator("p").first()).not.toBeEmpty({ timeout: 15_000 });
  await expect(page.getByTestId("talk-you")).toHaveCount(0);
  // Tap through the opener (the first tap shows it all, the next goes on) until the question goes up.
  await expect(async () => {
    if (!(await page.getByTestId("talk-you").isVisible())) await page.getByTestId("talk-bubble").click({ timeout: 1000 });
    await expect(page.getByTestId("talk-you")).toBeVisible({ timeout: 800 });
  }).toPass({ timeout: 15_000 });
  await expect(page.getByTestId("talk-you")).toHaveText("Who really runs things here?");
  await expect(page.getByTestId("talk-bubble").locator("p").first()).not.toBeEmpty({ timeout: 15_000 });
  // The bubble stays on screen and above the box.
  const bb = await page.getByTestId("talk-bubble").boundingBox();
  const db = await page.locator(".dlg-box-wrap").boundingBox();
  const vp = page.viewportSize();
  expect(bb && db && vp).toBeTruthy();
  if (bb && db && vp) {
    expect(bb.x).toBeGreaterThanOrEqual(0);
    expect(bb.x + bb.width).toBeLessThanOrEqual(vp.width);
    expect(bb.y).toBeGreaterThanOrEqual(0);
    expect(bb.y + bb.height).toBeLessThanOrEqual(db.y);
  }
  // A tap on the bubble shows the whole page (the typewriter).
  await page.getByTestId("talk-bubble").click();
  await expect(page.getByTestId("talk-bubble").locator(".dlg-untyped")).toHaveCount(0);
  await shot(page, "street-reply", isMobile);
  await readReply(page);
  // Met now: the box names them; what was learned (meeting them) is noted; where to look next is a lead.
  await expect(page.getByRole("dialog", { name: /^Talking with (?!someone)/ })).toBeVisible();
  await expect(page.getByTestId("talk-noted")).toHaveText(/Noted: /);
  expect(Number(await dlg.getAttribute("data-leads"))).toBeGreaterThan(0);
  await shot(page, "street-noted", isMobile);
  // Cheap while it lasts: the townsfolk stand still, so once the light has eased nothing is drawn.
  await page.waitForTimeout(2500);
  const f0 = (await street(page))?.frames ?? 0;
  await page.waitForTimeout(1000);
  expect(((await street(page))?.frames ?? 0) - f0).toBeLessThanOrEqual(2);
  // Escape leaves: the street is the player's again, and the keyboard goes back to it.
  await page.keyboard.press("Escape");
  await expect(dlg).toHaveCount(0);
  await expect(page.getByTestId("talk-veil")).toHaveCount(0);
  await expect.poll(() => street(page).then((v) => v?.talking ?? null)).toBeNull();
  if (!isMobile) await expect(page.locator(".town-street-stage canvas")).toBeFocused();
});

test("the keyboard in a conversation: arrows move between choices, Enter chooses, Escape steps back from a subject, then leaves", async ({ page, isMobile }) => {
  test.skip(isMobile, "keyboard on desktop");
  await start(page, "7311");
  const pocket = await openPeople(page);
  const opener = officeFigures(pocket).getByTestId("talk-button").first();
  await opener.click();
  const dlg = box(page);
  await choiceIn(dlg.getByTestId("talk-go")).click();
  await expect(dlg.getByTestId("talk-away")).toHaveCount(0);
  const choices = dlg.getByTestId("talk-topics").locator("button.dlg-choice");
  await choices.first().focus();
  await expect(choices.first()).toBeFocused();
  await page.keyboard.press("ArrowDown");
  await expect(choices.nth(1)).toBeFocused();
  await page.keyboard.press("ArrowUp");
  await expect(choices.first()).toBeFocused();
  // Up from the first wraps to the last (Goodbye); Home goes back to the first.
  await page.keyboard.press("ArrowUp");
  await expect(choiceIn(dlg.getByTestId("talk-goodbye"))).toBeFocused();
  await page.keyboard.press("Home");
  await expect(choices.first()).toBeFocused();
  // Only the focused choice is in the tab order (one Tab stop for the list).
  await expect(dlg.locator("button.dlg-choice[tabindex='0']")).toHaveCount(1);
  // To a topic about something, and Enter: its subjects, focused.
  const support = dlg.getByTestId("talk-topic-topic.support").or(dlg.getByTestId("talk-topic-topic.pitch")).first();
  const id = (await support.getAttribute("data-testid")) ?? "";
  for (let i = 0; i < 12 && !(await choiceIn(support).evaluate((el) => el === document.activeElement)); i++) await page.keyboard.press("ArrowDown");
  await expect(choiceIn(support)).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(dlg.getByTestId("talk-subjects")).toBeVisible();
  // The first subject has the keyboard (or the search box, on a long list: ArrowDown goes on to the subjects).
  if ((await dlg.getByTestId("talk-subject-search").count()) > 0) {
    await expect(dlg.getByTestId("talk-subject-search")).toBeFocused();
    await page.keyboard.press("ArrowDown");
  }
  await expect(dlg.getByTestId("talk-subjects").locator("button.dlg-choice:focus")).toHaveCount(1);
  // Escape steps back, to the topic it came from.
  await page.keyboard.press("Escape");
  await expect(dlg.getByTestId("talk-subjects")).toHaveCount(0);
  await expect(choiceIn(dlg.getByTestId(id))).toBeFocused();
  // Enter on a plain topic asks it; the reply comes; Enter turns its pages.
  await choiceIn(dlg.getByTestId("talk-topic-topic.who_runs")).focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("talk-bubble").locator("p").first()).not.toBeEmpty({ timeout: 15_000 });
  for (let i = 0; i < 12 && (await dlg.getByTestId("talk-continue").count()) > 0; i++) {
    await expect(choiceIn(dlg.getByTestId("talk-continue"))).toBeFocused();
    await page.keyboard.press("Enter");
  }
  await expect(dlg.getByTestId("talk-continue")).toHaveCount(0);
  // Escape leaves; the keyboard goes back to what opened it (or the street, where that has gone).
  await page.keyboard.press("Escape");
  await expect(dlg).toHaveCount(0);
  const back = await page.evaluate(() => {
    const a = document.activeElement;
    return a?.matches("[data-testid='talk-button']") ? "opener" : a?.tagName === "CANVAS" ? "street" : (a?.tagName ?? "none");
  });
  expect(["opener", "street"]).toContain(back);
});

test("out of time: the box says so and every topic waits", async ({ page }) => {
  test.setTimeout(120_000);
  await start(page, "7311");
  const pocket = await openPeople(page);
  await officeFigures(pocket).getByTestId("talk-button").first().click();
  const dlg = box(page);
  await choiceIn(dlg.getByTestId("talk-go")).click();
  await expect(dlg.getByTestId("talk-away")).toHaveCount(0);
  // Talk until the turn's time is spent. As the day goes on they may move
  // on (their daily routine): go where the engine says they are, and go on.
  for (let i = 0; i < 16; i++) {
    if ((await dlg.getByTestId("talk-go").count()) > 0) {
      await choiceIn(dlg.getByTestId("talk-go")).click();
      await expect(dlg.getByTestId("talk-away")).toHaveCount(0);
    }
    if ((await dlg.getByTestId("talk-out-of-time").count()) > 0) break;
    await raise(page, "topic.small_talk");
  }
  await expect(dlg.getByTestId("talk-out-of-time")).toHaveText(/out of time this turn/);
  await expect(dlg.locator(".dlg-cost-out")).toHaveText("Out of time");
  await expect(dlg.getByTestId("talk-cost")).toHaveCount(0);
  const topics = dlg.locator("[data-testid^='talk-topic-'] button.dlg-choice");
  const n = await topics.count();
  expect(n).toBeGreaterThan(0);
  for (let i = 0; i < n; i++) await expect(topics.nth(i)).toHaveAttribute("aria-disabled", "true");
  // Goodbye still works.
  await expect(choiceIn(dlg.getByTestId("talk-goodbye"))).not.toHaveAttribute("aria-disabled", "true");
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
