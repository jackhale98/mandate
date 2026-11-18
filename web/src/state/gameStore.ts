// App state (Zustand). Simulation data shown here is a cache of the latest
// view model from the engine; the engine stays authoritative.
import { create } from "zustand";
import { engine, expectKind } from "../engine/client";
import type { OverviewView } from "../engine/protocol/OverviewView";
import type { MapView } from "../engine/protocol/MapView";
import type { ScenarioView } from "../engine/protocol/ScenarioView";
import type { OfficeView } from "../engine/protocol/OfficeView";
import type { CampaignView } from "../engine/protocol/CampaignView";
import type { CommandDto } from "../engine/protocol/CommandDto";
import type { PocketView } from "../engine/protocol/PocketView";
import type { TownPlacesView } from "../engine/protocol/TownPlacesView";
import type { PlacePanelView } from "../engine/protocol/PlacePanelView";
import type { LocationDto } from "../engine/protocol/LocationDto";
import type { LocationView } from "../engine/protocol/LocationView";
import type { QueryDto } from "../engine/protocol/QueryDto";
import { findRoute, sameLocation } from "./routing";
import type { RunModeDto } from "../engine/protocol/RunModeDto";
import type { WorldSizeDto } from "../engine/protocol/WorldSizeDto";
import { EMPTY_TRANSLATOR, Translator } from "../i18n/fluent";
import { saveStore } from "../persistence/indexedDbStore";
import { deviceId, randomHex, type RunSummary } from "../persistence/saveStore";

type Screen = "title" | "game";
/** Main view of the game screen: the town (places), or the pocket's country views. */
export type GameView = "town" | "map" | "overview";

/** Longest walk shown before an auto-routed action (DEC-079: short, skippable). */
export const WALK_MS = 600;

interface ActiveRun {
  runId: string;
  mode: RunModeDto;
  revision: number;
}

interface GameState {
  screen: Screen;
  booting: boolean;
  busy: boolean;
  error: string | null;
  tr: Translator;
  overview: OverviewView | null;
  map: MapView | null;
  office: OfficeView | null;
  campaign: CampaignView | null;
  pocket: PocketView | null;
  town: TownPlacesView | null;
  /** The open place panel (follows the player when an action routes them). */
  openPlace: LocationDto | null;
  place: PlacePanelView | null;
  pocketOpen: boolean;
  view: GameView;
  /** Auto-routing in progress: where the player is walking to. */
  walking: LocationView | null;
  /** Shown after an election the player took part in. */
  electionNight: boolean;
  toast: { key: string; args: Record<string, string> } | null;
  progress: { stage: string; fraction: number } | null;
  run: ActiveRun | null;
  runs: RunSummary[];
  /** Starting worlds from the content registry (title screen). */
  scenarios: ScenarioView[];
  boot(): Promise<void>;
  refreshRuns(): Promise<void>;
  newGame(opts: { seed?: string; size: WorldSizeDto; mode: RunModeDto; scenario: string | null }): Promise<void>;
  loadMap(): Promise<void>;
  loadOffice(): Promise<void>;
  loadCampaign(): Promise<void>;
  /** Refresh the office and campaign views together. */
  refreshDesk(): Promise<void>;
  closeElectionNight(): void;
  /** Run a player command, save the whole game (Ironman), and refresh views. */
  command(cmd: CommandDto): Promise<number | null>;
  /** Run a command where it is done: walks there first if needed (auto-routing, DEC-079). */
  perform(cmd: CommandDto): Promise<number | null>;
  /** Walk to a location (free) and open its panel. */
  visit(location: LocationDto): Promise<void>;
  /** Open a location's panel without moving. */
  openPanel(location: LocationDto | null): Promise<void>;
  skipWalk(): void;
  setPocketOpen(open: boolean): void;
  setView(view: GameView): void;
  dismissToast(): void;
  endTurn(): Promise<void>;
  saveAndQuit(): Promise<void>;
  continueRun(runId: string): Promise<void>;
  deleteRun(runId: string): Promise<void>;
  clearError(): void;
}

function randomSeed(): string {
  const a = new Uint32Array(2);
  crypto.getRandomValues(a);
  return ((BigInt(a[0] ?? 0) << 32n) | BigInt(a[1] ?? 0)).toString();
}

