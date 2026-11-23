// The walkable town in a DOM element (DEC-080, DEC-048, DEC-051): a canvas
// drawn by ./render/townRenderer at CSS resolution (upscaled pixelated by
// the browser), HTML labels over it, pointer input through ./camera (pan,
// pinch, wheel) and taps through ./townScene (a building walks to its door,
// ground walks there); optionally the keyboard (arrows and WASD walk through
// ./townScene's steering, Enter at a door arrives). The loop runs only while
// something moves (DEC-080 "Battery"): the player, the camera and gestures at
// full rate, frames where only townsfolk move at `ambientFps` (15); after
// `idleMs` (20 s) without input the townsfolk stop where they are and nothing
// is drawn until the next pointer, wheel, key or touch; it stops too when
// the page is hidden or the view is paused (a panel or sheet over it).
// Dialogue mode (DEC-093, `beginTalk`): the camera eases to frame the player
// and the person they talk to, facing each other, above the dialogue box;
// walking, street input, labels and the townsfolk pause until `endTalk`, and
// only the speaker moves (a small bob while a reply is revealed, at most
// TALK_FPS), so a conversation costs less than the idle street.
// Framework-free, so the prototype page and the app's React screen can both
// mount it.
import { HubCamera } from "./camera";
import { DayClock, townsfolkShare, wrapMinutes } from "./dayCycle";
import type { ArtProvider } from "./render/art";
import { PlaceholderArt } from "./render/placeholderArt";
import { TownRenderer, type WalkerView } from "./render/townRenderer";
import { ROOF_ROWS, buildingOfPlace, locationCell, type TownBuilding, type TownMap } from "./townLayout";
import type { TownLocation } from "./townInput";
import { TALK_REACH, TownScene, type StandingPerson, type Walker } from "./townScene";

/** Someone standing in the street the player can walk up to and talk to (DEC-093). */
export interface StreetPerson extends StandingPerson {
  /** Their name tag (a name, or a role for a stranger). */
  label: string;
  /** The tag's marker (default `talk`). */
  marker?: PersonMarker;
  /** The tag's accessible name, saying what the marker means (default: the label). */
  tagLabel?: string;
}

/**
 * A name tag's marker (DEC-093): `talk`, someone to talk to (a 💬 bubble with
 * dots); `news`, someone with something new for the player (a bubble with
 * "!": the engine's `has_news`). The shapes differ, not only the colours, and
 * the tag's accessible name says it in words (DEC-051).
 */
export type PersonMarker = "talk" | "news";

/** Pixel speech bubbles, 12 × 10 art px at one CSS px each (crisp, no smoothing). */
const BUBBLE = "M1 0h10v1h1v6h-1v1H5v1H4v1H3V8H1V7H0V1h1z";
const MARK_SVG: Record<PersonMarker, string> = {
  talk: `<svg viewBox="0 0 12 10" width="12" height="10" shape-rendering="crispEdges"><path class="hub-mark-bubble" d="${BUBBLE}"/><path class="hub-mark-glyph" d="M2 3h2v2H2zM5 3h2v2H5zM8 3h2v2H8z"/></svg>`,
  news: `<svg viewBox="0 0 12 10" width="12" height="10" shape-rendering="crispEdges"><path class="hub-mark-bubble" d="${BUBBLE}"/><path class="hub-mark-glyph" d="M5 1h2v4H5zM5 6h2v1H5z"/></svg>`,
};

/**
 * Fill a person's name tag (in the street and in rooms): the marker, then
 * the name. Its accessible name is `tagLabel` ("Vera Holt, can talk"). The
 * "!" bobs a pixel on alternate frames the view already draws (`bobTags`),
 * never on a timer of its own (DEC-080 battery). Returns whether it changed
 * (a room measures its tags once).
 */
export function fillPersonTag(el: HTMLElement, label: string, marker: PersonMarker = "talk", tagLabel: string = label): boolean {
  const sig = `${marker}|${label}|${tagLabel}`;
  if (el.dataset.tag === sig) return false;
  el.dataset.tag = sig;
  el.dataset.marker = marker;
  el.classList.toggle("hub-person-news", marker === "news");
  el.classList.toggle("hub-person-talk", marker === "talk");
  el.setAttribute("aria-label", tagLabel);
  el.title = tagLabel;
  const mark = document.createElement("span");
  mark.className = "hub-tag-mark";
  mark.setAttribute("aria-hidden", "true");
  mark.innerHTML = MARK_SVG[marker];
  const name = document.createElement("span");
  name.className = "hub-tag-name";
  name.textContent = label;
  el.replaceChildren(mark, name);
  return true;
}

/** The "!" markers' two-frame bob: up on odd half-seconds of `seconds` (the view's clock, read only on frames it draws anyway). */
export function bobTags(tags: Iterable<HTMLElement>, seconds: number, still: boolean) {
  const up = !still && Math.floor(seconds * 2) % 2 === 1;
  for (const el of tags) if (el.dataset.marker === "news") el.classList.toggle("hub-tag-bob", up);
}

