// Buildings: the facade grammar (§2.4) that lays the kit in ./buildingParts
// out on a lot, on the device (D10). Round 2 (the owner: "Buildings don't
// look that different? ... should have some character and shape other than
// just squares"):
//   - a lot holds one to three masses: L-shapes with a lower, set-back wing,
//     forecourts with steps, railings or a front garden, a yard behind a low
//     building, a canted corner (often with a turret) where a terrace meets a
//     street, and now and then a narrow gap;
//   - masses rise by their storeys (2–4 in a terrace), so the skyline steps;
//     roofs are gabled (ridge, verges, chimneys on the party walls),
//     front-gabled (a pointed gable with a round window), hipped (cut
//     corners), mansard (dormers), flat (parapet, cornice and clutter that
//     stands up: tanks, bulkheads, AC units), saw-toothed (industry) or a lean-to;
//   - facades vary in window type and rhythm (one per bay, pairs, wide
//     casements, tall first floors, small attics, strip glazing), string
//     courses and cornices, bay windows, flower boxes, drainpipes, ivy, and
//     four shopfronts (classic with a muted awning, glazed, signboard with a
//     hanging sign, and the landmarks' own);
//   - landmarks have their own designs: the town hall's clock tower, steps
//     and giant columns, the council chamber's dome, the newspaper's blade
//     sign, the diner's stripes and rooftop sign, the party office's posters
//     and flag, the union hall's gable and banner, the market's canopy and
//     stalls, the clinic's canopy and strip windows, the school's cupola and
//     railings;
//   - a building avoids the design of the building on its left: neighbours
//     never look like copies (`designOf`, and the lint in src/cli/lint).
// Everything is picked by rendezvous hashing of the visual seed, so a place
// looks the same every time. Windows and doors come back with their lit
// (emissive) images and a use tag; the building with shadow casters and a
// per-pixel receiver map for lamp light (what is ground, roof or wall).
import { hash32, rendezvous } from "../../src/core/hash";
import { blank, blit, crop, fill, grid, mirrorX, remap, tileFill, type Indexed } from "../../src/core/image";
import { ramp, type Palette } from "../../src/core/palette";
import {
  AC_UNIT,
  AWNING,
  BAY,
  BIN,
  BLEGEND,
  BSLOT,
  BSLOT_COUNT,
  CHIMNEY,
  CLOCK,
  COLUMN,
  CORNER_DOOR,
  CUPOLA,
  DOME,
  DOORS,
  DORMER,
  FASCIA,
  HANGING_SIGN,
  HATCH,
  LIT_OVERLAYS,
  PEDIMENT_HALF,
  PICTOGRAMS,
  PLANK_DOOR,
  PLANKS,
  SHED,
  SHOPFRONT,
  SIGN_BOARD,
  SKYLIGHT,
  STACK2,
  STRIP,
  TANK,
  TEX,
  TURRET,
  VENT,
  WINDOWS,
  WINDOWS_R2,
  type DoorKind,
} from "./buildingParts";

export type EndKind = "joined" | "street" | "gap";

/** What the kit needs to know (the renderer's `BuildingSpec`, without importing the game). */
export interface FacadeSpec {
  kind: string;
  hood: string;
  /** Footprint, tiles. */
  w: number;
  h: number;
  /** Door bay (tile column from the left). */
  doorX: number;
  storeys: number;
  visualSeed: number;
  named: boolean;
  home: boolean;
  landmark: boolean;
  conditions: readonly string[];
  /** Px the art may rise above its one roof row (0..32): what stands behind decides. */
  headroom?: number;
  /** What touches the lot's ends: another building, a street (a corner), or open ground. */
  ends?: { left: EndKind; right: EndKind };
  /** The building touching this one on the left in the same row (its design is avoided). */
  left?: FacadeSpec | null;
}

export type LightUse = "home" | "office" | "shop" | "civic" | "industrial";

export interface Opening {
  x: number;
  y: number;
  w: number;
  h: number;
  /** The lit state: only the lit pixels, same size as the opening (palette indices). */
  emissive: Indexed | null;
  use: LightUse;
}

export interface ShadowCaster {
  footprint: [number, number][];
  height: number;
}

/** Receiver classes (per pixel of the albedo) for lamp light. */
export const RECEIVER = { none: 0, ground: 1, roof: 2, /** A wall face: 3 + px above its base. */ face: 3 } as const;

export interface Facade {
  /** The lot plus its roof row and headroom: w × 16 by (h + 1) × 16 + headroom px. */
  albedo: Indexed;
  /** Px of the frame above the footprint's top (the anchor row): 16 + headroom. */
  ay: number;
  windows: Opening[];
  doors: Opening[];
  /** Per pixel: 0 nothing drawn, 1 ground (forecourts, yards), 2 roof, 3 + k a wall face k px above its base. */
  receivers: Uint8Array;
  /** The main mass's shadow (round 1's single caster) and every mass's (and tower's). */
  shadow: ShadowCaster;
  shadows: ShadowCaster[];
  /** The design's features (the neighbour-similarity lint compares them). */
  signature: string[];
}

const T = 16;
const GROUND = 24;
const UPPER = 18;

type Family = "civic" | "shop" | "house" | "industrial" | "institution";

export function familyOf(kind: string, landmark: boolean): Family {
  switch (kind) {
    case "town_hall":
    case "council_chamber":
    case "regional_assembly":
    case "parliament":
    case "executive_residence":
      return "civic";
    case "shop":
    case "diner":
    case "market":
    case "news_office":
    case "party_office":
    case "business_association":
      return "shop";
    case "house":
    case "terrace":
    case "home":
    case "farmhouse":
    case "boathouse":
      return "house";
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
      return landmark ? "civic" : "institution";
  }
}

export function lightUseOf(kind: string, family: Family): LightUse {
  if (family === "house") return "home";
  if (family === "civic") return "civic";
  if (family === "industrial") return "industrial";
  if (family === "shop" && (kind === "shop" || kind === "diner" || kind === "market")) return "shop";
  return kind === "hotel" ? "home" : "office";
}

// ---------------------------------------------------------------------------
// Materials

interface WallMat {
  id: string;
  ramp: string;
  /** Steps above the ramp's anchor used as the wall's base (painted pastel fronts use +1). */
  off: number;
  tex: "brick" | "render" | "stone" | "cladding";
  /** Window frames: light (on brick and stone) or dark (on pale render). */
  frames: "light" | "dark";
}

const WALLS: Record<string, WallMat> = {
  brick: { id: "brick", ramp: "brick", off: 0, tex: "brick", frames: "light" },
  brickDark: { id: "brickDark", ramp: "brick", off: -1, tex: "brick", frames: "light" },
  brickPale: { id: "brickPale", ramp: "terracotta", off: 0, tex: "brick", frames: "light" },
  stone: { id: "stone", ramp: "stone", off: 0, tex: "stone", frames: "dark" },
  stoneGrey: { id: "stoneGrey", ramp: "paving", off: 0, tex: "stone", frames: "dark" },
  render: { id: "render", ramp: "render", off: 0, tex: "render", frames: "dark" },
  sky: { id: "sky", ramp: "cloth_sky", off: 1, tex: "render", frames: "light" },
  mint: { id: "mint", ramp: "cloth_teal", off: 1, tex: "render", frames: "light" },
  butter: { id: "butter", ramp: "cloth_mustard", off: 1, tex: "render", frames: "dark" },
  peach: { id: "peach", ramp: "terracotta", off: 1, tex: "render", frames: "light" },
  cladding: { id: "cladding", ramp: "slate", off: 0, tex: "cladding", frames: "light" },
  timber: { id: "timber", ramp: "wood", off: 0, tex: "cladding", frames: "light" },
};

type Weighted = readonly (readonly [string, number])[];

const HOOD_WALLS: Record<string, Weighted> = {
  core: [["stone", 3], ["stoneGrey", 2], ["render", 2], ["brick", 2], ["brickDark", 1], ["butter", 1]],
  inner: [["brick", 4], ["brickDark", 2], ["brickPale", 2], ["render", 2], ["sky", 1], ["mint", 1], ["peach", 1], ["butter", 1]],
  suburb: [["render", 3], ["brick", 2], ["brickPale", 2], ["butter", 1], ["mint", 1], ["sky", 1]],
  waterfront: [["sky", 2], ["mint", 2], ["render", 2], ["peach", 1], ["brick", 1], ["timber", 1]],
  industrial: [["brickDark", 3], ["cladding", 3], ["brick", 1]],
  rural: [["render", 3], ["stone", 2], ["brick", 1], ["timber", 1]],
};

/** Pitched-roof material: slate in the older centre and by the water, clay tiles in the newer suburbs. */
const HOOD_ROOFS: Record<string, Weighted> = {
  core: [["slates", 4], ["tiles", 1]],
  inner: [["tiles", 3], ["slates", 3]],
  suburb: [["tiles", 4], ["slates", 1]],
  waterfront: [["slates", 3], ["tiles", 1]],
  industrial: [["slates", 2], ["tiles", 1]],
  rural: [["tiles", 2], ["slates", 1]],
};

const DOOR_PAINTS = ["cloth_navy", "cloth_green", "cloth_red", "cloth_teal", "wood", "cloth_charcoal", "cloth_plum", "cloth_mustard"] as const;
/** Awnings in muted colours (round 1's party colours were the most saturated thing on screen): a ramp and the step used. */
const AWNINGS: readonly (readonly [string, number])[] = [
  ["cloth_green", -1],
  ["cloth_red", -1],
  ["wood", 0],
  ["slate", 0],
  ["cloth_navy", 0],
  ["terracotta", -1],
  ["cloth_teal", -1],
  ["cloth_mustard", -1],
  ["cloth_plum", -1],
  ["cloth_charcoal", 0],
];

/** Sign boards by kind (the pictogram's ground). */
const SIGN_RAMP: Record<string, string> = {
  town_hall: "lamp",
  council_chamber: "lamp",
  home: "lamp",
  diner: "cloth_red",
  market: "flag",
  clinic: "chalk",
  news_office: "chalk",
  party_office: "flag",
  union_hall: "cloth_red",
  school: "party_blue",
  university: "party_blue",
  park: "party_green",
  port: "party_blue",
};
const PARTIES = ["party_blue", "flag", "party_green"] as const;

const pickW = (seed: number, salt: string, xs: Weighted): string => rendezvous(seed, salt, xs.map(([id, weight]) => ({ id, weight }))).id;
const pick1 = <T extends string>(seed: number, salt: string, xs: readonly T[]): T => rendezvous(seed, salt, xs.map((id) => ({ id }))).id as T;
const chance = (seed: number, salt: string, p: number): boolean => hash32(seed, salt) % 1000 < p * 1000;

function stepOf(p: Palette, name: string, k: number): number {
  const r = ramp(p, name);
  return r.steps[Math.min(r.steps.length - 1, Math.max(0, r.anchorStep + k))] as number;
}

// ---------------------------------------------------------------------------
// Designs

type RoofKind = "gable" | "front" | "hip" | "mansard" | "flat" | "saw" | "lean" | "canopy";
type GroundKind = "house" | "classic" | "glazed" | "board" | "civic" | "industrial" | "market" | "diner" | "office";
type Rhythm = "bay" | "pair" | "wide" | "tall" | "strip";
type Fore = "none" | "steps" | "garden" | "railings" | "paved";
type UpperWindow = "sash" | "arched" | "tall" | "wide";

interface Mass {
  /** Px within the lot. */
  x0: number;
  x1: number;
  storeys: number;
  roof: RoofKind;
  /** Px from the lot's front edge to the wall (a forecourt). */
  inset: number;
  corner: "left" | "right" | null;
  turret: boolean;
  /** The secondary, lower part of an L. */
  wing: boolean;
}

export interface Design {
  family: Family;
  kind: string;
  wall: WallMat;
  roofMat: "tiles" | "slates";
  flatMat: "gravel" | "tar";
  masses: Mass[];
  ground: GroundKind;
  upper: UpperWindow;
  rhythm: Rhythm;
  attic: boolean;
  string: boolean;
  awning: number;
  stripes: boolean;
  door: string;
  fore: Fore;
  bayWindow: boolean;
  dormers: boolean;
  chimneys: boolean;
  boxes: boolean;
  drain: boolean;
  ivy: boolean;
  party: number;
}

const ROOFS: Record<string, Record<string, Weighted>> = {
  house: {
    inner: [["gable", 4], ["front", 2], ["mansard", 1], ["hip", 1]],
    suburb: [["hip", 3], ["front", 3], ["gable", 2]],
    waterfront: [["front", 3], ["gable", 3], ["mansard", 1]],
    core: [["mansard", 2], ["gable", 2], ["flat", 2]],
    industrial: [["gable", 2], ["flat", 1]],
    rural: [["gable", 3], ["hip", 2]],
  },
  shop: {
    core: [["flat", 4], ["mansard", 2], ["gable", 1]],
    inner: [["flat", 3], ["gable", 2], ["front", 1], ["mansard", 1]],
    suburb: [["gable", 2], ["flat", 2], ["front", 1]],
    waterfront: [["gable", 2], ["flat", 2], ["front", 2]],
    industrial: [["flat", 2], ["gable", 1]],
    rural: [["gable", 2], ["front", 1]],
  },
  institution: {
    core: [["flat", 3], ["mansard", 1], ["hip", 1]],
    inner: [["flat", 2], ["hip", 2], ["gable", 1]],
    suburb: [["hip", 2], ["flat", 1]],
    waterfront: [["flat", 2], ["hip", 1]],
    industrial: [["flat", 2], ["saw", 1]],
    rural: [["hip", 1], ["gable", 1]],
  },
};

