// The town map overview in a DOM element (DEC-086, DEC-080, DEC-048,
// DEC-051): a canvas at CSS resolution (at most 2× for crisp glyphs on
// phones), one button per hotspot over it and a name chip per hotspot (HTML,
// so names are sharp, translated and read by screen readers), pan, pinch and
// wheel zoom through ./camera (anchored pinch, re-baselined when fingers
// change, eased limits), taps resolved to the hotspot under the finger, and
// keyboard movement between places. Frames are drawn only when something
// changed (the camera, a walk, an overlay); an idle map draws nothing.
//
// Framework-free, like ./hubView: the app's React component renders the
// buttons and chips (`data-hotspot` / `data-chip` with the hotspot's key) and
// calls `syncElements()` after each render; this positions them.
import { HubCamera } from "./camera";
import type { Point } from "./pathfinding";
import { GLYPH_RISE, drawTownMapFrame, type BakedTownMap, type MapMarker } from "./render/townMapRenderer";
import { RouteCache, alongRoute, cellPoint, routeLength, type MapHotspot, type TownMapScene } from "./townMapScene";

export interface TownMapOverlay {
  /** The hotspot where the player is (null: somewhere the map has no hotspot for). */
  here: string | null;
  /** Hotspot keys with something waiting there. */
  waiting: ReadonlySet<string>;
  /** Minutes after midnight for the gentle grade, or null. */
  minutes: number | null;
}

export interface TownMapViewOptions {
  scene: TownMapScene;
  baked: BakedTownMap;
  /** A hotspot was tapped, clicked or activated from the keyboard. */
  onActivate: (h: MapHotspot) => void;
  /** Strength of the time-of-day grade (default 0.5: subtle, never at the cost of legibility). */
  gradeStrength?: number;
  reducedMotion?: boolean;
}

interface Placed {
  h: MapHotspot;
  box: HTMLElement | null;
  chip: HTMLElement | null;
  /** Chip size, CSS px (measured after each render). */
  cw: number;
  ch: number;
  /** Where the chip was put this frame (screen px), when shown. */
  chipRect: { x: number; y: number; w: number; h: number } | null;
}

/** Minimum hit size of a hotspot's button, CSS px. */
const MIN_HIT = 22;
/** Largest zoom, CSS px per tile (the walkable street is 48). */
const MAX_ZOOM = 28;
/** Fastest and slowest walk shown on the map, ms (DEC-079: short, skippable). */
const WALK_MIN_MS = 260;
export const WALK_MAX_MS = 600;

export class TownMapView {
  readonly scene: TownMapScene;
  readonly baked: BakedTownMap;
  camera: HubCamera;
  /** Frames drawn so far (tests: an idle map draws none). */
  frames = 0;
  /** Last frame's draw time, ms. */
  lastDrawMs = 0;
  private readonly canvas: HTMLCanvasElement;
  private readonly ctx: CanvasRenderingContext2D;
  private readonly opts: TownMapViewOptions;
  private readonly routes: RouteCache;
  private readonly placed = new Map<string, Placed>();
  private overlay: TownMapOverlay = { here: null, waiting: new Set(), minutes: null };
  private cssW = 1;
  private cssH = 1;
  private scale = 1;
  private touched = false;
  private raf = 0;
  private last = 0;
  private hover: string | null = null;
  private focused: string | null = null;
  /** The walk being shown: its route, when it started, how long it takes. */
  private walk: { route: Point[]; start: number; ms: number; to: string } | null = null;
  /** Where the "you" marker rests (a walk's end until the engine says otherwise). */
  private youAt: Point | null = null;
  private youKey: string | null = null;
  /** The last hotspot gone to: kept in view when the map is resized (a docked panel). */
  private keep: string | null = null;
  /** When a pointer last activated something (a click that follows it is the same press). */
  private pointerActivated = 0;
  private pressTarget: string | null = null;
  /** Focus moved by the keyboard (reveal the place), not by a press (the map must not move under a finger). */
  private keyboard = false;
  private destroyed = false;
  private readonly cleanup: (() => void)[] = [];

