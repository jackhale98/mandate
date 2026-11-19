// Buildings: the facade grammar (§2.4) that lays the kit in ./buildingParts
// out on a footprint, on the device (D10). A facade is split into storeys
// (ground 24 px, upper 18 px) and 16 px bays (one per footprint column); the
// door bay gets the door, the others windows or a shopfront; the roof is the
// footprint's depth seen from above. Materials are picked per building by
// rendezvous hashing of its visual seed, from the neighbourhood's mix, so a
// place looks the same every time. Windows and doors come back with their
// lit (emissive) images and a use tag; the building with its shadow
// footprint and height.
import { hash32, rendezvous } from "../../src/core/hash";
import { blank, blit, crop, fill, grid, mirrorX, remap, tileFill, type Indexed } from "../../src/core/image";
import { ramp, type Palette } from "../../src/core/palette";
import {
  AWNING,
  BLEGEND,
  BSLOT,
  BSLOT_COUNT,
  CHIMNEY,
  COLUMN,
  DOME,
  DOORS,
  FASCIA,
  LIT_OVERLAYS,
  PEDIMENT_HALF,
  PICTOGRAMS,
  PLANKS,
  SHOPFRONT,
  SIGN_BOARD,
  SKYLIGHT,
  TEX,
  TURRET,
  VENT,
  WINDOWS,
  type DoorKind,
  type WindowKind,
} from "./buildingParts";

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

export interface Facade {
  /** Footprint plus one roof row of tiles (h + 1) × 16 tall; anchor: the footprint's top-left (16 px down). */
  albedo: Indexed;
  windows: Opening[];
  doors: Opening[];
  shadow: { footprint: [number, number][]; height: number };
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
  tex: keyof typeof TEX;
  /** Window frames: light (on brick and stone) or dark (on pale render). */
  frames: "light" | "dark";
  weight?: number;
}

const WALLS: Record<string, WallMat> = {
  brick: { id: "brick", ramp: "brick", off: 0, tex: "brick", frames: "light" },
  brickDark: { id: "brickDark", ramp: "brick", off: -1, tex: "brick", frames: "light" },
  stone: { id: "stone", ramp: "stone", off: 0, tex: "stone", frames: "dark" },
  render: { id: "render", ramp: "render", off: 0, tex: "render", frames: "dark" },
  sky: { id: "sky", ramp: "cloth_sky", off: 1, tex: "render", frames: "light" },
  mint: { id: "mint", ramp: "cloth_teal", off: 1, tex: "render", frames: "light" },
  butter: { id: "butter", ramp: "cloth_mustard", off: 1, tex: "render", frames: "dark" },
  peach: { id: "peach", ramp: "terracotta", off: 1, tex: "render", frames: "light" },
  cladding: { id: "cladding", ramp: "slate", off: 0, tex: "cladding", frames: "light" },
};

const HOOD_WALLS: Record<string, readonly (readonly [string, number])[]> = {
  core: [["stone", 3], ["render", 2], ["brick", 3], ["butter", 1], ["sky", 1]],
  inner: [["brick", 4], ["brickDark", 2], ["render", 2], ["sky", 1], ["mint", 1], ["peach", 1], ["butter", 1]],
  suburb: [["render", 3], ["brick", 2], ["butter", 1], ["mint", 1]],
  waterfront: [["sky", 2], ["mint", 2], ["render", 2], ["peach", 1], ["brick", 1]],
  industrial: [["brickDark", 3], ["cladding", 3], ["brick", 1]],
  rural: [["render", 3], ["stone", 2], ["brick", 1]],
};

const HOOD_ROOFS: Record<string, readonly (readonly [string, number])[]> = {
  core: [["slates", 3], ["tiles", 1]],
  inner: [["tiles", 3], ["slates", 2]],
  suburb: [["tiles", 3], ["slates", 1]],
  waterfront: [["slates", 3], ["tiles", 1]],
  industrial: [["slates", 1]],
  rural: [["tiles", 2], ["slates", 1]],
};

