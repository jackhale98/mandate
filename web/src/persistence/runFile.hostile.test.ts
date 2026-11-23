// Hostile and damaged `.mandate` files (SECURITY_REVIEW.md, A5): they must be
// refused with a RunImportError (shown as a message), never hang, never throw
// anything else, and never reach the store.
import { IndexedDbStore } from "./indexedDbStore";
import { randomHex, type RunRecord } from "./saveStore";
import type { SaveMetaDto } from "../engine/protocol/SaveMetaDto";
import {
  MAX_HEADER_BYTES,
  MAX_RUN_FILE_BYTES,
  MAX_SAVE_BYTES,
  RUN_FILE_MAGIC,
  RunImportError,
  decodeRunFile,
  encodeRunFile,
  importRun,
  validateInProbe,
  type ProbeEngine,
} from "./runFile";

const RUN = "0123456789abcdef0123456789abcdef";
const meta: SaveMetaDto = {
  format_version: 25, run_id: RUN, revision: 1, parent_revision: null, device_id: "d", saved_at: "2026-01-01T00:00:00Z",
  engine_version: "0.1.0", content_hash: "c", protocol_version: 1, run_mode: "ironman", seed: "1",
  date: { day_number: 7, year: 2020, month: 1, day: 1 }, turn: 1,
};
const record: RunRecord = {
  runId: RUN, mode: "ironman", status: "active", revision: 1, meta,
  bytes: new Uint8Array(Array.from({ length: 500 }, (_, i) => (i * 7) & 0xff)), updatedAt: meta.saved_at,
};
const good = () => encodeRunFile(record, "0.1.0", new Date(2026, 10, 18, 12)).text;
const parts = () => good().split("\n") as [string, string, string];
const newStore = () => new IndexedDbStore(`h-${randomHex(4)}`);

async function refused(run: () => unknown) {
  let err: unknown;
  try {
    await run();
  } catch (e) {
    err = e;
  }
  expect(err).toBeInstanceOf(RunImportError);
  return err as RunImportError;
}

describe("hostile run files", () => {
  it("refuses a file over the size cap before decoding it (bytes and text)", async () => {
    const big = new Uint8Array(MAX_RUN_FILE_BYTES + 1);
    expect((await refused(() => decodeRunFile(big))).detail).toMatch(/too large/);
    expect((await refused(() => decodeRunFile("x".repeat(MAX_RUN_FILE_BYTES + 1)))).detail).toMatch(/too large/);
  });

  it("refuses a header that declares a huge save, without allocating it", async () => {
    const [magic, head, body] = parts();
    const h = JSON.parse(head) as { save: { length: number } };
    h.save.length = MAX_SAVE_BYTES + 1;
    expect((await refused(() => decodeRunFile(`${magic}\n${JSON.stringify(h)}\n${body}\n`))).detail).toMatch(/too large/);
    h.save.length = 4_000_000_000;
    await refused(() => decodeRunFile(`${magic}\n${JSON.stringify(h)}\n${body}\n`));
  });

  it("refuses a payload whose size does not match what the header declares (before decoding)", async () => {
    const [magic, head, body] = parts();
    await refused(() => decodeRunFile(`${magic}\n${head}\n${body}${body}\n`));
    await refused(() => decodeRunFile(`${magic}\n${head}\n${body.slice(0, 40)}\n`));
    await refused(() => decodeRunFile(`${magic}\n${head}\n\n`));
  });

  it("refuses a header that is too long or has no end", async () => {
    const [magic, , body] = parts();
    await refused(() => decodeRunFile(`${magic}\n{"format":1,"pad":"${"a".repeat(MAX_HEADER_BYTES + 10)}"}\n${body}\n`));
    await refused(() => decodeRunFile(`${magic}\n${"{".repeat(100_000)}`));
  });

  it("refuses text fields that are absurdly long, and ids that are not hex", async () => {
    const [magic, head, body] = parts();
    const h = JSON.parse(head) as { appVersion: string; run: { name: string | null; runId: string } };
    h.run.name = "N".repeat(5000);
    await refused(() => decodeRunFile(`${magic}\n${JSON.stringify(h)}\n${body}\n`));
    h.run.name = null;
    h.run.runId = "<img src=x onerror=alert(1)>";
    await refused(() => decodeRunFile(`${magic}\n${JSON.stringify(h)}\n${body}\n`));
  });

  it("refuses truncated files at every cut point without throwing anything else", () => {
    const text = good();
    for (let cut = 0; cut < text.length; cut += 7) {
      try {
        decodeRunFile(text.slice(0, cut));
        // A cut that only drops the final newline can still be a whole file.
        expect(cut).toBeGreaterThan(text.length - 3);
      } catch (e) {
        expect(e).toBeInstanceOf(RunImportError);
      }
    }
  });

  it("refuses random binary, a UTF-16 file and a JSON bomb of nested arrays", async () => {
    const noise = new Uint8Array(1_000_000);
    for (let i = 0; i < noise.length; i++) noise[i] = (i * 2654435761) >>> 24;
    await refused(() => decodeRunFile(noise));
    await refused(() => decodeRunFile(new Uint8Array([0xff, 0xfe, 0x4d, 0x00, 0x41, 0x00])));
    const [magic, , body] = parts();
    await refused(() => decodeRunFile(`${magic}\n${"[".repeat(20_000)}\n${body}\n`));
  });

  it("stores nothing from a refused file", async () => {
    const store = newStore();
    const [magic, head] = parts();
    const bad = [`${magic}\n${head}\n%%%%\n`, "hello", RUN_FILE_MAGIC];
    for (const text of bad) await refused(() => importRun(store, text, async () => meta));
    expect(await store.listRuns()).toHaveLength(0);
  });

  it("shows the text of a hostile town name as text only (it is stored as a plain string)", () => {
    const [magic, head, body] = parts();
    const h = JSON.parse(head) as { run: { name: string | null } };
    h.run.name = "<script>alert(1)</script>";
    const f = decodeRunFile(`${magic}\n${JSON.stringify(h)}\n${body}\n`);
    // Rendering is by React text nodes (no innerHTML anywhere on this path); the file name is slugged.
    expect(f.header.run.name).toBe("<script>alert(1)</script>");
  });
});

describe("the validation probe", () => {
  const fake = (over: Partial<ProbeEngine> = {}): ProbeEngine & { terminated: number } => {
    const p = {
      terminated: 0,
      ready: Promise.resolve(),
      readSaveMeta: async () => meta,
      importSave: async () => ({}),
      terminate() {
        p.terminated++;
      },
      ...over,
    };
    return p;
  };

  it("returns the metadata and always terminates the throwaway engine", async () => {
    const p = fake();
    expect(await validateInProbe(() => p, new Uint8Array(3))).toEqual(meta);
    expect(p.terminated).toBe(1);
  });

  it("terminates the engine when it rejects the save", async () => {
    const p = fake({ importSave: async () => Promise.reject(new Error("save data error: bad")) });
    await expect(validateInProbe(() => p, new Uint8Array(3))).rejects.toThrow("save data error");
    expect(p.terminated).toBe(1);
  });

  it("gives up on an engine that never answers", async () => {
    const p = fake({ importSave: () => new Promise(() => undefined) });
    const e = await refused(() => validateInProbe(() => p, new Uint8Array(3), 20));
    expect(e.code).toBe("corrupted");
    expect(e.detail).toMatch(/too long/);
    expect(p.terminated).toBe(1);
  });
});
