// A hand-authored town in the layout v2 vocabulary (round 4): every new
// terrain code, two levels (an upland at level 2 over the town at level 0,
// with a bank, a retaining wall, a cliff, steps and a weir between them), a
// quarry with its rock face, a headframe and a spoil heap; a river with a
// stone bridge running east–west and one running north–south, a brick
// viaduct and a rail bridge over a creek; a creek with stepping stones, a
// ford and a footbridge; a railway with 45° pieces, a platform and a level
// crossing; reeds, a beach, a rock shore and an urban river wall; an
// open-air market; and plots built, boarded, derelict, under construction
// and reserved. The contact sheets render it by day and at night, and the
// web tests draw every cell of it through the art provider.
import type { FacadeSpec } from "./buildings";
import { TERRAIN as K, type ConnectorView, type TerrainView } from "./terrain";

export interface SceneBuilding {
  x: number;
  y: number;
  spec: FacadeSpec;
  /** The plot state (`TownBuilding.state`); the spec's conditions carry it too. */
  state: "built" | "reserved" | "construction" | "boarded" | "derelict";
}

export interface SceneProp {
  kind: string;
  x: number;
  y: number;
  variant: number;
  blocks: boolean;
}

export interface Scene extends TerrainView {
  terrain: Uint8Array;
  elevation: Uint8Array;
  connectors: (ConnectorView & { name: string })[];
  buildings: SceneBuilding[];
  props: SceneProp[];
  /** Where townsfolk stand in the renders: cell and look seed. */
  people: { x: number; y: number; seed: number }[];
}

export const SCENE_W = 40;
export const SCENE_H = 28;

