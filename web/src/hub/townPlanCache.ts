// Town plans, cached (DEC-085, DEC-090 D2): a plan is a pure function of the
// town's geography (with its places), its seed, the layout registries and
// the generator, so it is made once (in the town worker, ./townWorkerClient)
// and kept in memory and in IndexedDB, keyed by all of them: the run's seed,
// the town, the layout content hash, LAYOUT_VERSION, the generator build and
// a hash of the plan's inputs. Any change of input is a new key, never a
// stale plan. Storage is best effort: when IndexedDB is missing, blocked or
// full the plan is simply made again (a few hundred ms in the worker).
// Presentation only: nothing here reaches the engine or the save.
import Dexie, { type Table } from "dexie";
import type { TownGeo } from "./layoutV2/geo";
import type { TownPlan } from "./layoutV2/plan";
import type { Recipe } from "./layoutV2/registry";
import { LAYOUT_VERSION } from "./townLayout";

export interface PlanRequest {
  /** The run's world seed (`OverviewView.seed`). */
  runSeed: string;
  /** Municipality id. */
  town: number;
  /** `LayoutContentView.content_hash`: the registries the recipe comes from. */
  contentHash: string;
  geo: TownGeo;
  /** The town's layout seed (`townSeed`). */
  seed: number;
  recipe: Recipe;
}

export type PlanSource = "memory" | "stored" | "planned";

export interface PlanResult {
  plan: TownPlan;
  source: PlanSource;
  /** Time to get it, ms (for "planned": the planner's own time is `planMs`). */
  ms: number;
  planMs: number | null;
  key: string;
}

/** Where plans persist (IndexedDB in the app; tests pass a fake). */
export interface PlanStore {
  get(key: string): Promise<TownPlan | null>;
  put(key: string, plan: TownPlan): Promise<void>;
}

export type Planner = (geo: TownGeo, seed: number, recipe: Recipe) => Promise<{ plan: TownPlan; ms: number }>;

/** FNV-1a over a string, hex. */
function fnv(text: string): string {
  let h = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) {
    h ^= text.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return (h >>> 0).toString(16).padStart(8, "0");
}

/** The cache key of a plan request (exported for tests). */
export function planKey(req: Pick<PlanRequest, "runSeed" | "town" | "contentHash" | "geo" | "seed">, generator: string): string {
  const inputs = fnv(JSON.stringify([req.geo, req.seed]));
  return [`v${LAYOUT_VERSION}`, fnv(generator), req.contentHash || "-", req.runSeed, req.town, inputs].join("|");
}

const now = () => (typeof performance !== "undefined" ? performance.now() : Date.now());

export class TownPlanCache {
  private readonly memory = new Map<string, Promise<TownPlan>>();
  private readonly limit: number;

  constructor(
    private readonly planner: Planner,
    private readonly store: PlanStore | null,
    private readonly generator: string,
    opts: { memory?: number } = {},
  ) {
    this.limit = Math.max(1, opts.memory ?? 4);
  }

  async get(req: PlanRequest): Promise<PlanResult> {
    const t0 = now();
    const key = planKey(req, this.generator);
    const hit = this.memory.get(key);
    if (hit) {
      // Most recently used last.
      this.memory.delete(key);
      this.memory.set(key, hit);
      return { plan: await hit, source: "memory", ms: now() - t0, planMs: null, key };
    }
    let source: PlanSource = "stored";
    let planMs: number | null = null;
    const made = (async () => {
      const stored = await this.read(key);
      if (stored) return stored;
      source = "planned";
      const r = await this.planner(req.geo, req.seed, req.recipe);
      planMs = r.ms;
      void this.write(key, r.plan);
      return r.plan;
    })();
    this.memory.set(key, made);
    while (this.memory.size > this.limit) this.memory.delete(this.memory.keys().next().value as string);
    try {
      const plan = await made;
      return { plan, source, ms: now() - t0, planMs, key };
    } catch (e) {
      this.memory.delete(key);
      throw e;
    }
  }

  /** Forget the plans in memory (tests: a reload). */
  clearMemory() {
    this.memory.clear();
  }

  private async read(key: string): Promise<TownPlan | null> {
    if (!this.store) return null;
    try {
      const p = await this.store.get(key);
      return p && p.version === LAYOUT_VERSION && p.terrain instanceof Uint8Array ? p : null;
    } catch {
      return null;
    }
  }

  private async write(key: string, plan: TownPlan) {
    if (!this.store) return;
    try {
      await this.store.put(key, plan);
    } catch {
      // Full or blocked storage: made again next time.
    }
  }
}

// ---------------------------------------------------------------------------
// IndexedDB (a database of its own: plans are a disposable cache, never part of a save)

interface PlanRow {
  key: string;
  savedAt: number;
  plan: TownPlan;
}

class PlanDb extends Dexie {
  plans!: Table<PlanRow, string>;
  constructor(name: string) {
    super(name);
    this.version(1).stores({ plans: "key, savedAt" });
  }
}

/** Plans in IndexedDB, the newest `keep` kept. Every call may throw; the cache catches. */
export class IdbPlanStore implements PlanStore {
  private db: PlanDb | null = null;
  constructor(
    private readonly name = "mandate-town-plans",
    private readonly keep = 8,
  ) {}

  private open(): PlanDb {
    this.db ??= new PlanDb(this.name);
    return this.db;
  }

  async get(key: string): Promise<TownPlan | null> {
    const row = await this.open().plans.get(key);
    return row?.plan ?? null;
  }

  async put(key: string, plan: TownPlan): Promise<void> {
    const db = this.open();
    await db.plans.put({ key, savedAt: Date.now(), plan });
    const n = await db.plans.count();
    if (n > this.keep) {
      const old = await db.plans.orderBy("savedAt").limit(n - this.keep).primaryKeys();
      await db.plans.bulkDelete(old);
    }
  }
}
