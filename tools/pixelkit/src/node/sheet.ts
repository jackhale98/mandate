// Contact sheets: RGBA canvases in memory that indexed images are pasted onto
// at a whole-number scale, with labels in a tiny built-in 3×5 font, written
// as PNG. For review only (the art itself never contains text, R14).
import type { Rgb8 } from "../core/color";
import type { Indexed } from "../core/image";
import type { Rgba } from "./png";

export class Sheet implements Rgba {
  readonly data: Uint8Array;
  readonly w: number;
  readonly h: number;
  constructor(w: number, h: number, bg: Rgb8 = [24, 26, 34]) {
    this.w = w;
    this.h = h;
    this.data = new Uint8Array(w * h * 4);
    for (let i = 0; i < w * h; i++) {
      this.data[i * 4] = bg[0];
      this.data[i * 4 + 1] = bg[1];
      this.data[i * 4 + 2] = bg[2];
      this.data[i * 4 + 3] = 255;
    }
  }

  rect(x: number, y: number, w: number, h: number, c: Rgb8, alpha = 1) {
    for (let yy = Math.max(0, y); yy < Math.min(this.h, y + h); yy++) {
      for (let xx = Math.max(0, x); xx < Math.min(this.w, x + w); xx++) this.px(xx, yy, c, alpha);
    }
  }

  px(x: number, y: number, c: Rgb8, alpha = 1) {
    if (x < 0 || y < 0 || x >= this.w || y >= this.h) return;
    const i = (y * this.w + x) * 4;
    const d = this.data;
    d[i] = Math.round((d[i] as number) * (1 - alpha) + c[0] * alpha);
    d[i + 1] = Math.round((d[i + 1] as number) * (1 - alpha) + c[1] * alpha);
    d[i + 2] = Math.round((d[i + 2] as number) * (1 - alpha) + c[2] * alpha);
  }

  /** Paste an indexed image through a palette at a scale; transparent pixels show a checker if `checker`. */
  paste(img: Indexed, rgb: readonly Rgb8[], x: number, y: number, scale = 1, checker = false) {
    for (let iy = 0; iy < img.h; iy++) {
      for (let ix = 0; ix < img.w; ix++) {
        const v = img.px[iy * img.w + ix] as number;
        if (v === 0) {
          if (checker) this.rect(x + ix * scale, y + iy * scale, scale, scale, ((ix + iy) & 1) === 0 ? [44, 48, 62] : [36, 40, 52]);
          continue;
        }
        this.rect(x + ix * scale, y + iy * scale, scale, scale, rgb[v] ?? [255, 0, 255]);
      }
    }
  }

  /** Paste another RGBA image (a screenshot) at (x, y), scaled down by an integer factor (nearest). */
  pasteRgba(src: Rgba, x: number, y: number, down = 1) {
    for (let sy = 0; sy < Math.floor(src.h / down); sy++) {
      for (let sx = 0; sx < Math.floor(src.w / down); sx++) {
        const i = (sy * down * src.w + sx * down) * 4;
        this.px(x + sx, y + sy, [src.data[i] as number, src.data[i + 1] as number, src.data[i + 2] as number]);
      }
    }
  }

  text(s: string, x: number, y: number, c: Rgb8 = [233, 234, 240], scale = 2) {
    let cx = x;
    for (const ch of s.toUpperCase()) {
      const g = FONT[ch] ?? FONT["?"];
      if (ch === " ") {
        cx += 4 * scale;
        continue;
      }
      g?.forEach((row, ry) => {
        for (let rx = 0; rx < 3; rx++) if (row & (4 >> rx)) this.rect(cx + rx * scale, y + ry * scale, scale, scale, c);
      });
      cx += 4 * scale;
    }
  }
}

/** Text width in px for `Sheet.text`. */
export const textWidth = (s: string, scale = 2) => s.length * 4 * scale;

// 3×5 glyphs, rows top to bottom, bits left to right (4, 2, 1).
const FONT: Record<string, readonly number[]> = {
  A: [2, 5, 7, 5, 5],
  B: [6, 5, 6, 5, 6],
  C: [3, 4, 4, 4, 3],
  D: [6, 5, 5, 5, 6],
  E: [7, 4, 6, 4, 7],
  F: [7, 4, 6, 4, 4],
  G: [3, 4, 5, 5, 3],
  H: [5, 5, 7, 5, 5],
  I: [7, 2, 2, 2, 7],
  J: [1, 1, 1, 5, 2],
  K: [5, 5, 6, 5, 5],
  L: [4, 4, 4, 4, 7],
  M: [5, 7, 7, 5, 5],
  N: [6, 5, 5, 5, 5],
  O: [2, 5, 5, 5, 2],
  P: [6, 5, 6, 4, 4],
  Q: [2, 5, 5, 6, 3],
  R: [6, 5, 6, 5, 5],
  S: [3, 4, 2, 1, 6],
  T: [7, 2, 2, 2, 2],
  U: [5, 5, 5, 5, 7],
  V: [5, 5, 5, 5, 2],
  W: [5, 5, 7, 7, 5],
  X: [5, 5, 2, 5, 5],
  Y: [5, 5, 2, 2, 2],
  Z: [7, 1, 2, 4, 7],
  "0": [7, 5, 5, 5, 7],
  "1": [2, 6, 2, 2, 7],
  "2": [6, 1, 2, 4, 7],
  "3": [6, 1, 2, 1, 6],
  "4": [5, 5, 7, 1, 1],
  "5": [7, 4, 6, 1, 6],
  "6": [3, 4, 7, 5, 7],
  "7": [7, 1, 2, 2, 2],
  "8": [7, 5, 7, 5, 7],
  "9": [7, 5, 7, 1, 6],
  "-": [0, 0, 7, 0, 0],
  _: [0, 0, 0, 0, 7],
  ".": [0, 0, 0, 0, 2],
  ":": [0, 2, 0, 2, 0],
  "/": [1, 1, 2, 4, 4],
  "×": [0, 5, 2, 5, 0],
  "(": [1, 2, 2, 2, 1],
  ")": [4, 2, 2, 2, 4],
  "#": [5, 7, 5, 7, 5],
  "%": [5, 1, 2, 4, 5],
  ",": [0, 0, 0, 2, 4],
  "?": [6, 1, 2, 0, 2],
  "=": [0, 7, 0, 7, 0],
  "+": [0, 2, 7, 2, 0],
  "<": [1, 2, 4, 2, 1],
  ">": [4, 2, 1, 2, 4],
};