  constructor(
    readonly container: HTMLElement,
    canvas: HTMLCanvasElement,
    readonly layer: HTMLElement,
    options: TownMapViewOptions,
  ) {
    this.scene = options.scene;
    this.baked = options.baked;
    this.opts = options;
    this.canvas = canvas;
    const ctx = canvas.getContext("2d", { alpha: false });
    if (!ctx) throw new Error("no 2D canvas");
    this.ctx = ctx;
    this.routes = new RouteCache(this.scene.map);
    const r = container.getBoundingClientRect();
    this.cssW = Math.max(1, r.width);
    this.cssH = Math.max(1, r.height);
    this.camera = this.makeCamera(null);
    this.resizeCanvas();
    this.bindInput();
    const ro = new ResizeObserver(() => this.resize());
    ro.observe(container);
    this.cleanup.push(() => ro.disconnect());
    this.syncElements();
    this.requestDraw();
  }

  // ------------------------------------------------------------------ camera

  /** The zoom that frames the whole town. */
  get fitZoom(): number {
    return Math.min(this.cssW / this.baked.width, this.cssH / this.baked.height);
  }

  private makeCamera(prev: { x: number; y: number; zoom: number } | null): HubCamera {
    const fit = this.fitZoom;
    const max = Math.max(fit * 1.5, MAX_ZOOM);
    const state = prev ? { ...prev, zoom: Math.min(max, Math.max(fit, prev.zoom)) } : { x: this.baked.width / 2, y: this.baked.height / 2, zoom: fit };
    const cam = new HubCamera(state, { width: this.cssW, height: this.cssH }, {
      minZoom: fit,
      maxZoom: max,
      bounds: { minX: 0, minY: 0, maxX: this.baked.width, maxY: this.baked.height },
      wheelRate: 0.0018,
    });
    const c = cam.clampCentre(cam.state.x, cam.state.y);
    cam.state.x = c.x;
    cam.state.y = c.y;
    return cam;
  }

  private resizeCanvas() {
    this.scale = Math.min(2, Math.max(1, window.devicePixelRatio || 1));
    this.canvas.width = Math.round(this.cssW * this.scale);
    this.canvas.height = Math.round(this.cssH * this.scale);
  }

  /** The container changed size: keep the view (no jumps), or keep framing the town if untouched. */
  private resize() {
    const r = this.container.getBoundingClientRect();
    const w = Math.max(1, r.width);
    const h = Math.max(1, r.height);
    if (Math.abs(w - this.cssW) < 0.5 && Math.abs(h - this.cssH) < 0.5) return;
    if (this.camera.gesturing) {
      // Mid-gesture (a phone's address bar): only the viewport changes; limits follow when it ends.
      this.camera.resize(w, h);
      this.cssW = w;
      this.cssH = h;
      this.resizeCanvas();
      this.requestDraw();
      return;
    }
    const s = this.camera.state;
    // Keep the map point at the top-left corner where it is.
    const left = s.x - this.cssW / 2 / s.zoom;
    const top = s.y - this.cssH / 2 / s.zoom;
    this.cssW = w;
    this.cssH = h;
    this.resizeCanvas();
    if (!this.touched) this.camera = this.makeCamera(null);
    else {
      this.camera = this.makeCamera({ x: left + w / 2 / s.zoom, y: top + h / 2 / s.zoom, zoom: s.zoom });
      this.revealKept();
    }
    this.requestDraw();
  }

  /** After a resize, bring the place last gone to back into view if the resize hid it. */
  private revealKept() {
    const h = this.keep ? this.placed.get(this.keep)?.h : null;
    if (!h) return;
    const p = this.camera.worldToScreen(h.anchor);
    if (p.x >= 0 && p.y >= 0 && p.x <= this.cssW && p.y <= this.cssH) return;
    const c = this.camera.clampCentre(h.anchor.x, h.anchor.y);
    this.camera.state.x = c.x;
    this.camera.state.y = c.y;
  }

  /** Zoom by a factor about the view centre (keyboard and buttons). */
  zoomBy(factor: number) {
    this.touched = true;
    const at = { x: this.cssW / 2, y: this.cssH / 2 };
    this.camera.wheel(at, -Math.log(factor) / 0.0018);
    this.requestDraw();
  }

