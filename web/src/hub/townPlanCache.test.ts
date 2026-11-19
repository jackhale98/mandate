import { dressHash, dressTown } from "./layoutV2/dress";
import { GEO_FIXTURES, type GeoFixture } from "./layoutV2/fixtures";
import { planHash, planTown, type TownPlan } from "./layoutV2/plan";
import { BASE_RECIPE } from "./layoutV2/registry";
import { runTownJob } from "./townJobs";
import { townSeed } from "./townLayout";
import { IdbPlanStore, TownPlanCache, planKey, type PlanRequest, type PlanStore, type Planner } from "./townPlanCache";
import { planInWorker, type TownJobRunner } from "./townWorkerClient";

vi.setConfig({ testTimeout: 60_000 });

const f = GEO_FIXTURES[0] as GeoFixture;
const g = GEO_FIXTURES[1] as GeoFixture;
const request = (x: GeoFixture, over: Partial<PlanRequest> = {}): PlanRequest => ({
  runSeed: x.seed,
  town: x.geo.municipality,
  contentHash: "content-a",
  geo: { ...x.geo, places: x.input.places.map((p) => ({ ...p })) },
  seed: townSeed(x.seed, x.input.municipality),
  recipe: BASE_RECIPE,
  ...over,
});

/** The worker's job, run in this thread (as the worker does), counting calls. */
function countingPlanner(): Planner & { calls: number } {
  const runner: TownJobRunner = { run: (job) => Promise.resolve(runTownJob({ ...job, id: 1 } as Parameters<typeof runTownJob>[0])) };
  const p = (async (geo, seed, recipe) => {
    p.calls++;
    return planInWorker(runner, geo, seed, recipe);
  }) as Planner & { calls: number };
  p.calls = 0;
  return p;
}

class MemoryStore implements PlanStore {
  rows = new Map<string, TownPlan>();
  // Structured clone, as IndexedDB and postMessage do.
  get(key: string) {
    const p = this.rows.get(key);
    return Promise.resolve(p ? structuredClone(p) : null);
  }
  put(key: string, plan: TownPlan) {
    this.rows.set(key, structuredClone(plan));
    return Promise.resolve();
  }
}

const flush = () => new Promise((r) => setTimeout(r, 0));

describe("the town worker's job", () => {
  it("plans exactly as planTown does, and the plan survives a structured clone (postMessage, IndexedDB)", () => {
    const r = request(f);
    const job = runTownJob({ id: 7, kind: "plan", geo: r.geo, seed: r.seed, recipe: r.recipe });
    expect(job.kind).toBe("plan");
    if (job.kind !== "plan") return;
    expect(job.id).toBe(7);
    const direct = planTown(r.geo, r.seed, r.recipe);
    expect(planHash(job.plan)).toBe(planHash(direct));
    const copy = structuredClone(job.plan);
    expect(planHash(copy)).toBe(planHash(direct));
    const state = { home: f.input.home, location: { type: "home" as const } };
    expect(dressHash(dressTown(copy, state))).toBe(dressHash(dressTown(direct, state)));
  });

  it("reports a failure instead of throwing", () => {
    const r = runTownJob({ id: 3, kind: "plan", geo: null as never, seed: 1, recipe: BASE_RECIPE });
    expect(r).toMatchObject({ id: 3, kind: "error" });
  });
});

