// The town's heavy, pure work, run off the main thread by ./town.worker
// (DEC-080: the town screen never freezes): planning a town with layout v2
// (DEC-090, ./layoutV2/plan) and building the pixel-art pack the walkable
// street draws with (DEC-084, tools/pixelkit). Both are deterministic data
// (typed arrays and plain objects), so they cross `postMessage` and go into
// IndexedDB by structured clone. The same function runs in the main thread
// when a worker is not available (./townWorkerClient).
import { buildPack, type ArtPack } from "../../../tools/pixelkit/art/mandate/pack";
import type { TownGeo } from "./layoutV2/geo";
import { planTown, type TownPlan } from "./layoutV2/plan";
import type { Recipe } from "./layoutV2/registry";

export type TownJob = { id: number; kind: "plan"; geo: TownGeo; seed: number; recipe: Recipe } | { id: number; kind: "pack" };

export type TownJobResult =
  | { id: number; kind: "plan"; plan: TownPlan; ms: number }
  | { id: number; kind: "pack"; pack: ArtPack; ms: number }
  | { id: number; kind: "error"; message: string };

const now = () => (typeof performance !== "undefined" ? performance.now() : Date.now());

export function runTownJob(job: TownJob): TownJobResult {
  const t0 = now();
  try {
    if (job.kind === "plan") {
      const plan = planTown(job.geo, job.seed, job.recipe);
      return { id: job.id, kind: "plan", plan, ms: now() - t0 };
    }
    const pack = buildPack();
    return { id: job.id, kind: "pack", pack, ms: now() - t0 };
  } catch (e) {
    return { id: job.id, kind: "error", message: e instanceof Error ? e.message : String(e) };
  }
}
