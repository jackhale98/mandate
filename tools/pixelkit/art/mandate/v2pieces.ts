// Small hand-made maps that show each layout v2 piece in context (round 4):
// water against every kind of shore, a creek network with its crossings,
// rail with 45° pieces, a platform and a level crossing, level faces of
// every material with their ends, and every bridge kind in both axes. The
// contact sheets render them; the lints and tests draw every cell of them.
import { TERRAIN as K, type ConnectorView } from "./terrain";

export interface Piece {
  label: string;
  width: number;
  height: number;
  terrain: Uint8Array;
  elevation: Uint8Array;
  connectors: ConnectorView[];
  props: { kind: string; x: number; y: number; variant: number; blocks: boolean }[];
}

function piece(label: string, w: number, h: number, fill: number, paint: (rect: (t: number, x: number, y: number, w?: number, h?: number) => void, level: (e: number, x: number, y: number, w?: number, h?: number) => void) => void, connectors: ConnectorView[] = [], props: Piece["props"] = []): Piece {
  const terrain = new Uint8Array(w * h).fill(fill);
  const elevation = new Uint8Array(w * h);
  const set = (a: Uint8Array) => (v: number, x: number, y: number, ww = 1, hh = 1) => {
    for (let yy = y; yy < y + hh; yy++) for (let xx = x; xx < x + ww; xx++) if (xx >= 0 && yy >= 0 && xx < w && yy < h) a[yy * w + xx] = v;
  };
  paint(set(terrain), set(elevation));
  return { label, width: w, height: h, terrain, elevation, connectors, props };
}

/** Water against each kind of shore: a pond in the middle of it. */
export function shorePieces(): Piece[] {
  const shores: [string, number][] = [
    ["shore: grass bank", K.Grass],
    ["shore: reeds", K.Reeds],
    ["shore: sand", K.Sand],
    ["shore: rock", K.Rock],
    ["shore: quay", K.Quay],
    ["shore: river wall", K.Embankment],
  ];
  return shores.map(([label, t]) =>
    piece(label, 6, 6, t, (rect) => {
      rect(K.Water, 1, 1, 4, 4);
      rect(K.Water, 3, 5, 2, 1);
    }),
  );
}

/** A creek network: bends, a junction, a pool, joining a river; stepping stones, a ford, a footbridge both ways. */
export function creekPiece(): Piece {
  return piece(
    "creek, stepping stones, ford, footbridges",
    10,
    7,
    K.Grass,
    (rect) => {
      rect(K.Creek, 2, 0, 1, 4);
      rect(K.Creek, 2, 3, 5, 1);
      rect(K.Creek, 6, 0, 1, 3);
      rect(K.Creek, 4, 4, 1, 2);
      rect(K.Water, 0, 6, 10, 1);
      rect(K.Creek, 8, 1, 1, 1);
      rect(K.Path, 0, 1, 2, 1);
      rect(K.Path, 3, 1, 3, 1);
      rect(K.Path, 7, 1, 1, 1);
      rect(K.Ford, 2, 1, 1, 1);
      rect(K.Bridge, 6, 1, 1, 1);
      rect(K.Path, 1, 4, 3, 1);
      rect(K.Path, 5, 4, 2, 1);
      rect(K.Bridge, 4, 4, 1, 1);
      rect(K.Creek, 4, 5, 1, 1);
    },
    [
      { kind: "ford", x: 2, y: 1, w: 1, h: 1, axis: "ew" },
      { kind: "footbridge", x: 6, y: 1, w: 1, h: 1, axis: "ew" },
      { kind: "footbridge", x: 4, y: 4, w: 1, h: 1, axis: "ew" },
    ],
    [{ kind: "stepping_stones", x: 2, y: 2, variant: 0, blocks: false }],
  );
}

