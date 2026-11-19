// Canvas 2D town renderer (DEC-080). Static layers are pre-rendered in
// chunks of the map, baked lazily when first seen and kept in a small LRU:
//   - base: the ground, buildings and props (baked once);
//   - shadow: building shadows for one sun-angle bucket (./dayCycle), at half
//     resolution, re-baked only when the bucket changes.
// Each drawn frame is one blit per visible chunk, the shadows at the sun's
// strength, the walkers (y-sorted) with the roofs and canopies they walk
// behind drawn again over them, the day's grade as three full-view
// composites (saturation, multiply, soft light), then lit windows, lamp
// pools and the "you are here" pin over the grade. A frame identical to the
// last is skipped, so an idle town costs nothing. Text is never drawn here:
// labels are HTML over the canvas (./hubView).
import { gradeAt, lightLevel, shadowBucket, shadowVector, sunAt, type Rgb } from "../dayCycle";
import { noise01 } from "../rng";
import { ROOF_ROWS, type TownBuilding, type TownMap, type TownProp } from "../townLayout";
import { domCanvasFactory, type CanvasFactory, type Ctx, type Layer, type Sprite, type TownArt } from "./art";

export interface WalkerView {
  /** Feet, tiles. */
  x: number;
  y: number;
  dir: number;
  /** 0 standing, 1..art.walkFrames stepping. */
  frame: number;
  /** Appearance; < 0 is the player. */
  variant: number;
}

export interface FrameState {
  /** View size, backing px. */
  width: number;
  height: number;
  /** Art px at the centre of the view. */
  camX: number;
  camY: number;
  /** Backing px per art px (whole numbers keep the pixels even). */
  zoom: number;
  /** Clock, minutes after midnight (./dayCycle). */
  minutes: number;
  /** Everyone to draw, the player included. */
  walkers: readonly WalkerView[];
  /** The player's feet, tiles (the "you are here" marker), or null. */
  player: { x: number; y: number } | null;
  /** Where the player is walking to. */
  target: { cell: number; building: number | null } | null;
  /** Seconds (the target marker's bob; ignored with reduced motion). */
  time: number;
}

export interface RenderStats {
  /** Frames drawn. */
  frames: number;
  /** Frames skipped because nothing visible changed. */
  skipped: number;
  baseBakes: number;
  shadowBakes: number;
  /** Chunks alive now. */
  chunks: number;
  /** Time of the last drawn frame, ms. */
  lastFrameMs: number;
}

export interface RendererOptions {
  createCanvas?: CanvasFactory;
  /** Chunk size, tiles (default 32: 512 art px). */
  chunkTiles?: number;
  /** Chunks kept baked (default 24). */
  maxChunks?: number;
  reducedMotion?: boolean;
  /** Clock for frame timings (default performance.now). */
  now?: () => number;
}

interface Static {
  kind: "b" | "p";
  index: number;
  /** Sort key: the bottom of its footprint, art px. */
  base: number;
  /** Conservative world rect of what it draws, art px. */
  l: number;
  t: number;
  r: number;
  b: number;
  /** Anchor, art px. */
  x: number;
  y: number;
}

interface Chunk {
  cx: number;
  cy: number;
  base: Layer | null;
  shadow: Layer | null;
  shadowBucket: number;
  used: number;
}

const css = (c: Rgb) => `rgb(${Math.round(c[0])},${Math.round(c[1])},${Math.round(c[2])})`;
const q = (v: number, step: number) => Math.round(v / step);

export class TownRenderer {
  readonly stats: RenderStats = { frames: 0, skipped: 0, baseBakes: 0, shadowBakes: 0, chunks: 0, lastFrameMs: 0 };
  private readonly T: number;
  private readonly CP: number;
  private readonly nx: number;
  private readonly ny: number;
  private readonly make: CanvasFactory;
  private readonly maxChunks: number;
  private readonly reducedMotion: boolean;
  private readonly now: () => number;
  private chunks = new Map<number, Chunk>();
  private statics: Static[] = [];
  /** Statics whose overhang (roof rows, canopies) covers a cell. */
  private overhang = new Map<number, number[]>();
  private lamps: { p: TownProp; light: { x: number; y: number }; threshold: number }[] = [];
  private windowThreshold: number[];
  private lastKey: number[] = [];
  private frameNo = 0;
  private scratch: Layer | null = null;
  private sortBuf: WalkerView[] = [];

