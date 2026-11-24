// An interior in a DOM element (DEC-088, DEC-080): one room, single screen,
// drawn on a canvas at device resolution with a whole-pixel zoom that fits
// the room in the element. The room comes from the pack's composer (cached by
// place, recipe version and seed, ./roomCache); its light from the turn's
// slots, as on the street: the static layers are graded once per light step
// with the pack's interior grade (`gradeRoom`, the same maths as the art
// tool's reference renderer) and blitted; people are graded where they stand.
//
// Battery (DEC-080): nothing in a room moves by itself, so the loop runs only
// while the player walks or the light eases, then stops (no ambient frames
// at all, which is cheaper than the street's 15 fps); the next input or a new
// light wakes it. Paused (a sheet over it) it draws nothing.
//
// Input: a tap on someone walks up to them and talks (`onTalk`, the talk
// hook), on a hotspot walks to its stand and opens it (`onHotspot`), on the
// door walks out (`onLeave`), on the floor walks there. Keyboard: arrows and
// WASD walk tile by tile, Enter or Space talks to the person beside you,
// uses the hotspot you stand at or goes out at the door; Escape goes out.
import type { ArtPack, InteriorGrade, InteriorRoom, Look } from "../../../tools/pixelkit/art/mandate/pack";
import { ANCHOR, BLOB, composeCharacter, faceLeft, gradeImage, gradeRoom, interiorGrade, lookFromSeed, placeholderLook, PLAYER_LOOK, sitCut } from "../../../tools/pixelkit/art/mandate/pack";
import type { Facing, HotspotDef } from "../../../tools/pixelkit/art/mandate/interiorsRecipes";
import { crop, type Indexed } from "../../../tools/pixelkit/src/core/image";
import type { Placement } from "./cast";
import { RoomCrowd } from "./crowd";
import { facingOf, InteriorScene, type Tile } from "./interiorScene";
import { bobTags, fillPersonTag, type PersonMarker } from "../hub/hubView";

interface Point {
  x: number;
  y: number;
}

/** Someone on a spot: a person the engine knows, or scenery staff. */
export interface RoomPerson extends Placement {
  /** Name tag text (people only; extras have none). */
  label: string | null;
  /** The tag's marker, as in the street (default `talk`). */
  marker?: PersonMarker;
  /** The tag's accessible name, saying what the marker means (default: the label). */
  tagLabel?: string;
  /** Their look's seed (the street's, so a person looks the same inside). */
  variant: number;
}

/** Someone as drawn: where they are now, and how far they have walked (while they walk). */
type ShownPerson = RoomPerson & { step?: number };

/** What the player stands by (the keyboard hint and the announcement). */
export type Near = { kind: "person"; key: string } | { kind: "hotspot"; id: string } | { kind: "door" } | null;

export interface InteriorViewOptions {
  room: InteriorRoom;
  pack: ArtPack;
  /** The time of day now (minutes after midnight). */
  minutes: number;
  reducedMotion?: boolean;
  /** The canvas's accessible name. */
  label: string;
  /** Hotspots offered now (the engine's place view filtered the recipe's). */
  hotspots: readonly HotspotDef[];
  hotspotLabel: (h: HotspotDef) => string;
  doorLabel: string;
  onTalk: (key: string) => void;
  onHotspot: (h: HotspotDef) => void;
  onLeave: () => void;
  /** The thing the player stands by changed. */
  onNear?: (n: Near) => void;
  /** The keyboard hint for it (shown while the canvas has focus). */
  hint?: (n: NonNullable<Near>) => string | null;
}

const T = 16;
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
/** Light steps (minutes) the static layers are re-graded at while the light eases. */
const LIGHT_STEP = 5;

interface Drawn {
  canvas: HTMLCanvasElement;
  x: number;
  y: number;
  key: number;
}

function canvasOf(w: number, h: number, data: Uint8ClampedArray | Uint8Array): HTMLCanvasElement {
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  const ctx = c.getContext("2d");
  if (ctx) ctx.putImageData(new ImageData(new Uint8ClampedArray(data), w, h), 0, 0);
  return c;
}

const FACE_ROW: Record<Facing, number> = { down: 0, up: 1, right: 2, left: 2 };

