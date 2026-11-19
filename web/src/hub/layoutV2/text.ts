// The text alternative of a layout v2 town (DEC-051, TOWN_GENERATION.md §8):
// what the map shows, as Fluent keys with facts (never prose; DEC-054's rule
// applied on the client too), generated from the same plan and dressed map,
// so the words and the map always agree:
//   - the town: its districts, river, coast, bridges;
//   - each district: its type, the districts it borders and what lies
//     between (a street, the river and the bridge over it, the railway, a
//     creek, open ground), and its tags (low by the river, storm surge, …);
//   - the landmarks, by kind and district;
//   - routes from home (or the square) to every place: over which bridge,
//     and how far (short, a walk, long).
// Stage B adds the Fluent strings (content/ is out of stage A's scope); the
// keys below are the contract.
import type { Grid } from "../pathfinding";
import { townGrid, type TownMap } from "../townLayout";
import type { TownPlan } from "./plan";

export interface TextLine {
  key: string;
  args: Record<string, string | number>;
}

export interface TownText {
  summary: TextLine;
  districts: { district: number; lines: TextLine[] }[];
  landmarks: TextLine[];
  routes: TextLine[];
}

const EDGE_KEY: Record<string, string> = {
  street: "town-edge-street",
  river: "town-edge-river",
  rail: "town-edge-rail",
  creek: "town-edge-creek",
  open: "town-edge-open",
};

/** Breadth-first walk from a cell: steps to every cell and the previous cell on the way. */
function walkTree(grid: Grid, from: number): { dist: Int32Array; prev: Int32Array } {
  const N = grid.width * grid.height;
  const dist = new Int32Array(N).fill(-1);
  const prev = new Int32Array(N).fill(-1);
  const q = new Int32Array(N);
  let qt = 0;
  dist[from] = 0;
  q[qt++] = from;
  const W = grid.width;
  for (let qh = 0; qh < qt; qh++) {
    const c = q[qh] as number;
    const x = c % W;
    for (const n of [x > 0 ? c - 1 : -1, x < W - 1 ? c + 1 : -1, c - W, c + W]) {
      if (n < 0 || n >= N || (dist[n] as number) >= 0 || !((grid.cost[n] ?? 0) > 0)) continue;
      dist[n] = (dist[c] as number) + 1;
      prev[n] = c;
      q[qt++] = n;
    }
  }
  return { dist, prev };
}

/** The text alternative of a dressed v2 town. */
export function describeTown(plan: TownPlan, map: TownMap): TownText {
  const W = map.width;
  const rivers = plan.connectors.filter((c) => c.kind === "bridge" && c.crosses !== "creek");
  const summary: TextLine = {
    key: "town-layout-summary",
    args: {
      town: plan.municipality,
      districts: plan.districts.length,
      river: rivers.length > 0 ? "yes" : "no",
      coast: plan.waterSide ?? "none",
      bridges: rivers.length,
      landmarks: plan.landmarks.length,
    },
  };
  const districts = plan.districts.map((d) => {
    const lines: TextLine[] = [{ key: "town-district", args: { name: d.name, type: d.nameKey, kind: `hood-${d.kind}` } }];
    for (const n of d.neighbours) {
      const o = plan.districts[n.district];
      if (!o) continue;
      lines.push({ key: EDGE_KEY[n.edge] ?? "town-edge-street", args: { name: d.name, other: o.name, bridge: n.via } });
    }
    for (const t of d.tags) lines.push({ key: "town-district-tag", args: { name: d.name, tag: `district-tag-${t}` } });
    // Low by the river: a fact of the map (whether floods hit it is the engine's, D8).
    if (d.floodplainPm >= 300) lines.push({ key: "town-district-floodplain", args: { name: d.name } });
    return { district: d.index, lines };
  });
  const landmarks = plan.landmarks.map((l) => {
    const c = l.y * W + l.x;
    const dIdx = c >= 0 && c < map.district.length ? (map.district[c] as number) : -1;
    return { key: "town-landmark", args: { kind: `landmark-${l.kind}`, name: l.name, district: plan.districts[dIdx]?.name ?? "" } };
  });
  // Routes from home, else from the square, to every place: which bridge they cross, how far.
  const grid = townGrid(map);
  const home = map.homeBuilding !== null ? map.buildings[map.homeBuilding] : undefined;
  const from = home ? home.front : plan.square.y * W + plan.square.x + (plan.square.w >> 1);
  const { dist, prev } = walkTree(grid, from);
  const bridgeAt = new Map<number, string>();
  for (const b of rivers) for (let y = b.y; y < b.y + b.h; y++) for (let x = b.x; x < b.x + b.w; x++) bridgeAt.set(y * W + x, b.name);
  const routes: TextLine[] = [];
  for (const p of map.places) {
    const b = map.buildings[p.building];
    if (!b) continue;
    const steps = dist[b.front] as number;
    let via = "";
    for (let c = b.front; c >= 0 && !via; c = prev[c] as number) via = bridgeAt.get(c) ?? "";
    routes.push({
      key: steps < 0 ? "town-route-none" : "town-route",
      args: { from: home ? "place-home" : "town-square", to: b.name || `place-${b.kind}`, bridge: via, length: steps < 0 ? "none" : steps <= 40 ? "short" : steps <= 100 ? "walk" : "long" },
    });
  }
  return { summary, districts, landmarks, routes };
}
