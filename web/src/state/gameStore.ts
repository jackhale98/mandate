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
import type { NotebookView } from "../engine/protocol/NotebookView";
import { freshKeys } from "../components/notebook/notebookModel";
import type { TalkView } from "../engine/protocol/TalkView";
import type { ExchangeView } from "../engine/protocol/ExchangeView";
import type { AboutRefDto } from "../engine/protocol/AboutRefDto";
import type { CanvassView } from "../engine/protocol/CanvassView";
import type { PersonRefView } from "../engine/protocol/PersonRefView";
import type { TalkTargetDto } from "../engine/protocol/TalkTargetDto";
import type { TermReviewView } from "../engine/protocol/TermReviewView";
import type { SkipProgressView } from "../engine/protocol/SkipProgressView";
import type { SkipSummaryView } from "../engine/protocol/SkipSummaryView";
import type { SkipTargetDto } from "../engine/protocol/SkipTargetDto";
import type { LegacyView } from "../engine/protocol/LegacyView";
import type { AsksView } from "../engine/protocol/AsksView";
import type { PromiseMenuView } from "../engine/protocol/PromiseMenuView";
import { findRoute, sameLocation } from "./routing";
import type { RunModeDto } from "../engine/protocol/RunModeDto";
import type { WorldSizeDto } from "../engine/protocol/WorldSizeDto";
import { EMPTY_TRANSLATOR, Translator } from "../i18n/fluent";
import { saveStore } from "../persistence/indexedDbStore";
import { deviceId, randomHex, type RunSummary } from "../persistence/saveStore";
import { noteCommand, recordError } from "./reportStore";
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

/**
 * Inside a place (DEC-088): the interior the player walked into from the
 * street. The engine has the player at the place (as after `visit`); the room
 * is presentation, composed on the client from the recipe and a keyed seed.
 */
export interface TalkAsked {
  text: string;
  /** The exchange id the question was asked after (null: the first). */
  after: number | null;
  /** The question has been put up (after the person's opener). */
  qShown: boolean;
}

