import { IndexedDbStore } from "./indexedDbStore";
import { randomHex, type RunRecord } from "./saveStore";
import type { RunModeDto } from "../engine/protocol/RunModeDto";
import type { SaveMetaDto } from "../engine/protocol/SaveMetaDto";
import {
  RUN_FILE_MAGIC,
  RunImportError,
  classifyEngineError,
  crc32,
  decodeRunFile,
  encodeRunFile,
  exportRun,
  fromBase64,
  importRun,
  runFileName,
  toBase64,
  type ImportErrorCode,
  type SaveValidator,
} from "./runFile";

const RUN = "0123456789abcdef0123456789abcdef";

const meta = (runId: string, revision: number, mode: RunModeDto = "ironman"): SaveMetaDto => ({
  format_version: 10, run_id: runId, revision, parent_revision: revision > 1 ? revision - 1 : null, device_id: "d",
  saved_at: `2026-01-0${Math.min(revision, 9)}T00:00:00Z`, engine_version: "0.1.0", content_hash: "c", protocol_version: 1,
  run_mode: mode, seed: "1", date: { day_number: revision * 7, year: 2020, month: 1, day: 1 }, turn: revision,
});

/** A stand-in for engine save bytes: the revision is the first byte. */
const saveBytes = (revision: number) => new Uint8Array([revision, 77, 78, 68, 84, ...Array.from({ length: 300 }, (_, i) => (i * 31) & 0xff)]);

const rec = (runId: string, revision: number, mode: RunModeDto = "ironman", name?: string): RunRecord => ({
  runId, mode, status: "active", revision, meta: meta(runId, revision, mode), bytes: saveBytes(revision),
  updatedAt: meta(runId, revision).saved_at, ...(name ? { name } : {}),
});

/** Fake engine: accepts the bytes produced by `saveBytes`. */
const engineOk = (runId = RUN, mode: RunModeDto = "ironman"): SaveValidator => async (bytes) => meta(runId, bytes[0] ?? 0, mode);
const engineFails = (message: string): SaveValidator => async () => {
  throw new Error(message);
};

const newStore = () => new IndexedDbStore(`t-${randomHex(4)}`);
const fileOf = (r: RunRecord) => encodeRunFile(r, "0.1.0", new Date(2026, 10, 18, 12)).text;

async function expectCode(p: Promise<unknown> | (() => unknown), code: ImportErrorCode) {
  try {
    await (typeof p === "function" ? p() : p);
  } catch (e) {
    expect(e).toBeInstanceOf(RunImportError);
    expect((e as RunImportError).code).toBe(code);
    return;
  }
  throw new Error(`expected ${code}`);
}

describe("run file format", () => {
  it("round-trips the save bytes exactly with the run's metadata", () => {
    const r = rec(RUN, 4, "story", "Port Ellery");
    const { name, text } = encodeRunFile(r, "0.1.0", new Date(2026, 10, 18, 12));
    expect(name).toBe("mandate-port-ellery-2026-11-18.mandate");
    expect(text.startsWith(`${RUN_FILE_MAGIC}\n`)).toBe(true);
    const f = decodeRunFile(new TextEncoder().encode(text));
    expect(Array.from(f.bytes)).toEqual(Array.from(r.bytes));
    expect(f.header.format).toBe(1);
    expect(f.header.appVersion).toBe("0.1.0");
    expect(f.header.run).toMatchObject({ runId: RUN, mode: "story", revision: 4, name: "Port Ellery", saveFormatVersion: 10, turn: 4 });
  });

  it("names files after the town, or the run when the town is unknown", () => {
    const day = new Date(2026, 0, 5);
    expect(runFileName({ name: "Saint-Étienne sur Mer", runId: RUN }, day)).toBe("mandate-saint-etienne-sur-mer-2026-01-05.mandate");
    expect(runFileName({ name: null, runId: RUN }, day)).toBe("mandate-run-01234567-2026-01-05.mandate");
    expect(runFileName({ name: "///", runId: RUN }, day)).toBe("mandate-run-01234567-2026-01-05.mandate");
  });

  it("base64 and crc32 helpers are standard", () => {
    const b = new Uint8Array(70_000).map((_, i) => i & 0xff);
    expect(Array.from(fromBase64(toBase64(b)))).toEqual(Array.from(b));
    expect(crc32(new TextEncoder().encode("123456789"))).toBe("cbf43926");
  });

  it("tolerates CRLF line endings and a byte-order mark", () => {
    const text = "\uFEFF" + fileOf(rec(RUN, 2)).replace(/\n/g, "\r\n");
    expect(decodeRunFile(text).header.run.revision).toBe(2);
  });
});

