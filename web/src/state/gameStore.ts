// App state (Zustand). Simulation data shown here is a cache of the latest
// view model from the engine; the engine stays authoritative.
import { create } from "zustand";
import { engine, expectKind } from "../engine/client";
import type { OverviewView } from "../engine/protocol/OverviewView";
import type { MapView } from "../engine/protocol/MapView";
import type { ScenarioView } from "../engine/protocol/ScenarioView";
import type { StorytellerView } from "../engine/protocol/StorytellerView";
import type { ParamValue } from "../engine/protocol/ParamValue";
import type { PolicyPreviewView } from "../engine/protocol/PolicyPreviewView";
import type { OfficeView } from "../engine/protocol/OfficeView";
import type { CampaignView } from "../engine/protocol/CampaignView";
import type { CommandDto } from "../engine/protocol/CommandDto";
import type { PocketView } from "../engine/protocol/PocketView";
import type { TownPlacesView } from "../engine/protocol/TownPlacesView";
import type { PlacePanelView } from "../engine/protocol/PlacePanelView";
import type { CharacterView } from "../engine/protocol/CharacterView";
import type { DecisionView } from "../engine/protocol/DecisionView";
import type { StoryView } from "../engine/protocol/StoryView";
import type { LocationDto } from "../engine/protocol/LocationDto";
import type { LocationView } from "../engine/protocol/LocationView";
import type { DateView } from "../engine/protocol/DateView";
import type { QueryDto } from "../engine/protocol/QueryDto";
import type { TalkView } from "../engine/protocol/TalkView";
import type { ExchangeView } from "../engine/protocol/ExchangeView";
import type { AboutRefDto } from "../engine/protocol/AboutRefDto";
import type { CanvassView } from "../engine/protocol/CanvassView";
import type { PersonRefView } from "../engine/protocol/PersonRefView";
import type { TalkTargetDto } from "../engine/protocol/TalkTargetDto";
import { findRoute, sameLocation } from "./routing";
import type { RunModeDto } from "../engine/protocol/RunModeDto";
import type { WorldSizeDto } from "../engine/protocol/WorldSizeDto";
import { EMPTY_TRANSLATOR, Translator } from "../i18n/fluent";
import { saveStore } from "../persistence/indexedDbStore";
import { deviceId, randomHex, type RunSummary } from "../persistence/saveStore";
import { isWide, readMapScope, readPocketDocked, readPocketTab, writeMapScope, writePocketDocked, writePocketTab, type MapScope, type PocketTab } from "./devicePrefs";

export type { MapScope, PocketTab } from "./devicePrefs";

type Screen = "title" | "game";
/** Main view of the game screen: the town (places), or the pocket's country views. */
export type GameView = "town" | "overview";

/** Longest walk shown before an auto-routed action (DEC-079: short, skippable). */
export const WALK_MS = 600;

/**
 * The night between two turns, shown when the player ends one (owner,
 * 2026-11-19): the light goes to night, a card shows the new date, dawn
 * brings the new turn's morning. The engine works out the turn meanwhile;
 * whatever opens after a turn (election night) waits for the passage.
 */