const DOOR_PAINTS = ["cloth_navy", "cloth_green", "cloth_red", "cloth_teal", "wood", "cloth_charcoal", "cloth_plum", "cloth_mustard"] as const;
const AWNINGS = ["flag", "party_blue", "party_green", "cloth_red", "cloth_teal", "cloth_mustard", "cloth_plum"] as const;

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

const pickW = (seed: number, salt: string, xs: readonly (readonly [string, number])[]): string => rendezvous(seed, salt, xs.map(([id, weight]) => ({ id, weight }))).id;
const pick1 = (seed: number, salt: string, xs: readonly string[]): string => rendezvous(seed, salt, xs.map((id) => ({ id }))).id;

function stepOf(p: Palette, name: string, k: number): number {
  const r = ramp(p, name);
  return r.steps[Math.min(r.steps.length - 1, Math.max(0, r.anchorStep + k))] as number;
}

interface Materials {
  wall: WallMat;
  roof: "tiles" | "slates" | "flat";
  door: string;
  awning: string;
  table: Uint8Array;
}

function materials(s: FacadeSpec, family: Family, p: Palette): Materials {
  const seed = s.visualSeed;
  let wallId = pickW(seed, "wall", HOOD_WALLS[s.hood] ?? (HOOD_WALLS.inner as readonly (readonly [string, number])[]));
  if (family === "civic") wallId = "stone";
  else if (family === "industrial") wallId = pickW(seed, "wall", HOOD_WALLS.industrial as readonly (readonly [string, number])[]);
  const wall = WALLS[wallId] as WallMat;
  let roof = pickW(seed, "roof", HOOD_ROOFS[s.hood] ?? (HOOD_ROOFS.inner as readonly (readonly [string, number])[])) as Materials["roof"];
  if (family === "civic" || family === "industrial" || (family === "shop" && (s.hood === "core" || s.w >= 5))) roof = "flat";
  const door = pick1(seed, "door", DOOR_PAINTS);
  const awning = pick1(seed, "awning", AWNINGS);
  const t = new Uint8Array(BSLOT_COUNT);
  const W = (k: number) => stepOf(p, wall.ramp, wall.off + k);
  t[BSLOT.wall] = W(0);
  t[BSLOT.wallS] = W(-1);
  t[BSLOT.wallD] = W(-2);
  t[BSLOT.wallL] = W(1);
  const trimRamp = wall.ramp === "stone" ? "chalk" : "stone";
  t[BSLOT.trim] = stepOf(p, trimRamp, trimRamp === "stone" ? 1 : 0);
  t[BSLOT.trimS] = stepOf(p, trimRamp, trimRamp === "stone" ? 0 : -1);
  t[BSLOT.trimL] = stepOf(p, trimRamp, trimRamp === "stone" ? 1 : 0);
  // Flat roofs are gravel, a step darker than the pavement.
  const roofRamp = roof === "tiles" ? "terracotta" : roof === "slates" ? "slate" : "paving";
  const ro = roof === "flat" ? -1 : 0;
  t[BSLOT.roof] = stepOf(p, roofRamp, ro);
  t[BSLOT.roofS] = stepOf(p, roofRamp, ro - 1);
  t[BSLOT.roofD] = stepOf(p, roofRamp, ro - 2);
  t[BSLOT.roofL] = stepOf(p, roofRamp, ro + 1);
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
  t[BSLOT.door] = stepOf(p, door, 0);
  t[BSLOT.doorS] = stepOf(p, door, -1);
  t[BSLOT.doorL] = stepOf(p, door, 1);
  t[BSLOT.acc] = stepOf(p, awning, 0);
  t[BSLOT.accS] = stepOf(p, awning, -1);
  t[BSLOT.stripe] = stepOf(p, "chalk", 0);
  t[BSLOT.stripeS] = stepOf(p, "chalk", -1);
  t[BSLOT.iron] = stepOf(p, "iron", 0);
  t[BSLOT.ironL] = stepOf(p, "iron", 1);
  t[BSLOT.deep] = stepOf(p, "ink", -1);
  const signRamp = s.home ? "lamp" : (SIGN_RAMP[s.kind] ?? (family === "civic" ? "lamp" : "chalk"));
  t[BSLOT.sign] = stepOf(p, signRamp, 0);
  t[BSLOT.signS] = stepOf(p, signRamp, -1);
  t[BSLOT.picto] = stepOf(p, "ink", -2);
  for (const [slots, r] of [
    [[BSLOT.cap, BSLOT.capS, BSLOT.capD, BSLOT.capL], "slate"],
    [[BSLOT.dome, BSLOT.domeS, BSLOT.domeD, BSLOT.domeL], "party_green"],
    [[BSLOT.stack, BSLOT.stackS, BSLOT.stackD, BSLOT.stackL], "brick"],
  ] as const) {
    t[slots[0]] = stepOf(p, r, 0);
    t[slots[1]] = stepOf(p, r, -1);
    t[slots[2]] = stepOf(p, r, -2);
    t[slots[3]] = stepOf(p, r, 1);
  }
  t[BSLOT.wood] = stepOf(p, "wood", 0);
  t[BSLOT.woodD] = stepOf(p, "wood", -1);
  t[BSLOT.woodL] = stepOf(p, "wood", 1);
  return { wall, roof, door, awning, table: t };
}

