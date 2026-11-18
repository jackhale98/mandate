// Reads the game's saved runs for the landing page's "Continue your run" (DEC-083).
// Read-only, without Dexie: the landing page stays a few kilobytes.

/** The fields of the game's run records read here (see src/persistence/saveStore.ts). */
export interface RunRow {
  status?: string;
  name?: string;
  updatedAt?: string;
  meta?: { turn?: number; date?: { year: number; month: number; day: number } };
}

const MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

/**
 * The most recent active run in the game's database, or null. Never creates
 * or upgrades the database: if it does not exist yet, opening it is aborted.
 */
export async function latestRun(dbName = "mandate"): Promise<RunRow | null> {
  if (!("indexedDB" in window)) return null;
  if (typeof indexedDB.databases === "function") {
    const dbs = await indexedDB.databases();
    if (!dbs.some((d) => d.name === dbName)) return null;
  }
  const db = await new Promise<IDBDatabase | null>((resolve) => {
    const req = indexedDB.open(dbName);
    req.onupgradeneeded = () => req.transaction?.abort();
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => resolve(null);
    req.onblocked = () => resolve(null);
  });
  if (!db) return null;
  try {
    if (!db.objectStoreNames.contains("runs")) return null;
    const rows = await new Promise<RunRow[]>((resolve) => {
      const req = db.transaction("runs", "readonly").objectStore("runs").getAll();
      req.onsuccess = () => resolve(req.result as RunRow[]);
      req.onerror = () => resolve([]);
    });
    const active = rows.filter((r) => r.status === "active");
    active.sort((a, b) => (b.updatedAt ?? "").localeCompare(a.updatedAt ?? ""));
    return active[0] ?? null;
  } finally {
    db.close();
  }
}

export function describe(run: RunRow): string {
  const parts: string[] = [];
  if (run.name) parts.push(run.name);
  if (run.meta?.turn !== undefined) parts.push(`turn ${run.meta.turn}`);
  const d = run.meta?.date;
  if (d) parts.push(`${d.day} ${MONTHS[d.month - 1] ?? ""} ${d.year}`.trim());
  return parts.join(" · ");
}