export class InteriorView {
  readonly scene: InteriorScene;
  /** People walking in from the door or out through it (`setPeople`). */
  readonly crowd: RoomCrowd;
  readonly room: InteriorRoom;
  readonly canvas: HTMLCanvasElement;
  /** Frames drawn and loop iterations (tests and measurements). */
  readonly stats = { frames: 0, ticks: 0, grades: 0 };
  private readonly opts: InteriorViewOptions;
  private readonly ctx: CanvasRenderingContext2D;
  private readonly layer: HTMLDivElement;
  private readonly hintEl: HTMLDivElement;
  private people: RoomPerson[] = [];
  private readonly tags = new Map<string, HTMLButtonElement>();
  private readonly spotTags = new Map<string, HTMLButtonElement>();
  private doorTag: HTMLButtonElement | null = null;
  private readonly labelSize = new WeakMap<HTMLElement, { w: number; h: number }>();
  private sheets = new Map<string, Indexed[][]>();
  private graded: { q: number; base: HTMLCanvasElement; sprites: { img: HTMLCanvasElement; front: HTMLCanvasElement | null }[]; g: InteriorGrade } | null = null;
  private actorCache = new Map<string, { sig: string; c: HTMLCanvasElement }>();
  private minutes: number;
  private ease: { from: number; to: number; t0: number; ms: number } | null = null;
  private raf = 0;
  private last = 0;
  private dirty = true;
  private pausedFlag = false;
  private destroyed = false;
  private focused = false;
  private near: Near = null;
  private nearSig = "";
  /** The conversation framed in the room (dialogue mode): with whom, and how much of the view the box covers (CSS px). */
  private talk: { key: string; inset: number } | null = null;
  private speaker: "player" | "person" | null = null;
  private bob = 0;
  private readonly frameListeners = new Set<() => void>();
  private zoom = 1;
  private ox = 0;
  private oy = 0;
  private dpr = 1;
  private cssW = 1;
  private cssH = 1;
  private readonly cleanup: (() => void)[] = [];
  private readonly held = new Map<string, readonly [number, number]>();
  private run = false;

  constructor(
    readonly container: HTMLElement,
    options: InteriorViewOptions,
  ) {
    this.opts = options;
    this.room = options.room;
    this.minutes = options.minutes;
    this.scene = new InteriorScene({ cols: this.room.cols, rows: this.room.rows, walk: this.room.walk, door: this.room.door });
    this.crowd = new RoomCrowd({ cols: this.room.cols, rows: this.room.rows, walk: this.room.walk, door: this.room.door });
    container.style.overflow = "hidden";
    container.style.touchAction = "none";
    container.style.userSelect = "none";
    this.canvas = document.createElement("canvas");
    this.canvas.tabIndex = 0;
    this.canvas.setAttribute("role", "application");
    this.canvas.setAttribute("aria-label", options.label);
    Object.assign(this.canvas.style, { position: "absolute", inset: "0", width: "100%", height: "100%", display: "block", imageRendering: "pixelated" });
    container.prepend(this.canvas);
    const ctx = this.canvas.getContext("2d", { alpha: false });
    if (!ctx) throw new Error("no 2D canvas");
    this.ctx = ctx;
    this.layer = document.createElement("div");
    this.layer.className = "interior-labels";
    Object.assign(this.layer.style, { position: "absolute", inset: "0", pointerEvents: "none", overflow: "hidden" });
    container.appendChild(this.layer);
    this.hintEl = document.createElement("div");
    this.hintEl.className = "hub-door-hint interior-hint";
    this.hintEl.setAttribute("aria-hidden", "true");
    Object.assign(this.hintEl.style, { position: "absolute", left: "0", top: "0", display: "none", whiteSpace: "nowrap" });
    this.layer.appendChild(this.hintEl);
    this.makeSpotTags();
    this.bindInput();
    this.resize();
    const ro = new ResizeObserver(() => this.resize());
    ro.observe(container);
    this.cleanup.push(() => ro.disconnect());
    const vis = () => (document.hidden ? this.stop() : this.wake());
    document.addEventListener("visibilitychange", vis);
    this.cleanup.push(() => document.removeEventListener("visibilitychange", vis));
    this.draw();
  }

  // ------------------------------------------------------------------ public

  /** Whether the loop has stopped (nothing moving): an idle room draws nothing. */
  get asleep(): boolean {
    return this.raf === 0;
  }

  get paused(): boolean {
    return this.pausedFlag;
  }
  set paused(v: boolean) {
    this.pausedFlag = v;
    if (v) {
      this.held.clear();
      this.scene.steer(0, 0);
      this.stop();
    } else this.wake();
  }

  /** The people present on their spots (and scenery staff). Their tiles block walking. */
  setPeople(people: readonly RoomPerson[], animate = false) {
    // Time passed: newcomers walk in from the door and those gone walk out (never with reduced motion, nor
    // behind a sheet, where nothing animates: they are simply there).
    if (animate && !this.opts.reducedMotion && !this.pausedFlag) this.crowd.sync(this.people, people);
    else this.crowd.clear();
    this.people = [...people];
    this.scene.setOccupied(people.map((p) => ({ x: p.x, y: p.y })));
    for (const [k, el] of this.tags) {
      if (!people.some((p) => p.key === k && p.label)) {
        el.remove();
        this.tags.delete(k);
      }
    }
    for (const p of people) {
      if (!p.label || p.extra) continue;
      let el = this.tags.get(p.key);
      if (!el) {
        el = document.createElement("button");
        el.type = "button";
        el.className = "hub-label hub-person-tag";
        // Out of the Tab order (the place panel's People list is the keyboard route, DEC-051), but
        // named for screen readers, with what the marker means.
        el.tabIndex = -1;
        el.dataset.person = p.key;
        Object.assign(el.style, { position: "absolute", left: "0", top: "0", pointerEvents: "auto", whiteSpace: "nowrap" });
        const key = p.key;
        el.addEventListener("click", (e) => {
          e.stopPropagation();
          this.walkToPerson(key);
        });
        el.addEventListener("pointerdown", (e) => e.stopPropagation());
        this.layer.appendChild(el);
        this.tags.set(p.key, el);
      }
      if (fillPersonTag(el, p.label, p.marker, p.tagLabel)) this.labelSize.delete(el);
    }
    this.actorCache.clear();
    this.invalidate();
  }