/** The lit colours: warm gold for homes, a cooler white for shops and offices (§6). */
function litTable(base: Uint8Array, use: LightUse, p: Palette): Uint8Array {
  const t = base.slice();
  // Offices light a cooler white; homes, shops and civic rooms the warm gold.
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
    windows: Object.fromEntries(Object.entries(WINDOWS).map(([k, v]) => [k, B(v)])) as Record<WindowKind, Indexed>,
    doors: Object.fromEntries(Object.entries(DOORS).map(([k, v]) => [k, B(v)])) as Record<DoorKind, Indexed>,
    shopfront: B(SHOPFRONT),
    fascia: B(FASCIA),
    awning: B(AWNING),
    planks: B(PLANKS),
    column: { capital: B(COLUMN.capital), shaft: B(COLUMN.shaft), base: B(COLUMN.base) },
    pediment: B(PEDIMENT_HALF),
    turret: B(TURRET),
    dome: B(DOME),
    chimney: B(CHIMNEY),
    vent: B(VENT),
    skylight: B(SKYLIGHT),
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
  out.push({ name: "shopfront", img: k.shopfront }, { name: "fascia", img: k.fascia }, { name: "awning", img: k.awning }, { name: "planks", img: k.planks });
  out.push({ name: "column capital", img: k.column.capital }, { name: "column base", img: k.column.base }, { name: "pediment", img: k.pediment });
  out.push({ name: "turret", img: k.turret }, { name: "dome", img: k.dome }, { name: "chimney", img: k.chimney }, { name: "vent", img: k.vent }, { name: "skylight", img: k.skylight }, { name: "sign board", img: k.board });
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
function litPart(part: Indexed, kind: WindowKind | "shopfront" | DoorKind, variant: number): Indexed {
  const out = remap(part, LIGHT_UP);
  if (kind === "sash" || kind === "ground" || kind === "arched") {
    if (variant === 1) {
      const [c0, c1] = LIT_OVERLAYS.curtains[kind];
      for (let y = 0; y < out.h; y++) for (const c of [c0, c1]) if (out.px[y * out.w + c] !== 0) out.px[y * out.w + c] = BSLOT.litD;
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
  }
  return out;
}

// ---------------------------------------------------------------------------
// The grammar

/** Build one building's facade, roof, openings and shadow. */
export function buildFacade(s: FacadeSpec, p: Palette): Facade {
  const k = kitParts();
  const family = familyOf(s.kind, s.landmark);
  const m = materials(s, family, p);
  const use = lightUseOf(s.kind, family);
  const seed = s.visualSeed;
  const W = s.w * T;
  const H = (s.h + 1) * T;
  const img = blank(W, H);
  const flat = m.roof === "flat";
  const parapet = flat ? 5 : 0;
  let storeys = Math.max(1, Math.min(3, s.kind === "shed" || s.kind === "barn" ? 1 : s.storeys));
  const wallFor = (n: number) => GROUND + UPPER * (n - 1) + parapet;
  while (storeys > 1 && H - wallFor(storeys) < 14) storeys--;
  const wallH = Math.min(H - 10, wallFor(storeys));
  const wallTop = H - wallH;
  const groundTop = H - GROUND;
  const doorBay = Math.max(0, Math.min(s.w - 1, s.doorX));
  const windows: { part: Indexed; kind: WindowKind | "shopfront"; x: number; y: number; use: LightUse }[] = [];
  const doors: { part: Indexed; kind: DoorKind; x: number; y: number }[] = [];
  const boarded = s.conditions.includes("boarded");

  // ---- roof (the footprint seen from above, down to the eaves)
  if (flat) {
    tileFill(img, k.tex.flat, 0, 0, W, wallTop, (seed >>> 3) & 15, 0);
    // The parapet's coping all round (light on the far and left sides), and its shadow on the gravel.
    fill(img, 0, 0, W, 2, BSLOT.trimL);
    fill(img, 0, 0, 2, wallTop, BSLOT.trimL);
    fill(img, W - 2, 0, 2, wallTop, BSLOT.trimS);
    fill(img, 2, 2, W - 4, 1, BSLOT.roofD);
    fill(img, 2, 2, 1, wallTop - 2, BSLOT.roofD);
  } else {
    tileFill(img, m.roof === "tiles" ? k.tex.tiles : k.tex.slates, 0, 1, W, wallTop - 1, (seed >>> 5) & 15, 0);
    fill(img, 0, 0, W, 1, BSLOT.roofL);
    fill(img, 0, 1, W, 1, BSLOT.roofS);
    fill(img, 0, wallTop - 2, W, 1, BSLOT.roofS);
    fill(img, 0, wallTop - 1, W, 1, BSLOT.roofD);
    fill(img, 0, 0, 1, wallTop, BSLOT.roofS);
    fill(img, W - 1, 0, 1, wallTop, BSLOT.roofD);
  }

  // ---- wall
  tileFill(img, k.tex[m.wall.tex], 0, wallTop, W, wallH, (seed >>> 7) & 15, 0);
  if (flat) {
    fill(img, 0, wallTop, W, 1, BSLOT.trimL);
    fill(img, 0, wallTop + 1, W, 1, BSLOT.trimS);
    fill(img, 0, wallTop + parapet - 1, W, 1, BSLOT.trimS);
  } else {
    // The eaves' shadow on the top of the wall.
    fill(img, 0, wallTop, W, 2, BSLOT.wallS);
  }
  fill(img, 0, H - 2, W, 1, BSLOT.wallS);
  fill(img, 0, H - 1, W, 1, BSLOT.wallD);
  // Party walls: a dark seam at the right edge, a light one at the left (the light comes from the upper left).
  fill(img, W - 1, wallTop, 1, wallH, BSLOT.wallD);
  fill(img, 0, wallTop + 2, 1, wallH - 4, BSLOT.wallL);

  // Storey bands.
  const floorTops: number[] = [];
  for (let f = 1; f < storeys; f++) floorTops.push(groundTop - UPPER * f);
  if (storeys > 1) fill(img, 0, groundTop, W, 1, BSLOT.trimS);
  for (const ft of floorTops) if (ft > wallTop + parapet) fill(img, 0, ft, W, 1, BSLOT.trimS);

  // Sign: over the door, on the first floor (its window gives way), or on the roof edge of a one-storey building.
  const signed = s.named || s.home;
  const signX = Math.max(1, Math.min(W - 12, doorBay * T + 3));
  const signY = storeys > 1 ? groundTop - 13 : Math.max(0, wallTop - 6);

  // ---- upper windows
  const upperKind: WindowKind = family === "civic" ? "arched" : "sash";
  for (const ft of floorTops) {
    for (let b = 0; b < s.w; b++) {
      if (signed && b === doorBay && ft === floorTops[0]) continue;
      const part = k.windows[upperKind];
      const y = ft + (upperKind === "arched" ? 2 : 3);
      if (y < wallTop + parapet) continue;
      windows.push({ part, kind: upperKind, x: b * T + 4, y, use: family === "shop" ? (s.kind === "shop" || s.kind === "diner" ? "home" : "office") : use });
    }
  }

  // ---- ground floor
  if (family === "shop") {
    for (let b = 0; b < s.w; b++) {
      if (b === doorBay) continue;
      windows.push({ part: k.shopfront, kind: "shopfront", x: b * T, y: H - 18, use });
    }
    doors.push({ part: k.doors.shop, kind: "shop", x: doorBay * T + 3, y: H - 21 });
  } else if (family === "civic") {
    // A recessed portico: shade behind columns between the bays.
    fill(img, 0, groundTop + 1, W, GROUND - 3, BSLOT.wallS);
    for (let b = 0; b < s.w; b++) {
      if (b === doorBay) continue;
      windows.push({ part: k.windows.ground, kind: "ground", x: b * T + 3, y: groundTop + 5, use });
    }
    doors.push({ part: k.doors.civic, kind: "civic", x: doorBay * T + 2, y: H - 23 });
  } else if (family === "industrial") {
    for (let b = 0; b < s.w; b++) {
      if (b === doorBay || b === doorBay + 1) continue;
      windows.push({ part: k.windows.sash, kind: "sash", x: b * T + 4, y: groundTop + 5, use });
    }
    doors.push({ part: k.doors.shutter, kind: "shutter", x: doorBay * T + 1, y: H - 20 });
  } else {
    for (let b = 0; b < s.w; b++) {
      if (b === doorBay) continue;
      windows.push({ part: k.windows.ground, kind: "ground", x: b * T + 3, y: groundTop + 5, use });
    }
    doors.push({ part: k.doors.panel, kind: "panel", x: doorBay * T + 3, y: H - 21 });
  }

  for (const w of windows) {
    blit(img, w.part, w.x, w.y);
    if (boarded && w.y >= groundTop - 1) blit(img, crop(k.planks, 0, 0, Math.min(k.planks.w, w.part.w - 2), Math.min(k.planks.h, w.part.h - 4)), w.x + 1, w.y + 2);
  }
  for (const d of doors) blit(img, d.part, d.x, d.y);

  if (family === "shop") {
    tileFill(img, k.fascia, 0, groundTop, W, 5);
    tileFill(img, k.awning, 0, groundTop + 5, W, 6, (seed >>> 2) & 7, 0);
  }
  if (family === "civic") {
    // Columns between the bays, clear of the door.
    for (let b = 1; b < s.w; b++) {
      const cx = b * T - 2;
      if (cx + 4 > doorBay * T && cx < (doorBay + 1) * T) continue;
      blit(img, k.column.capital, cx, groundTop + 1);
      for (let y = groundTop + 4; y < H - 2; y++) blit(img, k.column.shaft, cx, y);
      blit(img, k.column.base, cx, H - 2);
    }
    // A pediment over the middle of the front, standing on the parapet.
    if (s.w >= 4) {
      const px = Math.floor(W / 2) - 16;
      const py = Math.max(0, wallTop - 8);
      blit(img, k.pediment, px, py);
      blit(img, mirrorX(k.pediment), px + 16, py, { map: SHADE_TRIM });
    }
  }

  // ---- roof pieces
  if (s.kind === "town_hall" || (family === "civic" && s.kind !== "council_chamber" && s.kind !== "parliament" && s.kind !== "regional_assembly")) {
    const tx = Math.floor(W / 2) - 7;
    blit(img, k.turret, tx, Math.max(-6, wallTop - 8 - k.turret.h + 2), { map: TURRET_MAP });
  } else if (family === "civic") {
    const dx = Math.floor(W / 2) - 12;
    blit(img, k.dome, dx, wallTop - 8 - k.dome.h + 2, { map: DOME_MAP });
  } else if (flat) {
    const n = Math.max(1, Math.floor(s.w / 2));
    for (let i = 0; i < n; i++) {
      const v = hash32(seed, "clutter", i);
      const part = v % 3 === 0 ? k.skylight : k.vent;
      const x = 3 + (v % Math.max(1, W - part.w - 6));
      const y = 3 + ((v >>> 8) % Math.max(1, wallTop - part.h - 6));
      if (wallTop - part.h - 4 > 2) blit(img, part, x, y);
    }
  } else if (wallTop > 14) {
    const n = 1 + (hash32(seed, "chimneys") % 2);
    for (let i = 0; i < n && i < s.w - 1; i++) {
      const bay = 1 + (hash32(seed, "chimney", i) % Math.max(1, s.w - 1));
      blit(img, k.chimney, bay * T - 3, Math.max(0, Math.floor(wallTop * 0.25) - 2), { map: STACK_MAP });
    }
  }

  if (signed) {
    blit(img, k.board, signX, signY);
    const key = s.home ? "home" : s.kind in k.pictos ? s.kind : family === "civic" ? (s.kind === "town_hall" ? "civic" : "dome") : family === "shop" ? "shop" : "home";
    blit(img, k.pictos[key] as Indexed, signX + 2, signY + 2);
  }

  // ---- recolour; openings with their lit images
  const albedo = remap(img, m.table);
  const outW: Opening[] = windows
    .filter((w) => !(boarded && w.y >= groundTop - 1))
    .map((w, i) => {
      const h = hash32(seed, "lit", i);
      // Not every room is in use: about a quarter of the windows above the street stay dark.
      const dark = w.kind !== "shopfront" && family !== "civic" && h % 100 < 25;
      return { x: w.x, y: w.y, w: w.part.w, h: w.part.h, emissive: dark ? null : remap(litPart(w.part, w.kind, (h >>> 8) % 3), litTable(m.table, w.use, p)), use: w.use };
    });
  const outD: Opening[] = doors.map((d) => ({
    x: d.x,
    y: d.y,
    w: d.part.w,
    h: d.part.h,
    emissive: d.kind === "shutter" ? null : remap(litPart(d.part, d.kind, 0), litTable(m.table, use, p)),
    use,
  }));
  return {
    albedo,
    windows: outW,
    doors: outD,
    shadow: {
      footprint: [
        [0, 0],
        [W, 0],
        [W, s.h * T],
        [0, s.h * T],
      ],
      height: Math.round(wallH * 0.85),
    },
  };
}

/** The right half of a pediment: the same drawing mirrored, its light trim in shade. */
const SHADE_TRIM = (() => {
  const m = new Uint8Array(BSLOT_COUNT);
  for (let i = 0; i < BSLOT_COUNT; i++) m[i] = i;
  m[BSLOT.trimL] = BSLOT.trim;
  m[BSLOT.wall] = BSLOT.wallS;
  return m;
})();

/** The turret's cap is always slate, and the dome copper green: their roof slots point at their own. */
const redirect = (to: readonly [number, number, number, number]) => {
  const m = new Uint8Array(BSLOT_COUNT);
  for (let i = 0; i < BSLOT_COUNT; i++) m[i] = i;
  [m[BSLOT.roof], m[BSLOT.roofS], m[BSLOT.roofD], m[BSLOT.roofL]] = to;
  return m;
};
const TURRET_MAP = redirect([BSLOT.cap, BSLOT.capS, BSLOT.capD, BSLOT.capL]);
const DOME_MAP = redirect([BSLOT.dome, BSLOT.domeS, BSLOT.domeD, BSLOT.domeL]);
/** The slot table a building would use (for the kit's parts sheet): its materials, with lit glass in the warm light. */
export function kitTable(s: FacadeSpec, p: Palette): Uint8Array {
  return litTable(materials(s, familyOf(s.kind, s.landmark), p).table, "home", p);
}

const STACK_MAP = (() => {
  const m = new Uint8Array(BSLOT_COUNT);
  for (let i = 0; i < BSLOT_COUNT; i++) m[i] = i;
  [m[BSLOT.wall], m[BSLOT.wallS], m[BSLOT.wallD], m[BSLOT.wallL]] = [BSLOT.stack, BSLOT.stackS, BSLOT.stackD, BSLOT.stackL];
  return m;
})();
