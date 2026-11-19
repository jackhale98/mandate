// Round 6 contact sheets (PIXEL_ART_TOOL.md "Round 6: street ends"): the
// street-end test town by day, at dusk and at night (2×), close-ups of every
// kind of end (4×) by day and at night, and the three new props.
import type { Rgba } from "../node/png";
import { Sheet } from "../node/sheet";
import { hexToRgb } from "../core/color";
import { endScene } from "../../art/mandate/endscene";
import { END_FACINGS } from "../../art/mandate/ends";
import type { ArtPack } from "../../art/mandate/pack";
import { gridSheet, type Cell } from "./sheets";
import { renderScene, type Moment } from "./scene";

/** Close-ups: label and cells (x, y, w, h). */
export const END_DETAILS: readonly [string, number, number, number, number][] = [
  ["turning circles: s (2 a row, top) and n (4 a row, island)", 4, 3, 12, 22],
  ["hammerheads: s (an L, 3 a row) and n (a T, 2 a row)", 16, 3, 12, 22],
  ["bollards: s and n, a footpath through a green between them", 28, 3, 12, 22],
  ["plaza: s and n run into a square", 40, 3, 12, 22],
  ["field gates: s and n, a track across the field between them", 52, 3, 12, 22],
  ["railings: s and n at a basin", 64, 3, 12, 22],
  ["turning circles e and w; hammerheads e and w", 0, 28, 40, 11],
  ["bollards e and w (lit); plaza e and w", 38, 28, 42, 11],
  ["field gates e and w; railings e and w", 0, 39, 80, 11],
  ["diagonal ends (se above, ne below): turning circles, hammerheads, bollards", 0, 61, 40, 28],
  ["diagonal ends: plaza, field gates, railings", 39, 61, 41, 28],
];

function crop(src: Rgba, x: number, y: number, w: number, h: number): Rgba {
  const cw = Math.max(0, Math.min(w, src.w - x));
  const ch = Math.max(0, Math.min(h, src.h - y));
  const data = new Uint8Array(cw * ch * 4);
  for (let yy = 0; yy < ch; yy++) data.set(src.data.subarray(((y + yy) * src.w + x) * 4, ((y + yy) * src.w + x + cw) * 4), yy * cw * 4);
  return { w: cw, h: ch, data };
}

function details(pack: ArtPack, moment: Moment, title: string): Sheet {
  const scale = 4;
  const full = renderScene(pack, endScene(), moment, scale);
  const shots = END_DETAILS.map(([label, x, y, w, h]) => ({ label, img: crop(full, x * 16 * scale, y * 16 * scale, w * 16 * scale, h * 16 * scale) }));
  // Rows: the six orthogonal pairs side by side, then the wide crops one a row.
  const rows: (typeof shots)[] = [shots.slice(0, 6), ...shots.slice(6).map((s) => [s])];
  const rowW = rows.map((r) => r.reduce((a, s) => a + s.img.w + 16, 0));
  const rowH = rows.map((r) => Math.max(...r.map((s) => s.img.h)) + 24);
  const sheet = new Sheet(8 + Math.max(...rowW), 34 + rowH.reduce((a, b) => a + b, 0));
  sheet.text(title, 8, 8);
  let y = 34;
  rows.forEach((r, ri) => {
    let x = 8;
    r.forEach((s) => {
      sheet.text(s.label.toUpperCase(), x, y, [154, 160, 180], 1);
      sheet.pasteRgba(s.img, x, y + 12);
      x += s.img.w + 16;
    });
    y += rowH[ri] as number;
  });
  return sheet;
}

export function endSheets(pack: ArtPack, png: (name: string, img: Rgba) => void) {
  const scene = endScene();
  for (const m of ["day", "dusk", "night"] as const) png(`ends-scene-${m}.png`, renderScene(pack, scene, m, 2));
  png("ends-details.png", details(pack, "day", "ROUND 6 STREET ENDS: EVERY KIND IN ALL FOUR ORTHOGONAL FACINGS AND TWO DIAGONAL ONES (4x)"));
  png("ends-details-night.png", details(pack, "night", "ROUND 6 STREET ENDS AT NIGHT (PALETTE MOODS, 22:30) (4x)"));
  // The new props, every variant, on the pavement.
  const rgb = pack.palette.colors.map(hexToRgb);
  const cells: Cell[] = [];
  for (const kind of ["bollards", "field_gate", "railing"]) {
    (pack.props[kind] ?? []).forEach((p, i) => {
      cells.push({ img: p.albedo, label: `${kind.replace("_", " ")} ${END_FACINGS[i % 8]}${i >= 8 ? (kind === "bollards" ? " lit" : " steel") : ""}` });
      if (p.emissive) cells.push({ img: p.emissive, label: "lit" });
    });
  }
  const r = pack.palette.ramps.find((x) => x.name === "paving");
  const pavement = hexToRgb(pack.palette.colors[r ? (r.steps[r.anchorStep] as number) : 1] as string);
  png("ends-props.png", gridSheet("ROUND 6 PROPS: BOLLARDS, FIELD GATES AND RAILINGS PER FACING (VARIANT = FACING; +8 THE SECOND LOOK)", cells, rgb, { cols: 8, scale: 4, ground: pavement }));
}
