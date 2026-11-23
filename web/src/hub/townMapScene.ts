// The town map overview (DEC-086): what the low-resolution map of the
// player's town shows and where you can tap it, built from the same versioned
// layout as the walkable street (DEC-085, ./townLayout), so the two always
// agree. Pure and deterministic: no DOM, no clock, no randomness.
//
// - Hotspots: one per entry of the town's place list (`TownPlacesView`): the
//   player's home, the streets of every neighbourhood (tapped by the
//   neighbourhood's name) and every place, in the list's order, so the map
//   and the List view (DEC-051) offer exactly the same things. Places with
//   nothing to do yet (the list's "Also here") are hotspots too, marked
//   inactive: tapping one says so instead of going there, as the list does.
// - Routes: the streets from the central square to every hotspot (the town's
//   main connections), and the route from wherever the player is to any
//   hotspot (previews and the walk shown on the map). Both come from one
//   street-preferring shortest-path tree per source cell.
//
// Presentation only (AGENTS.md): the engine decides what each place offers
// and where the player is; this only decides where it is drawn.
import type { LocationDto } from "../engine/protocol/LocationDto";
import type { TownPlacesView } from "../engine/protocol/TownPlacesView";
import type { Point } from "./pathfinding";
import type { TownInput } from "./townInput";
import { ROOF_ROWS, StreetAxis, Terrain, buildingOfPlace, locationCell, type TownMap } from "./townLayout";

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export type HotspotKind = "place" | "home" | "streets";

export interface MapHotspot {
  /** Stable identity: `home`, `streets:<neighbourhood id>` or `place:<place id>`. */
  key: string;
  kind: HotspotKind;
  /** What visiting it sends to the engine (the same as the list's button). */
  location: LocationDto;
  /** Place-registry key (`town_hall`, …), `home` or `streets`. */
  kindKey: string;
  /** The place's generated proper name, or "". */
  name: string;
  /** Neighbourhood id and name (the list's section). */
  neighborhood: number;
  neighborhoodName: string;
  /** Index into `map.buildings` (null for streets). */
  building: number | null;
  /** Index into `map.districts`. */
  district: number;
  /** Hit area, tiles: the footprint and its roof row, or a plate round the neighbourhood's name. */
  rect: Rect;
  /** Label anchor, tiles: the top centre of the building, or the neighbourhood's centre. */
  anchor: Point;
  /** The cell routes to it end at (the door's doorstep, or the neighbourhood's street). */
  cell: number;
  /** It offers actions or services (a button in the list); false for the list's "Also here". */
  active: boolean;
  /** A civic landmark (drawn larger, labelled first). */
  landmark: boolean;
  /** Position in the list (tab order follows it). */
  order: number;
}

export interface TownMapScene {
  map: TownMap;
  hotspots: MapHotspot[];
  /** The cell the main routes start from (the central square, else the centre's streets). */
  hub: number;
  /** Main routes from the hub to every hotspot, as polylines of cell centres (tiles). */
  routes: Point[][];
}

/** Civic kinds, labelled before everything else. */
const CIVIC = new Set(["town_hall", "council_chamber", "parliament", "regional_assembly", "executive_residence"]);

/** The town's places as map hotspots, in the list's order. */
export function mapHotspots(map: TownMap, view: TownPlacesView): MapHotspot[] {
  const out: MapHotspot[] = [];
  const seen = new Set<string>();
  for (const h of view.neighborhoods) {
    const district = map.districts.findIndex((d) => d.id === h.id);
    for (const s of h.spots) {
      const l = s.location.location;
      const key = l.type === "home" ? "home" : l.type === "street" ? `streets:${l.neighborhood}` : `place:${l.place}`;
      if (seen.has(key)) continue;
      const active = s.actions.length + s.services.length + s.verbs.length > 0;
      const base = { key, location: l, kindKey: s.location.kind_key, name: s.location.name, neighborhood: h.id, neighborhoodName: h.name, active, order: out.length };
      if (l.type === "street") {
        const d = map.districts[district];
        if (!d || d.street < 0) continue;
        seen.add(key);
        out.push({ ...base, kind: "streets", building: null, district, rect: { x: d.centre.x - 2, y: d.centre.y - 0.75, w: 4, h: 1.5 }, anchor: { ...d.centre }, cell: d.street, landmark: false });
        continue;
      }
      const b = l.type === "home" ? (map.homeBuilding !== null ? (map.buildings[map.homeBuilding] ?? null) : null) : buildingOfPlace(map, l.place);
      if (!b) continue;
      seen.add(key);
      out.push({
        ...base,
        kind: l.type === "home" ? "home" : "place",
        building: b.index,
        district: b.district,
        rect: { x: b.x, y: b.y - ROOF_ROWS, w: b.w, h: b.h + ROOF_ROWS },
        anchor: { x: b.x + b.w / 2, y: b.y - ROOF_ROWS },
        cell: b.front,
        landmark: b.landmark || CIVIC.has(b.kind),
      });
    }
  }
  return out;
}

