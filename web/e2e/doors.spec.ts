import { mkdirSync } from "node:fs";
import { expect, test, type Page, type TestInfo } from "@playwright/test";
import { showPlaces } from "./pocket";

// Doors (DEC-114 §2, task B3): a street's houses have doors to knock at, and
// the same doors are rows in its panel (quick mode, DEC-051). Four doors by
// walking (a house's door, walked to, opens the doorstep; the fourth
// finishes the street), and four by the list. The doors and households come
// from the engine (the street's probe and the panel's rows), never from
// names or counts written here. Desktop and a 390×844 phone.
// MANDATE_CAPTURE=<dir> saves a screenshot of a doorstep.

const capture = process.env.MANDATE_CAPTURE;

interface DoorProbe {
  __townStreet?: {
    doors(): { building: number; front: number; hood: number; door: number }[];
    walkToDoor(building: number): boolean;
    view: { scene: { playerMoving: boolean } };
  } | null;
}

const phone = async (page: Page, info: TestInfo) => {
  if (info.project.name === "phone") await page.setViewportSize({ width: 390, height: 844 });
};

const start = async (page: Page) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("7");
  await page.getByText("Small", { exact: true }).click();
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
  await expect(page.locator(".town-street-stage[data-ready='1']")).toBeVisible({ timeout: 30_000 });
};

const shoot = async (page: Page, info: TestInfo, name: string) => {
  if (!capture) return;
  mkdirSync(capture, { recursive: true });
  await page.screenshot({ path: `${capture}/b3-${name}-${info.project.name}.png` });
};

/** Say something at the open doorstep, read the reply, and move on. */
async function doorstep(page: Page, k: number) {
  const sheet = page.getByTestId("door-sheet");
  await expect(sheet).toBeVisible();
  await expect(sheet.getByTestId("door-opener")).not.toContainText("⟦");
  const answers = (await sheet.getByTestId("door-opener").getAttribute("data-answers")) === "1";
  // Listen at most doors; make the case at the second that answers when there is a worry to make it on.
  const caseBtn = sheet.getByTestId("door-choice-case");
  if (answers && k === 1 && (await caseBtn.isEnabled())) {
    await caseBtn.click();
    await sheet.locator("[data-testid^='door-worry-']").first().click();
  } else {
    await sheet.getByTestId("door-choice-listen").click();
  }
  const reply = sheet.getByTestId("door-reply");
  await expect(reply).toBeVisible();
  await expect(reply).not.toContainText("⟦");
  return sheet;
}

test("four doors by walking: a house's door opens the doorstep, the fourth finishes the street", async ({ page }, info) => {
  test.setTimeout(120_000);
  await phone(page, info);
  await start(page);
  const doors = await page.evaluate(() => (window as unknown as DoorProbe).__townStreet?.doors() ?? []);
  // A neighbourhood with four doors on its houses (the engine's slots on the plan's houses).
  const byHood = new Map<number, typeof doors>();
  for (const d of doors) byHood.set(d.hood, [...(byHood.get(d.hood) ?? []), d]);
  const street = [...byHood.values()].find((ds) => ds.length >= 4);
  expect(street, "a street with four doors on its houses").toBeTruthy();
  if (!street) return;
  const four = street.sort((a, b) => a.door - b.door).slice(0, 4);
  for (const [k, d] of four.entries()) {
    const ok = await page.evaluate((b) => (window as unknown as DoorProbe).__townStreet?.walkToDoor(b) ?? false, d.building);
    expect(ok, `a route to door ${d.door}`).toBe(true);
    const sheet = page.getByTestId("door-sheet");
    await expect(sheet).toHaveAttribute("data-door", String(d.door), { timeout: 30_000 });
    if (k === 0) await shoot(page, info, "doorstep");
    await doorstep(page, k);
    if (k === 0) await shoot(page, info, "doorstep-reply");
    if (k === 3) await expect(sheet.getByTestId("door-street-done")).toBeVisible();
    await sheet.getByTestId("door-next").click();
    await expect(sheet).toHaveCount(0);
  }
});

test("four doors by the list: the street's rows knock the same doorsteps", async ({ page }, info) => {
  test.setTimeout(120_000);
  await phone(page, info);
  await start(page);
  // The streets the player stands on, from the Places tab (quick mode: no walking).
  const pocket = await showPlaces(page);
  await pocket.getByTestId("spot-streets").first().click();
  const panel = page.getByTestId("place-panel");
  await expect(panel).toBeVisible();
  const go = panel.getByRole("button", { name: "Go here" });
  if ((await go.count()) > 0) await go.click();
  const rows = panel.getByTestId("doors-here");
  await expect(rows).toBeVisible();
  await expect(rows).not.toContainText("⟦");
  for (let k = 0; k < 4; k++) {
    const knock = rows.locator("[data-testid^='door-knock-']").first();
    await expect(knock).toBeEnabled();
    const door = (await knock.getAttribute("data-testid"))?.replace("door-knock-", "") ?? "";
    await knock.click();
    const sheet = page.getByTestId("door-sheet");
    await expect(sheet).toHaveAttribute("data-door", door);
    await doorstep(page, k);
    if (k === 3) await expect(sheet.getByTestId("door-street-done")).toBeVisible();
    else await expect(rows.getByTestId(`door-row-${door}`)).toHaveAttribute("data-knocked", "1");
    await sheet.getByTestId("door-next").click();
    await expect(sheet).toHaveCount(0);
  }
  // The households met remember the call: their rows say how it went.
  await expect(rows.locator(".door-row .muted").first()).toBeVisible();
});