export interface HubViewOptions {
  map: TownMap;
  art?: ArtProvider;
  /** Text of a building's label (a place's translated name), or null for none. */
  label?: (b: TownBuilding) => string | null;
  /** An SVG symbol id to show in a building's label (the provider's `icon`), or null. */
  labelIcon?: (b: TownBuilding) => string | null;
  /** The player reached a building's door (or open ground: null). */
  onArrive?: (b: TownBuilding | null) => void;
  /** Ambient townsfolk (default 14). */
  townsfolk?: number;
  /** Fewer townsfolk out at night (`townsfolkShare`; default true). */
  townsfolkByTime?: boolean;
  /** The canvas's accessible name (default: an English one naming the list of places). */
  canvasLabel?: string;
  /** Backing store: CSS px (default, DEC-080) or device px (for comparison). */
  resolution?: "css" | "device";
  reducedMotion?: boolean;
  /** The clock driving the light; default: stopped at noon. */
  clock?: DayClock;
  /** Frame cap while only townsfolk move (default 15). */
  ambientFps?: number;
  /** Idle sleep: with no input for this long (ms; default 20 000), townsfolk pause and drawing stops until the next input. */
  idleMs?: number;
  /** Called after every drawn frame (a HUD). */
  onFrame?: (v: HubView) => void;
  /**
   * Keyboard walking (desktop): the canvas takes focus (Tab, or a click) and,
   * while it has it, arrows and WASD walk the player, Shift runs, and Enter or
   * Space at a door arrives there (`onArrive`, as a tap would). Default false.
   */
  keyboard?: boolean;
  /** With `keyboard`: the hint shown at a door the player stands at (e.g. "Enter: the diner"), or null for none. */
  doorHint?: (b: TownBuilding) => string | null;
  /** With `keyboard`: the door hint shown changed (a building, or null when it hides). */
  onDoor?: (b: TownBuilding | null) => void;
  /**
   * The talk hook (DEC-093): the player walked up to someone standing in the
   * street (`setPeople`) by a tap on them or their name tag, or pressed Enter
   * or Space standing next to them.
   */
  onTalk?: (key: string) => void;
  /** With `keyboard`: the hint shown next to someone the player stands by (e.g. "Enter: talk to Dana Price"). */
  personHint?: (key: string) => string | null;
  /** The person the player stands next to changed (a key, or null). */
  onPersonNear?: (key: string | null) => void;
}

/** Keyboard directions: arrows by key, WASD by physical key (any layout). */
const KEY_DIRS: Readonly<Record<string, readonly [number, number]>> = {
  ArrowUp: [0, -1],
  ArrowDown: [0, 1],
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
  KeyW: [0, -1],
  KeyS: [0, 1],
  KeyA: [-1, 0],
  KeyD: [1, 0],
};

const editable = (t: EventTarget | null) => t instanceof HTMLElement && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName));

const TILE = 16;

/**
 * Dialogue mode: the zoom (CSS px per art px) a conversation frames at, at
 * least (more on a tall screen: about TALK_ROWS art px of street in its
 * height); frames per second while only the speaker moves.
 */
export const TALK_ZOOM = 3;
const TALK_ROWS = 260;
export const TALK_FPS = 8;
/** Where the speakers stand in the street left above the dialogue box (a share of its height from the top): room for the bubbles above them. */
const TALK_AT = 0.62;

/** Labels shown at once when zoomed out (TOWN_GENERATION.md §8: at most 12 on the overview). */
export const LABEL_CAP = 12;
/** Zoom (CSS px per art px) from which every building label may show; below it, landmarks and at most LABEL_CAP. Slightly under 2 so a device-pixel step just below 2 (e.g. 1.9 at DPR 2.625) still counts. */
export const LABEL_ZOOM = 1.75;

export interface LabelBox {
  index: number;
  /** Screen rect, CSS px. */
  x: number;
  y: number;
  w: number;
  h: number;
  /** Lower first. */
  priority: number;
}

/** Which labels to show: by priority, skipping any that would overlap one already shown (2 px apart), at most `cap`. */
export function chooseLabels(boxes: readonly LabelBox[], cap = Infinity): Set<number> {
  const order = [...boxes].sort((a, b) => a.priority - b.priority || a.index - b.index);
  const placed: LabelBox[] = [];
  const out = new Set<number>();
  for (const b of order) {
    if (out.size >= cap) break;
    if (b.w > 0 && b.h > 0 && placed.some((p) => b.x < p.x + p.w + 2 && b.x + b.w + 2 > p.x && b.y < p.y + p.h + 2 && b.y + b.h + 2 > p.y)) continue;
    placed.push(b);
    out.add(b.index);
  }
  return out;
}

export class HubView {
  readonly map: TownMap;
  readonly scene: TownScene;
  readonly camera: HubCamera;
  readonly renderer: TownRenderer;
  readonly clock: DayClock;
  readonly canvas: HTMLCanvasElement;
  /** Recent frame intervals and draw times, ms (HUD and measurements). */
  readonly timings = { intervals: [] as number[], draws: [] as number[], firstFrameMs: 0 };
  /** Counters for measurements: frames drawn, and loop iterations (drawn or not). */
  readonly stats = { frames: 0, ticks: 0 };
  private readonly labelLayer: HTMLDivElement;
  private readonly labels: { b: TownBuilding; el: HTMLButtonElement }[] = [];
  private readonly labelSize = new WeakMap<HTMLElement, { w: number; h: number }>();
  private readonly ctx: CanvasRenderingContext2D;
  private readonly opts: HubViewOptions;
  private readonly reducedMotion: boolean;
  private raf = 0;
  /** A wait for the next ambient frame (setTimeout), so the loop does not wake every vsync for nothing. */
  private timer = 0;
  private last = 0;
  private lastDraw = 0;
  private lastInput = 0;
  /** When the loop last drew or tried to (an unchanged frame is skipped by the renderer). */
  private lastTry = 0;
  private sleeping = false;
  private time = 0;
  private follow = true;
  private cssW = 1;
  private cssH = 1;
  private dpr = 1;
  private pausedFlag = false;
  /** The canvas was cleared (resized) and not drawn since. */
  private blank = true;
  private destroyed = false;
  private readonly cleanup: (() => void)[] = [];
  private walkerViews: WalkerView[] = [];
  /** Held direction keys (by code) and Shift. */
  private readonly held = new Map<string, readonly [number, number]>();
  private run = false;
  /** Whether the keyboard focus is in the view (the door hint shows only then). */
  private focused = false;
  private hint: HTMLDivElement | null = null;
  private hintFor: number | null = null;
  /** The person the keyboard hint is shown for (it takes the hint over from a door). */
  private hintPerson: string | null = null;
  /** Name tags over the people standing in the street, by key. */
  private readonly personTags = new Map<string, HTMLButtonElement>();
  /** The person the player stands next to. */
  private nearKey: string | null = null;
  /**
   * Dialogue mode: the person spoken to (their walker: a passer-by met keeps it
   * under a new key), the street's height covered by the dialogue box, the
   * zoom framed at and the zoom to go back to.
   */
  /** A change of who is here made during a conversation, applied when it ends. */
  private queuedPeople: { people: readonly StreetPerson[]; exits: ReadonlyMap<string, TownLocation | null> | undefined } | null = null;
  private talk: { w: Walker; inset: number; zoom: number; from: number } | null = null;
  /** After a conversation: the zoom the camera eases back to, then it is the player's again. */
  private talkReturn: number | null = null;
  private speaker: "player" | "person" | null = null;
  private readonly frameListeners = new Set<() => void>();

