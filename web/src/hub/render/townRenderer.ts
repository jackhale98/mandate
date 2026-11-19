// Canvas 2D town renderer (DEC-080). Static layers are pre-rendered in
// chunks of the map, baked lazily when first seen and kept in a small LRU:
//   - base: the ground, buildings and props (baked once);
//   - shadow: shadows projected from each caster's footprint and height along
//     the grade's shadow vector, at half resolution, re-baked only when that
//     vector changes (the provider steps it with the sun).
// Each drawn frame is one blit per visible chunk, the shadows at the grade's
// strength, the walkers (y-sorted) with the roofs and canopies they walk
// behind drawn again over them, the grade as three full-view composites
// (saturation, multiply, soft light), then lit windows, doors and lamps and
// their pools over the grade, and the "you are here" pin. A frame identical
// to the last is skipped, so an idle town costs nothing. All colour comes
// from the art provider (./art); text is HTML over the canvas (../hubView).
import { lightLevel } from "../dayCycle";
import { mix, noise01 } from "../rng";
import { ROOF_ROWS, type TownBuilding, type TownMap, type TownProp } from "../townLayout";
import {
  buildingSpec,
  domCanvasFactory,
  groundCells,
  type ArtProvider,
  type BuiltBuilding,
  type CanvasFactory,
  type Ctx,
  type FrameRef,
  type Grade,
  type Layer,
  type LightUse,
  type PropSprite,
  type Rgb,
  type ShadowCaster,
} from "./art";

export interface WalkerView {
  /** Feet, tiles. */
  x: number;
  y: number;
  /** 0 down, 1 up, 2 left, 3 right. */
  dir: number;
  /** 0 standing, 1..art.walkFrames stepping. */
  frame: number;
  /** Appearance seed; < 0 is the player. */
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
  /** Clock, minutes after midnight. */
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
  /** World rect of what it draws, art px. */
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
  shadowKey: string;
  used: number;
}

/** A light: a lit opening of a building, or a lamp head or pool. */
interface Light {
  frame: FrameRef;
  /** Where the frame's anchor goes, art px. */
  x: number;
  y: number;
  /** Switch-on point, 0..1 of the grade's emissive level. */
  threshold: number;
}

