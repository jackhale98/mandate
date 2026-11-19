// The walkable town in a DOM element (DEC-080, DEC-048, DEC-051): a canvas
// drawn by ./render/townRenderer at CSS resolution (upscaled pixelated by
// the browser), HTML labels over it, pointer input through ./camera (pan,
// pinch, wheel) and taps through ./townScene (a building walks to its door,
// ground walks there). The loop runs only while something moves, caps
// ambient-only frames at 30 fps, and stops when the page is hidden or the
// view is paused (a panel is open). Framework-free, so the prototype page
// and the app's React screen can both mount it.
import { HubCamera } from "./camera";
import { DayClock, townsfolkShare, wrapMinutes } from "./dayCycle";
import type { ArtProvider } from "./render/art";
import { PlaceholderArt } from "./render/placeholderArt";
import { TownRenderer, type WalkerView } from "./render/townRenderer";
import { ROOF_ROWS, buildingOfPlace, type TownBuilding, type TownMap } from "./townLayout";
import type { TownLocation } from "./townInput";
import { TownScene, type Walker } from "./townScene";

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
  /** Frame cap while only townsfolk move (default 30). */
  ambientFps?: number;
  /** Called after every drawn frame (a HUD). */
  onFrame?: (v: HubView) => void;
}

const TILE = 16;