const STOREYS: Record<string, Record<string, Weighted>> = {
  house: { inner: [["2", 3], ["3", 2]], core: [["3", 2], ["4", 1]], suburb: [["2", 3], ["1", 1]], waterfront: [["2", 2], ["3", 1]], industrial: [["2", 1]], rural: [["1", 1], ["2", 2]] },
  shop: { inner: [["2", 2], ["3", 2]], core: [["3", 2], ["4", 2]], suburb: [["2", 2], ["1", 1]], waterfront: [["2", 2], ["3", 1]], industrial: [["2", 1]], rural: [["1", 1], ["2", 1]] },
  institution: { inner: [["2", 1], ["3", 1]], core: [["3", 2], ["4", 1]], suburb: [["2", 1]], waterfront: [["2", 1], ["3", 1]], industrial: [["2", 1]], rural: [["2", 1]] },
};

/** One design for a spec; `salt` > 0 re-rolls it (to differ from a neighbour). */
function chooseDesign(s: FacadeSpec, salt: number): Design {
  const seed = salt === 0 ? s.visualSeed >>> 0 : hash32(s.visualSeed, "reroll", salt);
  const family = familyOf(s.kind, s.landmark);
  const hood = HOOD_WALLS[s.hood] ? s.hood : "inner";
  const W = s.w * T;
  const doorBay = Math.max(0, Math.min(s.w - 1, s.doorX));
  const headroom = s.headroom ?? 0;

  let wallId = pickW(seed, "wall", HOOD_WALLS[hood] as Weighted);
  if (family === "civic") wallId = pick1(seed, "wall", ["stone", "stone", "stoneGrey"]);
  else if (family === "industrial") wallId = s.kind === "barn" || s.kind === "shed" ? pick1(seed, "wall", ["timber", "cladding", "cladding"]) : pickW(seed, "wall", HOOD_WALLS.industrial as Weighted);
  const roofMat = pickW(seed, "roof", HOOD_ROOFS[hood] as Weighted) as Design["roofMat"];
  const flatMat: Design["flatMat"] = family === "industrial" || chance(seed, "tar", 0.35) ? "tar" : "gravel";

  const roofTable = family === "house" ? ROOFS.house : family === "shop" ? ROOFS.shop : ROOFS.institution;
  let roof = pickW(seed, "roofKind", (roofTable as Record<string, Weighted>)[hood] ?? [["gable", 1]]) as RoofKind;
  if (roof === "front" && W > 64) roof = "gable";
  const storeyTable = family === "house" ? STOREYS.house : family === "shop" ? STOREYS.shop : STOREYS.institution;
  let storeys = Number(pickW(seed, "storeys", (storeyTable as Record<string, Weighted>)[hood] ?? [["2", 1]]));
  if (s.kind === "home") storeys = 2;

  let ground: GroundKind = "house";
  if (family === "shop") ground = pickW(seed, "ground", hood === "core" ? [["glazed", 3], ["classic", 2], ["board", 1]] : [["classic", 3], ["board", 2], ["glazed", 1]]) as GroundKind;
  else if (family === "industrial") ground = "industrial";
  else if (family === "institution") ground = "office";
  else if (family === "civic") ground = "civic";

  const older = hood === "core" || hood === "inner";
  let upper = pickW(seed, "upper", older ? [["sash", 4], ["arched", 1], ["tall", 1]] : [["sash", 3], ["wide", 2]]) as UpperWindow;
  let rhythm = pickW(seed, "rhythm", family === "shop" ? [["bay", 3], ["pair", 2], ["tall", 2], ["wide", 1]] : hood === "suburb" ? [["bay", 2], ["wide", 2], ["pair", 1]] : [["bay", 4], ["pair", 1], ["wide", 1]]) as Rhythm;
  if (rhythm === "wide") upper = "wide";

  let fore: Fore = "none";
  let inset = 0;
  if (family === "house") {
    if (hood === "suburb" || hood === "rural" || s.kind === "home") {
      fore = "garden";
      inset = s.h >= 3 ? pick1(seed, "inset", ["12", "16"]) === "12" ? 12 : 16 : 8;
    } else if (chance(seed, "steps", 0.5)) {
      fore = pick1(seed, "fore", ["steps", "railings"]);
      inset = fore === "railings" ? 8 : 4;
    }
  }

  const door = pick1(seed, "door", DOOR_PAINTS);
  const d: Design = {
    family,
    kind: s.kind,
    wall: WALLS[wallId] as WallMat,
    roofMat,
    flatMat,
    masses: [],
    ground,
    upper,
    rhythm,
    attic: chance(seed, "attic", 0.4),
    string: chance(seed, "string", hood === "core" ? 0.7 : hood === "inner" ? 0.5 : 0.25),
    awning: ground === "classic" && chance(seed, "awn", 0.8) ? hash32(seed, "awning") % AWNINGS.length : ground === "glazed" && chance(seed, "awn", 0.25) ? hash32(seed, "awning") % AWNINGS.length : -1,
    stripes: chance(seed, "stripes", 0.6),
    door,
    fore,
    bayWindow: family === "house" && s.w >= 3 && chance(seed, "bay", hood === "suburb" ? 0.55 : 0.3),
    dormers: false,
    chimneys: family === "house" || (family === "shop" && chance(seed, "chim", 0.6)),
    boxes: family === "house" && chance(seed, "boxes", 0.35),
    drain: chance(seed, "drain", 0.5),
    ivy: (hood === "suburb" || hood === "rural") && chance(seed, "ivy", 0.2),
    party: hash32(seed, "party") % PARTIES.length,
  };

  // ---- masses: one, or an L with a lower, set-back wing
  const main: Mass = { x0: 0, x1: W, storeys, roof, inset, corner: null, turret: false, wing: false };
  d.masses = [main];
  if (family === "house" && s.w >= 4 && s.h >= 3 && chance(seed, "ell", hood === "suburb" || hood === "rural" ? 0.6 : 0.3)) {
    const mw = s.w >= 5 ? 3 : 2;
    const leftMain = doorBay < s.w - mw ? chance(seed, "ellSide", 0.5) : false;
    const split = (leftMain ? mw : s.w - mw) * T;
    const wingRoof: RoofKind = roof === "front" ? "gable" : older ? "flat" : roof === "mansard" ? "gable" : roof;
    const wing: Mass = { x0: 0, x1: 0, storeys: Math.max(1, storeys - 1), roof: wingRoof, inset: Math.min(s.h * T - 28, inset + (chance(seed, "wingIn", 0.5) ? 8 : 16)), corner: null, turret: false, wing: true };
    const front: Mass = { ...main, roof: main.roof === "gable" && chance(seed, "ellFront", 0.6) ? "front" : main.roof };
    if (leftMain) {
      front.x1 = split;
      wing.x0 = split;
      wing.x1 = W;
      d.masses = [front, wing];
    } else {
      wing.x0 = 0;
      wing.x1 = split;
      front.x0 = split;
      d.masses = [wing, front];
    }
  }
  // A narrow gap at the end of a joined row (round 2: "occasional gaps/alleys").
  if ((family === "house" || family === "shop") && d.masses.length === 1 && s.w >= 4 && s.ends?.right === "joined" && doorBay < s.w - 1 && chance(seed, "gap", 0.15)) {
    main.x1 = W - 5;
  }
  // Corners: a canted corner where the row meets a street, often with a turret.
  for (const side of ["left", "right"] as const) {
    if (s.ends?.[side] !== "street" || (family !== "shop" && family !== "house" && family !== "institution") || s.w < 3) continue;
    if (family === "house" && hood !== "inner" && hood !== "core" && hood !== "waterfront") continue;
    const m = side === "left" ? d.masses[0] : d.masses[d.masses.length - 1];
    if (!m || m.wing) continue;
    m.corner = side;
    m.turret = headroom >= 16 && m.storeys >= 2 && chance(seed, `turret${side}`, family === "shop" ? 0.6 : 0.4);
    if (m.roof === "front") m.roof = "gable";
  }

  // ---- landmarks and named places: their own designs
  applyLandmark(d, s, seed);
  if (d.masses.some((m) => m.roof === "mansard") || (d.masses[0]?.roof === "gable" && older && chance(seed, "dormers", 0.3))) d.dormers = true;
  return d;
}

function applyLandmark(d: Design, s: FacadeSpec, seed: number) {
  const W = s.w * T;
  const doorBay = Math.max(0, Math.min(s.w - 1, s.doorX));
  const m0 = d.masses[0] as Mass;
  const one = (patch: Partial<Mass>) => {
    d.masses = [{ x0: 0, x1: W, storeys: 2, roof: "flat", inset: 0, corner: null, turret: false, wing: false, ...patch }];
  };
  switch (s.kind) {
    case "town_hall":
    case "regional_assembly":
    case "parliament":
    case "executive_residence":
    case "council_chamber": {
      // A centre block over the door (three bays) and lower wings, all behind steps.
      const c0 = Math.max(0, doorBay - 1) * T;
      const c1 = Math.min(s.w, doorBay + 2) * T;
      const wingRoof: RoofKind = s.kind === "council_chamber" ? "flat" : "hip";
      d.masses = [];
      if (c0 > 0) d.masses.push({ x0: 0, x1: c0, storeys: 2, roof: wingRoof, inset: 6, corner: null, turret: false, wing: true });
      d.masses.push({ x0: c0, x1: c1, storeys: 3, roof: "flat", inset: 10, corner: null, turret: false, wing: false });
      if (c1 < W) d.masses.push({ x0: c1, x1: W, storeys: 2, roof: wingRoof, inset: 6, corner: null, turret: false, wing: true });
      d.wall = WALLS.stone as WallMat;
      d.fore = "steps";
      d.upper = "arched";
      d.rhythm = "bay";
      d.string = true;
      d.chimneys = false;
      d.ground = "civic";
      break;
    }
    case "news_office":
      d.wall = WALLS[pick1(seed, "nwall", ["brick", "brickDark", "stoneGrey"])] as WallMat;
      d.ground = "glazed";
      d.awning = -1;
      d.rhythm = "tall";
      d.upper = "arched";
      d.string = true;
      for (const m of d.masses) m.roof = "flat";
      m0.storeys = Math.max(m0.storeys, 3);
      break;
    case "diner":
      one({ storeys: 1, roof: "flat", corner: m0.corner });
      d.wall = WALLS[pick1(seed, "dwall", ["render", "mint", "butter"])] as WallMat;
      d.ground = "diner";
      d.chimneys = false;
      break;
    case "party_office":
      d.ground = "classic";
      d.awning = -2;
      break;
    case "union_hall":
      one({ storeys: 2, roof: W <= 80 ? "front" : "gable", inset: 4 });
      d.wall = WALLS[pick1(seed, "uwall", ["brick", "brickDark"])] as WallMat;
      d.ground = "office";
      d.fore = "steps";
      d.rhythm = "tall";
      d.upper = "arched";
      break;
    case "market":
      one({ storeys: 1, roof: "canopy" });
      d.ground = "market";
      d.chimneys = false;
      break;
    case "clinic":
      one({ storeys: Math.max(2, Math.min(3, m0.storeys)), roof: "flat", inset: 8 });
      d.wall = WALLS[pick1(seed, "cwall", ["render", "render", "stoneGrey"])] as WallMat;
      d.ground = "office";
      d.rhythm = "strip";
      d.fore = "paved";
      d.chimneys = false;
      break;
    case "school":
    case "university":
      one({ storeys: 2, roof: "hip", inset: 12 });
      d.wall = WALLS[pick1(seed, "swall", ["brick", "brickPale"])] as WallMat;
      d.ground = "office";
      d.rhythm = "wide";
      d.upper = "wide";
      d.fore = "railings";
      d.chimneys = true;
      break;
    case "warehouse":
    case "factory":
    case "mill":
      for (const m of d.masses) {
        m.roof = pickW(seed, "wroof", [["saw", 3], ["gable", 2], ["flat", 1]]) as RoofKind;
        m.storeys = pick1(seed, "wst", ["1", "2", "2"]) === "1" ? 1 : 2;
      }
      d.rhythm = "wide";
      d.upper = "wide";
      d.chimneys = s.kind !== "warehouse";
      break;
    case "shed":
    case "barn":
      one({ storeys: 1, roof: s.kind === "barn" ? "front" : "lean" });
      d.chimneys = false;
      break;
    case "boathouse":
      one({ storeys: 1, roof: W <= 64 ? "front" : "gable" });
      d.chimneys = false;
      break;
    case "hotel":
    case "business_association":
    case "community_centre":
      for (const m of d.masses) m.storeys = Math.max(m.storeys, s.kind === "hotel" ? 3 : 2);
      d.rhythm = "tall";
      d.string = true;
      break;
    default:
      break;
  }
}

/** The features two neighbours are compared on. */
export function signatureOf(d: Design): string[] {
  const main = d.masses.find((m) => !m.wing) ?? (d.masses[0] as Mass);
  return [
    d.wall.id,
    main.roof,
    main.roof === "flat" ? d.flatMat : d.roofMat,
    d.upper,
    d.rhythm,
    d.ground,
    String(d.awning),
    String(main.storeys),
    d.string ? "string" : "plain",
    d.fore,
    String(d.masses.length),
    d.door,
  ];
}

/** Share of equal features (0..1). */
export function similarity(a: readonly string[], b: readonly string[]): number {
  let n = 0;
  for (let i = 0; i < a.length; i++) if (a[i] === b[i]) n++;
  return n / Math.max(1, a.length);
}

/** Neighbours that agree on this share of features or more look like copies; the grammar re-rolls. */
export const COPY_SIMILARITY = 0.75;

const designCache = new Map<number, Design>();
function specKey(s: FacadeSpec): number {
  const e = s.ends ? `${s.ends.left}/${s.ends.right}` : "-";
  return hash32(s.kind, s.hood, s.w, s.h, s.doorX, s.storeys, s.visualSeed, s.named ? 1 : 0, s.home ? 1 : 0, s.landmark ? 1 : 0, s.headroom ?? 0, e, s.left ? specKey(s.left) : 0);
}

