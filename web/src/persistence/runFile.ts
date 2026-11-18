// Run export and import (DEC-082, proposed; DEC-052 "file export and import
// remain the manual fallback"). A `.mandate` file carries one run's latest save
// exactly as stored (the engine's versioned, compressed, integrity-hashed bytes)
// plus the run's metadata, so a player can back up a run and restore it on
// another device or after clearing the browser.
//
// File layout (UTF-8 text, three lines):
//   MANDATE RUN EXPORT
//   {"format":1, ...header JSON...}
//   <base64 of the engine save bytes>
//
// The header is readable without the engine; the engine still validates the
// save itself before anything is stored.
import type { RunModeDto } from "../engine/protocol/RunModeDto";
import type { SaveMetaDto } from "../engine/protocol/SaveMetaDto";
import type { DateView } from "../engine/protocol/DateView";
import { randomHex, type RunRecord, type RunStatus, type SaveStore } from "./saveStore";

export const RUN_FILE_MAGIC = "MANDATE RUN EXPORT";
export const RUN_FILE_FORMAT = 1;
export const RUN_FILE_EXTENSION = ".mandate";
/** Refuse larger files before reading them (corrupt or hostile input). */
export const MAX_RUN_FILE_BYTES = 64 * 1024 * 1024;

export interface RunFileHeader {
  format: number;
  appVersion: string;
  exportedAt: string;
  run: {
    runId: string;
    mode: RunModeDto;
    status: RunStatus;
    revision: number;
    /** The player's town, when known. */
    name: string | null;
    /** Wall-clock time of the save (metadata only). */
    savedAt: string;
    saveFormatVersion: number;
    engineVersion: string;
    contentHash: string;
    turn: number;
    date: DateView;
  };
  save: { length: number; crc32: string };
}

export interface RunFile {
  header: RunFileHeader;
  bytes: Uint8Array;
}

/** Why an import was refused; each maps to a Fluent key `ui-import-error-<code>`. */
export type ImportErrorCode = "not-mandate" | "too-new" | "corrupted" | "incompatible" | "rewind";

export class RunImportError extends Error {
  constructor(
    readonly code: ImportErrorCode,
    readonly detail = "",
  ) {
    super(detail ? `${code}: ${detail}` : code);
    this.name = "RunImportError";
  }
}

/** How a valid file was stored. */
export type ImportOutcome =
  | { kind: "added"; runId: string }
  | { kind: "replaced"; runId: string }
  /** An older Story-mode revision of a run that exists here, kept beside it. */
  | { kind: "copied"; runId: string };

/** Loads the save through the engine (and so the current rules); returns its metadata. */
export type SaveValidator = (bytes: Uint8Array) => Promise<SaveMetaDto>;

// ---------------------------------------------------------------------------
// Encoding helpers

const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

/** CRC-32 (IEEE) as 8 hex digits: catches truncated or damaged downloads. */
export function crc32(bytes: Uint8Array): string {
  let c = 0xffffffff;
  for (const b of bytes) c = (CRC_TABLE[(c ^ b) & 0xff] ?? 0) ^ (c >>> 8);
  return ((c ^ 0xffffffff) >>> 0).toString(16).padStart(8, "0");
}

export function toBase64(bytes: Uint8Array): string {
  let s = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) s += String.fromCharCode(...bytes.subarray(i, i + chunk));
  return btoa(s);
}

export function fromBase64(text: string): Uint8Array {
  const s = atob(text);
  const out = new Uint8Array(s.length);
  for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i);
  return out;
}

