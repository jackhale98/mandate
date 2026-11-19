// Placeholder pixel art for the hub, drawn in code (no image assets). It
// implements `ArtProvider` (./art; PIXEL_ART_TOOL.md §3.5, §5.1) so the art
// pack from the pixel-art tool replaces it without renderer changes: 16×16
// ground tiles by kind, neighbour mask and variant; buildings by place kind
// and neighbourhood, with an icon sign on every named place, windows and
// doors with emissive frames and use tags, and a shadow footprint; props
// with emissive lamp heads, banded light pools and shadow footprints; 16×24
// characters in three facings; the day's grade from ../dayCycle's keyframe
// table (this provider's "palette"); icons as SVG symbols. All from the
// small palette in ./palette. Frames are made lazily and cached.
import { gradeAt, shadowBucket, shadowVector, sunAt } from "../dayCycle";
import { noise01 } from "../rng";
import { ROOF_ROWS, StreetAxis, Terrain, type TownMap } from "../townLayout";
import {
  domCanvasFactory,
  type AnimKey,
  type ArtProvider,
  type BuildingSpec,
  type BuiltBuilding,
  type CanvasFactory,
  type Ctx,
  type Facing,
  type FrameRef,
  type Grade,
  type Layer,
  type LightUse,
  type LookView,
  type MarkerKind,
  type Opening,
  type PropSprite,
  type ShadowCaster,
  type TerrainKind,
} from "./art";
import { PAL } from "./palette";

const T = 16;

const rect = (ctx: Ctx, x: number, y: number, w: number, h: number, c: string) => {
  ctx.fillStyle = c;
  ctx.fillRect(x, y, w, h);
};

/** 7×7 sign icons by place kind ('#' ink on the board). */
const GLYPHS: Record<string, readonly string[]> = {
  civic: ["...#...", "..###..", "#######", ".#.#.#.", ".#.#.#.", ".#.#.#.", "#######"],
  dome: ["...#...", "..###..", ".#####.", "#######", "#.#.#.#", "#.#.#.#", "#######"],
  diner: ["..#.#..", ".#.#...", ".......", "######.", "#####.#", "######.", ".####.."],
  market: ["#######", "#.#.#.#", ".......", ".#####.", ".#.#.#.", ".#####.", "..###.."],
  school: ["...#...", "..#.#..", ".#...#.", "#######", ".#...#.", ".#.#.#.", ".#####."],
  clinic: ["..###..", "..###..", "#######", "#######", "#######", "..###..", "..###.."],
  community_centre: [".#...#.", "###.###", ".#...#.", "#######", "#.....#", "#.###.#", "#######"],
  park: ["..###..", ".#####.", "#######", ".#####.", "...#...", "...#...", ".#####."],
  news_office: ["#######", "#.....#", "#.###.#", "#.....#", "#.###.#", "#.....#", "#######"],
  party_office: ["#......", "#####..", "######.", "#####..", "#......", "#......", "#......"],
  union_hall: ["...#...", "..###..", "#######", ".#####.", "..###..", ".##.##.", "##...##"],
  business_association: ["..###..", "..#.#..", "#######", "#.....#", "###.###", "#.....#", "#######"],
  factory: ["#......", "#..#..#", "#.##.##", "#######", "#.#.#.#", "#######", "#######"],
  port: ["...#...", "..#.#..", "...#...", "#######", "...#...", "#..#..#", ".#####."],
  farm: ["#..#..#", ".#.#.#.", "#.###.#", ".#.#.#.", "..###..", "...#...", "...#..."],
  mine: ["#####..", ".....#.", "..#...#", ".#....#", "#......", "#......", "......."],
  mill: ["#..#..#", ".#.#.#.", "..###..", "#######", "..###..", ".#.#.#.", "#..#..#"],
  hotel: ["#.....#", "#.....#", "#######", "#.....#", "#.....#", ".......", "#######"],
  university: ["...#...", ".#####.", "#######", ".#####.", "..#.#.#", "..###.#", "......#"],
  tech_campus: [".#...#.", "#.....#", "#.###.#", "#.#.#.#", "#.###.#", "#.....#", ".#...#."],
  executive_residence: ["...#...", "...#...", "#######", ".#####.", "..###..", ".##.##.", "##...##"],
  home: ["...#...", "..###..", ".#####.", "#######", ".#...#.", ".#.#.#.", ".#####."],
};
/** The icon (GLYPHS key) for a place kind. */
const glyphKey = (kind: string): string =>
  kind in GLYPHS ? kind : kind === "council_chamber" || kind === "regional_assembly" || kind === "parliament" ? "dome" : kind === "town_hall" ? "civic" : "home";

/** Sign board colour by kind (the icon's ground). */
const SIGN: Record<string, string> = {
  town_hall: PAL.gold,
  council_chamber: PAL.gold,
  regional_assembly: PAL.gold,
  parliament: PAL.gold,
  executive_residence: PAL.gold,
  diner: PAL.red,
  market: PAL.flag,
  clinic: PAL.white,
  news_office: PAL.cream,
  party_office: PAL.flag,
  union_hall: PAL.red,
  school: PAL.blue,
  university: PAL.blue,
  park: PAL.grass3,
  port: PAL.blue,
};

type RoofType = "gable" | "flat" | "dome" | "civic" | "saw" | "thatch";
type WallType = "brick" | "render" | "stone" | "wood" | "metal";

interface Look {
  roof: RoofType;
  roofC: [string, string];
  wall: WallType;
  wallC: [string, string];
  /** Front wall height, px. */
  wallH: number;
  door: string;
  shop: string | null;
  chimney: boolean;
  stacks: number;
}

const pick = <T,>(items: readonly T[], n: number): T => items[Math.floor(n * items.length) % items.length] as T;

function lookFor(b: BuildingSpec): Look {
  const hood = b.hood;
  const n = (k: number) => noise01(b.visualSeed, k);
  const H = (b.h + ROOF_ROWS) * T;
  const wallFor = (px: number) => Math.min(px, H - 10);
  const roofs: Record<string, readonly [string, string][]> = {
    core: [
      [PAL.slate2, PAL.slate],
      [PAL.terracotta, PAL.terracotta2],
      [PAL.slate3, PAL.slate2],
    ],
    inner: [
      [PAL.terracotta, PAL.terracotta2],
      [PAL.brick, PAL.brick2],
      [PAL.slate2, PAL.slate],
    ],
    suburb: [
      [PAL.terracotta, PAL.terracotta2],
      [PAL.slate3, PAL.slate2],
      [PAL.wood, PAL.wood2],
      [PAL.teal, PAL.teal2],
    ],
    waterfront: [
      [PAL.teal, PAL.teal2],
      [PAL.slate2, PAL.slate],
      [PAL.blue, PAL.water2],
    ],
    industrial: [
      [PAL.grey, PAL.slate3],
      [PAL.teal, PAL.teal2],
    ],
    rural: [
      [PAL.thatch, PAL.thatch2],
      [PAL.terracotta, PAL.terracotta2],
    ],
  };
  const walls: Record<string, readonly { t: WallType; c: [string, string] }[]> = {
    core: [
      { t: "stone", c: [PAL.cream3, PAL.plaza2] },
      { t: "brick", c: [PAL.brick, PAL.brick2] },
      { t: "render", c: [PAL.cream2, PAL.cream3] },
    ],
    inner: [
      { t: "brick", c: [PAL.brick, PAL.brick2] },
      { t: "brick", c: [PAL.terracotta, PAL.terracotta2] },
      { t: "render", c: [PAL.cream2, PAL.cream3] },
    ],
    suburb: [
      { t: "render", c: [PAL.cream, PAL.cream3] },
      { t: "wood", c: [PAL.cream2, PAL.cream3] },
      { t: "brick", c: [PAL.brick, PAL.brick2] },
      { t: "render", c: ["#dbe4ea", PAL.grey] },
    ],
    waterfront: [
      { t: "wood", c: [PAL.cream, PAL.cream3] },
      { t: "wood", c: ["#9cc3df", "#7aa3c2"] },
      { t: "render", c: [PAL.cream2, PAL.cream3] },
    ],
    industrial: [
      { t: "metal", c: [PAL.grey, PAL.slate3] },
      { t: "brick", c: [PAL.brick2, "#5e2d27"] },
    ],
    rural: [
      { t: "render", c: [PAL.cream, PAL.cream3] },
      { t: "wood", c: [PAL.wood, PAL.wood2] },
    ],
  };
  const r = pick(roofs[hood] ?? (roofs.inner as readonly [string, string][]), n(1));
  const w = pick(walls[hood] ?? (walls.inner as readonly { t: WallType; c: [string, string] }[]), n(2));
  const doors = [PAL.wood2, PAL.teal2, PAL.brick2, PAL.slate2, "#5a6e3a"];
  const base: Look = {
    roof: hood === "industrial" ? "flat" : hood === "rural" ? "thatch" : "gable",
    roofC: r,
    wall: w.t,
    wallC: w.c,
    wallH: wallFor(b.h >= 3 ? 24 : 18),
    door: pick(doors, n(3)),
    shop: null,
    chimney: n(4) < 0.45,
    stacks: 0,
  };
  switch (b.kind) {
    case "town_hall":
    case "executive_residence":
      return { ...base, roof: "civic", roofC: [PAL.slate2, PAL.slate], wall: "stone", wallC: [PAL.cream, PAL.cream3], wallH: wallFor(36), door: PAL.wood2, chimney: false };
    case "council_chamber":
    case "regional_assembly":
    case "parliament":
      return { ...base, roof: "dome", roofC: [PAL.teal, PAL.teal2], wall: "stone", wallC: [PAL.cream2, PAL.cream3], wallH: wallFor(32), door: PAL.wood2, chimney: false };
    case "university":
    case "school":
      return { ...base, roof: "gable", roofC: [PAL.slate2, PAL.slate], wall: "brick", wallC: [PAL.brick, PAL.brick2], wallH: wallFor(30), chimney: false };
    case "factory":
    case "mill":
    case "mine":
      return { ...base, roof: "saw", roofC: [PAL.grey, PAL.slate3], wall: "brick", wallC: [PAL.brick2, "#5e2d27"], wallH: wallFor(28), door: PAL.slate, chimney: false, stacks: b.kind === "mine" ? 0 : 2 };
    case "tech_campus":
      return { ...base, roof: "flat", roofC: [PAL.slate3, PAL.slate2], wall: "render", wallC: ["#c8d6e2", PAL.grey], wallH: wallFor(30), door: PAL.slate, chimney: false };
    case "warehouse":
      return { ...base, roof: "flat", roofC: [PAL.grey, PAL.slate3], wall: "metal", wallC: n(5) < 0.5 ? [PAL.grey, PAL.slate3] : [PAL.teal, PAL.teal2], wallH: wallFor(26), door: PAL.slate, chimney: false };
    case "shed":
      return { ...base, roof: "flat", roofC: [PAL.slate3, PAL.slate2], wall: "metal", wallC: [PAL.grey, PAL.slate3], wallH: wallFor(18), door: PAL.slate, chimney: false };
    case "port":
    case "boathouse":
      return { ...base, roof: "gable", roofC: [PAL.teal, PAL.teal2], wall: "wood", wallC: ["#9cc3df", "#7aa3c2"], wallH: wallFor(24), chimney: false };
    case "barn":
    case "farm":
      return { ...base, roof: "gable", roofC: [PAL.brick2, "#5e2d27"], wall: "wood", wallC: [PAL.brick, PAL.brick2], wallH: wallFor(24), door: PAL.cream, chimney: false };
    case "farmhouse":
      return { ...base, roof: "thatch", roofC: [PAL.thatch, PAL.thatch2], wall: "render", wallC: [PAL.cream, PAL.cream3] };
    case "shop":
    case "diner":
    case "market":
    case "hotel":
    case "news_office":
    case "party_office":
    case "union_hall":
    case "business_association":
    case "clinic":
    case "community_centre": {
      const awnings = [PAL.red, PAL.flag, PAL.teal, PAL.gold, PAL.blue];
      const awning = b.kind === "diner" ? PAL.red : b.kind === "market" ? PAL.flag : b.kind === "party_office" ? PAL.flag : b.kind === "clinic" ? PAL.white : pick(awnings, n(6));
      return { ...base, roof: hood === "core" || b.w >= 5 ? "flat" : "gable", wallH: wallFor(b.kind === "hotel" ? 34 : 28), shop: b.kind === "hotel" || b.kind === "clinic" ? null : awning };
    }
    case "home":
      return { ...base, door: PAL.gold, chimney: true };
    default:
      return base;
  }
}