  constructor(
    private readonly ctx: Ctx,
    readonly map: TownMap,
    readonly art: TownArt,
    options: RendererOptions = {},
  ) {
    this.T = art.tile;
    this.CP = (options.chunkTiles ?? 32) * this.T;
    this.nx = Math.ceil((map.width * this.T) / this.CP);
    this.ny = Math.ceil((map.height * this.T) / this.CP);
    this.make = options.createCanvas ?? domCanvasFactory;
    this.maxChunks = Math.max(4, options.maxChunks ?? 24);
    this.reducedMotion = options.reducedMotion ?? false;
    this.now = options.now ?? (() => performance.now());
    this.indexStatics();
    // Each building lights up at its own point of the evening; shops and
    // named places early, warehouses and sheds late (some never).
    this.windowThreshold = map.buildings.map((b) => {
      const n = noise01(b.style, 99);
      if (b.kind === "warehouse" || b.kind === "shed" || b.kind === "factory") return 0.7 + n * 0.4;
      if (b.place !== null || b.kind === "shop") return 0.1 + n * 0.3;
      return 0.15 + n * 0.65;
    });
  }

  private indexStatics() {
    const T = this.T;
    const map = this.map;
    for (const b of map.buildings) {
      this.statics.push({ kind: "b", index: b.index, base: (b.y + b.h) * T, l: b.x * T, t: (b.y - ROOF_ROWS) * T, r: (b.x + b.w) * T, b: (b.y + b.h) * T, x: b.x * T, y: b.y * T });
    }
    map.props.forEach((p, index) => {
      const s = this.art.prop(map, p);
      const x = p.x * T;
      const y = p.y * T;
      this.statics.push({ kind: "p", index, base: y + Math.max(T, s.h - s.ay), l: x - s.ax, t: y - s.ay, r: x - s.ax + s.w, b: y - s.ay + s.h, x, y });
      const light = this.art.lampLight(p);
      if (light) this.lamps.push({ p, light, threshold: 0.32 + noise01(p.x, p.y, 7) * 0.16 });
    });
    this.statics.sort((a, b) => a.base - b.base || a.l - b.l || (a.kind === b.kind ? a.index - b.index : a.kind === "b" ? -1 : 1));
    // Overhang cells: rows above a building's footprint, and above a blocking prop's cell.
    this.statics.forEach((s, si) => {
      let rows: [number, number] | null = null;
      if (s.kind === "b") rows = [Math.floor(s.t / T), Math.floor(s.y / T) - 1];
      else if ((map.props[s.index] as TownProp).blocks && s.t < s.y) rows = [Math.floor(s.t / T), Math.floor(s.y / T) - 1];
      if (!rows) return;
      for (let cy = rows[0]; cy <= rows[1]; cy++) {
        for (let cx = Math.floor(s.l / T); cx < Math.ceil(s.r / T); cx++) {
          if (cx < 0 || cy < 0 || cx >= map.width || cy >= map.height) continue;
          const c = cy * map.width + cx;
          const list = this.overhang.get(c);
          if (list) list.push(si);
          else this.overhang.set(c, [si]);
        }
      }
    });
  }

  private spriteOf(s: Static): Sprite {
    return s.kind === "b" ? this.art.building(this.map, this.map.buildings[s.index] as TownBuilding) : this.art.prop(this.map, this.map.props[s.index] as TownProp);
  }

  /** Forget the last frame, so the next `draw` draws (after a resize, or new art). */
  invalidate() {
    this.lastKey = [];
  }

  /** Free every baked layer. */
  dispose() {
    for (const c of this.chunks.values()) this.freeChunk(c);
    this.chunks.clear();
    this.stats.chunks = 0;
  }

  private freeChunk(c: Chunk) {
    for (const l of [c.base, c.shadow]) {
      if (!l) continue;
      l.canvas.width = 0;
      l.canvas.height = 0;
    }
    c.base = null;
    c.shadow = null;
  }

  private chunk(cx: number, cy: number): Chunk {
    const k = cy * this.nx + cx;
    let c = this.chunks.get(k);
    if (!c) {
      c = { cx, cy, base: null, shadow: null, shadowBucket: -2, used: 0 };
      this.chunks.set(k, c);
    }
    c.used = this.frameNo;
    return c;
  }

