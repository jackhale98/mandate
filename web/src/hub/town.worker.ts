// The town worker (DEC-080: the town screen never freezes the main thread):
// plans towns and builds the art pack (./townJobs). A worker of its own, not
// the engine's: the engine worker owns the one authoritative simulation and
// answers requests strictly in arrival order (DEC-043), so a 100–600 ms plan
// there would hold up every command and query behind it; and this work is
// TypeScript presentation code that never touches the simulation.
import { runTownJob, type TownJob } from "./townJobs";

self.onmessage = (ev: MessageEvent<TownJob>) => {
  self.postMessage(runTownJob(ev.data));
};