/** Light schedule by building kind. */
function lightUseOf(kind: string): LightUse {
  switch (kind) {
    case "home":
    case "house":
    case "terrace":
    case "farmhouse":
    case "boathouse":
      return "home";
    case "shop":
    case "diner":
    case "market":
    case "hotel":
      return "shop";
    case "town_hall":
    case "council_chamber":
    case "regional_assembly":
    case "parliament":
    case "executive_residence":
      return "civic";
    case "warehouse":
    case "shed":
    case "factory":
    case "mill":
    case "mine":
    case "port":
    case "barn":
    case "farm":
      return "industrial";
    default:
      return "office";
  }
}

/** Building height for shadows, tiles. */
function heightOf(b: BuildingSpec): number {
  if (b.landmark) return 2.2;
  if (b.kind === "warehouse" || b.kind === "factory" || b.kind === "mill") return 1.6;
  if (b.kind === "shed" || b.kind === "park") return 0.7;
  return 1 + Math.min(0.6, b.h * 0.12);
}

export interface PlaceholderArtOptions {
  createCanvas?: CanvasFactory;
}

/** Packs small frames into shared sheets (ground tiles, emissive windows). */
class Sheet {
  private page: Layer | null = null;
  private x = 0;
  private y = 0;
  private row = 0;
  constructor(
    private readonly make: (w: number, h: number) => Layer,
    private readonly size = 512,
  ) {}
  /** Room for a w×h frame; returns the page and where to draw. */
  alloc(w: number, h: number): { layer: Layer; x: number; y: number } {
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

export class PlaceholderArt implements ArtProvider {
  readonly tile = T;
  readonly outside = PAL.grass2;
  readonly walkFrames = 2;
  private readonly make: CanvasFactory;
  private tiles = new Map<number, FrameRef>();
  private cells = new Map<string, FrameRef>();
  private tileSheet: Sheet;
  private glowSheet: Sheet;
  private glows = new Map<string, FrameRef>();
  private props = new Map<string, PropSprite>();
  private characters = new Map<number, Layer>();
  private markers = new Map<MarkerKind, FrameRef>();
  private pool: FrameRef | null = null;
  /** Canvases made so far (tests: frames are made once and cached). */
  made = 0;

  constructor(options: PlaceholderArtOptions = {}) {
    this.make = options.createCanvas ?? domCanvasFactory;
    this.tileSheet = new Sheet((w, h) => this.layer(w, h));
    this.glowSheet = new Sheet((w, h) => this.layer(w, h), 256);
  }

  private layer(w: number, h: number): Layer {
    this.made++;
    const l = this.make(w, h);
    l.ctx.imageSmoothingEnabled = false;
    return l;
  }

  // ------------------------------------------------------------------ grade

  grade(minuteOfDay: number): Grade {
    const g = gradeAt(minuteOfDay);
    const sun = sunAt(minuteOfDay);
    const v = shadowVector(shadowBucket(minuteOfDay));
    return { mul: g.mul, glow: g.glow, glowAlpha: g.glowAlpha, sat: g.sat, shadow: { dx: v.dx, dy: v.dy, alpha: sun.shadowAlpha, color: "#141a33" }, emissive: g.lights, pool: g.lights };
  }

  // ------------------------------------------------------------------ icons

  icon(key: string): string {
    return `hub-icon-${glyphKey(key)}`;
  }

  /** The icons as an SVG sprite of <symbol>s (the art pack ships icons.svg instead). */
  iconSprite(): string {
    const symbols = Object.entries(GLYPHS).map(([k, g]) => {
      let d = "";
      g.forEach((row, y) => [...row].forEach((c, x) => (d += c === "#" ? `M${x} ${y}h1v1h-1z` : "")));
      return `<symbol id="hub-icon-${k}" viewBox="0 0 7 7"><path fill="currentColor" d="${d}"/></symbol>`;
    });
    return `<svg xmlns="http://www.w3.org/2000/svg" style="display:none">${symbols.join("")}</svg>`;
  }

  // ------------------------------------------------------------------ ground

  ground(kind: TerrainKind, cornerMask: number, variant: number): FrameRef {
    // The placeholder only looks at the four sides.
    const m = cornerMask & 15;
    const key = (kind << 8) | (m << 3) | (variant & 7);
    const hit = this.tiles.get(key);
    if (hit) return hit;
    const { layer, x, y } = this.tileSheet.alloc(T, T);
    paintGround(layer.ctx, kind, m, variant & 7, x, y);
    const f: FrameRef = { image: layer.canvas, x, y, w: T, h: T, ax: 0, ay: 0 };
    this.tiles.set(key, f);
    return f;
  }

  // ------------------------------------------------------------------ buildings

  building(spec: BuildingSpec): BuiltBuilding {
    const W = spec.w * T;
    const H = (spec.h + ROOF_ROWS) * T;
    const { canvas, ctx } = this.layer(W, H);
    // Layout v2 plot states (DEC-090 D2): an empty reserved lot and a building site have no building.
    const state = spec.conditions.find((c) => c === "reserved" || c === "construction" || c === "boarded" || c === "derelict");
    if (state === "reserved" || state === "construction") return this.lot(spec, canvas, ctx, state);
    const s = lookFor(spec);
    const n = (k: number) => noise01(spec.visualSeed, k);
    const use = lightUseOf(spec.kind);
    const windows: Opening[] = [];
    const doors: Opening[] = [];
    const wallTop = H - s.wallH;
    const doorPx = spec.door.x * T;
    const civic = s.roof === "civic" || s.roof === "dome";

    // ---- roof
    const roofH = wallTop;
    const [rl, rd] = s.roofC;
    if (s.roof === "gable" || s.roof === "thatch") {
      const ridge = Math.floor(roofH * 0.38);
      rect(ctx, 0, 0, W, ridge, rl);
      rect(ctx, 0, ridge, W, roofH - ridge, rd);
      rect(ctx, 0, ridge, W, 1, PAL.ink);
      const line = s.roof === "thatch" ? PAL.thatch2 : PAL.ink;
      ctx.globalAlpha = s.roof === "thatch" ? 0.8 : 0.25;
      for (let y = ridge + 3; y < roofH - 1; y += 3) rect(ctx, 0, y, W, 1, line);
      for (let y = 2; y < ridge; y += 3) rect(ctx, 0, y, W, 1, line);
      ctx.globalAlpha = 1;
      if (s.chimney && W >= 32) {
        const cx = 4 + Math.floor(n(7) * (W - 12));
        rect(ctx, cx, Math.max(0, ridge - 7), 5, 7, PAL.brick2);
        rect(ctx, cx, Math.max(0, ridge - 7), 5, 1, PAL.ink);
      }
    } else if (s.roof === "flat" || civic) {
      rect(ctx, 0, 0, W, roofH, rd);
      rect(ctx, 2, 2, W - 4, roofH - 4, rl);
      const vents = Math.floor(W / 24);
      for (let k = 0; k < vents; k++) rect(ctx, 6 + Math.floor(n(10 + k) * (W - 14)), 4 + Math.floor(n(20 + k) * Math.max(1, roofH - 12)), 5, 4, PAL.slate3);
      if (s.roof === "civic") {
        // A clock tower in the middle of the roof.
        const tw = 14;
        const tx = Math.floor(W / 2 - tw / 2);
        rect(ctx, tx, 0, tw, roofH, PAL.cream3);
        rect(ctx, tx + 1, 0, tw - 2, roofH - 1, PAL.cream);
        rect(ctx, tx + 2, 0, tw - 4, 4, PAL.slate2);
        rect(ctx, tx + 4, 6, 6, 6, PAL.gold);
        rect(ctx, tx + 6, 7, 1, 3, PAL.ink);
        rect(ctx, tx + 7, 9, 2, 1, PAL.ink);
      } else if (s.roof === "dome") {
        const cx = W / 2;
        const r = Math.min(W / 2 - 4, roofH - 2);
        for (let y = 0; y < r; y++) {
          const half = Math.round(Math.sqrt(r * r - (r - y) * (r - y)));
          rect(ctx, Math.round(cx - half), roofH - r + y - 1, half * 2, 1, y < r / 3 ? s.roofC[0] : PAL.teal2);
        }
        rect(ctx, Math.round(cx - r * 0.5), roofH - r + 1, 3, Math.max(1, r - 4), PAL.waterLight);
        rect(ctx, Math.round(cx - 1), Math.max(0, roofH - r - 4), 2, 4, PAL.gold);
      }
    } else if (s.roof === "saw") {
      rect(ctx, 0, 0, W, roofH, rd);
      for (let x0 = 0; x0 < W; x0 += 12) {
        for (let k = 0; k < 10; k++) rect(ctx, x0 + k, roofH - 4 - Math.floor(k * 0.8), 1, 4 + Math.floor(k * 0.8), rl);
        rect(ctx, x0 + 10, roofH - 12, 2, 12, PAL.glass2);
      }
      for (let k = 0; k < s.stacks; k++) {
        const sx = 6 + k * 14;
        if (sx + 5 > W) break;
        rect(ctx, sx, 0, 5, Math.max(6, roofH - 6), PAL.brick2);
        rect(ctx, sx, 0, 5, 2, PAL.ink);
      }
    }
    rect(ctx, 0, roofH - 1, W, 1, PAL.ink);
    // Layout v2 landmarks without a place: the church's tower and spire, the station's canopy.
    if (spec.kind === "church" || spec.kind === "chapel") {
      const tw = spec.kind === "church" ? 10 : 8;
      const tx = Math.floor(W / 2 - tw / 2);
      rect(ctx, tx - 1, 0, tw + 2, roofH, PAL.ink);
      rect(ctx, tx, 4, tw, roofH - 4, PAL.cream3);
      for (let y = 0; y < 6; y++) rect(ctx, tx + Math.floor(tw / 2) - 1 - Math.floor(y / 2), y, 2 + Math.floor(y / 2) * 2, 1, PAL.slate2);
      rect(ctx, tx + Math.floor(tw / 2) - 1, 7, 2, 3, PAL.ink);
    } else if (spec.kind === "station") {
      rect(ctx, 0, roofH - 5, W, 4, PAL.slate2);
      for (let x = 2; x < W; x += 6) rect(ctx, x, roofH - 5, 3, 1, PAL.cream);
      rect(ctx, Math.floor(W / 2) - 4, 1, 8, 6, PAL.cream);
      rect(ctx, Math.floor(W / 2) - 1, 2, 2, 3, PAL.ink);
    }

    // ---- front wall
    const [wl, wd] = s.wallC;
    rect(ctx, 0, wallTop, W, s.wallH, wl);
    if (s.wall === "brick") {
      for (let y = wallTop + 2; y < H; y += 3) {
        rect(ctx, 0, y, W, 1, wd);
        for (let x = (y % 2) * 3; x < W; x += 6) rect(ctx, x, y - 2, 1, 2, wd);
      }
    } else if (s.wall === "stone") {
      for (let y = wallTop + 4; y < H; y += 5) rect(ctx, 0, y, W, 1, wd);
      for (let y = wallTop; y < H; y += 5) for (let x = ((y / 5) % 2) * 4; x < W; x += 8) rect(ctx, x, y, 1, 5, wd);
    } else if (s.wall === "wood") {
      for (let y = wallTop + 2; y < H; y += 3) rect(ctx, 0, y, W, 1, wd);
    } else if (s.wall === "metal") {
      for (let x = 1; x < W; x += 3) rect(ctx, x, wallTop, 1, s.wallH, wd);
    } else {
      ctx.globalAlpha = 0.5;
      for (let k = 0; k < Math.floor(W / 6); k++) rect(ctx, Math.floor(n(30 + k) * W), wallTop + Math.floor(n(60 + k) * s.wallH), 2, 1, wd);
      ctx.globalAlpha = 1;
    }
    rect(ctx, 0, wallTop, W, 1, wd);
    rect(ctx, 0, H - 2, W, 2, wd);
    rect(ctx, 0, wallTop, 1, s.wallH, PAL.ink);
    rect(ctx, W - 1, wallTop, 1, s.wallH, PAL.ink);
    rect(ctx, 0, 0, 1, roofH, PAL.ink);
    rect(ctx, W - 1, 0, 1, roofH, PAL.ink);

    // Civic fronts: columns and a pediment.
    if (civic) {
      const top = wallTop + 6;
      for (let x = 4; x < W - 4; x += 8) {
        if (Math.abs(x + 2 - (doorPx + 8)) < 7) continue;
        rect(ctx, x, top, 4, H - top - 3, PAL.cream);
        rect(ctx, x + 3, top, 1, H - top - 3, PAL.cream3);
      }
      rect(ctx, 2, wallTop + 2, W - 4, 3, PAL.cream3);
      rect(ctx, 2, wallTop + 4, W - 4, 1, PAL.goldDark);
      rect(ctx, 0, H - 3, W, 3, PAL.plaza2);
    }

    // ---- windows
    const doorL = doorPx + 2;
    const doorR = doorPx + 14;
    const clearOfDoor = (x: number, w: number) => x + w <= doorL - 1 || x >= doorR + 1;
    const winW = s.wall === "metal" ? 6 : 4;
    const winH = s.wall === "metal" ? 3 : 5;
    const rows: number[] = [];
    if (civic) rows.push(wallTop + 10, wallTop + Math.max(10, s.wallH - 14));
    else if (s.wallH >= 26) rows.push(wallTop + 4, wallTop + s.wallH - 12);
    else rows.push(wallTop + Math.max(3, s.wallH - 13));
    const lower = rows[rows.length - 1] as number;
    for (const wy of rows) {
      if (s.shop && wy === lower) continue;
      const step = civic ? 8 : s.wall === "metal" ? 12 : 7;
      for (let x = 3; x + winW <= W - 3; x += step) {
        if (wy === lower && !clearOfDoor(x, winW)) continue;
        if (civic && x % 8 !== 3) continue;
        rect(ctx, x - 1, wy - 1, winW + 2, winH + 2, PAL.ink);
        rect(ctx, x, wy, winW, winH, PAL.glass);
        rect(ctx, x, wy, winW, 1, PAL.glass2);
        rect(ctx, x, wy + 1, 1, winH - 1, "#9db6cc");
        windows.push({ x, y: wy, w: winW, h: winH, emissive: this.glow(winW, winH), use });
      }
    }
    // Shop front: a big window under a striped awning.
    if (s.shop) {
      const sy = H - 12;
      rect(ctx, 0, sy - 4, W, 3, s.shop);
      for (let x = 0; x < W; x += 4) rect(ctx, x, sy - 4, 2, 3, PAL.cream);
      rect(ctx, 0, sy - 1, W, 1, PAL.ink);
      for (const [a, c] of [
        [2, doorL - 3],
        [doorR + 2, W - 3],
      ] as const) {
        if (c - a < 4) continue;
        rect(ctx, a - 1, sy, c - a + 2, 9, PAL.ink);
        rect(ctx, a, sy + 1, c - a, 7, PAL.glass);
        rect(ctx, a, sy + 1, c - a, 1, "#9db6cc");
        windows.push({ x: a, y: sy + 1, w: c - a, h: 7, emissive: this.glow(c - a, 7), use: "shop" });
      }
    }

    // ---- door
    const big = civic || spec.kind === "warehouse" || spec.kind === "factory";
    const dw = big ? 10 : 7;
    const dh = big ? 13 : 11;
    const dx = doorPx + Math.floor((T - dw) / 2);
    rect(ctx, dx - 1, H - dh - 1, dw + 2, dh + 1, PAL.ink);
    rect(ctx, dx, H - dh, dw, dh, s.door);
    if (spec.kind === "warehouse") for (let y = H - dh + 2; y < H; y += 2) rect(ctx, dx, y, dw, 1, PAL.slate3);
    else {
      rect(ctx, dx + dw - 2, H - Math.floor(dh / 2), 1, 1, PAL.gold);
      if (big) rect(ctx, dx + Math.floor(dw / 2), H - dh, 1, dh, PAL.ink);
    }
    if (spec.kind === "barn") {
      for (let k = 0; k < dh; k++) {
        rect(ctx, dx + Math.floor((k * (dw - 1)) / dh), H - dh + k, 1, 1, PAL.cream3);
        rect(ctx, dx + dw - 1 - Math.floor((k * (dw - 1)) / dh), H - dh + k, 1, 1, PAL.cream3);
      }
    }
    // Named places, homes and shops leave a light on over the door at night.
    const litDoor = spec.named || spec.home || s.shop !== null;
    doors.push({ x: dx, y: H - dh, w: dw, h: 3, emissive: litDoor ? this.glow(dw, 3) : null, use });

    // ---- sign over named places (and the player's home)
    if (spec.named || spec.home) {
      const g = GLYPHS[glyphKey(spec.kind)] ?? (GLYPHS.home as readonly string[]);
      const bw = 11;
      const bh = 11;
      const sx = Math.max(1, Math.min(W - bw - 1, doorPx + 8 - Math.floor(bw / 2)));
      const sy = Math.max(1, Math.min(H - dh - bh - 2, wallTop - (s.wallH < 22 ? 6 : 1)));
      rect(ctx, sx - 1, sy - 1, bw + 2, bh + 2, PAL.ink);
      rect(ctx, sx, sy, bw, bh, spec.home ? PAL.gold : (SIGN[spec.kind] ?? PAL.cream));
      for (let gy = 0; gy < 7; gy++) {
        const row = g[gy] ?? "";
        for (let gx = 0; gx < 7; gx++) if (row[gx] === "#") rect(ctx, sx + 2 + gx, sy + 2 + gy, 1, 1, PAL.ink);
      }
    }

    // Boarded up (vacancy, a closed works) and derelict (a sustained fall): planks over the openings, no lights.
    if (state === "boarded" || state === "derelict") {
      for (const o of [...windows, ...doors]) {
        rect(ctx, o.x - 1, o.y - 1, o.w + 2, o.h + 2, PAL.wood2);
        for (let y = o.y; y < o.y + o.h; y += 2) rect(ctx, o.x - 1, y, o.w + 2, 1, PAL.wood);
      }
      windows.length = 0;
      doors.length = 0;
      if (state === "derelict") {
        // Holes in the roof and a grimy front.
        for (let k = 0; k < Math.max(1, Math.floor(W / 20)); k++) rect(ctx, 3 + Math.floor(n(90 + k) * (W - 12)), 2 + Math.floor(n(95 + k) * Math.max(1, roofH - 8)), 6, 4, PAL.ink);
        ctx.globalAlpha = 0.3;
        rect(ctx, 0, wallTop, W, s.wallH, PAL.ink);
        ctx.globalAlpha = 1;
      }
    }

    const fh = spec.h * T;
    return {
      albedo: { image: canvas, x: 0, y: 0, w: W, h: H, ax: 0, ay: ROOF_ROWS * T },
      windows,
      doors,
      shadow: {
        footprint: [
          [0, 0],
          [W, 0],
          [W, fh],
          [0, fh],
        ],
        height: Math.round(heightOf(spec) * T),
      },
    };
  }

  /** A plot with nothing standing yet: a reserved lot (rough grass, survey pegs) or a building site (slab, scaffold). */
  private lot(spec: BuildingSpec, canvas: HTMLCanvasElement, ctx: Ctx, state: "reserved" | "construction"): BuiltBuilding {
    const W = spec.w * T;
    const H = (spec.h + ROOF_ROWS) * T;
    const top = ROOF_ROWS * T;
    const n = (k: number) => noise01(spec.visualSeed, k);
    if (state === "reserved") {
      for (let k = 0; k < spec.w * spec.h * 3; k++) rect(ctx, Math.floor(n(k) * (W - 2)), top + Math.floor(n(k + 500) * (H - top - 2)), 2, 1, PAL.grass2);
      // Pegs and a string line round the plot.
      for (const [x, y] of [
        [1, top + 1],
        [W - 3, top + 1],
        [1, H - 4],
        [W - 3, H - 4],
      ] as const) {
        rect(ctx, x, y - 3, 2, 5, PAL.wood);
        rect(ctx, x, y - 3, 2, 1, PAL.flag);
      }
      ctx.globalAlpha = 0.6;
      rect(ctx, 2, top, W - 4, 1, PAL.cream3);
      rect(ctx, 2, H - 3, W - 4, 1, PAL.cream3);
      ctx.globalAlpha = 1;
      return { albedo: { image: canvas, x: 0, y: 0, w: W, h: H, ax: 0, ay: top }, windows: [], doors: [], shadow: { footprint: [], height: 0 } };
    }
    // A building site: a concrete slab, scaffold poles and boards, a little crane.
    rect(ctx, 1, top + 4, W - 2, H - top - 5, PAL.pavement2);
    rect(ctx, 1, H - 2, W - 2, 1, PAL.slate3);
    for (let x = 2; x < W - 1; x += 8) rect(ctx, x, top - 6, 1, H - top + 4, PAL.gold);
    for (let y = top; y < H - 2; y += 7) rect(ctx, 1, y, W - 2, 1, PAL.goldDark);
    rect(ctx, W - 6, 2, 2, H - 6, PAL.gold);
    rect(ctx, Math.max(0, W - 20), 2, 18, 2, PAL.gold);
    const fh = spec.h * T;
    return {
      albedo: { image: canvas, x: 0, y: 0, w: W, h: H, ax: 0, ay: top },
      windows: [],
      doors: [],
      shadow: {
        footprint: [
          [0, 0],
          [W, 0],
          [W, fh],
          [0, fh],
        ],
        height: 6,
      },
    };
  }

  /**
   * Layout v2: a cell with what the map says around it (DEC-090, §7.7): its
   * ground tile, then lips where it stands above the cell north, east or west
   * of it (a south-facing change is its own Embankment or Rock tile), and a
   * lighter tone two levels up. A v1 map gets exactly `ground`.
   */
  groundCell(map: TownMap, x: number, y: number, cornerMask: number, variant: number): FrameRef {
    const W = map.width;
    const i = y * W + x;
    const kind = map.terrain[i] as number;
    if (kind === Terrain.Creek || kind === Terrain.Ford) {
      // The channel runs on through fords and under bridges.
      const wet = (nx: number, ny: number) => {
        if (nx < 0 || ny < 0 || nx >= W || ny >= map.height) return true;
        const t = map.terrain[ny * W + nx];
        return t === Terrain.Creek || t === Terrain.Ford || t === Terrain.Bridge || t === Terrain.Water;
      };
      cornerMask = (wet(x, y - 1) ? 1 : 0) | (wet(x + 1, y) ? 2 : 0) | (wet(x, y + 1) ? 4 : 0) | (wet(x - 1, y) ? 8 : 0);
    }
    const ax = map.streetAxis?.[i] ?? 0;
    if ((ax === StreetAxis.DiagonalNE || ax === StreetAxis.DiagonalNW) && (kind === Terrain.Road || kind === Terrain.Pavement || kind === Terrain.Crossing || kind === Terrain.Dirt)) return this.diagonalCell(map, x, y, kind, ax, variant);
    const corner = turningCorner(map, x, y);
    const base = corner ? this.roundedCell(kind, corner, cornerMask, variant) : this.ground(kind, cornerMask, variant);
    const el = map.elevation;
    if (!el || kind === Terrain.Water) return base;
    const lv = el[i] as number;
    const lower = (nx: number, ny: number) => nx >= 0 && ny >= 0 && nx < W && ny < map.height && (el[ny * W + nx] as number) < lv;
    const edges = (lower(x, y - 1) ? 1 : 0) | (lower(x + 1, y) ? 2 : 0) | (lower(x - 1, y) ? 8 : 0);
    const tone = lv >= 2 ? lv - 1 : 0;
    if (!edges && !tone) return base;
    const key = `${kind}:${cornerMask & 15}:${variant & 7}:${edges}:${tone}`;
    const hit = this.cells.get(key);
    if (hit) return hit;
    const { layer, x: px, y: py } = this.tileSheet.alloc(T, T);
    const g = layer.ctx;
    g.drawImage(base.image, base.x, base.y, T, T, px, py, T, T);
    if (tone) {
      g.globalAlpha = 0.07 * tone;
      rect(g, px, py, T, T, PAL.cream);
      g.globalAlpha = 1;
    }
    // A lip: the edge of higher ground seen from above (a light line, a shadow just outside it).
    if (edges & 1) {
      rect(g, px, py, T, 1, PAL.cream3);
      rect(g, px, py + 1, T, 1, PAL.grass2);
    }
    if (edges & 2) rect(g, px + T - 1, py, 1, T, PAL.tree2);
    if (edges & 8) rect(g, px, py, 1, T, PAL.cream3);
    const f: FrameRef = { image: layer.canvas, x: px, y: py, w: T, h: T, ax: 0, ay: 0 };
    this.cells.set(key, f);
    return f;
  }

  /**
   * A cell of a 45° street, drawn smooth: the street's edges and the line
   * between its pavement and carriageway run corner to corner across the
   * staircase of cells, so each cell is two triangles (split along the
   * street's direction): the ground beside the street, pavement or road.
   */
  private diagonalCell(map: TownMap, x: number, y: number, kind: number, ax: number, variant: number): FrameRef {
    const W = map.width;
    const tAt = (xx: number) => (xx < 0 || xx >= W ? Terrain.Grass : (map.terrain[y * W + xx] as number));
    const street = (t: number) => t === Terrain.Road || t === Terrain.Pavement || t === Terrain.Crossing || t === Terrain.Plaza || t === Terrain.Bridge || t === Terrain.Dirt;
    const landOf = (t: number) => (t === Terrain.Grass || t === Terrain.Garden || t === Terrain.Park || t === Terrain.Field || t === Terrain.Yard || t === Terrain.Path || t === Terrain.Sand || t === Terrain.Water ? t : Terrain.Grass);
    const left = tAt(x - 1);
    const right = tAt(x + 1);
    const lane = kind === Terrain.Dirt ? Terrain.Dirt : Terrain.Road;
    let a: number;
    let b: number;
    if (kind === Terrain.Pavement) {
      if (!street(left)) {
        a = landOf(left);
        b = Terrain.Pavement;
      } else if (!street(right)) {
        a = Terrain.Pavement;
        b = landOf(right);
      } else a = b = Terrain.Pavement;
    } else {
      a = left === Terrain.Pavement ? Terrain.Pavement : street(left) ? lane : landOf(left);
      b = right === Terrain.Pavement ? Terrain.Pavement : street(right) ? lane : landOf(right);
    }
    const key = `d:${kind}:${ax}:${a}:${b}:${variant & 3}`;
    const hit = this.cells.get(key);
    if (hit) return hit;
    const A = this.ground(a as TerrainKind, 15, variant & 7);
    const B = this.ground(b as TerrainKind, 15, (variant + 3) & 7);
    const { layer, x: px, y: py } = this.tileSheet.alloc(T, T);
    const g = layer.ctx;
    // "\" (DiagonalNW): A is the lower-left triangle; "/" (DiagonalNE): A is the upper-left one.
    for (let r = 0; r < T; r++) {
      const aW = ax === StreetAxis.DiagonalNW ? r : T - 1 - r;
      if (ax === StreetAxis.DiagonalNW) {
        if (aW > 0) g.drawImage(A.image, A.x, A.y + r, aW, 1, px, py + r, aW, 1);
        g.drawImage(B.image, B.x + aW, B.y + r, T - aW, 1, px + aW, py + r, T - aW, 1);
      } else {
        g.drawImage(A.image, A.x, A.y + r, aW + 1, 1, px, py + r, aW + 1, 1);
        if (aW + 1 < T) g.drawImage(B.image, B.x + aW + 1, B.y + r, T - aW - 1, 1, px + aW + 1, py + r, T - aW - 1, 1);
      }
    }
    // The kerb along the split where pavement meets road or the ground beside the street.
    if (a !== b) {
      for (let r = 0; r < T; r++) rect(g, px + (ax === StreetAxis.DiagonalNW ? r : T - 1 - r), py + r, 1, 1, PAL.slate3);
    }
    if (kind === Terrain.Crossing) for (let k = 2; k < T; k += 4) rect(g, px + k, py + (ax === StreetAxis.DiagonalNW ? T - 1 - k : k), 2, 2, PAL.cream2);
    const f: FrameRef = { image: layer.canvas, x: px, y: py, w: T, h: T, ax: 0, ay: 0 };
    this.cells.set(key, f);
    return f;
  }

  /** A corner cell of a turning circle's road block: the outer corner rounded off (pavement round the bulb). */
  private roundedCell(kind: number, corner: number, cornerMask: number, variant: number): FrameRef {
    const key = `r:${kind}:${corner}:${cornerMask & 15}:${variant & 7}`;
    const hit = this.cells.get(key);
    if (hit) return hit;
    const base = this.ground(kind as TerrainKind, cornerMask, variant);
    const pave = this.ground(Terrain.Pavement, 15, variant);
    const { layer, x: px, y: py } = this.tileSheet.alloc(T, T);
    const g = layer.ctx;
    g.drawImage(base.image, base.x, base.y, T, T, px, py, T, T);
    // corner: 1 top-left, 2 top-right, 4 bottom-right, 8 bottom-left of the block.
    const cx = corner === 1 || corner === 8 ? T : 0;
    const cy = corner === 1 || corner === 2 ? T : 0;
    for (let r = 0; r < T; r++) {
      const dy = r + 0.5 - cy;
      const half = Math.sqrt(Math.max(0, T * T - dy * dy));
      // Pixels further than the radius from the bulb's inner corner are pavement.
      const outFrom = cx === T ? T - Math.floor(half) : Math.ceil(half);
      if (cx === T && outFrom > 0) g.drawImage(pave.image, pave.x, pave.y + r, outFrom, 1, px, py + r, outFrom, 1);
      if (cx === 0 && outFrom < T) g.drawImage(pave.image, pave.x + outFrom, pave.y + r, T - outFrom, 1, px + outFrom, py + r, T - outFrom, 1);
      const kx = cx === T ? outFrom : outFrom - 1;
      if (kx >= 0 && kx < T) rect(g, px + kx, py + r, 1, 1, PAL.slate3);
    }
    const f: FrameRef = { image: layer.canvas, x: px, y: py, w: T, h: T, ax: 0, ay: 0 };
    this.cells.set(key, f);
    return f;
  }

  /** A lit window or doorway of a size, with a one-pixel halo (anchor: the opening's top-left). */
  private glow(w: number, h: number): FrameRef {
    const key = `${w}x${h}`;
    const hit = this.glows.get(key);
    if (hit) return hit;
    const { layer, x, y } = this.glowSheet.alloc(w + 2, h + 2);
    const g = layer.ctx;
    g.globalAlpha = 0.35;
    rect(g, x, y, w + 2, h + 2, PAL.lit);
    g.globalAlpha = 1;
    rect(g, x + 1, y + 1, w, h, PAL.lit);
    rect(g, x + 2, y + 2, Math.max(1, w - 2), Math.max(1, h - 2), PAL.litCore);
    const f: FrameRef = { image: layer.canvas, x, y, w: w + 2, h: h + 2, ax: 1, ay: 1 };
    this.glows.set(key, f);
    return f;
  }

  // ------------------------------------------------------------------ props

  prop(key: string, variant: number): PropSprite {
    const v = key === "tree" || key === "pine" || key === "flag" ? variant % 3 : key === "bollards" || key === "field_gate" || key === "railing" ? variant & 15 : 0;
    const k = `${key}:${v}`;
    const hit = this.props.get(k);
    if (hit) return hit;
    const s = this.paintProp(key, v);
    this.props.set(k, s);
    return s;
  }

  private paintProp(kind: string, v: number): PropSprite {
    const frame = (w: number, h: number, ay: number, paint: (ctx: Ctx) => void): FrameRef => {
      const { canvas, ctx } = this.layer(w, h);
      paint(ctx);
      return { image: canvas, x: 0, y: 0, w, h, ax: 0, ay };
    };
    const box = (x0: number, y0: number, x1: number, y1: number): ShadowCaster["footprint"] => [
      [x0, y0],
      [x1, y0],
      [x1, y1],
      [x0, y1],
    ];
    switch (kind) {
      case "tree": {
        const [c, d, l] = [
          [PAL.tree, PAL.tree2, PAL.treeLight],
          ["#46773c", "#30592f", "#6ba552"],
          ["#3a6a45", "#284d35", "#5a9456"],
        ][v] as [string, string, string];
        const albedo = frame(16, 26, 10, (ctx) => {
          rect(ctx, 7, 18, 2, 7, PAL.wood2);
          const rows = [4, 6, 7, 7, 7, 7, 7, 6, 5, 3];
          rows.forEach((half, i) => rect(ctx, 8 - half - 1, 2 + i, (half + 1) * 2, 1, PAL.ink));
          rect(ctx, 2, 1, 12, 1, PAL.ink);
          rows.forEach((half, i) => rect(ctx, 8 - half, 2 + i, half * 2, 1, i > 6 ? d : c));
          rect(ctx, 4, 3, 4, 2, l);
          rect(ctx, 3, 5, 2, 2, l);
          rect(ctx, 9, 9, 4, 2, d);
          rect(ctx, 3, 12, 10, 3, d);
          rect(ctx, 4, 15, 8, 2, PAL.ink);
          rect(ctx, 5, 12, 6, 1, c);
        });
        return { albedo, shadow: { footprint: box(3, 4, 13, 12), height: 21 } };
      }
      case "pine": {
        const albedo = frame(16, 28, 12, (ctx) => {
          rect(ctx, 7, 21, 2, 6, PAL.wood2);
          for (let i = 0; i < 20; i++) {
            const half = 1 + Math.floor((i % 7) * 0.9) + Math.floor(i / 7);
            rect(ctx, 8 - half - 1, 1 + i, (half + 1) * 2, 1, PAL.ink);
            rect(ctx, 8 - half, 1 + i, half * 2, 1, i % 7 < 2 ? PAL.treeLight : i % 7 > 4 ? PAL.tree2 : PAL.tree);
          }
        });
        return { albedo, shadow: { footprint: box(4, 6, 12, 14), height: 26 } };
      }
      case "lamp": {
        const albedo = frame(16, 30, 14, (ctx) => {
          rect(ctx, 7, 4, 2, 25, PAL.ink);
          rect(ctx, 7, 5, 1, 23, PAL.slate3);
          rect(ctx, 5, 0, 6, 4, PAL.ink);
          rect(ctx, 6, 1, 4, 2, PAL.cream);
          rect(ctx, 6, 28, 4, 2, PAL.ink);
        });
        const emissive = frame(16, 30, 14, (ctx) => {
          ctx.globalAlpha = 0.5;
          rect(ctx, 4, 0, 8, 5, PAL.lit);
          ctx.globalAlpha = 1;
          rect(ctx, 6, 1, 4, 2, PAL.litCore);
        });
        return { albedo, emissive, pool: { frame: this.lightPool(), x: 8, y: 14 } };
      }
      case "bench":
        return {
          albedo: frame(16, 16, 0, (ctx) => {
            rect(ctx, 2, 6, 12, 2, PAL.wood2);
            rect(ctx, 2, 9, 12, 3, PAL.wood);
            rect(ctx, 2, 12, 12, 1, PAL.ink);
            rect(ctx, 3, 12, 1, 3, PAL.ink);
            rect(ctx, 12, 12, 1, 3, PAL.ink);
          }),
        };
      case "fountain": {
        const albedo = frame(32, 34, 2, (ctx) => {
          rect(ctx, 2, 10, 28, 22, PAL.ink);
          rect(ctx, 3, 11, 26, 20, PAL.cream3);
          rect(ctx, 5, 13, 22, 15, PAL.water);
          rect(ctx, 7, 15, 8, 1, PAL.waterLight);
          rect(ctx, 17, 22, 6, 1, PAL.waterLight);
          rect(ctx, 14, 6, 4, 14, PAL.cream3);
          rect(ctx, 15, 0, 2, 8, PAL.waterLight);
          rect(ctx, 12, 4, 8, 3, PAL.cream);
          rect(ctx, 3, 29, 26, 2, PAL.plaza2);
        });
        return { albedo, shadow: { footprint: box(4, 10, 28, 30), height: 6 } };
      }
      case "boat":
        return {
          albedo: frame(16, 16, 0, (ctx) => {
            rect(ctx, 2, 8, 12, 5, PAL.ink);
            rect(ctx, 3, 8, 10, 3, PAL.cream);
            rect(ctx, 3, 11, 10, 1, PAL.red);
            rect(ctx, 7, 2, 1, 6, PAL.wood2);
            rect(ctx, 8, 3, 4, 4, PAL.cream2);
          }),
        };
      case "planter":
        return {
          albedo: frame(16, 16, 0, (ctx) => {
            rect(ctx, 3, 8, 10, 6, PAL.ink);
            rect(ctx, 4, 9, 8, 4, PAL.wood);
            rect(ctx, 4, 5, 8, 4, PAL.grass);
            rect(ctx, 5, 5, 1, 1, PAL.red);
            rect(ctx, 8, 6, 1, 1, PAL.gold);
            rect(ctx, 10, 5, 1, 1, PAL.cream);
          }),
        };
      case "flag":
        return {
          albedo: frame(16, 40, 24, (ctx) => {
            rect(ctx, 3, 0, 2, 39, PAL.ink);
            rect(ctx, 3, 1, 1, 37, PAL.grey);
            rect(ctx, 5, 2, 9, 6, v === 1 ? PAL.flag : PAL.gold);
            rect(ctx, 5, 8, 9, 1, PAL.ink);
            rect(ctx, 2, 37, 4, 3, PAL.ink);
          }),
        };
      // ---- layout v2 (DEC-090)
      case "headframe": {
        // A mine's winding tower: two legs, a cross beam, the wheel on top.
        const albedo = frame(16, 48, 32, (ctx) => {
          rect(ctx, 2, 10, 2, 37, PAL.ink);
          rect(ctx, 12, 10, 2, 37, PAL.ink);
          rect(ctx, 3, 10, 1, 36, PAL.red);
          rect(ctx, 12, 10, 1, 36, PAL.red);
          for (let y = 16; y < 46; y += 8) rect(ctx, 3, y, 10, 1, PAL.brick2);
          rect(ctx, 1, 8, 14, 3, PAL.ink);
          rect(ctx, 4, 1, 8, 8, PAL.ink);
          rect(ctx, 5, 2, 6, 6, PAL.slate3);
          rect(ctx, 7, 4, 2, 2, PAL.ink);
          rect(ctx, 3, 46, 10, 2, PAL.ink);
        });
        return { albedo, shadow: { footprint: box(2, 6, 14, 14), height: 40 } };
      }
      case "spoil_heap": {
        const albedo = frame(48, 32, 16, (ctx) => {
          for (let y = 0; y < 30; y++) {
            const half = Math.floor(((y + 2) * 23) / 30);
            rect(ctx, 24 - half - 1, y + 1, (half + 1) * 2, 1, PAL.ink);
            rect(ctx, 24 - half, y + 1, half * 2, 1, y < 8 ? PAL.slate3 : y < 20 ? PAL.slate2 : PAL.slate);
          }
          for (let k = 0; k < 12; k++) rect(ctx, 8 + ((k * 7) % 32), 12 + ((k * 5) % 16), 2, 1, PAL.ink);
        });
        return { albedo, shadow: { footprint: box(4, 18, 44, 32), height: 10 } };
      }
      case "reeds":
        return {
          albedo: frame(16, 20, 4, (ctx) => {
            for (let k = 0; k < 6; k++) {
              const x = 2 + k * 2;
              rect(ctx, x, 4 + (k % 3) * 2, 1, 14 - (k % 3) * 2, k % 2 ? PAL.thatch2 : PAL.thatch);
              rect(ctx, x, 3 + (k % 3) * 2, 1, 2, PAL.wood2);
            }
          }),
        };
      // Street ends (DEC-090): the variant is the end's facing (n, e, s, w, ne, nw, se, sw), +8 the second look.
      case "bollards": {
        const f = v & 7;
        const lit = v >= 8;
        return {
          albedo: frame(16, 20, 4, (ctx) => {
            // Three posts across the end: a row for n/s, a column for e/w, a diagonal otherwise.
            const at = f === 0 || f === 2 ? [[2, 9], [7, 9], [12, 9]] : f === 1 || f === 3 ? [[7, 3], [7, 9], [7, 15]] : f === 4 || f === 7 ? [[2, 15], [7, 9], [12, 3]] : [[2, 3], [7, 9], [12, 15]];
            for (const [x, y] of at as [number, number][]) {
              rect(ctx, x, y, 2, 5, PAL.ink);
              rect(ctx, x, y, 2, 1, lit ? PAL.lit : PAL.cream);
            }
          }),
        };
      }
      case "field_gate": {
        const f = v & 7;
        const steel = v >= 8;
        const c = steel ? PAL.grey : PAL.wood2;
        return {
          albedo: frame(16, 20, 4, (ctx) => {
            if (f === 1 || f === 3) {
              rect(ctx, 6, 2, 2, 18, PAL.ink);
              for (let k = 0; k < 3; k++) rect(ctx, 8, 4 + k * 5, 2, 4, c);
            } else {
              rect(ctx, 0, 9, 2, 8, PAL.ink);
              rect(ctx, 14, 9, 2, 8, PAL.ink);
              for (const y of [10, 13, 16]) rect(ctx, 2, y, 12, 1, c);
              rect(ctx, 2, 10, 1, 7, c);
              for (let k = 0; k < 6; k++) rect(ctx, 3 + k * 2, 16 - k, 1, 1, c);
            }
          }),
        };
      }
      case "railing": {
        const f = v & 7;
        return {
          albedo: frame(16, 20, 4, (ctx) => {
            if (f === 1 || f === 3) {
              rect(ctx, f === 1 ? 13 : 2, 2, 1, 18, PAL.ink);
              for (let y = 3; y < 20; y += 5) rect(ctx, f === 1 ? 12 : 1, y, 3, 2, PAL.grey);
            } else if (f === 0 || f === 2) {
              const y = f === 0 ? 5 : 16;
              rect(ctx, 0, y, 16, 1, PAL.ink);
              rect(ctx, 0, y + 3, 16, 1, PAL.grey);
              for (let x = 1; x < 16; x += 5) rect(ctx, x, y, 1, 4, PAL.ink);
            } else {
              for (let k = 0; k < 16; k++) rect(ctx, f === 4 || f === 7 ? 15 - k : k, 4 + k, 1, 1, PAL.ink);
            }
          }),
        };
      }
      case "stepping_stones":
        return {
          albedo: frame(16, 16, 0, (ctx) => {
            for (const [x, y] of [
              [2, 6],
              [7, 9],
              [11, 4],
            ] as const) {
              rect(ctx, x, y, 4, 3, PAL.ink);
              rect(ctx, x, y, 4, 2, PAL.cream3);
            }
          }),
        };
      case "crane":
        return {
          albedo: frame(32, 48, 32, (ctx) => {
            rect(ctx, 6, 4, 3, 44, PAL.gold);
            rect(ctx, 0, 4, 32, 3, PAL.gold);
            rect(ctx, 26, 7, 1, 14, PAL.ink);
          }),
        };
      default:
        return {
          albedo: frame(16, 20, 4, (ctx) => {
            rect(ctx, 1, 0, 14, 6, PAL.flag);
            for (let x = 1; x < 15; x += 4) rect(ctx, x, 0, 2, 6, PAL.cream);
            rect(ctx, 2, 6, 12, 10, PAL.wood);
            rect(ctx, 3, 8, 10, 3, PAL.gold);
          }),
        };
    }
  }

  // ------------------------------------------------------------------ characters

  character(look: LookView, anim: AnimKey, facing: Facing, frame: number): FrameRef {
    const key = look.player ? -1 : (look.seed >>> 0) % 60;
    let sheet = this.characters.get(key);
    if (!sheet) {
      sheet = this.layer(CW * 9, CH);
      paintCharacterSheet(sheet.ctx, key);
      this.characters.set(key, sheet);
    }
    const f = anim === "idle" ? 0 : 1 + (Math.max(0, frame - 1) % this.walkFrames);
    const col = (facing === "down" ? 0 : facing === "up" ? 1 : 2) * 3 + f;
    return { image: sheet.canvas, x: col * CW, y: 0, w: CW, h: CH, ax: CW / 2, ay: CH - 1 };
  }

  // ------------------------------------------------------------------ lights and markers

  private lightPool(): FrameRef {
    if (this.pool) return this.pool;
    const w = 44;
    const h = 24;
    const { canvas, ctx } = this.layer(w, h);
    // Stepped bands of warm light, brightest in the middle (pixel-art pools are banded).
    const rings = 5;
    for (let k = 0; k < rings; k++) {
      const rx = (w / 2) * (1 - k / rings);
      const ry = (h / 2) * (1 - k / rings);
      ctx.globalAlpha = 0.14;
      for (let y = -Math.floor(ry); y <= Math.floor(ry); y++) {
        const half = Math.floor(rx * Math.sqrt(Math.max(0, 1 - (y * y) / (ry * ry))));
        rect(ctx, w / 2 - half, h / 2 + y, half * 2, 1, "#ffb85c");
      }
    }
    ctx.globalAlpha = 1;
    this.pool = { image: canvas, x: 0, y: 0, w, h, ax: w / 2, ay: h / 2 };
    return this.pool;
  }

  marker(kind: MarkerKind): FrameRef {
    const hit = this.markers.get(kind);
    if (hit) return hit;
    let f: FrameRef;
    if (kind === "here") {
      const { canvas, ctx } = this.layer(16, 8);
      rect(ctx, 3, 0, 10, 8, PAL.ink);
      rect(ctx, 1, 2, 14, 4, PAL.ink);
      rect(ctx, 4, 1, 8, 6, PAL.gold);
      rect(ctx, 2, 3, 12, 2, PAL.gold);
      rect(ctx, 5, 2, 6, 4, PAL.ink);
      rect(ctx, 4, 3, 8, 2, PAL.ink);
      f = { image: canvas, x: 0, y: 0, w: 16, h: 8, ax: 8, ay: 4 };
    } else if (kind === "pin") {
      const { canvas, ctx } = this.layer(11, 13);
      rect(ctx, 1, 0, 9, 8, PAL.ink);
      rect(ctx, 0, 1, 11, 6, PAL.ink);
      rect(ctx, 3, 8, 5, 2, PAL.ink);
      rect(ctx, 4, 10, 3, 2, PAL.ink);
      rect(ctx, 5, 12, 1, 1, PAL.ink);
      rect(ctx, 2, 1, 7, 6, PAL.gold);
      rect(ctx, 1, 2, 9, 4, PAL.gold);
      rect(ctx, 4, 7, 3, 2, PAL.gold);
      rect(ctx, 5, 9, 1, 2, PAL.gold);
      rect(ctx, 4, 3, 3, 2, PAL.cream);
      f = { image: canvas, x: 0, y: 0, w: 11, h: 13, ax: 5, ay: 12 };
    } else if (kind === "target") {
      const { canvas, ctx } = this.layer(11, 11);
      for (let k = 0; k < 9; k++) {
        rect(ctx, k, k, 3, 3, PAL.ink);
        rect(ctx, 8 - k, k, 3, 3, PAL.ink);
      }
      for (let k = 0; k < 9; k++) {
        rect(ctx, k + 1, k + 1, 1, 1, PAL.gold);
        rect(ctx, 9 - k, k + 1, 1, 1, PAL.gold);
      }
      f = { image: canvas, x: 0, y: 0, w: 11, h: 11, ax: 5, ay: 5 };
    } else {
      const { canvas, ctx } = this.layer(20, 6);
      for (const x of [0, 15]) {
        rect(ctx, x, 0, 5, 6, PAL.ink);
        rect(ctx, x + 1, 1, 3, 4, PAL.gold);
      }
      rect(ctx, 2, 4, 16, 2, PAL.ink);
      rect(ctx, 2, 4, 16, 1, PAL.gold);
      f = { image: canvas, x: 0, y: 0, w: 20, h: 6, ax: 10, ay: 3 };
    }
    this.markers.set(kind, f);
    return f;
  }
}

// ---------------------------------------------------------------------------
// Ground tiles: a terrain kind, which sides join the same terrain group
// (N 1, E 2, S 4, W 8), and a variant (0..7) for small differences.

function paintGround(ctx: Ctx, t: number, m: number, v: number, dx: number, dy: number) {
  const N = (m & 1) !== 0;
  const E = (m & 2) !== 0;
  const S = (m & 4) !== 0;
  const Wj = (m & 8) !== 0;
  const n = (k: number) => noise01(t, m, v, k);
  const fleck = (c: string, k: number, count: number, w = 1, h = 1) => {
    for (let i = 0; i < count; i++) rect(ctx, dx + Math.floor(n(k + i * 2) * (T - w)), dy + Math.floor(n(k + i * 2 + 1) * (T - h)), w, h, c);
  };
  switch (t) {
    case Terrain.Road:
    case Terrain.Crossing: {
      rect(ctx, dx, dy, T, T, PAL.road);
      fleck(PAL.road2, 10, 3, 2, 1);
      const horizontal = (E || Wj) && !(N && S && !(E && Wj));
      if (t === Terrain.Crossing) {
        // Zebra stripes run with the traffic.
        const vertical = N && S && !(E && Wj);
        for (let k = 1; k < T; k += 4) {
          if (vertical) rect(ctx, dx + k, dy + 1, 2, T - 2, PAL.cream2);
          else rect(ctx, dx + 1, dy + k, T - 2, 2, PAL.cream2);
        }
      } else if (horizontal && S && !N) {
        // Centre line between the two lanes (every other tile: the variant's parity).
        if (v % 2 === 0) rect(ctx, dx + 3, dy + T - 1, 10, 1, PAL.cream3);
      } else if (!horizontal && E && !Wj) {
        if (v % 2 === 0) rect(ctx, dx + T - 1, dy + 3, 1, 10, PAL.cream3);
      }
      break;
    }
    case Terrain.Pavement:
    case Terrain.Door:
    case Terrain.Building: {
      rect(ctx, dx, dy, T, T, PAL.pavement);
      rect(ctx, dx, dy, T, 1, PAL.pavement2);
      rect(ctx, dx, dy, 1, T, PAL.pavement2);
      rect(ctx, dx + 8, dy + 8, 8, 1, PAL.pavement2);
      rect(ctx, dx + 8, dy, 1, 8, PAL.pavement2);
      // Kerbs where the pavement ends.
      if (!S) rect(ctx, dx, dy + T - 2, T, 2, PAL.slate3);
      if (!N) rect(ctx, dx, dy, T, 2, PAL.slate3);
      if (!E) rect(ctx, dx + T - 2, dy, 2, T, PAL.slate3);
      if (!Wj) rect(ctx, dx, dy, 2, T, PAL.slate3);
      break;
    }
    case Terrain.Plaza:
      rect(ctx, dx, dy, T, T, PAL.plaza);
      for (let k = 0; k < 4; k++) rect(ctx, dx + (k % 2) * 8 + (v % 2) * 4, dy + Math.floor(k / 2) * 8, 4, 4, PAL.plaza2);
      break;
    case Terrain.Park:
      rect(ctx, dx, dy, T, T, PAL.park);
      fleck(PAL.grass2, 20, 3, 2, 1);
      fleck(PAL.grass3, 30, 2);
      break;
    case Terrain.Garden:
      rect(ctx, dx, dy, T, T, PAL.grass3);
      fleck(PAL.grass, 20, 3, 2, 1);
      fleck(v < 3 ? PAL.red : v < 6 ? PAL.gold : PAL.cream, 42, 2);
      break;
    case Terrain.Field:
      rect(ctx, dx, dy, T, T, PAL.field);
      for (let k = 1; k < T; k += 3) rect(ctx, dx, dy + k, T, 1, PAL.field2);
      fleck(PAL.thatch, 20, 3);
      break;
    case Terrain.Path:
      rect(ctx, dx, dy, T, T, "#bba67a");
      fleck(PAL.dirt, 20, 4);
      break;
    case Terrain.Dirt:
      rect(ctx, dx, dy, T, T, PAL.dirt);
      fleck(PAL.dirt2, 20, 4, 2, 1);
      break;
    case Terrain.Yard:
      rect(ctx, dx, dy, T, T, PAL.yard);
      rect(ctx, dx, dy, T, 1, PAL.yard2);
      rect(ctx, dx, dy, 1, T, PAL.yard2);
      fleck(PAL.yard2, 20, 2, 3, 1);
      break;
    case Terrain.Sand:
      rect(ctx, dx, dy, T, T, PAL.sand);
      fleck(PAL.thatch, 20, 3);
      break;
    case Terrain.Water:
      rect(ctx, dx, dy, T, T, PAL.water);
      fleck(PAL.water2, 20, 2, 4, 1);
      fleck(PAL.waterLight, 30, 1, 3, 1);
      if (!N) rect(ctx, dx, dy, T, 2, PAL.waterLight);
      if (!S) rect(ctx, dx, dy + T - 1, T, 1, PAL.water2);
      if (!Wj) rect(ctx, dx, dy, 1, T, PAL.waterLight);
      if (!E) rect(ctx, dx + T - 1, dy, 1, T, PAL.waterLight);
      break;
    case Terrain.Quay:
      rect(ctx, dx, dy, T, T, PAL.pavement2);
      for (let k = 0; k < 4; k++) rect(ctx, dx, dy + k * 4, T, 1, PAL.slate3);
      for (let k = 0; k < 4; k++) rect(ctx, dx + (k % 2) * 4 + 2 + (k > 1 ? 8 : 0), dy + k * 4, 1, 4, PAL.slate3);
      if (!S) rect(ctx, dx, dy + T - 2, T, 2, PAL.ink);
      if (!E) rect(ctx, dx + T - 2, dy, 2, T, PAL.ink);
      if (!Wj) rect(ctx, dx, dy, 2, T, PAL.ink);
      if (!N) rect(ctx, dx, dy, T, 2, PAL.slate);
      break;
    case Terrain.Pier:
      rect(ctx, dx, dy, T, T, PAL.wood);
      for (let k = 0; k < T; k += 4) rect(ctx, dx, dy + k, T, 1, PAL.wood2);
      break;
    // ---- layout v2 (DEC-090, §7.7): readable without the pixel-art pack
    case Terrain.Creek:
    case Terrain.Ford: {
      // A one-tile stream: a channel through the middle, reaching the sides it joins.
      rect(ctx, dx, dy, T, T, PAL.grass);
      const c = t === Terrain.Ford ? PAL.waterLight : PAL.water;
      rect(ctx, dx + 4, dy + 4, 8, 8, c);
      if (N) rect(ctx, dx + 5, dy, 6, 5, c);
      if (S) rect(ctx, dx + 5, dy + 11, 6, 5, c);
      if (Wj) rect(ctx, dx, dy + 5, 5, 6, c);
      if (E) rect(ctx, dx + 11, dy + 5, 5, 6, c);
      if (t === Terrain.Ford) for (let k = 0; k < 3; k++) rect(ctx, dx + 3 + k * 4, dy + 6 + (k % 2) * 3, 3, 2, PAL.cream3);
      else fleck(PAL.waterLight, 30, 1, 2, 1);
      break;
    }
    case Terrain.Rock:
      rect(ctx, dx, dy, T, T, PAL.slate3);
      fleck(PAL.grey, 20, 4, 3, 1);
      fleck(PAL.slate2, 30, 3, 1, 3);
      if (!N) rect(ctx, dx, dy, T, 2, PAL.grey);
      if (!S) rect(ctx, dx, dy + T - 2, T, 2, PAL.slate);
      break;
    case Terrain.Embankment:
      // A retaining wall or bank face (it faces south: the view sees it), a grass lip on top.
      rect(ctx, dx, dy, T, T, PAL.cream3);
      for (let y = 4; y < T; y += 4) rect(ctx, dx, dy + y, T, 1, PAL.slate3);
      for (let y = 4; y < T; y += 4) for (let x = (y % 8) * 1; x < T; x += 8) rect(ctx, dx + x, dy + y, 1, 4, PAL.slate3);
      rect(ctx, dx, dy, T, 3, PAL.grass3);
      rect(ctx, dx, dy + 3, T, 1, PAL.grass2);
      rect(ctx, dx, dy + T - 1, T, 1, PAL.ink);
      break;
    case Terrain.Rail: {
      rect(ctx, dx, dy, T, T, PAL.dirt2);
      fleck(PAL.grey, 20, 4);
      const horiz = (E || Wj) && !(N && S && !(E || Wj));
      if (horiz) {
        for (let x = 1; x < T; x += 4) rect(ctx, dx + x, dy + 3, 2, 10, PAL.wood2);
        rect(ctx, dx, dy + 5, T, 1, PAL.grey);
        rect(ctx, dx, dy + 10, T, 1, PAL.grey);
      } else {
        for (let y = 1; y < T; y += 4) rect(ctx, dx + 3, dy + y, 10, 2, PAL.wood2);
        rect(ctx, dx + 5, dy, 1, T, PAL.grey);
        rect(ctx, dx + 10, dy, 1, T, PAL.grey);
      }
      break;
    }
    case Terrain.Reeds:
      rect(ctx, dx, dy, T, T, PAL.grass2);
      for (let k = 0; k < 6; k++) rect(ctx, dx + 1 + Math.floor(n(40 + k) * 13), dy + 3 + Math.floor(n(50 + k) * 6), 1, 6, PAL.thatch2);
      break;
    case Terrain.Bridge: {
      // A deck: stone setts, parapets where the deck ends sideways, the arch face on its south side.
      rect(ctx, dx, dy, T, T, PAL.plaza2);
      for (let y = 3; y < T; y += 4) rect(ctx, dx, dy + y, T, 1, PAL.pavement2);
      if (!N) rect(ctx, dx, dy, T, 3, PAL.cream3);
      if (!Wj) rect(ctx, dx, dy, 3, T, PAL.cream3);
      if (!E) rect(ctx, dx + T - 3, dy, 3, T, PAL.cream3);
      if (!S) {
        rect(ctx, dx, dy + T - 5, T, 5, PAL.slate3);
        rect(ctx, dx + 4, dy + T - 3, 8, 3, PAL.water2);
        rect(ctx, dx, dy + T - 5, T, 1, PAL.cream3);
      }
      break;
    }
    case Terrain.Pit:
      rect(ctx, dx, dy, T, T, PAL.dirt2);
      fleck(PAL.ink, 20, 3, 2, 1);
      break;
    case Terrain.Steps:
      rect(ctx, dx, dy, T, T, PAL.pavement);
      for (let y = 2; y < T; y += 4) {
        rect(ctx, dx, dy + y, T, 1, PAL.slate3);
        rect(ctx, dx, dy + y + 1, T, 1, PAL.cream2);
      }
      break;
    default:
      rect(ctx, dx, dy, T, T, PAL.grass);
      fleck(PAL.grass2, 20, 4, 2, 1);
      fleck(PAL.grass3, 30, 2);
      if (v === 7) fleck(n(51) < 0.5 ? PAL.cream : PAL.gold, 52, 1);
  }
}

// ---------------------------------------------------------------------------
// Characters: 16×24 frames, feet at the bottom centre; facings down, up and
// side (facing left; the renderer mirrors it), each standing plus two steps.

const CW = 16;
const CH = 24;
const SKIN = ["#f1c9a5", "#d9a47a", "#b07a50", "#7a4f32"];
const HAIR = ["#2a1f1a", "#5a3e2a", "#a8721a", "#c9c6bc", "#7e3a31", PAL.ink];
const SHIRT = [PAL.blue, PAL.red, PAL.teal, "#7f6aa8", PAL.grass, PAL.cream2, PAL.flag, PAL.slate3, "#c76f9a"];
const PANTS = [PAL.navy2, PAL.slate2, PAL.wood2, "#4f5f7a"];

function paintCharacterSheet(ctx: Ctx, key: number) {
  const player = key < 0;
  const n = (k: number) => noise01(0x5eed, key, k);
  const skin = player ? (SKIN[1] as string) : pick(SKIN, n(1));
  const hair = player ? "#2a1f1a" : pick(HAIR, n(2));
  const shirt = player ? PAL.gold : pick(SHIRT, n(3));
  const shirtD = player ? PAL.goldDark : PAL.ink;
  const pants = player ? PAL.navy : pick(PANTS, n(4));
  const long = !player && n(5) < 0.35;
  for (let facing = 0; facing < 3; facing++) {
    for (let f = 0; f < 3; f++) {
      const ox = (facing * 3 + f) * CW;
      const parts: [number, number, number, number, string][] = [];
      const P = (x: number, y: number, w: number, h: number, c: string) => parts.push([x, y, w, h, c]);
      const side = facing === 2;
      // Legs (y 16..22) and shoes.
      if (side) {
        if (f === 0) P(7, 16, 3, 7, pants);
        else if (f === 1) {
          P(5, 16, 2, 7, pants);
          P(9, 16, 2, 6, pants);
        } else {
          P(6, 16, 2, 6, pants);
          P(8, 16, 2, 7, pants);
        }
      } else {
        P(6, 16, 2, f === 1 ? 6 : 7, pants);
        P(9, 16, 2, f === 2 ? 6 : 7, pants);
      }
      // Body and arms (y 10..16).
      P(5, 10, 7, 7, shirt);
      if (!side) {
        P(4, 11 + (f === 1 ? 1 : 0), 1, 5, shirt);
        P(12, 11 + (f === 2 ? 1 : 0), 1, 5, shirt);
      } else P(7, 11 + (f === 0 ? 0 : 1), 2, 5, shirt);
      // Head (y 2..9).
      P(5, 3, 7, 7, skin);
      if (facing === 1) P(5, 3, 7, 6, hair);
      else if (side) {
        P(5, 3, 7, 2, hair);
        P(10, 3, 2, long ? 8 : 5, hair);
      } else {
        P(5, 3, 7, 2, hair);
        if (long) {
          P(5, 5, 1, 6, hair);
          P(11, 5, 1, 6, hair);
        }
      }
      // Outline first (the silhouette in ink, one pixel out), then colour.
      for (const [ddx, ddy] of [
        [-1, 0],
        [1, 0],
        [0, -1],
        [0, 1],
      ] as const) {
        for (const [x, y, w, h] of parts) rect(ctx, ox + x + ddx, y + ddy, w, h, PAL.ink);
      }
      for (const [x, y, w, h, c] of parts) rect(ctx, ox + x, y, w, h, c);
      rect(ctx, ox + 5, 16, 7, 1, shirtD);
      rect(ctx, ox + 5, 22, 7, 1, PAL.ink);
      if (facing === 0) {
        rect(ctx, ox + 6, 6, 1, 1, PAL.ink);
        rect(ctx, ox + 10, 6, 1, 1, PAL.ink);
        if (player) rect(ctx, ox + 8, 10, 1, 4, PAL.flag);
      } else if (side) rect(ctx, ox + 6, 6, 1, 1, PAL.ink);
    }
  }
}

/** The street ends' turning-circle corner cells of a map (cell → which corner of its bulb: 1 top-left, 2 top-right, 4 bottom-right, 8 bottom-left). */
const bulbCorners = new WeakMap<TownMap, Map<number, number>>();

function turningCorner(map: TownMap, x: number, y: number): number {
  if (!map.streetEnds?.length) return 0;
  let m = bulbCorners.get(map);
  if (!m) {
    m = new Map();
    for (const e of map.streetEnds) {
      if (e.kind !== "turning_circle" || e.w < 2 || e.h < 2) continue;
      const W = map.width;
      m.set(e.y * W + e.x, 1);
      m.set(e.y * W + e.x + e.w - 1, 2);
      m.set((e.y + e.h - 1) * W + e.x + e.w - 1, 4);
      m.set((e.y + e.h - 1) * W + e.x, 8);
    }
    bulbCorners.set(map, m);
  }
  return m.get(y * map.width + x) ?? 0;
}