  /** Frame the whole town again. */
  fit() {
    this.touched = false;
    this.camera = this.makeCamera(null);
    this.requestDraw();
  }

  /** Centre on a hotspot without changing the zoom. */
  private reveal(h: MapHotspot) {
    const p = this.camera.worldToScreen(h.anchor);
    const m = 40;
    if (p.x >= m && p.y >= m && p.x <= this.cssW - m && p.y <= this.cssH - m) return;
    this.touched = true;
    const c = this.camera.clampCentre(h.anchor.x, h.anchor.y + h.rect.h / 2);
    this.camera.state.x = c.x;
    this.camera.state.y = c.y;
  }

  // ------------------------------------------------------------------ overlays and walks

  setOverlay(o: TownMapOverlay) {
    const moved = o.here !== this.overlay.here;
    this.overlay = o;
    if (moved && !this.walk) {
      this.youKey = o.here;
      this.youAt = null;
    }
    this.requestDraw();
  }

  private herePoint(): Point | null {
    if (this.youAt) return this.youAt;
    const h = this.youKey ? this.placed.get(this.youKey)?.h : null;
    return h ? cellPoint(this.scene.map, h.cell) : null;
  }

  /** Show the player walking to a hotspot (presentation only: the engine moves them). */
  walkTo(h: MapHotspot, maxMs = WALK_MAX_MS) {
    this.keep = h.key;
    const from = this.herePoint();
    const start = from ? Math.floor(from.y) * this.scene.map.width + Math.floor(from.x) : null;
    const route = start !== null && start !== h.cell ? this.routes.route(start, h.cell) : null;
    if (!route || route.length < 2 || this.opts.reducedMotion) {
      this.walk = null;
      this.youAt = cellPoint(this.scene.map, h.cell);
      this.youKey = h.key;
      this.requestDraw();
      return;
    }
    const ms = Math.min(maxMs, Math.max(WALK_MIN_MS, routeLength(route) * 9));
    this.walk = { route, start: performance.now(), ms, to: h.key };
    this.requestDraw();
  }

  /** Show the light of another time of day until the game's next update (previews and screenshots). */
  previewMinutes(minutes: number | null) {
    this.overlay = { ...this.overlay, minutes };
    this.requestDraw();
  }

  /** Whether a walk is being shown (tests). */
  get walking(): boolean {
    return this.walk !== null;
  }

  // ------------------------------------------------------------------ elements

  /** Pick up the buttons and chips the app rendered (after every render). */
  syncElements() {
    const boxes = new Map<string, HTMLElement>();
    const chips = new Map<string, HTMLElement>();
    for (const el of this.layer.querySelectorAll<HTMLElement>("[data-hotspot]")) boxes.set(el.dataset.hotspot as string, el);
    for (const el of this.layer.querySelectorAll<HTMLElement>("[data-chip]")) chips.set(el.dataset.chip as string, el);
    this.placed.clear();
    for (const h of this.scene.hotspots) {
      const chip = chips.get(h.key) ?? null;
      this.placed.set(h.key, { h, box: boxes.get(h.key) ?? null, chip, cw: chip?.offsetWidth ?? 0, ch: chip?.offsetHeight ?? 0, chipRect: null });
    }
    if (this.youKey === null) this.youKey = this.overlay.here;
    this.requestDraw();
  }

  // ------------------------------------------------------------------ input

  private local(e: { clientX: number; clientY: number }): Point {
    const r = this.container.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }

  private bindInput() {
    const el = this.container;
    const on = <K extends keyof HTMLElementEventMap>(type: K, fn: (e: HTMLElementEventMap[K]) => void, opts?: AddEventListenerOptions) => {
      el.addEventListener(type, fn as EventListener, opts);
      this.cleanup.push(() => el.removeEventListener(type, fn as EventListener, opts));
    };
    on("pointerdown", (e) => {
      if (e.button !== 0 && e.pointerType === "mouse") return;
      this.keyboard = false;
      // The zoom buttons over the map are ordinary buttons, not part of the gesture.
      if ((e.target as Element | null)?.closest("[data-map-control]")) return;
      if (!this.camera.gesturing) this.pressTarget = (e.target as Element | null)?.closest<HTMLElement>("[data-hotspot]")?.dataset.hotspot ?? null;
      try {
        el.setPointerCapture(e.pointerId);
      } catch {
        // A synthetic pointer: nothing to capture.
      }
      this.camera.pointerDown({ id: e.pointerId, ...this.local(e) });
      this.requestDraw();
    });
    on("pointermove", (e) => {
      if (this.camera.gesturing) {
        this.camera.pointerMove({ id: e.pointerId, ...this.local(e) });
        this.touched = true;
        this.requestDraw();
      } else if (e.pointerType === "mouse") {
        const h = this.hitTest(this.local(e));
        const k = h?.key ?? null;
        if (k !== this.hover) {
          this.hover = k;
          this.requestDraw();
        }
      }
    });
    on("pointerup", (e) => {
      const tap = this.camera.pointerUp({ id: e.pointerId, ...this.local(e) });
      this.requestDraw();
      if (!tap) return;
      const h = (this.pressTarget ? this.placed.get(this.pressTarget)?.h : null) ?? this.hitTest(tap);
      this.pressTarget = null;
      if (h) {
        this.pointerActivated = performance.now();
        this.opts.onActivate(h);
      }
    });
    on("pointercancel", (e) => {
      this.camera.pointerCancel(e.pointerId);
      this.pressTarget = null;
      this.requestDraw();
    });
    on("pointerleave", () => {
      if (this.hover !== null) {
        this.hover = null;
        this.requestDraw();
      }
    });
    on(
      "wheel",
      (e) => {
        e.preventDefault();
        this.touched = true;
        const dy = e.deltaMode === 1 ? e.deltaY * 16 : e.deltaY;
        this.camera.wheel(this.local(e), dy);
        this.requestDraw();
      },
      { passive: false },
    );
    // Buttons: a click from the keyboard or a screen reader activates; one that follows a pointer press is that press.
    on("click", (e) => {
      const key = (e.target as Element | null)?.closest<HTMLElement>("[data-hotspot]")?.dataset.hotspot;
      if (!key) return;
      e.preventDefault();
      if (performance.now() - this.pointerActivated < 700) return;
      const h = this.placed.get(key)?.h;
      if (h) this.opts.onActivate(h);
    });
    on("focusin", (e) => {
      const key = (e.target as Element | null)?.closest<HTMLElement>("[data-hotspot]")?.dataset.hotspot ?? null;
      this.focused = key;
      const h = key ? this.placed.get(key)?.h : null;
      if (h && this.keyboard) this.reveal(h);
      this.requestDraw();
    });
    on("focusout", () => {
      this.focused = null;
      this.requestDraw();
    });
    on("keydown", (e) => this.onKey(e));
  }

  private onKey(e: KeyboardEvent) {
    this.keyboard = true;
    if (e.altKey || e.ctrlKey || e.metaKey) return;
    if (e.key === "+" || e.key === "=") {
      this.zoomBy(1.5);
      e.preventDefault();
      return;
    }
    if (e.key === "-" || e.key === "_") {
      this.zoomBy(1 / 1.5);
      e.preventDefault();
      return;
    }
    if (e.key === "0") {
      this.fit();
      e.preventDefault();
      return;
    }
    const dir = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[e.key];
    if (!dir) return;
    e.preventDefault();
    const from = this.focused ? this.placed.get(this.focused)?.h : null;
    const next = from ? this.neighbour(from, dir[0] as number, dir[1] as number) : this.scene.hotspots[0];
    const box = next ? this.placed.get(next.key)?.box : null;
    box?.focus();
  }

  /** The nearest hotspot in a direction (world space: the map's layout, not the screen's clutter). */
  neighbour(from: MapHotspot, dx: number, dy: number): MapHotspot | null {
    const a = centre(from);
    let best: MapHotspot | null = null;
    let bestScore = Infinity;
    for (const h of this.scene.hotspots) {
      if (h.key === from.key) continue;
      const b = centre(h);
      const along = (b.x - a.x) * dx + (b.y - a.y) * dy;
      if (along <= 0.5) continue;
      const across = Math.abs((b.x - a.x) * dy) + Math.abs((b.y - a.y) * dx);
      const score = along + across * 2.5;
      if (score < bestScore) {
        bestScore = score;
        best = h;
      }
    }
    return best;
  }