/** The design for a spec: picked from its seed, re-rolled (deterministically) while it would copy its left neighbour. */
export function designOf(s: FacadeSpec): Design {
  const key = specKey(s);
  const hit = designCache.get(key);
  if (hit) return hit;
  let best = chooseDesign(s, 0);
  if (s.left) {
    const other = signatureOf(designOf(s.left));
    let bestSim = similarity(signatureOf(best), other);
    for (let salt = 1; salt < 8 && bestSim >= COPY_SIMILARITY - 1e-9; salt++) {
      const d = chooseDesign(s, salt);
      const sim = similarity(signatureOf(d), other);
      if (sim < bestSim) {
        best = d;
        bestSim = sim;
      }
    }
  }
  if (designCache.size > 4000) designCache.clear();
  designCache.set(key, best);
  return best;
}

// ---------------------------------------------------------------------------
// The slot table

function materials(s: FacadeSpec, d: Design, p: Palette): Uint8Array {
  const seed = s.visualSeed;
  const wall = d.wall;
  const t = new Uint8Array(BSLOT_COUNT);
  const W = (k: number) => stepOf(p, wall.ramp, wall.off + k);
  t[BSLOT.wall] = W(0);
  t[BSLOT.wallS] = W(-1);
  t[BSLOT.wallD] = W(-2);
  t[BSLOT.wallL] = W(1);
  const trimRamp = wall.ramp === "stone" || wall.ramp === "paving" ? "chalk" : "stone";
  t[BSLOT.trim] = stepOf(p, trimRamp, trimRamp === "stone" ? 1 : 0);
  t[BSLOT.trimS] = stepOf(p, trimRamp, trimRamp === "stone" ? 0 : -1);
  t[BSLOT.trimL] = stepOf(p, trimRamp, trimRamp === "stone" ? 1 : 0);
  const roofRamp = d.roofMat === "tiles" ? "terracotta" : "slate";
  t[BSLOT.roof] = stepOf(p, roofRamp, 0);
  t[BSLOT.roofS] = stepOf(p, roofRamp, -1);
  t[BSLOT.roofD] = stepOf(p, roofRamp, -2);
  t[BSLOT.roofL] = stepOf(p, roofRamp, 1);
  // Flat roofs: gravel a step darker than the pavement, or tar.
  if (d.flatMat === "tar") {
    t[BSLOT.flat] = stepOf(p, "asphalt", 0);
    t[BSLOT.flatS] = stepOf(p, "asphalt", -1);
    t[BSLOT.flatD] = stepOf(p, "ink", 1);
    t[BSLOT.flatL] = stepOf(p, "asphalt", 1);
  } else {
    t[BSLOT.flat] = stepOf(p, "paving", -1);
    t[BSLOT.flatS] = stepOf(p, "paving", -2);
    t[BSLOT.flatD] = stepOf(p, "asphalt", -1);
    t[BSLOT.flatL] = stepOf(p, "paving", 0);
  }
  t[BSLOT.glass] = stepOf(p, "glass", 0);
  t[BSLOT.glassD] = stepOf(p, "glass", -1);
  t[BSLOT.glassY] = stepOf(p, "glass", 1);
  t[BSLOT.glassB] = stepOf(p, "glass", 2);
  if (wall.frames === "light") {
    t[BSLOT.frame] = stepOf(p, "chalk", -1);
    t[BSLOT.frameL] = stepOf(p, "chalk", 0);
  } else {
    t[BSLOT.frame] = stepOf(p, "iron", 0);
    t[BSLOT.frameL] = stepOf(p, "chalk", -1);
  }
  t[BSLOT.door] = stepOf(p, d.door, 0);
  t[BSLOT.doorS] = stepOf(p, d.door, -1);
  t[BSLOT.doorL] = stepOf(p, d.door, 1);
  // Awnings and fascias: muted (a darker step of a cloth or material ramp); the party office in its party's colour.
  const party = PARTIES[d.party] as string;
  const [awnRamp, awnStep] = d.awning === -2 ? [party, -1] : d.kind === "news_office" ? ["ink", 1] : d.kind === "diner" ? ["cloth_red", 0] : (AWNINGS[Math.max(0, d.awning) % AWNINGS.length] as readonly [string, number]);
  t[BSLOT.acc] = stepOf(p, awnRamp, awnStep);
  t[BSLOT.accS] = stepOf(p, awnRamp, awnStep - 1);
  t[BSLOT.stripe] = d.stripes || d.kind === "diner" ? stepOf(p, "chalk", d.kind === "diner" ? 0 : -1) : stepOf(p, awnRamp, awnStep);
  t[BSLOT.stripeS] = d.stripes || d.kind === "diner" ? stepOf(p, "paving", 0) : stepOf(p, awnRamp, awnStep - 1);
  t[BSLOT.iron] = stepOf(p, "iron", 0);
  t[BSLOT.ironL] = stepOf(p, "iron", 1);
  t[BSLOT.deep] = stepOf(p, "ink", -1);
  const signRamp = s.home ? "lamp" : d.kind === "party_office" ? party : (SIGN_RAMP[s.kind] ?? (d.family === "civic" ? "lamp" : "chalk"));
  t[BSLOT.sign] = stepOf(p, signRamp, 0);
  t[BSLOT.signS] = stepOf(p, signRamp, -1);
  t[BSLOT.picto] = stepOf(p, "ink", -2);
  for (const [slots, r] of [
    [[BSLOT.cap, BSLOT.capS, BSLOT.capD, BSLOT.capL], "slate"],
    [[BSLOT.dome, BSLOT.domeS, BSLOT.domeD, BSLOT.domeL], "cloth_teal"],
    [[BSLOT.stack, BSLOT.stackS, BSLOT.stackD, BSLOT.stackL], wall.ramp === "brick" ? "terracotta" : "brick"],
  ] as const) {
    t[slots[0]] = stepOf(p, r, 0);
    t[slots[1]] = stepOf(p, r, -1);
    t[slots[2]] = stepOf(p, r, -2);
    t[slots[3]] = stepOf(p, r, 1);
  }
  t[BSLOT.wood] = stepOf(p, "wood", 0);
  t[BSLOT.woodD] = stepOf(p, "wood", -1);
  t[BSLOT.woodL] = stepOf(p, "wood", 1);
  t[BSLOT.pave] = stepOf(p, "paving", 0);
  t[BSLOT.paveS] = stepOf(p, "paving", -1);
  t[BSLOT.paveL] = stepOf(p, "paving", 1);
  t[BSLOT.lawn] = stepOf(p, "grass", 0);
  t[BSLOT.lawnS] = stepOf(p, "grass", -1);
  t[BSLOT.hedge] = stepOf(p, "foliage", 0);
  t[BSLOT.hedgeS] = stepOf(p, "foliage", -1);
  t[BSLOT.bloom] = stepOf(p, pick1(seed, "bloom", ["cloth_red", "cloth_plum", "flag"]), 0);
  t[BSLOT.bloomY] = stepOf(p, "lamp", 1);
  t[BSLOT.poster] = stepOf(p, party, 0);
  t[BSLOT.posterS] = stepOf(p, "chalk", 0);
  t[BSLOT.earth] = stepOf(p, "earth", 0);
  t[BSLOT.earthL] = stepOf(p, "earth", 1);
  t[BSLOT.net] = stepOf(p, "cloth_teal", 0);
  t[BSLOT.netS] = stepOf(p, "cloth_teal", -1);
  t[BSLOT.hazard] = stepOf(p, "flag", 0);
  return t;
}

/** The lit colours: warm gold for homes, a cooler white for shops and offices (§6). */
function litTable(base: Uint8Array, use: LightUse, p: Palette): Uint8Array {
  const t = base.slice();
  const cool = use === "office" || use === "industrial";
  t[BSLOT.lit] = cool ? stepOf(p, "chalk", 0) : stepOf(p, "lamp", 1);
  t[BSLOT.litD] = cool ? stepOf(p, "chalk", -1) : stepOf(p, "lamp", 0);
  t[BSLOT.litL] = stepOf(p, "lamp", 2);
  t[BSLOT.litC] = stepOf(p, "lamp", 2);
  return t;
}

// ---------------------------------------------------------------------------
// Parsed parts

const B = (t: string) => grid(t, BLEGEND);
let cache: ReturnType<typeof parse> | null = null;
function parse() {
  return {
    tex: Object.fromEntries(Object.entries(TEX).map(([k, v]) => [k, B(v)])) as Record<keyof typeof TEX, Indexed>,
    windows: Object.fromEntries([...Object.entries(WINDOWS), ...Object.entries(WINDOWS_R2)].map(([k, v]) => [k, B(v)])) as Record<string, Indexed>,
    doors: Object.fromEntries(Object.entries(DOORS).map(([k, v]) => [k, B(v)])) as Record<DoorKind, Indexed>,
    plankDoor: B(PLANK_DOOR),
    cornerDoor: B(CORNER_DOOR),
    shopfront: B(SHOPFRONT),
    fascia: B(FASCIA),
    awning: B(AWNING),
    planks: B(PLANKS),
    strip: B(STRIP),
    bay: B(BAY),
    dormer: B(DORMER),
    column: { capital: B(COLUMN.capital), shaft: B(COLUMN.shaft), base: B(COLUMN.base) },
    pediment: B(PEDIMENT_HALF),
    turret: B(TURRET),
    dome: B(DOME),
    chimney: B(CHIMNEY),
    stack2: B(STACK2),
    vent: B(VENT),
    skylight: B(SKYLIGHT),
    tank: B(TANK),
    ac: B(AC_UNIT),
    hatch: B(HATCH),
    bin: B(BIN),
    shed: B(SHED),
    clock: B(CLOCK),
    cupola: B(CUPOLA),
    hanging: B(HANGING_SIGN),
    board: B(SIGN_BOARD),
    pictos: Object.fromEntries(Object.entries(PICTOGRAMS).map(([k, v]) => [k, B(v)])) as Record<string, Indexed>,
  };
}
export const kitParts = () => (cache ??= parse());

/** Every kit part (slot codes), for the parts sheet and lints. */
export function kitPartList(): { name: string; img: Indexed }[] {
  const k = kitParts();
  const out: { name: string; img: Indexed }[] = [];
  for (const [n, v] of Object.entries(k.tex)) out.push({ name: `tex ${n}`, img: v });
  for (const [n, v] of Object.entries(k.windows)) out.push({ name: `window ${n}`, img: v });
  for (const [n, v] of Object.entries(k.doors)) out.push({ name: `door ${n}`, img: v });
  out.push({ name: "door planks", img: k.plankDoor }, { name: "door corner", img: k.cornerDoor });
  out.push({ name: "shopfront", img: k.shopfront }, { name: "fascia", img: k.fascia }, { name: "awning", img: k.awning }, { name: "planks", img: k.planks }, { name: "strip", img: k.strip }, { name: "bay window", img: k.bay }, { name: "dormer", img: k.dormer });
  out.push({ name: "column capital", img: k.column.capital }, { name: "column base", img: k.column.base }, { name: "pediment", img: k.pediment });
  out.push({ name: "turret", img: k.turret }, { name: "dome", img: k.dome }, { name: "cupola", img: k.cupola }, { name: "clock", img: k.clock });
  out.push({ name: "chimney", img: k.chimney }, { name: "stack", img: k.stack2 }, { name: "vent", img: k.vent }, { name: "skylight", img: k.skylight }, { name: "tank", img: k.tank }, { name: "ac unit", img: k.ac }, { name: "hatch", img: k.hatch });
  out.push({ name: "bin", img: k.bin }, { name: "shed", img: k.shed }, { name: "hanging sign", img: k.hanging }, { name: "sign board", img: k.board });
  for (const [n, v] of Object.entries(k.pictos)) out.push({ name: `picto ${n}`, img: v });
  return out;
}

/** Glass slots to lit slots. */
const LIGHT_UP = (() => {
  const m = new Uint8Array(BSLOT_COUNT);
  m[BSLOT.glass] = BSLOT.lit;
  m[BSLOT.glassD] = BSLOT.litD;
  m[BSLOT.glassY] = BSLOT.litL;
  m[BSLOT.glassB] = BSLOT.litC;
  return m;
})();

/** The lit image of a part (only its glass, lit), with a variant: plain, curtains, or a half-drawn blind. */
function litPart(part: Indexed, kind: string, variant: number): Indexed {
  const out = remap(part, LIGHT_UP);
  if (kind === "shopfront" || kind === "strip" || part.w < 6) return out;
  if (variant === 1) {
    const cols = kind in LIT_OVERLAYS.curtains ? LIT_OVERLAYS.curtains[kind as keyof typeof LIT_OVERLAYS.curtains] : [1, part.w - 2];
    for (let y = 0; y < out.h; y++) for (const c of cols) if (out.px[y * out.w + c] !== 0) out.px[y * out.w + c] = BSLOT.litD;
  } else if (variant === 2) {
    let rows = 0;
    for (let y = 0; y < out.h && rows < 3; y++) {
      let any = false;
      for (let x = 0; x < out.w; x++) {
        if (out.px[y * out.w + x] !== 0) {
          out.px[y * out.w + x] = BSLOT.litD;
          any = true;
        }
      }
      if (any) rows++;
    }
  }
  return out;
}

// ---------------------------------------------------------------------------
// Drawing

interface Geo {
  m: Mass;
  storeys: number;
  groundY: number;
  groundTop: number;
  wallH: number;
  wallTop: number;
  roofTop: number;
  /** Floor tops of the upper storeys (first floor first). */
  floors: number[];
  /** The flat roof's parapet and cornice band (px). */
  parapet: number;
}