export function v2Scene(): Scene {
  const W = SCENE_W;
  const H = SCENE_H;
  const terrain = new Uint8Array(W * H).fill(K.Grass);
  const elevation = new Uint8Array(W * H);
  const rect = (t: number, x: number, y: number, w = 1, h = 1) => {
    for (let yy = y; yy < y + h; yy++) for (let xx = x; xx < x + w; xx++) if (xx >= 0 && yy >= 0 && xx < W && yy < H) terrain[yy * W + xx] = t;
  };
  const level = (e: number, x: number, y: number, w = 1, h = 1) => {
    for (let yy = y; yy < y + h; yy++) for (let xx = x; xx < x + w; xx++) if (xx >= 0 && yy >= 0 && xx < W && yy < H) elevation[yy * W + xx] = e;
  };

  // ---- the upland (level 2): fields, a lane, the mine
  level(2, 0, 0, W, 9);
  rect(K.Field, 10, 1, 11, 4);
  rect(K.Dirt, 0, 7, 26, 1);
  rect(K.Dirt, 12, 8, 1, 1);
  rect(K.Garden, 21, 3, 4, 1);
  // The quarry: a rock face two tiles tall under the upland's edge, a pit floor below it, all at level 0.
  rect(K.Rock, 31, 1, 7, 2);
  rect(K.Pit, 31, 3, 7, 3);
  level(0, 31, 1, 7, 5);
  rect(K.Rock, 30, 0, 1, 1);
  rect(K.Path, 38, 4, 1, 4);

  // ---- the river: north–south through the upland (a weir where it drops to the town), then west along the town
  rect(K.Water, 26, 0, 3, 21);
  rect(K.Water, 0, 18, 29, 3);
  // Banks: reeds upstream on the north bank, a beach and a rock shore on the south bank, an urban river wall by the quay.
  rect(K.Reeds, 0, 17, 7, 1);
  rect(K.Reeds, 25, 2, 1, 4);
  rect(K.Sand, 0, 21, 6, 1);
  rect(K.Rock, 6, 21, 3, 1);
  rect(K.Quay, 16, 16, 10, 1);
  rect(K.Embankment, 16, 17, 10, 1);
  rect(K.Pier, 11, 18, 1, 2);

  // ---- the face between the levels (row 9): a grass bank, a cliff, a retaining wall above the platform, steps
  rect(K.Rock, 0, 9, 4, 2);
  rect(K.Embankment, 17, 9, 7, 1);
  rect(K.Steps, 12, 9, 1, 1);

  // ---- the creek: from the upland down into the river, with stepping stones, a rail bridge, a ford and a footbridge
  rect(K.Creek, 8, 0, 1, 18);
  rect(K.Path, 0, 14, 8, 1);
  rect(K.Path, 9, 14, 5, 1);
  rect(K.Ford, 8, 14, 1, 1);
  rect(K.Path, 6, 16, 2, 1);
  rect(K.Path, 9, 16, 3, 1);
  rect(K.Bridge, 8, 16, 1, 1);

  // ---- the railway: west to east at row 11, a viaduct over the river, 45° pieces down to row 13 in the east
  rect(K.Rail, 4, 11, 22, 1);
  rect(K.Bridge, 8, 11, 1, 1);
  rect(K.Bridge, 26, 11, 3, 1);
  rect(K.Rail, 29, 11, 4, 1);
  rect(K.Rail, 33, 12, 1, 1);
  rect(K.Rail, 34, 13, 6, 1);
  // West of the cliff the line climbs away north-west at 45° (two diagonal cells).
  rect(K.Rail, 3, 12, 1, 1);
  rect(K.Rail, 2, 13, 1, 1);
  rect(K.Rail, 0, 13, 2, 1);
  // The platform (a paved strip north of the line, under the retaining wall), a road along the wall's foot.
  rect(K.Pavement, 17, 10, 7, 1);
  rect(K.Road, 9, 10, 7, 1);

  // ---- the town (level 0): a north–south road over a level crossing and a bridge, an east–west road over the river
  rect(K.Road, 14, 10, 2, 18);
  rect(K.Crossing, 14, 11, 2, 1);
  rect(K.Pavement, 16, 14, 10, 1);
  rect(K.Road, 16, 15, 24, 1);
  rect(K.Road, 26, 15, 3, 1);
  rect(K.Bridge, 26, 15, 3, 1);
  rect(K.Pavement, 29, 14, 11, 1);
  rect(K.Pavement, 29, 16, 1, 2);
  rect(K.Bridge, 14, 18, 2, 3);
  rect(K.Pavement, 12, 12, 2, 6);
  rect(K.Pavement, 16, 21, 24, 1);
  rect(K.Pavement, 0, 25, 40, 1);
  rect(K.Road, 0, 26, 40, 2);
  rect(K.Road, 14, 21, 2, 5);
  // An open-air market on a small square by the bridge.
  rect(K.Plaza, 30, 18, 7, 3);
  rect(K.Park, 37, 16, 3, 5);

  // ---- buildings (footprints are Building, doors Door; doors face south)
  const buildings: SceneBuilding[] = [];
  const base = { named: false, home: false, landmark: false, headroom: 16, ends: { left: "gap" as const, right: "gap" as const } };
  const put = (x: number, y: number, w: number, h: number, kind: string, hood: string, state: SceneBuilding["state"], seed: number, doorX = 1, headroom = 16) => {
    rect(K.Building, x, y, w, h);
    rect(K.Door, x + doorX, y + h - 1, 1, 1);
    const conditions = state === "built" ? [] : [state];
    buildings.push({ x, y, state, spec: { ...base, kind, hood, w, h, doorX, storeys: h >= 3 ? 2 : 1, visualSeed: seed, conditions, headroom } });
  };
  put(17, 13, 3, 2, "house", "inner", "built", 4101, 1, 0);
  put(21, 13, 4, 2, "shop", "inner", "boarded", 4102, 1, 0);
  put(30, 16, 4, 2, "terrace", "inner", "derelict", 4103);
  put(34, 16, 3, 2, "terrace", "inner", "built", 4104);
  put(1, 22, 4, 3, "house", "suburb", "built", 4105);
  put(17, 22, 4, 3, "shop", "inner", "boarded", 4106);
  put(21, 22, 4, 3, "terrace", "inner", "derelict", 4107);
  put(25, 22, 4, 3, "terrace", "inner", "construction", 4108);
  put(29, 22, 3, 3, "house", "inner", "reserved", 4109);
  put(32, 22, 4, 3, "shop", "core", "built", 4110);
  put(36, 22, 4, 3, "shop", "core", "derelict", 4111);
  put(20, 5, 4, 2, "farmhouse", "rural", "built", 4112);
  put(33, 7, 4, 2, "mine", "industrial", "built", 4113, 2);
  put(9, 22, 4, 3, "warehouse", "industrial", "construction", 4114, 2);

  const connectors: Scene["connectors"] = [
    { kind: "viaduct", x: 26, y: 11, w: 3, h: 1, axis: "ew", name: "the Viaduct" },
    { kind: "viaduct", x: 8, y: 11, w: 1, h: 1, axis: "ew", name: "" },
    { kind: "bridge", x: 26, y: 15, w: 3, h: 1, axis: "ew", name: "the Old Bridge" },
    { kind: "bridge", x: 14, y: 18, w: 2, h: 3, axis: "ns", name: "the New Bridge" },
    { kind: "footbridge", x: 8, y: 16, w: 1, h: 1, axis: "ew", name: "" },
    { kind: "ford", x: 8, y: 14, w: 1, h: 1, axis: "ew", name: "the Ford" },
    { kind: "level_crossing", x: 14, y: 11, w: 2, h: 1, axis: "ns", name: "" },
  ];

  const props: SceneProp[] = [
    { kind: "headframe", x: 38, y: 2, variant: 0, blocks: true },
    { kind: "spoil_heap", x: 3, y: 5, variant: 2, blocks: true },
    { kind: "spoil_heap", x: 33, y: 5, variant: 0, blocks: true },
    { kind: "stepping_stones", x: 8, y: 5, variant: 0, blocks: false },
    { kind: "reeds", x: 2, y: 17, variant: 0, blocks: false },
    { kind: "reeds", x: 5, y: 17, variant: 1, blocks: false },
    { kind: "reeds", x: 25, y: 4, variant: 1, blocks: false },
    { kind: "stall", x: 31, y: 19, variant: 1, blocks: true },
    { kind: "stall", x: 33, y: 19, variant: 2, blocks: true },
    { kind: "stall", x: 35, y: 19, variant: 3, blocks: true },
    { kind: "stall", x: 31, y: 20, variant: 4, blocks: true },
    { kind: "stall", x: 33, y: 20, variant: 5, blocks: true },
    { kind: "lamp", x: 18, y: 10, variant: 0, blocks: false },
    { kind: "lamp", x: 22, y: 10, variant: 0, blocks: false },
    { kind: "lamp", x: 25, y: 14, variant: 0, blocks: false },
    { kind: "lamp", x: 29, y: 16, variant: 0, blocks: false },
    { kind: "lamp", x: 13, y: 17, variant: 0, blocks: false },
    { kind: "lamp", x: 16, y: 25, variant: 0, blocks: false },
    { kind: "lamp", x: 30, y: 21, variant: 0, blocks: false },
    { kind: "lamp", x: 36, y: 21, variant: 0, blocks: false },
    { kind: "tree", x: 1, y: 2, variant: 0, blocks: true },
    { kind: "tree", x: 6, y: 2, variant: 1, blocks: true },
    { kind: "spoil_heap", x: 30, y: 5, variant: 1, blocks: true },
    { kind: "pine", x: 23, y: 1, variant: 0, blocks: true },
    { kind: "tree", x: 38, y: 18, variant: 0, blocks: true },
    { kind: "bench", x: 37, y: 20, variant: 0, blocks: false },
  ];

  const people = [
    { x: 19, y: 10, seed: 3 },
    { x: 15, y: 13, seed: 8 },
    { x: 27, y: 15, seed: 11 },
    { x: 32, y: 21, seed: 14 },
    { x: 34, y: 18, seed: 5 },
    { x: 12, y: 8, seed: 17 },
    { x: 15, y: 19, seed: 20 },
    { x: 24, y: 25, seed: 9 },
  ];

  return { width: W, height: H, terrain, elevation, connectors, buildings, props, people };
}
