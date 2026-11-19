// Placeholder pixel art for the hub, drawn in code (no image assets): 16×16
// tiles, walls and roofs by place kind and neighbourhood, an icon sign on
// every named place, four-direction walkers, lamps, trees and markers, all
// from the small palette in ./palette. It implements `TownArt`, so the real
// art (atlases from the art tool) replaces it without touching the renderer.
// Sprites are made lazily and cached per map.
import { noise01 } from "../rng";
import { ROOF_ROWS, Terrain, type TownBuilding, type TownMap, type TownProp } from "../townLayout";
import { domCanvasFactory, type CanvasFactory, type Ctx, type Layer, type MarkerKind, type Sprite, type TownArt } from "./art";
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
const glyphFor = (kind: string): readonly string[] =>
  GLYPHS[kind] ?? (kind === "council_chamber" || kind === "regional_assembly" || kind === "parliament" ? (GLYPHS.dome as readonly string[]) : kind === "town_hall" ? (GLYPHS.civic as readonly string[]) : (GLYPHS.home as readonly string[]));

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

interface BuildingSpec {
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

function specFor(map: TownMap, b: TownBuilding): BuildingSpec {
  const hood = map.districts[b.district]?.kind ?? "inner";
  const n = (k: number) => noise01(b.style, k);
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
  const base: BuildingSpec = {
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

interface Built {
  sprite: Sprite;
  windows: { x: number; y: number; w: number; h: number }[];
  lights: Sprite | null | undefined;
}

export interface PlaceholderArtOptions {
  createCanvas?: CanvasFactory;
}

export class PlaceholderArt implements TownArt {
  readonly tile = T;
  readonly outside = PAL.grass2;
  readonly shadow = "#141a33";
  readonly walkFrames = 2;
  private readonly make: CanvasFactory;
  private buildings = new WeakMap<TownMap, Map<number, Built>>();
  private props = new Map<string, Sprite>();
  private walkers = new Map<number, Layer>();
  private markers = new Map<MarkerKind, Sprite>();
  private pool: Sprite | null = null;
  /** Sprites made so far (tests: art is made once and cached). */
  made = 0;

  constructor(options: PlaceholderArtOptions = {}) {
    this.make = options.createCanvas ?? domCanvasFactory;
  }

  private layer(w: number, h: number): Layer {
    this.made++;
    const l = this.make(w, h);
    l.ctx.imageSmoothingEnabled = false;
    return l;
  }

  // ------------------------------------------------------------------ ground

  ground(ctx: Ctx, map: TownMap, x: number, y: number, dx: number, dy: number): void {
    const W = map.width;
    const at = (cx: number, cy: number) => (cx >= 0 && cy >= 0 && cx < W && cy < map.height ? (map.terrain[cy * W + cx] as number) : -1);
    const t = at(x, y);
    const n = (k: number) => noise01(map.seed, x, y, k);
    const fleck = (c: string, k: number, count: number, w = 1, h = 1) => {
      for (let i = 0; i < count; i++) rect(ctx, dx + Math.floor(n(k + i * 2) * (T - w)), dy + Math.floor(n(k + i * 2 + 1) * (T - h)), w, h, c);
    };
    const roadish = (v: number) => v === Terrain.Road || v === Terrain.Crossing;
    switch (t) {
      case Terrain.Road:
      case Terrain.Crossing: {
        rect(ctx, dx, dy, T, T, PAL.road);
        fleck(PAL.road2, 10, 3, 2, 1);
        const horizontal = roadish(at(x - 1, y)) || roadish(at(x + 1, y)) ? !(roadish(at(x, y - 1)) && roadish(at(x, y + 1)) && !(roadish(at(x - 1, y)) && roadish(at(x + 1, y)))) : false;
        if (t === Terrain.Crossing) {
          // Zebra stripes run with the traffic.
          const vertical = at(x, y - 1) === Terrain.Road || at(x, y + 1) === Terrain.Road ? !(at(x - 1, y) === Terrain.Road || at(x + 1, y) === Terrain.Road) : false;
          for (let k = 1; k < T; k += 4) {
            if (vertical) rect(ctx, dx + k, dy + 1, 2, T - 2, PAL.cream2);
            else rect(ctx, dx + 1, dy + k, T - 2, 2, PAL.cream2);
          }
        } else if (horizontal && at(x, y + 1) === Terrain.Road && !roadish(at(x, y - 1))) {
          // Centre line between the two lanes.
          if (x % 2 === 0) rect(ctx, dx + 3, dy + T - 1, 10, 1, PAL.cream3);
        } else if (!horizontal && at(x + 1, y) === Terrain.Road && !roadish(at(x - 1, y))) {
          if (y % 2 === 0) rect(ctx, dx + T - 1, dy + 3, 1, 10, PAL.cream3);
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
        // Kerbs where the pavement meets the road.
        if (roadish(at(x, y + 1))) rect(ctx, dx, dy + T - 2, T, 2, PAL.slate3);
        if (roadish(at(x, y - 1))) rect(ctx, dx, dy, T, 2, PAL.slate3);
        if (roadish(at(x + 1, y))) rect(ctx, dx + T - 2, dy, 2, T, PAL.slate3);
        if (roadish(at(x - 1, y))) rect(ctx, dx, dy, 2, T, PAL.slate3);
        break;
      }
      case Terrain.Plaza: {
        rect(ctx, dx, dy, T, T, PAL.plaza);
        for (let k = 0; k < 4; k++) rect(ctx, dx + (k % 2) * 8 + ((y % 2) * 4) % 8, dy + Math.floor(k / 2) * 8, 4, 4, PAL.plaza2);
        break;
      }
      case Terrain.Park:
        rect(ctx, dx, dy, T, T, PAL.park);
        fleck(PAL.grass2, 20, 3, 2, 1);
        fleck(PAL.grass3, 30, 2);
        break;
      case Terrain.Garden:
        rect(ctx, dx, dy, T, T, PAL.grass3);
        fleck(PAL.grass, 20, 3, 2, 1);
        if (n(40) < 0.5) fleck(n(41) < 0.5 ? PAL.red : PAL.gold, 42, 2);
        else fleck(PAL.cream, 42, 2);
        break;
      case Terrain.Field: {
        rect(ctx, dx, dy, T, T, PAL.field);
        for (let k = 1; k < T; k += 3) rect(ctx, dx, dy + k, T, 1, PAL.field2);
        fleck(PAL.thatch, 20, 3);
        break;
      }
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
      case Terrain.Water: {
        rect(ctx, dx, dy, T, T, PAL.water);
        fleck(PAL.water2, 20, 2, 4, 1);
        fleck(PAL.waterLight, 30, 1, 3, 1);
        const shore = (v: number) => v !== Terrain.Water && v !== Terrain.Pier && v !== -1;
        if (shore(at(x, y - 1))) rect(ctx, dx, dy, T, 2, PAL.waterLight);
        if (shore(at(x, y + 1))) rect(ctx, dx, dy + T - 1, T, 1, PAL.water2);
        if (shore(at(x - 1, y))) rect(ctx, dx, dy, 1, T, PAL.waterLight);
        if (shore(at(x + 1, y))) rect(ctx, dx + T - 1, dy, 1, T, PAL.waterLight);
        break;
      }
      case Terrain.Quay: {
        rect(ctx, dx, dy, T, T, PAL.pavement2);
        for (let k = 0; k < 4; k++) rect(ctx, dx, dy + k * 4, T, 1, PAL.slate3);
        for (let k = 0; k < 4; k++) rect(ctx, dx + ((k % 2) * 4 + 2) + (k > 1 ? 8 : 0), dy + Math.floor(k) * 4, 1, 4, PAL.slate3);
        if (at(x, y + 1) === Terrain.Water) rect(ctx, dx, dy + T - 2, T, 2, PAL.ink);
        if (at(x + 1, y) === Terrain.Water) rect(ctx, dx + T - 2, dy, 2, T, PAL.ink);
        if (at(x - 1, y) === Terrain.Water) rect(ctx, dx, dy, 2, T, PAL.ink);
        if (at(x, y - 1) === Terrain.Water) rect(ctx, dx, dy, T, 2, PAL.slate);
        break;
      }
      case Terrain.Pier:
        rect(ctx, dx, dy, T, T, PAL.wood);
        for (let k = 0; k < T; k += 4) rect(ctx, dx, dy + k, T, 1, PAL.wood2);
        break;
      default:
        rect(ctx, dx, dy, T, T, PAL.grass);
        fleck(PAL.grass2, 20, 4, 2, 1);
        fleck(PAL.grass3, 30, 2);
        if (n(50) < 0.08) fleck(n(51) < 0.5 ? PAL.cream : PAL.gold, 52, 1);
    }
  }

  // ------------------------------------------------------------------ buildings

  private built(map: TownMap, b: TownBuilding): Built {
    let cache = this.buildings.get(map);
    if (!cache) {
      cache = new Map();
      this.buildings.set(map, cache);
    }
    const hit = cache.get(b.index);
    if (hit) return hit;
    const made = this.paintBuilding(map, b);
    cache.set(b.index, made);
    return made;
  }

  building(map: TownMap, b: TownBuilding): Sprite {
    return this.built(map, b).sprite;
  }

  buildingHeight(b: TownBuilding): number {
    if (b.landmark) return 2.2;
    if (b.kind === "warehouse" || b.kind === "factory" || b.kind === "mill") return 1.6;
    if (b.kind === "shed" || b.kind === "park") return 0.7;
    return 1 + Math.min(0.6, b.h * 0.12);
  }

  buildingLights(map: TownMap, b: TownBuilding): Sprite | null {
    const made = this.built(map, b);
    if (made.lights !== undefined) return made.lights;
    if (made.windows.length === 0) {
      made.lights = null;
      return null;
    }
    const s = made.sprite;
    const l = this.layer(s.w, s.h);
    for (const w of made.windows) {
      l.ctx.globalAlpha = 0.35;
      rect(l.ctx, w.x - 1, w.y - 1, w.w + 2, w.h + 2, PAL.lit);
      l.ctx.globalAlpha = 1;
      rect(l.ctx, w.x, w.y, w.w, w.h, PAL.lit);
      rect(l.ctx, w.x + 1, w.y + 1, Math.max(1, w.w - 2), Math.max(1, w.h - 2), PAL.litCore);
    }
    made.lights = { image: l.canvas, sx: 0, sy: 0, w: s.w, h: s.h, ax: s.ax, ay: s.ay };
    return made.lights;
  }

  private paintBuilding(map: TownMap, b: TownBuilding): Built {
    const W = b.w * T;
    const H = (b.h + ROOF_ROWS) * T;
    const { canvas, ctx } = this.layer(W, H);
    const s = specFor(map, b);
    const n = (k: number) => noise01(b.style, k);
    const windows: Built["windows"] = [];
    const wallTop = H - s.wallH;
    const doorPx = (b.doorX - b.x) * T;

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
    } else if (s.roof === "flat" || s.roof === "civic" || s.roof === "dome") {
      rect(ctx, 0, 0, W, roofH, rd);
      rect(ctx, 2, 2, W - 4, roofH - 4, rl);
      // Vents and skylights.
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
    if (s.roof === "civic" || s.roof === "dome") {
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
    if (s.roof === "civic" || s.roof === "dome") rows.push(wallTop + 10, wallTop + Math.max(10, s.wallH - 14));
    else if (s.wallH >= 26) rows.push(wallTop + 4, wallTop + s.wallH - 12);
    else rows.push(wallTop + Math.max(3, s.wallH - 13));
    const lower = rows[rows.length - 1] as number;
    for (const wy of rows) {
      if (s.shop && wy === lower) continue;
      const step = s.roof === "civic" || s.roof === "dome" ? 8 : s.wall === "metal" ? 12 : 7;
      for (let x = 3; x + winW <= W - 3; x += step) {
        if (wy === lower && !clearOfDoor(x, winW)) continue;
        if (s.roof === "civic" || s.roof === "dome") {
          // Between the columns.
          if (x % 8 !== 3) continue;
        }
        rect(ctx, x - 1, wy - 1, winW + 2, winH + 2, PAL.ink);
        rect(ctx, x, wy, winW, winH, PAL.glass);
        rect(ctx, x, wy, winW, 1, PAL.glass2);
        rect(ctx, x, wy + 1, 1, winH - 1, "#9db6cc");
        windows.push({ x, y: wy, w: winW, h: winH });
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
        windows.push({ x: a, y: sy + 1, w: c - a, h: 7 });
      }
    }

    // ---- door
    const big = s.roof === "civic" || s.roof === "dome" || b.kind === "warehouse" || b.kind === "factory";
    const dw = big ? 10 : 7;
    const dh = big ? 13 : 11;
    const dx = doorPx + Math.floor((T - dw) / 2);
    rect(ctx, dx - 1, H - dh - 1, dw + 2, dh + 1, PAL.ink);
    rect(ctx, dx, H - dh, dw, dh, s.door);
    if (b.kind === "warehouse") for (let y = H - dh + 2; y < H; y += 2) rect(ctx, dx, y, dw, 1, PAL.slate3);
    else {
      rect(ctx, dx + dw - 2, H - Math.floor(dh / 2), 1, 1, PAL.gold);
      if (big) rect(ctx, dx + Math.floor(dw / 2), H - dh, 1, dh, PAL.ink);
    }
    if (b.kind === "barn") {
      for (let k = 0; k < dh; k++) {
        rect(ctx, dx + Math.floor((k * (dw - 1)) / dh), H - dh + k, 1, 1, PAL.cream3);
        rect(ctx, dx + dw - 1 - Math.floor((k * (dw - 1)) / dh), H - dh + k, 1, 1, PAL.cream3);
      }
    }

    // ---- sign over named places (and the player's home)
    if (b.place !== null || b.home) {
      const g = glyphFor(b.home ? "home" : b.kind);
      const bw = 11;
      const bh = 11;
      let sx = doorPx + 8 - Math.floor(bw / 2);
      sx = Math.max(1, Math.min(W - bw - 1, sx));
      const sy = Math.max(1, Math.min(H - dh - bh - 2, wallTop - (s.wallH < 22 ? 6 : 1)));
      rect(ctx, sx - 1, sy - 1, bw + 2, bh + 2, PAL.ink);
      rect(ctx, sx, sy, bw, bh, b.home ? PAL.gold : (SIGN[b.kind] ?? PAL.cream));
      for (let gy = 0; gy < 7; gy++) {
        const row = g[gy] ?? "";
        for (let gx = 0; gx < 7; gx++) if (row[gx] === "#") rect(ctx, sx + 2 + gx, sy + 2 + gy, 1, 1, PAL.ink);
      }
    }

    const sprite: Sprite = { image: canvas, sx: 0, sy: 0, w: W, h: H, ax: 0, ay: ROOF_ROWS * T };
    return { sprite, windows, lights: undefined };
  }

  // ------------------------------------------------------------------ props

  prop(_map: TownMap, p: TownProp): Sprite {
    const v = p.kind === "tree" || p.kind === "pine" || p.kind === "flag" ? p.variant % 3 : 0;
    const key = `${p.kind}:${v}`;
    const hit = this.props.get(key);
    if (hit) return hit;
    const s = this.paintProp(p.kind, v);
    this.props.set(key, s);
    return s;
  }

  propHeight(p: TownProp): number {
    return p.kind === "tree" ? 1.3 : p.kind === "pine" ? 1.6 : p.kind === "fountain" ? 0.4 : 0;
  }

  lampLight(p: TownProp): { x: number; y: number } | null {
    return p.kind === "lamp" ? { x: 8, y: -1 } : null;
  }

  private paintProp(kind: TownProp["kind"], v: number): Sprite {
    const sprite = (w: number, h: number, ay: number, paint: (ctx: Ctx) => void): Sprite => {
      const { canvas, ctx } = this.layer(w, h);
      paint(ctx);
      return { image: canvas, sx: 0, sy: 0, w, h, ax: 0, ay };
    };
    switch (kind) {
      case "tree": {
        const [c, d, l] = [
          [PAL.tree, PAL.tree2, PAL.treeLight],
          ["#46773c", "#30592f", "#6ba552"],
          ["#3a6a45", "#284d35", "#5a9456"],
        ][v] as [string, string, string];
        return sprite(16, 26, 10, (ctx) => {
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
      }
      case "pine":
        return sprite(16, 28, 12, (ctx) => {
          rect(ctx, 7, 21, 2, 6, PAL.wood2);
          for (let i = 0; i < 20; i++) {
            const half = 1 + Math.floor((i % 7) * 0.9) + Math.floor(i / 7);
            rect(ctx, 8 - half - 1, 1 + i, (half + 1) * 2, 1, PAL.ink);
            rect(ctx, 8 - half, 1 + i, half * 2, 1, i % 7 < 2 ? PAL.treeLight : i % 7 > 4 ? PAL.tree2 : PAL.tree);
          }
        });
      case "lamp":
        return sprite(16, 30, 14, (ctx) => {
          rect(ctx, 7, 4, 2, 25, PAL.ink);
          rect(ctx, 7, 5, 1, 23, PAL.slate3);
          rect(ctx, 5, 0, 6, 4, PAL.ink);
          rect(ctx, 6, 1, 4, 2, PAL.cream);
          rect(ctx, 6, 28, 4, 2, PAL.ink);
        });
      case "bench":
        return sprite(16, 16, 0, (ctx) => {
          rect(ctx, 2, 6, 12, 2, PAL.wood2);
          rect(ctx, 2, 9, 12, 3, PAL.wood);
          rect(ctx, 2, 12, 12, 1, PAL.ink);
          rect(ctx, 3, 12, 1, 3, PAL.ink);
          rect(ctx, 12, 12, 1, 3, PAL.ink);
        });
      case "fountain":
        return sprite(32, 34, 2, (ctx) => {
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
      case "boat":
        return sprite(16, 16, 0, (ctx) => {
          rect(ctx, 2, 8, 12, 5, PAL.ink);
          rect(ctx, 3, 8, 10, 3, PAL.cream);
          rect(ctx, 3, 11, 10, 1, PAL.red);
          rect(ctx, 7, 2, 1, 6, PAL.wood2);
          rect(ctx, 8, 3, 4, 4, PAL.cream2);
        });
      case "planter":
        return sprite(16, 16, 0, (ctx) => {
          rect(ctx, 3, 8, 10, 6, PAL.ink);
          rect(ctx, 4, 9, 8, 4, PAL.wood);
          rect(ctx, 4, 5, 8, 4, PAL.grass);
          rect(ctx, 5, 5, 1, 1, PAL.red);
          rect(ctx, 8, 6, 1, 1, PAL.gold);
          rect(ctx, 10, 5, 1, 1, PAL.cream);
        });
      case "flag":
        return sprite(16, 40, 24, (ctx) => {
          rect(ctx, 3, 0, 2, 39, PAL.ink);
          rect(ctx, 3, 1, 1, 37, PAL.grey);
          rect(ctx, 5, 2, 9, 6, v === 1 ? PAL.flag : PAL.gold);
          rect(ctx, 5, 8, 9, 1, PAL.ink);
          rect(ctx, 2, 37, 4, 3, PAL.ink);
        });
      case "crane":
        return sprite(32, 48, 32, (ctx) => {
          rect(ctx, 6, 4, 3, 44, PAL.gold);
          rect(ctx, 0, 4, 32, 3, PAL.gold);
          rect(ctx, 26, 7, 1, 14, PAL.ink);
        });
      case "stall":
        return sprite(16, 20, 4, (ctx) => {
          rect(ctx, 1, 0, 14, 6, PAL.flag);
          for (let x = 1; x < 15; x += 4) rect(ctx, x, 0, 2, 6, PAL.cream);
          rect(ctx, 2, 6, 12, 10, PAL.wood);
          rect(ctx, 3, 8, 10, 3, PAL.gold);
        });
    }
  }

  // ------------------------------------------------------------------ walkers

  walker(variant: number, dir: number, frame: number): Sprite {
    const key = variant < 0 ? -1 : variant % 60;
    let sheet = this.walkers.get(key);
    if (!sheet) {
      sheet = this.layer(16 * 12, 16);
      paintWalkerSheet(sheet.ctx, key);
      this.walkers.set(key, sheet);
    }
    const f = Math.max(0, Math.min(2, frame));
    return { image: sheet.canvas, sx: ((dir & 3) * 3 + f) * 16, sy: 0, w: 16, h: 16, ax: 8, ay: 15 };
  }

  // ------------------------------------------------------------------ lights and markers

  lightPool(): Sprite {
    if (this.pool) return this.pool;
    const w = 44;
    const h = 24;
    const { canvas, ctx } = this.layer(w, h);
    // A stepped (pixel-art) ellipse of warm light, brightest in the middle.
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
    this.pool = { image: canvas, sx: 0, sy: 0, w, h, ax: w / 2, ay: h / 2 };
    return this.pool;
  }

  marker(kind: MarkerKind): Sprite {
    const hit = this.markers.get(kind);
    if (hit) return hit;
    let s: Sprite;
    if (kind === "here") {
      const { canvas, ctx } = this.layer(16, 8);
      rect(ctx, 3, 0, 10, 8, PAL.ink);
      rect(ctx, 1, 2, 14, 4, PAL.ink);
      rect(ctx, 4, 1, 8, 6, PAL.gold);
      rect(ctx, 2, 3, 12, 2, PAL.gold);
      rect(ctx, 5, 2, 6, 4, PAL.ink);
      rect(ctx, 4, 3, 8, 2, PAL.ink);
      s = { image: canvas, sx: 0, sy: 0, w: 16, h: 8, ax: 8, ay: 4 };
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
      s = { image: canvas, sx: 0, sy: 0, w: 11, h: 13, ax: 5, ay: 12 };
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
      s = { image: canvas, sx: 0, sy: 0, w: 11, h: 11, ax: 5, ay: 5 };
    } else {
      const { canvas, ctx } = this.layer(20, 6);
      for (const x of [0, 15]) {
        rect(ctx, x, 0, 5, 6, PAL.ink);
        rect(ctx, x + 1, 1, 3, 4, PAL.gold);
      }
      rect(ctx, 2, 4, 16, 2, PAL.ink);
      rect(ctx, 2, 4, 16, 1, PAL.gold);
      s = { image: canvas, sx: 0, sy: 0, w: 20, h: 6, ax: 10, ay: 3 };
    }
    this.markers.set(kind, s);
    return s;
  }
}

// ---------------------------------------------------------------------------
// Walkers: 4 directions × (stand + 2 steps), 16×16, feet at the bottom centre.

const SKIN = ["#f1c9a5", "#d9a47a", "#b07a50", "#7a4f32"];
const HAIR = ["#2a1f1a", "#5a3e2a", "#a8721a", "#c9c6bc", "#7e3a31", PAL.ink];
const SHIRT = [PAL.blue, PAL.red, PAL.teal, "#7f6aa8", PAL.grass, PAL.cream2, PAL.flag, PAL.slate3, "#c76f9a"];
const PANTS = [PAL.navy2, PAL.slate2, PAL.wood2, "#4f5f7a"];

function paintWalkerSheet(ctx: Ctx, key: number) {
  const player = key < 0;
  const n = (k: number) => noise01(0x5eed, key, k);
  const skin = player ? (SKIN[1] as string) : pick(SKIN, n(1));
  const hair = player ? "#2a1f1a" : pick(HAIR, n(2));
  const shirt = player ? PAL.gold : pick(SHIRT, n(3));
  const shirtD = player ? PAL.goldDark : PAL.ink;
  const pants = player ? PAL.navy : pick(PANTS, n(4));
  const long = !player && n(5) < 0.35;
  for (let dir = 0; dir < 4; dir++) {
    for (let f = 0; f < 3; f++) {
      const ox = (dir * 3 + f) * 16;
      const parts: [number, number, number, number, string][] = [];
      const P = (x: number, y: number, w: number, h: number, c: string) => parts.push([x, y, w, h, c]);
      const side = dir >= 2;
      const flip = dir === 3;
      // Legs.
      if (side) {
        if (f === 0) P(7, 11, 3, 4, pants);
        else if (f === 1) {
          P(5, 11, 2, 4, pants);
          P(9, 11, 2, 3, pants);
        } else {
          P(6, 11, 2, 3, pants);
          P(8, 11, 2, 4, pants);
        }
      } else {
        P(6, 11, 2, f === 1 ? 3 : 4, pants);
        P(9, 11, 2, f === 2 ? 3 : 4, pants);
      }
      // Body and arms.
      P(5, 7, 7, 5, shirt);
      if (!side) {
        P(4, 8 + (f === 1 ? 1 : 0), 1, 3, shirt);
        P(12, 8 + (f === 2 ? 1 : 0), 1, 3, shirt);
      } else {
        P(flip ? 7 : 9, 8 + (f === 0 ? 0 : 1), 2, 3, shirt);
      }
      // Head.
      P(5, 1, 7, 6, skin);
      if (dir === 1) P(5, 1, 7, 5, hair);
      else if (side) {
        P(5, 1, 7, 2, hair);
        P(flip ? 5 : 10, 1, 2, long ? 6 : 4, hair);
      } else {
        P(5, 1, 7, 2, hair);
        if (long) {
          P(5, 3, 1, 4, hair);
          P(11, 3, 1, 4, hair);
        }
      }
      // Outline first (the silhouette drawn in ink, one pixel out), then colour.
      for (const [dx, dy] of [
        [-1, 0],
        [1, 0],
        [0, -1],
        [0, 1],
      ] as const) {
        for (const [x, y, w, h] of parts) rect(ctx, ox + x + dx, y + dy, w, h, PAL.ink);
      }
      for (const [x, y, w, h, c] of parts) rect(ctx, ox + x, y, w, h, c);
      // Shading and face.
      rect(ctx, ox + 5, 11, 7, 1, shirtD);
      if (dir === 0) {
        rect(ctx, ox + 6, 4, 1, 1, PAL.ink);
        rect(ctx, ox + 10, 4, 1, 1, PAL.ink);
        if (player) rect(ctx, ox + 8, 7, 1, 3, PAL.flag);
      } else if (side) {
        rect(ctx, ox + (flip ? 10 : 6), 4, 1, 1, PAL.ink);
      }
    }
  }
}