interface Placed {
  part: Indexed;
  kind: string;
  x: number;
  y: number;
  use: LightUse;
  ground: boolean;
}

const minRoof = (r: RoofKind) => (r === "flat" ? 9 : r === "canopy" || r === "lean" ? 8 : r === "mansard" ? 15 : 12);

function geometry(m: Mass, storeys0: number, IH: number, FD: number): Geo {
  const groundY = IH - m.inset;
  const parapet = m.roof === "flat" ? 6 : 0;
  const MD = FD - m.inset;
  // Roofs seen from above, foreshortened: pitched about half the mass's depth, flat and mansard tops less (big plain fields read as programmer art).
  const want = Math.max(minRoof(m.roof), Math.round(MD * (m.roof === "flat" ? 0.36 : m.roof === "mansard" ? 0.42 : 0.5)));
  let storeys = storeys0;
  let wallH = 0;
  let roofTop = 0;
  for (;;) {
    wallH = GROUND + UPPER * (storeys - 1) + parapet;
    let roofD = want;
    let top = groundY - wallH - roofD;
    if (top < 0) {
      roofD = Math.max(minRoof(m.roof), roofD + top);
      top = groundY - wallH - roofD;
    }
    roofTop = top;
    if (top >= 0 || storeys === 1) break;
    storeys--;
  }
  roofTop = Math.max(0, roofTop);
  const wallTop = groundY - wallH;
  const groundTop = groundY - GROUND;
  const floors: number[] = [];
  for (let f = 1; f < storeys; f++) floors.push(groundTop - UPPER * f);
  return { m, storeys, groundY, groundTop, wallH, wallTop, roofTop, floors, parapet };
}

/** Bays of a mass: about 16 px each. */
function baysOf(x0: number, x1: number): { x: number; w: number }[] {
  const n = Math.max(1, Math.round((x1 - x0) / T));
  const out: { x: number; w: number }[] = [];
  for (let b = 0; b < n; b++) {
    const a = x0 + Math.floor(((x1 - x0) * b) / n);
    const e = x0 + Math.floor(((x1 - x0) * (b + 1)) / n);
    out.push({ x: a, w: e - a });
  }
  return out;
}

class Canvas {
  readonly img: Indexed;
  /** Rectangles of ground and of wall faces (x, y, w, h, base y), for the receiver map. */
  readonly grounds: [number, number, number, number][] = [];
  readonly faces: [number, number, number, number, number][] = [];
  constructor(w: number, h: number) {
    this.img = blank(w, h);
  }
  rect(x: number, y: number, w: number, h: number, v: number) {
    fill(this.img, x, y, w, h, v);
  }
  px(x: number, y: number, v: number) {
    if (x >= 0 && y >= 0 && x < this.img.w && y < this.img.h) this.img.px[y * this.img.w + x] = v;
  }
  tex(t: Indexed, x: number, y: number, w: number, h: number, ox = 0, oy = 0) {
    tileFill(this.img, t, x, y, w, h, ox, oy);
  }
  part(p: Indexed, x: number, y: number, map?: ArrayLike<number>) {
    blit(this.img, p, x, y, map ? { map } : {});
  }
}

const identity = () => {
  const m = new Uint8Array(BSLOT_COUNT);
  for (let i = 0; i < BSLOT_COUNT; i++) m[i] = i;
  return m;
};
/** The right half of a pediment: the same drawing mirrored, its light trim in shade. */
const SHADE_TRIM = (() => {
  const m = identity();
  m[BSLOT.trimL] = BSLOT.trim;
  m[BSLOT.wall] = BSLOT.wallS;
  return m;
})();
const redirect = (to: readonly [number, number, number, number]) => {
  const m = identity();
  [m[BSLOT.roof], m[BSLOT.roofS], m[BSLOT.roofD], m[BSLOT.roofL]] = to;
  return m;
};
const DOME_MAP = redirect([BSLOT.dome, BSLOT.domeS, BSLOT.domeD, BSLOT.domeL]);
const COPPER_CAP = (() => {
  const m = identity();
  [m[BSLOT.cap], m[BSLOT.capS], m[BSLOT.capD], m[BSLOT.capL]] = [BSLOT.dome, BSLOT.domeS, BSLOT.domeD, BSLOT.domeL];
  return m;
})();
const STACK_MAP = (() => {
  const m = identity();
  [m[BSLOT.wall], m[BSLOT.wallS], m[BSLOT.wallD], m[BSLOT.wallL]] = [BSLOT.stack, BSLOT.stackS, BSLOT.stackD, BSLOT.stackL];
  return m;
})();

/** Build one building: masses, roofs, facades, openings, shadows and the receiver map. */
export function buildFacade(s: FacadeSpec, p: Palette): Facade {
  const d = designOf(s);
  const k = kitParts();
  const table = materials(s, d, p);
  const use = lightUseOf(s.kind, d.family);
  const seed = s.visualSeed >>> 0;
  const W = s.w * T;
  const FD = s.h * T;
  const HR = Math.max(0, Math.min(32, s.headroom ?? 0));
  const IH = FD + T + HR;
  const ay = T + HR;
  const c = new Canvas(W, IH);
  const doorBay = Math.max(0, Math.min(s.w - 1, s.doorX));
  const doorCx = doorBay * T + 8;
  // Plot states (layout v2, DEC-090 D2): an empty lot and a building site are drawn on their own; boarded and derelict buildings are the building, vacant.
  const state = plotStateOf(s.conditions);
  if (state === "reserved" || state === "construction") return buildLot(s, d, k, table, state);
  const derelict = state === "derelict";
  const vacant = derelict || state === "boarded";
  const boarded = vacant;
  const windows: Placed[] = [];
  const doors: { part: Indexed; kind: string; x: number; y: number }[] = [];
  const shadows: ShadowCaster[] = [];
  const geos = d.masses.map((m) => geometry(m, m.storeys, IH, FD));
  const upperUse: LightUse = d.family === "shop" ? (s.kind === "shop" || s.kind === "diner" ? "home" : "office") : use;
  let signDone = false;

  // ---- ground inside the lot: yards behind low masses, forecourts in front
  geos.forEach((g, gi) => {
    const { m } = g;
    const w = m.x1 - m.x0;
    if (g.roofTop > ay + 1) {
      const yh = g.roofTop - ay;
      c.tex(k.tex.paving, m.x0, ay, w, yh, m.x0, 0);
      // The yard's back wall, and its shadow.
      c.rect(m.x0, ay, w, 1, BSLOT.stackL);
      c.rect(m.x0, ay + 1, w, 2, BSLOT.stack);
      c.rect(m.x0, ay + 3, w, 1, BSLOT.paveS);
      if (yh >= 12 && w >= 16) {
        const v = hash32(seed, "yard", gi);
        if (v % 3 === 0 && yh >= 13) c.part(k.shed, m.x0 + 2 + (v % Math.max(1, w - 15)), ay + 2);
        else c.part(k.bin, m.x0 + 3 + ((v >>> 4) % Math.max(1, w - 8)), ay + Math.max(4, yh - 7));
      }
      c.grounds.push([m.x0, ay, w, yh]);
    }
    if (m.inset > 0) drawForecourt(c, d, g, k, doorCx, seed);
  });

  // ---- masses: walls, then roofs
  geos.forEach((g, gi) => {
    const prev = geos[gi - 1];
    const next = geos[gi + 1];
    const exposedL = gi === 0 ? s.ends?.left !== "joined" : (prev as Geo).roofTop > g.roofTop || (prev as Geo).groundY < g.groundY;
    const exposedR = gi === geos.length - 1 ? s.ends?.right !== "joined" || g.m.x1 < W : (next as Geo).roofTop > g.roofTop || (next as Geo).groundY < g.groundY;
    drawWall(c, d, g, k, seed, exposedL, exposedR);
    drawRoof(c, d, g, k, seed + gi, exposedL, exposedR);
    // Shadow: the mass's footprint, px from the anchor (the footprint's top-left).
    const back = 0;
    const height = Math.round((g.wallH + (g.groundY - g.wallH - g.roofTop) * 0.4) * 0.85);
    shadows.push({
      footprint: [
        [g.m.x0, back],
        [g.m.x1, back],
        [g.m.x1, FD - g.m.inset],
        [g.m.x0, FD - g.m.inset],
      ],
      height,
    });
  });

  // ---- facades
  geos.forEach((g) => {
    const { m } = g;
    const hasDoor = doorCx >= m.x0 && doorCx < m.x1;
    let fx0 = m.x0;
    let fx1 = m.x1;
    if (m.corner === "left") fx0 += 6;
    if (m.corner === "right") fx1 -= 6;
    const bays = baysOf(fx0, fx1);
    const doorB = hasDoor ? bays.findIndex((b) => doorCx >= b.x && doorCx < b.x + b.w) : -1;
    // Upper floors.
    g.floors.forEach((ft, fi) => {
      if (ft < g.wallTop + g.parapet + 2) return;
      const top = fi === g.floors.length - 1;
      const first = fi === 0;
      bays.forEach((b, bi) => {
        const signHere = !signDone && (s.named || s.home) && first && bi === doorB && d.kind !== "diner" && d.kind !== "news_office";
        if (signHere) return;
        const cx = b.x + Math.floor(b.w / 2);
        let kind: string = d.upper;
        if (d.rhythm === "tall") kind = first ? "tall" : d.upper === "wide" ? "sash" : d.upper;
        if (d.attic && top && g.storeys >= 3 && d.rhythm !== "strip") kind = "small";
        if (d.rhythm === "pair" && kind !== "small") {
          for (const dx of [-6, 2]) windows.push({ part: k.windows.narrow as Indexed, kind: "narrow", x: cx + dx, y: ft + 3, use: upperUse, ground: false });
          return;
        }
        if (d.rhythm === "strip") return;
        const part = k.windows[kind] as Indexed;
        if (part.w > b.w - 2) kind = "sash";
        const pp = k.windows[kind] as Indexed;
        const y = ft + (kind === "arched" ? 2 : kind === "small" ? 5 : kind === "wide" ? 4 : 3);
        windows.push({ part: pp, kind, x: cx - Math.floor(pp.w / 2), y, use: upperUse, ground: false });
      });
      if (d.rhythm === "strip") windows.push(...stripWindows(k, fx0, fx1, ft + 4, upperUse));
    });
    // Ground floor.
    drawGround(c, d, s, g, k, bays, doorB, hasDoor, windows, doors, use, seed);
    // A canted corner: a narrow face turned to the street, its own windows and (shops) a corner door.
    if (m.corner) drawCorner(c, d, g, k, m.corner, windows, doors, use, upperUse);
  });

  // ---- roof pieces: turrets, landmark towers, clutter, chimneys, dormers
  geos.forEach((g, gi) => drawRoofPieces(c, d, s, g, k, seed + gi * 31, windows, shadows));
  if (d.family === "civic") drawCivic(c, d, s, geos, k, shadows);

  // ---- signs
  if (s.named || s.home) signDone = drawSign(c, d, s, geos, k, doorCx);

  // ---- windows and doors (with boarding), then wear and ivy over the wall
  // Boarded: the ground floor and about half the windows above it; derelict: the ground floor and a third above (the rest are broken).
  const boardedPlanks = (w: Placed) => boarded && (w.ground || (vacant && hash32(seed, "board", w.x, w.y) % (derelict ? 3 : 2) === 0));
  for (const w of windows) {
    c.part(w.part, w.x, w.y);
    if (boardedPlanks(w)) c.part(crop(k.planks, 0, 0, Math.min(k.planks.w, w.part.w - 2), Math.min(k.planks.h, w.part.h - 4)), w.x + 1, w.y + 2);
  }
  for (const dd of doors) {
    c.part(dd.part, dd.x, dd.y);
    // A vacant building's doors are boarded too.
    if (vacant && dd.part.h >= 8) for (const oy of [3, 12]) if (oy + 6 <= dd.part.h) c.part(crop(k.planks, 0, 0, Math.min(k.planks.w, dd.part.w - 2), 6), dd.x + 1, dd.y + oy);
  }
  geos.forEach((g, gi) => drawWear(c, d, g, seed + gi * 7, windows));
  if (derelict) decay(c, geos, windows.filter((w) => !boardedPlanks(w)), seed);
  // Party-office posters in the shop windows.
  if (d.kind === "party_office") {
    for (const w of windows) {
      if (!w.ground) continue;
      c.rect(w.x + 4, w.y + 4, 3, 5, BSLOT.poster);
      c.rect(w.x + 4, w.y + 6, 3, 1, BSLOT.posterS);
      c.rect(w.x + 9, w.y + 5, 3, 4, BSLOT.posterS);
      c.rect(w.x + 10, w.y + 6, 1, 2, BSLOT.poster);
    }
  }

  // ---- recolour; openings with their lit images
  const albedo = remap(c.img, table);
  const outW: Opening[] = windows
    .filter((w) => !boardedPlanks(w))
    .map((w, i) => {
      const h = hash32(seed, "lit", i);
      // Not every room is in use: about a quarter of the windows above the street stay dark.
      const dark = vacant || (!w.ground && d.family !== "civic" && h % 100 < 25);
      return { x: w.x, y: w.y, w: w.part.w, h: w.part.h, emissive: dark ? null : remap(litPart(w.part, w.kind, (h >>> 8) % 3), litTable(table, w.use, p)), use: w.use };
    })
    .filter((o) => o.x >= 0 && o.y >= 0 && o.x + o.w <= W && o.y + o.h <= IH);
  const outD: Opening[] = doors
    .map((dd) => ({
      x: dd.x,
      y: dd.y,
      w: dd.part.w,
      h: dd.part.h,
      emissive: vacant || dd.kind === "shutter" || dd.kind === "planks" ? null : remap(litPart(dd.part, dd.kind, 0), litTable(table, use, p)),
      use,
    }))
    .filter((o) => o.x >= 0 && o.y >= 0 && o.x + o.w <= W && o.y + o.h <= IH);

  // ---- the receiver map
  const receivers = new Uint8Array(W * IH);
  for (let i = 0; i < receivers.length; i++) receivers[i] = c.img.px[i] ? RECEIVER.roof : RECEIVER.none;
  for (const [x, y, w, h] of c.grounds) for (let yy = Math.max(0, y); yy < Math.min(IH, y + h); yy++) for (let xx = Math.max(0, x); xx < Math.min(W, x + w); xx++) if (c.img.px[yy * W + xx]) receivers[yy * W + xx] = RECEIVER.ground;
  for (const [x, y, w, h, base] of c.faces) {
    for (let yy = Math.max(0, y); yy < Math.min(IH, y + h); yy++) {
      for (let xx = Math.max(0, x); xx < Math.min(W, x + w); xx++) if (c.img.px[yy * W + xx]) receivers[yy * W + xx] = Math.min(255, RECEIVER.face + Math.max(0, base - 1 - yy));
    }
  }
  const main = shadows[d.masses.findIndex((m) => !m.wing)] ?? (shadows[0] as ShadowCaster);
  return { albedo, ay, windows: outW, doors: outD, receivers, shadow: main, shadows, signature: signatureOf(d) };
}