/** The hotspot for an engine location, if the map has one. */
export function hotspotOf(hotspots: readonly MapHotspot[], loc: LocationDto): MapHotspot | null {
  const key = loc.type === "home" ? "home" : loc.type === "street" ? `streets:${loc.neighborhood}` : `place:${loc.place}`;
  return hotspots.find((h) => h.key === key) ?? null;
}

/** Where the player stands for a location: the door's doorstep, or the street (tiles, cell centre). */
export function locationPoint(map: TownMap, loc: LocationDto): Point | null {
  const c = locationCell(map, loc);
  return c === null ? null : cellPoint(map, c);
}

export const cellPoint = (map: TownMap, cell: number): Point => ({ x: (cell % map.width) + 0.5, y: Math.floor(cell / map.width) + 0.5 });

// ---------------------------------------------------------------------------
// Routes: street-preferring shortest paths (4 directions, and 45° along
// layout v2's diagonal streets; few turns), drawn as orthogonal and 45° legs

/** Cost of entering a cell, ×10 (0: impassable). Streets are cheapest, so routes follow them. */
function stepCost(t: number): number {
  switch (t) {
    case Terrain.Road:
    case Terrain.Crossing:
    case Terrain.Dirt:
    case Terrain.Bridge:
      return 10;
    case Terrain.Pavement:
    case Terrain.Plaza:
    case Terrain.Quay:
    case Terrain.Path:
    case Terrain.Pier:
    case Terrain.Steps:
      return 16;
    case Terrain.Yard:
      return 24;
    case Terrain.Grass:
    case Terrain.Garden:
    case Terrain.Park:
    case Terrain.Field:
    case Terrain.Sand:
    case Terrain.Reeds:
    case Terrain.Ford:
      return 50;
    default:
      return 0;
  }
}
const TURN = 1;
/** A diagonal step costs about √2 of a straight one. */
const DIAG = (w: number) => Math.round((w * 14) / 10);
const MAX_STEP = DIAG(50) + TURN;

/** A shortest-path tree from one cell: the parent of every reached cell (-1 unreached, -2 the source). */
export interface RouteTree {
  source: number;
  parent: Int32Array;
}

// East, south, west, north, then the diagonals (south-east, south-west, north-west, north-east).
const DX = [1, 0, -1, 0, 1, -1, -1, 1];
const DY = [0, 1, 0, -1, 1, 1, -1, -1];

/** Whether a cell's street runs along a diagonal step (dx, dy): "/" for north-east–south-west, "\\" for the other. */
function alongDiagonal(axis: number, dx: number, dy: number): boolean {
  if (axis === StreetAxis.Junction) return true;
  return dx === dy ? axis === StreetAxis.DiagonalNW : axis === StreetAxis.DiagonalNE;
}

/**
 * One shortest-path tree over the walkable cells (Dial's buckets on integer
 * costs). Turning costs a little, so routes run straight along a street and
 * turn at corners instead of zig-zagging. Along layout v2's 45° streets
 * (`TownMap.streetAxis`) a route steps diagonally, so it runs straight down
 * the street rather than in a staircase (never cutting a corner past a
 * blocked cell). Deterministic: fixed neighbour order.
 */
