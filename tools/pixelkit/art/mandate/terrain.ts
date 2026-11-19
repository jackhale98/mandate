// Town layout v2 terrain (DEC-090; TOWN_GENERATION.md §7.7; PIXEL_ART_TOOL.md
// "Round 4"): the ground of a cell drawn from what the map says about its
// neighbours, in the flat three-quarter style of rounds 1–3.
//   - water edges by what the shore is: a soft earth bank (with reeds beside
//     reed beds), a beach, a rock edge, a quay or a wall's dark foot; each
//     quarter of a water tile takes the edge of the neighbour it touches;
//   - a one-tile creek (its own channel set from the four sides it joins), a
//     ford (shallow, a track across, pebbles), stepping stones (a prop);
//   - levels (0–3, about a storey each): a level change facing south is a
//     visible face 8–16 px tall drawn at the top of the lower cell (an earth
//     bank under grass, a retaining wall under paving, rock under rock); cells
//     of `Embankment` or `Rock` below a change are whole faces, so walls and
//     cliffs run 1–3 tiles tall; a north-facing change is only a lip line;
//     each two levels up lift the ground one tone step; no gradients or hillshade;
//   - rail: a track autotile with 45° pieces, drawn in map pixels so diagonal
//     track crosses cell corners without seams; platforms beside it; a level
//     crossing where it meets a road;
//   - bridges from connectors: a kit per kind (stone road bridge, timber
//     footbridge, brick viaduct) in both axes: abutments, spans, parapets and
//     piers; running east–west the south face and its arches show, running
//     north–south the deck, its parapets, cutwaters and a shadow on the water.
// Everything here is integer pixel rules over palette indices (the runtime
// does no colour maths), deterministic for the map: no clock, no randomness.
import { NB, autotile, canonicalMask, quarterKind, templateQuarter } from "../../src/core/autotile";
import { edgeTemplate, type EdgeProfile } from "../../src/core/edges";
import { hash32 } from "../../src/core/hash";
import { blank, blit, crop, grid, type Indexed } from "../../src/core/image";
import type { Palette } from "../../src/core/palette";
import { DiagonalStreets, type StreetRules } from "./streets";
import { groundTile, legendFor, TILE_LEGEND_RAMPS, type TileSet } from "./tiles";

/** The layout's terrain codes (web/src/hub/townLayout.ts `Terrain`; a web test checks they agree). */
export const TERRAIN = {
  Grass: 0,
  Road: 1,
  Pavement: 2,
  Crossing: 3,
  Plaza: 4,
  Water: 5,
  Building: 6,
  Door: 7,
  Park: 8,
  Path: 9,
  Yard: 10,
  Field: 11,
  Dirt: 12,
  Sand: 13,
  Quay: 14,
  Pier: 15,
  Garden: 16,
  Creek: 17,
  Rock: 18,
  Embankment: 19,
  Rail: 20,
  Reeds: 21,
  Bridge: 22,
  Ford: 23,
  Pit: 24,
  Steps: 25,
} as const;
const K = TERRAIN;

/** v1 codes to the tile set's terrain names (the v2 codes are drawn here). */
export const V1_NAMES: Readonly<Record<number, string>> = {
  [K.Grass]: "grass",
  [K.Road]: "road",
  [K.Pavement]: "pavement",
  [K.Crossing]: "crossing",
  [K.Plaza]: "plaza",
  [K.Water]: "water",
  [K.Building]: "pavement",
  [K.Door]: "pavement",
  [K.Park]: "park",
  [K.Path]: "path",
  [K.Yard]: "yard",
  [K.Field]: "field",
  [K.Dirt]: "dirt",
  [K.Sand]: "sand",
  [K.Quay]: "quay",
  [K.Pier]: "pier",
  [K.Garden]: "garden",
};

/** The v2 terrains this module draws (the coverage lint walks them). */
export const V2_CODES = [K.Creek, K.Rock, K.Embankment, K.Rail, K.Reeds, K.Bridge, K.Ford, K.Pit, K.Steps] as const;
export const V2_NAMES: Readonly<Record<number, string>> = { [K.Creek]: "creek", [K.Rock]: "rock", [K.Embankment]: "embankment", [K.Rail]: "rail", [K.Reeds]: "reeds", [K.Bridge]: "bridge", [K.Ford]: "ford", [K.Pit]: "pit", [K.Steps]: "steps" };

export type ConnectorKind = "bridge" | "footbridge" | "ford" | "viaduct" | "level_crossing";
export const CONNECTOR_KINDS: readonly ConnectorKind[] = ["bridge", "footbridge", "ford", "viaduct", "level_crossing"];

/** A connector as the layout gives it (`TownConnector`): covered cells, and the way it runs. */
export interface ConnectorView {
  kind: string;
  x: number;
  y: number;
  w: number;
  h: number;
  axis: "ew" | "ns";
}

/** What the painter reads from a town map (`TownMap` has this shape). */
export interface TerrainView {
  width: number;
  height: number;
  terrain: ArrayLike<number>;
  elevation?: ArrayLike<number> | undefined;
  connectors?: readonly ConnectorView[] | undefined;
  /** Layout v2 street axis per cell (`TownMap.streetAxis`, `STREET_AXIS` codes): diagonal streets (round 5). */
  streetAxis?: ArrayLike<number> | undefined;
}

// ---------------------------------------------------------------------------
// Built ahead of time: legend colours, fills and shore templates, step tables

export type Shore = "soft" | "reeds" | "sand" | "rock" | "quay" | "wall";
export const SHORES: readonly Shore[] = ["soft", "reeds", "sand", "rock", "quay", "wall"];
/** When a water corner touches two kinds of shore, the harder one draws it. */
const SHORE_RANK: Readonly<Record<Shore, number>> = { wall: 5, quay: 4, rock: 3, sand: 2, reeds: 1, soft: 0 };

/** Extra legend for v2 drawing (on top of the tile legend). */
const V2_LEGEND: Readonly<Record<string, readonly [string, number]>> = {
  I: ["iron", 1],
  K: ["brick", 0],
  B: ["brick", -1],
  X: ["brick", -2],
  L: ["brick", 1],
  o: ["ink", 0],
  O: ["ink", -1],
  Y: ["lamp", 0],
  R: ["flag", 0],
  u: ["slate", 0],
  U: ["slate", -1],
  z: ["water", -1],
};

export interface TerrainSet {
  /** Legend character → palette index (the tile legend and the v2 extras). */
  legend: Record<string, number>;
  /** Water's autotile template per kind of shore (32×48). */
  shores: Record<Shore, Indexed>;
  /** Autotiled v2 grounds: template and fills (rock tops, pit floors, reed beds, shallow water). */
  grounds: Record<"rock" | "pit" | "reeds" | "shallow", { template: Indexed; fills: Indexed[] }>;
  /** One step darker and lighter within each colour's ramp (lips, shade on water, tone steps). */
  darker: Uint8Array;
  lighter: Uint8Array;
}

/** Edge profiles of water per shore, outside in (legend characters). */
const SHORE_EDGES: Readonly<Record<Shore, { N: string; S: string; W: string; E: string }>> = {
  // A bank: the north bank's earth face, then the water in its shade; a light lap on the south shore.
  soft: { N: "eeW", S: "v", W: "ev", E: "ev" },
  reeds: { N: "neW", S: "nv", W: "nv", E: "nv" },
  // A beach: wet sand and a line of foam.
  sand: { N: "scv", S: "cv", W: "cv", E: "cv" },
  // Rock: a grey edge and dark water under it.
  rock: { N: "PQW", S: "Pv", W: "Pv", E: "Pv" },
  // A quay or paved edge (round 1's water edges).
  quay: { N: "WW", S: "v", W: "v", E: "v" },
  // The foot of a wall: deep shade.
  wall: { N: "AzW", S: "Pv", W: "Pv", E: "Pv" },
};

