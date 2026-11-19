// The pixel-art tool's art behind `ArtProvider` (./art; PIXEL_ART_TOOL.md
// §3.5, §5.1; DEC-084), for the P0 spike. It reads a built pack (palette,
// tiles, props, lighting: made ahead of time) and composes characters and
// buildings on the device from the pack's parts and kit (D10), through the
// tool's small runtime. The renderer needs no changes.
//
// Where the tool plan (§5.1) and the renderer's interface differ, this adapts:
//   - side frames: the kit draws them facing right (style guide §6); the
//     renderer wants them facing left, so they are mirrored here;
//   - walk frames: the kit has 4, and the hub view (../hubView) feeds frames
//     1..walkFrames (one per 4 px walked); `walkFrames: 2` shows only the two
//     contact frames;
//   - idle frames (2, breathing) are not shown: the renderer has no idle clock;
//   - palette moods (D12): the renderer bakes static layers once and grades
//     everything with one multiply; in the "moods" look this provider repaints
//     its static canvases with the moment's mood palette (`setClock`, which
//     says when the caller must re-bake: `TownRenderer.crossFade()`), and
//     `grade()` returns only the people's gentler multiply;
//   - markers and icons are not in the tool plan: the placeholder draws them;
//   - layout v2 (round 4, DEC-090): `groundCell` draws each cell of a v2 map
//     with its neighbours in view (levels, shores, creek, rail, bridges)
//     through the pack's terrain painter, one painter per map; cells that look
//     as v1 draws them use `ground`'s tiles, and drawn cells that come out the
//     same share one frame. A v1 map always gets `ground`'s tiles. Plot states
//     arrive as building conditions; unknown terrain codes draw as grass and
//     unknown props as the placeholder's;
//   - diagonal streets (round 5, DEC-090 D5 amended): a map whose
//     `streetAxis` marks any cell diagonal is drawn through the painter too,
//     which draws its 45° staircases as smooth 45° roads, kerbs and markings;
//     an absent or all-orthogonal `streetAxis` changes nothing.
import { gradeAt, shadowBucket, shadowVector, sunAt, wrapMinutes } from "../dayCycle";
import type { TownMap } from "../townLayout";
import {
  buildFacade,
  composeCharacter,
  faceLeft,
  groundTile,
  lookFromSeed,
  PLAYER_LOOK,
  TerrainPainter,
  tileOf,
  V1_NAMES,
  V2_NAMES,
  type ArtPack,
  type Facade,
  type FacadeSpec,
} from "../../../../tools/pixelkit/art/mandate/pack";
import { rgbaTable, toRgba, type Indexed } from "../../../../tools/pixelkit/src/runtime";
import {
  domCanvasFactory,
  type AnimKey,
  type ArtProvider,
  type BuildingSpec,
  type BuiltBuilding,
  type CanvasFactory,
  type Facing,
  type FrameRef,
  type Grade,
  type Layer,
  type LightShape,
  type LookView,
  type MarkerKind,
  type Opening,
  type PropSprite,
  type TerrainKind,
} from "./art";
import { PlaceholderArt } from "./placeholderArt";

export type NightLook = "multiply" | "moods";

export interface PixelkitArtOptions {
  createCanvas?: CanvasFactory;
  /** D12: multiply grading only (default), or palette moods for the static layers. */
  night?: NightLook;
  /** Walk frames to show (default 4, the whole cycle; 2 shows only the contact frames). */
  walkFrames?: 2 | 4;
  /** Composed characters kept (least recently used go first; §3.5 budget: about 150). */
  maxCharacters?: number;
}

/** Whether a map uses the layout v2 vocabulary (levels, connectors, diagonal streets, a v2 layout version). */
function isV2(map: TownMap): boolean {
  return map.version >= 2 || map.elevation !== undefined || (map.connectors !== undefined && map.connectors.length > 0) || (map.streetAxis !== undefined && map.streetAxis.some((a) => a !== 0));
}

const T = 16;
const CW = 16;
const CH = 24;

/** A static image (ground, building, prop) painted into a canvas region, kept so moods can repaint it. */
interface StaticArt {
  img: Indexed;
  layer: Layer;
  x: number;
  y: number;
}