  private evict() {
    if (this.chunks.size <= this.maxChunks) return;
    const old = [...this.chunks.entries()].filter(([, c]) => c.used !== this.frameNo).sort((a, b) => a[1].used - b[1].used);
    for (const [k, c] of old) {
      if (this.chunks.size <= this.maxChunks) break;
      this.freeChunk(c);
      this.chunks.delete(k);
    }
  }

  private bakeBase(c: Chunk): Layer {
    const { CP, T, map } = this;
    const layer = this.make(CP, CP);
    const g = layer.ctx;
    g.imageSmoothingEnabled = false;
    const x0 = c.cx * CP;
    const y0 = c.cy * CP;
    g.setTransform(1, 0, 0, 1, -x0, -y0);
    g.fillStyle = this.art.outside;
    g.fillRect(x0, y0, CP, CP);
    const tx0 = Math.floor(x0 / T);
    const ty0 = Math.floor(y0 / T);
    const tx1 = Math.min(map.width, Math.ceil((x0 + CP) / T));
    const ty1 = Math.min(map.height, Math.ceil((y0 + CP) / T));
    for (let y = ty0; y < ty1; y++) for (let x = tx0; x < tx1; x++) this.art.ground(g, map, x, y, x * T, y * T);
    for (const s of this.statics) {
      if (s.r <= x0 || s.l >= x0 + CP || s.b <= y0 || s.t >= y0 + CP) continue;
      const sp = this.spriteOf(s);
      g.drawImage(sp.image, sp.sx, sp.sy, sp.w, sp.h, s.x - sp.ax, s.y - sp.ay, sp.w, sp.h);
    }
    g.setTransform(1, 0, 0, 1, 0, 0);
    this.stats.baseBakes++;
    return layer;
  }

  private bakeShadow(c: Chunk, bucket: number): Layer {
    const { CP, T, map } = this;
    const half = CP / 2;
    const layer = c.shadow ?? this.make(half, half);
    const g = layer.ctx;
    g.setTransform(1, 0, 0, 1, 0, 0);
    g.globalCompositeOperation = "source-over";
    g.clearRect(0, 0, half, half);
    const x0 = c.cx * CP;
    const y0 = c.cy * CP;
    g.setTransform(0.5, 0, 0, 0.5, -x0 / 2, -y0 / 2);
    const v = shadowVector(bucket);
    const reach = 4 * T * 3;
    const even = (n: number) => Math.round(n / 2) * 2;
    g.beginPath();
    const sweep = (x: number, y: number, w: number, h: number, height: number) => {
      const vx = v.dx * height * T;
      const vy = v.dy * height * T;
      const steps = Math.max(1, Math.ceil(Math.max(Math.abs(vx), Math.abs(vy)) / 2));
      for (let k = 0; k <= steps; k++) g.rect(even(x + (vx * k) / steps), even(y + (vy * k) / steps), w, h);
    };
    const near = (s: Static) => !(s.r + reach <= x0 || s.l - reach >= x0 + CP || s.b + reach <= y0 || s.t - reach >= y0 + CP);
    for (const s of this.statics) {
      if (!near(s)) continue;
      if (s.kind === "b") {
        const b = map.buildings[s.index] as TownBuilding;
        sweep(b.x * T, b.y * T, b.w * T, b.h * T, this.art.buildingHeight(b));
      } else {
        const p = map.props[s.index] as TownProp;
        const h = this.art.propHeight(p);
        if (h > 0) sweep(s.x + 3, s.y + 4, T - 6, T - 6, h);
      }
    }
    g.fillStyle = this.art.shadow;
    g.fill();
    // Shadows fall on the ground, not on the buildings and trees that stand in them.
    g.globalCompositeOperation = "destination-out";
    for (const s of this.statics) {
      if (s.r <= x0 || s.l >= x0 + CP || s.b <= y0 || s.t >= y0 + CP) continue;
      if (s.kind === "p" && !(map.props[s.index] as TownProp).blocks) continue;
      const sp = this.spriteOf(s);
      g.drawImage(sp.image, sp.sx, sp.sy, sp.w, sp.h, s.x - sp.ax, s.y - sp.ay, sp.w, sp.h);
    }
    g.globalCompositeOperation = "source-over";
    g.setTransform(1, 0, 0, 1, 0, 0);
    c.shadowBucket = bucket;
    this.stats.shadowBakes++;
    return layer;
  }