  constructor(
    readonly container: HTMLElement,
    options: HubViewOptions,
  ) {
    const t0 = performance.now();
    this.opts = options;
    this.map = options.map;
    this.reducedMotion = options.reducedMotion ?? (typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches);
    this.clock = options.clock ?? new DayClock(12 * 60, 0, false);
    this.scene = new TownScene(this.map, { townsfolk: options.townsfolk ?? 14 });
    this.syncFolk();

    container.style.position ||= "relative";
    container.style.overflow = "hidden";
    container.style.touchAction = "none";
    container.style.userSelect = "none";
    this.canvas = document.createElement("canvas");
    this.canvas.setAttribute("role", "img");
    this.canvas.setAttribute("aria-label", options.canvasLabel ?? `Map of ${this.map.municipality}. Use the list of places to go somewhere.`);
    Object.assign(this.canvas.style, { position: "absolute", inset: "0", width: "100%", height: "100%", display: "block", imageRendering: "pixelated" });
    container.prepend(this.canvas);
    const ctx = this.canvas.getContext("2d", { alpha: false });
    if (!ctx) throw new Error("no 2D canvas");
    this.ctx = ctx;
    this.labelLayer = document.createElement("div");
    this.labelLayer.className = "hub-labels";
    Object.assign(this.labelLayer.style, { position: "absolute", inset: "0", pointerEvents: "none", overflow: "hidden" });
    container.appendChild(this.labelLayer);

    this.renderer = new TownRenderer(this.ctx, this.map, options.art ?? new PlaceholderArt(), { reducedMotion: this.reducedMotion });
    const p = this.scene.player;
    this.camera = new HubCamera({ x: p.x * TILE, y: p.y * TILE - 8, zoom: 3 }, { width: 1, height: 1 }, {
      minZoom: 1,
      maxZoom: 6,
      integerZoom: true,
      bounds: { minX: 0, minY: 0, maxX: this.map.width * TILE, maxY: this.map.height * TILE },
    });
    this.makeLabels();
    this.bindInput();
    if (options.keyboard) this.bindKeys();
    this.resize(true);
    const ro = new ResizeObserver(() => this.resize(false));
    ro.observe(container);
    this.cleanup.push(() => ro.disconnect());
    const vis = () => (document.hidden ? this.stop() : this.wake());
    document.addEventListener("visibilitychange", vis);
    this.cleanup.push(() => document.removeEventListener("visibilitychange", vis));
    // Any input anywhere on the page (the pocket too) counts against idle sleep, and wakes a sleeping street.
    const poke = () => {
      this.lastInput = performance.now();
      if (this.sleeping) {
        this.sleeping = false;
        this.wake();
      }
    };
    for (const type of ["pointerdown", "pointermove", "wheel", "keydown", "touchstart"] as const) {
      window.addEventListener(type, poke, { capture: true, passive: true });
      this.cleanup.push(() => window.removeEventListener(type, poke, { capture: true }));
    }
    this.lastInput = performance.now();
    // First frame now, so the time to it is honest.
    this.last = performance.now();
    this.drawNow();
    this.timings.firstFrameMs = performance.now() - t0;
    this.wake();
  }

  // ------------------------------------------------------------------ public

  /** Set the time of day (minutes after midnight). */
  setClock(minutes: number) {
    this.clock.set(wrapMinutes(minutes));
    this.syncFolk();
    this.wake();
  }

  /** Move the time of day forward to `minutes` over `ms` (the game: a slot was spent). */
  easeClockTo(minutes: number, ms: number) {
    this.clock.easeTo(minutes, ms);
    this.syncFolk();
    this.wake();
  }

  private syncFolk() {
    if (this.opts.townsfolkByTime === false) return;
    this.scene.activeFolk = Math.round(this.scene.folk.length * townsfolkShare(this.clock.minutes));
  }

  /** Panels open over the hub pause it (nothing animates behind them). */
  get paused(): boolean {
    return this.pausedFlag;
  }
  set paused(v: boolean) {
    this.pausedFlag = v;
    if (v) {
      this.letGo();
      this.stop();
      // Whatever covers the street (a portrait conversation is see-through) shows its last frame, never a cleared canvas.
      if (this.blank && !this.destroyed) this.drawNow();
    } else this.wake();
  }

  /** Walk to a building's door. Returns false if it cannot be reached. */
  goToBuilding(index: number): boolean {
    const b = this.map.buildings[index];
    if (!b) return false;
    const ok = this.scene.goToBuilding(index, () => this.arrived(b));
    if (ok) this.follow = true;
    this.wake();
    return ok;
  }

  goToPlace(place: number): boolean {
    const b = buildingOfPlace(this.map, place);
    return b ? this.goToBuilding(b.index) : false;
  }