/** Packs small frames into shared pages. */
class Pages {
  private page: Layer | null = null;
  private x = 0;
  private y = 0;
  private row = 0;
  private readonly make: (w: number, h: number) => Layer;
  private readonly size: number;
  constructor(make: (w: number, h: number) => Layer, size = 512) {
    this.make = make;
    this.size = size;
  }
  alloc(w: number, h: number): { layer: Layer; x: number; y: number } {
    if (w > this.size || h > this.size) return { layer: this.make(w, h), x: 0, y: 0 };
    if (!this.page || this.x + w > this.size) {
      this.x = 0;
      this.y += this.row;
      this.row = 0;
    }
    if (!this.page || this.y + h > this.size) {
      this.page = this.make(this.size, this.size);
      this.x = 0;
      this.y = 0;
      this.row = 0;
    }
    const at = { layer: this.page, x: this.x, y: this.y };
    this.x += w;
    this.row = Math.max(this.row, h);
    return at;
  }
}

/** The kit's view of the renderer's building spec (the left neighbour's too). */
function facadeSpec(spec: BuildingSpec): FacadeSpec {
  const f: FacadeSpec = { kind: spec.kind, hood: spec.hood, w: spec.w, h: spec.h, doorX: spec.door.x, storeys: spec.storeys, visualSeed: spec.visualSeed, named: spec.named, home: spec.home, landmark: spec.landmark, conditions: spec.conditions };
  if (spec.headroom !== undefined) f.headroom = spec.headroom;
  if (spec.ends) f.ends = spec.ends;
  if (spec.left) f.left = facadeSpec(spec.left);
  return f;
}

/** A cheap content key for identical lit images (many windows share one). */
function imageKey(img: Indexed): string {
  let h = 0x811c9dc5;
  for (let i = 0; i < img.px.length; i++) h = Math.imul(h ^ (img.px[i] as number), 0x01000193);
  return `${img.w}x${img.h}:${(h >>> 0).toString(36)}`;
}

export class PixelkitArt implements ArtProvider {
  readonly tile = T;
  readonly outside: string;
  readonly walkFrames: number;
  /** Buildings rise up to 2 rows (32 px) above their roof row where nothing stands behind them (round 2). */
  readonly maxRoofRows = 3;
  /** Lit shop windows and doors light the pavement in front of them (round 2). */
  readonly windowLight: LightShape;
  readonly night: NightLook;
  readonly pack: ArtPack;
  private readonly make: CanvasFactory;
  private readonly placeholder: PlaceholderArt;
  private readonly maxCharacters: number;
  private readonly baseTable: Uint8Array;
  private table: Uint8Array;
  private moodStep = -1;
  private readonly statics: StaticArt[] = [];
  private readonly tilePages: Pages;
  private readonly glowPages: Pages;
  private readonly tiles = new Map<number, FrameRef>();
  /** Layout v2: a terrain painter and the frames of drawn cells, per map; drawn cells deduplicated by their pixels. */
  private readonly v2 = new WeakMap<TownMap, { painter: TerrainPainter; cells: Map<number, FrameRef> }>();
  private readonly v2Tiles = new Map<string, FrameRef>();
  /** Maps known to be layout v1 (so `isV2` scans a map's street axes once). */
  private readonly v1Maps = new WeakSet<TownMap>();
  private readonly glows = new Map<string, FrameRef>();
  private readonly props = new Map<string, PropSprite>();
  private readonly characters = new Map<string, Layer>();
  private pool: FrameRef | null = null;
  /** Canvases made so far, and time spent composing (ms), for the HUD and tests. */
  made = 0;
  composeMs = { characters: 0, buildings: 0, terrain: 0, count: 0 };

  constructor(pack: ArtPack, options: PixelkitArtOptions = {}) {
    this.pack = pack;
    this.make = options.createCanvas ?? domCanvasFactory;
    this.night = options.night ?? "multiply";
    this.walkFrames = options.walkFrames ?? 4;
    this.maxCharacters = options.maxCharacters ?? 150;
    this.placeholder = new PlaceholderArt(options.createCanvas ? { createCanvas: options.createCanvas } : {});
    this.baseTable = rgbaTable(pack.palette.colors);
    this.table = this.baseTable;
    const grass = pack.palette.ramps.find((r) => r.name === "grass");
    this.outside = pack.palette.colors[grass ? (grass.steps[grass.anchorStep] as number) : 1] as string;
    const spill = pack.spill;
    this.windowLight = { rx: 2, ry: spill.depth, rz: 0, steps: spill.steps, bands: spill.colours, dither: true };
    this.tilePages = new Pages((w, h) => this.layer(w, h));
    this.glowPages = new Pages((w, h) => this.layer(w, h), 256);
  }

