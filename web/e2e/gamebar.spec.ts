import { expect, test } from "./csp";
import type { Page } from "@playwright/test";
import { useWideFont } from "./wide-font";

// Play note 18: on a narrow phone with three badges beside the pocket button the bar's date read
// "26 …" and "Turn …". The badges compress and wrap first; the date shortens before it truncates.
// Touch targets stay at least 44 px and the page never scrolls sideways.

const start = async (page: Page) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill("4242");
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
};

/** Three badges beside the pocket button, as a busy turn shows (the real badges when the run has them). */
async function threeBadges(page: Page) {
  await page.evaluate(() => {
    const btn = document.querySelector<HTMLElement>("[data-testid=pocket-button]");
    if (!btn) return;
    const have = btn.querySelectorAll(".badge").length;
    const icons = ["❗", "🤝", "🔔"];
    for (let i = have; i < 3; i++) {
      const b = document.createElement("span");
      b.className = "badge";
      b.textContent = `${icons[i]}${i + 2}`;
      btn.appendChild(b);
    }
  });
  await expect(page.getByTestId("pocket-button").locator(".badge")).toHaveCount(3);
}

// The wide-font cases use a deliberately wide face, so the result does not depend on the machine's fonts.
for (const [width, wide] of [[360, false], [390, false], [360, true], [320, true], [390, true]] as const) {
  test(`the bar's date and turn stay readable at ${width}px with three badges${wide ? " (wide font)" : ""}`, async ({ page }) => {
    if (wide) await useWideFont(page);
    await page.setViewportSize({ width, height: 844 });
    await start(page);
    await threeBadges(page);
    const bar = page.getByTestId("gamebar");

    // Neither the date nor the turn line is cut (the browser would clip them with an ellipsis).
    const clipped = await bar.evaluate((el) => {
      const out: string[] = [];
      for (const sel of [".gamebar-date strong", ".gamebar-sub"]) {
        const n = el.querySelector<HTMLElement>(sel);
        if (n && n.scrollWidth > n.clientWidth) out.push(`${sel} ${n.scrollWidth}>${n.clientWidth}`);
      }
      return out;
    });
    expect(clipped).toEqual([]);
    await expect(bar.getByText(/^Turn 0/)).toBeVisible();
    // The date shows (day and short month on a phone).
    await expect(bar.locator(".gamebar-date-short")).toBeVisible();
    expect(((await bar.locator(".gamebar-date-short").textContent()) ?? "").trim()).toMatch(/^\d{1,2} [A-Z][a-z]{2}$/);

    // No sideways scroll; every control in the bar keeps a 44 px touch target.
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    const barBox = await bar.boundingBox();
    expect((barBox?.x ?? 0) + (barBox?.width ?? 0)).toBeLessThanOrEqual(width + 0.5);
    // The date block (the turn line may wrap) stays inside the bar vertically.
    const dateBox = await bar.locator(".gamebar-date").boundingBox();
    expect(dateBox?.y ?? 0).toBeGreaterThanOrEqual((barBox?.y ?? 0) - 0.5);
    expect((dateBox?.y ?? 0) + (dateBox?.height ?? 0)).toBeLessThanOrEqual((barBox?.y ?? 0) + (barBox?.height ?? 0) + 0.5);
    if (wide) {
      const dir = process.env.BAR_SHOTS;
      if (dir) await page.screenshot({ path: `${dir}/bar-${width}.png`, clip: { x: 0, y: (barBox?.y ?? 0) - 6, width, height: (barBox?.height ?? 60) + 6 } });
    }
    for (const id of ["town-map-button", "pocket-button", "end-turn"]) {
      const box = await page.getByTestId(id).boundingBox();
      expect(box?.height ?? 0, id).toBeGreaterThanOrEqual(44);
      expect(box?.width ?? 0, id).toBeGreaterThanOrEqual(44);
      expect((box?.x ?? 0) + (box?.width ?? 0), id).toBeLessThanOrEqual(width + 0.5);
    }
  });
}

test("on a wide screen the date stays whole", async ({ page }, info) => {
  test.skip(info.project.name !== "desktop", "desktop width");
  await start(page);
  await expect(page.locator(".gamebar-date-full")).toBeVisible();
  await expect(page.locator(".gamebar-date-short")).toBeHidden();
});