  /**
   * The people standing in the street to walk up to (the talk hook): new ones
   * stand a few steps from the player, those already there stay, the rest go.
   * Each has a name tag (a tap walks up to them).
   */
  setPeople(people: readonly StreetPerson[], exits?: ReadonlyMap<string, TownLocation | null>) {
    // In a conversation the speaker stays put (they would walk off in frame); the change is made when it ends.
    if (this.talk) {
      this.queuedPeople = { people, exits };
      return;
    }
    this.queuedPeople = null;
    // Anyone no longer here walks off towards where they go next (at once with reduced motion).
    const cells = new Map<string, number | null>();
    if (exits && !this.reducedMotion) {
      for (const [k, loc] of exits) cells.set(k, loc ? locationCell(this.map, loc) : null);
      for (const s of this.scene.standing) if (!cells.has(s.key)) cells.set(s.key, null);
    }
    this.scene.setStanding(people, this.reducedMotion ? undefined : cells);
    for (const [k, el] of this.personTags) {
      if (!this.scene.standing.some((x) => x.key === k)) {
        el.remove();
        this.personTags.delete(k);
      }
    }
    for (const p of people) {
      if (!this.scene.standing.some((x) => x.key === p.key)) continue;
      let el = this.personTags.get(p.key);
      if (!el) {
        el = document.createElement("button");
        el.type = "button";
        el.className = "hub-label hub-person-tag";
        // Out of the Tab order (the Notebook's People and the place panel are the keyboard route, DEC-051),
        // but named for screen readers, with what the marker means.
        el.tabIndex = -1;
        el.dataset.person = p.key;
        Object.assign(el.style, { position: "absolute", left: "0", top: "0", pointerEvents: "auto", whiteSpace: "nowrap", willChange: "transform" });
        const key = p.key;
        el.addEventListener("click", (e) => {
          e.stopPropagation();
          this.walkToPerson(key);
        });
        el.addEventListener("pointerdown", (e) => e.stopPropagation());
        this.labelLayer.appendChild(el);
        this.personTags.set(p.key, el);
      }
      fillPersonTag(el, p.label, p.marker, p.tagLabel);
    }
    this.nearKey = null;
    this.wake();
  }

  /** Walk up to someone standing in the street, then talk (`onTalk`). Returns false if they cannot be reached. */
  walkToPerson(key: string): boolean {
    const ok = this.scene.goToPerson(key, () => this.opts.onTalk?.(key));
    if (ok) this.follow = true;
    this.wake();
    return ok;
  }

  // ------------------------------------------------------------------ dialogue mode (DEC-093)

  /** Whether a conversation with someone standing in the street can be framed here: they stand by the player, and the street is in sight. */
  canTalk(key: string): boolean {
    if (this.pausedFlag || this.destroyed || this.scene.player.walk) return false;
    const s = this.scene.standing.find((x) => x.key === key);
    const p = this.scene.player;
    return !!s && Math.hypot(s.w.x - p.x, s.w.y - p.y) <= TALK_REACH + 0.4;
  }

  /**
   * Dialogue mode: frame the player and `key` facing each other, centred in
   * the street above `insetBottom` CSS px (the dialogue box), zoomed in to at
   * least TALK_ZOOM; walking and street input pause, labels hide and the
   * townsfolk stop. Returns false if they are not standing in the street.
   */
  beginTalk(key: string, insetBottom = 0): boolean {
    const s = this.scene.standing.find((x) => x.key === key);
    if (!s) return false;
    this.letGo();
    const step = 1 / this.dpr;
    const from = this.talk?.from ?? this.talkReturn ?? this.camera.state.zoom;
    const want = Math.min(6, Math.max(Math.ceil(TALK_ZOOM / step - 1e-9), Math.round(this.cssH / TALK_ROWS / step)) * step);
    this.talkReturn = null;
    this.talk = { w: s.w, inset: Math.max(0, insetBottom), zoom: Math.max(from, want), from };
    this.scene.folkPaused = true;
    this.labelLayer.style.visibility = "hidden";
    this.follow = false;
    this.wake();
    return true;
  }

  /** The dialogue box covers this much of the street now (CSS px from the bottom). */
  setTalkInset(px: number) {
    if (!this.talk || Math.abs(this.talk.inset - px) < 1) return;
    this.talk.inset = Math.max(0, px);
    this.wake();
  }

  /** Who is speaking (a reply being revealed): they bob a little (not with reduced motion). */
  setSpeaking(who: "player" | "person" | null) {
    if (this.speaker === who) return;
    this.speaker = who;
    this.wake();
  }

  /** Leave dialogue mode: the camera eases back to its zoom and follows the player again; walking resumes. */
  endTalk() {
    if (!this.talk) return;
    this.talkReturn = this.talk.from;
    this.talk = null;
    this.speaker = null;
    this.scene.folkPaused = false;
    this.labelLayer.style.visibility = "";
    this.follow = true;
    this.lastInput = performance.now();
    const q = this.queuedPeople;
    if (q) this.setPeople(q.people, q.exits);
    this.wake();
  }

  /** The person spoken to (their key now), or null outside dialogue mode. */
  get talking(): string | null {
    const t = this.talk;
    return t ? (this.scene.standing.find((s) => s.w === t.w)?.key ?? null) : null;
  }

  /** In dialogue mode: the top of the speakers' heads (CSS px in the container), and CSS px per tile; null when the person is gone. */
  talkAnchors(): { player: { x: number; y: number }; person: { x: number; y: number }; tile: number } | null {
    const t = this.talk;
    if (!t || !this.scene.standing.some((s) => s.w === t.w)) return null;
    const head = (w: Walker) => this.camera.worldToScreen({ x: w.x * TILE, y: (w.y - 1.45) * TILE });
    return { player: head(this.scene.player), person: head(t.w), tile: TILE * this.camera.state.zoom };
  }

  /** Called after every drawn frame (overlays that follow the camera). Returns the unsubscribe. */
  addFrameListener(fn: () => void): () => void {
    this.frameListeners.add(fn);
    return () => this.frameListeners.delete(fn);
  }

  /** The camera's goal in dialogue mode: the two speakers' midpoint at TALK_AT of the street above the box. */
  private talkCentre(t: { w: Walker; inset: number }, zoom: number): { x: number; y: number } {
    const a = this.scene.player;
    const b = t.w;
    const free = Math.max(this.cssH * 0.3, this.cssH - t.inset);
    const mx = ((a.x + b.x) / 2) * TILE;
    const my = ((a.y + b.y) / 2 - 0.7) * TILE;
    return this.camera.clampCentre(mx, my - (free * TALK_AT - this.cssH / 2) / zoom, zoom);
  }

