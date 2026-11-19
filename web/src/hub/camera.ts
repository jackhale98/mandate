// Hub camera: pan, pinch zoom and wheel zoom (DEC-080, M6 zoom requirements).
// A plain class over pointer events given as data (screen px relative to the
// view's top-left), so it is testable without a DOM and the renderer only
// reads `state`.
//
// - A pinch zooms about the midpoint between the fingers: the town point
//   under them stays under them (and follows them when they move together).
// - Whenever the set of pointers changes (a finger lands, lifts, or is
//   cancelled), the gesture re-baselines from where the remaining fingers are
//   now, so nothing jumps.
// - Past the zoom limits a gesture meets resistance, and when it ends the
//   zoom eases back inside them (never snapped). The same goes for panning
//   past the town's edges.
// - The wheel zooms about the cursor.

export interface Vec {
  x: number;
  y: number;
}

export interface CameraState {
  /** World point at the centre of the view. */
  x: number;
  y: number;
  /** Screen px per world unit. */
  zoom: number;
}

export interface WorldBounds {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
}

export interface CameraOptions {
  minZoom: number;
  maxZoom: number;
  /** The view eases back over this rect (null: anywhere). */
  bounds?: WorldBounds | null;
  /** Exponential ease rate back inside the limits, per second. */
  easeRate?: number;
  /** Screen px a press may move and still count as a tap. */
  tapSlop?: number;
  /** Resistance past the zoom limits during a gesture: 0 stops dead, 1 is none. */
  overshoot?: number;
  /** Settle on whole-number zoom steps after a gesture (crisp pixel art, DEC-080). */
  integerZoom?: boolean;
  /**
   * The step `integerZoom` settles on (default 1). Use 1 / devicePixelRatio when
   * drawing at device resolution, so every step is a whole number of device pixels.
   */
  zoomStep?: number;
  /** Wheel zoom factor per wheel delta unit. */
  wheelRate?: number;
}

export interface PointerInput {
  id: number;
  x: number;
  y: number;
}

interface Baseline {
  /** World point held under the gesture's focus (a finger, or the pinch midpoint). */
  anchor: Vec;
  /** Finger distance at the baseline (pinch only). */
  dist: number;
  /** Unsoftened zoom at the baseline. */
  raw: number;
}

const SETTLE = 1e-4;

export class HubCamera {
  readonly state: CameraState;
  readonly viewport = { width: 1, height: 1 };
  private opts: Required<Omit<CameraOptions, "bounds">> & { bounds: WorldBounds | null };
  private pointers = new Map<number, Vec>();
  private base: Baseline | null = null;
  private raw: number;
  /** Where the current press started (tap detection). */
  private press: Vec | null = null;
  private dragging = false;
  private multi = false;
  /** Screen point the ease back keeps still (the last gesture focus). */
  private easeFocus: Vec | null = null;
  /** The zoom the camera last settled on: whole steps snap away from it in the direction moved. */
  private rest: number;

  constructor(state: CameraState, viewport: { width: number; height: number }, options: CameraOptions) {
    this.state = { ...state };
    this.resize(viewport.width, viewport.height);
    this.opts = {
      easeRate: 10,
      tapSlop: 8,
      overshoot: 0.35,
      integerZoom: false,
      zoomStep: 1,
      wheelRate: 0.0015,
      ...options,
      bounds: options.bounds ?? null,
    };
    this.raw = this.state.zoom;
    this.rest = this.state.zoom;
  }

  /** Change the zoom limits and step (e.g. when the device pixel ratio changes). */
  setZoomLimits(minZoom: number, maxZoom: number, zoomStep = this.opts.zoomStep) {
    this.opts.minZoom = minZoom;
    this.opts.maxZoom = maxZoom;
    this.opts.zoomStep = zoomStep;
  }

  resize(width: number, height: number) {
    this.viewport.width = Math.max(1, width);
    this.viewport.height = Math.max(1, height);
  }

