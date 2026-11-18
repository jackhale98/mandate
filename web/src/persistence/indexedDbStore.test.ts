import { IndexedDbStore } from "./indexedDbStore";
import { deviceId, randomHex, type RunRecord } from "./saveStore";

const rec = (runId: string, revision: number, updatedAt: string): RunRecord => ({
  runId,
  mode: "ironman",
  status: "active",
  revision,
  updatedAt,
  bytes: new Uint8Array([1, 2, 3]),
  meta: {
    format_version: 1, run_id: runId, revision, parent_revision: null, device_id: "d", saved_at: updatedAt,
    engine_version: "0.1.0", content_hash: "x", protocol_version: 1, run_mode: "ironman", seed: "1",
    date: { day_number: 0, year: 2000, month: 1, day: 1 }, turn: revision,
  },
});

describe("IndexedDbStore", () => {
  it("stores runs newest first and lists without bytes", async () => {
    const s = new IndexedDbStore(`t-${randomHex(4)}`);
    await s.putRun(rec("a", 1, "2026-01-01T00:00:00Z"));
    await s.putRun(rec("b", 1, "2026-01-02T00:00:00Z"));
    await s.putRun(rec("a", 2, "2026-01-03T00:00:00Z"));
    const runs = await s.listRuns();
    expect(runs.map((r) => r.runId)).toEqual(["a", "b"]);
    expect(runs[0]?.revision).toBe(2);
    expect("bytes" in (runs[0] ?? {})).toBe(false);
    expect(Array.from((await s.getRun("a"))?.bytes ?? [])).toEqual([1, 2, 3]);
  });

  it("deletes runs with their checkpoints and keeps a stable device id", async () => {
    const s = new IndexedDbStore(`t-${randomHex(4)}`);
    await s.putRun(rec("a", 1, "2026-01-01T00:00:00Z"));
    await s.putCheckpoint("a", 1, new Uint8Array([9]));
    expect(await s.listCheckpoints("a")).toEqual([1]);
    await s.deleteRun("a");
    expect(await s.listRuns()).toEqual([]);
    expect(await s.listCheckpoints("a")).toEqual([]);
    const id = await deviceId(s);
    expect(await deviceId(s)).toBe(id);
  });
});
