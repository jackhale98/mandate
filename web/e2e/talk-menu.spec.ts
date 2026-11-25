import { expect, test, type Locator, type Page } from "@playwright/test";
import { officeFigures, openPeople } from "./pocket";
import { seedFor } from "./seeds";
import { useWideFont } from "./wide-font";

// Fewer options, better answers (M6 note 26): a topic's subjects come in the
// engine's order, the ones the person could be expected to know first and
// the rest behind "Something else…" with search; "What's on your mind?" asks
// the person to say what they most want to. With a wide font, at 320, 360
// and 390 px on the phone; the keyboard on desktop.

const box = (page: Page) => page.getByTestId("talk-panel");
const choiceIn = (row: Locator) => row.locator("button.dlg-choice");
const RAW = /\b(lead|talk|notebook|ui|outcome|topic|line|occupation)[-.][a-z]/;

/** Nothing in the box is wider than the screen, and the page does not scroll sideways. */
const fits = async (page: Page) => {
  const r = await page.evaluate(() => {
    const b = document.querySelector("[data-testid='talk-panel']")?.getBoundingClientRect();
    return { scroll: document.documentElement.scrollWidth, width: window.innerWidth, left: b?.left ?? 0, right: b?.right ?? 0 };
  });
  expect(r.scroll).toBeLessThanOrEqual(r.width);
  expect(r.left).toBeGreaterThanOrEqual(0);
  expect(r.right).toBeLessThanOrEqual(r.width + 0.5);
};

test("a conversation lists the likely subjects first, the rest behind Something else…, and asks what's on their mind", async ({ page, isMobile }) => {
  test.setTimeout(120_000);
  await useWideFont(page);
  if (isMobile) await page.setViewportSize({ width: 320, height: 720 });
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill(seedFor("notebook_talk_reply_has_an_outcome"));
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
  const pocket = await openPeople(page);
  await officeFigures(pocket).getByTestId("talk-button").first().click();
  const dlg = box(page);
  await expect(dlg).toBeVisible();
  const go = dlg.getByTestId("talk-go");
  if ((await go.count()) > 0) await choiceIn(go).click();
  await expect(dlg.getByTestId("talk-away")).toHaveCount(0);

  // The open question is a topic of its own, first.
  const topics = dlg.getByTestId("talk-topics");
  await expect(topics.locator("li").first()).toHaveAttribute("data-testid", "talk-topic-topic.open");
  await expect(choiceIn(dlg.getByTestId("talk-topic-topic.open"))).toHaveText(/What's on your mind\?/);

  // Asking about something: the likely subjects, then one entry for the rest.
  await choiceIn(dlg.getByTestId("talk-topic-topic.about")).click();
  const subjects = dlg.getByTestId("talk-subjects");
  await expect(subjects).toBeVisible();
  const likely = await subjects.getByTestId("talk-subject").count();
  expect(likely).toBeGreaterThan(0);
  const more = dlg.getByTestId("talk-something-else");
  await expect(more).toHaveCount(1);
  await expect(subjects.locator("li").last()).toHaveAttribute("data-testid", "talk-something-else");
  await fits(page);
  if (isMobile) {
    for (const width of [360, 390]) {
      await page.setViewportSize({ width, height: 760 });
      await fits(page);
    }
    await page.setViewportSize({ width: 320, height: 720 });
  }
  if (isMobile) await choiceIn(more).click();
  else {
    // The keyboard: End reaches the entry, Enter opens it, the search has the keyboard.
    await choiceIn(subjects.getByTestId("talk-subject").first()).focus();
    await page.keyboard.press("End");
    await expect(choiceIn(more)).toBeFocused();
    await page.keyboard.press("Enter");
  }
  const search = dlg.getByTestId("talk-subject-search");
  await expect(search).toBeVisible();
  await expect(dlg.getByTestId("talk-something-else")).toHaveCount(0);
  const rest = subjects.getByTestId("talk-subject");
  const all = await rest.count();
  expect(all).toBeGreaterThan(0);
  await fits(page);
  // Search narrows the rest by name.
  const first = ((await choiceIn(rest.first()).locator(".dlg-choice-label").textContent()) ?? "").trim();
  await search.fill(first);
  await expect(rest.first()).toContainText(first);
  expect(await rest.count()).toBeLessThanOrEqual(all);
  await search.fill("");
  // Back to the likely subjects, then to the topics.
  await dlg.getByTestId("talk-back").click();
  await expect(dlg.getByTestId("talk-something-else")).toHaveCount(1);
  await dlg.getByTestId("talk-back").click();
  await expect(topics).toBeVisible();

  // What's on your mind? One tap; they say something in their own voice.
  const open = dlg.getByTestId("talk-topic-topic.open");
  if (isMobile) await choiceIn(open).click();
  else {
    await choiceIn(open).focus();
    await page.keyboard.press("Enter");
  }
  await expect(page.getByTestId("talk-you")).toHaveText("What's on your mind?");
  const bubble = page.getByTestId("talk-bubble");
  await expect(bubble.locator("p").first()).not.toBeEmpty({ timeout: 15_000 });
  for (let i = 0; i < 8; i++) {
    const cont = dlg.getByTestId("talk-continue");
    if ((await cont.count()) === 0) break;
    await choiceIn(cont).click();
    if ((await cont.count()) > 0) await choiceIn(cont).click();
  }
  await expect(dlg.getByTestId("talk-outcome")).not.toBeEmpty();
  await fits(page);
  // No raw key or id anywhere in the box or the bubble.
  for (const text of [await dlg.innerText(), await page.getByTestId("talk-reply-live").innerText(), await bubble.innerText()]) {
    expect(text).not.toMatch(RAW);
    expect(text).not.toMatch(/\{\s*\$|\$[a-z_]+/);
  }
});