  private layer(w: number, h: number): Layer {
    this.made++;
    const l = this.make(w, h);
    l.ctx.imageSmoothingEnabled = false;
    return l;
  }

  /** Paint an indexed image into a canvas region (no-op on canvases without pixels, as in tests). */
  private paint(layer: Layer, img: Indexed, x: number, y: number, table: Uint8Array) {
    const id = layer.ctx.createImageData?.(img.w, img.h) as ImageData | undefined;
    if (!id || !id.data) return;
    toRgba(img, table, id.data);
    layer.ctx.putImageData(id, x, y);
  }

  private paintStatic(layer: Layer, img: Indexed, x: number, y: number) {
    this.statics.push({ img, layer, x, y });
    this.paint(layer, img, x, y, this.table);
  }

  // ------------------------------------------------------------------ the clock (palette moods)

  /**
   * Tell the provider the time of day. In the "moods" look, returns true when
   * the static layers' palette changed (the caller re-bakes: the renderer's
   * `crossFade()`, which re-bakes and fades from the old layers); always false in the "multiply" look.
   */
  setClock(minuteOfDay: number): boolean {
    if (this.night !== "moods") return false;
    const L = this.pack.lighting;
    const step = Math.floor(wrapMinutes(minuteOfDay) / L.stepMinutes) % L.entries.length;
    if (step === this.moodStep) return false;
    const prev = this.moodStep >= 0 ? L.entries[this.moodStep] : undefined;
    const next = L.entries[step];
    this.moodStep = step;
    if (!next) return false;
    if (prev && prev.moodPalette.every((c, i) => c === next.moodPalette[i])) return false;
    this.table = rgbaTable(next.moodPalette);
    for (const s of this.statics) this.paint(s.layer, s.img, s.x, s.y, this.table);
    return true;
  }

  // ------------------------------------------------------------------ grade

  grade(minuteOfDay: number): Grade {
    const L = this.pack.lighting;
    const n = L.entries.length;
    const m = wrapMinutes(minuteOfDay) / L.stepMinutes;
    const i = Math.floor(m) % n;
    const t = m - Math.floor(m);
    const a = L.entries[i];
    const b = L.entries[(i + 1) % n];
    const sun = sunAt(minuteOfDay);
    const v = shadowVector(shadowBucket(minuteOfDay));
    const shadow = { dx: v.dx, dy: v.dy, alpha: sun.shadowAlpha, color: L.shadowColor };
    if (!a || !b) {
      const g = gradeAt(minuteOfDay);
      return { mul: g.mul, glow: g.glow, glowAlpha: g.glowAlpha, sat: g.sat, shadow, emissive: g.lights, pool: g.lights };
    }
    const lerp = (x: number, y: number) => x + (y - x) * t;
    const rgb = (x: readonly number[], y: readonly number[]) => [lerp(x[0] as number, y[0] as number), lerp(x[1] as number, y[1] as number), lerp(x[2] as number, y[2] as number)] as const;
    const lights = lerp(a.lights, b.lights);
    if (this.night === "moods") {
      // The static layers carry the mood; the grade only darkens the people (and the shadows' strength stays).
      return { mul: rgb(a.spriteMul, b.spriteMul), glow: [255, 255, 255], glowAlpha: 0, sat: 1, shadow, emissive: lights, pool: lights };
    }
    return { mul: rgb(a.mul, b.mul), glow: rgb(a.glow, b.glow), glowAlpha: lerp(a.glowAlpha, b.glowAlpha), sat: lerp(a.sat, b.sat), shadow, emissive: lights, pool: lights };
  }

  // ------------------------------------------------------------------ ground