export function routeTree(map: TownMap, source: number): RouteTree {
  const W = map.width;
  const H = map.height;
  const n = W * H;
  const cost = new Uint16Array(n);
  for (let i = 0; i < n; i++) cost[i] = map.solid[i] ? 0 : stepCost(map.terrain[i] as number);
  // Doors are part of the building; their doorstep (the front) is walkable already.
  const dist = new Float64Array(n).fill(Infinity);
  const dirIn = new Int8Array(n).fill(-1);
  const parent = new Int32Array(n).fill(-1);
  const done = new Uint8Array(n);
  const axes = map.streetAxis && map.streetAxis.length === n ? map.streetAxis : null;
  if (source < 0 || source >= n) return { source, parent };
  const B = MAX_STEP + 1;
  const buckets: number[][] = Array.from({ length: B }, () => []);
  dist[source] = 0;
  parent[source] = -2;
  buckets[0]?.push(source);
  let pending = 1;
  let at = 0;
  let d = 0;
  while (pending > 0) {
    const bucket = buckets[at] as number[];
    if (bucket.length === 0) {
      at = (at + 1) % B;
      d++;
      continue;
    }
    const c = bucket.pop() as number;
    pending--;
    if (done[c] || dist[c] !== d) continue;
    done[c] = 1;
    const cx = c % W;
    const cy = (c - cx) / W;
    const ac = axes ? (axes[c] as number) : 0;
    for (let k = 0; k < (ac ? 8 : 4); k++) {
      const dx = DX[k] as number;
      const dy = DY[k] as number;
      const x = cx + dx;
      const y = cy + dy;
      if (x < 0 || y < 0 || x >= W || y >= H) continue;
      const m = y * W + x;
      let w = cost[m] as number;
      if (!w || done[m]) continue;
      if (k >= 4) {
        // Along a diagonal street only (both cells on it), past two open cells.
        if (!axes || !alongDiagonal(ac, dx, dy) || !alongDiagonal(axes[m] as number, dx, dy)) continue;
        if (!cost[cy * W + x] || !cost[y * W + cx]) continue;
        w = DIAG(w);
      }
      const din = dirIn[c] as number;
      const nd = d + w + (din >= 0 && din !== k ? TURN : 0);
      if (nd < (dist[m] as number)) {
        dist[m] = nd;
        dirIn[m] = k;
        parent[m] = c;
        buckets[nd % B]?.push(m);
        pending++;
      }
    }
  }
  return { source, parent };
}

/** The route from the tree's source to `cell` as a polyline of cell centres (corners only), or null if unreachable. */
export function routeTo(map: TownMap, tree: RouteTree, cell: number): Point[] | null {
  if (cell < 0 || cell >= tree.parent.length || tree.parent[cell] === -1) return null;
  const cells: number[] = [];
  for (let c = cell; c >= 0; c = tree.parent[c] as number) {
    cells.push(c);
    if (cells.length > tree.parent.length) return null;
  }
  cells.reverse();
  return dejogRoute(map, corners(map, cells));
}

/** Whether a leg runs orthogonally or at 45° (and has length). */
const gridLeg = (a: Point, b: Point) => (a.x !== b.x || a.y !== b.y) && (a.x === b.x || a.y === b.y || Math.abs(b.x - a.x) === Math.abs(b.y - a.y));

/** The cells a leg (orthogonal or 45°) passes, ends included. */
function legCells(map: TownMap, a: Point, b: Point): number[] {
  const n = Math.max(Math.abs(b.x - a.x), Math.abs(b.y - a.y));
  const sx = Math.sign(b.x - a.x);
  const sy = Math.sign(b.y - a.y);
  const out: number[] = [];
  for (let k = 0; k <= n; k++) out.push(Math.floor(a.y + sy * k) * map.width + Math.floor(a.x + sx * k));
  return out;
}

/**
 * Take out the one-cell jogs between longer legs (a lane change on a wide
 * street, a step at a junction): the leg before or after the jog moves
 * across by the jog, where every cell it then passes is open and no rougher
 * than the street it left, so the drawn route turns only where it really
 * turns. The ends stay where they are.
 */
