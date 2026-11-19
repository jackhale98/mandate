import { expect, test, type Page } from "@playwright/test";
import { openPocket } from "./pocket";

// The street's battery budget (DEC-080 "Battery", owner 2026-11-19), on the
// phone profile with the CPU slowed 4× (CDP): main-thread busy time on the
// idle street, while only townsfolk move (frames capped at 15 fps) and after
// idle sleep (no input for 20 s: townsfolk stop and nothing is drawn).
// Thresholds are the measurements on the test machine (no GPU: the canvas is
// rasterised on the main thread, which dominates the ambient figure) plus
// headroom: ambient 86–87 % busy at 14 fps (it was 100 %, saturated, at the
// old 30 fps cap), idle sleep 1.0–1.2 % (was 100 %).

test.skip(({ isMobile }) => !isMobile, "the budget is measured on the phone profile");

interface Street {
  __townStreet?: { view: { stats: { frames: number; ticks: number }; asleep: boolean; paused: boolean } } | null;
}
const stats = (page: Page) =>
  page.evaluate(() => {
    const v = (window as unknown as Street).__townStreet?.view;
    return { frames: v?.stats.frames ?? 0, ticks: v?.stats.ticks ?? 0, asleep: v?.asleep ?? false, paused: v?.paused ?? false };
  });

const start = async (page: Page) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("4242");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.locator(".town-street-stage[data-ready='1']")).toBeVisible({ timeout: 60_000 });
};

test("idle street budget: ambient frames at 15 fps, then idle sleep draws nothing; a touch wakes it", async ({ page }) => {
  test.setTimeout(150_000);
  await start(page);
  const t0 = Date.now();
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Performance.enable");
  await cdp.send("Emulation.setCPUThrottlingRate", { rate: 4 });
  const busy = async () => (await cdp.send("Performance.getMetrics")).metrics.find((m) => m.name === "TaskDuration")?.value ?? 0;

  // Before idle sleep: only townsfolk move.
  await page.waitForTimeout(2000);
  const a0 = await busy();
  const s0 = await stats(page);
  await page.waitForTimeout(5000);
  const ambient = (await busy()) - a0;
  const s1 = await stats(page);
  const fps = (s1.frames - s0.frames) / 5;
  expect(s1.asleep).toBe(false);

  // Idle sleep comes about 20 s after the last input (the Begin tap, before t0).
  await expect.poll(async () => (await stats(page)).asleep, { timeout: 30_000, intervals: [500] }).toBe(true);
  expect(Date.now() - t0).toBeGreaterThan(10_000);
  const b0 = await busy();
  const i0 = await stats(page);
  await page.waitForTimeout(10_000);
  const idle = (await busy()) - b0;
  const i1 = await stats(page);
  console.log(`street budget (phone, CPU 4x): ambient ${(ambient * 1000).toFixed(0)} ms busy in 5 s (${((ambient / 5) * 100).toFixed(1)} %), ${fps.toFixed(1)} fps; idle sleep ${(idle * 1000).toFixed(0)} ms in 10 s (${((idle / 10) * 100).toFixed(1)} %)`);

  expect(fps).toBeLessThanOrEqual(16);
  expect(fps).toBeGreaterThanOrEqual(5);
  expect(ambient / 5).toBeLessThan(0.95);
  expect(idle / 10).toBeLessThan(0.05);
  // Asleep: no frame drawn, the loop not even ticking.
  expect(i1.frames).toBe(i0.frames);
  expect(i1.ticks).toBe(i0.ticks);
  expect(i1.asleep).toBe(true);

  // Any input wakes it.
  await page.touchscreen.tap(20, 200);
  await expect.poll(async () => (await stats(page)).frames, { timeout: 5000 }).toBeGreaterThan(i1.frames);
  expect((await stats(page)).asleep).toBe(false);
  await cdp.send("Emulation.setCPUThrottlingRate", { rate: 1 });
});

test("the street pauses while the pocket covers most of it, and runs again when it closes", async ({ page }) => {
  await start(page);
  await openPocket(page, "today");
  await expect.poll(async () => (await stats(page)).paused).toBe(true);
  const a = await stats(page);
  await page.waitForTimeout(1000);
  expect((await stats(page)).ticks).toBe(a.ticks);
  await page.getByTestId("pocket").getByRole("button", { name: "Close" }).first().click();
  await expect(page.getByTestId("pocket")).toHaveCount(0);
  await expect.poll(async () => (await stats(page)).paused).toBe(false);
  await expect.poll(async () => (await stats(page)).ticks).toBeGreaterThan(a.ticks);
});
