import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test, type Page } from "@playwright/test";
import { seedFor } from "./seeds";

// Street dressing (DEC-114, task B4): a fault on the street shows one cue
// while it is open, another while it is being repaired, and a third once it
// is repaired, on the same spot. The world is a real one: the seed comes from
// tests/seeds.json (`fault_seen_open_repairing_and_fixed`, which the Rust
// side checks: one fault of the player's town is seen at a turn's end in each
// state within the turns it names). The spec ends turns, reads the faults the
// engine's town state reports (`window.__townPlan.state`), waits for the
// street to show each one's cue on its spot, and photographs the spot the
// first time each fault shows each state. Nothing here decides a status or a
// cue. On the phone the viewport is 390 × 844. MANDATE_CAPTURE=<dir> also
// saves the photographs there.

const REQ = "fault_seen_open_repairing_and_fixed";
const capture = process.env.MANDATE_CAPTURE;
const seeds = JSON.parse(
  readFileSync(
    path.resolve(
      path.dirname(fileURLToPath(import.meta.url)),
      "../../tests/seeds.json",
    ),
    "utf8",
  ),
) as Record<string, { needs: string }>;
const TURNS = Number(
  /within (\d+) turns/.exec(seeds[REQ]?.needs ?? "")?.[1] ?? 0,
);

interface Fault {
  id: number;
  status: string;
  cue: string;
}
interface Probes {
  __townPlan?: { state?: { key: string; faults: Fault[] } };
  __townStreet?: {
    minutes: number;
    target: number;
    view: {
      map: {
        width: number;
        props: { kind: string; x: number; y: number; fault?: number }[];
      };
      centreOn(x: number, y: number): void;
      toScreen(x: number, y: number): { x: number; y: number };
      wake(): void;
    };
  } | null;
}

const start = async (page: Page, seed: string) => {
  await page.goto("./");
  await page.getByPlaceholder("e.g. 20260").fill(seed);
  await page.getByText("Small", { exact: true }).click();
  await page.getByRole("button", { name: "Begin" }).click();
  await expect(page.getByTestId("town")).toBeVisible();
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled();
  await expect(page.locator(".town-street-stage[data-ready='1']")).toBeVisible({
    timeout: 60_000,
  });
};

const endTurn = async (page: Page) => {
  const before = await page.locator(".gamebar-date strong").textContent();
  await page.getByRole("button", { name: "End turn" }).click();
  await expect(page.locator(".gamebar-date strong")).not.toHaveText(
    before ?? "",
    { timeout: 30_000 },
  );
  await expect(page.getByRole("button", { name: "End turn" })).toBeEnabled({
    timeout: 30_000,
  });
  // The passage of time ("2 weeks later") goes; the street comes round to the new turn's hour.
  const passage = page.getByTestId("turn-passage");
  if (await passage.count()) await passage.click().catch(() => undefined);
  await expect(passage).toHaveCount(0, { timeout: 20_000 });
  await expect
    .poll(
      () =>
        page.evaluate(() => {
          const s = (window as unknown as Probes).__townStreet;
          return s ? Math.abs(s.minutes - s.target) : 999;
        }),
      { timeout: 20_000 },
    )
    .toBeLessThan(2);
};

/** The engine's faults this turn, once the street shows every one's cue on a spot: [fault, cell] pairs. */
async function dressed(
  page: Page,
  key: string | null,
): Promise<{
  key: string;
  faults: (Fault & { x: number; y: number; kind: string })[];
}> {
  let out: {
    key: string;
    faults: (Fault & { x: number; y: number; kind: string })[];
  } | null = null;
  await expect
    .poll(
      async () => {
        out = await page.evaluate((prev) => {
          const w = window as unknown as Probes;
          const st = w.__townPlan?.state;
          const map = w.__townStreet?.view.map;
          if (!st || !map || st.key === prev) return null;
          const faults = st.faults.map((f) => {
            const p = map.props.find((x) => x.fault === f.id);
            return p ? { ...f, x: p.x, y: p.y, kind: p.kind } : null;
          });
          if (faults.some((f) => f === null)) return null;
          // The street shows what the view says: each fault's own cue.
          if (faults.some((f) => f && f.kind !== `cue_${f.cue}`)) return null;
          if (
            map.props.filter((x) => x.fault !== undefined).length !==
            st.faults.length
          )
            return null;
          return {
            key: st.key,
            faults: faults as (Fault & {
              x: number;
              y: number;
              kind: string;
            })[],
          };
        }, key);
        return out !== null;
      },
      { timeout: 30_000 },
    )
    .toBe(true);
  return out as unknown as {
    key: string;
    faults: (Fault & { x: number; y: number; kind: string })[];
  };
}