  /** The hotspot at a screen point: a shown name chip, else a building (padded to a finger), nearest first. */
  hitTest(p: Point): MapHotspot | null {
    for (const pl of this.placed.values()) {
      const r = pl.chipRect;
      if (r && p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h) return pl.h;
    }
    let best: MapHotspot | null = null;
    let bestD = Infinity;
    for (const pl of this.placed.values()) {
      const r = this.screenRect(pl.h);
      const pad = 4;
      if (p.x < r.x - pad || p.x > r.x + r.w + pad || p.y < r.y - pad || p.y > r.y + r.h + pad) continue;
      const cx = r.x + r.w / 2;
      const cy = r.y + r.h / 2;
      const d = (p.x - cx) ** 2 + (p.y - cy) ** 2;
      if (d < bestD) {
        bestD = d;
        best = pl.h;
      }
    }
    return best;
  }

  /** A hotspot's hit area on screen (CSS px in the container), at least a finger's size. */
  screenRect(h: MapHotspot): { x: number; y: number; w: number; h: number } {
    const z = this.camera.state.zoom;
    const tl = this.camera.worldToScreen({ x: h.rect.x, y: h.rect.y });
    let w = h.rect.w * z;
    let hh = h.rect.h * z;
    let x = tl.x;
    let y = tl.y;
    if (w < MIN_HIT) {
      x -= (MIN_HIT - w) / 2;
      w = MIN_HIT;
    }
    if (hh < MIN_HIT) {
      y -= (MIN_HIT - hh) / 2;
      hh = MIN_HIT;
    }
    return { x, y, w, h: hh };
  }

  /** A hotspot's centre on screen (tests). */
  screenCentre(key: string): Point | null {
    const h = this.placed.get(key)?.h;
    if (!h) return null;
    const r = this.screenRect(h);
    return { x: r.x + r.w / 2, y: r.y + r.h / 2 };
  }

  // ------------------------------------------------------------------ frames

  requestDraw() {
    if (this.destroyed || this.raf) return;
    this.raf = requestAnimationFrame((t) => this.frame(t));
  }

  private frame(now: number) {
    this.raf = 0;
    const dt = this.last ? Math.min(0.1, (now - this.last) / 1000) : 1 / 60;
    this.last = now;
    const easing = this.camera.update(dt);
    const t0 = performance.now();
    const walking = this.stepWalk();
    this.draw();
    this.lastDrawMs = performance.now() - t0;
    this.frames++;
    if (easing || walking || this.camera.gesturing) this.requestDraw();
    else this.last = 0;
  }

  private stepWalk(): boolean {
    const w = this.walk;
    if (!w) return false;
    const t = Math.min(1, (performance.now() - w.start) / w.ms);
    // Ease in and out: a short stroll, not a slide.
    const e = t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2;
    this.youAt = alongRoute(w.route, routeLength(w.route) * e);
    if (t >= 1) {
      this.walk = null;
      this.youKey = w.to;
      return false;
    }
    return true;
  }

  private draw() {
    const o = this.overlay;
    const markers: MapMarker[] = [];
    const home = this.placed.get("home")?.h;
    if (home) markers.push({ kind: "home", at: { x: home.rect.x + home.rect.w / 2, y: home.rect.y } });
    for (const key of o.waiting) {
      const h = this.placed.get(key)?.h;
      if (!h) continue;
      markers.push({ kind: "waiting", at: h.kind === "streets" ? { x: h.anchor.x + 2.2, y: h.anchor.y - 1.2 } : { x: h.rect.x + h.rect.w, y: h.rect.y + 0.3 } });
    }
    const you = this.herePoint();
    if (you) markers.push({ kind: "you", at: you });
    let route: Point[] | null = null;
    if (this.walk) route = this.walk.route;
    else {
      const target = this.focused ?? this.hover;
      const h = target ? this.placed.get(target)?.h : null;
      if (h && you && h.key !== this.youKey) route = this.routes.route(Math.floor(you.y) * this.scene.map.width + Math.floor(you.x), h.cell);
    }
    const s = this.camera.state;
    drawTownMapFrame(this.ctx, this.baked, {
      cam: { x: s.x, y: s.y, zoom: s.zoom },
      width: this.cssW,
      height: this.cssH,
      scale: this.scale,
      minutes: o.minutes,
      gradeStrength: this.opts.gradeStrength ?? 0.5,
      route,
      walked: null,
      markers,
    });
    this.placeElements();
  }

