import { IndexedDbStore } from "../../src/persistence/indexedDbStore";
import { randomHex, type RunRecord } from "../../src/persistence/saveStore";
import { describe as describeRun, latestRun } from "./runs";

// The landing page reads the game's own database (DEC-083) without Dexie.
// These tests write through the game's real store, so a schema change there
// that the landing page cannot read fails here.

const rec = (runId: string, status: RunRecord["status"], updatedAt: string, name?: string): RunRecord => ({
  runId,
  mode: "ironman",
  status,
  revision: 1,
  updatedAt,
  bytes: new Uint8Array([1]),
  name,
  meta: {
    format_version: 1, run_id: runId, revision: 1, parent_revision: null, device_id: "d", saved_at: updatedAt,
    engine_version: "0.1.0", content_hash: "x", protocol_version: 1, run_mode: "ironman", seed: "1",
    date: { day_number: 0, year: 2020, month: 11, day: 5 }, turn: 26,
  },
});

const exists = async (name: string) => (await indexedDB.databases()).some((d) => d.name === name);

describe("latestRun", () => {
  it("finds nothing, and creates nothing, when the game has never run here", async () => {
    const name = `landing-${randomHex(4)}`;
    expect(await latestRun(name)).toBeNull();
    expect(await exists(name)).toBe(false);
  });

  it("returns the most recently saved active run from the game's store", async () => {
    const name = `landing-${randomHex(4)}`;
    const store = new IndexedDbStore(name);
    await store.putRun(rec("old", "active", "2026-01-01T00:00:00Z", "Fenbridge"));
    await store.putRun(rec("new", "active", "2026-01-03T00:00:00Z", "Thornerhaven"));
    await store.putRun(rec("done", "ended", "2026-01-04T00:00:00Z", "Larkobridge"));
    const run = await latestRun(name);
    expect(run?.name).toBe("Thornerhaven");
    expect(describeRun(run ?? {})).toBe("Thornerhaven · turn 26 · 5 November 2020");
  });

  it("returns nothing when every run has ended", async () => {
    const name = `landing-${randomHex(4)}`;
    const store = new IndexedDbStore(name);
    await store.putRun(rec("done", "ended", "2026-01-04T00:00:00Z"));
    expect(await latestRun(name)).toBeNull();
  });
});