/** `mandate-<town or run>-<YYYY-MM-DD>.mandate` (local date), safe on every file system. */
export function runFileName(run: { name: string | null; runId: string }, now: Date): string {
  const slug = (run.name ?? "")
    .normalize("NFKD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 40);
  const name = slug || `run-${run.runId.slice(0, 8)}`;
  return `mandate-${name}-${localDay(now)}${RUN_FILE_EXTENSION}`;
}

/** Local calendar date as YYYY-MM-DD (file names use the player's day). */
function localDay(d: Date): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

// ---------------------------------------------------------------------------
// Encode and decode

export function encodeRunFile(record: RunRecord, appVersion: string, now = new Date()): { name: string; text: string } {
  const header: RunFileHeader = {
    format: RUN_FILE_FORMAT,
    appVersion,
    exportedAt: now.toISOString(),
    run: {
      runId: record.runId,
      mode: record.mode,
      status: record.status,
      revision: record.revision,
      name: record.name ?? null,
      savedAt: record.meta.saved_at,
      saveFormatVersion: record.meta.format_version,
      engineVersion: record.meta.engine_version,
      contentHash: record.meta.content_hash,
      turn: record.meta.turn,
      date: record.meta.date,
    },
    save: { length: record.bytes.length, crc32: crc32(record.bytes) },
  };
  const text = `${RUN_FILE_MAGIC}\n${JSON.stringify(header)}\n${toBase64(record.bytes)}\n`;
  return { name: runFileName(header.run, now), text };
}

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null && !Array.isArray(v);
const isStr = (v: unknown): v is string => typeof v === "string";
const isNat = (v: unknown): v is number => typeof v === "number" && Number.isInteger(v) && v >= 0;

function checkHeader(h: unknown): RunFileHeader {
  const bad = (what: string) => new RunImportError("corrupted", `header: ${what}`);
  if (!isObj(h)) throw bad("not an object");
  if (!isNat(h.format)) throw bad("format");
  // Checked before the rest of the shape: a newer layout may differ.
  if (h.format > RUN_FILE_FORMAT) throw new RunImportError("too-new", `file format v${h.format}`);
  if (!isStr(h.appVersion) || !isStr(h.exportedAt)) throw bad("versions");
  const r = h.run;
  if (!isObj(r)) throw bad("run");
  if (!isStr(r.runId) || !/^[0-9a-f]{8,64}$/.test(r.runId)) throw bad("run id");
  if (r.mode !== "ironman" && r.mode !== "story") throw bad("mode");
  if (r.status !== "active" && r.status !== "ended" && r.status !== "diverged") throw bad("status");
  if (!isNat(r.revision) || !isNat(r.saveFormatVersion) || !isNat(r.turn)) throw bad("numbers");
  if (!(r.name === null || isStr(r.name)) || !isStr(r.savedAt) || !isStr(r.engineVersion) || !isStr(r.contentHash)) throw bad("fields");
  const d = r.date;
  if (!isObj(d) || !isNat(d.day_number) || !isNat(d.year) || !isNat(d.month) || !isNat(d.day)) throw bad("date");
  const s = h.save;
  if (!isObj(s) || !isNat(s.length) || !isStr(s.crc32)) throw bad("save");
  return h as unknown as RunFileHeader;
}

/** Parse a `.mandate` file. Throws `RunImportError` (never stores anything). */
export function decodeRunFile(data: Uint8Array | string): RunFile {
  if (typeof data !== "string" && data.length > MAX_RUN_FILE_BYTES) throw new RunImportError("corrupted", "file too large");
  let text: string;
  try {
    text = typeof data === "string" ? data : new TextDecoder("utf-8", { fatal: true }).decode(data);
  } catch {
    throw new RunImportError("not-mandate");
  }
  text = text.replace(/^\uFEFF/, "");
  const firstBreak = text.indexOf("\n");
  const first = (firstBreak < 0 ? text : text.slice(0, firstBreak)).replace(/\r$/, "");
  if (first !== RUN_FILE_MAGIC) throw new RunImportError("not-mandate");
  const lines = text.slice(firstBreak + 1).split(/\r?\n/);
  let raw: unknown;
  try {
    raw = JSON.parse(lines[0] ?? "");
  } catch {
    throw new RunImportError("corrupted", "header is not JSON");
  }
  const header = checkHeader(raw);
  let bytes: Uint8Array;
  try {
    bytes = fromBase64((lines[1] ?? "").trim());
  } catch {
    throw new RunImportError("corrupted", "save is not base64");
  }
  if (bytes.length !== header.save.length) throw new RunImportError("corrupted", "save length");
  if (crc32(bytes) !== header.save.crc32) throw new RunImportError("corrupted", "save checksum");
  return { header, bytes };
}

/** Map an engine save error (`mandate-save` / `Engine::from_state` messages) to an import error. */
export function classifyEngineError(e: unknown): RunImportError {
  if (e instanceof RunImportError) return e;
  const msg = e instanceof Error ? e.message : String(e);
  if (/newer than this build/i.test(msg)) return new RunImportError("too-new", msg);
  if (/no longer exist|cannot be migrated|older development build/i.test(msg)) return new RunImportError("incompatible", msg);
  if (/malformed|truncated|integrity|not a mandate save|save data error/i.test(msg)) return new RunImportError("corrupted", msg);
  // Anything else the engine refuses (for example a world it cannot rebuild)
  // cannot be played under the current rules.
  return new RunImportError("incompatible", msg);
}

// ---------------------------------------------------------------------------
// Store-level API (works over any SaveStore)

/** Build the export file for a stored run. */
export async function exportRun(store: SaveStore, runId: string, appVersion: string, now = new Date()) {
  const record = await store.getRun(runId);
  if (!record) throw new Error(`run ${runId} not found`);
  return encodeRunFile(record, appVersion, now);
}

/**
 * Validate a `.mandate` file and store its run (DEC-082 rules):
 * - no local run with this id: stored as-is;
 * - a local run with this id: replaced when the file's revision is >= the
 *   local one; an older Ironman revision is refused (no rewinding); an older
 *   Story revision is kept as a separate copy.
 */
export async function importRun(store: SaveStore, data: Uint8Array | string, validate: SaveValidator): Promise<ImportOutcome> {
  const { header, bytes } = decodeRunFile(data);
  let meta: SaveMetaDto;
  try {
    meta = await validate(bytes);
  } catch (e) {
    throw classifyEngineError(e);
  }
  // The header must describe the save it carries.
  if (meta.revision !== header.run.revision || meta.run_mode !== header.run.mode) {
    throw new RunImportError("corrupted", "header does not match the save");
  }
  const h = header.run;
  const record: RunRecord = {
    runId: h.runId,
    mode: h.mode,
    status: h.status,
    revision: h.revision,
    meta,
    bytes,
    updatedAt: meta.saved_at,
    ...(h.name ? { name: h.name } : {}),
  };
  const local = await store.getRun(h.runId);
  if (!local) {
    await store.putRun(record);
    return { kind: "added", runId: h.runId };
  }
  if (h.revision >= local.revision) {
    await store.putRun(record);
    return { kind: "replaced", runId: h.runId };
  }
  if (h.mode === "ironman" || local.mode === "ironman") {
    throw new RunImportError("rewind", `file revision ${h.revision} < local revision ${local.revision}`);
  }
  const runId = randomHex(16);
  await store.putRun({ ...record, runId });
  return { kind: "copied", runId };
}