/** Switch-on points by use (start, spread): shops and civic early, homes through the evening, industry late. */
const USE_SHIFT: Record<LightUse, [number, number]> = {
  shop: [0.08, 0.25],
  civic: [0.12, 0.25],
  home: [0.15, 0.6],
  office: [0.3, 0.5],
  industrial: [0.7, 0.4],
};

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
  private readonly ground: { mask: Uint8Array; variant: Uint8Array };
  private chunks = new Map<number, Chunk>();
  private statics: Static[] = [];
  private built = new Map<number, BuiltBuilding>();
  private lightCache = new Map<number, Light[]>();
  private propSprites: PropSprite[] = [];
  /** Statics whose overhang (roof rows, canopies) covers a cell. */
  private overhang = new Map<number, number[]>();
  private lamps: { light: Light | null; pool: Light | null }[] = [];
  private lastKey: number[] = [];
  private frameNo = 0;
  private scratch: Layer | null = null;
  private sortBuf: WalkerView[] = [];

  constructor(
    private readonly ctx: Ctx,
    readonly map: TownMap,
    readonly art: ArtProvider,
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
    this.ground = groundCells(map);
    this.indexStatics();
  }

  private indexStatics() {
    const T = this.T;
    const map = this.map;
    for (const b of map.buildings) {
      this.statics.push({ kind: "b", index: b.index, base: (b.y + b.h) * T, l: b.x * T, t: (b.y - ROOF_ROWS) * T, r: (b.x + b.w) * T, b: (b.y + b.h) * T, x: b.x * T, y: b.y * T });
    }
    map.props.forEach((p, index) => {
      const ps = this.art.prop(p.kind, p.variant);
      this.propSprites.push(ps);
      const s = ps.albedo;
      const x = p.x * T;
      const y = p.y * T;
      this.statics.push({ kind: "p", index, base: y + Math.max(T, s.h - s.ay), l: x - s.ax, t: y - s.ay, r: x - s.ax + s.w, b: y - s.ay + s.h, x, y });
      const threshold = 0.32 + noise01(p.x, p.y, 7) * 0.16;
      this.lamps.push({
        light: ps.emissive ? { frame: ps.emissive, x, y, threshold } : null,
        pool: ps.pool ? { frame: ps.pool.frame, x: x + ps.pool.x, y: y + ps.pool.y, threshold } : null,
      });
    });
    this.statics.sort((a, b) => a.base - b.base || a.l - b.l || (a.kind === b.kind ? a.index - b.index : a.kind === "b" ? -1 : 1));
    // Overhang cells: rows above a building's footprint, and above a blocking prop's cell.
    this.statics.forEach((s, si) => {
      const blocks = s.kind === "b" || (map.props[s.index] as TownProp).blocks;
      if (!blocks || s.t >= s.y) return;
      for (let cy = Math.floor(s.t / T); cy <= Math.floor(s.y / T) - 1; cy++) {
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

  /** A building as the art draws it (made on first use, kept). */
  private buildingArt(b: TownBuilding): BuiltBuilding {
    let a = this.built.get(b.index);
    if (!a) {
      a = this.art.building(buildingSpec(this.map, b));
      this.built.set(b.index, a);
    }
    return a;
  }

  private frameOf(s: Static): FrameRef {
    return s.kind === "b" ? this.buildingArt(this.map.buildings[s.index] as TownBuilding).albedo : (this.propSprites[s.index] as PropSprite).albedo;
  }

  private casterOf(s: Static): ShadowCaster | undefined {
    return s.kind === "b" ? this.buildingArt(this.map.buildings[s.index] as TownBuilding).shadow : (this.propSprites[s.index] as PropSprite).shadow;
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
      c = { cx, cy, base: null, shadow: null, shadowKey: "", used: 0 };
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

  private drawFrame(g: Ctx, f: FrameRef, x: number, y: number) {
    g.drawImage(f.image, f.x, f.y, f.w, f.h, Math.round(x - f.ax), Math.round(y - f.ay), f.w, f.h);
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
    const tx1 = Math.min(map.width, Math.ceil((x0 + CP) / T));
    const ty1 = Math.min(map.height, Math.ceil((y0 + CP) / T));
    // Ask for every frame first, then draw: a provider that paints frames
    // into shared sheets on demand would otherwise have each sheet copied
    // (copy-on-write) every time it is drawn from between two paints.
    const tiles: [FrameRef, number, number][] = [];
    for (let y = Math.floor(y0 / T); y < ty1; y++) {
      for (let x = Math.floor(x0 / T); x < tx1; x++) {
        const i = y * map.width + x;
        tiles.push([this.art.ground(map.terrain[i] as number, this.ground.mask[i] as number, this.ground.variant[i] as number), x * T, y * T]);
      }
    }
    const items: [FrameRef, number, number][] = [];
    for (const s of this.statics) {
      if (s.r <= x0 || s.l >= x0 + CP || s.b <= y0 || s.t >= y0 + CP) continue;
      items.push([this.frameOf(s), s.x, s.y]);
    }
    for (const [f, x, y] of tiles) this.drawFrame(g, f, x, y);
    for (const [f, x, y] of items) this.drawFrame(g, f, x, y);
    g.setTransform(1, 0, 0, 1, 0, 0);
    this.stats.baseBakes++;
    return layer;
  }

  private bakeShadow(c: Chunk, grade: Grade): Layer {
    const { CP, map } = this;
    const half = CP / 2;
    const layer = c.shadow ?? this.make(half, half);
    const g = layer.ctx;
    g.setTransform(1, 0, 0, 1, 0, 0);
    g.globalCompositeOperation = "source-over";
    g.clearRect(0, 0, half, half);
    const x0 = c.cx * CP;
    const y0 = c.cy * CP;
    g.setTransform(0.5, 0, 0, 0.5, -x0 / 2, -y0 / 2);
    const { dx, dy } = grade.shadow;
    // Offsets snap to whole shadow pixels (2 art px), so axis-aligned edges stay crisp.
    const even = (n: number) => Math.round(n / 2) * 2;
    const reach = 3 * 4 * this.T;
    g.beginPath();
    for (const s of this.statics) {
      if (s.r + reach <= x0 || s.l - reach >= x0 + CP || s.b + reach <= y0 || s.t - reach >= y0 + CP) continue;
      const caster = this.casterOf(s);
      if (!caster || caster.height <= 0 || caster.footprint.length < 3) continue;
      const vx = dx * caster.height;
      const vy = dy * caster.height;
      const steps = Math.max(1, Math.ceil(Math.max(Math.abs(vx), Math.abs(vy)) / 2));
      for (let k = 0; k <= steps; k++) {
        const ox = s.x + even((vx * k) / steps);
        const oy = s.y + even((vy * k) / steps);
        caster.footprint.forEach(([px, py], i) => (i === 0 ? g.moveTo(ox + px, oy + py) : g.lineTo(ox + px, oy + py)));
        g.closePath();
      }
    }
    g.fillStyle = grade.shadow.color;
    g.fill("nonzero");
    // Shadows fall on the ground, not on the buildings and trees that stand in them.
    g.globalCompositeOperation = "destination-out";
    for (const s of this.statics) {
      if (s.r <= x0 || s.l >= x0 + CP || s.b <= y0 || s.t >= y0 + CP) continue;
      if (s.kind === "p" && !(map.props[s.index] as TownProp).blocks) continue;
      this.drawFrame(g, this.frameOf(s), s.x, s.y);
    }
    g.globalCompositeOperation = "source-over";
    g.setTransform(1, 0, 0, 1, 0, 0);
    this.stats.shadowBakes++;
    return layer;
  }

  /** Everything that decides what a frame looks like, quantised to what can be seen. */
  private frameKey(f: FrameState, g: Grade, tx: number, ty: number): number[] {
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
      q(g.emissive, 1 / 512),
      q(g.pool, 1 / 512),
      q(g.shadow.alpha, 1 / 512),
      q(g.shadow.dx, 1 / 256),
      q(g.shadow.dy, 1 / 256),
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
    const grade = this.art.grade(f.minutes);
    const key = this.frameKey(f, grade, tx, ty);
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
    const visible: Chunk[] = [];
    for (let cy = Math.max(0, Math.floor(vy0 / CP)); cy <= Math.min(this.ny - 1, Math.floor(vy1 / CP)); cy++) {
      for (let cx = Math.max(0, Math.floor(vx0 / CP)); cx <= Math.min(this.nx - 1, Math.floor(vx1 / CP)); cx++) {
        const c = this.chunk(cx, cy);
        c.base ??= this.bakeBase(c);
        ctx.drawImage(c.base.canvas, cx * CP, cy * CP);
        visible.push(c);
      }
    }

    // Shadows at the grade's strength.
    if (grade.shadow.alpha > 0.002) {
      const sk = `${grade.shadow.dx},${grade.shadow.dy},${grade.shadow.color}`;
      ctx.globalAlpha = grade.shadow.alpha;
      for (const c of visible) {
        if (!c.shadow || c.shadowKey !== sk) {
          c.shadow = this.bakeShadow(c, grade);
          c.shadowKey = sk;
        }
        ctx.drawImage(c.shadow.canvas, 0, 0, CP / 2, CP / 2, c.cx * CP, c.cy * CP, CP, CP);
      }
      ctx.globalAlpha = 1;
    }

    // Ground markers: where the player is going, and where they stand.
    const bob = this.reducedMotion ? 0 : Math.round(Math.sin(f.time * 5) * 1.5);
    if (f.target) {
      const tc = f.target.cell;
      if (f.target.building !== null) {
        const b = map.buildings[f.target.building] as TownBuilding;
        this.drawFrame(ctx, art.marker("door"), b.doorX * T + T / 2, (b.doorY + 1) * T + 1);
      } else this.drawFrame(ctx, art.marker("target"), (tc % map.width) * T + T / 2, Math.floor(tc / map.width) * T + T / 2 + bob);
    }
    if (f.player) this.drawFrame(ctx, art.marker("here"), f.player.x * T, f.player.y * T + 1);

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
    ctx.fillStyle = grade.shadow.color;
    ctx.globalAlpha = 0.3;
    for (const w of walkers) ctx.fillRect(Math.round(w.x * T) - 4, Math.round(w.y * T) - 1, 8, 2);
    ctx.globalAlpha = 1;
    for (const w of walkers) this.drawWalker(ctx, w, Math.round(w.x * T), Math.round(w.y * T));

    // Roofs and canopies over the walkers behind them.
    this.drawOverhangs(walkers);

    // The grade, over the whole view.
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    if (grade.sat < 0.995) {
      ctx.globalCompositeOperation = "saturation";
      ctx.globalAlpha = 1 - grade.sat;
      ctx.fillStyle = "#808080";
      ctx.fillRect(0, 0, f.width, f.height);
    }
    if (grade.mul[0] < 254.5 || grade.mul[1] < 254.5 || grade.mul[2] < 254.5) {
      ctx.globalCompositeOperation = "multiply";
      ctx.globalAlpha = 1;
      ctx.fillStyle = css(grade.mul);
      ctx.fillRect(0, 0, f.width, f.height);
    }
    if (grade.glowAlpha > 0.002) {
      ctx.globalCompositeOperation = "soft-light";
      ctx.globalAlpha = grade.glowAlpha;
      ctx.fillStyle = css(grade.glow);
      ctx.fillRect(0, 0, f.width, f.height);
    }
    ctx.globalCompositeOperation = "source-over";
    ctx.globalAlpha = 1;
    ctx.setTransform(s, 0, 0, s, tx, ty);

    // Lights over the grade: windows and doors (not over walkers in front of them), lamp heads, then pools.
    if (grade.emissive > 0.002) {
      for (const b of map.buildings) {
        if (b.x * T > vx1 || (b.x + b.w) * T < vx0 || (b.y - ROOF_ROWS) * T > vy1 || (b.y + b.h) * T < vy0) continue;
        for (const l of this.buildingLights(b)) {
          const lvl = lightLevel(grade.emissive, l.threshold);
          if (lvl <= 0.002) continue;
          ctx.globalAlpha = lvl;
          this.lightOverWalkers(l, walkers);
        }
      }
      for (const lamp of this.lamps) {
        const l = lamp.light;
        if (!l || l.x < vx0 - T || l.x > vx1 + T || l.y < vy0 - T || l.y > vy1 + 2 * T) continue;
        const lvl = lightLevel(grade.emissive, l.threshold, 0.1);
        if (lvl <= 0.002) continue;
        ctx.globalAlpha = lvl;
        this.drawFrame(ctx, l.frame, l.x, l.y);
      }
    }
    if (grade.pool > 0.002) {
      ctx.globalCompositeOperation = "lighter";
      for (const lamp of this.lamps) {
        const p = lamp.pool;
        if (!p || p.x < vx0 - p.frame.w || p.x > vx1 + p.frame.w || p.y < vy0 - p.frame.h || p.y > vy1 + p.frame.h) continue;
        const lvl = lightLevel(grade.pool, p.threshold, 0.1);
        if (lvl <= 0.002) continue;
        ctx.globalAlpha = lvl * 0.85;
        this.drawFrame(ctx, p.frame, p.x, p.y);
      }
      ctx.globalCompositeOperation = "source-over";
    }
    ctx.globalAlpha = 1;

    // "You are here", always bright, and the pin over the place being walked to.
    if (f.target && f.target.building !== null) {
      const b = map.buildings[f.target.building] as TownBuilding;
      this.drawFrame(ctx, art.marker("pin"), b.doorX * T + T / 2, (b.y - ROOF_ROWS) * T - 2 + bob);
    }
    if (f.player) this.drawFrame(ctx, art.marker("pin"), f.player.x * T, f.player.y * T - 26);

    ctx.setTransform(1, 0, 0, 1, 0, 0);
    this.evict();
    this.stats.chunks = this.chunks.size;
    this.stats.frames++;
    this.stats.lastFrameMs = this.now() - t0;
    return true;
  }

  private walkerFrame(w: WalkerView): FrameRef {
    const facing = w.dir === 0 ? "down" : w.dir === 1 ? "up" : "side";
    return this.art.character({ seed: w.variant, player: w.variant < 0 }, w.frame > 0 ? "walk" : "idle", facing, w.frame);
  }

  /** A walker at (x, y) art px; side frames face left and are mirrored for facing right. */
  private drawWalker(g: Ctx, w: WalkerView, x: number, y: number) {
    const fr = this.walkerFrame(w);
    if (w.dir !== 3) {
      this.drawFrame(g, fr, x, y);
      return;
    }
    g.save();
    g.translate(x, 0);
    g.scale(-1, 1);
    g.drawImage(fr.image, fr.x, fr.y, fr.w, fr.h, -fr.ax, y - fr.ay, fr.w, fr.h);
    g.restore();
  }

  /** A building's lit openings, each with its own switch-on point (by use, then a little by chance). */
  private buildingLights(b: TownBuilding): Light[] {
    let ls = this.lightCache.get(b.index);
    if (ls) return ls;
    const a = this.buildingArt(b);
    const ox = b.x * this.T - a.albedo.ax;
    const oy = b.y * this.T - a.albedo.ay;
    ls = [];
    for (const [k, o] of [...a.windows, ...a.doors].entries()) {
      if (!o.emissive) continue;
      const [lo, span] = USE_SHIFT[o.use];
      // A building's windows come on close together, not all at once.
      const threshold = lo + span * noise01(mix(b.style, 0x11), 0) + 0.06 * noise01(b.style, k);
      ls.push({ frame: o.emissive, x: ox + o.x, y: oy + o.y, threshold });
    }
    this.lightCache.set(b.index, ls);
    return ls;
  }

  private drawOverhangs(walkers: readonly WalkerView[]) {
    if (walkers.length === 0 || this.overhang.size === 0) return;
    const map = this.map;
    const hit = new Set<number>();
    for (const w of walkers) {
      for (let cy = Math.floor(w.y - 1.6); cy <= Math.floor(w.y); cy++) {
        for (let cx = Math.floor(w.x - 0.5); cx <= Math.floor(w.x + 0.49); cx++) {
          if (cx < 0 || cy < 0 || cx >= map.width || cy >= map.height) continue;
          for (const si of this.overhang.get(cy * map.width + cx) ?? []) hit.add(si);
        }
      }
    }
    for (const si of [...hit].sort((a, b) => a - b)) {
      const s = this.statics[si] as Static;
      const fr = this.frameOf(s);
      if (fr.ay <= 0) continue;
      this.ctx.drawImage(fr.image, fr.x, fr.y, fr.w, fr.ay, s.x - fr.ax, s.y - fr.ay, fr.w, fr.ay);
    }
  }

  /** A light, cut out where a walker stands in front of it. */
  private lightOverWalkers(l: Light, walkers: readonly WalkerView[]) {
    const T = this.T;
    const fr = l.frame;
    const left = Math.round(l.x - fr.ax);
    const top = Math.round(l.y - fr.ay);
    const over = walkers.filter((w) => {
      const wx = w.x * T;
      const wy = w.y * T;
      return wx + 8 > left && wx - 8 < left + fr.w && wy > top && wy - 24 < top + fr.h;
    });
    if (over.length === 0) {
      this.ctx.drawImage(fr.image, fr.x, fr.y, fr.w, fr.h, left, top, fr.w, fr.h);
      return;
    }
    let sc = this.scratch;
    if (!sc || sc.canvas.width < fr.w || sc.canvas.height < fr.h) {
      sc = this.make(Math.max(fr.w, sc?.canvas.width ?? 0, 64), Math.max(fr.h, sc?.canvas.height ?? 0, 64));
      this.scratch = sc;
    }
    const g = sc.ctx;
    g.setTransform(1, 0, 0, 1, 0, 0);
    g.globalCompositeOperation = "source-over";
    g.clearRect(0, 0, sc.canvas.width, sc.canvas.height);
    g.drawImage(fr.image, fr.x, fr.y, fr.w, fr.h, 0, 0, fr.w, fr.h);
    g.globalCompositeOperation = "destination-out";
    for (const w of over) this.drawWalker(g, w, Math.round(w.x * T) - left, Math.round(w.y * T) - top);
    g.globalCompositeOperation = "source-over";
    this.ctx.drawImage(sc.canvas, 0, 0, fr.w, fr.h, left, top, fr.w, fr.h);
  }
}
