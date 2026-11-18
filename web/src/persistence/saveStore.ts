// Storage abstraction (DEC-052, TECH-009). The game talks only to SaveStore;
// IndexedDB is the MVP implementation and cloud providers plug in later.
import type { RunModeDto } from "../engine/protocol/RunModeDto";
import type { SaveMetaDto } from "../engine/protocol/SaveMetaDto";

export type RunStatus = "active" | "ended" | "diverged";

export interface RunRecord {
  runId: string;
  mode: RunModeDto;
  status: RunStatus;
  revision: number;
  meta: SaveMetaDto;
  bytes: Uint8Array;
  updatedAt: string;
}

export type RunSummary = Omit<RunRecord, "bytes">;

export interface SaveStore {
  listRuns(): Promise<RunSummary[]>;
  getRun(runId: string): Promise<RunRecord | undefined>;
  putRun(record: RunRecord): Promise<void>;
  deleteRun(runId: string): Promise<void>;
  /** Story-mode chapter checkpoints (DEC-008). */
  putCheckpoint(runId: string, revision: number, bytes: Uint8Array): Promise<void>;
  listCheckpoints(runId: string): Promise<number[]>;
  getSetting<T>(key: string): Promise<T | undefined>;
  putSetting<T>(key: string, value: T): Promise<void>;
}

/** Random hex identifier with `bytes` bytes of entropy. */
export function randomHex(bytes = 16): string {
  const a = new Uint8Array(bytes);
  crypto.getRandomValues(a);
  return Array.from(a, (b) => b.toString(16).padStart(2, "0")).join("");
}

/** Stable per-device identifier for sync lineage. */
export async function deviceId(store: SaveStore): Promise<string> {
  const existing = await store.getSetting<string>("device_id");
  if (existing) return existing;
  const id = randomHex(8);
  await store.putSetting("device_id", id);
  return id;
}
