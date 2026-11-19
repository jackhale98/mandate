// Round 4 contact sheets (PIXEL_ART_TOOL.md "Round 4: v2 terrain"): the v2
// test scene by day, at dusk and at night; the plot states of every building
// family; the new props; and the v2 terrain pieces.
import { hexToRgb } from "../core/color";
import type { Rgba } from "../node/png";
import { Sheet } from "../node/sheet";
import { PLOT_STATES } from "../../art/mandate/buildings";
import { FAMILY_SPECS } from "../../art/mandate/samples";
import type { ArtPack } from "../../art/mandate/pack";
import { TERRAIN as K } from "../../art/mandate/terrain";
import { bridgePieces, creekPiece, levelPiece, minePiece, railPiece, shorePieces, type Piece } from "../../art/mandate/v2pieces";
import { v2Scene, type SceneBuilding } from "../../art/mandate/v2scene";
import { renderScene, type Moment } from "./scene";
import { gridSheet, type Cell } from "./sheets";

const V2_PROPS = ["headframe", "spoil_heap", "reeds", "stepping_stones", "stall"] as const;

/** Every family in every plot state on one street block per row. */
function plotScene() {
  const cols = PLOT_STATES.length;
  const W = cols * 6;
  const H = FAMILY_SPECS.length * 7;
  const terrain = new Uint8Array(W * H).fill(K.Grass);
  const buildings: SceneBuilding[] = [];
  FAMILY_SPECS.forEach(([, spec], r) => {
    const y = r * 7 + 3;
    for (let x = 0; x < W; x++) terrain[(y + 3) * W + x] = K.Pavement;
    PLOT_STATES.forEach((state, ci) => {
      const x = ci * 6;
      for (let yy = y; yy < y + spec.h; yy++) for (let xx = x; xx < x + spec.w; xx++) terrain[yy * W + xx] = K.Building;
      buildings.push({ x, y, state, spec: { ...spec, conditions: state === "built" ? [] : [state] } });
    });
  });
  return { width: W, height: H, terrain, buildings };
}

function labelled(s: Sheet, labels: readonly [string, number, number][]): Sheet {
  for (const [t, x, y] of labels) {
    s.rect(x - 2, y - 2, t.length * 8 + 4, 14, [24, 26, 34]);
    s.text(t, x, y, [233, 234, 240], 2);
  }
  return s;
}

/** Mini maps rendered at a scale, labelled, in rows of a sheet at most `maxW` px wide. */
function board(pack: ArtPack, title: string, pieces: readonly Piece[], scale: number, moment: Moment = "day", maxW = 1800): Sheet {
  const shots = pieces.map((p) => ({ p, img: renderScene(pack, p, moment, scale) }));
  const rows: (typeof shots)[] = [[]];
  let x = 8;
  for (const s of shots) {
    if (x + s.img.w > maxW && (rows[rows.length - 1] as typeof shots).length) {
      rows.push([]);
      x = 8;
    }
    (rows[rows.length - 1] as typeof shots).push(s);
    x += s.img.w + 12;
  }
  const rowH = rows.map((r) => Math.max(...r.map((s) => s.img.h)) + 22);
  const W = Math.max(...rows.map((r) => r.reduce((a, s) => a + s.img.w + 12, 8)));
  const sheet = new Sheet(Math.max(W, title.length * 8 + 16), 30 + rowH.reduce((a, b) => a + b, 0));
  sheet.text(title, 8, 8);
  let y = 30;
  rows.forEach((r, ri) => {
    let xx = 8;
    for (const s of r) {
      sheet.text(s.p.label, xx, y, [154, 160, 180], 1);
      sheet.pasteRgba(s.img, xx, y + 10);
      xx += s.img.w + 12;
    }
    y += rowH[ri] as number;
  });
  return sheet;
}

export function v2Sheets(pack: ArtPack, png: (name: string, img: Rgba) => void) {
  const rgb = pack.palette.colors.map(hexToRgb);
  const scene = v2Scene();
  for (const m of ["day", "dusk", "night"] as const) png(`v2-scene-${m}.png`, renderScene(pack, scene, m, 2));

  // ---- plot states, by day and at night (palette moods: vacant buildings stay dark)
  const ps = plotScene();
  const labels: [string, number, number][] = [];
  PLOT_STATES.forEach((st, ci) => labels.push([st, ci * 6 * 32 + 4, 4]));
  FAMILY_SPECS.forEach(([fam], r) => labels.push([fam, 4, (r * 7 + 6) * 32 + 8]));
  for (const m of ["day", "night"] as Moment[]) png(`v2-plot-states-${m}.png`, labelled(renderScene(pack, ps, m, 2), labels));

  // ---- terrain pieces and bridge kits
  const pieces = [...shorePieces(), creekPiece(), railPiece(), levelPiece(), minePiece()];
  png("v2-terrain.png", board(pack, "ROUND 4 TERRAIN: SHORES, CREEK AND CROSSINGS, RAIL, LEVELS, THE MINE (3x)", pieces, 3));
  png("v2-terrain-night.png", board(pack, "ROUND 4 TERRAIN AT NIGHT (PALETTE MOODS, 22:30)", pieces, 3, "night"));
  png("v2-bridges.png", board(pack, "ROUND 4 BRIDGE KITS: STONE BRIDGE, TIMBER FOOTBRIDGE, BRICK VIADUCT; EAST-WEST (SOUTH FACE) AND NORTH-SOUTH (DECK, SHADOW) (3x)", bridgePieces(), 3, "day", 1400));

  // ---- props
  const cells: Cell[] = [];
  for (const kind of V2_PROPS) (pack.props[kind] ?? []).forEach((p, i) => cells.push({ img: p.albedo, label: `${kind.replace("_", " ")} ${i}` }));
  const grass = rgb[pack.terrain.legend.g as number] ?? [94, 150, 70];
  png("v2-props.png", gridSheet("ROUND 4 PROPS: HEADFRAME, SPOIL HEAPS (3 SIZES), REEDS, STEPPING STONES, MARKET STALLS (ROUND 1, FOUR GOODS, CLOSED)", cells, rgb, { cols: 7, scale: 4, ground: grass }));
}
