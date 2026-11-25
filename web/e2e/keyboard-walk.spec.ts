import { expect, test, type Page } from "@playwright/test";
import { openPocket } from "./pocket";

// Keyboard walking on the street (desktop; owner, 2026-11-19): the street
// takes focus in the Tab order (after the skip link, before the bar); with
// it, arrows and WASD walk the player over walkable ground, Shift hurries, the
// camera follows and the "where you are" label follows the ground underfoot.
// At a place's door a hint says "Enter: <place>", and Enter goes in: the same
// command as a tap (the place panel opens). Keys do nothing while the focus is
// in the pocket. DEC-051: the canvas's name points keyboard users to the
// arrows, Enter and the Places tab.

test.skip(({ isMobile }) => isMobile, "keyboard walking is for desktop");

interface Building {
  index: number;
  front: number;
  place: number | null;
  kind: string;
  home: boolean;
}
interface Probes {
  __townStreet?: {
    view: {
      map: { width: number; buildings: Building[]; district: ArrayLike<number>; districts: { id: number }[] };
      scene: {
        player: { x: number; y: number };
        playerCell: number;
        doorHere: number | null;
        grid: { cost: ArrayLike<number> };
        placePlayer(cell: number): void;
        setStanding(people: readonly { key: string; variant: number }[]): void;
      };
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
  await expect(page.locator(".town-street-stage[data-ready='1']")).toBeVisible({ timeout: 30_000 });
};

const street = (page: Page) => page.locator(".town-street-stage canvas");
const player = (page: Page) => page.evaluate(() => {
  const p = (window as unknown as Probes).__townStreet?.view.scene.player;
  return p ? { x: p.x, y: p.y } : { x: NaN, y: NaN };
});

/** Focus the street the way a keyboard user does: the skip link, then the street. */
const tabToStreet = async (page: Page) => {
  await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
  await page.keyboard.press("Tab");
  await expect(page.getByTestId("skip-to-places")).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(street(page)).toBeFocused();
};

/** Stand the player at a cell (the probe), as if they had walked there. */
const standAt = (page: Page, cell: number) =>
  page.evaluate((c) => {
    const v = (window as unknown as Probes).__townStreet?.view;
    v?.scene.placePlayer(c);
    v?.wake();
  }, cell);

test("the street is in the Tab order, named for keyboard users, with a focus ring; Tab still reaches the bar", async ({ page }) => {
  await start(page, "4242");
  const canvas = street(page);
  await expect(canvas).toHaveAttribute("tabindex", "0");
  await expect(canvas).toHaveAttribute("aria-label", /arrow keys.*Enter.*Places tab/);
  await tabToStreet(page);
  await expect(canvas).toHaveCSS("outline-style", "solid");
  // On to the bar, in its order, and back.
  await page.keyboard.press("Tab");
  await expect(page.getByTestId("town-map-button")).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByTestId("pocket-button")).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await page.keyboard.press("Shift+Tab");
  await expect(canvas).toBeFocused();
});

test("held arrows walk the player; the camera follows and the zone label changes across a border", async ({ page }) => {
  await start(page, "4242");
  await tabToStreet(page);
  // From the start (at home), at least one way is open.
  let moved = false;
  for (const key of ["ArrowDown", "ArrowRight", "ArrowLeft", "ArrowUp"]) {
    const a = await player(page);
    await page.keyboard.down(key);
    await page.waitForTimeout(400);
    await page.keyboard.up(key);
    const b = await player(page);
    if (Math.hypot(b.x - a.x, b.y - a.y) > 0.5) {
      moved = true;
      break;
    }
  }
  expect(moved).toBe(true);
  // The camera follows: the player stays well inside the street.
  const box = await page.locator(".town-street-stage").boundingBox();
  const onScreen = await page.evaluate(() => {
    const v = (window as unknown as { __townStreet: { view: { scene: { player: { x: number; y: number } }; toScreen(x: number, y: number): { x: number; y: number } } } }).__townStreet.view;
    return v.toScreen(v.scene.player.x, v.scene.player.y);
  });
  expect(onScreen.x).toBeGreaterThan(40);
  expect(onScreen.x).toBeLessThan((box?.width ?? 0) - 40);
  expect(onScreen.y).toBeGreaterThan(40);
  expect(onScreen.y).toBeLessThan((box?.height ?? 0) - 40);

  // A border between two neighbourhoods on an open row: stand on one side, walk right across it.
  const border = await page.evaluate(() => {
    const v = (window as unknown as Probes).__townStreet?.view;
    if (!v) return null;
    const { width: W, district, districts } = v.map;
    const cost = v.scene.grid.cost;
    const hood = (i: number) => districts[district[i] ?? -1]?.id ?? null;
    const open = (i: number) => (cost[i] ?? 0) > 0;
    for (let i = 0; i < cost.length - 4; i++) {
      if (i % W > W - 5 || [0, 1, 2, 3].some((k) => !open(i + k))) continue;
      const a = hood(i);
      const b = hood(i + 2);
      if (a !== null && b !== null && a !== b && hood(i + 1) === a && hood(i + 3) === b) return i;
    }
    return null;
  });
  expect(border).not.toBeNull();
  await standAt(page, border as number);
  const zone = page.getByTestId("where-zone");
  await expect.poll(() => page.evaluate((c) => (window as unknown as Probes).__townStreet?.view.scene.playerCell === c, border)).toBe(true);
  await page.waitForTimeout(400);
  const before = ((await zone.textContent()) ?? "").trim();
  await page.keyboard.down("ArrowRight");
  await expect.poll(() => page.evaluate((c) => ((window as unknown as Probes).__townStreet?.view.scene.playerCell ?? 0) >= (c as number) + 2, border), { timeout: 5000 }).toBe(true);
  await page.keyboard.up("ArrowRight");
  await expect(zone).not.toHaveText(before);
  await expect(page.getByTestId("where-announce")).toContainText(((await zone.textContent()) ?? "").trim());

  // Having stopped there, the pocket and the label agree (owner, 2026-11-25: the two disagreed after a walk by
  // keys): crossed out of the neighbourhood the engine had the player in, they are on the new one's streets; still
  // inside it (the border walked over may lead into home's own neighbourhood), they are where they were.
  const now = ((await zone.textContent()) ?? "").trim();
  await page.waitForTimeout(1200);
  const sameHood = await page.evaluate(() => {
    const v = (window as unknown as Probes).__townStreet?.view;
    if (!v) return false;
    const home = v.map.buildings.find((b) => b.home);
    const d = (c: number) => v.map.district[c] ?? -1;
    return home !== undefined && d(home.front) === d(v.scene.playerCell);
  });
  await openPocket(page);
  if (sameHood) await expect(page.getByTestId("pocket-location")).toContainText("Home");
  else await expect(page.getByTestId("pocket-location")).toContainText(now, { timeout: 10_000 });
});

test("walk up to a door: a hint says Enter and the place; Enter goes in and the place panel opens", async ({ page }) => {
  await start(page, "4242");
  // A place with something to do whose door has two open cells to its left on the same row.
  const spot = await page.evaluate(() => {
    const v = (window as unknown as Probes).__townStreet?.view;
    if (!v) return null;
    const W = v.map.width;
    const cost = v.scene.grid.cost;
    const open = (i: number) => (cost[i] ?? 0) > 0 && i % W !== W - 1;
    // Places that open their panel at the door (the town hall, the chamber and the diner are walked into: e2e/interiors.spec.ts).
    const active = ["party_office", "news_office", "market", "union_hall", "community_centre"];
    const fronts = new Set(v.map.buildings.map((b) => b.front));
    const b = v.map.buildings.find((x) => x.place !== null && active.includes(x.kind) && x.front % W >= 2 && open(x.front - 1) && open(x.front - 2) && !fronts.has(x.front - 1) && !fronts.has(x.front - 2));
    return b ? { front: b.front, index: b.index } : null;
  });
  expect(spot).not.toBeNull();
  if (!spot) return;
  await tabToStreet(page);
  // Nobody standing in the street for this one: someone stood beside the door takes Enter first (the talk hook),
  // and the hint then names them, not the place.
  await page.evaluate(() => (window as unknown as Probes).__townStreet?.view.scene.setStanding([]));
  await standAt(page, spot.front - 2);
  const hint = page.locator(".hub-door-hint");
  await expect(hint).toBeHidden();
  // Short presses until the hint shows (each walks a few tenths of a tile).
  for (let k = 0; k < 20 && !(await hint.isVisible()); k++) {
    // (The street's people are listed again as time passes: keep it clear.)
    await page.evaluate(() => (window as unknown as Probes).__townStreet?.view.scene.setStanding([]));
    await page.keyboard.down("ArrowRight");
    await page.waitForTimeout(50);
    await page.keyboard.up("ArrowRight");
    await page.waitForTimeout(50);
  }
  await page.evaluate(() => (window as unknown as Probes).__townStreet?.view.scene.setStanding([]));
  await expect(hint).toBeVisible();
  await expect(hint).toHaveText(/^Enter: .+/);
  const name = ((await hint.textContent()) ?? "").replace(/^Enter: /, "");
  await expect(page.getByTestId("door-announce")).toHaveText(`Enter: ${name}`);
  expect(await page.evaluate(() => (window as unknown as Probes).__townStreet?.view.scene.doorHere)).toBe(spot.index);
  await page.keyboard.press("Enter");
  const panel = page.getByTestId("place-panel");
  await expect(panel.locator("#place-title")).toContainText(name);
  await expect(page.getByTestId("where-place")).toContainText(name);
});

test("keys do nothing while the focus is in the pocket", async ({ page }) => {
  await start(page, "4242");
  const pocket = await openPocket(page, "people");
  await pocket.getByTestId("notebook-tab-people").focus();
  const a = await player(page);
  await page.keyboard.down("ArrowRight");
  await page.waitForTimeout(300);
  await page.keyboard.up("ArrowRight");
  await expect(pocket.getByTestId("notebook-tab-places")).toBeFocused();
  await page.keyboard.press("d");
  await page.keyboard.press("s");
  const b = await player(page);
  expect(b).toEqual(a);
});
