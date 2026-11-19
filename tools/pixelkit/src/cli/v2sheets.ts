// Round 4 contact sheets (PIXEL_ART_TOOL.md "Round 4: v2 terrain"): the v2
// test scene by day, at dusk and at night; the plot states of every building
// family; the new props; and the v2 terrain pieces.
import { hexToRgb } from "../core/color";
import type { Rgba } from "../node/png";
import type { Sheet } from "../node/sheet";
import { PLOT_STATES, type FacadeSpec } from "../../art/mandate/buildings";
import type { ArtPack } from "../../art/mandate/pack";
import { TERRAIN as K } from "../../art/mandate/terrain";
import { v2Scene, type SceneBuilding } from "../../art/mandate/v2scene";
import { renderScene, type Moment } from "./scene";
import { gridSheet, type Cell } from "./sheets";

const V2_PROPS = ["headframe", "spoil_heap", "reeds", "stepping_stones", "stall"] as const;

/** One building of each family, for the plot-state sheet. */
export const FAMILY_SPECS: readonly (readonly [string, FacadeSpec])[] = (() => {
  const base: FacadeSpec = { kind: "terrace", hood: "inner", w: 3, h: 3, doorX: 1, storeys: 2, visualSeed: 7001, named: false, home: false, landmark: false, conditions: [], headroom: 32, ends: { left: "gap", right: "gap" } };
  return [
    ["civic", { ...base, kind: "town_hall", hood: "core", w: 5, doorX: 2, storeys: 3, named: true, landmark: true, visualSeed: 7002 }],
    ["shop", { ...base, kind: "shop", w: 4, visualSeed: 7003 }],
    ["house", { ...base, kind: "terrace", visualSeed: 7004 }],
    ["industrial", { ...base, kind: "warehouse", hood: "industrial", w: 5, doorX: 2, visualSeed: 7005 }],
    ["institution", { ...base, kind: "school", hood: "suburb", w: 5, doorX: 2, named: true, visualSeed: 7006 }],
  ] as const;
})();

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

  // ---- props
  const cells: Cell[] = [];
  for (const kind of V2_PROPS) (pack.props[kind] ?? []).forEach((p, i) => cells.push({ img: p.albedo, label: `${kind.replace("_", " ")} ${i}` }));
  const grass = rgb[pack.terrain.legend.g as number] ?? [94, 150, 70];
  png("v2-props.png", gridSheet("ROUND 4 PROPS: HEADFRAME, SPOIL HEAPS (3 SIZES), REEDS, STEPPING STONES, MARKET STALLS (ROUND 1, FOUR GOODS, CLOSED)", cells, rgb, { cols: 7, scale: 4, ground: grass }));
}