  /** Ease the camera into (or back out of) dialogue mode; snaps with reduced motion. Returns true while still moving. */
  private stepTalkCamera(dt: number): boolean {
    const st = this.camera.state;
    const k = this.reducedMotion ? 1 : 1 - Math.exp(-5 * Math.max(0, dt));
    const zt = this.talk ? this.talk.zoom : (this.talkReturn ?? st.zoom);
    let moving = false;
    if (Math.abs(st.zoom - zt) > 1e-4) {
      st.zoom = Math.abs(zt - st.zoom) < 0.01 ? zt : st.zoom * Math.pow(zt / st.zoom, k);
      moving = true;
    } else st.zoom = zt;
    if (this.talk) {
      const c = this.talkCentre(this.talk, st.zoom);
      const dx = c.x - st.x;
      const dy = c.y - st.y;
      if (Math.hypot(dx, dy) * st.zoom > 0.3) {
        st.x += dx * k;
        st.y += dy * k;
        moving = true;
      } else {
        st.x = c.x;
        st.y = c.y;
      }
    } else if (!moving) this.talkReturn = null;
    return moving;
  }

  /** Walk to an engine location (a place, home, or a neighbourhood's streets); `notify`: call `onArrive` there. */
  goToLocation(loc: TownLocation, notify = true, onDone?: () => void, withinSeconds?: number): boolean {
    const ok = this.scene.goToLocation(
      loc,
      () => {
        onDone?.();
        if (!notify) return;
        const b = this.scene.atBuilding;
        this.arrived(b !== null ? (this.map.buildings[b] ?? null) : null);
      },
      withinSeconds,
    );
    if (ok) this.follow = true;
    this.wake();
    return ok;
  }

  /** Skip the walk: put the player at a building's door now. */
  placeAtBuilding(index: number) {
    const b = this.map.buildings[index];
    if (!b) return;
    this.scene.placePlayer(b.front);
    this.follow = true;
    this.wake();
  }

  /** Look at a point (tiles) without moving the player; following resumes with the next walk. */
  centreOn(x: number, y: number) {
    const c = this.camera.clampCentre(x * TILE, y * TILE);
    this.camera.state.x = c.x;
    this.camera.state.y = c.y;
    this.follow = false;
    this.wake();
  }

  /** Screen point (CSS px in the container) of a world point in tiles: for tests and overlays. */
  toScreen(x: number, y: number): { x: number; y: number } {
    return this.camera.worldToScreen({ x: x * TILE, y: y * TILE });
  }

  get zoom(): number {
    return this.camera.state.zoom;
  }

  destroy() {
    this.destroyed = true;
    this.stop();
    for (const f of this.cleanup) f();
    this.renderer.dispose();
    this.canvas.remove();
    this.labelLayer.remove();
    this.personTags.clear();
    this.frameListeners.clear();
  }

  // ------------------------------------------------------------------ labels

  private makeLabels() {
    const label = this.opts.label;
    if (!label) return;
    for (const b of this.map.buildings) {
      const text = label(b);
      if (!text) continue;
      const el = document.createElement("button");
      el.type = "button";
      el.className = b.landmark ? "hub-label hub-label-landmark" : "hub-label";
      const icon = this.opts.labelIcon?.(b);
      if (icon) {
        const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
        svg.setAttribute("class", "hub-label-icon");
        svg.setAttribute("aria-hidden", "true");
        const use = document.createElementNS("http://www.w3.org/2000/svg", "use");
        use.setAttribute("href", `#${icon}`);
        svg.appendChild(use);
        el.appendChild(svg);
      }
      el.append(text);
      // The place list is the keyboard and screen-reader route (DEC-051).
      el.tabIndex = -1;
      el.setAttribute("aria-hidden", "true");
      el.dataset.building = String(b.index);
      Object.assign(el.style, { position: "absolute", left: "0", top: "0", pointerEvents: "auto", whiteSpace: "nowrap", willChange: "transform" });
      el.addEventListener("click", (e) => {
        e.stopPropagation();
        this.goToBuilding(b.index);
      });
      el.addEventListener("pointerdown", (e) => e.stopPropagation());
      this.labelLayer.appendChild(el);
      this.labels.push({ b, el });
    }
  }

  private placeLabels() {
    const z = this.camera.state.zoom;
    // Candidates on screen, then a greedy pass so no two labels overlap (zoomed far out they would pile up),
    // landmarks first; zoomed out, at most LABEL_CAP at once (TOWN_GENERATION.md §8).
    const cands: LabelBox[] = [];
    const pos: { x: number; y: number }[] = [];
    this.labels.forEach(({ b, el }, i) => {
      const p = this.camera.worldToScreen({ x: (b.x + b.w / 2) * TILE, y: (b.y - ROOF_ROWS) * TILE });
      pos[i] = p;
      const want = (z >= LABEL_ZOOM || b.landmark) && p.x > -120 && p.x < this.cssW + 120 && p.y > -10 && p.y < this.cssH + 40;
      if (!want) return;
      let size = this.labelSize.get(el);
      if (!size) {
        // Measured once, while shown (0 × 0 where layout is not available, e.g. tests).
        el.style.display = "";
        size = { w: el.offsetWidth, h: el.offsetHeight };
        this.labelSize.set(el, size);
      }
      cands.push({ index: i, x: p.x - size.w / 2, y: p.y - size.h, w: size.w, h: size.h, priority: (b.landmark ? 0 : b.home ? 1 : 2) * 100000 + b.index });
    });
    const shown = chooseLabels(cands, z < LABEL_ZOOM ? LABEL_CAP : Infinity);
    this.labels.forEach(({ el }, i) => {
      const show = shown.has(i);
      el.style.display = show ? "" : "none";
      const p = pos[i] as { x: number; y: number };
      if (show) el.style.transform = `translate(${Math.round(p.x)}px, ${Math.round(p.y)}px) translate(-50%, -100%)`;
    });
    // Name tags over the people standing in the street (always, while on screen).
    for (const { key, w } of this.scene.standing) {
      const el = this.personTags.get(key);
      if (!el) continue;
      const p = this.camera.worldToScreen({ x: w.x * TILE, y: (w.y - 1.5) * TILE });
      const on = p.x > -120 && p.x < this.cssW + 120 && p.y > -10 && p.y < this.cssH + 40;
      el.style.display = on ? "" : "none";
      el.classList.toggle("hub-person-near", key === this.nearKey);
      if (on) el.style.transform = `translate(${Math.round(p.x)}px, ${Math.round(p.y)}px) translate(-50%, -100%)`;
    }
    // The "!" bobs with the frames drawn anyway (ambient ones included): still once the loop sleeps.
    bobTags(this.personTags.values(), this.time, this.reducedMotion);
    this.placeHint();
  }

