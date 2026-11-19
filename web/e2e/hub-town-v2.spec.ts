import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { expect, test, type Page } from "@playwright/test";

// Town layout v2 in the walkable town prototype (DEC-090): the town planned
// from its geography (the only layout since v1 was retired, DEC-085's
// pre-release note). Build the prototype first:
//   npx vite build --config spikes/hub/vite.config.ts --outDir ../../dist/spike-hub --emptyOutDir
// HUB_V2_SCREENS=<dir> also saves overview, street and town-map captures there.
const SPIKE = "../spike-hub/";
const built = existsSync(path.join(process.cwd(), "dist", "spike-hub", "index.html"));
const SCREENS = process.env.HUB_V2_SCREENS ?? "";

interface Probe {
  ready: boolean;
  layout: number;
  map: {
    version: number;
    width: number;
    height: number;
    municipality: string;
    terrain: Record<string, number>;
    solid: Record<string, number>;
    connectors?: { kind: string; x: number; y: number; w: number; h: number; axis: string; name: string }[];
    buildings: { place: number | null }[];
  } | null;
  plan: { districts: unknown[]; stages: Record<string, number> } | null;
  text: { routes: unknown[] } | null;
  view: {
    scene: { playerCell: number; player: { x: number; y: number; walk: unknown }; placePlayer(c: number): void; goToCell(c: number): boolean };
    camera: { state: { zoom: number } };
    centreOn(x: number, y: number): void;
    wake(): void;
  } | null;
  timings: Record<string, number>;
  setClock(m: number): void;
  look(x: number, y: number, zoom: number): void;
  overview(w?: number): { url: string; ms: number } | null;
}

async function open(page: Page, query: string) {
  const errors: string[] = [];
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto(`${SPIKE}?${query}`);
  await page.waitForFunction(() => (window as unknown as { __hub?: { ready: boolean } }).__hub?.ready === true, undefined, { timeout: 90_000 });
  return errors;
}

const probe = (page: Page) => page.evaluate(() => {
  const h = (window as unknown as { __hub: Probe }).__hub;
  return { layout: h.layout, version: h.map?.version, w: h.map?.width, h: h.map?.height, districts: h.plan?.districts.length ?? 0, places: h.map?.buildings.filter((b) => b.place !== null).length ?? 0, routes: h.text?.routes.length ?? 0, timings: h.timings };
});

test.describe("hub town prototype, layout v2", () => {
  test.skip(!built, "spike-hub is not built (see the comment at the top)");

  // 632: the owner's town (Brinerford) where the street faults were reported (DEC-090 D5 amended).
  for (const seed of ["4242", "2026", "99", "632"]) {
    test(`seed ${seed}: the v2 town loads with no console errors`, async ({ page }) => {
      const errors = await open(page, `seed=${seed}&t=720&speed=0&n=4`);
      await expect(page.locator("#loading")).toBeHidden();
      await expect(page.locator("#stage canvas")).toBeVisible();
      const p = await probe(page);
      expect(p.layout).toBe(2);
      expect(p.version).toBe(2);
      expect(p.districts).toBeGreaterThan(2);
      expect(p.places).toBeGreaterThan(8);
      expect(p.routes).toBe(p.places);
      // Every place is in the list (the keyboard and screen-reader route, DEC-051).
      await expect(page.locator("#place-list button[data-testid^='go-place-']")).toHaveCount(p.places);
      expect(errors).toEqual([]);
    });
  }

  test("seed 632: grown streets with 45° runs and recorded street ends reach the renderer", async ({ page }) => {
    const errors = await open(page, "seed=632&t=720&speed=0&n=0");
    const s = await page.evaluate(() => {
      const m = (window as unknown as { __hub: { map: { streetAxis?: Record<string, number>; streetEnds?: { kind: string }[] } | null } }).__hub.map;
      const axes = Object.values(m?.streetAxis ?? {});
      return { diagonal: axes.filter((a) => a === 1 || a === 2).length, ends: (m?.streetEnds ?? []).map((e) => e.kind) };
    });
    expect(s.diagonal).toBeGreaterThan(20);
    expect(s.ends.length).toBeGreaterThan(0);
    for (const k of s.ends) expect(["turning_circle", "hammerhead", "bollards", "plaza", "field_gate", "railing"]).toContain(k);
    expect(errors).toEqual([]);
  });

  test("the player walks across a bridge", async ({ page }) => {
    // Strathowick (seed 2026) stands on a river (class 2): its arterial crosses on a bridge.
    const errors = await open(page, "seed=2026&t=720&speed=0&n=0&art=placeholder");
    const ends = await page.evaluate(() => {
      const h = (window as unknown as { __hub: Probe }).__hub;
      const m = h.map;
      if (!m || !h.view) return null;
      const b = (m.connectors ?? []).find((c) => c.kind === "bridge" && Math.max(c.w, c.h) >= 6);
      if (!b) return null;
      const W = m.width;
      const walk = (x: number, y: number) => x >= 0 && y >= 0 && x < W && y < m.height && !m.solid[y * W + x];
      // A walkable cell just beyond each end of the deck, along its axis.
      const mid = b.axis === "ns" ? b.x + Math.floor(b.w / 2) : b.y + Math.floor(b.h / 2);
      const pick = (from: number, dir: number) => {
        for (let k = 1; k <= 4; k++) {
          const a = from + dir * k;
          const [x, y] = b.axis === "ns" ? [mid, a] : [a, mid];
          if (walk(x, y)) return y * W + x;
        }
        return -1;
      };
      const a = b.axis === "ns" ? pick(b.y, -1) : pick(b.x, -1);
      const z = b.axis === "ns" ? pick(b.y + b.h - 1, 1) : pick(b.x + b.w - 1, 1);
      return { a, z, bridge: b, W };
    });
    expect(ends).not.toBeNull();
    const { a, z, bridge } = ends as { a: number; z: number; bridge: { x: number; y: number; w: number; h: number } };
    expect(a).toBeGreaterThanOrEqual(0);
    expect(z).toBeGreaterThanOrEqual(0);
    const ok = await page.evaluate(
      ([a, z]) => {
        const h = (window as unknown as { __hub: Probe }).__hub;
        h.view?.scene.placePlayer(a as number);
        const started = h.view?.scene.goToCell(z as number) ?? false;
        h.view?.wake();
        return started;
      },
      [a, z],
    );
    expect(ok).toBe(true);
    // Sample the walk: the player is on the deck at some point, and arrives.
    let onDeck = false;
    for (let k = 0; k < 120; k++) {
      const s = await page.evaluate(() => {
        const h = (window as unknown as { __hub: Probe }).__hub;
        const p = h.view?.scene.player;
        return p ? { x: p.x, y: p.y, cell: h.view?.scene.playerCell ?? -1, walking: p.walk !== null } : null;
      });
      if (!s) break;
      if (s.x >= bridge.x && s.x < bridge.x + bridge.w && s.y >= bridge.y && s.y < bridge.y + bridge.h) onDeck = true;
      if (!s.walking && s.cell === z) break;
      await page.waitForTimeout(100);
    }
    const end = await page.evaluate(() => (window as unknown as { __hub: Probe }).__hub.view?.scene.playerCell);
    expect(onDeck).toBe(true);
    expect(end).toBe(z);
    expect(errors).toEqual([]);
  });

  test("layout v2 is the layout: no parameter selects it", async ({ page }) => {
    const errors = await open(page, "seed=4242&t=720&speed=0&n=0");
    const p = await probe(page);
    expect(p.layout).toBe(2);
    expect(p.version).toBe(2);
    expect(errors).toEqual([]);
  });
});