describe("import errors", () => {
  it("not a Mandate file: other text, binary, empty, an engine save without the wrapper", async () => {
    await expectCode(() => decodeRunFile("hello world"), "not-mandate");
    await expectCode(() => decodeRunFile(new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0xff, 0xfe, 0x00])), "not-mandate");
    await expectCode(() => decodeRunFile(""), "not-mandate");
    await expectCode(() => decodeRunFile(new TextEncoder().encode("MNDT\u000a\u0000\u0000")), "not-mandate");
    await expectCode(importRun(newStore(), "{}", engineOk()), "not-mandate");
  });

  it("a newer file format than this app", async () => {
    const text = fileOf(rec(RUN, 1)).replace('{"format":1,', '{"format":2,');
    await expectCode(() => decodeRunFile(text), "too-new");
  });

  it("a newer save format than this app (the engine refuses it)", async () => {
    const p = importRun(newStore(), fileOf(rec(RUN, 1)), engineFails("save format v99 is newer than this build supports (v10)"));
    await expectCode(p, "too-new");
  });

  it("corrupted: truncated, damaged payload, broken header, header shape, mismatched header", async () => {
    const good = fileOf(rec(RUN, 3));
    const [magic, head, body] = good.split("\n") as [string, string, string];
    // Truncated download.
    await expectCode(() => decodeRunFile(good.slice(0, good.length - 40)), "corrupted");
    await expectCode(() => decodeRunFile(`${magic}\n${head.slice(0, 20)}`), "corrupted");
    // One character of the payload changed (same length): the checksum catches it.
    const flipped = body.slice(0, 50) + (body[50] === "A" ? "B" : "A") + body.slice(51);
    await expectCode(() => decodeRunFile(`${magic}\n${head}\n${flipped}\n`), "corrupted");
    // Not base64.
    await expectCode(() => decodeRunFile(`${magic}\n${head}\n%%%%\n`), "corrupted");
    // Header shape.
    const h = JSON.parse(head) as Record<string, unknown> & { run: Record<string, unknown> };
    for (const mutate of [
      (x: typeof h) => void (x.run.mode = "hardcore"),
      (x: typeof h) => void (x.run.revision = -1),
      (x: typeof h) => void (x.run.runId = "../../etc"),
      (x: typeof h) => void delete x.save,
      (x: typeof h) => void (x.run.date = null),
    ]) {
      const c = structuredClone(h);
      mutate(c);
      await expectCode(() => decodeRunFile(`${magic}\n${JSON.stringify(c)}\n${body}\n`), "corrupted");
    }
    // The engine's own integrity check fails.
    await expectCode(importRun(newStore(), good, engineFails("save integrity check failed")), "corrupted");
    await expectCode(importRun(newStore(), good, engineFails("save file is truncated or malformed")), "corrupted");
    // The header does not describe the save inside it.
    await expectCode(importRun(newStore(), good, async () => meta(RUN, 9)), "corrupted");
    await expectCode(importRun(newStore(), good, engineOk(RUN, "story")), "corrupted");
  });

  it("a save the current rules cannot load", async () => {
    const good = fileOf(rec(RUN, 1));
    await expectCode(importRun(newStore(), good, engineFails("this save needs rules that no longer exist: policy.rent_cap")), "incompatible");
    await expectCode(
      importRun(newStore(), good, engineFails("save data error: this save is from an older development build (format v3) and cannot be migrated")),
      "incompatible",
    );
    const e = classifyEngineError(new Error("this save needs rules that no longer exist: policy.rent_cap"));
    expect(e.detail).toContain("policy.rent_cap");
  });

  it("nothing is stored when an import fails", async () => {
    const s = newStore();
    await expectCode(importRun(s, fileOf(rec(RUN, 1)), engineFails("save integrity check failed")), "corrupted");
    expect(await s.listRuns()).toEqual([]);
  });
});

describe("import rules (DEC-082)", () => {
  it("a run with no local counterpart is stored as-is", async () => {
    const s = newStore();
    const r = rec(RUN, 5, "ironman", "Ashby");
    expect(await importRun(s, fileOf(r), engineOk())).toEqual({ kind: "added", runId: RUN });
    const got = await s.getRun(RUN);
    expect(got).toMatchObject({ runId: RUN, mode: "ironman", revision: 5, name: "Ashby", status: "active" });
    expect(Array.from(got?.bytes ?? [])).toEqual(Array.from(r.bytes));
  });

  it("export from one store, import into an empty one: identical record", async () => {
    const a = newStore();
    await a.putRun(rec(RUN, 7, "ironman", "Ashby"));
    const { text } = await exportRun(a, RUN, "0.1.0");
    const b = newStore();
    await importRun(b, new TextEncoder().encode(text), engineOk());
    const [ra, rb] = [await a.getRun(RUN), await b.getRun(RUN)];
    expect(rb).toEqual(ra);
  });

  it("Ironman: a newer or equal revision replaces the local run", async () => {
    const s = newStore();
    await s.putRun(rec(RUN, 3));
    expect(await importRun(s, fileOf(rec(RUN, 6)), engineOk())).toEqual({ kind: "replaced", runId: RUN });
    expect((await s.getRun(RUN))?.revision).toBe(6);
    expect(await importRun(s, fileOf(rec(RUN, 6)), engineOk())).toEqual({ kind: "replaced", runId: RUN });
    expect(await s.listRuns()).toHaveLength(1);
  });

  it("Ironman: an older revision is refused (no rewinding) and the local run is untouched", async () => {
    const s = newStore();
    await s.putRun(rec(RUN, 8));
    await expectCode(importRun(s, fileOf(rec(RUN, 3)), engineOk()), "rewind");
    const runs = await s.listRuns();
    expect(runs).toHaveLength(1);
    expect(runs[0]?.revision).toBe(8);
  });

  it("an older file is refused when the local run is Ironman, whatever the file says", async () => {
    const s = newStore();
    await s.putRun(rec(RUN, 8, "ironman"));
    await expectCode(importRun(s, fileOf(rec(RUN, 3, "story")), engineOk(RUN, "story")), "rewind");
  });

  it("Story: an older revision is kept as a separate copy", async () => {
    const s = newStore();
    await s.putRun(rec(RUN, 8, "story"));
    const out = await importRun(s, fileOf(rec(RUN, 3, "story")), engineOk(RUN, "story"));
    expect(out.kind).toBe("copied");
    expect(out.runId).not.toBe(RUN);
    expect((await s.getRun(RUN))?.revision).toBe(8);
    expect((await s.getRun(out.runId))?.revision).toBe(3);
    expect(await s.listRuns()).toHaveLength(2);
  });
});