  /** Everything that decides what a frame looks like, quantised to what can be seen. */
  private frameKey(f: FrameState, tx: number, ty: number): number[] {
    const g = gradeAt(f.minutes);
    const sun = sunAt(f.minutes);
    const key = [
      f.width,
      f.height,
      tx,
      ty,
      f.zoom,
      q(g.mul[0], 0.5),
      q(g.mul[1], 0.5),
      q(g.mul[2], 0.5),
      q(g.glow[0], 1),
      q(g.glow[1], 1),
      q(g.glow[2], 1),
      q(g.glowAlpha, 1 / 512),
      q(g.sat, 1 / 512),
      q(g.lights, 1 / 512),
      q(sun.shadowAlpha, 1 / 512),
      shadowBucket(f.minutes),
      f.player ? Math.round(f.player.x * this.T * f.zoom) : -1e9,
      f.player ? Math.round(f.player.y * this.T * f.zoom) : -1e9,
      f.target ? f.target.cell : -1,
      f.target && !this.reducedMotion ? q(Math.sin(f.time * 5), 0.25) : 0,
      f.walkers.length,
    ];
    for (const w of f.walkers) key.push(Math.round(w.x * this.T * f.zoom), Math.round(w.y * this.T * f.zoom), w.dir, w.frame, w.variant);
    return key;
  }