function dejogRoute(map: TownMap, pts: Point[]): Point[] {
  const cost = (c: number) => (map.solid[c] ? 0 : stepCost(map.terrain[c] as number));
  const roughest = (cells: number[]) => cells.reduce((m, c) => Math.max(m, cost(c)), 0);
  const fits = (legs: [Point, Point][], limit: number) =>
    legs.every(([a, b]) => gridLeg(a, b) && legCells(map, a, b).every((c) => {
      const k = cost(c);
      return k > 0 && k <= limit;
    }));
  const short = (a: Point, b: Point) => Math.max(Math.abs(b.x - a.x), Math.abs(b.y - a.y)) === 1;
  let p = pts.map((q) => ({ ...q }));
  for (let guard = 0; guard < 64; guard++) {
    let done = false;
    for (let i = 1; i + 2 < p.length && !done; i++) {
      const a = p[i] as Point;
      const b = p[i + 1] as Point;
      if (!short(a, b)) continue;
      const jx = b.x - a.x;
      const jy = b.y - a.y;
      // The jog itself, or (a diagonal one) either of its square steps: the other then runs on along a leg.
      const moves: [number, number][] = jx && jy ? [[jx, jy], [jx, 0], [0, jy]] : [[jx, jy]];
      for (const [vx, vy] of moves) {
        // Move the leg before the jog across (its start must not be the route's start).
        if (i >= 2) {
          const p0 = p[i - 2] as Point;
          const p1 = p[i - 1] as Point;
          const q1 = { x: p1.x + vx, y: p1.y + vy };
          const qa = { x: a.x + vx, y: a.y + vy };
          const limit = Math.max(16, roughest([...legCells(map, p0, p1), ...legCells(map, p1, a)]));
          const legs: [Point, Point][] = [[p0, q1], [q1, qa]];
          if (qa.x !== b.x || qa.y !== b.y) legs.push([qa, b]);
          if (fits(legs, limit)) {
            p = [...p.slice(0, i - 1), q1, qa, ...p.slice(i + 1)];
            done = true;
            break;
          }
        }
        // Else move the leg after it back (its end must not be the route's end).
        if (i + 3 < p.length) {
          const p2 = p[i + 2] as Point;
          const p3 = p[i + 3] as Point;
          const qb = { x: b.x - vx, y: b.y - vy };
          const q2 = { x: p2.x - vx, y: p2.y - vy };
          const limit = Math.max(16, roughest([...legCells(map, b, p2), ...legCells(map, p2, p3)]));
          const legs: [Point, Point][] = [[qb, q2], [q2, p3]];
          if (qb.x !== a.x || qb.y !== a.y) legs.unshift([a, qb]);
          if (fits(legs, limit)) {
            p = [...p.slice(0, i + 1 - 1), a, qb, q2, ...p.slice(i + 3)];
            done = true;
            break;
          }
        }
      }
    }
    if (!done) break;
    // Merge legs that now run on in the same direction.
    const m: Point[] = [p[0] as Point];
    for (let i = 1; i < p.length; i++) {
      const q = p[i] as Point;
      const last = m[m.length - 1] as Point;
      if (q.x === last.x && q.y === last.y) continue;
      const prev = m[m.length - 2];
      if (prev && Math.sign(q.x - last.x) === Math.sign(last.x - prev.x) && Math.sign(q.y - last.y) === Math.sign(last.y - prev.y) && (q.x - last.x) * (last.y - prev.y) === (q.y - last.y) * (last.x - prev.x)) m[m.length - 1] = q;
      else m.push(q);
    }
    p = m;
  }
  return p;
}

/**
 * The route as it is drawn: orthogonal and 45° legs between cell centres.
 * A staircase of alternating unit steps (three or more: across a square, or
 * along a 45° street the route joined part-way) becomes one 45° leg; then
 * only the ends and the cells where the direction changes are kept.
 */
export function corners(map: TownMap, cells: readonly number[]): Point[] {
  const W = map.width;
  if (cells.length === 0) return [];
  const xs = cells.map((c) => c % W);
  const ys = cells.map((c) => Math.floor(c / W));
  // The steps, then each staircase run folded into diagonal steps.
  const steps: [number, number][] = [];
  for (let i = 1; i < cells.length; i++) steps.push([(xs[i] as number) - (xs[i - 1] as number), (ys[i] as number) - (ys[i - 1] as number)]);
  const unit = (s: readonly [number, number]) => Math.abs(s[0]) + Math.abs(s[1]) === 1;
  const folded: [number, number][] = [];
  for (let i = 0; i < steps.length; ) {
    const a = steps[i] as [number, number];
    const b = steps[i + 1];
    let j = i + 1;
    if (unit(a) && b && unit(b) && a[0] !== b[0] && a[1] !== b[1]) {
      // How far the a, b, a, b, ... pattern runs.
      while (j < steps.length) {
        const s = steps[j] as [number, number];
        const want = (j - i) % 2 === 0 ? a : b;
        if (s[0] !== want[0] || s[1] !== want[1]) break;
        j++;
      }
    }
    const run = j - i;
    if (run >= 3) {
      for (let k = 0; k + 1 < run; k += 2) folded.push([a[0] + (b as [number, number])[0], a[1] + (b as [number, number])[1]]);
      if (run % 2 === 1) folded.push(a);
      i = j;
    } else {
      folded.push(a);
      i++;
    }
  }
  const pts: Point[] = [{ x: (xs[0] as number) + 0.5, y: (ys[0] as number) + 0.5 }];
  let x = xs[0] as number;
  let y = ys[0] as number;
  for (let i = 0; i < folded.length; i++) {
    const s = folded[i] as [number, number];
    x += s[0];
    y += s[1];
    const next = folded[i + 1];
    if (next && next[0] === s[0] && next[1] === s[1]) continue;
    pts.push({ x: x + 0.5, y: y + 0.5 });
  }
  return pts;
}