  /** Everyone drawn now: those present (a newcomer on their way in is on the walk), and those walking out. */
  private shown(): ShownPerson[] {
    const out: ShownPerson[] = this.people.map((p) => {
      const w = this.crowd.walkers.get(p.key);
      return w ? { ...p, x: w.x, y: w.y, facing: w.dir, pose: "stand", step: w.dist } : p;
    });
    for (const w of this.crowd.walkers.values()) if (w.leaving) out.push({ ...(w.person as RoomPerson), x: w.x, y: w.y, facing: w.dir, pose: "stand", step: w.dist });
    return out;
  }

  /** The hotspots offered now (the engine's view changed: a new slot, late at night). */
  setHotspots(hotspots: readonly HotspotDef[]) {
    this.opts.hotspots = hotspots;
    this.makeSpotTags();
    this.invalidate();
  }

  /** The time of day: eased over `ms` (0: at once). */
  setMinutes(minutes: number, ms = 0) {
    if (Math.abs(minutes - this.minutes) < 0.5 && !this.ease) return;
    if (ms <= 0 || this.opts.reducedMotion) {
      this.ease = null;
      this.minutes = minutes;
    } else this.ease = { from: this.minutes, to: minutes, t0: performance.now(), ms };
    this.invalidate();
  }

  /** Walk up to someone present, then talk (`onTalk`). Scenery staff are not people to talk to. */
  walkToPerson(key: string): boolean {
    const p = this.people.find((x) => x.key === key && !x.extra);
    if (!p) return false;
    const ok = this.scene.goNear({ x: p.x, y: p.y }, () => this.opts.onTalk(key));
    this.wake();
    return ok;
  }

  /** Walk to a hotspot's stand, face it, then open it (`onHotspot`). */
  walkToHotspot(id: string): boolean {
    const h = this.opts.hotspots.find((x) => x.id === id);
    if (!h) return false;
    const ok = this.scene.goTo(h.stand, () => this.opts.onHotspot(h), h.facing);
    this.wake();
    return ok;
  }

  /** Walk to the door and out (`onLeave`). */
  walkOut(): boolean {
    const ok = this.scene.goTo(this.room.door, () => this.opts.onLeave(), "down");
    this.wake();
    return ok;
  }

  // ------------------------------------------------------------------ dialogue mode (DEC-093)
  // The room is a talk stage (../components/talk/stage, registered by ./Interior): a conversation with
  // someone beside the player is framed here, above the dialogue box, with the speech bubbles on them.

  /** Whether a conversation with `key` can be framed in the room now: they are here, beside the player, in sight. */
  canTalk(key: string): boolean {
    if (this.pausedFlag || this.destroyed || this.scene.moving) return false;
    const p = this.people.find((x) => x.key === key && !x.extra);
    const t = this.scene.tile;
    return !!p && Math.abs(p.x - t.x) + Math.abs(p.y - t.y) <= 2;
  }

  /**
   * Frame a conversation with someone present: they and the player turn to
   * each other, the room moves up so both stand clear of the dialogue box
   * (`insetBottom` CSS px over the bottom of the view), tags hide, walking stops.
   */
  beginTalk(key: string, insetBottom = 0): boolean {
    const p = this.people.find((x) => x.key === key && !x.extra);
    if (!p) return false;
    this.held.clear();
    this.scene.steer(0, 0);
    this.talk = { key, inset: Math.max(0, insetBottom) };
    const t = this.scene.tile;
    this.scene.player.dir = facingOf(p.x - t.x, p.y - t.y);
    this.layer.style.visibility = "hidden";
    this.layout();
    return true;
  }

  /** The dialogue box covers this much of the view now (CSS px from the bottom). */
  setTalkInset(px: number) {
    if (!this.talk || Math.abs(this.talk.inset - px) < 1) return;
    this.talk.inset = Math.max(0, px);
    this.layout();
  }

  /** Who is speaking (a reply being revealed): they bob a little (not with reduced motion). */
  setSpeaking(who: "player" | "person" | null) {
    if (this.speaker === who) return;
    this.speaker = who;
    this.invalidate();
  }

  /** The conversation ended: the room settles back, tags return. */
  endTalk() {
    if (!this.talk) return;
    this.talk = null;
    this.speaker = null;
    this.layer.style.visibility = "";
    this.layout();
  }

  /** The person a conversation is framed on, or null. */
  get talking(): string | null {
    return this.talk?.key ?? null;
  }