  /**
   * Put every button over its building and show as many name chips as fit
   * without overlapping, most important first: where you are, home, places
   * with something waiting, the neighbourhoods, civic landmarks, the rest.
   * The focused or hovered place is always named.
   */
  private placeElements() {
    const z = this.camera.state.zoom;
    // The zoom buttons' corner is taken.
    const shown: { x: number; y: number; w: number; h: number }[] = [{ x: this.cssW - 56, y: this.cssH - 152, w: 56, h: 152 }];
    const rank = (pl: Placed): number => {
      const h = pl.h;
      if (h.key === this.focused || h.key === this.hover) return 0;
      if (h.key === this.overlay.here) return 1;
      if (h.kind === "home") return 2;
      if (this.overlay.waiting.has(h.key)) return 3;
      if (h.kind === "streets") return 4;
      if (h.landmark) return 5;
      return h.active ? 6 : 7;
    };
    const order = [...this.placed.values()].sort((a, b) => rank(a) - rank(b) || a.h.order - b.h.order);
    for (const pl of order) {
      const h = pl.h;
      const r = this.screenRect(h);
      if (pl.box) {
        const st = pl.box.style;
        st.transform = `translate(${Math.round(r.x)}px, ${Math.round(r.y)}px)`;
        st.width = `${Math.round(r.w)}px`;
        st.height = `${Math.round(r.h)}px`;
      }
      pl.chipRect = null;
      if (!pl.chip) continue;
      const a = this.camera.worldToScreen(h.kind === "place" ? { x: h.anchor.x, y: h.anchor.y - (GLYPH_RISE[h.kindKey] ?? 0) } : h.anchor);
      const k = rank(pl);
      // Quiet places are named only when zoomed in (or focused); the neighbourhoods only when zoomed out a little.
      let want = k <= 3 || (k === 7 ? z >= 9 : true);
      if (h.kind === "streets" && z > 18 && k > 3) want = false;
      const cw = pl.cw || 60;
      const ch = pl.ch || 20;
      const onScreen = a.x > -10 && a.y > -10 && a.x < this.cssW + 10 && a.y < this.cssH + 10;
      // Above the building, else below it (a neighbourhood's name sits on its centre).
      const bottom = this.camera.worldToScreen({ x: h.anchor.x, y: h.rect.y + h.rect.h }).y;
      const tries = h.kind === "streets" ? [a.y - ch / 2] : [a.y - ch - 3, bottom + 3];
      let x = Math.min(this.cssW - cw - 2, Math.max(2, a.x - cw / 2));
      let y = 0;
      let show = false;
      for (const ty of tries) {
        y = Math.min(this.cssH - ch - 2, Math.max(2, ty));
        const box = { x: x - 2, y: y - 1, w: cw + 4, h: ch + 2 };
        const free = !shown.some((q) => box.x < q.x + q.w && box.x + box.w > q.x && box.y < q.y + q.h && box.y + box.h > q.y);
        if (onScreen && want && (k === 0 || free)) {
          shown.push(box);
          show = true;
          break;
        }
      }
      if (!show) y = Math.min(this.cssH - ch - 2, Math.max(2, tries[0] as number));
      x = Math.round(x);
      if (show) pl.chipRect = { x, y, w: cw, h: ch };
      const st = pl.chip.style;
      st.transform = `translate(${Math.round(x)}px, ${Math.round(y)}px)`;
      if (show !== (pl.chip.dataset.shown === "1")) pl.chip.dataset.shown = show ? "1" : "0";
    }
  }

  destroy() {
    this.destroyed = true;
    cancelAnimationFrame(this.raf);
    for (const f of this.cleanup) f();
  }
}

const centre = (h: MapHotspot): Point => ({ x: h.rect.x + h.rect.w / 2, y: h.rect.y + h.rect.h / 2 });