describe("the plan cache", () => {
  it("plans once, then serves the plan from memory", async () => {
    const planner = countingPlanner();
    const cache = new TownPlanCache(planner, null, "gen-1");
    const a = await cache.get(request(f));
    expect(a.source).toBe("planned");
    expect(a.planMs).toBeGreaterThan(0);
    const b = await cache.get(request(f));
    expect(b.source).toBe("memory");
    expect(b.plan).toBe(a.plan);
    expect(planner.calls).toBe(1);
  });

  it("asks for a plan once when two screens want it at the same time", async () => {
    const planner = countingPlanner();
    const cache = new TownPlanCache(planner, null, "gen-1");
    const [a, b] = await Promise.all([cache.get(request(f)), cache.get(request(f))]);
    expect(a.plan).toBe(b.plan);
    expect(planner.calls).toBe(1);
  });

  it("after a reload (memory gone) serves the stored plan, identical to a new one", async () => {
    const planner = countingPlanner();
    const store = new MemoryStore();
    const first = new TownPlanCache(planner, store, "gen-1");
    const a = await first.get(request(f));
    await flush();
    expect(store.rows.size).toBe(1);
    const second = new TownPlanCache(planner, store, "gen-1");
    const b = await second.get(request(f));
    expect(b.source).toBe("stored");
    expect(planHash(b.plan)).toBe(planHash(a.plan));
    expect(planner.calls).toBe(1);
  });

  it("keys plans by everything that makes them: run, town, content, generator, geography and places", () => {
    const base = request(f);
    const k = planKey(base, "gen-1");
    expect(planKey(request(f), "gen-1")).toBe(k);
    expect(planKey({ ...base, runSeed: "other" }, "gen-1")).not.toBe(k);
    expect(planKey({ ...base, town: base.town + 1 }, "gen-1")).not.toBe(k);
    expect(planKey({ ...base, contentHash: "content-b" }, "gen-1")).not.toBe(k);
    expect(planKey(base, "gen-2")).not.toBe(k);
    expect(planKey({ ...base, seed: base.seed + 1 }, "gen-1")).not.toBe(k);
    expect(planKey({ ...base, geo: { ...base.geo, places: base.geo.places.slice(1) } }, "gen-1")).not.toBe(k);
    expect(planKey(request(g), "gen-1")).not.toBe(k);
  });

  it("falls back to planning when storage fails, and still answers", async () => {
    const planner = countingPlanner();
    const broken: PlanStore = { get: () => Promise.reject(new Error("blocked")), put: () => Promise.reject(new Error("full")) };
    const cache = new TownPlanCache(planner, broken, "gen-1");
    const a = await cache.get(request(f));
    expect(a.source).toBe("planned");
    await flush();
    const again = new TownPlanCache(planner, broken, "gen-1");
    expect((await again.get(request(f))).source).toBe("planned");
    expect(planner.calls).toBe(2);
  });

  it("ignores a stored plan that is not one (an old or damaged row)", async () => {
    const planner = countingPlanner();
    const store = new MemoryStore();
    const key = planKey(request(f), "gen-1");
    store.rows.set(key, { version: 1 } as unknown as TownPlan);
    const cache = new TownPlanCache(planner, store, "gen-1");
    expect((await cache.get(request(f))).source).toBe("planned");
  });

  it("forgets a failed plan, so the next ask tries again", async () => {
    let fail = true;
    const planner: Planner = async (geo, seed, recipe) => {
      if (fail) throw new Error("worker gone");
      return { plan: planTown(geo, seed, recipe), ms: 1 };
    };
    const cache = new TownPlanCache(planner, null, "gen-1");
    await expect(cache.get(request(f))).rejects.toThrow("worker gone");
    fail = false;
    expect((await cache.get(request(f))).source).toBe("planned");
  });

  it("keeps a few plans in memory, the least recently used go first", async () => {
    const planner = countingPlanner();
    const cache = new TownPlanCache(planner, null, "gen-1", { memory: 1 });
    await cache.get(request(f));
    await cache.get(request(g));
    expect((await cache.get(request(f))).source).toBe("planned");
    expect(planner.calls).toBe(3);
  });
});

describe("plans in IndexedDB", () => {
  it("stores and reads a plan back intact, keeping only the newest", async () => {
    const store = new IdbPlanStore(`test-plans-${Math.random()}`, 2);
    const plan = planTown(request(f).geo, request(f).seed);
    await store.put("a", plan);
    const back = await store.get("a");
    expect(back && planHash(back)).toBe(planHash(plan));
    expect(ArrayBuffer.isView(back?.terrain)).toBe(true);
    await store.put("b", plan);
    await new Promise((r) => setTimeout(r, 2));
    await store.put("c", plan);
    expect(await store.get("a")).toBeNull();
    expect(await store.get("c")).not.toBeNull();
    expect(await store.get("missing")).toBeNull();
  });
});