  /** In a conversation: the top of the speakers' heads (CSS px in the container), and CSS px per tile. */
  talkAnchors(): { player: Point; person: Point; tile: number } | null {
    const t = this.talk;
    const p = t ? this.people.find((x) => x.key === t.key) : undefined;
    const a = this.anchorOf("player");
    const b = p ? this.anchorOf(p.key) : null;
    if (!a || !b) return null;
    return { player: { x: a.x + a.w / 2, y: a.y + a.w * 0.3 }, person: { x: b.x + b.w / 2, y: b.y + b.w * 0.3 }, tile: (T * this.zoom) / this.dpr };
  }

  /** Called after every drawn frame (overlays that follow the room). Returns the unsubscribe. */
  addFrameListener(fn: () => void): () => void {
    this.frameListeners.add(fn);
    return () => this.frameListeners.delete(fn);
  }

  /** Where someone is drawn, CSS px in the container (the head's top, the sprite's box): for bubbles and overlays. */
  anchorOf(key: string): { x: number; y: number; w: number; h: number } | null {
    const t = key === "player" ? { ...this.scene.player, pose: "stand" as const } : this.people.find((p) => p.key === key);
    if (!t) return null;
    const { fx, fy } = this.floor();
    const top = fy + t.y * T + 14 - ANCHOR.y + (t.pose === "sit" ? 5 : 0);
    const a = this.toScreen(fx + t.x * T, top);
    return { x: a.x, y: a.y, w: (T * this.zoom) / this.dpr, h: ((ANCHOR.y + 1) * this.zoom) / this.dpr };
  }

  /** CSS px in the container of a room pixel. */
  toScreen(px: number, py: number): { x: number; y: number } {
    return { x: (this.ox + px * this.zoom) / this.dpr, y: (this.oy + py * this.zoom) / this.dpr };
  }

  /** CSS px of a tile's centre (tests). */
  tileToScreen(t: Tile): { x: number; y: number } {
    const { fx, fy } = this.floor();
    return this.toScreen(fx + t.x * T + 8, fy + t.y * T + 8);
  }

  focus() {
    this.canvas.focus({ preventScroll: true });
  }

  destroy() {
    this.destroyed = true;
    this.stop();
    for (const f of this.cleanup) f();
    this.canvas.remove();
    this.layer.remove();
    this.tags.clear();
    this.spotTags.clear();
  }

  // ------------------------------------------------------------------ input