export interface Passage {
  /** dusk: the light going to night; night: the date card, waiting for the engine if need be; dawn: into the morning. */
  phase: "dusk" | "night" | "dawn";
  from: DateView;
  /** The new turn's date, once the engine has answered. */
  to: DateView | null;
  /** Days the turn spans (`OverviewView.turn_days`). */
  days: number;
  /** Counts passages (a new one restarts the timeline). */
  id: number;
}

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
  /** The open character sheet (DEC-031), over the place panel. */
  character: CharacterView | null;
  /** The open decision sheet (DEC-077 §4): pending, or its aftermath once answered. */
  decision: DecisionView | null;
  /** The story so far (DEC-033), when open. */
  story: StoryView | null;
  /** The talk panel (DEC-093): whom the player is talking to, as the engine offers it. */
  talk: TalkView | null;
  /** Whom the talk panel is for: someone stored, or a passer-by here (met as the talk starts). */
  talkTo: TalkTargetDto | null;
  /** The panel is for a late-night conversation (the late slot). */
  talkLate: boolean;
  /** The latest exchange in the talk panel (what was said, what was learned). */
  exchange: ExchangeView | null;
  pocketOpen: boolean;
  /** The pocket's open tab (DEC-088 main screen), remembered per device. */
  pocketTab: PocketTab;
  /** Wide screens: the pocket stays open beside the town (remembered per device). */
  pocketDocked: boolean;
  /** Bumped to move the keyboard into the pocket's open tab (the "Skip to Places" link). */
  pocketFocus: number;
  /** The map overlay over the street, full screen: the town map (DEC-086) or the country's. */
  townMapOpen: boolean;
  /** Which map the overlay shows, remembered per device. */
  mapScope: MapScope;
  view: GameView;
  /** Auto-routing in progress: where the player is walking to. */
  walking: LocationView | null;
  /** Shown after an election the player took part in. */
  electionNight: boolean;
  /** The night between turns, while it is shown. */
  passage: Passage | null;
  /** Opens when the passage ends (it never covers the passage). */
  afterPassage: { electionNight: boolean };
  toast: { key: string; args: Record<string, string> } | null;
  progress: { stage: string; fraction: number } | null;
  run: ActiveRun | null;
  runs: RunSummary[];
  /** Starting worlds from the content registry (title screen). */
  scenarios: ScenarioView[];
  /** Storyteller profiles (DEC-033), chosen at new game. */
  storytellers: StorytellerView[];
  boot(): Promise<void>;
  refreshRuns(): Promise<void>;
  newGame(opts: { seed?: string; size: WorldSizeDto; mode: RunModeDto; scenario: string | null; storyteller?: string | null }): Promise<void>;
  /** The engine's estimate for a draft with these parameters (the UI never computes it). */
  previewPolicy(template: string, params: ParamValue[]): Promise<PolicyPreviewView | null>;
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
  /** Open (or close, with null) a character's sheet. */
  openCharacter(id: number | null): Promise<void>;
  /** Meet a passer-by where the player is, then open their sheet. */
  meet(slot: number): Promise<void>;
  /** Open (or close, with null) a decision's sheet. */
  openDecision(id: number | null): Promise<void>;
  /** Answer a decision; the sheet then shows its aftermath. */
  answer(id: number, option: string): Promise<void>;
  /** Open (or close) the story so far. */
  openStory(open: boolean): Promise<void>;
  /** Open the talk panel for someone (or close it, with null). */
  openTalk(to: TalkTargetDto | null): Promise<void>;
  /** Talk on a topic (half a slot): the engine composes the reply; the panel shows the exchange. */
  talkAbout(topic: string, about: AboutRefDto | null): Promise<void>;
  /** The talk panel for a late-night conversation, or not. */
  setTalkLate(late: boolean): Promise<void>;
  /** Go (free) to where the person in the talk panel is, keeping the panel open. */
  goToTalk(): Promise<void>;
  /** The worries the player can canvass on in a neighbourhood, with the engine's match (null if unavailable). */
  previewCanvass(neighborhood: number): Promise<CanvassView | null>;
  /** Who is at a location (the street's passers-by, to walk up to). */
  peopleAt(location: LocationDto): Promise<PersonRefView[]>;
  skipWalk(): void;
  setPocketOpen(open: boolean): void;
  /** Open the pocket on a tab; `focus` moves the keyboard into it. */
  openPocket(tab?: PocketTab, focus?: boolean): void;
  setPocketTab(tab: PocketTab): void;
  setPocketDocked(docked: boolean): void;
  setTownMapOpen(open: boolean): void;
  setMapScope(scope: MapScope): void;
  /** Open the map overlay (over the town, from any view) on a map. */
  openMap(scope: MapScope): void;
  setView(view: GameView): void;
  dismissToast(): void;
  /** End the turn: the engine runs it while the night passes (`passage`). During a passage, finish it first. */
  endTurn(): Promise<void>;
  /** The passage moves to its next phase (the passage card's timeline). */
  setPassagePhase(phase: Passage["phase"]): void;
  /** The passage is over (or skipped once the engine has answered): what waited for it opens. */
  finishPassage(): void;
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