  // ------------------------------------------------------------------ input

  private bindInput() {
    const c = this.container;
    const local = (e: PointerEvent | WheelEvent) => {
      const r = c.getBoundingClientRect();
      return { x: e.clientX - r.left, y: e.clientY - r.top };
    };
    const on = <K extends keyof HTMLElementEventMap>(type: K, f: (e: HTMLElementEventMap[K]) => void, o?: AddEventListenerOptions) => {
      c.addEventListener(type, f as EventListener, o);
      this.cleanup.push(() => c.removeEventListener(type, f as EventListener, o));
    };
    // Input that reaches the street means the street is in sight: a pause (set for a sheet thought to
    // cover it) never swallows it. The owner sets `paused` again when what covers it changes.
    const seen = () => {
      if (this.pausedFlag) this.pausedFlag = false;
    };
    for (const type of ["pointerdown", "wheel", "keydown"] as const) on(type, seen, { capture: true });
    on("pointerdown", (e) => {
      // Dialogue mode: the street takes no input until the conversation ends.
      if (this.talk) return;
      if (e.button !== 0 && e.pointerType === "mouse") return;
      c.setPointerCapture?.(e.pointerId);
      this.camera.pointerDown({ id: e.pointerId, ...local(e) });
      this.wake();
    });
    on("pointermove", (e) => {
      if (this.talk) return;
      const s = this.camera.state;
      const before = [s.x, s.y, s.zoom];
      this.camera.pointerMove({ id: e.pointerId, ...local(e) });
      if (s.x !== before[0] || s.y !== before[1] || s.zoom !== before[2]) {
        // The player took the camera: stop following until the next walk.
        this.follow = false;
        this.wake();
      }
    });
    on("pointerup", (e) => {
      if (this.talk) return;
      const tap = this.camera.pointerUp({ id: e.pointerId, ...local(e) });
      if (tap) this.tap(tap.x, tap.y);
      this.wake();
    });
    on("pointercancel", (e) => {
      this.camera.pointerCancel(e.pointerId);
      this.wake();
    });
    on(
      "wheel",
      (e) => {
        e.preventDefault();
        if (this.talk) return;
        this.follow = false;
        this.camera.wheel(local(e), e.deltaY);
        this.wake();
      },
      { passive: false },
    );
  }

  private tap(sx: number, sy: number) {
    const w = this.camera.screenToWorld({ x: sx, y: sy });
    // A tap on someone standing in the street: walk up to them and talk.
    const who = this.scene.personAt({ x: w.x / TILE, y: w.y / TILE });
    if (who !== null && this.walkToPerson(who)) return;
    const t = this.scene.tap({ x: w.x / TILE, y: w.y / TILE }, () => {
      const b = this.scene.atBuilding;
      this.arrived(b !== null ? (this.map.buildings[b] ?? null) : null);
    });
    if (t) this.follow = true;
  }

  private arrived(b: TownBuilding | null) {
    this.opts.onArrive?.(b);
  }

  // ------------------------------------------------------------------ keyboard

  private bindKeys() {
    const c = this.container;
    const cv = this.canvas;
    cv.tabIndex = 0;
    cv.setAttribute("role", "application");
    const hint = document.createElement("div");
    hint.className = "hub-door-hint";
    hint.setAttribute("aria-hidden", "true");
    Object.assign(hint.style, { position: "absolute", left: "0", top: "0", display: "none", pointerEvents: "none", whiteSpace: "nowrap" });
    this.labelLayer.appendChild(hint);
    this.hint = hint;
    const on = <K extends keyof HTMLElementEventMap>(target: HTMLElement | Window, type: K, f: (e: HTMLElementEventMap[K]) => void) => {
      target.addEventListener(type, f as EventListener);
      this.cleanup.push(() => target.removeEventListener(type, f as EventListener));
    };
    on(c, "keydown", (e) => {
      if (this.pausedFlag || this.talk || e.altKey || e.ctrlKey || e.metaKey || editable(e.target)) return;
      const d = KEY_DIRS[e.code] ?? KEY_DIRS[e.key];
      if (d) {
        e.preventDefault();
        this.held.set(KEY_DIRS[e.code] ? e.code : e.key, d);
        this.run = e.shiftKey;
        this.applyKeys();
        return;
      }
      if (e.key === "Shift") {
        this.run = true;
        this.applyKeys();
        return;
      }
      // Enter or Space at a door: arrive there, as a tap on the building would. Only from the canvas
      // itself (a clicked label button has its own Enter).
      if ((e.key === "Enter" || e.key === " ") && e.target === cv) {
        // Standing next to someone: talk to them (the talk hook takes Enter before a door).
        const near = this.opts.onTalk ? this.scene.personNear() : null;
        if (near !== null) {
          e.preventDefault();
          if (!e.repeat) this.opts.onTalk?.(near);
          return;
        }
        const at = this.scene.doorHere;
        const b = at !== null ? this.map.buildings[at] : undefined;
        if (!b) return;
        e.preventDefault();
        if (!e.repeat) this.arrived(b);
      }
    });
    on(c, "keyup", (e) => {
      const id = KEY_DIRS[e.code] ? e.code : e.key;
      if (this.held.delete(id) || e.key === "Shift") {
        this.run = e.shiftKey && e.key !== "Shift";
        this.applyKeys();
      }
    });
    on(c, "focusin", () => {
      this.focused = true;
      this.updateDoor();
    });
    on(c, "focusout", (e) => {
      if (e.relatedTarget instanceof Node && c.contains(e.relatedTarget)) return;
      this.focused = false;
      this.letGo();
      this.updateDoor();
    });
    on(window, "blur", () => this.letGo());
  }