  private bindInput() {
    const c = this.container;
    const on = <K extends keyof HTMLElementEventMap>(target: HTMLElement | Window, type: K, f: (e: HTMLElementEventMap[K]) => void) => {
      target.addEventListener(type, f as EventListener);
      this.cleanup.push(() => target.removeEventListener(type, f as EventListener));
    };
    let down: { x: number; y: number; id: number } | null = null;
    on(c, "pointerdown", (e) => {
      if (e.button !== 0 && e.pointerType === "mouse") return;
      down = { x: e.clientX, y: e.clientY, id: e.pointerId };
    });
    on(c, "pointerup", (e) => {
      if (!down || down.id !== e.pointerId) return;
      const moved = Math.hypot(e.clientX - down.x, e.clientY - down.y);
      down = null;
      if (moved > 12) return;
      const r = c.getBoundingClientRect();
      this.tap(e.clientX - r.left, e.clientY - r.top);
    });
    on(c, "keydown", (e) => {
      if (this.pausedFlag || e.altKey || e.ctrlKey || e.metaKey || e.target !== this.canvas) return;
      const d = KEY_DIRS[e.code] ?? KEY_DIRS[e.key];
      if (d) {
        e.preventDefault();
        // Down at the door: out to the street.
        if (d[1] === 1 && this.scene.atDoor) {
          if (!e.repeat) this.opts.onLeave();
          return;
        }
        this.held.set(KEY_DIRS[e.code] ? e.code : e.key, d);
        this.run = e.shiftKey;
        this.applyKeys();
        return;
      }
      if (e.key === "Escape") {
        // Not prevented: an open sheet (Escape closes the top one) takes it first; the owner ignores it then.
        this.opts.onLeave();
        return;
      }
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        if (e.repeat) return;
        const n = this.near;
        if (n?.kind === "person") this.opts.onTalk(n.key);
        else if (n?.kind === "hotspot") {
          const h = this.opts.hotspots.find((x) => x.id === n.id);
          if (h) this.opts.onHotspot(h);
        } else if (n?.kind === "door") this.opts.onLeave();
      }
    });
    on(c, "keyup", (e) => {
      const id = KEY_DIRS[e.code] ? e.code : e.key;
      if (this.held.delete(id) || e.key === "Shift") {
        this.run = e.shiftKey && e.key !== "Shift";
        this.applyKeys();
      }
    });
    on(this.canvas, "focus", () => {
      this.focused = true;
      this.updateNear(true);
    });
    on(this.canvas, "blur", () => {
      this.focused = false;
      this.held.clear();
      this.scene.steer(0, 0);
      this.updateNear(true);
    });
  }

  private applyKeys() {
    let dx = 0;
    let dy = 0;
    for (const [x, y] of this.held.values()) {
      dx += x;
      dy += y;
    }
    this.scene.steer(dx, dy, this.run);
    this.wake();
  }

  private tap(sx: number, sy: number) {
    const { fx, fy } = this.floor();
    const px = (sx * this.dpr - this.ox) / this.zoom;
    const py = (sy * this.dpr - this.oy) / this.zoom;
    // Someone: their figure's box (a seated one's upper half).
    for (const p of this.shown().sort((a, b) => b.y - a.y)) {
      if (p.extra) continue;
      const x0 = fx + p.x * T;
      const top = fy + p.y * T + 14 - ANCHOR.y;
      if (px >= x0 + 2 && px < x0 + 14 && py >= top && py <= fy + p.y * T + 15) {
        this.walkToPerson(p.key);
        return;
      }
    }
    const tx = Math.floor((px - fx) / T);
    const ty = Math.floor((py - fy) / T);
    // The door, or the gap in the front wall below it.
    if (tx === this.room.door.x && ty >= this.room.door.y) {
      this.walkOut();
      return;
    }
    const h = this.opts.hotspots.find((x) => tx >= x.x && tx < x.x + x.w && ty >= x.y && ty < x.y + x.h);
    if (h) {
      this.walkToHotspot(h.id);
      return;
    }
    // Things on the back wall (the notice board, the office door) count as the hotspot under them.
    if (ty < 0) {
      const w = this.opts.hotspots.find((x) => x.y === 0 && tx >= x.x && tx < x.x + x.w);
      if (w) {
        this.walkToHotspot(w.id);
        return;
      }
    }
    if (this.scene.inside(tx, ty) && this.scene.passable(tx, ty)) {
      this.scene.goTo({ x: tx, y: ty });
      this.wake();
    }
  }

  // ------------------------------------------------------------------ labels

  private makeSpotTags() {
    for (const el of this.spotTags.values()) el.remove();
    this.spotTags.clear();
    for (const h of this.opts.hotspots) {
      const el = document.createElement("button");
      el.type = "button";
      el.className = "hub-label interior-spot-tag";
      el.tabIndex = -1;
      el.setAttribute("aria-hidden", "true");
      el.dataset.hotspot = h.id;
      Object.assign(el.style, { position: "absolute", left: "0", top: "0", pointerEvents: "auto", whiteSpace: "nowrap" });
      // A small marker; its name shows when the player stands at it (and on hover), so markers never crowd.
      el.title = this.opts.hotspotLabel(h);
      el.dataset.name = this.opts.hotspotLabel(h);
      el.textContent = "•";
      el.addEventListener("click", (e) => {
        e.stopPropagation();
        this.walkToHotspot(h.id);
      });
      el.addEventListener("pointerdown", (e) => e.stopPropagation());
      this.layer.appendChild(el);
      this.spotTags.set(h.id, el);
    }
    if (!this.doorTag) {
      const el = document.createElement("button");
      el.type = "button";
      el.className = "hub-label interior-door-tag";
      el.tabIndex = -1;
      el.setAttribute("aria-hidden", "true");
      el.dataset.door = "1";
      Object.assign(el.style, { position: "absolute", left: "0", top: "0", pointerEvents: "auto", whiteSpace: "nowrap" });
      el.textContent = this.opts.doorLabel;
      el.addEventListener("click", (e) => {
        e.stopPropagation();
        this.walkOut();
      });
      el.addEventListener("pointerdown", (e) => e.stopPropagation());
      this.layer.appendChild(el);
      this.doorTag = el;
    }
  }

  private placeLabels() {
    const { fx, fy } = this.floor();
    type Box = { x: number; y: number; w: number; h: number };
    const hit = (p: Box, q: Box) => p.x < q.x + q.w + 2 && p.x + p.w + 2 > q.x && p.y < q.y + q.h + 2 && p.y + p.h + 2 > q.y;
    const measure = (el: HTMLElement, text: string | null) => {
      let size = this.labelSize.get(el);
      if (!size) {
        if (text !== null) el.textContent = text;
        el.style.display = "";
        size = { w: el.offsetWidth, h: el.offsetHeight };
        this.labelSize.set(el, size);
      }
      return size;
    };
    const at = (el: HTMLElement, x: number, y: number) => {
      el.style.transform = `translate(${Math.round(x)}px, ${Math.round(y)}px) translate(-50%, -100%)`;
    };
    // Kept inside the view (a tag over someone at the wall would be cut off).
    const inX = (x: number, w: number) => Math.min(Math.max(x, w / 2 + 2), this.cssW - w / 2 - 2);
    // What the player stands by is always named. Then people's tags, front rows first; then each hotspot's
    // marker where it is clear, named where the name crowds nothing. Whatever is left out is still tapped
    // directly (a person, the thing on the wall).
    const placed: Box[] = [];
    const people = this.shown()
      .filter((p) => this.tags.has(p.key))
      .map((p) => ({ p, near: this.near?.kind === "person" && this.near.key === p.key }))
      .sort((a, c) => Number(c.near) - Number(a.near) || c.p.y - a.p.y || a.p.x - c.p.x);
    for (const { p, near } of people) {
      const el = this.tags.get(p.key) as HTMLButtonElement;
      const s = this.toScreen(fx + p.x * T + 8, fy + p.y * T + 14 - ANCHOR.y - (p.pose === "sit" ? 4 : 0) - 2);
      const size = measure(el, null);
      const x = inX(s.x, size.w);
      at(el, x, s.y);
      el.classList.toggle("hub-person-near", near);
      const box = { x: x - size.w / 2, y: s.y - size.h, w: size.w, h: size.h };
      const fits = near || !placed.some((q) => hit(box, q));
      el.style.display = fits ? "" : "none";
      if (fits) placed.push(box);
    }
    // The "!" bobs only with frames drawn anyway: an idle room draws nothing, so it stays still.
    bobTags(this.tags.values(), performance.now() / 1000, this.opts.reducedMotion === true);
    const spots = this.opts.hotspots.flatMap((h) => {
      const el = this.spotTags.get(h.id);
      if (!el) return [];
      const s = this.toScreen(fx + (h.x + h.w / 2) * T, fy + h.y * T + (h.y === 0 ? -6 : 2));
      const size = measure(el, el.dataset.name ?? "");
      const nx = inX(s.x, size.w);
      const near = this.near?.kind === "hotspot" && this.near.id === h.id;
      el.classList.toggle("interior-spot-near", near);
      return [{ el, near, at: s, nx, dot: { x: s.x - 9, y: s.y - size.h, w: 18, h: size.h }, name: { x: nx - size.w / 2, y: s.y - size.h, w: size.w, h: size.h } }];
    });
    const state = new Map<HTMLElement, "name" | "dot" | "none">();
    for (const sp of spots) {
      const near = sp.near;
      const shown = near || !placed.some((q) => hit(sp.dot, q));
      state.set(sp.el, near ? "name" : shown ? "dot" : "none");
      if (shown) placed.push(near ? sp.name : sp.dot);
    }
    for (const sp of spots) {
      if (state.get(sp.el) !== "dot") continue;
      const others = placed.filter((q) => q !== sp.dot);
      if (others.some((q) => hit(sp.name, q))) continue;
      placed.splice(placed.indexOf(sp.dot), 1, sp.name);
      state.set(sp.el, "name");
    }
    for (const sp of spots) {
      const st = state.get(sp.el);
      const text = st === "name" ? (sp.el.dataset.name ?? "") : "•";
      if (sp.el.textContent !== text) sp.el.textContent = text;
      at(sp.el, st === "name" ? sp.nx : sp.at.x, sp.at.y);
      sp.el.style.display = st === "none" ? "none" : "";
    }
    if (this.doorTag) {
      const s = this.toScreen(fx + this.room.door.x * T + 8, fy + this.room.rows * T);
      this.doorTag.style.transform = `translate(${Math.round(s.x)}px, ${Math.round(s.y)}px) translate(-50%, 0)`;
    }
    this.placeHint();
  }

  private updateNear(force = false) {
    const n = this.pausedFlag ? null : this.computeNear();
    const sig = n ? `${n.kind}:${n.kind === "person" ? n.key : n.kind === "hotspot" ? n.id : ""}` : "";
    if (sig === this.nearSig && !force) return;
    this.nearSig = sig;
    this.near = n;
    this.opts.onNear?.(n);
    // The hint is for keyboard walkers (a tap's focus shows none).
    const keyboard = this.focused && this.canvas.matches(":focus-visible");
    const text = n && keyboard && this.opts.hint ? this.opts.hint(n) : null;
    this.hintEl.textContent = text ?? "";
    this.hintEl.style.display = text ? "" : "none";
    this.placeLabels();
  }

  private computeNear(): Near {
    if (this.scene.moving) return null;
    const t = this.scene.tile;
    const person = this.people.find((p) => !p.extra && Math.abs(p.x - t.x) + Math.abs(p.y - t.y) === 1);
    if (person) return { kind: "person", key: person.key };
    const h = this.opts.hotspots.find((x) => x.stand.x === t.x && x.stand.y === t.y);
    if (h) return { kind: "hotspot", id: h.id };
    if (this.scene.atDoor) return { kind: "door" };
    return null;
  }

  private placeHint() {
    if (this.hintEl.style.display === "none") return;
    const { fx, fy } = this.floor();
    const p = this.scene.player;
    const s = this.toScreen(fx + p.x * T + 8, fy + p.y * T + 14 - ANCHOR.y - 2);
    this.hintEl.style.transform = `translate(${Math.round(s.x)}px, ${Math.round(s.y)}px) translate(-50%, -100%)`;
  }

  // ------------------------------------------------------------------ loop

  private floor() {
    return { fx: this.room.floor.x, fy: this.room.floor.y };
  }

  private resize() {
    const r = this.container.getBoundingClientRect();
    this.cssW = Math.max(1, Math.round(r.width));
    this.cssH = Math.max(1, Math.round(r.height));
    this.dpr = window.devicePixelRatio || 1;
    this.canvas.width = Math.round(this.cssW * this.dpr);
    this.canvas.height = Math.round(this.cssH * this.dpr);
    // The whole room on screen at a whole number of device pixels per art pixel (at least 1).
    this.zoom = Math.max(1, Math.floor(Math.min(this.canvas.width / this.room.w, this.canvas.height / this.room.h)));
    this.layout();
  }

  /** Place the room in the view: centred; in a conversation, moved up so the speakers stand clear of the dialogue box. */
  private layout() {
    const h = this.room.h * this.zoom;
    this.ox = Math.floor((this.canvas.width - this.room.w * this.zoom) / 2);
    const centred = Math.floor((this.canvas.height - h) / 2);
    this.oy = centred;
    const t = this.talk;
    const p = t ? this.people.find((x) => x.key === t.key) : undefined;
    if (t && p) {
      const free = Math.max(this.canvas.height * 0.3, this.canvas.height - t.inset * this.dpr);
      if (centred + h > free) {
        // The speakers' midpoint (about their heads) at the middle of the room left above the box.
        const mid = this.room.floor.y + ((this.scene.player.y + p.y) / 2) * T - 6;
        this.oy = Math.round(Math.min(centred, Math.max(free - h, free * 0.55 - mid * this.zoom)));
      }
    }
    this.invalidate();
  }

  private invalidate() {
    this.dirty = true;
    // Behind a sheet nothing animates, but a change (a resize, new people) is drawn once so the room is right when it shows.
    if (this.pausedFlag && !this.destroyed) this.draw();
    else this.wake();
  }

  private stop() {
    if (this.raf > 0) cancelAnimationFrame(this.raf);
    this.raf = 0;
  }

  /** Run the loop (something may change). */
  wake() {
    if (this.destroyed || this.pausedFlag || this.raf > 0 || (typeof document !== "undefined" && document.hidden)) return;
    this.last = performance.now();
    this.raf = requestAnimationFrame((t) => this.tick(t));
  }

  private tick(now: number) {
    this.raf = 0;
    this.stats.ticks++;
    const dt = Math.min(0.1, Math.max(0, (now - this.last) / 1000));
    this.last = now;
    const moved = this.scene.step(dt);
    const crowded = this.crowd.step(dt);
    if (this.ease) {
      const k = Math.min(1, (now - this.ease.t0) / this.ease.ms);
      this.minutes = this.ease.from + (this.ease.to - this.ease.from) * (k * k * (3 - 2 * k));
      if (k >= 1) this.ease = null;
      this.dirty = true;
    }
    if (moved || crowded || this.dirty) this.draw();
    this.updateNear();
    if (this.scene.moving || this.scene.steering || this.ease || this.crowd.moving) this.raf = requestAnimationFrame((t) => this.tick(t));
    else if (this.speaker && !this.opts.reducedMotion) {
      // Someone speaking bobs a little: a few frames a second, only while the reply is revealed.
      this.raf = -1;
      window.setTimeout(() => {
        if (this.raf !== -1 || this.destroyed) return;
        this.raf = 0;
        this.bob ^= 1;
        this.dirty = true;
        this.wake();
      }, 180);
    }
  }

  // ------------------------------------------------------------------ drawing

  private grade() {
    const q = Math.round(this.minutes / LIGHT_STEP) * LIGHT_STEP;
    if (this.graded?.q === q) return this.graded;
    const g = interiorGrade(this.room, this.opts.pack.palette, this.opts.pack.lighting, q);
    const r = gradeRoom(this.room, g);
    this.graded = {
      q,
      g,
      base: canvasOf(r.base.w, r.base.h, r.base.data),
      sprites: this.room.sprites.map((s, i) => {
        const gs = r.sprites[i] as { img: Uint8ClampedArray; front: Uint8ClampedArray | null };
        return { img: canvasOf(s.img.w, s.img.h, gs.img), front: s.front && gs.front ? canvasOf(s.front.img.w, s.front.img.h, gs.front) : null };
      }),
    };
    this.actorCache.clear();
    this.stats.grades++;
    return this.graded;
  }

  private sheet(look: Look, id: string): Indexed[][] {
    let s = this.sheets.get(id);
    if (!s) {
      s = composeCharacter(look, this.opts.pack.palette, this.opts.pack.lightness).frames;
      this.sheets.set(id, s);
    }
    return s;
  }

  private frameOf(frames: Indexed[][], dir: Facing, k: number): Indexed {
    const img = frames[FACE_ROW[dir]]?.[k] as Indexed;
    return dir === "left" ? faceLeft(img) : img;
  }

  /** A person as a graded canvas at their place (cached while nothing about them changes). */
  private actor(id: string, img: Indexed, x: number, y: number, sig: string, g: InteriorGrade): HTMLCanvasElement {
    const full = `${sig}|${x}|${y}`;
    const hit = this.actorCache.get(id);
    if (hit && hit.sig === full) return hit.c;
    const c = canvasOf(img.w, img.h, gradeImage(g, img, x, y));
    this.actorCache.set(id, { sig: full, c });
    return c;
  }

  private draw() {
    this.dirty = false;
    const graded = this.grade();
    const ctx = this.ctx;
    const { fx, fy } = this.floor();
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.fillStyle = "#11131a";
    ctx.fillRect(0, 0, this.canvas.width, this.canvas.height);
    ctx.imageSmoothingEnabled = false;
    ctx.setTransform(this.zoom, 0, 0, this.zoom, this.ox, this.oy);
    ctx.drawImage(graded.base, 0, 0);
    const layers: Drawn[] = [];
    this.room.sprites.forEach((s, i) => {
      const gs = graded.sprites[i];
      if (!gs) return;
      layers.push({ canvas: gs.img, x: s.x, y: s.y, key: s.key });
      if (s.front && gs.front) layers.push({ canvas: gs.front, x: s.x, y: s.front.y, key: s.front.key });
    });
    const shadows: { x: number; y: number }[] = [];
    const lift = (x: number, y: number) => this.room.lift[Math.round(y) * this.room.cols + Math.round(x)] ?? 0;
    // The people present.
    for (const p of this.shown()) {
      const look = p.extra || p.role !== "public" && p.role !== "patron" ? placeholderLook(p.role, p.variant) : lookFromSeed(p.variant >>> 0);
      const frames = this.sheet(look, `${p.role}:${p.variant}:${p.extra ? 1 : 0}`);
      const t = this.scene.player;
      const talking = this.talk?.key === p.key;
      const dir = talking || (this.near?.kind === "person" && this.near.key === p.key) ? facingOf(t.x - p.x, t.y - p.y) : p.facing;
      const stride = p.step !== undefined ? 2 + (Math.floor(p.step * 4) % 4) : 0;
      let img = this.frameOf(frames, dir, stride);
      const tileX = p.step !== undefined ? Math.round(fx + p.x * T) : fx + p.x * T;
      const tileY = fy + p.y * T;
      let x: number;
      let y: number;
      if (p.pose === "sit") {
        const seat = this.room.sprites.find((sp) => sp.sitY && p.x >= sp.tiles.x && p.x < sp.tiles.x + sp.tiles.w && p.y >= sp.tiles.y && p.y < sp.tiles.y + sp.tiles.h);
        const hip = seat?.sitY?.[p.y - (seat?.tiles.y ?? p.y)] ?? tileY + 9 - lift(p.x, p.y);
        const cut = sitCut(look);
        img = crop(img, 0, 0, 16, cut);
        x = tileX + (dir === "right" ? 1 : dir === "left" ? -1 : 0);
        y = hip - cut - (talking && this.speaker === "person" ? this.bob : 0);
      } else {
        const feetY = p.step !== undefined ? Math.round(tileY + 14 - lift(p.x, p.y)) : tileY + 14 - lift(p.x, p.y);
        x = tileX;
        y = feetY - ANCHOR.y - (talking && this.speaker === "person" ? this.bob : 0);
        shadows.push({ x: tileX + ANCHOR.x, y: feetY - 1 });
      }
      layers.push({ canvas: this.actor(p.key, img, x, y, `${dir}:${p.pose}:${stride}`, graded.g), x, y, key: (tileY + 14) * 4 });
    }
    // The player.
    {
      const p = this.scene.player;
      const frames = this.sheet(PLAYER_LOOK, "player");
      const walking = this.scene.moving;
      const k = walking ? 2 + (Math.floor(p.dist * 4) % 4) : 0;
      const img = this.frameOf(frames, p.dir, k);
      const x = Math.round(fx + p.x * T);
      const tileY = fy + p.y * T;
      const feetY = Math.round(tileY + 14 - lift(p.x, p.y));
      const y = feetY - ANCHOR.y - (this.talk && this.speaker === "player" ? this.bob : 0);
      shadows.push({ x: x + ANCHOR.x, y: feetY - 1 });
      layers.push({ canvas: this.actor("player", img, x, y, `${p.dir}:${k}`, graded.g), x, y, key: Math.round((tileY + 14) * 4) + 3 });
    }
    // Blob shadows on the floor, under everything drawn over it.
    ctx.fillStyle = "rgba(0, 0, 0, 0.3)";
    for (const s of shadows) for (const [dx, dy] of BLOB) ctx.fillRect(s.x + dx, s.y + dy, 1, 1);
    layers.sort((a, b) => a.key - b.key);
    for (const l of layers) ctx.drawImage(l.canvas, l.x, l.y);
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    this.stats.frames++;
    this.placeLabels();
    for (const f of this.frameListeners) f();
  }
}