/** Rail: straight runs, 45° bends both ways, a diagonal, a platform on each side, a level crossing. */
export function railPiece(): Piece {
  return piece(
    "rail: 45 degree pieces, platforms, level crossing",
    14,
    7,
    K.Grass,
    (rect) => {
      rect(K.Rail, 0, 1, 4, 1);
      rect(K.Rail, 4, 2, 1, 1);
      rect(K.Rail, 5, 3, 1, 1);
      rect(K.Rail, 6, 4, 8, 1);
      rect(K.Pavement, 8, 3, 4, 1);
      rect(K.Pavement, 8, 5, 4, 1);
      rect(K.Road, 2, 0, 2, 1);
      rect(K.Road, 12, 0, 2, 4);
      rect(K.Road, 12, 5, 2, 2);
      rect(K.Crossing, 12, 4, 2, 1);
      // A second line rising to the north-east at 45° and ending at a buffer.
      rect(K.Rail, 0, 6, 1, 1);
      rect(K.Rail, 1, 5, 1, 1);
      rect(K.Rail, 2, 4, 1, 1);
    },
    [{ kind: "level_crossing", x: 12, y: 4, w: 2, h: 1, axis: "ns" }],
  );
}

/** Levels: an upland at level 2 and a terrace at 1 over the ground at 0, faced by a bank, a retaining wall, a rock face two tiles tall, an embankment wall and steps; a hollow at 0 inside level 1 shows the north-facing lip. */
export function levelPiece(): Piece {
  return piece(
    "levels: bank, retaining wall, cliff, embankment, steps, lips",
    14,
    9,
    K.Grass,
    (rect, level) => {
      level(2, 0, 0, 14, 3);
      level(1, 0, 3, 14, 2);
      // A paved terrace (level 1) faced by a retaining wall; a cliff of rock two tiles tall; an embankment wall.
      rect(K.Pavement, 5, 3, 5, 2);
      rect(K.Road, 5, 0, 5, 3);
      rect(K.Rock, 0, 5, 3, 2);
      level(0, 0, 5, 3, 2);
      rect(K.Embankment, 11, 5, 3, 1);
      rect(K.Steps, 7, 5, 1, 1);
      rect(K.Steps, 3, 3, 1, 1);
      rect(K.Path, 3, 6, 1, 3);
      rect(K.Steps, 3, 5, 1, 1);
      // A hollow (level 0) in the upland: its north side shows only a lip, its south side the upland's bank.
      level(0, 10, 1, 3, 1);
      rect(K.Pit, 10, 1, 3, 1);
    },
  );
}

/** Every bridge kind running east–west over a river flowing south, and north–south over a river flowing west. */
export function bridgePieces(): Piece[] {
  const out: Piece[] = [];
  for (const kind of ["bridge", "footbridge", "viaduct"] as const) {
    out.push(
      piece(`${kind} (ew)`, 9, 6, K.Grass, (rect) => {
        rect(K.Water, 3, 0, 3, 6);
        rect(K.Sand, 2, 0, 1, 6);
        rect(kind === "viaduct" ? K.Rail : kind === "footbridge" ? K.Path : K.Road, 0, 1, 9, 1);
        rect(K.Bridge, 3, 1, 3, 1);
      }, [{ kind, x: 3, y: 1, w: 3, h: 1, axis: "ew" }]),
    );
    out.push(
      piece(`${kind} (ns)`, 9, 6, K.Grass, (rect) => {
        rect(K.Water, 0, 2, 9, 3);
        rect(K.Reeds, 0, 1, 3, 1);
        rect(kind === "viaduct" ? K.Rail : kind === "footbridge" ? K.Path : K.Road, 4, 0, 1, 6);
        rect(K.Bridge, 4, 2, 1, 3);
      }, [{ kind, x: 4, y: 2, w: 1, h: 3, axis: "ns" }]),
    );
  }
  return out;
}

/** The mine: a quarry's rock face over its pit, a headframe, spoil heaps. */
export function minePiece(): Piece {
  return piece(
    "mine: quarry face, pit, headframe, spoil heaps",
    12,
    7,
    K.Grass,
    (rect, level) => {
      level(2, 0, 0, 12, 7);
      rect(K.Rock, 1, 1, 6, 2);
      rect(K.Pit, 1, 3, 6, 3);
      level(0, 1, 1, 6, 5);
      rect(K.Dirt, 8, 0, 1, 7);
    },
    [],
    [
      { kind: "headframe", x: 10, y: 3, variant: 0, blocks: true },
      { kind: "spoil_heap", x: 4, y: 5, variant: 0, blocks: true },
      { kind: "spoil_heap", x: 10, y: 6, variant: 1, blocks: true },
    ],
  );
}

export function allPieces(): Piece[] {
  return [...shorePieces(), creekPiece(), railPiece(), levelPiece(), ...bridgePieces(), minePiece()];
}