  setBounds(bounds: WorldBounds | null) {
    this.opts.bounds = bounds;
  }

  get gesturing(): boolean {
    return this.pointers.size > 0;
  }

  screenToWorld(p: Vec): Vec {
    const s = this.state;
    return { x: s.x + (p.x - this.viewport.width / 2) / s.zoom, y: s.y + (p.y - this.viewport.height / 2) / s.zoom };
  }

  worldToScreen(p: Vec): Vec {
    const s = this.state;
    return { x: (p.x - s.x) * s.zoom + this.viewport.width / 2, y: (p.y - s.y) * s.zoom + this.viewport.height / 2 };
  }

  /** Move the camera so that `world` sits under the screen point `screen`. */
  private place(world: Vec, screen: Vec) {
    this.state.x = world.x - (screen.x - this.viewport.width / 2) / this.state.zoom;
    this.state.y = world.y - (screen.y - this.viewport.height / 2) / this.state.zoom;
  }

  /** Zoom with resistance past the limits. */
  private soften(raw: number): number {
    const { minZoom, maxZoom, overshoot } = this.opts;
    if (raw > maxZoom) return maxZoom * Math.pow(raw / maxZoom, overshoot);
    if (raw < minZoom) return minZoom * Math.pow(raw / minZoom, overshoot);
    return raw;
  }

  private unsoften(z: number): number {
    const { minZoom, maxZoom, overshoot } = this.opts;
    if (overshoot <= 0) return z;
    if (z > maxZoom) return maxZoom * Math.pow(z / maxZoom, 1 / overshoot);
    if (z < minZoom) return minZoom * Math.pow(z / minZoom, 1 / overshoot);
    return z;
  }

  /** The two fingers of a pinch (the first two down), or the one finger of a pan. */
  private focus(): { at: Vec; dist: number } | null {
    const pts = [...this.pointers.values()];
    const a = pts[0];
    if (!a) return null;
    const b = pts[1];
    if (!b) return { at: { ...a }, dist: 0 };
    return { at: { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 }, dist: Math.hypot(a.x - b.x, a.y - b.y) };
  }

  /** Start the gesture afresh from the pointers as they are now. */
  private rebaseline() {
    const f = this.focus();
    if (!f) {
      this.base = null;
      return;
    }
    this.raw = this.unsoften(this.state.zoom);
    this.base = { anchor: this.screenToWorld(f.at), dist: f.dist, raw: this.raw };
    this.easeFocus = f.at;
  }

  pointerDown(p: PointerInput) {
    if (this.pointers.size === 0) {
      this.press = { x: p.x, y: p.y };
      this.dragging = false;
      this.multi = false;
    }
    this.pointers.set(p.id, { x: p.x, y: p.y });
    if (this.pointers.size > 1) {
      this.multi = true;
      this.dragging = true;
    }
    this.rebaseline();
  }

  pointerMove(p: PointerInput) {
    const cur = this.pointers.get(p.id);
    if (!cur) return;
    cur.x = p.x;
    cur.y = p.y;
    if (!this.dragging) {
      const press = this.press;
      if (press && Math.hypot(p.x - press.x, p.y - press.y) < this.opts.tapSlop) return;
      this.dragging = true;
    }
    const f = this.focus();
    const base = this.base;
    if (!f || !base) return;
    if (base.dist > 0 && f.dist > 0) {
      this.raw = (base.raw * f.dist) / base.dist;
      this.state.zoom = this.soften(this.raw);
    }
    this.place(base.anchor, f.at);
    this.easeFocus = f.at;
  }

  /** A pointer lifted. Returns the screen point of a tap, if this was one. */
  pointerUp(p: PointerInput): Vec | null {
    if (!this.pointers.has(p.id)) return null;
    this.pointers.delete(p.id);
    const tap = this.pointers.size === 0 && !this.dragging && !this.multi ? { x: p.x, y: p.y } : null;
    this.rebaseline();
    return tap;
  }