// ---------------------------------------------------------------------------
// Plot states (round 4; DEC-090 D2: the monthly dress pass shows them)

export const PLOT_STATES = ["built", "reserved", "construction", "boarded", "derelict"] as const;
export type PlotState = (typeof PLOT_STATES)[number];

/** The plot state among a spec's conditions (the renderer passes `TownBuilding.state` there); built when none. */
export function plotStateOf(conditions: readonly string[]): PlotState {
  for (const st of ["reserved", "construction", "derelict", "boarded"] as const) if (conditions.includes(st)) return st;
  return "built";
}

const GLASS_SLOTS = new Set<number>([BSLOT.glass, BSLOT.glassD, BSLOT.glassY, BSLOT.glassB]);
const ROOF_SLOTS = new Set<number>([BSLOT.roof, BSLOT.roofS, BSLOT.roofD, BSLOT.roofL, BSLOT.flat, BSLOT.flatS, BSLOT.flatD, BSLOT.flatL]);

/**
 * Decay for a derelict building: holes in the roof with the rafters showing,
 * broken panes, a crack down the wall, ivy, and weeds along the foot of the
 * walls and through the paving of forecourts and yards.
 */
function decay(c: Canvas, geos: readonly Geo[], broken: readonly Placed[], seed: number) {
  const img = c.img;
  const at = (x: number, y: number) => (x < 0 || y < 0 || x >= img.w || y >= img.h ? 0 : (img.px[y * img.w + x] as number));
  // Broken windows: the panes gone dark, a shard or two left in a corner.
  for (const w of broken) {
    for (let y = w.y; y < w.y + w.part.h; y++) {
      for (let x = w.x; x < w.x + w.part.w; x++) {
        if (!GLASS_SLOTS.has(at(x, y))) continue;
        const shard = x - w.x + (y - w.y) < 4 && hash32(seed, "shard", x, y) % 2 === 0;
        c.px(x, y, shard ? BSLOT.glassY : BSLOT.deep);
      }
    }
  }
  geos.forEach((g, gi) => {
    const { m } = g;
    const w = m.x1 - m.x0;
    // Roof holes: one or two ragged gaps with rafters across them.
    const roofH = g.wallTop - g.roofTop;
    if (roofH >= 6 && m.roof !== "canopy") {
      const holes = w >= 40 ? 2 : 1;
      for (let hI = 0; hI < holes; hI++) {
        const hw = Math.min(w - 8, 9 + (hash32(seed, "hw", gi, hI) % 7));
        const hh = Math.max(3, Math.min(roofH - 3, 4 + (hash32(seed, "hh", gi, hI) % 3)));
        const hx = m.x0 + 3 + (hash32(seed, "hx", gi, hI) % Math.max(1, w - hw - 6));
        const hy = g.roofTop + 2 + (hash32(seed, "hy", gi, hI) % Math.max(1, roofH - hh - 3));
        for (let y = hy; y < hy + hh; y++) {
          // Ragged ends: each row a little shorter or longer.
          const j = hash32(seed, "rag", y, hI) % 3;
          for (let x = hx + j; x < hx + hw - ((j + y) % 2); x++) {
            if (!ROOF_SLOTS.has(at(x, y))) continue;
            // Rafters across the gap every 4 px (lit on top), the dark loft between them.
            const rafter = (x - hx) % 4 === 1;
            c.px(x, y, rafter ? (y === hy ? BSLOT.woodL : BSLOT.woodD) : y === hy ? BSLOT.roofD : BSLOT.deep);
          }
        }
      }
    }
    if (m.roof === "canopy") return;
    // A crack down the wall from the eaves.
    let cx = m.x0 + 4 + (hash32(seed, "crack", gi) % Math.max(1, w - 8));
    for (let y = g.wallTop + 3; y < Math.min(g.wallTop + 16, g.groundY - 4); y++) {
      const v = at(cx, y);
      if (v && !GLASS_SLOTS.has(v) && v !== BSLOT.deep) c.px(cx, y, BSLOT.wallD);
      if (hash32(seed, "cz", y) % 3 === 0) cx += hash32(seed, "cd", y) % 2 ? 1 : -1;
    }
    // Ivy up one end of the wall, weeds along its foot.
    const ivyX = hash32(seed, "ivy", gi) % 2 ? m.x0 + 1 : m.x1 - 8;
    for (let y = g.groundY - 1; y > g.groundY - 22 && y > g.wallTop; y--) {
      const reach = 7 - Math.floor((g.groundY - y) / 4);
      for (let x = ivyX; x < ivyX + 7; x++) {
        if (x - ivyX > reach || hash32(seed, "leaf", x, y) % 3 === 0) continue;
        c.px(x, y, (x + y) % 3 === 0 ? BSLOT.hedgeS : BSLOT.hedge);
      }
    }
    for (let x = m.x0; x < m.x1; x++) {
      const h = hash32(seed, "weed", x) % 5;
      if (h > 2) continue;
      for (let k = 0; k <= h; k++) c.px(x, g.groundY - 1 - k, k === h ? BSLOT.lawn : BSLOT.hedgeS);
    }
  });
  // Weeds through the paving of forecourts and yards.
  for (const [x0, y0, w, h] of c.grounds) {
    for (let y = y0; y < y0 + h; y++) for (let x = x0; x < x0 + w; x++) if (hash32(seed, "grow", x >> 1, y >> 1) % 5 === 0 && at(x, y)) c.px(x, y, (x + y) % 2 ? BSLOT.lawn : BSLOT.lawnS);
  }
}

/**
 * A plot with no building on it yet: a reserved lot (rough grass and bare
 * earth inside a low timber fence, a gate, a board on posts, a heap of spoil)
 * or a building site (a slab and walls rising about a storey in the design's
 * material, openings without glass, scaffolding with boards, a brace, a
 * ladder and netting, a skip). Neither has lit openings. Same frame and
 * anchor as the building that will stand there.
 */
function buildLot(s: FacadeSpec, d: Design, k: ReturnType<typeof kitParts>, table: Uint8Array, state: "reserved" | "construction"): Facade {
  const seed = s.visualSeed >>> 0;
  const W = s.w * T;
  const FD = s.h * T;
  const HR = Math.max(0, Math.min(32, s.headroom ?? 0));
  const IH = FD + T + HR;
  const ay = T + HR;
  const c = new Canvas(W, IH);
  const doorCx = Math.max(0, Math.min(s.w - 1, s.doorX)) * T + 8;
  const shadows: ShadowCaster[] = [];
  const doors: Opening[] = [];
  // The lot's ground: rough grass, bare earth (most of a building site).
  c.tex(k.tex.lawn, 0, ay, W, FD, seed & 15, 0);
  c.grounds.push([0, ay, W, FD]);
  for (let y = ay; y < IH; y += 2) {
    for (let x = 0; x < W; x += 2) {
      const h = hash32(seed, "lot", x >> 2, y >> 2);
      if (state === "construction" ? h % 3 !== 0 : h % 5 === 0) c.rect(x, y, 2, 2, (x + y) % 6 === 0 ? BSLOT.earthL : BSLOT.earth);
    }
  }
  if (state === "reserved") {
    // A low timber fence round the lot (rails seen from above along the sides and back, its face along the front), a gate at the door.
    const fy = IH - 6;
    for (let x = 0; x < W; x++) {
      if (Math.abs(x - doorCx) <= 4) continue;
      c.px(x, fy + 1, BSLOT.woodL);
      c.px(x, fy + 3, BSLOT.wood);
      if (x % 6 === 0) {
        c.rect(x, fy, 2, 6, BSLOT.woodD);
        c.px(x, fy, BSLOT.woodL);
      }
    }
    for (let y = ay + 1; y < fy; y++) {
      c.px(0, y, y % 6 === 0 ? BSLOT.woodD : BSLOT.wood);
      c.px(W - 1, y, y % 6 === 0 ? BSLOT.woodD : BSLOT.wood);
    }
    for (let x = 0; x < W; x++) c.px(x, ay + 1, x % 6 === 0 ? BSLOT.woodD : BSLOT.wood);
    c.faces.push([0, fy, W, 6, IH]);
    // A board on two posts with a house pictogram: the plot is for building on.
    const bx = doorCx + 7 <= W - 13 ? doorCx + 7 : Math.max(1, doorCx - 19);
    c.rect(bx + 2, fy - 8, 1, 8, BSLOT.woodD);
    c.rect(bx + 9, fy - 8, 1, 8, BSLOT.woodD);
    c.rect(bx, fy - 15, 12, 8, BSLOT.sign);
    c.rect(bx, fy - 8, 12, 1, BSLOT.signS);
    // A gabled roof (rows of 2, 4, 6) over a house with a door in it.
    c.rect(bx + 5, fy - 14, 2, 1, BSLOT.picto);
    c.rect(bx + 4, fy - 13, 4, 1, BSLOT.picto);
    c.rect(bx + 3, fy - 12, 6, 1, BSLOT.picto);
    c.rect(bx + 4, fy - 11, 4, 3, BSLOT.picto);
    c.rect(bx + 5, fy - 10, 2, 2, BSLOT.sign);
    c.faces.push([bx, fy - 15, 12, 15, fy]);
    // A heap of spoil at the back.
    const hx = (hash32(seed, "heap") % Math.max(1, W - 16)) + 2;
    for (let r = 0; r < 5; r++) c.rect(hx + r, ay + 4 + (4 - r), 12 - 2 * r, 1, r === 4 ? BSLOT.earthL : BSLOT.earth);
    shadows.push({
      footprint: [
        [0, FD - 5],
        [W, FD - 5],
        [W, FD],
        [0, FD],
      ],
      height: 6,
    });
    doors.push({ x: doorCx - 4, y: IH - 6, w: 8, h: 6, emissive: null, use: "home" });
  } else {
    // The slab, and walls rising about a storey with ragged tops (courses still being laid).
    const builtH = 18 + (hash32(seed, "built") % 10);
    const wallTop = IH - builtH;
    c.rect(1, ay + 2, W - 2, FD - 2, BSLOT.pave);
    c.rect(1, ay + 2, W - 2, 1, BSLOT.paveL);
    c.tex(k.tex[d.wall.tex], 2, wallTop, W - 4, builtH, (seed >>> 7) & 15, 0);
    for (let x = 2; x < W - 2; x += 4) {
      const drop = hash32(seed, "course", x) % 5;
      c.rect(x, wallTop, Math.min(4, W - 2 - x), drop, BSLOT.pave);
      c.rect(x, wallTop + drop, Math.min(4, W - 2 - x), 1, BSLOT.wallL);
    }
    c.rect(2, IH - 2, W - 4, 1, BSLOT.wallS);
    c.rect(2, IH - 1, W - 4, 1, BSLOT.wallD);
    c.faces.push([2, wallTop, W - 4, builtH, IH]);
    // Openings without glass: the door, and a window in the other bays once the walls are high enough.
    for (let b = 0; b < s.w; b++) {
      const bx = b * T;
      if (bx + 8 === doorCx) c.rect(bx + 3, IH - 21, 10, 20, BSLOT.deep);
      else if (builtH > 20) c.rect(bx + 4, IH - 18, 8, 9, BSLOT.deep);
    }
    // Materials on the slab: a stack of blocks and a pallet.
    c.rect(4, ay + 4, 8, 4, BSLOT.stack);
    c.rect(4, ay + 4, 8, 1, BSLOT.stackL);
    c.rect(W - 14, ay + 5, 10, 3, BSLOT.wood);
    c.rect(W - 14, ay + 7, 10, 1, BSLOT.woodD);
    // Scaffolding over the front: standards every 12 px, ledgers and boards every lift, a brace, a ladder, netting on the top lift.
    const top = Math.max(ay - 6, wallTop - 12);
    for (let x = 1; x < W - 1; x += 12) c.rect(x, top, 1, IH - top, BSLOT.iron);
    c.rect(W - 2, top, 1, IH - top, BSLOT.iron);
    for (let y = IH - 3; y >= top + 2; y -= 9) {
      c.rect(0, y, W, 1, BSLOT.iron);
      c.rect(0, y - 2, W, 2, BSLOT.wood);
      c.rect(0, y - 2, W, 1, BSLOT.woodL);
    }
    for (let y = top; y < IH - 3; y++) c.px(13 + Math.floor(((y - top) * 10) / Math.max(1, IH - 3 - top)), y, BSLOT.ironL);
    const lx = Math.min(W - 8, 26);
    for (let y = top + 2; y < IH - 1; y++) {
      c.px(lx, y, BSLOT.woodD);
      c.px(lx + 3, y, BSLOT.woodD);
      if (y % 3 === 0) c.rect(lx, y, 4, 1, BSLOT.woodL);
    }
    for (let y = top; y < top + 7; y++) for (let x = 0; x < W; x++) if (x % 12 !== 1 && (hash32(seed, "net", x >> 2) + y) % 9 !== 0) c.px(x, y, (x + (y >> 1)) % 5 === 0 ? BSLOT.netS : BSLOT.net);
    // A skip by the front.
    const sx = doorCx + 10 <= W - 12 ? doorCx + 9 : 2;
    c.rect(sx, IH - 6, 11, 5, BSLOT.hazard);
    c.rect(sx, IH - 6, 11, 1, BSLOT.stackL);
    c.rect(sx + 1, IH - 7, 9, 1, BSLOT.stack);
    shadows.push({
      footprint: [
        [0, 2],
        [W, 2],
        [W, FD],
        [0, FD],
      ],
      height: builtH + 6,
    });
    doors.push({ x: doorCx - 5, y: IH - 21, w: 10, h: 20, emissive: null, use: "home" });
  }
  const albedo = remap(c.img, table);
  const receivers = new Uint8Array(W * IH);
  for (let i = 0; i < receivers.length; i++) receivers[i] = c.img.px[i] ? RECEIVER.ground : RECEIVER.none;
  for (const [x, y, w, h, base] of c.faces) {
    for (let yy = Math.max(0, y); yy < Math.min(IH, y + h); yy++) {
      for (let xx = Math.max(0, x); xx < Math.min(W, x + w); xx++) if (c.img.px[yy * W + xx]) receivers[yy * W + xx] = Math.min(255, RECEIVER.face + Math.max(0, base - 1 - yy));
    }
  }
  return { albedo, ay, windows: [], doors, receivers, shadow: shadows[0] as ShadowCaster, shadows, signature: [...signatureOf(d), state] };
}

