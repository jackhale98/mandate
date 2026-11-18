import Dexie, { type Table } from "dexie";
import type { RunRecord, RunSummary, SaveStore } from "./saveStore";

interface CheckpointRow {
  runId: string;
  revision: number;
  bytes: Uint8Array;
}
interface JournalRow {
  id?: number;
  runId: string;
  revision: number;
  command: string;
}
interface SettingRow {
  key: string;
  value: unknown;
}

class MandateDb extends Dexie {
  runs!: Table<RunRecord, string>;
  checkpoints!: Table<CheckpointRow, [string, number]>;
  settings!: Table<SettingRow, string>;
  journal!: Table<JournalRow, number>;
  constructor(name: string) {
    super(name);
    this.version(1).stores({
      runs: "runId, updatedAt",
      checkpoints: "[runId+revision], runId",
      settings: "key",
    });
    this.version(2).stores({
      journal: "++id, runId, [runId+revision]",
    });
  }
}

export class IndexedDbStore implements SaveStore {
  private db: MandateDb;
  constructor(name = "mandate") {
    this.db = new MandateDb(name);
  }

  async listRuns(): Promise<RunSummary[]> {
    const rows = await this.db.runs.orderBy("updatedAt").reverse().toArray();
    return rows.map(({ bytes: _bytes, ...rest }) => rest);
  }
  getRun(runId: string) {
    return this.db.runs.get(runId);
  }
  async putRun(record: RunRecord) {
    await this.db.runs.put(record);
  }
  async deleteRun(runId: string) {
    await this.db.transaction("rw", [this.db.runs, this.db.checkpoints, this.db.journal], async () => {
      await this.db.runs.delete(runId);
      await this.db.checkpoints.where("runId").equals(runId).delete();
      await this.db.journal.where("runId").equals(runId).delete();
    });
  }
  async putCheckpoint(runId: string, revision: number, bytes: Uint8Array) {
    await this.db.checkpoints.put({ runId, revision, bytes });
  }
  async listCheckpoints(runId: string) {
    const rows = await this.db.checkpoints.where("runId").equals(runId).toArray();
    return rows.map((r) => r.revision).sort((a, b) => a - b);
  }
  async appendJournal(runId: string, revision: number, command: string) {
    await this.db.journal.add({ runId, revision, command });
  }
  async getJournal(runId: string, revision: number) {
    const rows = await this.db.journal.where("[runId+revision]").equals([runId, revision]).sortBy("id");
    return rows.map((r) => r.command);
  }
  async clearJournal(runId: string) {
    await this.db.journal.where("runId").equals(runId).delete();
  }
  async getSetting<T>(key: string) {
    return (await this.db.settings.get(key))?.value as T | undefined;
  }
  async putSetting<T>(key: string, value: T) {
    await this.db.settings.put({ key, value });
  }
}

let shared: SaveStore | null = null;
export function saveStore(): SaveStore {
  shared ??= new IndexedDbStore();
  return shared;
}
