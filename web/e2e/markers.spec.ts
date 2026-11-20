import { mkdirSync } from "node:fs";
import { expect, test, type Page, type TestInfo } from "@playwright/test";
import { officeFigures, openPeople, showPlaces } from "./pocket";

// Who can be talked to, at a glance (DEC-093): every person's name tag in the
// street and in rooms carries a pixel marker, a bubble with dots ("can talk")
// or with "!" (the engine's `has_news`: something new for the player), said
// in the tag's accessible name, never by colour alone (DEC-051). And the
// Notebook's People section is the single place for people: the key figures
// pinned, each with Talk (the dialogue) and Go to them.
// MARKERS_SCREENS=<dir> also saves screenshots there (light and dark).
const SCREENS = process.env.MARKERS_SCREENS ?? "";

interface Probes {
  __townStreet?: {
    view: {
      map: { buildings: { kind: string; place: number | null; front: number }[] };
      scene: { standing: { key: string }[]; placePlayer(cell: number): void };
      wake(): void;
    };
  } | null;
  __interior?: { people: { key: string; extra: boolean; label: string | null }[] } | null;
}

const MARKED = /, (can talk|has something new)$/;

const start = async (page: Page, seed: string) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill(seed);
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
  await expect(page.locator(".town-street-stage[data-ready='1']")).toBeVisible({ timeout: 30_000 });
};

const shot = async (page: Page, info: TestInfo, name: string) => {
  if (!SCREENS) return;
  mkdirSync(SCREENS, { recursive: true });
  for (const scheme of ["light", "dark"] as const) {
    await page.emulateMedia({ colorScheme: scheme, reducedMotion: "reduce" });
    await page.waitForTimeout(250);
    await page.screenshot({ path: `${SCREENS}/${name}-${info.project.name}-${scheme}.png` });
  }
  await page.emulateMedia({ colorScheme: null, reducedMotion: null });
};

/** Every person's tag shown has a marker and says what it means. */
const expectMarked = async (page: Page, scope: string) => {
  const tags = page.locator(`${scope} .hub-person-tag:visible`);
  await expect(tags.first()).toBeVisible({ timeout: 15_000 });
  const n = await tags.count();
  for (let i = 0; i < n; i++) {
    const tag = tags.nth(i);
    await expect(tag).toHaveAccessibleName(MARKED);
    await expect(tag.locator(".hub-tag-mark svg")).toBeVisible();
    const marker = await tag.getAttribute("data-marker");
    expect(["talk", "news"]).toContain(marker);
    // The words match the marker (the shape, not only the colour, differs too).
    await expect(tag).toHaveAccessibleName(marker === "news" ? /, has something new$/ : /, can talk$/);
  }
  return n;
};

test("talk markers: everyone to talk to in the street and in a room has a marker, with an accessible name", async ({ page }, info) => {
  test.setTimeout(120_000);
  await start(page, "7311");
  // The street: the engine's people on these streets stand about the player, each tag marked.
  const pocket = await showPlaces(page);
  await pocket.getByTestId("spot-streets").first().click();
  const panel = page.getByTestId("place-panel");
  await expect(panel.getByTestId("you-are-here-panel")).toBeVisible();
  await panel.getByRole("button", { name: "Close" }).click();
  await expect(panel).toHaveCount(0);
  expect(await expectMarked(page, ".town-street-stage")).toBeGreaterThan(0);
  // Named for screen readers: a tag is a button with its marker in words.
  await expect(page.getByRole("button", { name: MARKED }).first()).toBeVisible();
  await shot(page, info, "street-markers");

  // A room: inside the town hall, the people present have the same tags.
  await start(page, "31337");
  await page.evaluate(() => {
    const v = (window as unknown as Probes).__townStreet?.view;
    const b = v?.map.buildings.find((x) => x.kind === "town_hall" && x.place !== null);
    if (v && b) {
      v.scene.placePlayer(b.front);
      v.wake();
    }
  });
  await page.locator(".town-street-stage canvas").focus();
  await expect(page.locator(".hub-door-hint")).toBeVisible();
  await page.keyboard.press("Enter");
  await expect(page.locator("[data-testid='interior'][data-ready='1']")).toBeVisible({ timeout: 30_000 });
  const present = await page.evaluate(() => (window as unknown as Probes).__interior?.people.filter((p) => !p.extra && p.label).length ?? 0);
  expect(present, "people the engine says are here").toBeGreaterThan(0);
  expect(await expectMarked(page, ".interior-stage")).toBeGreaterThan(0);
  // Someone stored and not asked anything this week has something new.
  await expect(page.locator(".interior-stage .hub-person-tag[data-marker='news']").first()).toBeAttached();
  await shot(page, info, "room-markers");
});

test("the Notebook's People: key figures pinned, each with Talk (the dialogue opens) and Go to them; the pocket has four tabs", async ({ page }, info) => {
  await start(page, "7311");
  const pocket = await openPeople(page);
  await expect(pocket.getByRole("tablist").first().getByRole("tab")).toHaveCount(4);
  const key = pocket.getByTestId("notebook-key-figures");
  await expect(key.getByRole("heading", { name: "Key figures" })).toBeVisible();
  // The rival is always known by name (DEC-091); the members of the body are pinned too.
  await expect(key.getByText("Your rival").first()).toBeVisible();
  const office = officeFigures(pocket);
  expect(await office.count()).toBeGreaterThan(1);
  const first = office.first();
  await expect(first.getByTestId("notebook-standing")).not.toBeEmpty();
  await expect(first.getByRole("button", { name: /^Go to / })).toBeVisible();
  await shot(page, info, "notebook-people");
  await first.getByTestId("talk-button").click();
  const talk = page.getByTestId("talk-panel");
  await expect(talk).toBeVisible();
  await expect(page.getByRole("dialog", { name: /^Talking with / })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(talk).toHaveCount(0);

  // Go to them: the engine says where they are; the player goes there and its panel opens.
  const again = await openPeople(page);
  await officeFigures(again).first().getByTestId("notebook-go-to").click();
  await expect(page.getByTestId("place-panel").or(page.getByTestId("talk-panel")).first()).toBeVisible({ timeout: 15_000 });
});