test("a fault shows a different cue on its spot while open, being repaired and new", async ({
  page,
  isMobile,
}, info) => {
  test.setTimeout(600_000);
  expect(TURNS, `the turn limit in tests/seeds.json's ${REQ}`).toBeGreaterThan(
    0,
  );
  if (isMobile) await page.setViewportSize({ width: 390, height: 844 });
  await start(page, seedFor(REQ));
  /** Per fault: the photographs by status, and the cue kind shown. */
  const shots = new Map<
    number,
    Map<
      string,
      { png: Buffer; close: Buffer; kind: string; cell: [number, number] }
    >
  >();
  let key: string | null = null;
  let done: number | null = null;
  for (let turn = 0; turn < TURNS && done === null; turn++) {
    await endTurn(page);
    const now = await dressed(page, key);
    key = now.key;
    for (const f of now.faults) {
      const seen = shots.get(f.id) ?? new Map();
      shots.set(f.id, seen);
      if (seen.has(f.status)) continue;
      // Look at the spot and photograph it: the whole screen, and the spot close up.
      const at = await page.evaluate(
        ([x, y]) => {
          const v = (window as unknown as Probes).__townStreet?.view;
          v?.centreOn(x + 0.5, y + 0.5);
          return v?.toScreen(x + 0.5, y + 0.5) ?? null;
        },
        [f.x, f.y] as const,
      );
      expect(at).not.toBeNull();
      await page.waitForTimeout(400);
      const stage = page.locator(".town-street-stage");
      const box = await stage.boundingBox();
      const png = await page.screenshot();
      const close =
        box && at
          ? await page.screenshot({
              clip: {
                x: Math.max(0, box.x + at.x - 90),
                y: Math.max(0, box.y + at.y - 100),
                width: 180,
                height: 150,
              },
            })
          : png;
      seen.set(f.status, { png, close, kind: f.kind, cell: [f.x, f.y] });
      if (seen.has("open") && seen.has("repairing") && seen.has("fixed"))
        done = f.id;
    }
  }
  expect(
    done,
    `a fault seen open, being repaired and repaired within ${TURNS} turns`,
  ).not.toBeNull();
  const seen = shots.get(done as number) as Map<
    string,
    { png: Buffer; close: Buffer; kind: string; cell: [number, number] }
  >;
  const states = ["open", "repairing", "fixed"].map(
    (s) =>
      seen.get(s) as {
        png: Buffer;
        close: Buffer;
        kind: string;
        cell: [number, number];
      },
  );
  // One spot, three different cues.
  expect(new Set(states.map((s) => s.cell.join(","))).size).toBe(1);
  expect(new Set(states.map((s) => s.kind)).size).toBe(3);
  ["open", "repairing", "fixed"].forEach((s, i) => {
    const st = states[i] as { png: Buffer; close: Buffer; kind: string };
    const name = `b4-fault-${s}-${info.project.name}`;
    writeFileSync(info.outputPath(`${name}.png`), st.png);
    writeFileSync(info.outputPath(`${name}-close.png`), st.close);
    void info.attach(name, { body: st.close, contentType: "image/png" });
    if (capture) {
      mkdirSync(capture, { recursive: true });
      writeFileSync(path.join(capture, `${name}.png`), st.png);
      writeFileSync(path.join(capture, `${name}-close.png`), st.close);
    }
  });
});