  ground(kind: TerrainKind, cornerMask: number, variant: number): FrameRef {
    const key = (kind << 16) | ((cornerMask & 255) << 3) | (variant & 7);
    const hit = this.tiles.get(key);
    if (hit) return hit;
    // v1 codes from the tile set; v2 codes drawn from the mask alone (flat, no connectors known); anything else as grass.
    const img = V2_NAMES[kind] !== undefined ? tileOf(this.pack.tiles, this.pack.terrain, kind, cornerMask & 255, variant & 7) : groundTile(this.pack.tiles, V1_NAMES[kind] ?? "grass", cornerMask, variant & 7);
    const { layer, x, y } = this.tilePages.alloc(T, T);
    this.paintStatic(layer, img, x, y);
    const f: FrameRef = { image: layer.canvas, x, y, w: T, h: T, ax: 0, ay: 0 };
    this.tiles.set(key, f);
    return f;
  }

  /**
   * Layout v2: the cell (x, y) drawn with the map around it (round 4). A v1 map, and any cell that looks as
   * v1 draws it, gets `ground`'s tile; other cells are drawn once, and identical drawings share a frame.
   */
  groundCell(map: TownMap, x: number, y: number, cornerMask: number, variant: number): FrameRef {
    const kind = (map.terrain[y * map.width + x] as number | undefined) ?? 0;
    let v2 = this.v2.get(map);
    if (!v2 && (this.v1Maps.has(map) || !isV2(map))) {
      this.v1Maps.add(map);
      return this.ground(kind, cornerMask, variant);
    }
    if (x < 0 || y < 0 || x >= map.width || y >= map.height) return this.ground(kind, cornerMask, variant);
    if (!v2) {
      v2 = { painter: new TerrainPainter(this.pack.tiles, this.pack.terrain, map), cells: new Map() };
      this.v2.set(map, v2);
    }
    const i = y * map.width + x;
    const hit = v2.cells.get(i);
    if (hit) return hit;
    let f: FrameRef;
    if (v2.painter.simple(x, y)) f = this.ground(kind, cornerMask, variant);
    else {
      const t0 = performance.now();
      const img = v2.painter.draw(x, y, variant & 7);
      const key = imageKey(img);
      const shared = this.v2Tiles.get(key);
      if (shared) f = shared;
      else {
        const at = this.tilePages.alloc(T, T);
        this.paintStatic(at.layer, img, at.x, at.y);
        f = { image: at.layer.canvas, x: at.x, y: at.y, w: T, h: T, ax: 0, ay: 0 };
        this.v2Tiles.set(key, f);
      }
      this.composeMs.terrain += performance.now() - t0;
    }
    v2.cells.set(i, f);
    return f;
  }

  // ------------------------------------------------------------------ buildings

  building(spec: BuildingSpec): BuiltBuilding {
    const t0 = performance.now();
    const f: Facade = buildFacade(facadeSpec(spec), this.pack.palette);
    const layer = this.layer(f.albedo.w, f.albedo.h);
    this.paintStatic(layer, f.albedo, 0, 0);
    const open = (o: Facade["windows"][number]): Opening => ({ x: o.x, y: o.y, w: o.w, h: o.h, emissive: o.emissive ? this.glow(o.emissive) : null, use: o.use });
    this.composeMs.buildings += performance.now() - t0;
    return {
      albedo: { image: layer.canvas, x: 0, y: 0, w: f.albedo.w, h: f.albedo.h, ax: 0, ay: f.ay },
      windows: f.windows.map(open),
      doors: f.doors.map(open),
      shadow: f.shadow,
      shadows: f.shadows,
      receivers: f.receivers,
    };
  }

  /** A lit image, shared between identical windows (anchor: the opening's top-left). Never mood-remapped: lights stay bright. */
  private glow(img: Indexed): FrameRef {
    const key = imageKey(img);
    const hit = this.glows.get(key);
    if (hit) return hit;
    const { layer, x, y } = this.glowPages.alloc(img.w, img.h);
    this.paint(layer, img, x, y, this.baseTable);
    const f: FrameRef = { image: layer.canvas, x, y, w: img.w, h: img.h, ax: 0, ay: 0 };
    this.glows.set(key, f);
    return f;
  }

  // ------------------------------------------------------------------ props