  /** Steer by the held keys (the last of two opposite keys does not win: they cancel). */
  private applyKeys() {
    let dx = 0;
    let dy = 0;
    for (const [x, y] of this.held.values()) {
      dx += x;
      dy += y;
    }
    this.scene.steer(dx, dy, this.run);
    if (this.scene.steering) {
      this.follow = true;
      this.wake();
    }
  }

  /** Release every key (focus left, the window lost focus, the view paused). */
  private letGo() {
    if (!this.held.size && !this.scene.steering) return;
    this.held.clear();
    this.run = false;
    this.scene.steer(0, 0);
  }

  /** The door hint: shown while the keyboard is in the view and the player stands at a place's door. */
  private updateDoor(wake = true) {
    this.updatePerson();
    if (!this.hint) return;
    // Next to someone (the talk hook): their hint, not the door's.
    const who = this.focused && !this.pausedFlag && this.opts.onTalk ? this.nearKey : null;
    const ptext = who !== null ? (this.opts.personHint?.(who) ?? null) : null;
    if (ptext !== null || this.hintPerson !== null) {
      if (who === this.hintPerson && ptext !== null) return;
      this.hintPerson = ptext !== null ? who : null;
      if (ptext !== null) {
        this.hintFor = null;
        this.hint.textContent = ptext;
        this.hint.style.display = "";
        this.placeHint();
        this.opts.onDoor?.(null);
        if (wake) this.wake();
        return;
      }
      this.hint.style.display = "none";
      this.hintFor = null;
    }
    const at = this.focused && !this.pausedFlag ? this.scene.doorHere : null;
    const b = at !== null ? (this.map.buildings[at] ?? null) : null;
    const text = b ? (this.opts.doorHint?.(b) ?? null) : null;
    const id = text ? at : null;
    if (id === this.hintFor) return;
    this.hintFor = id;
    this.hint.textContent = text ?? "";
    this.hint.style.display = text ? "" : "none";
    this.placeHint();
    this.opts.onDoor?.(text ? b : null);
    if (wake) this.wake();
  }

  /** Who the player stands next to (the talk hook): tells the owner when it changes. */
  private updatePerson() {
    const k = this.pausedFlag || this.scene.standing.length === 0 ? null : this.scene.personNear();
    if (k === this.nearKey) return;
    this.nearKey = k;
    for (const [key, el] of this.personTags) el.classList.toggle("hub-person-near", key === k);
    this.opts.onPersonNear?.(k);
  }

  private placeHint() {
    if (this.hint && this.hintPerson !== null) {
      const s = this.scene.standing.find((x) => x.key === this.hintPerson);
      if (s) {
        const p = this.camera.worldToScreen({ x: s.w.x * TILE, y: (s.w.y - 1.5) * TILE });
        this.hint.style.transform = `translate(${Math.round(p.x)}px, ${Math.round(p.y)}px) translate(-50%, -100%)`;
      }
      return;
    }
    const b = this.hintFor !== null ? this.map.buildings[this.hintFor] : undefined;
    if (!this.hint || !b) return;
    // Over the door, on the building's front.
    const p = this.camera.worldToScreen({ x: (b.doorX + 0.5) * TILE, y: b.doorY * TILE });
    this.hint.style.transform = `translate(${Math.round(p.x)}px, ${Math.round(p.y)}px) translate(-50%, -100%)`;
  }

  // ------------------------------------------------------------------ loop

  private resize(first: boolean) {
    const r = this.container.getBoundingClientRect();
    this.cssW = Math.max(1, Math.round(r.width));
    this.cssH = Math.max(1, Math.round(r.height));
    // Device resolution unless a caller asks for CSS pixels: many crisp zoom steps.
    this.dpr = this.opts.resolution === "css" ? 1 : window.devicePixelRatio || 1;
    this.canvas.width = Math.round(this.cssW * this.dpr);
    this.canvas.height = Math.round(this.cssH * this.dpr);
    this.ctx.imageSmoothingEnabled = false;
    this.camera.resize(this.cssW, this.cssH);
    // Zoom in whole device pixels: at device resolution a phone gets many crisp
    // steps, out to one device pixel per art pixel (most of a town on screen).
    const step = 1 / this.dpr;
    this.camera.setZoomLimits(step, 6, step);
    if (first) {
      // About fourteen tiles across the shorter side, on a whole zoom step.
      const z0 = Math.max(2, Math.min(4, Math.min(this.cssW, this.cssH) / (TILE * 14)));
      // Round up to a whole step: never open below the labelled range.
      const z = Math.max(step, Math.ceil(z0 / step - 1e-9) * step);
      this.camera.state.zoom = z;
      const c = this.camera.clampCentre(this.scene.player.x * TILE, this.scene.player.y * TILE - 8, z);
      this.camera.state.x = c.x;
      this.camera.state.y = c.y;
    }
    this.renderer.invalidate();
    this.blank = true;
    // Paused behind something (a portrait conversation, a sheet), the resized canvas is drawn once rather than left blank.
    if (!first) {
      if (this.pausedFlag && !this.destroyed) this.drawNow();
      else this.wake();
    }
  }

  private stop() {
    if (this.raf) cancelAnimationFrame(this.raf);
    this.raf = 0;
    if (this.timer) clearTimeout(this.timer);
    this.timer = 0;
  }