/** The central square's middle cell, else the centre district's streets, else the start. */
export function hubCell(map: TownMap): number {
  const sq = map.square;
  if (sq) {
    const c = (sq.y + Math.floor(sq.h / 2)) * map.width + sq.x + Math.floor(sq.w / 2);
    if (!map.solid[c] && stepCost(map.terrain[c] as number) > 0) return c;
    // A fountain in the middle: the nearest open plaza cell along the square's bottom row.
    for (let x = sq.x; x < sq.x + sq.w; x++) {
      const e = (sq.y + sq.h - 1) * map.width + x;
      if (!map.solid[e] && stepCost(map.terrain[e] as number) > 0) return e;
    }
  }
  const core = map.districts.find((d) => d.kind === "core") ?? map.districts[0];
  return core && core.street >= 0 ? core.street : map.start;
}

/** The scene: hotspots in the list's order and the main routes between them. */
export function buildTownMapScene(map: TownMap, view: TownPlacesView): TownMapScene {
  const hotspots = mapHotspots(map, view);
  const hub = hubCell(map);
  const tree = routeTree(map, hub);
  const routes: Point[][] = [];
  for (const h of hotspots) {
    const r = routeTo(map, tree, h.cell);
    if (r && r.length > 1) routes.push(r);
  }
  return { map, hotspots, hub, routes };
}

/**
 * Routes from one cell to any hotspot, computed once per source and kept
 * (previews while the player looks around, and the walk on the map).
 */
export class RouteCache {
  private trees = new Map<number, RouteTree>();
  constructor(readonly map: TownMap) {}
  route(from: number, to: number): Point[] | null {
    let t = this.trees.get(from);
    if (!t) {
      if (this.trees.size > 8) this.trees.clear();
      t = routeTree(this.map, from);
      this.trees.set(from, t);
    }
    return routeTo(this.map, t, to);
  }
}

/** Length of a leg (orthogonal or 45°), tiles. */
const legLength = (a: Point, b: Point) => Math.hypot(b.x - a.x, b.y - a.y);

/** Length of a polyline, tiles. */
export function routeLength(points: readonly Point[]): number {
  let s = 0;
  for (let i = 1; i < points.length; i++) s += legLength(points[i - 1] as Point, points[i] as Point);
  return s;
}

/** The point a distance along a polyline (orthogonal and 45° legs), tiles. */
export function alongRoute(points: readonly Point[], distance: number): Point {
  let left = Math.max(0, distance);
  for (let i = 1; i < points.length; i++) {
    const a = points[i - 1] as Point;
    const b = points[i] as Point;
    const len = legLength(a, b);
    if (left <= len && len > 0) {
      const t = left / len;
      return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t };
    }
    left -= len;
  }
  const last = points[points.length - 1];
  return last ? { ...last } : { x: 0, y: 0 };
}

/** A `TownPlacesView` for a bare town input (measurements of other towns; every place counts as active). */
export function viewForInput(input: TownInput): TownPlacesView {
  return {
    municipality: input.municipality,
    location: input.location,
    home_neighborhood: input.home,
    character: null,
    neighborhoods: [...input.neighborhoods]
      .sort((a, b) => a.id - b.id)
      .map((h) => ({
        id: h.id,
        name: h.name,
        kind: h.kind,
        is_home: h.id === input.home,
        in_ward: false,
        ward_number: null,
        spots: [
          ...(h.id === input.home ? [{ location: { location: { type: "home" as const }, kind_key: "home", name: "", neighborhood: h.id, neighborhood_name: h.name }, actions: ["rest"], late_night: [], services: [], verbs: [], here: false }] : []),
          { location: { location: { type: "street" as const, neighborhood: h.id }, kind_key: "streets", name: "", neighborhood: h.id, neighborhood_name: h.name }, actions: ["canvass"], late_night: [], services: [], verbs: [], here: false },
          ...input.places
            .filter((p) => p.neighborhood === h.id)
            .map((p) => ({ location: { location: { type: "place" as const, place: p.id }, kind_key: p.kind, name: p.name, neighborhood: h.id, neighborhood_name: h.name }, actions: ["lobby"], late_night: [], services: [], verbs: [], here: false })),
        ],
        top_concern_key: null,
        concern_knowledge: { level: "unknown" as const, as_of: null, stale: false, lead: null },
      })),
  };
}