  /** A pointer was cancelled (the browser took it over): never a tap. */
  pointerCancel(id: number) {
    if (!this.pointers.delete(id)) return;
    this.dragging = true;
    this.rebaseline();
  }

  /** Wheel zoom about the cursor. */
  wheel(at: Vec, deltaY: number) {
    const anchor = this.screenToWorld(at);
    this.raw = this.unsoften(this.state.zoom) * Math.exp(-deltaY * this.opts.wheelRate);
    this.state.zoom = this.soften(this.raw);
    this.place(anchor, at);
    this.easeFocus = { ...at };
  }

  /** Zoom the camera settles on: inside the limits, on a whole step if asked. */
  settledZoom(z = this.state.zoom): number {
    const { minZoom, maxZoom, integerZoom, zoomStep } = this.opts;
    let t = Math.min(maxZoom, Math.max(minZoom, z));
    if (integerZoom) {
      const step = zoomStep > 0 ? zoomStep : 1;
      const lo = Math.ceil(minZoom / step - 1e-9);
      const hi = Math.floor(maxZoom / step + 1e-9);
      // Snap in the direction of travel once the zoom has moved a little from where
      // it last rested, so a single wheel notch or a small pinch always takes a step.
      const r = t / step;
      const from = this.rest / step;
      const n = r > from + 0.02 ? Math.ceil(r - 1e-9) : r < from - 0.02 ? Math.floor(r + 1e-9) : Math.round(r);
      if (lo <= hi) t = Math.min(hi, Math.max(lo, n)) * step;
    }
    return t;
  }

  /** The view centre nearest (x, y) that keeps the view over the bounds at `zoom`. */
  clampCentre(x: number, y: number, zoom = this.state.zoom): Vec {
    const b = this.opts.bounds;
    if (!b) return { x, y };
    const hw = this.viewport.width / 2 / zoom;
    const hh = this.viewport.height / 2 / zoom;
    const cx = b.maxX - b.minX <= hw * 2 ? (b.minX + b.maxX) / 2 : Math.min(b.maxX - hw, Math.max(b.minX + hw, x));
    const cy = b.maxY - b.minY <= hh * 2 ? (b.minY + b.maxY) / 2 : Math.min(b.maxY - hh, Math.max(b.minY + hh, y));
    return { x: cx, y: cy };
  }

  /**
   * Ease back inside the zoom limits and bounds while no gesture is active.
   * Returns true while still easing (keep drawing frames).
   */
  update(dt: number): boolean {
    if (this.gesturing) return false;
    const k = 1 - Math.exp(-this.opts.easeRate * Math.max(0, dt));
    let moving = false;
    const target = this.settledZoom();
    const dz = target - this.state.zoom;
    if (Math.abs(dz) > SETTLE) {
      const focus = this.easeFocus ?? { x: this.viewport.width / 2, y: this.viewport.height / 2 };
      const anchor = this.screenToWorld(focus);
      // Ease in log space so zooming in and out feel the same.
      this.state.zoom = Math.abs(dz) < SETTLE * 10 ? target : this.state.zoom * Math.pow(target / this.state.zoom, k);
      this.place(anchor, focus);
      moving = true;
    } else if (dz !== 0) {
      this.state.zoom = target;
    }
    if (Math.abs(this.state.zoom - target) <= SETTLE) this.rest = target;
    this.raw = this.state.zoom;
    const c = this.clampCentre(this.state.x, this.state.y);
    const dx = c.x - this.state.x;
    const dy = c.y - this.state.y;
    if (Math.hypot(dx, dy) * this.state.zoom > 0.05) {
      this.state.x += dx * k;
      this.state.y += dy * k;
      moving = true;
    } else {
      this.state.x = c.x;
      this.state.y = c.y;
    }
    return moving;
  }
}
