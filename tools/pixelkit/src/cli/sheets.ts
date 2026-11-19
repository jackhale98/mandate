// Contact sheets for review (§7.3): parts, townsfolk, buildings, tiles, props.
import type { Rgb8 } from "../core/color";
import type { Indexed } from "../core/image";
import { Sheet, textWidth } from "../node/sheet";

export interface Cell {
  img: Indexed;
  label?: string;
}

/** A grid of images at a scale, with labels under them, over an optional ground colour. */
export function gridSheet(title: string, cells: readonly Cell[], rgb: readonly Rgb8[], o: { cols: number; scale: number; ground?: Rgb8; checker?: boolean; pad?: number; labelScale?: number }): Sheet {
  const pad = o.pad ?? 8;
  const ls = o.labelScale ?? 1;
  const cw = Math.max(...cells.map((c) => c.img.w)) * o.scale;
  const ch = Math.max(...cells.map((c) => c.img.h)) * o.scale;
  const labelW = Math.max(0, ...cells.map((c) => textWidth(c.label ?? "", ls)));
  const colW = Math.max(cw, labelW) + pad;
  const rowH = ch + (cells.some((c) => c.label) ? 8 * ls : 0) + pad;
  const rows = Math.ceil(cells.length / o.cols);
  const s = new Sheet(colW * o.cols + pad, 30 + rowH * rows + pad);
  s.text(title, pad, 8);
  cells.forEach((c, i) => {
    const x = pad + (i % o.cols) * colW;
    const y = 30 + Math.floor(i / o.cols) * rowH;
    if (o.ground) s.rect(x, y, c.img.w * o.scale, c.img.h * o.scale, o.ground);
    s.paste(c.img, rgb, x, y, o.scale, o.checker ?? false);
    if (c.label) s.text(c.label, x, y + c.img.h * o.scale + 2, [154, 160, 180], ls);
  });
  return s;
}