  /** Draw a frame. Returns false (and draws nothing) when it would look the same as the last one. */
  draw(f: FrameState): boolean {
    const t0 = this.now();
    const s = f.zoom;
    const tx = Math.round(f.width / 2 - f.camX * s);
    const ty = Math.round(f.height / 2 - f.camY * s);
    const key = this.frameKey(f, tx, ty);
    if (key.length === this.lastKey.length && key.every((v, i) => v === this.lastKey[i])) {
      this.stats.skipped++;
      return false;
    }
    this.lastKey = key;
    this.frameNo++;
    const { ctx, T, CP, map, art } = this;
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.globalAlpha = 1;
    ctx.globalCompositeOperation = "source-over";
    ctx.imageSmoothingEnabled = false;
    ctx.fillStyle = art.outside;
    ctx.fillRect(0, 0, f.width, f.height);
    ctx.setTransform(s, 0, 0, s, tx, ty);

    // The visible world, art px.
    const vx0 = -tx / s;
    const vy0 = -ty / s;
    const vx1 = (f.width - tx) / s;
    const vy1 = (f.height - ty) / s;
    const cx0 = Math.max(0, Math.floor(vx0 / CP));
    const cy0 = Math.max(0, Math.floor(vy0 / CP));
    const cx1 = Math.min(this.nx - 1, Math.floor(vx1 / CP));
    const cy1 = Math.min(this.ny - 1, Math.floor(vy1 / CP));
    const visible: Chunk[] = [];
    for (let cy = cy0; cy <= cy1; cy++) {
      for (let cx = cx0; cx <= cx1; cx++) {
        const c = this.chunk(cx, cy);
        c.base ??= this.bakeBase(c);
        ctx.drawImage(c.base.canvas, cx * CP, cy * CP);
        visible.push(c);
      }
    }

    // Shadows at the sun's strength.
    const sun = sunAt(f.minutes);
    const bucket = shadowBucket(f.minutes);
    if (bucket >= 0 && sun.shadowAlpha > 0.002) {
      ctx.globalAlpha = sun.shadowAlpha;
      for (const c of visible) {
        if (!c.shadow || c.shadowBucket !== bucket) c.shadow = this.bakeShadow(c, bucket);
        ctx.drawImage(c.shadow.canvas, 0, 0, CP / 2, CP / 2, c.cx * CP, c.cy * CP, CP, CP);
      }
      ctx.globalAlpha = 1;
    }

    // Ground markers: where the player is going, and where they stand.
    const bob = this.reducedMotion ? 0 : Math.round(Math.sin(f.time * 5) * 1.5);
    if (f.target) {
      const tc = f.target.cell;
      const gx = (tc % map.width) * T + T / 2;
      const gy = Math.floor(tc / map.width) * T + T / 2;
      if (f.target.building !== null) {
        const b = map.buildings[f.target.building] as TownBuilding;
        this.sprite(art.marker("door"), b.doorX * T + T / 2, (b.doorY + 1) * T + 1);
      } else this.sprite(art.marker("target"), gx, gy + bob);
    }
    if (f.player) this.sprite(art.marker("here"), Math.round(f.player.x * T), Math.round(f.player.y * T) + 1);

    // Walkers, back to front, with soft feet shadows.
    const walkers = this.sortBuf;
    walkers.length = 0;
    for (const w of f.walkers) {
      const wx = w.x * T;
      const wy = w.y * T;
      if (wx < vx0 - T || wx > vx1 + T || wy < vy0 - T || wy > vy1 + 2 * T) continue;
      walkers.push(w);
    }
    walkers.sort((a, b) => a.y - b.y || a.x - b.x);
    ctx.fillStyle = art.shadow;
    ctx.globalAlpha = 0.3;
    for (const w of walkers) ctx.fillRect(Math.round(w.x * T) - 4, Math.round(w.y * T) - 1, 8, 2);
    ctx.globalAlpha = 1;
    for (const w of walkers) this.sprite(art.walker(w.variant, w.dir, w.frame), Math.round(w.x * T), Math.round(w.y * T));

    // Roofs and canopies over the walkers behind them.
    this.drawOverhangs(walkers);

    // The day's grade, over the whole view.
    const g = gradeAt(f.minutes);
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    if (g.sat < 0.995) {
      ctx.globalCompositeOperation = "saturation";
      ctx.globalAlpha = 1 - g.sat;
      ctx.fillStyle = "#808080";
      ctx.fillRect(0, 0, f.width, f.height);
    }
    if (g.mul[0] < 254.5 || g.mul[1] < 254.5 || g.mul[2] < 254.5) {
      ctx.globalCompositeOperation = "multiply";
      ctx.globalAlpha = 1;
      ctx.fillStyle = css(g.mul);
      ctx.fillRect(0, 0, f.width, f.height);
    }
    if (g.glowAlpha > 0.002) {
      ctx.globalCompositeOperation = "soft-light";
      ctx.globalAlpha = g.glowAlpha;
      ctx.fillStyle = css(g.glow);
      ctx.fillRect(0, 0, f.width, f.height);
    }
    ctx.globalCompositeOperation = "source-over";
    ctx.globalAlpha = 1;
    ctx.setTransform(s, 0, 0, s, tx, ty);

    // Lights over the grade: windows (not over walkers in front of them), then lamps.
    if (g.lights > 0.002) {
      for (const b of map.buildings) {
        const l = b.x * T;
        const t = (b.y - ROOF_ROWS) * T;
        if (l > vx1 || l + b.w * T < vx0 || t > vy1 || (b.y + b.h) * T < vy0) continue;
        const lvl = lightLevel(g.lights, this.windowThreshold[b.index] as number);
        if (lvl <= 0.002) continue;
        const sp = art.buildingLights(map, b);
        if (!sp) continue;
        ctx.globalAlpha = lvl;
        this.lightsOverWalkers(sp, b.x * T, b.y * T, walkers);
      }
      ctx.globalCompositeOperation = "lighter";
      const pool = art.lightPool();
      for (const lamp of this.lamps) {
        const lx = lamp.p.x * T + lamp.light.x;
        const ly = lamp.p.y * T + lamp.light.y;
        if (lx < vx0 - pool.w || lx > vx1 + pool.w || ly < vy0 - pool.h || ly > vy1 + pool.h) continue;
        const lvl = lightLevel(g.lights, lamp.threshold, 0.1);
        if (lvl <= 0.002) continue;
        ctx.globalAlpha = lvl * 0.85;
        this.sprite(pool, lx, ly + T);
      }
      ctx.globalCompositeOperation = "source-over";
      for (const lamp of this.lamps) {
        const lvl = lightLevel(g.lights, lamp.threshold, 0.1);
        if (lvl <= 0.002) continue;
        const lx = lamp.p.x * T + lamp.light.x;
        const ly = lamp.p.y * T + lamp.light.y;
        if (lx < vx0 || lx > vx1 || ly < vy0 || ly > vy1) continue;
        ctx.globalAlpha = lvl;
        ctx.fillStyle = "#fff1c4";
        ctx.fillRect(lx - 2, ly - 13, 4, 2);
      }
      ctx.globalAlpha = 1;
    }

    // "You are here", always bright, and the pin over the place being walked to.
    if (f.target && f.target.building !== null) {
      const b = map.buildings[f.target.building] as TownBuilding;
      this.sprite(art.marker("pin"), b.doorX * T + T / 2, (b.y - ROOF_ROWS) * T - 2 + bob);
    }
    if (f.player) this.sprite(art.marker("pin"), Math.round(f.player.x * T), Math.round(f.player.y * T) - 18);

    ctx.setTransform(1, 0, 0, 1, 0, 0);
    this.evict();
    this.stats.chunks = this.chunks.size;
    this.stats.frames++;
    this.stats.lastFrameMs = this.now() - t0;
    return true;
  }