/** A seamless 16×16 fill of cells (Worley-style slabs on a torus): `n` seeds, each slab a colour, cracks where slabs meet. */
function slabFill(seed: number, n: number, face: (slab: number) => string, crack: string, lit: string, shade: string, legend: Record<string, number>): Indexed {
  const pts = Array.from({ length: n }, (_, i) => [hash32(seed, i, 1) % 16, hash32(seed, i, 2) % 16] as const);
  const owner = new Uint8Array(256);
  for (let y = 0; y < 16; y++) {
    for (let x = 0; x < 16; x++) {
      let best = 0;
      let bd = 1e9;
      pts.forEach(([px, py], i) => {
        const dx = Math.min(Math.abs(px - x), 16 - Math.abs(px - x));
        const dy = Math.min(Math.abs(py - y), 16 - Math.abs(py - y));
        const d = dx * dx + dy * dy * 2;
        if (d < bd) {
          bd = d;
          best = i;
        }
      });
      owner[y * 16 + x] = best;
    }
  }
  const at = (x: number, y: number) => owner[(((y + 16) % 16) * 16 + ((x + 16) % 16)) | 0] as number;
  const img = blank(16, 16);
  for (let y = 0; y < 16; y++) {
    for (let x = 0; x < 16; x++) {
      const o = at(x, y);
      let ch = face(o);
      // A crack where the slab below or to the right is another: cracks are drawn once, on the upper-left slab's edge.
      if (at(x + 1, y) !== o || at(x, y + 1) !== o) ch = crack;
      // Light catches the top-left edge of a slab, shade the bottom-right inside a crack.
      else if (at(x - 1, y) !== o || at(x, y - 1) !== o) ch = lit;
      else if (at(x + 2, y) !== o || at(x, y + 2) !== o) ch = shade;
      img.px[y * 16 + x] = legend[ch] as number;
    }
  }
  return img;
}

/**
 * A reed bed (seamless): damp dark grass, and clumps of three to five stems
 * 5–9 px tall in golden and green, leaning a little, some with a dark seed
 * head; clumps wrap round the tile's edges so beds join without seams.
 */
function reedFill(legend: Record<string, number>, seed: number): Indexed {
  const img = blank(16, 16);
  const put = (x: number, y: number, ch: string) => {
    img.px[(((y % 16) + 16) % 16) * 16 + (((x % 16) + 16) % 16)] = legend[ch] as number;
  };
  for (let y = 0; y < 16; y++) for (let x = 0; x < 16; x++) put(x, y, (hash32(seed, x >> 1, y >> 1) & 3) === 0 ? "G" : "N");
  for (let c = 0; c < 6; c++) {
    const cx = hash32(seed, c, 1) % 16;
    const cy = hash32(seed, c, 2) % 16;
    const n = 3 + (hash32(seed, c, 3) % 3);
    for (let k = 0; k < n; k++) {
      const sx = cx + k - (n >> 1);
      const tall = 5 + (hash32(seed, c, k, 4) % 5);
      const lean = (k - (n >> 1)) * (hash32(seed, c, 5) % 2 ? 1 : 0);
      const col = (k + c) % 3 === 0 ? "n" : (k + c) % 3 === 1 ? "f" : "F";
      for (let s2 = 0; s2 < tall; s2++) put(sx + (s2 > tall / 2 ? lean : 0), cy - s2, s2 === 0 ? "N" : col);
      if (hash32(seed, c, k, 6) % 3 === 0) {
        put(sx + lean, cy - tall, "D");
        put(sx + lean, cy - tall - 1, "D");
      }
    }
  }
  return img;
}

/** Shallow water (fords): light water over a bed of pebbles. */
const SHALLOW = `
  vvvvvvvvvvvvvvvv
  vvwvvvvvvvvvvvvv
  vvvvvvqpvvvvvvvv
  vvvvvvppvvvvvwvv
  vvvvvvvvvvvvvvvv
  vvvvvvvvvvvvvvvv
  vqpvvvvvvvvvvvvv
  vppvvvvvwvvvvvvv
  vvvvvvvvvvvvvvvv
  vvvvvvvvvvvqpvvv
  vvvwvvvvvvvppvvv
  vvvvvvvvvvvvvvvv
  vvvvvvvvvvvvvvvv
  vvvvvvqpvvvvvvvv
  vvvvvvppvvvvvvvw
  vvvvvvvvvvvvvvvv`;

export function buildTerrain(p: Palette): TerrainSet {
  const legend = { ...legendFor(p, TILE_LEGEND_RAMPS), ...legendFor(p, V2_LEGEND) };
  const prof = (s: string) => [...s].map((ch) => legend[ch] as number);
  const edges = (e: { N: string; S: string; W: string; E: string }): EdgeProfile => ({ N: prof(e.N), S: prof(e.S), W: prof(e.W), E: prof(e.E) });
  const water = grid(
    `
    wwwwwwwwwwwwwwww
    wwwwwwwwwwwwwwww
    wwwvvvwwwwwwwwww
    wwwwwwwwwwwwwwww
    wwwwwwwwwwWWWwww
    wwwwwwwwwwwwwwww
    wwwwwwwwwwwwwwww
    wwwwwwwwwwwwwwww
    wwwwwwwwvvvwwwww
    wwwwwwwwwwwwwwww
    wWWWwwwwwwwwwwww
    wwwwwwwwwwwwwwww
    wwwwwwwwwwwwwwww
    wwwwwwwwwwwwvvvw
    wwwwwwwwwwwwwwww
    wwwwwwwwwwwwwwww`,
    legend,
  );
  const shores = Object.fromEntries(SHORES.map((s) => [s, edgeTemplate(water, edges(SHORE_EDGES[s]))])) as Record<Shore, Indexed>;
  // Rock tops: grey slabs with cracks; pit floors: darker rubble; two variants each.
  const rock = [0, 1].map((v) => slabFill(0x70c + v, 6, (o) => (o % 3 === 0 ? "p" : "P"), "Q", "q", "P", legend));
  const pit = [0, 1].map((v) => slabFill(0x917 + v, 9, (o) => (o % 3 === 0 ? "e" : o % 3 === 1 ? "P" : "e"), "Q", "E", "P", legend));
  const reeds = [reedFill(legend, 0x2ee), reedFill(legend, 0x2ef)];
  const shallow = [grid(SHALLOW, legend)];
  const grounds: TerrainSet["grounds"] = {
    rock: { template: edgeTemplate(rock[0] as Indexed, edges({ N: "q", S: "QP", W: "q", E: "Q" })), fills: rock },
    pit: { template: edgeTemplate(pit[0] as Indexed, edges({ N: "A", S: "Q", W: "Q", E: "Q" })), fills: pit },
    reeds: { template: edgeTemplate(reeds[0] as Indexed, edges({ N: "G", S: "HG", W: "G", E: "G" })), fills: reeds },
    shallow: { template: edgeTemplate(shallow[0] as Indexed, edges({ N: "eW", S: "w", W: "w", E: "w" })), fills: shallow },
  };
  const n = p.colors.length;
  const darker = new Uint8Array(256);
  const lighter = new Uint8Array(256);
  for (let i = 0; i < 256; i++) {
    darker[i] = i;
    lighter[i] = i;
  }
  for (let i = 1; i < n; i++) {
    const r = p.ramps[p.rampOf[i] as number];
    const k = p.stepOf[i] as number;
    if (!r || k < 0) continue;
    darker[i] = r.steps[Math.max(0, k - 1)] as number;
    lighter[i] = r.steps[Math.min(r.steps.length - 1, k + 1)] as number;
  }
  return { legend, shores, grounds, darker, lighter };
}

// ---------------------------------------------------------------------------
// Classifying terrain

const PAVED = new Set<number>([K.Road, K.Pavement, K.Crossing, K.Plaza, K.Yard, K.Quay, K.Building, K.Door, K.Bridge, K.Steps, K.Pier]);
const WATERY = new Set<number>([K.Water, K.Pier, K.Creek, K.Ford]);

function shoreOf(t: number): Shore {
  switch (t) {
    case K.Reeds:
      return "reeds";
    case K.Sand:
      return "sand";
    case K.Rock:
    case K.Pit:
    case K.Rail:
      return "rock";
    case K.Embankment:
      return "wall";
    case K.Quay:
    case K.Pavement:
    case K.Plaza:
    case K.Road:
    case K.Crossing:
    case K.Building:
    case K.Door:
    case K.Yard:
    case K.Steps:
      return "quay";
    default:
      return "soft";
  }
}

/**
 * The autotile group of a terrain (as the renderer's `terrainGroup`, plus the
 * v2 codes). Rail, creek and ford cells stand on grass (their own drawing
 * covers the middle of the cell), so grass beside them draws no edge and a
 * diagonal line does not leave a staircase of tile edges.
 */
function group(t: number): number {
  switch (t) {
    case K.Rail:
    case K.Creek:
    case K.Ford:
      return K.Grass;
    case K.Road:
    case K.Crossing:
      return K.Road;
    case K.Pavement:
    case K.Door:
    case K.Building:
      return K.Pavement;
    case K.Water:
    case K.Pier:
      return K.Water;
    default:
      return t;
  }
}

