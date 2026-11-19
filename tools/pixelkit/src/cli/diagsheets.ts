// Round 5 contact sheets (PIXEL_ART_TOOL.md "Round 5: diagonal streets"): the
// diagonal test town by day, at dusk and at night (2×), and close-ups of its
// junctions, crossings, wedges and the bridge (4×) by day and at night.
import type { Rgba } from "../node/png";
import { Sheet } from "../node/sheet";
import { diagScene } from "../../art/mandate/diagscene";
import type { ArtPack } from "../../art/mandate/pack";
import { renderScene, type Moment } from "./scene";

/** Close-ups: label and cells (x, y, w, h). */
const DETAILS: readonly [string, number, number, number, number][] = [
  ["radial leaving the square; a small square in the acute wedge; a back lane (1 cell a row)", 0, 8, 17, 11],
  ["X and offset junctions at Market Street; the yard in the V", 11, 14, 15, 11],
  ["zebra and level crossing on a / street (3 a row); T into the quay road", 6, 21, 12, 9],
  ["high street (\\, 4 a row): zebra, buildings stepping, the bridge", 19, 22, 16, 14],
  ["narrow / street (2 a row) across North Road; the green wedge", 22, 4, 16, 14],
  ["wide / street (4 a row): zebra, T into North Road; leaving the map", 40, 0, 16, 12],
];

function crop(src: Rgba, x: number, y: number, w: number, h: number): Rgba {
  const data = new Uint8Array(w * h * 4);
  for (let yy = 0; yy < h; yy++) data.set(src.data.subarray(((y + yy) * src.w + x) * 4, ((y + yy) * src.w + x + w) * 4), yy * w * 4);
  return { w, h, data };
}

function details(pack: ArtPack, moment: Moment, title: string): Sheet {
  const scale = 4;
  const full = renderScene(pack, diagScene(), moment, scale);
  const shots = DETAILS.map(([label, x, y, w, h]) => ({ label, img: crop(full, x * 16 * scale, y * 16 * scale, w * 16 * scale, h * 16 * scale) }));
  const cols = 2;
  const colW = Math.max(...shots.map((s) => s.img.w)) + 16;
  const rows: (typeof shots)[] = [];
  for (let i = 0; i < shots.length; i += cols) rows.push(shots.slice(i, i + cols));
  const rowH = rows.map((r) => Math.max(...r.map((s) => s.img.h)) + 24);
  const sheet = new Sheet(8 + cols * colW, 34 + rowH.reduce((a, b) => a + b, 0));
  sheet.text(title, 8, 8);
  let y = 34;
  rows.forEach((r, ri) => {
    r.forEach((s, ci) => {
      const x = 8 + ci * colW;
      sheet.text(s.label.toUpperCase(), x, y, [154, 160, 180], 1);
      sheet.pasteRgba(s.img, x, y + 12);
    });
    y += rowH[ri] as number;
  });
  return sheet;
}

export function diagSheets(pack: ArtPack, png: (name: string, img: Rgba) => void) {
  const scene = diagScene();
  for (const m of ["day", "dusk", "night"] as const) png(`diag-scene-${m}.png`, renderScene(pack, scene, m, 2));
  png("diag-details.png", details(pack, "day", "ROUND 5 DIAGONAL STREETS: JUNCTIONS, CROSSINGS, WEDGES, THE BRIDGE (4x)"));
  png("diag-details-night.png", details(pack, "night", "ROUND 5 DIAGONAL STREETS AT NIGHT (PALETTE MOODS, 22:30) (4x)"));
}
