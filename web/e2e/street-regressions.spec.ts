import { expect, test, type Page } from "@playwright/test";
import { showPlaces } from "./pocket";

// Owner-reported street bugs (2026-11-19):
// 1. "The player jumps to a different spot in the same area when somewhere is
//    clicked": a tap on open ground walked there, then moving the engine to
//    the neighbourhood's streets stood the player on the neighbourhood's
//    canonical street cell. Anywhere in the neighbourhood is its streets.
// 2. "Sometimes zoom gets locked" (desktop): with a place panel open beside
//    the street, the street was paused and ignored the wheel.

interface Probes {
  __townStreet?: {
    view: {
      map: { width: number; district: ArrayLike<number>; districts: { id: number; street: number }[]; buildings: { front: number }[] };
      scene: { playerCell: number; playerMoving: boolean; target: unknown; grid: { cost: ArrayLike<number> }; footprints: { x: number; y: number; w: number; h: number; roofRows?: number }[] };
      zoom: number;
      paused: boolean;
      stats: { frames: number };
      centreOn(x: number, y: number): void;
      toScreen(x: number, y: number): { x: number; y: number };
    };
    arrivals: number;
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

const probe = <T,>(page: Page, f: (s: NonNullable<Probes["__townStreet"]>) => T) =>
  page.evaluate((src) => {
    const s = (window as unknown as Probes).__townStreet;
    return s ? (new Function("s", `return (${src})(s)`) as (x: typeof s) => T)(s) : null;
  }, f.toString());

test("a tap on open ground in your neighbourhood: you stay where you walked to (no jump)", async ({ page }, info) => {
  await start(page, "4242");
  // Open ground in the player's neighbourhood, a few tiles from its canonical street cell, away from roofs and doors.
  const goal = await probe(page, (s) => {
    const v = s.view;
    const W = v.map.width;
    const cost = v.scene.grid.cost;
    const d = v.map.district;
    const mine = d[v.scene.playerCell];
    const street = v.map.districts[mine ?? -1]?.street ?? -1;
    const fronts = new Set(v.map.buildings.map((b) => b.front));
    const underRoof = (x: number, y: number) => v.scene.footprints.some((f) => f.w > 0 && x >= f.x - 1 && x < f.x + f.w + 1 && y >= f.y - (f.roofRows ?? 0) - 1 && y < f.y + f.h + 1);
    const px = v.scene.playerCell % W;
    const py = Math.floor(v.scene.playerCell / W);
    let best = -1;
    let bestD = Infinity;
    for (let i = 0; i < cost.length; i++) {
      const x = i % W;
      const y = Math.floor(i / W);
      if (d[i] !== mine || fronts.has(i) || underRoof(x, y)) continue;
      if ([-1, 0, 1].some((dy) => [-1, 0, 1].some((dx) => !((cost[(y + dy) * W + x + dx] ?? 0) > 0)))) continue;
      if (Math.hypot(x - (street % W), y - Math.floor(street / W)) < 3) continue;
      const dist = Math.hypot(x - px, y - py);
      if (dist >= 3 && dist < bestD) {
        bestD = dist;
        best = i;
      }
    }
    if (best < 0) return null;
    v.centreOn((best % W) + 0.5, Math.floor(best / W) + 0.5);
    return { cell: best, street, at: v.toScreen((best % W) + 0.5, Math.floor(best / W) + 0.5) };
  });
  expect(goal).not.toBeNull();
  if (!goal) return;
  expect(goal.cell).not.toBe(goal.street);
  const zoneBefore = ((await page.getByTestId("where-zone").textContent()) ?? "").trim();
  const box = await page.locator(".town-street-stage").boundingBox();
  if (!box) throw new Error("no street");
  if (info.project.name === "phone") await page.touchscreen.tap(box.x + goal.at.x, box.y + goal.at.y);
  else await page.mouse.click(box.x + goal.at.x, box.y + goal.at.y);
  // The walk ends there; the engine moves to the neighbourhood's streets; the player stays put.
  await expect.poll(() => probe(page, (s) => s.arrivals), { timeout: 15_000 }).toBeGreaterThan(0);
  await expect(page.getByTestId("where-place")).toHaveCount(0, { timeout: 10_000 });
  await page.waitForTimeout(800);
  expect(await probe(page, (s) => s.view.scene.playerCell)).toBe(goal.cell);
  expect(await probe(page, (s) => s.view.scene.playerMoving)).toBe(false);
  await expect(page.getByTestId("where-zone")).toHaveText(zoneBefore);
});

test("desktop: with a place's panel open beside the street, the wheel still zooms the street", async ({ page }, info) => {
  test.skip(info.project.name !== "desktop", "the panel stands beside the street on wide screens");
  await start(page, "4242");
  const pocket = await showPlaces(page);
  await pocket.getByTestId("spot-town_hall").click();
  const panel = page.getByTestId("place-panel");
  await expect(panel.getByRole("heading", { name: /Town hall/ })).toBeVisible();
  // The street is not covered, so it is not paused.
  await expect.poll(() => probe(page, (s) => s.view.paused)).toBe(false);
  const z0 = (await probe(page, (s) => s.view.zoom)) ?? 0;
  const f0 = (await probe(page, (s) => s.view.stats.frames)) ?? 0;
  const street = await page.locator(".town-street-stage").boundingBox();
  const pb = await panel.boundingBox();
  if (!street || !pb) throw new Error("no layout");
  // Over the street, left of the panel.
  await page.mouse.move(street.x + Math.min(200, (pb.x - street.x) / 2), street.y + street.height / 2);
  await page.mouse.wheel(0, -400);
  await expect.poll(async () => (await probe(page, (s) => s.view.zoom)) ?? 0).toBeGreaterThan(z0);
  expect((await probe(page, (s) => s.view.stats.frames)) ?? 0).toBeGreaterThan(f0);
  await expect(panel).toBeVisible();
});