export interface InteriorState {
  location: LocationDto;
  place: number;
  /** The recipe's id (the room itself comes with the place view, `interiorPlace.interior`). */
  recipe: string;
  /** The town layout's seed (the room's seed is keyed by it and the place). */
  townSeed: number;
  /** Counts entries (a new one remounts the room). */
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
  /** Term in review (DEC-012), when open: directives and the points to skip to. */
  termReview: TermReviewView | null;
  /** A skip under way: the engine's progress after its last chunk. */
  skipProgress: SkipProgressView | null;
  /** The player asked to stop the skip (taken at the next safe point). */
  skipCancel: boolean;
  /** What happened while the player was away (the interrupt or homecoming summary), when shown. */
  skipSummary: SkipSummaryView | null;
  /** The act's Legacy record and epilogue (DEC-006), when open. */
  legacy: LegacyView | null;
  /** Asks and promises on record (DEC-097 §2): loaded with the story so far (its promises). */
  asks: AsksView | null;
  /** The promise picker (campaigning), when open. */
  promiseMenu: PromiseMenuView | null;
  /** The notebook (DEC-091, K5), while its tab is open. */
  notebook: NotebookView | null;
  /**
   * Seen marks (K5): the entries that were new when the player opened the
   * notebook, kept marked "new" while they look. The engine forgets them
   * as new (`notebook_seen`), so the badge clears and stays clear.
   */
  notebookFresh: string[];
  /** The talk panel (DEC-093): whom the player is talking to, as the engine offers it. */
  talk: TalkView | null;
  /** Whom the talk panel is for: someone stored, or a passer-by here (met as the talk starts). */
  talkTo: TalkTargetDto | null;
  /** The panel is for a late-night conversation (the late slot). */
  talkLate: boolean;
  /**
   * The player's question in this conversation, shown over them once the person's opener is said. Kept here, not in the
   * dialogue's own state: meeting a passer-by rebuilds the dialogue while the engine answers, and the question must outlive it.
   */
  talkAsked: TalkAsked | null;
  setTalkAsked(update: TalkAsked | null | ((a: TalkAsked | null) => TalkAsked | null)): void;
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
  /** The interior the player is in (DEC-088), or null in the street. */
  interior: InteriorState | null;
  /** The place view for the interior (who is here, what it offers now), refreshed after every command. */
  interiorPlace: PlacePanelView | null;
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
  /** Runs saved before names were kept: read whose they are from each save, once, and keep it in the index. */
  fillRunNames(): Promise<void>;
  /** `localKnowledge`: "Start with local knowledge" (DEC-091, off by default). */
  newGame(opts: { seed?: string; size: WorldSizeDto; mode: RunModeDto; scenario: string | null; storyteller?: string | null; localKnowledge?: boolean }): Promise<void>;
  /** The engine's estimate for a draft with these parameters (the UI never computes it). */
  previewPolicy(template: string, params: ParamValue[]): Promise<PolicyPreviewView | null>;
  loadMap(): Promise<void>;
  loadOffice(): Promise<void>;
  loadCampaign(): Promise<void>;
  /** Refresh the office and campaign views together. */
  refreshDesk(): Promise<void>;
  closeElectionNight(): void;
  /** Run a player command, save the whole game (Ironman), and refresh views. */
  /** `deferToast`: keep the outcome's toast until `perform`'s walk is over (it shows it on arrival). */
  command(cmd: CommandDto, opts?: { deferToast?: boolean }): Promise<number | null>;
  /** Run a command where it is done: walks there first if needed (auto-routing, DEC-079). */
  perform(cmd: CommandDto): Promise<number | null>;
  /** Walk to a location (free) and open its panel. */
  visit(location: LocationDto): Promise<void>;
  /** Open a location's panel without moving. */
  openPanel(location: LocationDto | null): Promise<void>;
  /**
   * Walk in from the street (DEC-088): move there (free) and enter the place's
   * interior when its view carries one the client can draw (`canEnter` gives
   * its recipe id), else open its panel, as `visit` does. The Places tab and
   * the town map use `visit`.
   */
  walkIn(location: LocationDto, townSeed: number, canEnter: (interior: unknown) => string | null): Promise<void>;
  /** Out of the interior, back to the street at its door. */
  leaveInterior(): void;
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
  /** Open (or close) Term in review (DEC-012). */
  openTermReview(open: boolean): Promise<void>;
  /** Set one standing directive (the engine validates it). */
  setDirective(directive: string, option: string): Promise<void>;
  /** Skip to a point the engine offers: a turn at a time, showing progress, until done or cancelled; then the summary. */
  skip(target: SkipTargetDto): Promise<void>;
  /** Stop the skip at the next safe point (between turns). */
  cancelSkip(): void;
  closeSkipSummary(): void;
  /** Open (or close) the Legacy record and epilogue. */
  openLegacy(open: boolean): Promise<void>;
  /** Accept or decline an ask (free, from anywhere; DEC-097 §2). */
  respondAsk(ask: number, accept: boolean): Promise<void>;
  /** Casework on an accepted ask ("Sort it out", one slot): walks to where it is done first. */
  casework(ask: number): Promise<void>;
  /** Open (or close) the promise picker (campaigning only): the engine's menu. */
  openPromises(open: boolean): Promise<void>;
  /** Make a campaign promise from the menu; the picker follows what is left. */
  makePromise(template: string, answer: string | null): Promise<void>;
  /** Fetch the notebook (its tab is open). */
  loadNotebook(): Promise<void>;
  /**
   * The player is looking at the notebook: keep what is new marked for
   * this look, and tell the engine it has been seen (the badge clears).
   */
  seeNotebook(): Promise<void>;
  /** The notebook's tab closed: what was new has been seen. */
  leaveNotebook(): void;
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
  /** Where someone is now, as the engine's routines have it (the talk view's route: what "Go to them" uses); null if nowhere the player can walk to. */
  whereIs(to: TalkTargetDto): Promise<LocationDto | null>;
  skipWalk(): void;
  /** The street reports the player has reached the walk's destination. */
  arrived(): void;
  /** Whether the walkable street is shown (walks then end on arrival, not after `WALK_MS`). */
  streetWalks: boolean;
  setStreetWalks(on: boolean): void;
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
/** An outcome held back until the walk to where it happened is over (owner: no result before arriving). */
let heldToast: { key: string; args: Record<string, string> } | null = null;
/** How long an action's walk on the street may take before the result shows anyway (the walk is brisk, about 2.5 s; a stuck route never hides the result). */
export const STREET_WALK_MAX_MS = 4_000;

/** Going somewhere or opening a sheet closes the pocket, unless it is docked open beside the town. */
const pocketAfterMove = (s: { pocketOpen: boolean; pocketDocked: boolean }) => ({ pocketOpen: s.pocketOpen && s.pocketDocked && isWide() });
let passageCount = 0;
let interiorCount = 0;

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
    const office = get().office;
    await store.putRun({
      runId: run.runId,
      mode: run.mode,
      status,
      revision,
      meta,
      bytes,
      updatedAt: meta.saved_at,
      name: get().town?.municipality,
      ...(office ? { player: { name: office.player.name, roleKey: office.player.role_key, home: office.player.municipality } } : {}),
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
    termReview: null,
    skipProgress: null,
    skipCancel: false,
    skipSummary: null,
    legacy: null,
    asks: null,
    promiseMenu: null,
    notebook: null,
    notebookFresh: [],
    talk: null,
    talkTo: null,
    talkLate: false,
    talkAsked: null,
    setTalkAsked(update) {
      set((s) => ({ talkAsked: typeof update === "function" ? update(s.talkAsked) : update }));
    },
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
    interior: null,
    interiorPlace: null,
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

    async fillRunNames() {
      const missing = get().runs.filter((r) => !r.player);
      if (missing.length === 0) return;
      const store = saveStore();
      let changed = false;
      for (const r of missing) {
        try {
          const rec = await store.getRun(r.runId);
          if (!rec || rec.player) continue;
          const s = await engine().readSaveSummary(rec.bytes);
          // The run may have been saved or deleted while the worker read it.
          const now = await store.getRun(r.runId);
          if (!now || now.revision !== rec.revision) continue;
          await store.putRun({ ...now, player: { name: s.player_name, roleKey: s.role_key, home: s.home } });
          changed = true;
        } catch {
          // A corrupt or too-new save: its row stays as it is.
        }
      }
      if (changed) await get().refreshRuns();
    },

    async refreshRuns() {
      try {
        set({ runs: await saveStore().listRuns() });
      } catch (e) {
        set({ error: `Saves unavailable: ${errText(e)}` });
      }
    },

    async newGame({ seed, size, mode, scenario, storyteller, localKnowledge }) {
      set({ busy: true, error: null, map: null, progress: { stage: "progress-terrain", fraction: 0 } });
      const off = engine().onProgress((stage, fraction) => set({ progress: { stage, fraction } }));
      try {
        const body = await engine().request({ kind: "new_game", payload: { seed: seed ?? randomSeed(), size, scenario, storyteller: storyteller ?? null, local_knowledge: localKnowledge ?? false } });
        const overview = expectKind(body, "game_started");
        set({ overview, office: null, campaign: null, pocket: null, town: null, place: null, character: null, decision: null, story: null, termReview: null, skipSummary: null, legacy: null, asks: null, promiseMenu: null, notebook: null, notebookFresh: [], talk: null, exchange: null, openPlace: null, interior: null, interiorPlace: null, view: "town", townMapOpen: false, run: { runId: randomHex(16), mode, revision: 0 }, screen: "game" });
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
        // The notebook follows what is learned while its tab is open.
        const reading = get().pocketOpen && get().pocketTab === "notebook";
        const notebook = reading ? expectKind(await q({ type: "notebook" }), "notebook") : get().notebook;
        const talking = get().talkTo;
        // A passer-by met elsewhere (or gone with the period) is no longer there to talk to: the panel closes.
        const talk = talking ? await q({ type: "talk", to: talking, late_night: get().talkLate }).then((b) => expectKind(b, "talk"), () => null) : null;
        // Inside a place: its view follows (who is here, what it offers now); moved elsewhere, back to the street.
        const inside = get().interior;
        const pocketNow = expectKind(pocket, "pocket");
        const stays = inside !== null && sameLocation(pocketNow.location.location, inside.location);
        const interiorPlace = stays && inside ? expectKind(await q({ type: "place", location: inside.location }), "place") : null;
        set({
          interior: stays ? inside : null,
          interiorPlace,
          notebook,
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

    async command(cmd, opts) {
      const run = get().run;
      if (!run || get().busy) return null;
      set({ busy: true, error: null });
      noteCommand(cmd);
      try {
        const body = await engine().request({ kind: "command", payload: cmd });
        const applied = expectKind(body, "command_applied");
        // A full save after every action (about 90 ms). Nothing is ever replayed,
        // so an app update can never re-roll an outcome the player already saw.
        await persist();
        const r = applied.result;
        // Walking somewhere is not news; a decision's aftermath is on its sheet.
        const toast = r && r.outcome_key !== "outcome-moved" && cmd.type !== "answer_decision" && cmd.type !== "talk" ? { key: r.outcome_key, args: r.args } : null;
        if (opts?.deferToast) {
          heldToast = toast;
          set({ overview: applied.overview, toast: null });
        } else set({ overview: applied.overview, toast });
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
      // The walk ends when the street reports arrival (or after WALK_MS without a street); the
      // outcome is held until then, so the player never reads what happened before getting there.
      const walk = new Promise<void>((resolve) => {
        skipResolve = resolve;
        window.setTimeout(resolve, reducedMotion() ? 0 : get().streetWalks ? STREET_WALK_MAX_MS : WALK_MS);
      });
      heldToast = null;
      try {
        const [id] = await Promise.all([get().command({ type: "go", location: to.location, then: cmd }, { deferToast: true }), walk]);
        if (heldToast) set({ toast: heldToast });
        return id;
      } finally {
        heldToast = null;
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

    async walkIn(location, townSeed, canEnter) {
      if (location.type !== "place") return get().visit(location);
      const here = get().pocket?.location.location;
      set({ ...pocketAfterMove(get()), view: "town" });
      if (!here || !sameLocation(here, location)) await get().command({ type: "move_to", location });
      try {
        const v = expectKind(await engine().request({ kind: "query", payload: { type: "place", location } }), "place");
        const recipe = v.here ? canEnter(v.interior) : null;
        if (recipe) {
          set({ interior: { location, place: location.place, recipe, townSeed, id: ++interiorCount }, interiorPlace: v, openPlace: null, place: null });
          return;
        }
        set({ openPlace: location, place: v });
      } catch (e) {
        set({ error: errText(e) });
      }
    },

    leaveInterior() {
      set({ interior: null, interiorPlace: null });
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
        const asks = expectKind(await engine().request({ kind: "query", payload: { type: "asks" } }), "asks");
        set({ asks, story: expectKind(await engine().request({ kind: "query", payload: { type: "story" } }), "story"), ...pocketAfterMove(get()) });
      } catch (e) {
        set({ error: errText(e) });
      }
    },

    async openTermReview(open) {
      if (!open) {
        set({ termReview: null });
        return;
      }
      try {
        set({ termReview: expectKind(await engine().request({ kind: "query", payload: { type: "term_review" } }), "term_review"), ...pocketAfterMove(get()) });
      } catch (e) {
        set({ error: errText(e) });
      }
    },

    async setDirective(directive, option) {
      if (get().busy) return;
      const cmd: CommandDto = { type: "set_directives", choices: [{ directive, option }] };
      noteCommand(cmd);
      try {
        await engine().request({ kind: "command", payload: cmd });
        await persist();
        set({ termReview: expectKind(await engine().request({ kind: "query", payload: { type: "term_review" } }), "term_review") });
      } catch (e) {
        set({ error: errText(e) });
      }
    },

    async skip(target) {
      if (get().busy || !get().run) return;
      set({ busy: true, error: null, skipCancel: false, skipSummary: null, skipProgress: null });
      try {
        // A turn at a time: the engine keeps the skip between chunks, so the
        // player sees it move and can stop it between turns (a safe point).
        for (;;) {
          if (get().skipCancel) {
            const cancel: CommandDto = { type: "cancel_skip" };
            noteCommand(cancel);
            await engine().request({ kind: "command", payload: cancel });
            break;
          }
          const cmd: CommandDto = { type: "skip", target, limit: 1 };
          noteCommand(cmd);
          const applied = expectKind(await engine().request({ kind: "command", payload: cmd }), "command_applied");
          set({ overview: applied.overview, skipProgress: applied.skip ?? null });
          if (!applied.skip || applied.skip.done) break;
          // Let the screen paint and a Cancel tap land.
          await new Promise((r) => setTimeout(r, 0));
        }
        await persist();
        const summary = expectKind(await engine().request({ kind: "query", payload: { type: "skip_summary" } }), "skip_summary");
        set({ skipSummary: summary, termReview: null, skipProgress: null });
        await get().refreshDesk();
      } catch (e) {
        set({ error: errText(e), skipProgress: null });
      } finally {
        set({ busy: false, skipCancel: false });
      }
    },

    cancelSkip() {
      if (get().skipProgress || get().busy) set({ skipCancel: true });
    },

    closeSkipSummary() {
      set({ skipSummary: null });
    },

    async respondAsk(ask, accept) {
      await get().command({ type: "respond_ask", ask, accept });
    },

    async casework(ask) {
      await get().perform({ type: "act", action: { type: "casework", ask }, late_night: false });
    },

    async openPromises(open) {
      if (!open) {
        set({ promiseMenu: null });
        return;
      }
      try {
        set({ promiseMenu: expectKind(await engine().request({ kind: "query", payload: { type: "promise_menu" } }), "promise_menu"), ...pocketAfterMove(get()) });
      } catch (e) {
        set({ error: errText(e) });
      }
    },

    async makePromise(template, answer) {
      await get().command({ type: "promise", template, answer });
      if (!get().promiseMenu) return;
      try {
        set({ promiseMenu: expectKind(await engine().request({ kind: "query", payload: { type: "promise_menu" } }), "promise_menu") });
      } catch (e) {
        set({ error: errText(e) });
      }
    },

    async openLegacy(open) {
      if (!open) {
        set({ legacy: null });
        return;
      }
      try {
        set({ legacy: expectKind(await engine().request({ kind: "query", payload: { type: "legacy" } }), "legacy"), ...pocketAfterMove(get()) });
      } catch (e) {
        set({ error: errText(e) });
      }
    },

    async openTalk(to) {
      if (to === null) {
        set({ talk: null, talkTo: null, exchange: null, talkLate: false, talkAsked: null });
        return;
      }
      try {
        const talk = expectKind(await engine().request({ kind: "query", payload: { type: "talk", to, late_night: false } }), "talk");
        // The talk panel replaces the character sheet and the pocket (unless docked) rather than stacking on them.
        set({ talk, talkTo: to, exchange: null, talkLate: false, talkAsked: null, character: null, ...pocketAfterMove(get()) });
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

    async whereIs(to) {
      try {
        const v = expectKind(await engine().request({ kind: "query", payload: { type: "talk", to, late_night: false } }), "talk");
        return v.here ? null : (v.route?.to?.location ?? null);
      } catch {
        return null;
      }
    },

    streetWalks: false,
    setStreetWalks(on) {
      set({ streetWalks: on });
    },
    arrived() {
      skipResolve?.();
    },
    skipWalk() {
      skipResolve?.();
    },

    setPocketOpen(open) {
      set({ pocketOpen: open });
    },

    async loadNotebook() {
      try {
        set({ notebook: expectKind(await engine().request({ kind: "query", payload: { type: "notebook" } }), "notebook") });
      } catch (e) {
        set({ error: errText(e) });
      }
    },

    async seeNotebook() {
      if (!get().run || get().busy || (get().pocket?.notebook_new ?? 0) === 0) return;
      set({ busy: true });
      try {
        const q = (payload: QueryDto) => engine().request({ kind: "query", payload });
        // What is new now stays marked while the player looks.
        const before = expectKind(await q({ type: "notebook" }), "notebook");
        set({ notebook: before, notebookFresh: [...new Set([...get().notebookFresh, ...freshKeys(before)])] });
        noteCommand({ type: "notebook_seen" });
        expectKind(await engine().request({ kind: "command", payload: { type: "notebook_seen" } }), "command_applied");
        await persist();
        const [pocket, notebook] = await Promise.all([q({ type: "pocket" }), q({ type: "notebook" })]);
        set({ pocket: expectKind(pocket, "pocket"), notebook: expectKind(notebook, "notebook") });
      } catch (e) {
        set({ error: errText(e) });
      } finally {
        set({ busy: false });
      }
    },

    leaveNotebook() {
      set({ notebookFresh: [] });
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
      noteCommand({ type: "end_turn" });
      try {
        const body = await engine().request({ kind: "command", payload: { type: "end_turn" } });
        set({ overview: expectKind(body, "command_applied").overview, toast: null });
        await persist();
        const before = get().campaign?.last_election?.day_number ?? null;
        await get().refreshDesk();
        const after = get().campaign;
        const lastDay = after?.last_election?.day_number ?? null;
        // Election night whenever an election was held where the player
        // lives, on a ballot or not (play note 20).
        const night = lastDay !== null && lastDay !== before && (after?.results.length ?? 0) > 0;
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
        set({ screen: "title", overview: null, map: null, office: null, campaign: null, pocket: null, town: null, place: null, character: null, decision: null, story: null, termReview: null, skipSummary: null, legacy: null, asks: null, promiseMenu: null, talk: null, exchange: null, openPlace: null, interior: null, interiorPlace: null, pocketOpen: false, townMapOpen: false, view: "town", electionNight: false, passage: null, afterPassage: { electionNight: false }, run: null });
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
        set({ overview, map: null, office: null, campaign: null, pocket: null, town: null, place: null, character: null, decision: null, story: null, termReview: null, skipSummary: null, legacy: null, asks: null, promiseMenu: null, notebook: null, notebookFresh: [], talk: null, exchange: null, openPlace: null, interior: null, interiorPlace: null, view: "town", townMapOpen: false, run: { runId, mode: rec.mode, revision: rec.revision }, screen: "game" });
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

// Every error the app shows is kept for "Report a problem" (the last few).
useGame.subscribe((s, prev) => {
  if (s.error && s.error !== prev.error) recordError(s.error);
});
