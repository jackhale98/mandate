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
import { renderScene } from "./scene";

/** Close-ups in three sheets (facing s and n; e and w; se and ne): label and cells (x, y, w, h). Widths are road cells a row. */
export const END_DETAILS: Readonly<Record<"sn" | "ew" | "diag", readonly [string, number, number, number, number][]>> = {
  sn: [
    ["turning circles: s (3, an island), n (2)", 4, 3, 12, 22],
    ["hammerheads: s (an L, 4), n (a T, 3)", 16, 3, 12, 22],
    ["bollards: s (2), n (4, lit); a footpath between", 28, 3, 12, 22],
    ["plaza: s (3), n (2) run into a square", 40, 3, 12, 22],
    ["field gates: s (4), n (3, steel); a track between", 52, 3, 12, 22],
    ["railings: s (2), n (4) at a basin", 64, 3, 12, 22],
  ],
  ew: [
    ["turning circles: e (4, an island), w (2)", 0, 28, 38, 11],
    ["hammerheads: e (a T, 2), w (an L, 3)", 38, 28, 42, 11],
    ["bollards: e (3, lit), w (4)", 0, 39, 38, 11],
    ["plaza: e (4), w (2)", 38, 39, 42, 11],
    ["field gates: e (2), w (3)", 0, 50, 38, 11],
    ["railings: e (3), w (4)", 38, 50, 42, 11],
  ],
  diag: [
    ["turning circles: se (4, an island), ne (3)", 0, 61, 15, 32],
    ["hammerheads: se (2), ne (4)", 14, 61, 14, 32],
    ["bollards: se (3), ne (2, lit)", 27, 61, 14, 32],
    ["plaza: se (4), ne (3)", 40, 61, 14, 32],
    ["field gates: se (2), ne (4, steel)", 53, 61, 14, 32],
    ["railings: se (3), ne (2)", 66, 61, 14, 32],
  ],
};

function crop(src: Rgba, x: number, y: number, w: number, h: number): Rgba {
  const cw = Math.max(0, Math.min(w, src.w - x));
  const ch = Math.max(0, Math.min(h, src.h - y));
  const data = new Uint8Array(cw * ch * 4);
  for (let yy = 0; yy < ch; yy++) data.set(src.data.subarray(((y + yy) * src.w + x) * 4, ((y + yy) * src.w + x + cw) * 4), yy * cw * 4);
  return { w: cw, h: ch, data };
}

function details(full: Rgba, list: readonly [string, number, number, number, number][], perRow: number, title: string): Sheet {
  const scale = 4;
  const shots = list.map(([label, x, y, w, h]) => ({ label, img: crop(full, x * 16 * scale, y * 16 * scale, w * 16 * scale, h * 16 * scale) }));
  const rows: (typeof shots)[] = [];
  for (let i = 0; i < shots.length; i += perRow) rows.push(shots.slice(i, i + perRow));
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
  for (const m of ["day", "night"] as const) {
    const full = renderScene(pack, scene, m, 4);
    const when = m === "day" ? "" : " AT NIGHT (PALETTE MOODS, 22:30)";
    const sfx = m === "day" ? "" : "-night";
    png(`ends-details-sn${sfx}.png`, details(full, END_DETAILS.sn, 6, `ROUND 6 STREET ENDS FACING S (TOP) AND N (BOTTOM)${when} (4x)`));
    png(`ends-details-ew${sfx}.png`, details(full, END_DETAILS.ew, 2, `ROUND 6 STREET ENDS FACING E (LEFT) AND W (RIGHT)${when} (4x)`));
    png(`ends-details-diag${sfx}.png`, details(full, END_DETAILS.diag, 6, `ROUND 6 STREET ENDS ON 45° STREETS, FACING SE (TOP) AND NE (BOTTOM)${when} (4x)`));
  }
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