/** Labels shown at once when zoomed out (TOWN_GENERATION.md §8: at most 12 on the overview). */
export const LABEL_CAP = 12;

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
  private readonly labelLayer: HTMLDivElement;
  private readonly labels: { b: TownBuilding; el: HTMLButtonElement }[] = [];
  private readonly labelSize = new WeakMap<HTMLElement, { w: number; h: number }>();
  private readonly ctx: CanvasRenderingContext2D;
  private readonly opts: HubViewOptions;
  private readonly reducedMotion: boolean;
  private raf = 0;
  private last = 0;
  private lastDraw = 0;
  private time = 0;
  private follow = true;
  private cssW = 1;
  private cssH = 1;
  private dpr = 1;
  private pausedFlag = false;
  private destroyed = false;
  private readonly cleanup: (() => void)[] = [];
  private walkerViews: WalkerView[] = [];

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
    this.resize(true);
    const ro = new ResizeObserver(() => this.resize(false));
    ro.observe(container);
    this.cleanup.push(() => ro.disconnect());
    const vis = () => (document.hidden ? this.stop() : this.wake());
    document.addEventListener("visibilitychange", vis);
    this.cleanup.push(() => document.removeEventListener("visibilitychange", vis));
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
    if (v) this.stop();
    else this.wake();
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

  /** Walk to an engine location (a place, home, or a neighbourhood's streets); `notify`: call `onArrive` there. */
  goToLocation(loc: TownLocation, notify = true): boolean {
    const ok = this.scene.goToLocation(loc, () => {
      if (!notify) return;
      const b = this.scene.atBuilding;
      this.arrived(b !== null ? (this.map.buildings[b] ?? null) : null);
    });
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
      const want = (z >= 2 || b.landmark) && p.x > -120 && p.x < this.cssW + 120 && p.y > -10 && p.y < this.cssH + 40;
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
    const shown = chooseLabels(cands, z < 2 ? LABEL_CAP : Infinity);
    this.labels.forEach(({ el }, i) => {
      const show = shown.has(i);
      el.style.display = show ? "" : "none";
      const p = pos[i] as { x: number; y: number };
      if (show) el.style.transform = `translate(${Math.round(p.x)}px, ${Math.round(p.y)}px) translate(-50%, -100%)`;
    });
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
    on("pointerdown", (e) => {
      if (e.button !== 0 && e.pointerType === "mouse") return;
      c.setPointerCapture?.(e.pointerId);
      this.camera.pointerDown({ id: e.pointerId, ...local(e) });
      this.wake();
    });
    on("pointermove", (e) => {
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
        this.follow = false;
        this.camera.wheel(local(e), e.deltaY);
        this.wake();
      },
      { passive: false },
    );
  }

  private tap(sx: number, sy: number) {
    const w = this.camera.screenToWorld({ x: sx, y: sy });
    const t = this.scene.tap({ x: w.x / TILE, y: w.y / TILE }, () => {
      const b = this.scene.atBuilding;
      this.arrived(b !== null ? (this.map.buildings[b] ?? null) : null);
    });
    if (t) this.follow = true;
  }

  private arrived(b: TownBuilding | null) {
    this.opts.onArrive?.(b);
  }

  // ------------------------------------------------------------------ loop

  private resize(first: boolean) {
    const r = this.container.getBoundingClientRect();
    this.cssW = Math.max(1, Math.round(r.width));
    this.cssH = Math.max(1, Math.round(r.height));
    this.dpr = this.opts.resolution === "device" ? window.devicePixelRatio || 1 : 1;
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
      const z = Math.max(step, Math.round(z0 / step) * step);
      this.camera.state.zoom = z;
      const c = this.camera.clampCentre(this.scene.player.x * TILE, this.scene.player.y * TILE - 8, z);
      this.camera.state.x = c.x;
      this.camera.state.y = c.y;
    }
    this.renderer.invalidate();
    if (!first) this.wake();
  }

  private stop() {
    if (this.raf) cancelAnimationFrame(this.raf);
    this.raf = 0;
  }

  /** Start the loop if it is not running (something may move). */
  wake() {
    if (this.destroyed || this.pausedFlag || this.raf || (typeof document !== "undefined" && document.hidden)) return;
    this.last = performance.now();
    this.raf = requestAnimationFrame((t) => this.frame(t));
  }

  private walking(w: Walker) {
    return w.walk !== null;
  }

  private frame(now: number) {
    this.raf = 0;
    const dt = Math.min(0.1, Math.max(0, (now - this.last) / 1000));
    this.last = now;
    this.time += dt;
    this.scene.step(dt);
    const eased = this.camera.update(dt);
    const p = this.scene.player;
    if (this.follow && !this.camera.gesturing) {
      const k = 1 - Math.exp(-6 * dt);
      const t = this.camera.clampCentre(p.x * TILE, p.y * TILE - 8);
      this.camera.state.x += (t.x - this.camera.state.x) * k;
      this.camera.state.y += (t.y - this.camera.state.y) * k;
    }
    const ticked = this.clock.advance(dt);
    if (ticked) this.syncFolk();
    const playerMoving = this.walking(p);
    const folkMoving = this.scene.activeFolk > 0;
    const busy = playerMoving || eased || this.camera.gesturing || ticked || this.clock.running || this.renderer.animating;
    // Ambient-only frames are capped (DEC-080: battery).
    const cap = 1000 / (this.opts.ambientFps ?? 30);
    if (busy || now - this.lastDraw >= cap - 1) this.drawNow(now);
    if (busy || folkMoving || this.renderer.animating || this.followSettling()) this.raf = requestAnimationFrame((t) => this.frame(t));
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
    const view = (w: Walker): WalkerView => ({ x: w.x, y: w.y, dir: w.dir, frame: w.moving ? 1 + (Math.floor(w.dist * 4) % frames) : 0, variant: w.variant });
    views.push(view(s.player));
    for (const w of s.outFolk) views.push(view(w));
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
      player: { x: s.player.x, y: s.player.y },
      target: s.target ? { cell: s.target.cell, building: s.target.kind === "building" ? s.target.building : null } : null,
      time: this.time,
    });
    if (!drew) return;
    const t1 = performance.now();
    if (this.lastDraw) push(this.timings.intervals, now - this.lastDraw);
    push(this.timings.draws, t1 - t0);
    this.lastDraw = now;
    this.placeLabels();
    this.opts.onFrame?.(this);
  }
}

function push(a: number[], v: number) {
  a.push(v);
  if (a.length > 600) a.splice(0, a.length - 600);
}
