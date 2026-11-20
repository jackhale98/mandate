// A bug report ("Report a problem"): one JSON file a tester saves and sends,
// from which a developer rebuilds the run exactly (`mandate-cli replay`).
// It holds the engine's part (`reportCore`: versions, the state hash, the run
// log since the start), the save, the last errors the app caught, the device
// facts the app already knows, and the tester's own words. Nothing else, and
// nothing is sent anywhere: the file is downloaded or shared by the tester.
import type { EngineClient } from "../../engine/client";
import type { RunModeDto } from "../../engine/protocol/RunModeDto";
import type { DateView } from "../../engine/protocol/DateView";
import { toBase64 } from "../../persistence/runFile";
import { deviceId, type SaveStore } from "../../persistence/saveStore";
import type { CaughtError } from "../../state/reportStore";

export const REPORT_FORMAT = "mandate-report";
export const REPORT_FORMAT_VERSION = 1;

/** The engine's part (`mandate_wasm::session::ReportCore`); the log is kept as the engine wrote it. */
export interface ReportCore {
  engine_version: string;
  content_hash: string;
  protocol_version: number;
  save_format_version: number;
  state_hash: string;
  turn: number;
  date: DateView;
  log: {
    new_game: { seed: string; size: string; scenario: string | null; storyteller: string | null; local_knowledge: boolean } | null;
    commands: unknown[];
    checkpoints: unknown[];
    note?: string | null;
    [k: string]: unknown;
  };
}

export interface DeviceInfo {
  viewport: { width: number; height: number };
  dpr: number;
  user_agent: string;
}

/** The save in a report: the running game's, or the latest stored one. */
export interface ReportSave {
  source: "live" | "autosave";
  revision: number;
  bytes: Uint8Array;
}

export interface Report {
  format: typeof REPORT_FORMAT;
  format_version: number;
  created_at: string;
  description: string;
  app: { version: string; build_id: string };
  engine: { engine_version: string; content_hash: string; protocol_version: number; save_format_version: number } | null;
  run: { seed: string | null; new_game: ReportCore["log"]["new_game"]; mode: RunModeDto | null; turn: number; date: DateView } | null;
  state_hash: string | null;
  log: ReportCore["log"] | null;
  save: { source: ReportSave["source"]; revision: number; length: number; base64: string } | null;
  errors: CaughtError[];
  device: DeviceInfo;
  /** What could not be included, and why (for the developer). */
  notes: string[];
}

/** What the app already knows about the device: nothing more. */
export function deviceInfo(): DeviceInfo {
  return {
    viewport: { width: window.innerWidth, height: window.innerHeight },
    dpr: window.devicePixelRatio || 1,
    user_agent: navigator.userAgent,
  };
}

/** The build: the app version and the hashed name of this bundle (it changes with every build). */
export function buildId(version: string, scriptUrl: string = import.meta.url): string {
  const file = scriptUrl.split(/[?#]/)[0]?.split("/").pop() ?? "";
  return file ? `${version}+${file}` : version;
}

/** `mandate-report-<seed>-<YYYY-MM-DD>.json` (the tester's local date). */
export function reportFileName(seed: string | null, now: Date): string {
  const p = (n: number) => String(n).padStart(2, "0");
  const day = `${now.getFullYear()}-${p(now.getMonth() + 1)}-${p(now.getDate())}`;
  const s = seed && /^\d+$/.test(seed) ? seed : "unknown";
  return `mandate-report-${s}-${day}.json`;
}

export interface ReportParts {
  core: ReportCore | null;
  save: ReportSave | null;
  mode: RunModeDto | null;
  notes: string[];
}

/** Put a report together (pure: everything comes in). */
export function assembleReport(parts: ReportParts, extra: { description: string; errors: CaughtError[]; device: DeviceInfo; appVersion: string; buildId: string; now: Date }): Report {
  const { core, save } = parts;
  return {
    format: REPORT_FORMAT,
    format_version: REPORT_FORMAT_VERSION,
    created_at: extra.now.toISOString(),
    description: extra.description.trim(),
    app: { version: extra.appVersion, build_id: extra.buildId },
    engine: core ? { engine_version: core.engine_version, content_hash: core.content_hash, protocol_version: core.protocol_version, save_format_version: core.save_format_version } : null,
    run: core ? { seed: core.log.new_game?.seed ?? null, new_game: core.log.new_game, mode: parts.mode, turn: core.turn, date: core.date } : null,
    state_hash: core?.state_hash ?? null,
    log: core?.log ?? null,
    save: save ? { source: save.source, revision: save.revision, length: save.bytes.length, base64: toBase64(save.bytes) } : null,
    errors: extra.errors,
    device: extra.device,
    notes: parts.notes,
  };
}

/** Where the report's run comes from. */
export interface ReportSources {
  /** The app's engine, when a game is running in it and it has not failed. */
  live: EngineClient | null;
  /** The running game, if any. */
  run: { runId: string; mode: RunModeDto; revision: number } | null;
  store: SaveStore;
  /** A second engine to read a stored save with (the app's own may have failed). */
  fresh: () => EngineClient;
  now: Date;
}

const errText = (e: unknown) => (e instanceof Error ? e.message : String(e));

/**
 * The run for a report: the running game when its engine answers (its save
 * and log as they are now); otherwise the latest stored save of this run (or
 * of the most recent run), read by a fresh engine. Whatever cannot be had is
 * left out and noted.
 */
export async function gatherRun(src: ReportSources): Promise<ReportParts> {
  const notes: string[] = [];
  if (src.live && src.run && !src.live.failed) {
    try {
      const bytes = await src.live.exportSave({
        run_id: src.run.runId,
        revision: src.run.revision,
        parent_revision: null,
        device_id: await deviceId(src.store),
        saved_at: src.now.toISOString(),
        run_mode: src.run.mode,
      });
      const core = JSON.parse(await src.live.reportCore()) as ReportCore;
      return { core, save: { source: "live", revision: src.run.revision, bytes }, mode: src.run.mode, notes };
    } catch (e) {
      notes.push(`the running game could not be read: ${errText(e)}`);
    }
  }
  let runId = src.run?.runId ?? null;
  if (!runId) {
    try {
      runId = (await src.store.listRuns())[0]?.runId ?? null;
    } catch (e) {
      notes.push(`saves unavailable: ${errText(e)}`);
    }
  }
  const rec = runId ? await src.store.getRun(runId).catch(() => undefined) : undefined;
  if (!rec) {
    notes.push("no saved game");
    return { core: null, save: null, mode: null, notes };
  }
  const save: ReportSave = { source: "autosave", revision: rec.revision, bytes: rec.bytes };
  const engine = src.fresh();
  try {
    await engine.ready;
    await engine.importSave(rec.bytes);
    const core = JSON.parse(await engine.reportCore()) as ReportCore;
    return { core, save, mode: rec.mode, notes };
  } catch (e) {
    notes.push(`the saved game could not be read: ${errText(e)}`);
    return { core: null, save, mode: rec.mode, notes };
  } finally {
    engine.terminate();
  }
}

/** The report file: its name and text. */
export function encodeReport(report: Report, now: Date): { name: string; text: string } {
  return { name: reportFileName(report.run?.seed ?? null, now), text: JSON.stringify(report, null, 1) };
}

/** A size for people: "840 KB". */
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