/** Going somewhere or opening a sheet closes the pocket, unless it is docked open beside the town. */
const pocketAfterMove = (s: { pocketOpen: boolean; pocketDocked: boolean }) => ({ pocketOpen: s.pocketOpen && s.pocketDocked && isWide() });
let passageCount = 0;

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
      name: get().town?.municipality,
      storyteller: get().overview?.storyteller_name_key,
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
    character: null,
    decision: null,
    story: null,
    talk: null,
    talkTo: null,
    talkLate: false,
    exchange: null,
    pocketOpen: false,
    pocketTab: readPocketTab(),
    pocketDocked: readPocketDocked(),
    pocketFocus: 0,
    townMapOpen: false,
    mapScope: readMapScope(),
    view: "town",
    walking: null,
    electionNight: false,
    passage: null,
    afterPassage: { electionNight: false },
    toast: null,
    progress: null,
    run: null,
    runs: [],
    scenarios: [],
    storytellers: [],

    async boot() {
      try {
        await engine().ready;
        const locale = expectKind(await engine().request({ kind: "query", payload: { type: "locale", lang: "en" } }), "locale");
        set({ tr: new Translator(locale.lang, locale.ftl) });
        const list = expectKind(await engine().request({ kind: "query", payload: { type: "scenarios" } }), "scenarios");
        set({ scenarios: list.scenarios });
        const tellers = expectKind(await engine().request({ kind: "query", payload: { type: "storytellers" } }), "storytellers");
        set({ storytellers: tellers.storytellers });
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

    async newGame({ seed, size, mode, scenario, storyteller }) {
      set({ busy: true, error: null, map: null, progress: { stage: "progress-terrain", fraction: 0 } });
      const off = engine().onProgress((stage, fraction) => set({ progress: { stage, fraction } }));
      try {
        const body = await engine().request({ kind: "new_game", payload: { seed: seed ?? randomSeed(), size, scenario, storyteller: storyteller ?? null, local_knowledge: false } });
        const overview = expectKind(body, "game_started");
        set({ overview, office: null, campaign: null, pocket: null, town: null, place: null, character: null, decision: null, story: null, talk: null, exchange: null, openPlace: null, view: "town", townMapOpen: false, run: { runId: randomHex(16), mode, revision: 0 }, screen: "game" });
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
        const who = get().character;
        const character = who ? expectKind(await q({ type: "character", id: who.id }), "character") : null;
        const open_decision = get().decision;
        const decision = open_decision ? expectKind(await q({ type: "decision", id: open_decision.id }), "decision") : null;
        const talking = get().talkTo;
        // A passer-by met elsewhere (or gone with the period) is no longer there to talk to: the panel closes.
        const talk = talking ? await q({ type: "talk", to: talking, late_night: get().talkLate }).then((b) => expectKind(b, "talk"), () => null) : null;
        set({
          talkTo: talk ? talking : null,
          character,
          decision,
          talk,
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

    async previewPolicy(template, params) {
      try {
        return expectKind(await engine().request({ kind: "query", payload: { type: "policy_preview", template, params } }), "policy_preview");
      } catch {
        // A preview is a convenience: the catalogue keeps its default estimate.
        return null;
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
          // Walking somewhere is not news; a decision's aftermath is on its sheet.
          toast: r && r.outcome_key !== "outcome-moved" && cmd.type !== "answer_decision" && cmd.type !== "talk" ? { key: r.outcome_key, args: r.args } : null,
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
      set({ walking: to, ...pocketAfterMove(get()) });
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
      set({ ...pocketAfterMove(get()), view: "town" });
      if (!here || !sameLocation(here, location)) await get().command({ type: "move_to", location });
      await get().openPanel(location);
    },

    async openPanel(location) {
      if (!location) {
        set({ openPlace: null, place: null });
        return;
      }
      // Load first, then open: the sheet slides up at its full height instead of
      // growing once its contents arrive.
      try {
        const place = expectKind(await engine().request({ kind: "query", payload: { type: "place", location } }), "place");
        set({ openPlace: location, place });
      } catch (e) {
        set({ error: errText(e) });
      }
    },

    async openCharacter(id) {
      if (id === null) {
        set({ character: null });
        return;
      }
      try {
        set({ character: expectKind(await engine().request({ kind: "query", payload: { type: "character", id } }), "character") });
      } catch (e) {
        set({ error: errText(e) });
      }
    },

    async meet(slot) {
      const id = await get().command({ type: "meet", slot });
      if (id !== null) await get().openCharacter(id);
    },

    async openDecision(id) {
      if (id === null) {
        set({ decision: null });
        return;
      }
      try {
        // The decision sheet replaces the pocket rather than stacking on top of it.
        set({ decision: expectKind(await engine().request({ kind: "query", payload: { type: "decision", id } }), "decision"), ...pocketAfterMove(get()) });
      } catch (e) {
        set({ error: errText(e) });
      }
    },

    async answer(id, option) {
      await get().command({ type: "answer_decision", decision: id, option });
      // `command` refreshed the open sheet: it now shows the aftermath.
      if (get().decision?.id !== id) await get().openDecision(id);
    },

    async openStory(open) {
      if (!open) {
        set({ story: null });
        return;
      }
      try {
        set({ story: expectKind(await engine().request({ kind: "query", payload: { type: "story" } }), "story"), ...pocketAfterMove(get()) });
      } catch (e) {
        set({ error: errText(e) });
      }
    },

    async openTalk(to) {
      if (to === null) {
        set({ talk: null, talkTo: null, exchange: null, talkLate: false });
        return;
      }
      try {
        const talk = expectKind(await engine().request({ kind: "query", payload: { type: "talk", to, late_night: false } }), "talk");
        // The talk panel replaces the character sheet and the pocket (unless docked) rather than stacking on them.
        set({ talk, talkTo: to, exchange: null, talkLate: false, character: null, ...pocketAfterMove(get()) });
      } catch (e) {
        set({ error: errText(e) });
      }
    },

    async talkAbout(topic, about) {
      const to = get().talkTo;
      if (!to) return;
      const id = await get().command({ type: "talk", to, topic, about, late_night: get().talkLate });
      if (id === null) return;
      try {
        const exchange = expectKind(await engine().request({ kind: "query", payload: { type: "exchange", id } }), "exchange");
        const speaker = exchange.speaker.id;
        // A passer-by is met now: the panel goes on with them as someone the engine knows.
        if (to.type === "ambient" && speaker !== null) {
          const met: TalkTargetDto = { type: "person", person: speaker };
          const talk = expectKind(await engine().request({ kind: "query", payload: { type: "talk", to: met, late_night: get().talkLate } }), "talk");
          set({ exchange, talk, talkTo: met });
        } else set({ exchange });
      } catch (e) {
        set({ error: errText(e) });
      }
    },

    async setTalkLate(late) {
      const to = get().talkTo;
      if (!to) return;
      try {
        set({ talk: expectKind(await engine().request({ kind: "query", payload: { type: "talk", to, late_night: late } }), "talk"), talkLate: late });
      } catch (e) {
        set({ error: errText(e) });
      }
    },

    async goToTalk() {
      const to = get().talk?.route?.to;
      if (!to) return;
      // A place panel that is open follows the player.
      if (get().openPlace) set({ openPlace: to.location });
      await get().command({ type: "move_to", location: to.location });
    },

    async previewCanvass(neighborhood) {
      try {
        return expectKind(await engine().request({ kind: "query", payload: { type: "canvass", neighborhood } }), "canvass");
      } catch {
        // A preview is a convenience: the form still offers a general canvass.
        return null;
      }
    },

    async peopleAt(location) {
      try {
        return expectKind(await engine().request({ kind: "query", payload: { type: "place", location } }), "place").people;
      } catch {
        return [];
      }
    },

    skipWalk() {
      skipResolve?.();
    },

    setPocketOpen(open) {
      set({ pocketOpen: open });
    },

    openPocket(tab, focus = false) {
      if (tab) get().setPocketTab(tab);
      set((s) => ({ pocketOpen: true, pocketFocus: focus ? s.pocketFocus + 1 : s.pocketFocus }));
    },

    setPocketTab(tab) {
      writePocketTab(tab);
      set({ pocketTab: tab });
    },

    setPocketDocked(docked) {
      writePocketDocked(docked);
      set({ pocketDocked: docked });
    },

    setTownMapOpen(open) {
      set({ townMapOpen: open });
    },

    setMapScope(scope) {
      writeMapScope(scope);
      set({ mapScope: scope });
    },

    openMap(scope) {
      writeMapScope(scope);
      set({ view: "town", townMapOpen: true, mapScope: scope, ...pocketAfterMove(get()) });
    },

    setView(view) {
      set({ view, townMapOpen: false, ...pocketAfterMove(get()) });
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
      const running = get().passage;
      if (running) {
        // Ending the turn again while the night passes: let it pass now; if something waited for it, show that first.
        if (!running.to) return;
        const waited = get().afterPassage.electionNight;
        get().finishPassage();
        if (waited) return;
      }
      if (get().busy) return;
      const from = get().overview;
      set({
        busy: true,
        error: null,
        passage: from ? { phase: "dusk", from: from.date, to: null, days: from.turn_days, id: ++passageCount } : null,
      });
      try {
        const body = await engine().request({ kind: "command", payload: { type: "end_turn" } });
        set({ overview: expectKind(body, "command_applied").overview, toast: null });
        await persist();
        const before = get().campaign?.last_election?.day_number ?? null;
        await get().refreshDesk();
        const after = get().campaign;
        const lastDay = after?.last_election?.day_number ?? null;
        const night = lastDay !== null && lastDay !== before && (after?.results.some((r) => r.player_ran) ?? false);
        const p = get().passage;
        const date = get().overview?.date ?? null;
        if (p && date) set({ passage: { ...p, to: date }, afterPassage: { electionNight: night } });
        else if (night) set({ electionNight: true });
      } catch (e) {
        set({ error: errText(e), passage: null });
      } finally {
        set({ busy: false });
      }
    },

    setPassagePhase(phase) {
      const p = get().passage;
      if (p && p.phase !== phase) set({ passage: { ...p, phase } });
    },

    finishPassage() {
      const after = get().afterPassage;
      set({ passage: null, afterPassage: { electionNight: false }, ...(after.electionNight ? { electionNight: true } : {}) });
    },

    async saveAndQuit() {
      set({ busy: true });
      try {
        await persist();
        set({ screen: "title", overview: null, map: null, office: null, campaign: null, pocket: null, town: null, place: null, character: null, decision: null, story: null, talk: null, exchange: null, openPlace: null, pocketOpen: false, townMapOpen: false, view: "town", electionNight: false, passage: null, afterPassage: { electionNight: false }, run: null });
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
        set({ overview, map: null, office: null, campaign: null, pocket: null, town: null, place: null, character: null, decision: null, story: null, talk: null, exchange: null, openPlace: null, view: "town", townMapOpen: false, run: { runId, mode: rec.mode, revision: rec.revision }, screen: "game" });
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
