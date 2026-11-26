// The player's town as a dressed layout v2 map, for the town map and the
// walkable street (../hub/gameTown). Planning runs in the town worker and is
// cached, so the screen shows the loader until it is ready and never blocks.
import { useEffect, useEffectEvent, useMemo, useState } from "react";
import type { TownPlacesView } from "../engine/protocol/TownPlacesView";
import type { TownStateView } from "../engine/protocol/TownStateView";
import { dressStateOf, dressStats, dressedTown, planPlayerTown, townState } from "../hub/gameTown";
import type { TownPlan } from "../hub/layoutV2/plan";
import type { TownMap } from "../hub/townLayout";
import type { PlanSource } from "../hub/townPlanCache";
import { useGame } from "../state/gameStore";

export type TownLayout = { status: "loading" } | { status: "error"; message: string } | { status: "ready"; map: TownMap; plan: TownPlan };

/** What tests and measurements see (`window.__townPlan`). */
interface PlanProbe {
  /** Each plan the town screen asked for: where it came from and how long it took, ms. */
  loads: { source: PlanSource; ms: number; planMs: number | null; key: string }[];
  /** The last dress pass, ms. */
  dress: { ms: number };
  /** The town state the street is dressed for: its turn key and the faults it reports (B4: the cues come from these). */
  state?: { key: string; faults: TownStateView["faults"] };
}
declare global {
  interface Window {
    __townPlan?: PlanProbe;
  }
}
const probe = (): PlanProbe => (window.__townPlan ??= { loads: [], dress: dressStats });

/** The places the plan is made from (ids, kinds, names, neighbourhoods): not where the player is. */
function placesKey(town: TownPlacesView): string {
  return JSON.stringify(town.neighborhoods.map((h) => [h.id, h.kind, h.name, h.spots.map((s) => [s.location.location, s.location.kind_key, s.location.name])]));
}

export function useTownLayout(town: TownPlacesView): TownLayout {
  const seed = useGame((s) => s.overview?.seed ?? null);
  const municipality = useGame((s) => s.office?.player.municipality_id ?? null);
  const turn = useGame((s) => s.pocket?.turn ?? null);
  const places = placesKey(town);
  const [plan, setPlan] = useState<{ key: string; plan: TownPlan } | null>(null);
  const [state, setState] = useState<{ key: string; view: TownStateView } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const planFor = seed !== null && municipality !== null ? `${seed}|${municipality}|${places}` : null;
  const stateFor = municipality !== null ? `${municipality}|${turn}` : null;

  // The town as it is when its places change (later renders only move the player).
  const request = useEffectEvent((runSeed: string, id: number) => planPlayerTown(runSeed, id, town));
  useEffect(() => {
    if (planFor === null || seed === null || municipality === null) return;
    let live = true;
    request(seed, municipality)
      .then((r) => {
        probe().loads.push({ source: r.source, ms: r.ms, planMs: r.planMs, key: r.key });
        if (live) setPlan({ key: planFor, plan: r.plan });
      })
      .catch((e: unknown) => live && setError(e instanceof Error ? e.message : String(e)));
    return () => {
      live = false;
    };
  }, [planFor, seed, municipality]);

  useEffect(() => {
    if (stateFor === null || municipality === null) return;
    let live = true;
    townState(municipality)
      .then((view) => {
        if (!live) return;
        probe().state = { key: stateFor, faults: view.faults };
        setState({ key: stateFor, view });
      })
      .catch((e: unknown) => live && setError(e instanceof Error ? e.message : String(e)));
    return () => {
      live = false;
    };
  }, [stateFor, municipality]);

  // A new turn's state arrives a moment after the turn: keep the last one meanwhile (no loader flash).
  const ready = plan !== null && plan.key === planFor && state !== null;
  const home = town.home_neighborhood;
  const map = useMemo(() => {
    if (!ready || !plan || !state) return null;
    return dressedTown(plan.plan, dressStateOf(state.view, home));
  }, [ready, plan, state, home]);

  if (error) return { status: "error", message: error };
  if (!map || !plan) return { status: "loading" };
  return { status: "ready", map, plan: plan.plan };
}