  /** Start the loop if it is not running (something may move); a wait for an ambient frame ends now. */
  wake() {
    if (this.destroyed || this.pausedFlag || this.raf || (typeof document !== "undefined" && document.hidden)) return;
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = 0;
    } else this.last = performance.now();
    this.raf = requestAnimationFrame((t) => this.frame(t));
  }

  /** Whether idle sleep stopped the loop (no input for `idleMs`); the next input wakes it. */
  get asleep(): boolean {
    return this.sleeping;
  }

  private walking(w: Walker) {
    return w.walk !== null;
  }

  private frame(now: number) {
    this.raf = 0;
    this.stats.ticks++;
    const dt = Math.min(0.1, Math.max(0, (now - this.last) / 1000));
    this.last = now;
    this.time += dt;
    this.scene.step(dt);
    // Dialogue mode drives the camera itself (framing the speakers, then back to the player's zoom).
    const eased = this.talk || this.talkReturn !== null ? this.stepTalkCamera(dt) : this.camera.update(dt);
    const p = this.scene.player;
    if (this.hint) this.updateDoor(false);
    else this.updatePerson();
    if (this.follow && !this.camera.gesturing) {
      // Walked by the keyboard with reduced motion: the camera keeps the player still on screen (no easing).
      const k = this.reducedMotion && this.scene.steering ? 1 : 1 - Math.exp(-6 * dt);
      const t = this.camera.clampCentre(p.x * TILE, p.y * TILE - 8);
      this.camera.state.x += (t.x - this.camera.state.x) * k;
      this.camera.state.y += (t.y - this.camera.state.y) * k;
    }
    const ticked = this.clock.advance(dt);
    if (ticked) this.syncFolk();
    const playerMoving = this.walking(p) || this.scene.steering;
    // In dialogue mode the townsfolk stand still and only the speaker moves.
    const folkMoving = (this.scene.activeFolk > 0 || this.scene.leaving.length > 0) && !this.scene.folkPaused;
    const bobbing = this.speaker !== null && !this.reducedMotion;
    // Full rate: the player, the camera (easing, following, a gesture) and the light changing.
    const busy = playerMoving || eased || this.camera.gesturing || ticked || this.clock.running || this.renderer.animating || this.followSettling();
    // Idle sleep (DEC-080 "Battery"): nobody has touched anything for a while; draw this frame and stop.
    const idle = !busy && this.scene.leaving.length === 0 && now - this.lastInput >= (this.opts.idleMs ?? 20_000);
    // Ambient-only frames are capped (DEC-080: battery).
    const cap = 1000 / (this.talk ? TALK_FPS : (this.opts.ambientFps ?? 15));
    if (busy || idle || now - this.lastTry >= cap - 1) {
      this.lastTry = now;
      this.drawNow(now);
    }
    this.sleeping = idle;
    if (idle || this.pausedFlag) return;
    if (busy) this.raf = requestAnimationFrame((t) => this.frame(t));
    else if (folkMoving || bobbing) {
      // Wait for the next ambient frame off the vsync; a frame or less away, the next vsync.
      const wait = this.lastTry + cap - performance.now();
      if (wait < 12) this.raf = requestAnimationFrame((t) => this.frame(t));
      else
        this.timer = window.setTimeout(() => {
          this.timer = 0;
          this.raf = requestAnimationFrame((t) => this.frame(t));
        }, wait - 8);
    }
  }

  private followSettling(): boolean {
    if (!this.follow) return false;
    const p = this.scene.player;
    const t = this.camera.clampCentre(p.x * TILE, p.y * TILE - 8);
    return Math.hypot(t.x - this.camera.state.x, t.y - this.camera.state.y) * this.camera.state.zoom > 0.5;
  }

  private drawNow(now = performance.now()) {
    const s = this.scene;
    const views = this.walkerViews;
    views.length = 0;
    // One walk frame per 4 art px walked (PIXEL_ART_TOOL §6), through every frame the art has.
    const frames = this.renderer.art.walkFrames;
    const t = this.talk;
    // Dialogue mode: the two speakers face each other; the one speaking bobs an art pixel (about 4 a second).
    if (t) {
      s.player.dir = facing(t.w.x - s.player.x, t.w.y - s.player.y);
      t.w.dir = facing(s.player.x - t.w.x, s.player.y - t.w.y);
    }
    const bob = t && this.speaker && !this.reducedMotion && Math.floor(this.time * 4) % 2 === 1 ? (this.speaker === "player" ? s.player : t.w) : null;
    const view = (w: Walker): WalkerView => ({ x: w.x, y: w.y - (w === bob ? 1 / TILE : 0), dir: w.dir, frame: w.moving ? 1 + (Math.floor(w.dist * 4) % frames) : 0, variant: w.variant });
    views.push(view(s.player));
    for (const w of s.outFolk) views.push(view(w));
    for (const { w } of s.leaving) views.push(view(w));
    // People standing in the street turn to face the player when the player is by them.
    for (const { key, w } of s.standing) {
      if (key === this.nearKey && !t) w.dir = facing(s.player.x - w.x, s.player.y - w.y);
      views.push(view(w));
    }
    const cam = this.camera.state;
    const t0 = performance.now();
    const drew = this.renderer.draw({
      width: this.canvas.width,
      height: this.canvas.height,
      camX: cam.x,
      camY: cam.y,
      zoom: cam.zoom * this.dpr,
      minutes: this.clock.minutes,
      walkers: views,
      // In a conversation the speakers need no "you are here" (it would sit under the bubbles).
      player: t ? null : { x: s.player.x, y: s.player.y },
      target: s.target ? { cell: s.target.cell, building: s.target.kind === "building" ? s.target.building : null } : null,
      time: this.time,
    });
    if (!drew) return;
    this.blank = false;
    const t1 = performance.now();
    if (this.lastDraw) push(this.timings.intervals, now - this.lastDraw);
    push(this.timings.draws, t1 - t0);
    this.lastDraw = now;
    this.stats.frames++;
    this.placeLabels();
    for (const f of this.frameListeners) f();
    this.opts.onFrame?.(this);
  }
}

/** 0 down, 1 up, 2 left, 3 right: the way to look along (dx, dy). */
function facing(dx: number, dy: number): number {
  return Math.abs(dx) > Math.abs(dy) ? (dx < 0 ? 2 : 3) : dy < 0 ? 1 : 0;
}

function push(a: number[], v: number) {
  a.push(v);
  if (a.length > 600) a.splice(0, a.length - 600);
}
