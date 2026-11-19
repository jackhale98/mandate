// `pixelkit palette`: the palette as JSON and a swatch sheet (each ramp a row,
// darkest to lightest, its outline colour at the left, Oklab L under each).
import { hexToRgb, lightness, type Rgb8 } from "../core/color";
import type { Palette } from "../core/palette";
import { Sheet } from "../node/sheet";

export function paletteSheet(p: Palette): Sheet {
  const rgb = p.colors.map((c) => hexToRgb(c));
  const cell = 40;
  const nameW = 150;
  const maxSteps = Math.max(...p.ramps.map((r) => r.steps.length));
  const W = nameW + (maxSteps + 1) * (cell + 4) + 16;
  const H = 40 + p.ramps.length * (cell + 14) + 10;
  const s = new Sheet(W, H);
  s.text(`PALETTE: ${p.colors.length - 1} COLOURS, ${p.ramps.length} RAMPS`, 8, 10);
  p.ramps.forEach((r, i) => {
    const y = 36 + i * (cell + 14);
    s.text(r.name.replace(/_/g, " "), 8, y + cell / 2 - 5);
    const lineX = nameW;
    s.rect(lineX, y, cell, cell, rgb[r.line] as Rgb8);
    s.text("LN", lineX + 2, y + cell + 2, [154, 160, 180], 1);
    r.steps.forEach((idx, k) => {
      const x = nameW + (k + 1) * (cell + 4);
      s.rect(x, y, cell, cell, rgb[idx] as Rgb8);
      if (k === r.anchorStep) s.rect(x + cell / 2 - 2, y + cell - 6, 4, 4, lightness(rgb[idx] as Rgb8) > 0.6 ? [17, 19, 26] : [233, 234, 240]);
      s.text(`${Math.round(lightness(rgb[idx] as Rgb8) * 100)}`, x + 2, y + cell + 2, [154, 160, 180], 1);
    });
  });
  return s;
}