/** Terrains a diagonal street smooths (round 5): streets, squares, greens, yards and open ground. */
const STREET_SOFT = new Set<number>([K.Grass, K.Road, K.Pavement, K.Crossing, K.Plaza, K.Park, K.Path, K.Yard, K.Dirt, K.Garden, K.Field]);
const STREET_RANK: Readonly<Record<number, number>> = { [K.Road]: 5, [K.Pavement]: 4, [K.Plaza]: 3, [K.Yard]: 2, [K.Path]: 2 };
const STREET_RULES: StreetRules = {
  group,
  soft: STREET_SOFT,
  joins: new Set<number>([...STREET_SOFT, K.Building, K.Door]),
  tileName: (t) => V1_NAMES[t] ?? "grass",
  materialName: (g) => V1_NAMES[g] ?? "grass",
  rank: (g) => STREET_RANK[g] ?? 1,
  roadGroup: K.Road,
  road: K.Road,
  crossing: K.Crossing,
};

const DIRS: readonly (readonly [number, number, number])[] = [
  [0, -1, NB.N],
  [1, 0, NB.E],
  [0, 1, NB.S],
  [-1, 0, NB.W],
  [1, -1, NB.NE],
  [1, 1, NB.SE],
  [-1, 1, NB.SW],
  [-1, -1, NB.NW],
];

interface Deck {
  c: ConnectorView;
  kind: ConnectorKind;
}

const known = (k: string): ConnectorKind => (CONNECTOR_KINDS.includes(k as ConnectorKind) ? (k as ConnectorKind) : "bridge");

// ---------------------------------------------------------------------------
// The painter

/** Face materials: an earth bank, a stone retaining wall, a rock face. */
type FaceKind = "bank" | "wall" | "rock";

/**
 * Draws v2 ground for one map. Construct once per map (it indexes the
 * connectors and rail links), then ask for cells; `simple(x, y)` says when a
 * cell looks exactly as layout v1 would draw it (the caller can use its
 * ordinary tile).
 */
export class TerrainPainter {
  readonly v: TerrainView;
  private readonly ts: TileSet;
  private readonly t2: TerrainSet;
  private readonly L: Record<string, number>;
  private readonly W: number;
  private readonly H: number;
  /** Per cell: the connector deck covering it, if any. */
  private readonly deckAt: (Deck | null)[];
  private readonly decks: Deck[];
  private readonly railMask: Uint8Array;
  /** Cells something v2 reaches (bridge faces and shadows, rail corners): never "simple". */
  private readonly touched: Uint8Array;
  /** Diagonal streets (round 5), when the map marks any. */
  private readonly streets: DiagonalStreets | null;