function stripWindows(k: ReturnType<typeof kitParts>, x0: number, x1: number, y: number, use: LightUse): Placed[] {
  const out: Placed[] = [];
  for (let x = x0 + 2; x + 16 <= x1 - 1; x += 16) out.push({ part: k.strip, kind: "strip", x, y, use, ground: false });
  return out;
}

function drawForecourt(c: Canvas, d: Design, g: Geo, k: ReturnType<typeof kitParts>, doorCx: number, seed: number) {
  const { m } = g;
  const w = m.x1 - m.x0;
  const y = g.groundY;
  const h = m.inset;
  c.grounds.push([m.x0, y, w, h]);
  if (d.fore === "garden" && !m.wing) {
    c.tex(k.tex.lawn, m.x0, y, w, h, seed & 15, 0);
    // A hedge along the front, a path to the door.
    c.rect(m.x0, y + h - 3, w, 2, BSLOT.hedge);
    c.rect(m.x0, y + h - 1, w, 1, BSLOT.hedgeS);
    for (let x = m.x0 + 1; x < m.x1; x += 5) c.px(x, y + h - 3, BSLOT.hedgeS);
    if (doorCx >= m.x0 && doorCx < m.x1) c.rect(doorCx - 3, y, 6, h, BSLOT.pave);
    if (doorCx >= m.x0 && doorCx < m.x1) c.rect(doorCx + 2, y, 1, h, BSLOT.paveS);
    // Flowers by the path.
    if (hash32(seed, "flowers") % 2 === 0) for (let x = m.x0 + 2; x < m.x1 - 2; x += 3) if (Math.abs(x - doorCx) > 5) c.px(x, y + 2 + (x % 2), x % 6 < 3 ? BSLOT.bloom : BSLOT.bloomY);
    return;
  }
  c.tex(k.tex.paving, m.x0, y, w, h, m.x0, 0);
  if (d.fore === "steps" || (d.family === "civic" && !m.wing)) {
    // Steps up to the door (or across a civic front): light treads, dark risers.
    const sx = d.family === "civic" ? m.x0 + 2 : Math.max(m.x0, doorCx - 7);
    const sw = d.family === "civic" ? w - 4 : Math.min(14, m.x1 - sx);
    for (let r = 0; r < Math.min(h, 10); r += 2) {
      c.rect(sx - r / 2, y + r, sw + r, 1, BSLOT.trimL);
      c.rect(sx - r / 2, y + r + 1, sw + r, 1, BSLOT.trimS);
    }
  }
  if (d.fore === "railings" && !m.wing) {
    // Railings along the front, a gate at the door.
    c.rect(m.x0, y + h - 5, w, 1, BSLOT.iron);
    for (let x = m.x0; x < m.x1; x += 2) if (Math.abs(x - doorCx) > 4) c.rect(x, y + h - 5, 1, 4, BSLOT.iron);
    c.rect(m.x0, y + h - 1, w, 1, BSLOT.paveS);
  }
  if (d.fore === "paved") {
    // A canopy's posts and two planters.
    c.rect(m.x0 + 2, y + 1, 4, 3, BSLOT.wood);
    c.rect(m.x0 + 2, y, 4, 1, BSLOT.hedge);
    c.rect(m.x1 - 6, y + 1, 4, 3, BSLOT.wood);
    c.rect(m.x1 - 6, y, 4, 1, BSLOT.hedge);
  }
}

function drawWall(c: Canvas, d: Design, g: Geo, k: ReturnType<typeof kitParts>, seed: number, exposedL: boolean, exposedR: boolean) {
  const { m } = g;
  const w = m.x1 - m.x0;
  if (m.roof === "canopy") return;
  c.tex(k.tex[d.wall.tex], m.x0, g.wallTop, w, g.wallH, (seed >>> 7) & 15, 0);
  c.faces.push([m.x0, g.wallTop, w, g.wallH, g.groundY]);
  if (g.parapet) {
    // Coping, parapet, a cornice with dentils, and its shadow.
    c.rect(m.x0, g.wallTop, w, 1, BSLOT.trimL);
    c.rect(m.x0, g.wallTop + 1, w, 1, BSLOT.trim);
    c.rect(m.x0, g.wallTop + 4, w, 1, BSLOT.trim);
    for (let x = m.x0; x < m.x1; x += 2) c.px(x, g.wallTop + 4, BSLOT.trimS);
    c.rect(m.x0, g.wallTop + 5, w, 1, BSLOT.trimS);
  } else {
    // The eaves' shadow on the top of the wall.
    c.rect(m.x0, g.wallTop, w, 2, BSLOT.wallS);
  }
  c.rect(m.x0, g.groundY - 2, w, 1, BSLOT.wallS);
  c.rect(m.x0, g.groundY - 1, w, 1, BSLOT.wallD);
  // Ends: a light edge where the light (upper left) catches an exposed end, a dark seam at the other.
  if (exposedL) c.rect(m.x0, g.wallTop + 2, 1, g.wallH - 4, BSLOT.wallL);
  c.rect(m.x1 - 1, g.wallTop, 1, g.wallH, BSLOT.wallD);
  if (exposedR) c.rect(m.x1 - 2, g.wallTop + 2, 1, g.wallH - 4, BSLOT.wallS);
  // Storey bands: string courses (trim) or a plain shadow line.
  const bands = [g.groundTop, ...g.floors.slice(0, -1).map((ft) => ft)].filter((y) => y > g.wallTop + g.parapet + 2);
  for (const y of g.storeys > 1 ? bands : []) {
    if (d.string) {
      c.rect(m.x0, y - 1, w, 1, BSLOT.trim);
      c.rect(m.x0, y, w, 1, BSLOT.trimS);
    } else c.rect(m.x0, y, w, 1, BSLOT.wallS);
  }
}

/** Texture of a pitched roof (tiles or slates). */
const roofTex = (k: ReturnType<typeof kitParts>, d: Design) => (d.roofMat === "tiles" ? k.tex.tiles : k.tex.slates);

function drawRoof(c: Canvas, d: Design, g: Geo, k: ReturnType<typeof kitParts>, seed: number, exposedL: boolean, exposedR: boolean) {
  const { m } = g;
  const x0 = m.x0;
  const x1 = m.x1;
  const w = x1 - x0;
  const top = g.roofTop;
  const wt = g.wallTop;
  const H = wt - top;
  if (H <= 0) return;
  const tex = roofTex(k, d);
  const ox = (seed >>> 5) & 15;
  switch (m.roof) {
    case "flat": {
      c.tex(k.tex.flat, x0, top, w, H, (seed >>> 3) & 15, 0);
      c.rect(x0, top, w, 2, BSLOT.trimL);
      c.rect(x0, top, 2, H, BSLOT.trimL);
      c.rect(x1 - 2, top, 2, H, BSLOT.trimS);
      c.rect(x0 + 2, top + 2, w - 4, 1, BSLOT.flatD);
      c.rect(x0 + 2, top + 2, 1, H - 2, BSLOT.flatD);
      return;
    }
    case "gable": {
      const ridge = top + Math.max(2, Math.round(H * 0.32));
      c.rect(x0, top, w, ridge - top, BSLOT.roofS);
      c.rect(x0, top, w, 1, BSLOT.roofD);
      c.tex(tex, x0, ridge + 1, w, wt - ridge - 1, ox, 0);
      c.rect(x0, ridge, w, 1, BSLOT.roofL);
      c.rect(x0, wt - 1, w, 1, BSLOT.roofD);
      // Verges (bargeboards) at exposed gable ends.
      if (exposedL) c.rect(x0, top, 1, H, BSLOT.trimL);
      if (exposedR) c.rect(x1 - 1, top, 1, H, BSLOT.trimS);
      return;
    }
    case "lean": {
      c.tex(k.tex.corrugated, x0, top, w, H, ox, 0);
      c.rect(x0, top, w, 1, BSLOT.roofL);
      c.rect(x0, wt - 1, w, 1, BSLOT.roofD);
      return;
    }
    case "canopy": {
      // A market hall: a striped canopy on iron posts, open below.
      const ridge = top + Math.max(2, Math.round(H * 0.3));
      c.rect(x0, top, w, ridge - top, BSLOT.roofS);
      c.tex(k.tex.corrugated, x0, ridge, w, wt - ridge, ox, 0);
      c.rect(x0, ridge, w, 1, BSLOT.roofL);
      return;
    }
    case "hip": {
      const ridge = top + Math.max(2, Math.round(H * 0.38));
      const hip = Math.max(2, Math.min(Math.floor(w / 2) - 3, Math.round(H * 0.8)));
      for (let y = top; y < wt; y++) {
        // Far slope (above the ridge) narrows to the back; the side hips run from the ridge's ends to the eave corners.
        if (y < ridge) {
          const ins = Math.round((hip * (ridge - y)) / Math.max(1, ridge - top)) + Math.round(hip * 0.5);
          c.rect(x0 + ins, y, w - 2 * ins, 1, y === top ? BSLOT.roofD : BSLOT.roofS);
          continue;
        }
        const t = (y - ridge) / Math.max(1, wt - ridge);
        const side = Math.round(Math.round(hip * 0.5) * (1 - t));
        c.tex(tex, x0 + side, y, w - 2 * side, 1, ox, y - ridge);
        c.rect(x0, y, side + 1, 1, BSLOT.roofL);
        c.rect(x1 - side - 1, y, side + 1, 1, BSLOT.roofS);
      }
      const r0 = x0 + Math.round(hip * 0.5);
      c.rect(r0, ridge, w - 2 * (r0 - x0), 1, BSLOT.roofL);
      c.rect(x0, wt - 1, w, 1, BSLOT.roofD);
      return;
    }
    case "front": {
      // A gable end to the street: the roof's outline peaks over the middle; the facade rises into a gable.
      const cx = x0 + Math.floor(w / 2);
      const shoulder = top + Math.round(H * 0.5);
      for (let y = top; y < wt; y++) {
        const half = y >= shoulder ? Math.ceil(w / 2) : Math.max(1, Math.round((Math.ceil(w / 2) * (y - top + 1)) / Math.max(1, shoulder - top + 1)));
        const l = Math.max(x0, cx - half);
        const r = Math.min(x1, cx + half);
        c.tex(tex, l, y, cx - l, 1, ox, y - top);
        c.rect(cx, y, r - cx, 1, BSLOT.roofS);
        for (let x = cx + 1; x < r; x += 4) c.px(x, y, (y - top) % 4 === 3 ? BSLOT.roofD : BSLOT.roofS);
        c.px(l, y, BSLOT.roofL);
        c.px(r - 1, y, BSLOT.roofD);
      }
      c.rect(cx - 1, top, 2, wt - top, BSLOT.roofL);
      // The gable: wall rising above the eaves, with bargeboards and a round window.
      const gh = Math.min(H - 2, Math.floor(w * 0.3));
      for (let r = 0; r < gh; r++) {
        const y = wt - 1 - r;
        const half = Math.round(((Math.floor(w / 2) - 2) * (gh - r)) / gh);
        if (half <= 0) continue;
        c.tex(k.tex[d.wall.tex], cx - half, y, half * 2, 1, 0, y);
        c.faces.push([cx - half, y, half * 2, 1, g.groundY]);
        c.px(cx - half - 1, y, BSLOT.trimL);
        c.px(cx + half, y, BSLOT.trimS);
      }
      if (gh >= 10) c.part(k.windows.round as Indexed, cx - 3, wt - gh + Math.max(1, Math.floor(gh / 2) - 3));
      c.rect(x0, wt - 1, cx - Math.floor(w / 2) + 2 - x0, 1, BSLOT.roofD);
      return;
    }
    case "mansard": {
      // A steep slate skirt (where the dormers sit) and a flat top behind a lead edge.
      const skirt = Math.min(H - 3, Math.max(11, Math.round(H * 0.62)));
      const y0 = wt - skirt;
      // The top: lead in standing seams.
      c.rect(x0, top, w, y0 - top, BSLOT.capL);
      for (let x = x0 + 3; x < x1 - 1; x += 4) c.rect(x, top + 1, 1, Math.max(0, y0 - top - 2), BSLOT.cap);
      c.rect(x0 + 1, top, w - 2, 1, BSLOT.capS);
      c.rect(x0, y0 - 1, w, 1, BSLOT.capL);
      for (let y = y0; y < wt; y++) {
        c.rect(x0, y, w, 1, (y - y0) % 3 === 2 ? BSLOT.capS : BSLOT.cap);
        for (let x = x0 + ((y - y0) % 2 === 0 ? 2 : 5); x < x1; x += 6) c.px(x, y, BSLOT.capS);
      }
      c.rect(x0, top, 1, H, BSLOT.capL);
      c.rect(x1 - 1, top, 1, H, BSLOT.capD);
      c.rect(x0, wt - 1, w, 1, BSLOT.capD);
      return;
    }
    case "saw": {
      // North-light sheds: teeth along the street, each a lit slope and a strip of glass.
      for (let x = x0; x < x1; x++) {
        const i = (x - x0) % 16;
        const yTop = top + Math.round(((15 - i) * Math.min(7, H - 4)) / 15);
        if (i >= 13) {
          c.rect(x, yTop, 1, wt - yTop, i === 15 ? BSLOT.frame : BSLOT.glassD);
          for (let y = yTop + 2; y < wt; y += 3) c.px(x, y, BSLOT.frame);
        } else {
          c.tex(k.tex.corrugated, x, yTop, 1, wt - yTop, ox + i, 0);
          c.px(x, yTop, BSLOT.roofL);
        }
      }
      c.rect(x0, wt - 1, w, 1, BSLOT.roofD);
      return;
    }
  }
}