const errText = (e: unknown) => (e instanceof Error ? e.message : String(e));

const reducedMotion = () => typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches === true;

let skipResolve: (() => void) | null = null;

export const useGame = create<GameState>((set, get) => {
  /** Persist the running game as the run's next revision (Ironman: every turn). */
  async function persist(status: "active" | "ended" = "active") {
    const run = get().run;
    if (!run) return;
    const store = saveStore();
    const revision = run.revision + 1;
    const bytes = await engine().exportSave({
      run_id: run.runId,
      revision,
      parent_revision: run.revision === 0 ? null : run.revision,
      device_id: await deviceId(store),
      saved_at: new Date().toISOString(),
      run_mode: run.mode,
    });
    const meta = await engine().readSaveMeta(bytes);
    await store.putRun({
      runId: run.runId,
      mode: run.mode,
      status,
      revision,
      meta,
      bytes,
      updatedAt: meta.saved_at,
    });
    set({ run: { ...run, revision } });
  }

  return {
    screen: "title",
    booting: true,
    busy: false,
    error: null,
    tr: EMPTY_TRANSLATOR,
    overview: null,
    map: null,
    office: null,
    campaign: null,
    pocket: null,
    town: null,
    openPlace: null,
    place: null,
    pocketOpen: false,
    view: "town",
    walking: null,
    electionNight: false,
    toast: null,
    progress: null,
    run: null,
    runs: [],
    scenarios: [],

    async boot() {
      try {
        await engine().ready;
        const locale = expectKind(await engine().request({ kind: "query", payload: { type: "locale", lang: "en" } }), "locale");
        set({ tr: new Translator(locale.lang, locale.ftl) });
        const list = expectKind(await engine().request({ kind: "query", payload: { type: "scenarios" } }), "scenarios");
        set({ scenarios: list.scenarios });
        await get().refreshRuns();
      } catch (e) {
        set({ error: errText(e) });
      } finally {
        set({ booting: false });
      }
    },

    async refreshRuns() {
      try {
        set({ runs: await saveStore().listRuns() });
      } catch (e) {
        set({ error: `Saves unavailable: ${errText(e)}` });
      }
    },

    async newGame({ seed, size, mode, scenario }) {
      set({ busy: true, error: null, map: null, progress: { stage: "progress-terrain", fraction: 0 } });
      const off = engine().onProgress((stage, fraction) => set({ progress: { stage, fraction } }));
      try {
        const body = await engine().request({ kind: "new_game", payload: { seed: seed ?? randomSeed(), size, scenario } });
        const overview = expectKind(body, "game_started");
        set({ overview, office: null, campaign: null, pocket: null, town: null, place: null, openPlace: null, view: "town", run: { runId: randomHex(16), mode, revision: 0 }, screen: "game" });
        await persist();
        await get().refreshDesk();
      } catch (e) {
        set({ error: errText(e) });
      } finally {
        off();
        set({ busy: false, progress: null });
      }
    },

    async loadCampaign() {
      try {
        set({ campaign: expectKind(await engine().request({ kind: "query", payload: { type: "campaign" } }), "campaign") });
      } catch (e) {
        set({ error: errText(e) });
      }
    },

    async refreshDesk() {
      try {
        const q = (payload: QueryDto) => engine().request({ kind: "query", payload });
        const [office, campaign, pocket, town] = await Promise.all([q({ type: "office" }), q({ type: "campaign" }), q({ type: "pocket" }), q({ type: "town" })]);
        const open = get().openPlace;
        const place = open ? expectKind(await q({ type: "place", location: open }), "place") : null;
        set({
          office: expectKind(office, "office"),
          campaign: expectKind(campaign, "campaign"),
          pocket: expectKind(pocket, "pocket"),
          town: expectKind(town, "town"),
          place,
        });
      } catch (e) {
        set({ error: errText(e) });
      }
    },

    closeElectionNight() {
      set({ electionNight: false });
    },

    async loadOffice() {
      try {
        set({ office: expectKind(await engine().request({ kind: "query", payload: { type: "office" } }), "office") });
      } catch (e) {
        set({ error: errText(e) });
      }
    },

    async command(cmd) {
      const run = get().run;
      if (!run || get().busy) return null;
      set({ busy: true, error: null });
      try {
        const body = await engine().request({ kind: "command", payload: cmd });
        const applied = expectKind(body, "command_applied");
        // A full save after every action (about 90 ms). Nothing is ever replayed,
        // so an app update can never re-roll an outcome the player already saw.
        await persist();
        const r = applied.result;
        set({
          overview: applied.overview,
          // Walking somewhere is not news.
          toast: r && r.outcome_key !== "outcome-moved" ? { key: r.outcome_key, args: r.args } : null,
        });
        await get().refreshDesk();
        return applied.result?.created ?? null;
      } catch (e) {
        set({ error: errText(e) });
        return null;
      } finally {
        set({ busy: false });
      }
    },

    async perform(cmd) {
      const route = findRoute(get().pocket, get().office, cmd);
      if (!route?.to || route.here) return get().command(cmd);
      const to = route.to;
      // A place panel that is open follows the player to where the action is done.
      if (get().openPlace) set({ openPlace: to.location });
      set({ walking: to, pocketOpen: false });
      const walk = new Promise<void>((resolve) => {
        skipResolve = resolve;
        window.setTimeout(resolve, reducedMotion() ? 0 : WALK_MS);
      });
      try {
        const [id] = await Promise.all([get().command({ type: "go", location: to.location, then: cmd }), walk]);
        return id;
      } finally {
        skipResolve = null;
        set({ walking: null });
      }
    },

    async visit(location) {
      const here = get().pocket?.location.location;
      set({ openPlace: location, pocketOpen: false, view: "town" });
      if (here && sameLocation(here, location)) {
        await get().refreshDesk();
        return;
      }
      await get().command({ type: "move_to", location });
    },

    async openPanel(location) {
      set({ openPlace: location, place: null });
      if (location) await get().refreshDesk();
    },

    skipWalk() {
      skipResolve?.();
    },

    setPocketOpen(open) {
      set({ pocketOpen: open });
    },

    setView(view) {
      set({ view, pocketOpen: false });
    },

    dismissToast() {
      set({ toast: null });
    },

    async loadMap() {
      try {
        set({ map: expectKind(await engine().request({ kind: "query", payload: { type: "map" } }), "map") });
      } catch (e) {
        set({ error: errText(e) });
      }
    },

    async endTurn() {
      if (get().busy) return;
      set({ busy: true, error: null });
      try {
        const body = await engine().request({ kind: "command", payload: { type: "end_turn" } });
        set({ overview: expectKind(body, "command_applied").overview, toast: null });
        await persist();
        const before = get().campaign?.last_election?.day_number ?? null;
        await get().refreshDesk();
        const after = get().campaign;
        const lastDay = after?.last_election?.day_number ?? null;
        if (lastDay !== null && lastDay !== before && after?.results.some((r) => r.player_ran)) {
          set({ electionNight: true });
        }
      } catch (e) {
        set({ error: errText(e) });
      } finally {
        set({ busy: false });
      }
    },

    async saveAndQuit() {
      set({ busy: true });
      try {
        await persist();
        set({ screen: "title", overview: null, map: null, office: null, campaign: null, pocket: null, town: null, place: null, openPlace: null, pocketOpen: false, view: "town", electionNight: false, run: null });
        await get().refreshRuns();
      } catch (e) {
        set({ error: errText(e) });
      } finally {
        set({ busy: false });
      }
    },

    async continueRun(runId) {
      set({ busy: true, error: null });
      try {
        const rec = await saveStore().getRun(runId);
        if (!rec) throw new Error("Save not found");
        const overview = await engine().importSave(rec.bytes);
        set({ overview, map: null, office: null, campaign: null, pocket: null, town: null, place: null, openPlace: null, view: "town", run: { runId, mode: rec.mode, revision: rec.revision }, screen: "game" });
        await get().refreshDesk();
      } catch (e) {
        set({ error: errText(e) });
      } finally {
        set({ busy: false });
      }
    },

    async deleteRun(runId) {
      await saveStore().deleteRun(runId);
      await get().refreshRuns();
    },

    clearError() {
      set({ error: null });
    },
  };
});