  private sprite(sp: Sprite, x: number, y: number) {
    this.ctx.drawImage(sp.image, sp.sx, sp.sy, sp.w, sp.h, Math.round(x - sp.ax), Math.round(y - sp.ay), sp.w, sp.h);
  }

  private drawOverhangs(walkers: readonly WalkerView[]) {
    if (walkers.length === 0 || this.overhang.size === 0) return;
    const map = this.map;
    const hit = new Set<number>();
    for (const w of walkers) {
      const x0 = Math.floor(w.x - 0.5);
      const x1 = Math.floor(w.x + 0.49);
      const y0 = Math.floor(w.y - 1);
      const y1 = Math.floor(w.y);
      for (let cy = y0; cy <= y1; cy++) {
        for (let cx = x0; cx <= x1; cx++) {
          if (cx < 0 || cy < 0 || cx >= map.width || cy >= map.height) continue;
          for (const si of this.overhang.get(cy * map.width + cx) ?? []) hit.add(si);
        }
      }
    }
    for (const si of [...hit].sort((a, b) => a - b)) {
      const s = this.statics[si] as Static;
      const sp = this.spriteOf(s);
      if (sp.ay <= 0) continue;
      this.ctx.drawImage(sp.image, sp.sx, sp.sy, sp.w, sp.ay, s.x - sp.ax, s.y - sp.ay, sp.w, sp.ay);
    }
  }

  /** A building's lit windows, cut out where a walker stands in front of them. */
  private lightsOverWalkers(sp: Sprite, x: number, y: number, walkers: readonly WalkerView[]) {
    const T = this.T;
    const left = x - sp.ax;
    const top = y - sp.ay;
    const over = walkers.filter((w) => {
      const wx = w.x * T;
      const wy = w.y * T;
      return wx + 8 > left && wx - 8 < left + sp.w && wy > top && wy - 16 < top + sp.h;
    });
    if (over.length === 0) {
      this.ctx.drawImage(sp.image, sp.sx, sp.sy, sp.w, sp.h, left, top, sp.w, sp.h);
      return;
    }
    let s = this.scratch;
    if (!s || s.canvas.width < sp.w || s.canvas.height < sp.h) {
      s = this.make(Math.max(sp.w, s?.canvas.width ?? 0), Math.max(sp.h, s?.canvas.height ?? 0));
      this.scratch = s;
    }
    const g = s.ctx;
    g.setTransform(1, 0, 0, 1, 0, 0);
    g.globalCompositeOperation = "source-over";
    g.clearRect(0, 0, s.canvas.width, s.canvas.height);
    g.drawImage(sp.image, sp.sx, sp.sy, sp.w, sp.h, 0, 0, sp.w, sp.h);
    g.globalCompositeOperation = "destination-out";
    for (const w of over) {
      const ws = this.art.walker(w.variant, w.dir, w.frame);
      g.drawImage(ws.image, ws.sx, ws.sy, ws.w, ws.h, Math.round(w.x * T) - ws.ax - left, Math.round(w.y * T) - ws.ay - top, ws.w, ws.h);
    }
    g.globalCompositeOperation = "source-over";
    this.ctx.drawImage(s.canvas, 0, 0, sp.w, sp.h, left, top, sp.w, sp.h);
  }
}