function drawGround(
  c: Canvas,
  d: Design,
  s: FacadeSpec,
  g: Geo,
  k: ReturnType<typeof kitParts>,
  bays: { x: number; w: number }[],
  doorB: number,
  hasDoor: boolean,
  windows: Placed[],
  doors: { part: Indexed; kind: string; x: number; y: number }[],
  use: LightUse,
  seed: number,
) {
  const { m } = g;
  const gt = g.groundTop;
  const gy = g.groundY;
  const fx0 = m.corner === "left" ? m.x0 + 6 : m.x0;
  const fx1 = m.corner === "right" ? m.x1 - 6 : m.x1;
  const fw = fx1 - fx0;
  const door = (part: Indexed, kind: string, cx: number) => {
    doors.push({ part, kind, x: cx - Math.floor(part.w / 2), y: gy - part.h });
  };
  const ground = (b: { x: number; w: number }, part: Indexed, kind: string, dy: number) => {
    windows.push({ part, kind, x: b.x + Math.floor((b.w - part.w) / 2), y: gt + dy, use, ground: true });
  };
  switch (d.ground) {
    case "house": {
      let bayUsed = !d.bayWindow;
      bays.forEach((b, bi) => {
        if (bi === doorB) {
          door(d.wall.tex === "cladding" || s.hood === "rural" ? k.plankDoor : k.doors.panel, d.wall.tex === "cladding" || s.hood === "rural" ? "planks" : "panel", b.x + Math.floor(b.w / 2));
          return;
        }
        if (!bayUsed && b.w >= 15) {
          bayUsed = true;
          windows.push({ part: k.bay, kind: "bay", x: b.x + Math.floor((b.w - 14) / 2), y: gy - 20, use, ground: true });
          return;
        }
        ground(b, k.windows.ground as Indexed, "ground", 5);
      });
      return;
    }
    case "office": {
      bays.forEach((b, bi) => {
        if (bi === doorB) door(s.kind === "union_hall" || s.kind === "school" ? k.doors.civic : k.doors.panel, "panel", b.x + Math.floor(b.w / 2));
        else ground(b, (d.rhythm === "wide" ? k.windows.wide : d.upper === "arched" ? k.windows.arched : k.windows.ground) as Indexed, "ground", d.upper === "arched" ? 4 : 5);
      });
      if (d.rhythm === "strip") {
        // A flat canopy over the entrance.
        const cx = bays[Math.max(0, doorB)] ? (bays[Math.max(0, doorB)] as { x: number; w: number }).x + 8 : fx0 + 8;
        c.rect(cx - 10, gt + 1, 20, 2, BSLOT.capL);
        c.rect(cx - 10, gt + 3, 20, 1, BSLOT.capD);
      }
      return;
    }
    case "industrial": {
      bays.forEach((b, bi) => {
        if (bi === doorB) door(s.kind === "barn" || s.kind === "boathouse" ? k.plankDoor : k.doors.shutter, s.kind === "barn" || s.kind === "boathouse" ? "planks" : "shutter", b.x + Math.floor(b.w / 2));
        else if (bi !== doorB + 1 || s.kind === "shed") ground(b, k.windows.small as Indexed, "small", 4);
      });
      return;
    }
    case "market": {
      // Open below the canopy: deep shade, posts, and stalls of produce.
      c.rect(m.x0, gt - 6, fw, GROUND + 6, BSLOT.deep);
      c.faces.push([m.x0, gt - 6, fw, GROUND + 6, gy]);
      for (let x = m.x0; x < m.x1; x += 16) c.rect(x + (x === m.x0 ? 1 : 0), gt - 6, 2, GROUND + 6, BSLOT.iron);
      c.rect(m.x1 - 2, gt - 6, 2, GROUND + 6, BSLOT.iron);
      const produce = [BSLOT.bloom, BSLOT.lawn, BSLOT.bloomY, BSLOT.hedge, BSLOT.acc];
      for (let x = m.x0 + 4, i = 0; x + 9 < m.x1; x += 12, i++) {
        c.rect(x, gy - 9, 9, 6, BSLOT.wood);
        c.rect(x, gy - 3, 9, 1, BSLOT.woodD);
        for (let j = 0; j < 3; j++) c.rect(x + 1 + j * 3, gy - 10, 2, 2, produce[(i + j + seed) % produce.length] as number);
        c.rect(x, gy - 13, 9, 2, i % 2 ? BSLOT.stripe : BSLOT.acc);
      }
      if (hasDoor) doors.push({ part: blank(10, 2), kind: "open", x: Math.max(m.x0, Math.min(m.x1 - 10, (bays[Math.max(0, doorB)]?.x ?? m.x0) + 3)), y: gy - 2 });
      return;
    }
    case "diner": {
      // Big windows in a strip, a striped band under the cornice, a glazed door.
      c.rect(m.x0, g.wallTop + g.parapet, m.x1 - m.x0, 2, BSLOT.acc);
      c.rect(m.x0, g.wallTop + g.parapet + 2, m.x1 - m.x0, 1, BSLOT.stripe);
      c.rect(m.x0, g.wallTop + g.parapet + 3, m.x1 - m.x0, 2, BSLOT.acc);
      c.rect(m.x0, g.wallTop + g.parapet + 5, m.x1 - m.x0, 1, BSLOT.accS);
      bays.forEach((b, bi) => {
        if (bi === doorB) door(k.doors.shop, "shop", b.x + Math.floor(b.w / 2));
        else windows.push({ part: k.strip, kind: "strip", x: b.x, y: gy - 15, use, ground: true });
      });
      c.rect(m.x0, gy - 7, m.x1 - m.x0, 1, BSLOT.stripe);
      return;
    }
    case "civic":
      bays.forEach((b, bi) => {
        if (bi === doorB) door(k.doors.civic, "civic", b.x + Math.floor(b.w / 2));
        else ground(b, k.windows.arched as Indexed, "arched", 6);
      });
      return;
    case "glazed": {
      // Plate glass in slim iron frames, a dark fascia with a light line.
      c.rect(fx0, gt, fw, 4, BSLOT.iron);
      c.rect(fx0, gt + 1, fw, 1, BSLOT.ironL);
      bays.forEach((b, bi) => {
        if (bi === doorB) {
          door(k.doors.shop, "shop", b.x + Math.floor(b.w / 2));
          return;
        }
        const glass = blank(b.w - 2, GROUND - 7);
        fill(glass, 0, 0, glass.w, glass.h, BSLOT.frame);
        fill(glass, 1, 1, glass.w - 2, glass.h - 3, BSLOT.glass);
        fill(glass, 1, 1, glass.w - 2, 1, BSLOT.glassD);
        for (let y = 2; y < glass.h - 3; y++) for (let x = 2; x < 5 && x < glass.w - 1; x++) if ((x + y) % 5 === 0) glass.px[y * glass.w + x] = BSLOT.glassY;
        fill(glass, Math.floor(glass.w / 2), 1, 1, glass.h - 3, BSLOT.frame);
        fill(glass, 0, glass.h - 2, glass.w, 2, BSLOT.iron);
        windows.push({ part: glass, kind: "shopfront", x: b.x + 1, y: gt + 5, use, ground: true });
      });
      if (d.awning >= 0) {
        c.tex(k.awning, fx0, gt + 4, fw, 4, (seed >>> 2) & 7, 2);
      }
      return;
    }
    case "classic":
    case "board": {
      bays.forEach((b, bi) => {
        if (bi === doorB) {
          door(k.doors.shop, "shop", b.x + Math.floor(b.w / 2));
          return;
        }
        windows.push({ part: b.w >= 16 ? k.shopfront : crop(k.shopfront, 0, 0, b.w, 18), kind: "shopfront", x: b.x, y: gy - 18, use, ground: true });
      });
      // Fascia over the whole front; a muted awning, or (board) a shutter box and a hanging sign.
      for (let x = fx0; x < fx1; x += 16) c.part(crop(k.fascia, 0, 0, Math.min(16, fx1 - x), 5), x, gt);
      if (d.ground === "classic" && d.awning !== -1) {
        for (let x = fx0; x < fx1; x += 16) c.part(crop(k.awning, (seed >>> 2) & 7 ? 0 : 0, 0, Math.min(16, fx1 - x), 6), x, gt + 5);
      } else if (d.ground === "board") {
        c.rect(fx0, gt + 5, fw, 2, BSLOT.iron);
        c.rect(fx0, gt + 7, fw, 1, BSLOT.deep);
        if (!s.named && hasDoor) {
          const b = bays[Math.max(0, doorB)] as { x: number; w: number };
          c.part(k.hanging, b.x + b.w - 3, gt - 10);
        }
      }
      return;
    }
  }
}

function drawCorner(
  c: Canvas,
  d: Design,
  g: Geo,
  k: ReturnType<typeof kitParts>,
  side: "left" | "right",
  windows: Placed[],
  doors: { part: Indexed; kind: string; x: number; y: number }[],
  use: LightUse,
  upperUse: LightUse,
) {
  const { m } = g;
  const x = side === "left" ? m.x0 : m.x1 - 6;
  const top = g.wallTop + g.parapet;
  // The canted face: lit on the left corner (it faces the light), shaded on the right.
  c.rect(x, top, 6, g.groundY - top, side === "left" ? BSLOT.wallL : BSLOT.wallS);
  c.rect(side === "left" ? x + 5 : x, top, 1, g.groundY - top, side === "left" ? BSLOT.wallS : BSLOT.wallD);
  c.rect(x, g.groundY - 1, 6, 1, BSLOT.wallD);
  for (const ft of g.floors) if (ft >= top + 2) windows.push({ part: k.windows.narrow as Indexed, kind: "narrow", x: x + 1, y: ft + 3, use: upperUse, ground: false });
  if (d.family === "shop") doors.push({ part: k.cornerDoor, kind: "shop", x, y: g.groundY - k.cornerDoor.h });
  else windows.push({ part: k.windows.narrow as Indexed, kind: "narrow", x: x + 1, y: g.groundTop + 6, use, ground: true });
}