  prop(key: string, variant: number): PropSprite {
    const list = this.pack.props[key];
    if (!list || list.length === 0) return this.placeholder.prop(key, variant);
    const v = variant % list.length;
    const k = `${key}:${v}`;
    const hit = this.props.get(k);
    if (hit) return hit;
    const p = list[v] as (typeof list)[number];
    const layer = this.layer(p.albedo.w, p.albedo.h);
    this.paintStatic(layer, p.albedo, 0, 0);
    const sprite: PropSprite = { albedo: { image: layer.canvas, x: 0, y: 0, w: p.albedo.w, h: p.albedo.h, ax: p.ax, ay: p.ay } };
    if (p.emissive) {
      const e = this.layer(p.emissive.w, p.emissive.h);
      this.paint(e, p.emissive, 0, 0, this.baseTable);
      sprite.emissive = { image: e.canvas, x: 0, y: 0, w: p.emissive.w, h: p.emissive.h, ax: p.ax, ay: p.ay };
      sprite.pool = { frame: this.lightPool(), x: T / 2, y: T - 2, light: this.lampShape() };
    }
    if (p.shadow) sprite.shadow = p.shadow;
    this.props.set(k, sprite);
    return sprite;
  }

  /** The lamp's pool as a shape the renderer fits to the town (round 2: clipped to the ground, splashed up walls). */
  private lampShape(): LightShape {
    const pool = this.pack.pool;
    const c = (i: number) => (pool.colours[i] ?? [0, 0, 0, 0]) as readonly [number, number, number, number];
    return { rx: pool.w / 2, ry: pool.h / 2, rz: pool.rz, steps: pool.steps, bands: [c(4), c(3), c(2), c(1)], dither: true };
  }

  private lightPool(): FrameRef {
    if (this.pool) return this.pool;
    const pool = this.pack.pool;
    const layer = this.layer(pool.w, pool.h);
    const id = layer.ctx.createImageData?.(pool.w, pool.h) as ImageData | undefined;
    if (id && id.data) {
      for (let i = 0; i < pool.bands.length; i++) {
        const c = pool.colours[pool.bands[i] as number];
        if (!c) continue;
        id.data[i * 4] = c[0];
        id.data[i * 4 + 1] = c[1];
        id.data[i * 4 + 2] = c[2];
        id.data[i * 4 + 3] = Math.round(c[3] * 255);
      }
      layer.ctx.putImageData(id, 0, 0);
    }
    this.pool = { image: layer.canvas, x: 0, y: 0, w: pool.w, h: pool.h, ax: pool.w / 2, ay: pool.h / 2 };
    return this.pool;
  }

  // ------------------------------------------------------------------ characters

  character(look: LookView, anim: AnimKey, facing: Facing, frame: number): FrameRef {
    const key = look.player ? "player" : String(look.seed >>> 0);
    let sheet = this.characters.get(key);
    if (sheet) {
      // Least recently used goes first: re-insert on use.
      this.characters.delete(key);
      this.characters.set(key, sheet);
    } else {
      const t0 = performance.now();
      const c = composeCharacter(look.player ? PLAYER_LOOK : lookFromSeed(look.seed >>> 0), this.pack.palette, this.pack.lightness);
      sheet = this.layer(CW * 18, CH);
      c.frames.forEach((row, fi) => row.forEach((img, k) => this.paint(sheet as Layer, fi === 2 ? faceLeft(img) : img, (fi * 6 + k) * CW, 0, this.baseTable)));
      this.characters.set(key, sheet);
      this.composeMs.characters += performance.now() - t0;
      this.composeMs.count++;
      while (this.characters.size > this.maxCharacters) {
        const oldest = this.characters.keys().next().value as string;
        const l = this.characters.get(oldest);
        if (l) {
          l.canvas.width = 0;
          l.canvas.height = 0;
        }
        this.characters.delete(oldest);
      }
    }
    // Frames per facing: idle 0..1, walk 2..5 (contact, passing, contact, passing).
    let f = 0;
    if (anim === "walk" && frame > 0) f = this.walkFrames === 2 ? (frame % 2 === 1 ? 2 : 4) : 2 + ((frame - 1) % 4);
    const col = (facing === "down" ? 0 : facing === "up" ? 1 : 2) * 6 + f;
    return { image: sheet.canvas, x: col * CW, y: 0, w: CW, h: CH, ax: 7, ay: CH - 1 };
  }

  // ------------------------------------------------------------------ not in the tool plan: the placeholder's

  icon(key: string): string {
    return this.placeholder.icon(key);
  }

  iconSprite(): string {
    return this.placeholder.iconSprite();
  }

  marker(kind: MarkerKind): FrameRef {
    return this.placeholder.marker(kind);
  }
}