  constructor(ts: TileSet, t2: TerrainSet, view: TerrainView) {
    this.ts = ts;
    this.t2 = t2;
    this.v = view;
    this.L = t2.legend;
    this.W = view.width;
    this.H = view.height;
    const n = this.W * this.H;
    this.deckAt = new Array<Deck | null>(n).fill(null);
    this.touched = new Uint8Array(n);
    this.decks = [];
    const add = (c: ConnectorView) => {
      const d: Deck = { c, kind: known(c.kind) };
      this.decks.push(d);
      for (let y = c.y; y < c.y + c.h; y++) for (let x = c.x; x < c.x + c.w; x++) if (this.inside(x, y)) this.deckAt[y * this.W + x] = d;
      // What a deck reaches beyond itself: the face below (ew), the shadow and cutwaters beside it (ns), the ends.
      for (let y = c.y - 1; y < c.y + c.h + (c.axis === "ew" ? 3 : 1); y++) for (let x = c.x - 1; x <= c.x + c.w; x++) if (this.inside(x, y)) this.touched[y * this.W + x] = 1;
    };
    for (const c of view.connectors ?? []) add(c);
    // Bridge cells no connector covers (a generator that leaves connectors out): one stone bridge per run of them.
    for (let y = 0; y < this.H; y++) {
      for (let x = 0; x < this.W; x++) {
        const i = y * this.W + x;
        if (this.t(x, y) !== K.Bridge || this.deckAt[i]) continue;
        // Bounded by the map: `t()` clamps to the edge, so a run reaching the edge would never end.
        let w = 1;
        while (x + w < this.W && this.t(x + w, y) === K.Bridge && !this.deckAt[i + w]) w++;
        let h = 1;
        while (y + h < this.H && this.t(x, y + h) === K.Bridge && !this.deckAt[i + h * this.W]) h++;
        const ew = w >= h;
        add({ kind: "bridge", x, y, w: ew ? w : 1, h: ew ? 1 : h, axis: ew ? "ew" : "ns" });
      }
    }
    // Rail: which cells carry track, and their links (orthogonal; diagonal where no orthogonal path turns the corner).
    this.railMask = new Uint8Array(n);
    for (let y = 0; y < this.H; y++) {
      for (let x = 0; x < this.W; x++) {
        if (!this.isRail(x, y)) continue;
        let m = 0;
        for (const [dx, dy, bit] of DIRS) {
          if (!this.isRail(x + dx, y + dy)) continue;
          if (dx !== 0 && dy !== 0 && (this.isRail(x + dx, y) || this.isRail(x, y + dy))) continue;
          m |= bit;
        }
        this.railMask[y * this.W + x] = m;
        for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) if (this.inside(x + dx, y + dy)) this.touched[(y + dy) * this.W + x + dx] = 1;
      }
    }
    // Diagonal streets: only when the map marks a street axis other than orthogonal.
    const axis = view.streetAxis;
    const streets = axis
      ? new DiagonalStreets(
          {
            width: this.W,
            height: this.H,
            t: (x, y) => this.t(x, y),
            e: (x, y) => this.e(x, y),
            axis: (x, y) => (this.inside(x, y) ? ((axis[y * this.W + x] as number | undefined) ?? 0) : 0),
            levelCrossing: (x, y) => this.t(x, y) === K.Crossing && this.isRail(x, y),
          },
          STREET_RULES,
          ts,
          this.L,
        )
      : null;
    this.streets = streets?.active ? streets : null;
  }

  private inside(x: number, y: number) {
    return x >= 0 && y >= 0 && x < this.W && y < this.H;
  }

  /** Terrain at a cell; off the map, the nearest cell's (so map edges draw no seams). */
  t(x: number, y: number): number {
    const cx = Math.max(0, Math.min(this.W - 1, x));
    const cy = Math.max(0, Math.min(this.H - 1, y));
    return (this.v.terrain[cy * this.W + cx] as number | undefined) ?? K.Grass;
  }

  /** Ground level at a cell (0–3); off the map, the nearest cell's. */
  e(x: number, y: number): number {
    const el = this.v.elevation;
    if (!el) return 0;
    const cx = Math.max(0, Math.min(this.W - 1, x));
    const cy = Math.max(0, Math.min(this.H - 1, y));
    return Math.min(3, (el[cy * this.W + cx] as number | undefined) ?? 0);
  }

  private deck(x: number, y: number): Deck | null {
    return this.inside(x, y) ? (this.deckAt[y * this.W + x] ?? null) : null;
  }

  /** Track runs here: rail, a viaduct's deck, a level crossing. */
  isRail(x: number, y: number): boolean {
    if (!this.inside(x, y)) return false;
    const t = this.t(x, y);
    if (t === K.Rail) return true;
    const d = this.deck(x, y);
    if (d && (d.kind === "viaduct" || d.kind === "level_crossing")) return true;
    // A crossing with rail on both sides of it (a level crossing no connector names).
    if (t === K.Crossing) {
      const r = (dx: number, dy: number) => this.t(x + dx, y + dy) === K.Rail;
      return (r(-1, 0) && r(1, 0)) || (r(0, -1) && r(0, 1));
    }
    return false;
  }

  /** Whether the cell looks exactly as v1 draws it: a v1 terrain, flat, nothing v2 around it. */
  simple(x: number, y: number): boolean {
    const t = this.t(x, y);
    if (t > K.Garden || t === K.Water || t === K.Pier || this.touched[y * this.W + x] || this.streets?.touched(x, y)) return false;
    const e0 = this.e(x, y);
    if (e0 >= 2) return false;
    for (let dy = -1; dy <= 1; dy++) {
      for (let dx = -1; dx <= 1; dx++) {
        if (this.t(x + dx, y + dy) > K.Garden || this.e(x + dx, y + dy) !== e0) return false;
      }
    }
    return true;
  }

  private mask(x: number, y: number, same: (t: number, x: number, y: number) => boolean): number {
    let m = 0;
    for (const [dx, dy, bit] of DIRS) {
      const nx = x + dx;
      const ny = y + dy;
      if (!this.inside(nx, ny) || same(this.t(nx, ny), nx, ny)) m |= bit;
    }
    return m;
  }

  /** The v1 neighbour mask (the renderer's `groundCells`), for an ordinary tile. */
  v1Mask(x: number, y: number): number {
    const g = group(this.t(x, y));
    return this.mask(x, y, (t) => group(t) === g);
  }

  // ------------------------------------------------------------ drawing a cell

  /** The cell's image (16×16 palette indices). `variant` 0..7 as the renderer's `groundCells` gives it. */
  draw(x: number, y: number, variant: number): Indexed {
    const t = this.t(x, y);
    const img = this.base(x, y, t, variant);
    const e0 = this.e(x, y);
    if (e0 >= 2 && !WATERY.has(t)) this.remapAll(img, this.t2.lighter);
    this.levels(img, x, y, t);
    if (this.isRail(x, y) || this.nearRail(x, y)) this.rail(img, x, y);
    this.platform(img, x, y, t);
    this.bridgeAround(img, x, y);
    return img;
  }

  private remapAll(img: Indexed, table: Uint8Array) {
    for (let i = 0; i < img.px.length; i++) img.px[i] = table[img.px[i] as number] as number;
  }

  private tile(name: string, mask: number, variant: number): Indexed {
    return groundTile(this.ts, name, mask, variant);
  }

  private ground2(name: keyof TerrainSet["grounds"], x: number, y: number, variant: number, same: (t: number) => boolean): Indexed {
    const g = this.t2.grounds[name];
    const fill = g.fills.length > 1 && variant >> 1 === 3 ? g.fills[1] : g.fills[0];
    return autotile(g.template, canonicalMask(this.mask(x, y, same)), fill);
  }

  /** The cell's own ground, before levels, rail and bridges. */
  private base(x: number, y: number, t: number, variant: number): Indexed {
    if (this.streets?.draws(x, y)) {
      const img = this.streets.draw(x, y, variant);
      return t === K.Crossing && this.isRail(x, y) ? this.levelCrossing(x, y, img) : img;
    }
    switch (t) {
      case K.Water:
      case K.Pier:
        return t === K.Pier ? this.tile("pier", this.v1Mask(x, y), variant) : this.water(x, y, variant);
      case K.Creek:
        return this.creek(x, y, variant, false);
      case K.Ford:
        return this.ford(x, y, variant);
      case K.Rock:
        return this.faceCell(x, y) ?? this.ground2("rock", x, y, variant, (n) => n === K.Rock);
      case K.Pit:
        return this.ground2("pit", x, y, variant, (n) => n === K.Pit);
      case K.Reeds:
        return this.ground2("reeds", x, y, variant, (n) => n === K.Reeds);
      case K.Embankment:
        return this.faceCell(x, y) ?? this.wallCell(x, y);
      case K.Rail:
        return this.tile("grass", 255, variant);
      case K.Steps:
        return this.steps(x, y);
      case K.Bridge:
        return this.deckCell(x, y, variant);
      case K.Crossing:
        if (this.isRail(x, y)) return this.levelCrossing(x, y);
        return this.tile("crossing", this.v1Mask(x, y), variant);
      default: {
        const name = V1_NAMES[t];
        return name ? this.tile(name, this.v1Mask(x, y), variant) : this.tile("grass", 255, variant);
      }
    }
  }

  // ------------------------------------------------------------ water

  /** Water: each quarter takes the edge of the shore it touches (a bank, a beach, rock, a quay, a wall's foot). */
  private water(x: number, y: number, _variant: number): Indexed {
    const sameW = (n: number) => WATERY.has(n) || n === K.Bridge;
    const m = canonicalMask(this.mask(x, y, sameW));
    const out = blank(16, 16);
    const fill = (this.ts.terrains.water?.fills[0] ?? blank(16, 16)) as Indexed;
    const shoreAt = (dx: number, dy: number): Shore => shoreOf(this.t(x + dx, y + dy));
    for (const qy of [0, 1] as const) {
      for (const qx of [0, 1] as const) {
        const kind = quarterKind(m, qx, qy);
        if (kind === "fill") {
          blit(out, crop(fill, qx * 8, qy * 8, 8, 8), qx * 8, qy * 8);
          continue;
        }
        const sx = qx === 0 ? -1 : 1;
        const sy = qy === 0 ? -1 : 1;
        let shore: Shore;
        if (kind === "edgeH") shore = shoreAt(0, sy);
        else if (kind === "edgeV") shore = shoreAt(sx, 0);
        else if (kind === "inner") shore = shoreAt(sx, sy);
        else {
          const a = shoreAt(0, sy);
          const b = shoreAt(sx, 0);
          shore = SHORE_RANK[a] >= SHORE_RANK[b] ? a : b;
        }
        const [tx, ty] = templateQuarter(kind, qx, qy);
        blit(out, crop(this.t2.shores[shore], tx * 8, ty * 8, 8, 8), qx * 8, qy * 8);
      }
    }
    // Reeds stand in the water along a reed bed: stems of 2–4 px, by hash, within 3 px of that shore.
    const L = this.L;
    for (const [dx, dy] of [
      [0, -1],
      [1, 0],
      [0, 1],
      [-1, 0],
    ] as const) {
      if (this.t(x + dx, y + dy) !== K.Reeds) continue;
      for (let k = 0; k < 16; k += 3) {
        const h = hash32(x * 16 + k, y, dx * 3 + dy, 0x2eed);
        if (h % 3 === 0) continue;
        const along = k + (h & 1);
        const depth = 1 + ((h >>> 3) % 3);
        const tall = 2 + ((h >>> 5) % 3);
        for (let s = 0; s < tall; s++) {
          let px: number;
          let py: number;
          if (dy !== 0) {
            px = along;
            py = dy < 0 ? depth + 2 - s : 15 - depth - s;
          } else {
            px = dx < 0 ? depth : 15 - depth;
            py = along - s;
          }
          if (px < 0 || py < 0 || px > 15 || py > 15) continue;
          out.px[py * 16 + px] = s === tall - 1 ? (L.D as number) : s === 0 ? (L.F as number) : (L.f as number);
        }
      }
    }
    return out;
  }

  /** Creek and fords join: creek, ford, water and bridges (a footbridge over a creek). */
  private creekMask(x: number, y: number): number {
    let m = 0;
    for (const [dx, dy, bit] of DIRS.slice(0, 4)) {
      const n = this.t(x + dx, y + dy);
      if (n === K.Creek || n === K.Ford || n === K.Water || n === K.Pier || (n === K.Bridge && this.creekUnder(x + dx, y + dy))) m |= bit;
    }
    return m;
  }

  /** A bridge cell over a creek: creek beyond it in the way it crosses. */
  private creekUnder(x: number, y: number): boolean {
    const c = (dx: number, dy: number) => {
      const n = this.t(x + dx, y + dy);
      return n === K.Creek || n === K.Ford;
    };
    return (c(0, -1) && c(0, 1)) || (c(-1, 0) && c(1, 0));
  }

  /**
   * A one-tile creek: a channel 6 px wide from the middle of the cell to each
   * side it joins (a pool when it joins none), cut into the bank: the north
   * bank's earth face shows, the water darkens under it, a light lap on the
   * south bank, and a light streak in the current. `shallow` draws a ford.
   */
  private creek(x: number, y: number, variant: number, shallow: boolean, maskOverride?: number): Indexed {
    const L = this.L;
    const img = this.tile(this.bankGround(x, y), 255, variant);
    const m = maskOverride ?? this.creekMask(x, y);
    const wet = (px: number, py: number): boolean => {
      if (px < 0 || py < 0 || px > 15 || py > 15) {
        // Beyond the cell: wet where an arm continues into the neighbour.
        if (py < 0) return (m & NB.N) !== 0 && px >= 5 && px <= 10;
        if (py > 15) return (m & NB.S) !== 0 && px >= 5 && px <= 10;
        if (px < 0) return (m & NB.W) !== 0 && py >= 5 && py <= 10;
        return (m & NB.E) !== 0 && py >= 5 && py <= 10;
      }
      const inMid = px >= 5 && px <= 10 && py >= 5 && py <= 10;
      if (m === 0) return px >= 4 && px <= 11 && py >= 5 && py <= 11 && !((px === 4 || px === 11) && (py === 5 || py === 11));
      if (inMid) return true;
      if (m & NB.N && px >= 5 && px <= 10 && py < 5) return true;
      if (m & NB.S && px >= 5 && px <= 10 && py > 10) return true;
      if (m & NB.W && py >= 5 && py <= 10 && px < 5) return true;
      if (m & NB.E && py >= 5 && py <= 10 && px > 10) return true;
      return false;
    };
    const base = shallow ? L.v : L.w;
    const deep = shallow ? L.w : L.W;
    for (let py = 0; py < 16; py++) {
      for (let px = 0; px < 16; px++) {
        const i = py * 16 + px;
        if (wet(px, py)) {
          let c = base as number;
          // The bank's shade on the water just below a north bank.
          if (!wet(px, py - 1)) c = deep as number;
          // A light lap where the water meets the south bank.
          else if (!wet(px, py + 1)) c = L.v as number;
          // Current: a light dash along the middle, every 6 px.
          else if (!shallow && (m & (NB.E | NB.W)) !== 0 && (m & (NB.N | NB.S)) === 0 && py === 7 && (x * 16 + px) % 6 < 2) c = L.v as number;
          else if (!shallow && (m & (NB.N | NB.S)) !== 0 && (m & (NB.E | NB.W)) === 0 && px === 7 && (y * 16 + py) % 6 < 2) c = L.v as number;
          img.px[i] = c;
          if (shallow && (hash32(x * 16 + px, y * 16 + py, 0xf0d) & 15) === 0 && wet(px + 1, py)) {
            img.px[i] = L.q as number;
            if (px < 15) img.px[i + 1] = L.p as number;
          }
        } else if (wet(px, py + 1) || wet(px, py + 2)) {
          // The north bank's face: 2 px of earth above the water.
          img.px[i] = (wet(px, py + 1) ? L.e : L.E) as number;
        } else if (wet(px - 1, py) || wet(px + 1, py)) {
          img.px[i] = (wet(px + 1, py) ? L.e : L.G) as number;
        } else if (wet(px, py - 1)) {
          img.px[i] = L.G as number;
        }
      }
    }
    return img;
  }

  /** What the banks of a creek are: the grass, field or park around it. */
  private bankGround(x: number, y: number): string {
    for (const [dx, dy] of [
      [-1, 0],
      [1, 0],
      [0, -1],
      [0, 1],
    ] as const) {
      const n = V1_NAMES[this.t(x + dx, y + dy)];
      if (n === "park" || n === "field" || n === "garden") return n === "field" ? "grass" : n;
    }
    return "grass";
  }

  /** A ford: on a creek, the creek's channel made shallow with a track across it; across a river, shallow water. */
  private ford(x: number, y: number, variant: number): Indexed {
    const L = this.L;
    const riverSides = [this.t(x - 1, y), this.t(x + 1, y), this.t(x, y - 1), this.t(x, y + 1)].filter((n) => n === K.Water || n === K.Ford).length;
    const creekSides = [this.t(x - 1, y), this.t(x + 1, y), this.t(x, y - 1), this.t(x, y + 1)].filter((n) => n === K.Creek).length;
    if (riverSides >= 2 && creekSides === 0) {
      const img = this.ground2("shallow", x, y, variant, (n) => WATERY.has(n) || n === K.Bridge);
      return img;
    }
    const m = this.creekMask(x, y);
    const img = this.creek(x, y, variant, true, m);
    // The track crosses the channel at right angles: ruts of earth in the shallows.
    const across = (m & (NB.E | NB.W)) !== 0 && (m & (NB.N | NB.S)) === 0 ? "ns" : "ew";
    for (let k = 0; k < 16; k++) {
      for (const r of [5, 10]) {
        const [px, py] = across === "ns" ? [r, k] : [k, r];
        const i = py * 16 + px;
        const c = img.px[i] as number;
        img.px[i] = c === L.v || c === L.w ? (L.E as number) : c === L.e || c === L.G ? (L.e as number) : (L.E as number);
      }
    }
    return img;
  }

  // ------------------------------------------------------------ levels

  /** A face that fills this cell (an embankment wall or a rock face below a level change), or null. */
  private faceCell(x: number, y: number): Indexed | null {
    const t = this.t(x, y);
    const top = this.faceTop(x, y);
    if (top === null) return null;
    const kind: FaceKind = t === K.Embankment ? "wall" : "rock";
    const img = this.tile("grass", 255, 0);
    // The face's bottom: where the cell below is not the same face.
    const bottom = !(this.t(x, y + 1) === t && this.e(x, y + 1) === this.e(x, y));
    this.paintFace(img, x, y, kind, 0, 16, top === y, bottom);
    return img;
  }

  /** The row where this cell's face starts (a level change at or above it, running down through face cells of the same kind), or null. */
  private faceTop(x: number, y: number): number | null {
    const t = this.t(x, y);
    let yy = y;
    for (let k = 0; k < 4; k++) {
      if (this.e(x, yy - 1) > this.e(x, yy) && yy - 1 >= 0) return yy;
      if (this.t(x, yy - 1) === t && this.e(x, yy - 1) === this.e(x, yy) && yy - 1 >= 0) yy--;
      else break;
    }
    return null;
  }

  /** An embankment that is not under a level change: a wall face all the same (an urban river wall), its coping on top. */
  private wallCell(x: number, y: number): Indexed {
    const img = this.tile("grass", 255, 0);
    const top = this.t(x, y - 1) !== K.Embankment;
    const bottom = this.t(x, y + 1) !== K.Embankment;
    this.paintFace(img, x, y, "wall", 0, 16, top, bottom);
    return img;
  }

  /**
   * Level changes around a cell: the face of the higher ground to the north
   * (drawn at the top of this cell), a crest where this cell drops to the
   * south, and lip lines where it drops to the north, east or west.
   */
  private levels(img: Indexed, x: number, y: number, t: number) {
    if (!this.v.elevation) return;
    const e0 = this.e(x, y);
    const { darker, lighter } = this.t2;
    const faceFull = (t === K.Embankment || t === K.Rock) && this.faceTop(x, y) !== null;
    // North-facing drop: only a lip line.
    if (this.e(x, y - 1) < e0) {
      for (let px = 0; px < 16; px++) {
        img.px[px] = darker[darker[img.px[px] as number] as number] as number;
        img.px[16 + px] = lighter[img.px[16 + px] as number] as number;
      }
    }
    // East and west drops: a shade line on the east edge, a light one on the west (light comes from the upper left).
    if (this.e(x + 1, y) < e0) for (let py = 0; py < 16; py++) img.px[py * 16 + 15] = darker[img.px[py * 16 + 15] as number] as number;
    if (this.e(x - 1, y) < e0) for (let py = 0; py < 16; py++) img.px[py * 16] = lighter[img.px[py * 16] as number] as number;
    // South-facing drop: the crest catches the light.
    if (this.e(x, y + 1) < e0 && t !== K.Steps) for (let px = 0; px < 16; px++) img.px[15 * 16 + px] = lighter[img.px[15 * 16 + px] as number] as number;
    // The face of the higher ground to the north.
    const up = this.e(x, y - 1) - e0;
    if (up > 0 && !faceFull && t !== K.Steps && t !== K.Water && t !== K.Creek) {
      const above = this.t(x, y - 1);
      const kind: FaceKind = above === K.Rock || above === K.Pit || t === K.Pit ? "rock" : PAVED.has(above) || above === K.Embankment ? "wall" : "bank";
      const h = up >= 3 ? 16 : up === 2 ? 12 : 8;
      this.paintFace(img, x, y, kind, 0, h, true, true);
    } else if (up > 0 && (t === K.Water || t === K.Creek)) {
      // Over water the face is a wall at the water's edge, 8 px (the bank's shade is in the water edge).
      const above = this.t(x, y - 1);
      this.paintFace(img, x, y, above === K.Rock ? "rock" : "wall", 0, 8, true, true);
    }
  }

  /**
   * Paint a face (an earth bank, a coursed stone wall, a rock face) from row
   * `y0` for `h` px of a cell, in map pixels so runs continue across cells.
   * `top`: the face starts here (a coping, a grass overhang, a rock lip);
   * `bottom`: it ends here (a dark foot). A face's ends (east and west, where
   * the cell beside it has none) slope (banks) or square off (walls, rock).
   */
  private paintFace(img: Indexed, x: number, y: number, kind: FaceKind, y0: number, h: number, top: boolean, bottom: boolean) {
    const L = this.L;
    const has = (dx: number) => {
      const nx = x + dx;
      const t = this.t(nx, y);
      if ((t === K.Embankment || t === K.Rock) && this.faceTop(nx, y) !== null) return true;
      return this.e(nx, y - 1) > this.e(nx, y) || ((t === K.Embankment) && kind === "wall");
    };
    const endW = !has(-1);
    const endE = !has(1);
    for (let py = y0; py < Math.min(16, y0 + h); py++) {
      const r = py - y0;
      for (let px = 0; px < 16; px++) {
        // Banks slope down at their ends (a triangle of 4 px).
        if (kind === "bank") {
          if (endE && px > 11 && r > h - 1 - (16 - px) * 2) continue;
          if (endW && px < 4 && r > h - 1 - (px + 1) * 2) continue;
        }
        const X = x * 16 + px;
        const Y = y * 16 + py;
        let c: string;
        if (kind === "wall") c = this.wallPx(X, Y, r, h, top, bottom);
        else if (kind === "rock") c = this.rockPx(X, Y, r, h, top, bottom);
        else c = this.bankPx(X, Y, r, h, top);
        if (kind !== "bank" && endW && px === 0) c = "q";
        if (kind !== "bank" && endE && px === 15) c = "Q";
        img.px[py * 16 + px] = L[c] as number;
      }
    }
  }

  /** Coursed stone (paving greys): 4 px courses, 8 px blocks, staggered; a light coping and its shadow; a dark foot. */
  private wallPx(X: number, Y: number, r: number, h: number, top: boolean, bottom: boolean): string {
    if (top && r === 0) return "q";
    if (top && r === 1) return "t";
    if (top && r === 2) return "Q";
    if (bottom && r === h - 1) return "A";
    const yy = Y;
    const course = Math.floor((((yy % 4096) + 4096) % 4096) / 4);
    const inC = (((yy % 4) + 4) % 4);
    if (inC === 3) return "P";
    const off = course % 2 ? 4 : 0;
    if ((X + off) % 8 === 7) return "P";
    const block = hash32(Math.floor((X + off) / 8), course, 0x3a1);
    if (inC === 0 && (X + off) % 8 < 6) return block % 5 === 0 ? "t" : "q";
    return block % 4 === 0 ? "s" : block % 7 === 0 ? "P" : "p";
  }

  /**
   * Rock: vertical prisms 4–10 px wide (lit on their left, shaded to the right,
   * a crevice between them), broken by ledges at different heights in each
   * column (a lit top over a dark break), so the face never reads as coursed
   * masonry; a lip on top, scree and a dark foot at the bottom.
   */
  private rockPx(X: number, Y: number, r: number, h: number, top: boolean, bottom: boolean): string {
    if (top && r === 0) return "q";
    if (top && r === 1) return hash32(X >> 1, 0x11) % 3 === 0 ? "q" : "p";
    if (bottom && r === h - 1) return "A";
    if (bottom && r === h - 2) return hash32(X, 0x5c) % 3 === 0 ? "p" : "Q";
    const edge = (k: number) => 7 * k + (hash32(k, 0x7c) % 5) - 2;
    let k = Math.floor(X / 7);
    if (X < edge(k)) k--;
    else if (X >= edge(k + 1)) k++;
    const fx = X - edge(k);
    const fw = edge(k + 1) - edge(k);
    const off = hash32(k, 0x1ed) % 9;
    const band = (Y + off) % 9;
    if (band === 0) return fx < fw - 1 ? "Q" : "A";
    if (band === 1 && fx < fw - 2) return "q";
    if (fx === fw - 1) return fw >= 6 ? "A" : "Q";
    if (fx === 0) return "q";
    if (fx * 10 < fw * 4) return "p";
    if (fx * 4 < fw * 3) return hash32(X >> 1, Y >> 1, 0x2) % 7 === 0 ? "Q" : "P";
    return "Q";
  }

  /** An earth bank under grass: the turf overhangs the top, roots and stones in the earth, a dark foot. */
  private bankPx(X: number, Y: number, r: number, h: number, top: boolean): string {
    const tuft = hash32(X, 0x7f) % 5;
    if (top && r === 0) return "g";
    if (top && r === 1) return "G";
    if (top && r === 2 && tuft < 2) return "G";
    if (top && r === 3 && tuft === 0) return "H";
    if (r === h - 1) return "H";
    const hh = hash32(Math.floor(X / 2), Math.floor(Y / 2), 0xba);
    if (hh % 11 === 0) return "p";
    if (hh % 7 === 0) return "E";
    return r < 4 ? "e" : (X + Y) % 9 === 0 ? "E" : "e";
  }

  /** Steps where a path crosses a level change: treads 4 px deep rising to the north, cheeks where no steps continue beside them. */
  private steps(x: number, y: number): Indexed {
    const L = this.L;
    const img = blank(16, 16);
    // Treads run east–west, unless the level changes east–west.
    const ew = this.e(x - 1, y) !== this.e(x + 1, y) && this.e(x, y - 1) === this.e(x, y + 1);
    const cheekW = this.t(x - 1, y) !== K.Steps;
    const cheekE = this.t(x + 1, y) !== K.Steps;
    for (let py = 0; py < 16; py++) {
      for (let px = 0; px < 16; px++) {
        const k = ew ? px % 4 : py % 4;
        let c = k === 0 ? "t" : k === 3 ? "Q" : "s";
        if (!ew && cheekW && px < 2) c = px === 0 ? "q" : "P";
        if (!ew && cheekE && px > 13) c = px === 15 ? "Q" : "P";
        img.px[py * 16 + px] = L[c] as number;
      }
    }
    return img;
  }

  // ------------------------------------------------------------ rail

  /** A cell beside a diagonal link (the track crosses its corner). */
  private nearRail(x: number, y: number): boolean {
    for (const [dx, dy] of [
      [1, 0],
      [-1, 0],
      [0, 1],
      [0, -1],
    ] as const) {
      if (!this.inside(x + dx, y + dy)) continue;
      const m = this.railMask[(y + dy) * this.W + x + dx] as number;
      if (!m) continue;
      // A neighbour's diagonal link that passes this cell's corner.
      for (const [ddx, ddy, bit] of DIRS.slice(4)) if (m & bit && (ddx === -dx || ddy === -dy)) return true;
    }
    return false;
  }

  /**
   * Track, in map pixels: every link (cell centre to cell centre) near this
   * cell is a band of ballast with sleepers across it and two rails; a 45°
   * link crosses the shared corner of two other cells, which draw their part.
   * On a level crossing only the rails show, set in the road.
   */
  private rail(img: Indexed, x: number, y: number) {
    const L = this.L;
    const links: [number, number, number, number][] = [];
    for (let cy = y - 1; cy <= y + 1; cy++) {
      for (let cx = x - 1; cx <= x + 1; cx++) {
        if (!this.inside(cx, cy)) continue;
        const m = this.railMask[cy * this.W + cx] as number;
        for (const [dx, dy, bit] of DIRS) if (m & bit) links.push([cx, cy, dx, dy]);
      }
    }
    if (links.length === 0) return;
    const inRoad = (cx: number, cy: number) => this.t(cx, cy) === K.Crossing;
    const onDeck = (cx: number, cy: number) => this.deck(cx, cy)?.kind === "viaduct";
    // Per pixel: the strongest layer any link puts there (0 none, 1 ballast edge, 2 ballast, 3 sleeper, 4 rail shadow, 5 rail head).
    for (let py = 0; py < 16; py++) {
      for (let px = 0; px < 16; px++) {
        const X = x * 16 + px;
        const Y = y * 16 + py;
        let layer = 0;
        let gravel = false;
        for (const [cx, cy, dx, dy] of links) {
          const bare = inRoad(cx, cy) || inRoad(cx + dx, cy + dy);
          let n: number;
          let a: number;
          let len: number;
          let diag = false;
          if (dy === 0) {
            n = 2 * Y - 32 * cy - 15;
            a = (X - (16 * cx + 8)) * dx;
            len = 16;
          } else if (dx === 0) {
            n = 2 * X - 32 * cx - 15;
            a = (Y - (16 * cy + 8)) * dy;
            len = 16;
          } else {
            diag = true;
            const lx = X - 16 * cx;
            const ly = Y - 16 * cy;
            // Perpendicular offset in diagonal units (real distance n/√2) and position along (real a/√2, 0 at the start centre).
            n = dx === dy ? lx - ly : lx + ly - 15;
            a = dx === dy ? (lx + ly - 15) * dx : (lx - ly) * dx;
            len = 32;
          }
          const an = Math.abs(n);
          const reach = (ext: number) => a >= -ext && a <= len + ext;
          // Ballast and sleepers (not in a road).
          if (!bare && !onDeck(cx, cy) && !onDeck(cx + dx, cy + dy)) {
            if (reach(diag ? 5 : 4)) {
              if (diag ? an <= 9 : an <= 13) {
                const edge = diag ? an >= 9 : an >= 13;
                layer = Math.max(layer, edge ? 1 : 2);
                if (!edge) gravel = true;
              }
            }
          } else if (!bare && reach(diag ? 5 : 4) && (diag ? an <= 6 : an <= 9)) layer = Math.max(layer, 2);
          const deckLink = onDeck(cx, cy) || onDeck(cx + dx, cy + dy);
          if (!bare && reach(diag ? 3 : 2)) {
            const along = diag ? (dx === dy ? X + Y : X - Y) : dy === 0 ? X : Y;
            const sl = diag ? ((along % 8) + 8) % 8 < 3 : ((along % 4) + 4) % 4 < 2;
            if (sl && (diag ? an <= (deckLink ? 6 : 7) : an <= (deckLink ? 9 : 11))) layer = Math.max(layer, 3);
          }
          // Rails: heads at real ±3.5 px, their shadow one pixel toward the lower right.
          if (reach(diag ? 1 : 1)) {
            if (diag ? an === 5 : an === 7) layer = Math.max(layer, 5);
            else if (diag ? n === 6 || n === -4 : n === 9 || n === -5) layer = Math.max(layer, 4);
          }
        }
        const i = py * 16 + px;
        if (layer === 5) img.px[i] = L.m as number;
        else if (layer === 4) img.px[i] = L.i as number;
        else if (layer === 3) img.px[i] = L.D as number;
        else if (layer === 2) img.px[i] = (gravel && (hash32(X >> 1, Y >> 1, 0xba1) & 7) === 0 ? L.Q : L.P) as number;
        else if (layer === 1) img.px[i] = L.Q as number;
      }
    }
  }

  /** A platform: paving beside the track, a yellow line, its coping, and its face toward rail to the south. */
  private platform(img: Indexed, x: number, y: number, t: number) {
    if (t !== K.Pavement && t !== K.Plaza && t !== K.Quay && t !== K.Yard) return;
    // A diagonal street's pavement beside the line is not a platform (round 5).
    if ((this.v.streetAxis?.[y * this.W + x] ?? 0) !== 0) return;
    const L = this.L;
    const put = (px: number, py: number, c: string) => (img.px[py * 16 + px] = L[c] as number);
    if (this.t(x, y + 1) === K.Rail) {
      for (let px = 0; px < 16; px++) {
        put(px, 9, (x * 16 + px) % 4 < 3 ? "Y" : "y");
        put(px, 11, "q");
        put(px, 12, "t");
        put(px, 13, "Q");
        put(px, 14, "A");
        put(px, 15, "A");
      }
    }
    if (this.t(x, y - 1) === K.Rail) {
      for (let px = 0; px < 16; px++) {
        put(px, 0, "Q");
        put(px, 1, "t");
        put(px, 4, (x * 16 + px) % 4 < 3 ? "Y" : "y");
      }
    }
  }

  /** A level crossing: the road (its group's autotile) with the rails set in it, a stop line each side and barrier posts at the corners. */
  private levelCrossing(x: number, y: number, under?: Indexed): Indexed {
    const L = this.L;
    const road = this.ts.terrains.road;
    const img = under ?? (road ? autotile(road.template, canonicalMask(this.mask(x, y, (n) => group(n) === K.Road)), road.fills[0]) : this.tile("road", 255, 0));
    const railEW = this.isRail(x - 1, y) || this.isRail(x + 1, y);
    const put = (px: number, py: number, c: string) => (img.px[py * 16 + px] = L[c] as number);
    const ax = this.v.streetAxis?.[y * this.W + x] ?? 0;
    if (this.streets && ax !== 0) {
      // Set into a diagonal road (round 5): stop lines where the carriageway meets it, posts on the verge side.
      this.streets.levelCrossingMarks(img, x, y, railEW);
      for (let k = 0; k < 16; k++) for (const r of [3, 10]) put(railEW ? k : r, railEW ? r : k, "A");
      return img;
    }
    // The road's edges along the track: a dashed white line where cars stop.
    for (let k = 0; k < 16; k++) {
      if (k % 4 === 3) continue;
      if (railEW) {
        if (this.t(x, y - 1) !== K.Crossing) put(k, 0, "M");
        if (this.t(x, y + 1) !== K.Crossing) put(k, 15, "M");
      } else {
        if (this.t(x - 1, y) !== K.Crossing) put(0, k, "M");
        if (this.t(x + 1, y) !== K.Crossing) put(15, k, "M");
      }
    }
    // Barrier posts (red and white) where the crossing meets the verge.
    const corners: [number, number, boolean][] = [
      [0, 0, this.t(x - 1, y) !== K.Crossing && this.t(x, y - 1) !== K.Crossing],
      [14, 0, this.t(x + 1, y) !== K.Crossing && this.t(x, y - 1) !== K.Crossing],
      [0, 12, this.t(x - 1, y) !== K.Crossing && this.t(x, y + 1) !== K.Crossing],
      [14, 12, this.t(x + 1, y) !== K.Crossing && this.t(x, y + 1) !== K.Crossing],
    ];
    for (const [cx, cy, on] of corners) {
      if (!on) continue;
      put(cx, cy, "R");
      put(cx + 1, cy, "R");
      put(cx, cy + 1, "c");
      put(cx + 1, cy + 1, "c");
      put(cx, cy + 2, "R");
      put(cx + 1, cy + 2, "R");
      put(cx, cy + 3, "O");
      put(cx + 1, cy + 3, "O");
    }
    // Flangeways: a dark groove beside each rail (the rails themselves come from the track pass).
    for (let k = 0; k < 16; k++) {
      for (const r of [3, 10]) {
        if (railEW) put(k, r, "A");
        else put(r, k, "A");
      }
    }
    return img;
  }

  // ------------------------------------------------------------ bridges

  /** A deck cell: road, planks or a viaduct's ballast between parapets; over water or a creek where a footbridge leaves the sides open. */
  private deckCell(x: number, y: number, variant: number): Indexed {
    const d = this.deck(x, y);
    const L = this.L;
    const c = d?.c ?? { kind: "bridge", x, y, w: 1, h: 1, axis: "ew" as const };
    const kind = d?.kind ?? "bridge";
    const ew = c.axis !== "ns";
    // What is under it (seen past a footbridge's open sides).
    let img: Indexed;
    if (this.creekUnder(x, y)) img = this.creek(x, y, variant, false, ew ? NB.N | NB.S : NB.E | NB.W);
    else if (WATERY.has(this.t(x, y - 1)) || WATERY.has(this.t(x, y + 1)) || WATERY.has(this.t(x - 1, y)) || WATERY.has(this.t(x + 1, y))) img = this.water(x, y, variant);
    else img = this.tile("grass", 255, variant);
    // Pixel position across the deck (0 at its north or west edge) and along it.
    const X0 = c.x * 16;
    const Y0 = c.y * 16;
    const span = (ew ? c.h : c.w) * 16;
    const run = (ew ? c.w : c.h) * 16;
    const parapet = kind === "footbridge" ? 2 : 3;
    const inset = kind === "footbridge" ? 2 : 0;
    for (let py = 0; py < 16; py++) {
      for (let px = 0; px < 16; px++) {
        const X = x * 16 + px;
        const Y = y * 16 + py;
        const across = ew ? Y - Y0 : X - X0;
        const along = ew ? X - X0 : Y - Y0;
        if (across < inset || across >= span - inset) continue;
        const a = across - inset;
        const sw = span - 2 * inset;
        let ch: string;
        const nearEnd = along < 3 || along >= run - 3;
        if (a < parapet || a >= sw - parapet) {
          // Parapets: stone or brick coping; a footbridge's timber handrail on posts.
          const k = a < parapet ? a : sw - 1 - a;
          if (kind === "footbridge") ch = k === 0 ? (along % 6 === 0 ? "D" : "k") : "D";
          else {
            const brick = kind === "viaduct";
            // Ew: north parapet shows its south face (rows 1..2); south parapet its coping (row 0 of it) then its face.
            if (ew) ch = a < parapet ? (k === 0 ? (brick ? "L" : "t") : k === 1 ? (brick ? "K" : "s") : brick ? "B" : "S") : k === parapet - 1 ? (brick ? "L" : "t") : brick ? "K" : "s";
            else ch = a < parapet ? (k === 0 ? "T" : k === 1 ? (brick ? "L" : "t") : brick ? "B" : "S") : k === 0 ? "T" : k === 1 ? (brick ? "K" : "s") : brick ? "L" : "t";
            if (nearEnd) ch = brick ? (along < 3 ? "L" : "B") : along < 3 ? "t" : "S";
          }
        } else if (kind === "footbridge") {
          // Planks laid across the way: a dark gap every 3 px.
          ch = along % 3 === 2 ? "D" : (hash32(Math.floor(along / 3), 0x9a) & 3) === 0 ? "k" : "d";
        } else if (kind === "viaduct") ch = "P";
        else {
          // The carriageway: asphalt, a kerb line by each parapet.
          ch = a === parapet || a === sw - parapet - 1 ? "Q" : (hash32(X >> 1, Y >> 1, 0xdec) & 15) === 0 ? "A" : "a";
        }
        img.px[py * 16 + px] = L[ch] as number;
      }
    }
    return img;
  }

  /**
   * What a deck draws beyond itself on this cell: below an east–west bridge
   * its south face (parapet, string course, spandrel and arches, piers and
   * their ripples; a viaduct's two tiers of brick; a footbridge's beam and
   * trestles) and its shadow on the water; beside a north–south bridge, its
   * shadow on the water to the east and cutwaters on both sides.
   */
  private bridgeAround(img: Indexed, x: number, y: number) {
    for (const d of this.decks) {
      const c = d.c;
      if (d.kind === "ford" || d.kind === "level_crossing") continue;
      if (c.axis !== "ns") {
        // A viaduct of three or more spans stands two tiers tall; a short one (over a creek) is a single low arch.
        const faceH = d.kind === "viaduct" ? (c.w >= 3 ? 30 : 14) : d.kind === "footbridge" ? 6 : 14;
        const Yd = (c.y + c.h) * 16;
        if (x < c.x || x >= c.x + c.w || y * 16 + 16 <= Yd || y * 16 >= Yd + faceH + 3) continue;
        this.ewFace(img, x, y, d, Yd, faceH);
      } else {
        if (y < c.y || y >= c.y + c.h) continue;
        if (x === c.x + c.w) this.nsSide(img, x, y, d, "east");
        else if (x === c.x - 1) this.nsSide(img, x, y, d, "west");
      }
    }
  }

  private ewFace(img: Indexed, x: number, y: number, d: Deck, Yd: number, faceH: number) {
    const L = this.L;
    const { darker } = this.t2;
    const c = d.c;
    const X0 = c.x * 16;
    const len = c.w * 16;
    const brick = d.kind === "viaduct";
    const timber = d.kind === "footbridge";
    // Spans: abutments of 4 px at both ends, piers of 4 px between spans.
    const abut = timber ? 0 : 4;
    const pier = timber ? 2 : 4;
    const want = brick ? 18 : timber ? 16 : 28;
    const nSpan = Math.max(1, Math.round((len - 2 * abut) / want));
    const spanW = (len - 2 * abut - pier * (nSpan - 1)) / nSpan;
    const archTop = 6;
    const archH = faceH - archTop;
    for (let py = 0; py < 16; py++) {
      const Y = y * 16 + py;
      const r = Y - Yd;
      if (r < 0 || r >= faceH + 3) continue;
      for (let px = 0; px < 16; px++) {
        const X = x * 16 + px;
        const u = X - X0;
        const i = py * 16 + px;
        // Which span (or pier/abutment) this column is in.
        const v = u - abut;
        const k = Math.floor(v / (spanW + pier));
        const inSpan = v >= 0 && u < len - abut && v - k * (spanW + pier) < spanW;
        const su = v - k * (spanW + pier) - spanW / 2;
        if (r >= faceH) {
          // Below the face: the bridge's shade on the water (2 px), ripples at the piers' feet.
          if (r < faceH + 2 && inSpan) img.px[i] = darker[img.px[i] as number] as number;
          else if (r === faceH && !inSpan && !timber) img.px[i] = L.v as number;
          continue;
        }
        if (timber) {
          // A beam (3 px) and trestle legs at the span ends going down into the water.
          if (r < 3) img.px[i] = (r === 0 ? L.k : r === 1 ? L.d : L.D) as number;
          else if (!inSpan) img.px[i] = (((u % 32) + 32) % 32 < 1 ? L.k : L.D) as number;
          else if (r === 3) img.px[i] = darker[img.px[i] as number] as number;
          continue;
        }
        // Inside an arch: what is behind, in deep shade; the intrados darkest under the ring.
        if (inSpan && r >= archTop) {
          const half = spanW / 2;
          const q = (su / half) ** 2 + ((faceH - r) / archH) ** 2;
          if (q < 1) {
            const ring = q > 0.72;
            if (ring) img.px[i] = (brick ? (su < 0 ? L.L : L.X) : su < 0 ? L.t : L.T) as number;
            else img.px[i] = (r < archTop + 3 ? L.A : darker[darker[img.px[i] as number] as number]) as number;
            continue;
          }
        }
        // The face: the south parapet's outer face (2 px), a string course, the spandrel's courses.
        let ch: string;
        if (r < 2) ch = brick ? "K" : "s";
        else if (r === 2) ch = brick ? "L" : "t";
        else if (r === 3) ch = brick ? "X" : "S";
        else {
          const course = Math.floor((r - 4) / 3);
          const off = course % 2 ? 3 : 0;
          const joint = (r - 4) % 3 === 2 || (X + off) % 6 === 5;
          ch = brick ? (joint ? "B" : "K") : joint ? "S" : "s";
          // Light on the left edge of the abutment and the piers, shade on their right.
          if (!inSpan && v - k * (spanW + pier) - spanW === 0) ch = brick ? "L" : "t";
          if (u === 0) ch = brick ? "L" : "t";
          if (u === len - 1) ch = brick ? "X" : "T";
        }
        img.px[i] = L[ch] as number;
      }
    }
  }

  private nsSide(img: Indexed, x: number, y: number, d: Deck, side: "east" | "west") {
    const L = this.L;
    const { darker } = this.t2;
    const c = d.c;
    if (!WATERY.has(this.t(x, y)) && this.t(x, y) !== K.Bridge) return;
    for (let py = 0; py < 16; py++) {
      const Y = y * 16 + py;
      const along = Y - c.y * 16;
      for (let px = 0; px < 16; px++) {
        const i = py * 16 + px;
        // The deck's shadow on the water to the east (5 px, light from the upper left).
        if (side === "east" && px < 5) img.px[i] = darker[img.px[i] as number] as number;
        // Cutwaters: a pier's pointed foot every 32 px, 3 px out from the deck, with a ripple.
        if (d.kind !== "footbridge" && along % 32 >= 12 && along % 32 < 18) {
          const out = side === "east" ? px : 15 - px;
          const w = 3 - Math.abs((along % 32) - 15) / 2;
          if (out < w) img.px[i] = (d.kind === "viaduct" ? (side === "west" ? L.L : L.B) : side === "west" ? L.t : L.S) as number;
          else if (out < w + 1) img.px[i] = L.v as number;
        } else if (d.kind === "footbridge" && along % 32 === 15 && (side === "east" ? px < 1 : px > 14)) img.px[i] = L.D as number;
      }
    }
  }
}

