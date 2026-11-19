// The main thread's side of the town worker (./town.worker, ./townJobs).
// One worker, started on first use. If module workers are not available, or
// the worker fails to start, the jobs run here instead (the town still
// opens, it just costs the main thread the time the worker would have).
import type { ArtPack } from "../../../tools/pixelkit/art/mandate/pack";
import type { TownGeo } from "./layoutV2/geo";
import type { TownPlan } from "./layoutV2/plan";
import type { Recipe } from "./layoutV2/registry";
import type { TownJob, TownJobResult } from "./townJobs";

type Job = TownJob extends infer J ? (J extends { id: number } ? Omit<J, "id"> : never) : never;

/** Where the jobs run (tests pass their own). */
export interface TownJobRunner {
  run(job: Job): Promise<TownJobResult>;
}

/**
 * What made the plans (the plan cache's generator id): this module's URL. In
 * a production build it is a chunk whose file name carries a hash of its
 * code, which includes the worker's hashed URL, so any change to the
 * generator gives a new id; in development it has no hash (./townPlanCache
 * keeps no persistent cache there).
 */
export const TOWN_GENERATOR = import.meta.url;

class WorkerRunner implements TownJobRunner {
  private worker: Worker | null = null;
  private failed = false;
  private next = 1;
  private readonly pending = new Map<number, (r: TownJobResult) => void>();

  private start(): Worker | null {
    if (this.worker || this.failed) return this.worker;
    try {
      const w = new Worker(new URL("./town.worker.ts", import.meta.url), { type: "module" });
      w.onmessage = (ev: MessageEvent<TownJobResult>) => {
        const done = this.pending.get(ev.data.id);
        this.pending.delete(ev.data.id);
        done?.(ev.data);
      };
      w.onerror = (ev) => {
        // The worker could not load or crashed: finish what was asked here, and from now on.
        ev.preventDefault();
        this.failed = true;
        this.worker = null;
        w.terminate();
        const left = [...this.pending.entries()];
        this.pending.clear();
        for (const [id, done] of left) done({ id, kind: "error", message: "town worker failed" });
      };
      this.worker = w;
    } catch {
      this.failed = true;
    }
    return this.worker;
  }

  async run(job: Job): Promise<TownJobResult> {
    const w = typeof Worker === "function" ? this.start() : null;
    if (w) {
      const id = this.next++;
      const r = await new Promise<TownJobResult>((resolve) => {
        this.pending.set(id, resolve);
        w.postMessage({ ...job, id } as TownJob);
      });
      if (r.kind !== "error" || !this.failed) return r;
    }
    const { runTownJob } = await import("./townJobs");
    return runTownJob({ ...job, id: 0 } as TownJob);
  }
}

let shared: TownJobRunner | null = null;
export function townJobs(): TownJobRunner {
  shared ??= new WorkerRunner();
  return shared;
}

/** Plan a town off the main thread. */
export async function planInWorker(runner: TownJobRunner, geo: TownGeo, seed: number, recipe: Recipe): Promise<{ plan: TownPlan; ms: number }> {
  const r = await runner.run({ kind: "plan", geo, seed, recipe });
  if (r.kind === "error") throw new Error(r.message);
  if (r.kind !== "plan") throw new Error(`town worker: expected a plan, got ${r.kind}`);
  return { plan: r.plan, ms: r.ms };
}

let pack: Promise<ArtPack> | null = null;
/** The pixel-art pack, built once per session off the main thread. */
export function artPack(runner: TownJobRunner = townJobs()): Promise<ArtPack> {
  pack ??= runner.run({ kind: "pack" }).then((r) => {
    if (r.kind !== "pack") {
      pack = null;
      throw new Error(r.kind === "error" ? r.message : `town worker: expected the pack, got ${r.kind}`);
    }
    return r.pack;
  });
  return pack;
}