function drawRoofPieces(c: Canvas, d: Design, s: FacadeSpec, g: Geo, k: ReturnType<typeof kitParts>, seed: number, windows: Placed[], shadows: ShadowCaster[]) {
  const { m } = g;
  const w = m.x1 - m.x0;
  const top = g.roofTop;
  const wt = g.wallTop;
  const H = wt - top;
  // A corner turret: a round tower on the corner with a conical cap, standing above the roof.
  if (m.turret && m.corner) {
    const tw = 12;
    const tx = m.corner === "left" ? m.x0 : m.x1 - tw;
    const capH = 12;
    const bodyTop = Math.max(capH, top - 4);
    c.tex(k.tex[d.wall.tex], tx, bodyTop, tw, wt + 2 - bodyTop, 0, 0);
    c.rect(tx, bodyTop, 1, wt + 2 - bodyTop, BSLOT.wallL);
    c.rect(tx + tw - 2, bodyTop, 2, wt + 2 - bodyTop, BSLOT.wallS);
    c.rect(tx, bodyTop, tw, 1, BSLOT.trim);
    if (wt - bodyTop >= 14) windows.push({ part: k.windows.small as Indexed, kind: "small", x: tx + 3, y: bodyTop + 3, use: "home", ground: false });
    const capMap = d.wall.ramp === "stone" || s.hood === "core" ? COPPER_CAP : undefined;
    for (let r = 0; r < capH; r++) {
      const half = Math.max(1, Math.round(((tw / 2) * (r + 1)) / capH));
      const y = bodyTop - capH + r;
      const cx = tx + tw / 2;
      c.part(rowOf(half * 2, BSLOT.cap, BSLOT.capS, BSLOT.capL), cx - half, y, capMap);
    }
    c.px(tx + tw / 2 - 1, bodyTop - capH - 1, BSLOT.iron);
    c.px(tx + tw / 2 - 1, bodyTop - capH - 2, BSLOT.ironL);
    shadows.push({ footprint: [[tx, 0], [tx + tw, 0], [tx + tw, s.h * T - m.inset], [tx, s.h * T - m.inset]], height: Math.round((g.groundY - bodyTop + capH) * 0.85) });
  }
  if (H < 6) return;
  if (m.roof === "flat" && d.family !== "civic") {
    // Clutter on the roof: vents and skylights low; tanks, bulkheads and AC units that stand up into the skyline.
    const n = Math.max(1, Math.floor(w / 18));
    for (let i = 0; i < n; i++) {
      const v = hash32(seed, "clutter", i);
      const kind = v % 6;
      const lo = m.x0 + 4 + Math.floor((i * (w - 8)) / n);
      const span = Math.max(1, Math.floor((w - 8) / n) - 12);
      const x = lo + ((v >>> 8) % span);
      if (kind === 0 && H >= 10) {
        c.part(k.tank, x, top + Math.min(H - 4, 6) - k.tank.h + 4);
      } else if (kind === 1 && H >= 12) {
        c.part(k.hatch, x, top + 3 + Math.min(4, H - 14));
      } else if (kind === 2) {
        c.part(k.ac, x, top + 4 + ((v >>> 16) % Math.max(1, H - 10)));
      } else if (kind === 3) {
        c.part(k.skylight, x, top + 4 + ((v >>> 16) % Math.max(1, H - 10)));
      } else {
        c.part(k.vent, x, top + 4 + ((v >>> 16) % Math.max(1, H - 9)));
      }
    }
    return;
  }
  // Chimneys on the party walls (and between bays of wide masses), poking above the ridge.
  if (d.chimneys && (m.roof === "gable" || m.roof === "hip" || m.roof === "front" || m.roof === "mansard")) {
    const xs: number[] = [];
    if (m.roof === "front") xs.push(m.x0 + 3);
    else {
      xs.push(m.x0 + 2);
      if (w >= 48 && hash32(seed, "midstack") % 2 === 0) xs.push(m.x0 + Math.floor(w / 2) - 4);
      if (hash32(seed, "endstack") % 3 !== 0) xs.push(m.x1 - 9);
    }
    for (const [i, x] of xs.entries()) {
      const wide = hash32(seed, "stackw", i) % 3 === 0 && m.roof !== "front";
      const part = wide ? k.stack2 : k.chimney;
      const y = top - 4;
      if (y < 0) continue;
      c.part(part, x, y, STACK_MAP);
      const bottom = top + Math.max(2, Math.round(H * 0.35));
      for (let yy = y + part.h; yy < bottom; yy++) {
        c.rect(x, yy, part.w - 1, 1, BSLOT.stack);
        c.px(x + part.w - 1, yy, BSLOT.stackS);
      }
    }
  }
  // Dormers on mansards (always) and on some old gabled roofs.
  if (d.dormers && (m.roof === "mansard" || m.roof === "gable") && H >= 14) {
    const bays = baysOf(m.x0, m.x1);
    bays.forEach((b, bi) => {
      if (m.roof === "gable" && bi % 2 === 1) return;
      const x = b.x + Math.floor((b.w - k.dormer.w) / 2);
      const y = wt - k.dormer.h + 1;
      c.part(k.dormer, x, y);
      windows.push({ part: crop(k.dormer, 2, 6, 6, 4), kind: "dormer", x: x + 2, y: y + 6, use: d.family === "shop" ? "home" : lightUseOf(d.kind, d.family), ground: false });
    });
  }
}

/** One row of a cone or dome: lit left, base, shaded right. */
function rowOf(w: number, base: number, shade: number, light: number): Indexed {
  const r = blank(Math.max(1, w), 1);
  for (let x = 0; x < r.w; x++) r.px[x] = x < Math.max(1, Math.floor(r.w / 3)) ? light : x >= r.w - Math.max(1, Math.floor(r.w / 3)) ? shade : base;
  return r;
}

/** Civic landmarks: giant columns and a pediment on the centre block, then a clock tower or a dome. */
function drawCivic(c: Canvas, d: Design, s: FacadeSpec, geos: Geo[], k: ReturnType<typeof kitParts>, shadows: ShadowCaster[]) {
  const centre = geos.find((g) => !g.m.wing) ?? (geos[0] as Geo);
  const { m } = centre;
  const w = m.x1 - m.x0;
  const top = centre.wallTop + centre.parapet;
  // Giant columns through the ground and first floors, in front of a shaded portico.
  const colTop = centre.floors[0] !== undefined ? (centre.floors[1] ?? centre.floors[0]) + 2 : centre.groundTop;
  c.rect(m.x0 + 2, colTop + 3, w - 4, centre.groundY - colTop - 5, BSLOT.wallS);
  for (let x = m.x0 + 3; x < m.x1 - 4; x += Math.max(8, Math.floor((w - 6) / 6))) {
    c.part(k.column.capital, x, colTop);
    for (let y = colTop + 3; y < centre.groundY - 2; y++) c.part(k.column.shaft, x, y);
    c.part(k.column.base, x, centre.groundY - 2);
  }
  // Entablature and a pediment over it.
  c.rect(m.x0, colTop - 2, w, 2, BSLOT.trim);
  c.rect(m.x0, colTop - 1, w, 1, BSLOT.trimS);
  if (w >= 32) {
    const px = m.x0 + Math.floor(w / 2) - 16;
    const py = Math.max(0, top - 8);
    c.part(k.pediment, px, py);
    c.part(mirrorX(k.pediment), px + 16, py, SHADE_TRIM);
  }
  const cx = m.x0 + Math.floor(w / 2);
  if (s.kind === "council_chamber" || s.kind === "parliament" || s.kind === "regional_assembly") {
    // A dome on a drum.
    const dx = cx - 12;
    const dy = Math.max(0, centre.roofTop + 4 - k.dome.h);
    c.part(k.dome, dx, dy, DOME_MAP);
    shadows.push({ footprint: [[dx, 8], [dx + 24, 8], [dx + 24, 24], [dx, 24]], height: Math.round((centre.groundY - dy) * 0.85) });
  } else {
    // A clock tower: a stone shaft with a clock, an open belfry and a slate (or copper) spire.
    const tw = 18;
    const tx = cx - tw / 2;
    const spire = 12;
    const shaftTop = Math.max(spire + 1, centre.roofTop - 26);
    const shaftBottom = centre.roofTop + Math.floor((centre.wallTop - centre.roofTop) / 2);
    c.tex(k.tex.stone, tx, shaftTop, tw, shaftBottom - shaftTop, 0, 0);
    c.rect(tx, shaftTop, 1, shaftBottom - shaftTop, BSLOT.wallL);
    c.rect(tx + tw - 2, shaftTop, 2, shaftBottom - shaftTop, BSLOT.wallS);
    c.rect(tx - 1, shaftTop, tw + 2, 2, BSLOT.trimL);
    c.rect(tx - 1, shaftTop + 2, tw + 2, 1, BSLOT.trimS);
    // Belfry openings, then the clock.
    c.rect(tx + 4, shaftTop + 4, 3, 6, BSLOT.deep);
    c.rect(tx + 11, shaftTop + 4, 3, 6, BSLOT.deep);
    c.part(k.clock, tx + 4, shaftTop + 12);
    c.rect(tx - 1, shaftTop + 22, tw + 2, 1, BSLOT.trim);
    for (let r = 0; r < spire; r++) {
      const half = Math.max(1, Math.round(((tw / 2 + 1) * (r + 1)) / spire));
      c.part(rowOf(half * 2, BSLOT.cap, BSLOT.capS, BSLOT.capL), cx - half, shaftTop - spire + r, COPPER_CAP);
    }
    c.px(cx - 1, shaftTop - spire - 1, BSLOT.iron);
    c.px(cx - 1, shaftTop - spire - 2, BSLOT.ironL);
    shadows.push({ footprint: [[tx, 8], [tx + tw, 8], [tx + tw, 26], [tx, 26]], height: Math.round((centre.groundY - shaftTop + spire) * 0.85) });
  }
  void d;
}

/** The sign: a board with the place's pictogram (no text, R14), where the design puts it. Returns true when drawn. */
function drawSign(c: Canvas, d: Design, s: FacadeSpec, geos: Geo[], k: ReturnType<typeof kitParts>, doorCx: number): boolean {
  const g = geos.find((x) => doorCx >= x.m.x0 && doorCx < x.m.x1) ?? (geos[0] as Geo);
  const key = s.home ? "home" : s.kind in k.pictos ? s.kind : d.family === "civic" ? (s.kind === "town_hall" ? "civic" : "dome") : d.family === "shop" ? "shop" : "home";
  const picto = k.pictos[key] as Indexed;
  const W = s.w * T;
  if (d.kind === "diner" || g.storeys === 1) {
    // On posts above the roof edge: a rooftop sign.
    const x = Math.max(1, Math.min(W - 12, doorCx - 5));
    const y = Math.max(0, g.roofTop - 6);
    c.rect(x + 2, y + 10, 1, 6, BSLOT.iron);
    c.rect(x + 8, y + 10, 1, 6, BSLOT.iron);
    c.part(k.board, x, y);
    c.part(picto, x + 2, y + 2);
    return true;
  }
  if (d.kind === "news_office") {
    // A blade sign: a tall board on brackets, standing out from the facade.
    const x = g.m.x1 - 12;
    const y = g.floors[g.floors.length - 1] ?? g.groundTop - 30;
    c.rect(x - 1, y + 2, 2, 1, BSLOT.iron);
    c.rect(x - 1, y + 24, 2, 1, BSLOT.iron);
    c.rect(x + 1, y, 9, 28, BSLOT.signS);
    c.rect(x + 2, y + 1, 7, 26, BSLOT.sign);
    c.part(picto, x + 2, y + 2);
    for (let i = 0; i < 4; i++) c.rect(x + 3, y + 12 + i * 4, 5, 2, BSLOT.picto);
    return true;
  }
  const y = g.storeys > 1 ? g.groundTop - 13 : Math.max(0, g.wallTop - 6);
  const x = Math.max(1, Math.min(W - 12, doorCx - 5));
  c.part(k.board, x, y);
  c.part(picto, x + 2, y + 2);
  if (d.kind === "party_office") {
    // A flag on a pole off the first floor.
    const fx = Math.min(W - 10, x + 13);
    for (let i = 0; i < 6; i++) c.px(fx + i, y + 6 - i, BSLOT.iron);
    c.rect(fx + 3, y - 4, 7, 5, BSLOT.poster);
    c.rect(fx + 3, y - 2, 7, 1, BSLOT.posterS);
  }
  if (d.kind === "union_hall") {
    // A banner hanging beside the door.
    const bx = Math.max(1, x - 9);
    c.rect(bx, g.groundTop - 16, 6, 14, BSLOT.acc);
    c.rect(bx, g.groundTop - 16, 6, 1, BSLOT.iron);
    c.rect(bx + 1, g.groundTop - 12, 4, 1, BSLOT.stripe);
    c.rect(bx + 1, g.groundTop - 9, 4, 1, BSLOT.stripe);
    c.px(bx, g.groundTop - 2, BSLOT.accS);
    c.px(bx + 5, g.groundTop - 2, BSLOT.accS);
  }
  return true;
}

/** Wear and life on a facade, from its seed: streaks under sills, a drainpipe, flower boxes, ivy, a school's cupola. */
function drawWear(c: Canvas, d: Design, g: Geo, seed: number, windows: Placed[]) {
  const { m } = g;
  if (m.roof === "canopy") return;
  if (d.drain) {
    const x = hash32(seed, "drainSide") % 2 === 0 ? m.x0 + 1 : m.x1 - 3;
    c.rect(x, g.wallTop + 2, 1, g.wallH - 3, BSLOT.iron);
    c.px(x, g.wallTop + 2, BSLOT.ironL);
    c.rect(x - 1, g.groundY - 2, 3, 1, BSLOT.iron);
  }
  for (const [i, w] of windows.entries()) {
    if (w.x < m.x0 || w.x >= m.x1) continue;
    const v = hash32(seed, "wear", i);
    if (!w.ground && v % 5 === 0 && w.y + w.part.h + 3 < g.groundY) c.rect(w.x + 2 + (v % 3), w.y + w.part.h, 1, 2 + ((v >>> 4) % 2), BSLOT.wallS);
    if (d.boxes && !w.ground && w.kind !== "small" && (v >>> 8) % 2 === 0) {
      const y = w.y + w.part.h - 2;
      c.rect(w.x, y, w.part.w, 1, BSLOT.wood);
      for (let x = w.x; x < w.x + w.part.w; x += 2) c.px(x, y - 1, (x + v) % 4 < 2 ? BSLOT.bloom : BSLOT.hedge);
    }
  }
  if (d.ivy) {
    const x = hash32(seed, "ivyX") % 2 === 0 ? m.x0 : m.x1 - 7;
    for (let y = g.wallTop + 6; y < g.groundY - 1; y++) {
      const span = 2 + ((hash32(seed, "ivy", y) % 4) as number);
      c.rect(x + ((y * 3) % 2), y, span, 1, (y % 3 === 0 ? BSLOT.hedgeS : BSLOT.hedge) as number);
    }
  }
  if (d.kind === "school" && !m.wing && g.wallTop - g.roofTop >= 10) {
    const k = kitParts();
    const cx = m.x0 + Math.floor((m.x1 - m.x0) / 2);
    c.part(k.cupola, cx - 6, Math.max(0, g.roofTop - 8));
  }
}

/** The slot table a building would use (for the kit's parts sheet): its materials, with lit glass in the warm light. */
export function kitTable(s: FacadeSpec, p: Palette): Uint8Array {
  return litTable(materials(s, designOf(s), p), "home", p);
}
