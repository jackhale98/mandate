// App state (Zustand). Simulation data shown here is a cache of the latest
// view model from the engine; the engine stays authoritative.
import { create } from "zustand";
import { engine, expectKind } from "../engine/client";
import type { OverviewView } from "../engine/protocol/OverviewView";
import type { MapView } from "../engine/protocol/MapView";
import type { PresetDto } from "../engine/protocol/PresetDto";
import type { RunModeDto } from "../engine/protocol/RunModeDto";
import type { WorldSizeDto } from "../engine/protocol/WorldSizeDto";
import { EMPTY_TRANSLATOR, Translator } from "../i18n/fluent";
import { saveStore } from "../persistence/indexedDbStore";
import { deviceId, randomHex, type RunSummary } from "../persistence/saveStore";

type Screen = "title" | "game";

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
  progress: { stage: string; fraction: number } | null;
  run: ActiveRun | null;
  runs: RunSummary[];
  boot(): Promise<void>;
  refreshRuns(): Promise<void>;
  newGame(opts: { seed?: string; size: WorldSizeDto; mode: RunModeDto; preset: PresetDto | null }): Promise<void>;
  loadMap(): Promise<void>;
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
    progress: null,
    run: null,
    runs: [],

    async boot() {
      try {
        await engine().ready;
        const locale = expectKind(await engine().request({ kind: "query", payload: { type: "locale", lang: "en" } }), "locale");
        set({ tr: new Translator(locale.lang, locale.ftl) });
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

    async newGame({ seed, size, mode, preset }) {
      set({ busy: true, error: null, map: null, progress: { stage: "progress-terrain", fraction: 0 } });
      const off = engine().onProgress((stage, fraction) => set({ progress: { stage, fraction } }));
      try {
        const body = await engine().request({ kind: "new_game", payload: { seed: seed ?? randomSeed(), size, preset } });
        const overview = expectKind(body, "game_started");
        set({ overview, run: { runId: randomHex(16), mode, revision: 0 }, screen: "game" });
        await persist();
      } catch (e) {
        set({ error: errText(e) });
      } finally {
        off();
        set({ busy: false, progress: null });
      }
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
        set({ overview: expectKind(body, "command_applied").overview });
        await persist();
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
        set({ screen: "title", overview: null, map: null, run: null });
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
        set({ overview, map: null, run: { runId, mode: rec.mode, revision: rec.revision }, screen: "game" });
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