// Captures for review (only with HUB_V2_SCREENS): overview zoomed out, a street close-up (day), the town map overview,
// for seeds 4242, 2026 and the largest capital, in the pixel-art pack and the placeholder.
test.describe("hub town v2 captures", () => {
  test.skip(({ isMobile }) => !built || !SCREENS || isMobile, "set HUB_V2_SCREENS to a directory (desktop only)");
  test.setTimeout(240_000);
  const save = (name: string, buf: Buffer) => {
    mkdirSync(SCREENS, { recursive: true });
    writeFileSync(path.join(SCREENS, name), buf);
  };
  const cases: [string, string][] = [
    ["632", ""],
    ["4242", ""],
    ["2026", ""],
    ["4242", "largest"],
  ];
  for (const [seed, town] of cases) {
    for (const art of ["pixelkit", "placeholder"]) {
      test(`capture ${seed} ${town || "town"} ${art}`, async ({ page }) => {
        await page.setViewportSize({ width: 1600, height: 1000 });
        const q = `seed=${seed}&t=720&speed=0&n=10&res=css${town ? `&town=${town}` : ""}${art === "placeholder" ? "&art=placeholder" : ""}`;
        await open(page, q);
        const tag = `${seed}-${town || "town"}-${art}`;
        // Overview: all the way out.
        await page.evaluate(() => {
          const h = (window as unknown as { __hub: Probe }).__hub;
          const m = h.map;
          if (m) h.look(m.width / 2, m.height / 2, 1);
        });
        await page.waitForTimeout(600);
        save(`v2-${tag}-overview.png`, await page.locator("#stage").screenshot());
        // A street close-up by the square.
        await page.evaluate(() => (window as unknown as { __hub: { showStreet(z?: number): unknown } }).__hub.showStreet(3));
        await page.waitForTimeout(600);
        save(`v2-${tag}-street.png`, await page.locator("#stage").screenshot());
        if (art === "pixelkit") {
          const ov = await page.evaluate(() => (window as unknown as { __hub: Probe }).__hub.overview(1600));
          if (ov) save(`v2-${tag}-townmap.png`, Buffer.from(ov.url.split(",")[1] as string, "base64"));
          const t = await page.evaluate(() => (window as unknown as { __hub: Probe }).__hub.timings);
          save(`v2-${tag}-timings.json`, Buffer.from(JSON.stringify(t, null, 1)));
        }
      });
    }
  }
});