// ---------------------------------------------------------------------------
// One tile without a map (the v1 path: `ground(kind, mask, variant)`)

/**
 * A v2 terrain as one tile from its neighbour mask alone, for a renderer
 * that asks only `ground(kind, mask, variant)`: the cell's neighbours with
 * the bit set are the same terrain, the rest grass, and the map is flat.
 */
export function tileOf(ts: TileSet, t2: TerrainSet, code: number, mask: number, variant: number): Indexed {
  const terrain = new Uint8Array(9).fill(K.Grass);
  terrain[4] = code;
  const at: [number, number, number][] = [
    [1, 0, NB.N],
    [2, 1, NB.E],
    [1, 2, NB.S],
    [0, 1, NB.W],
    [2, 0, NB.NE],
    [2, 2, NB.SE],
    [0, 2, NB.SW],
    [0, 0, NB.NW],
  ];
  for (const [x, y, bit] of at) if (mask & bit) terrain[y * 3 + x] = code;
  const connectors: ConnectorView[] = [];
  if (code === K.Bridge) {
    const ew = (mask & (NB.E | NB.W)) !== 0 || (mask & (NB.N | NB.S)) === 0;
    connectors.push(ew ? { kind: "bridge", x: 0, y: 1, w: 3, h: 1, axis: "ew" } : { kind: "bridge", x: 1, y: 0, w: 1, h: 3, axis: "ns" });
    // Water beside the deck, so the deck reads as a bridge.
    for (const i of ew ? [0, 1, 2, 6, 7, 8] : [0, 3, 6, 2, 5, 8]) terrain[i] = K.Water;
    for (const i of ew ? [3, 5] : [1, 7]) terrain[i] = K.Bridge;
  }
  const p = new TerrainPainter(ts, t2, { width: 3, height: 3, terrain, connectors });
  return p.draw(1, 1, variant);
}
