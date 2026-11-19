// Indexed images: one byte per pixel, 0 is transparent. Before recolouring
// the bytes are slot codes (a part's "skin base", "top shade", …); after,
// they are palette indices. Integer-only, no DOM.

export interface Indexed {
  w: number;
  h: number;
  px: Uint8Array;
}

export const blank = (w: number, h: number): Indexed => ({ w, h, px: new Uint8Array(w * h) });

export const at = (img: Indexed, x: number, y: number): number => (x < 0 || y < 0 || x >= img.w || y >= img.h ? 0 : (img.px[y * img.w + x] as number));

export function set(img: Indexed, x: number, y: number, v: number) {
  if (x < 0 || y < 0 || x >= img.w || y >= img.h) return;
  img.px[y * img.w + x] = v;
}

export function clone(img: Indexed): Indexed {
  return { w: img.w, h: img.h, px: img.px.slice() };
}

export function mirrorX(img: Indexed): Indexed {
  const out = blank(img.w, img.h);
  for (let y = 0; y < img.h; y++) for (let x = 0; x < img.w; x++) out.px[y * img.w + (img.w - 1 - x)] = img.px[y * img.w + x] as number;
  return out;
}

export function crop(img: Indexed, x0: number, y0: number, w: number, h: number): Indexed {
  const out = blank(w, h);
  for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) out.px[y * w + x] = at(img, x0 + x, y0 + y);
  return out;
}

export interface BlitOptions {
  /** Draw mirrored left to right. */
  mirror?: boolean;
  /** Only where the destination is already opaque (an overlay clipped to what is under it). */
  clip?: boolean;
  /** Only where the destination is transparent (a layer that goes behind). */
  under?: boolean;
  /** Replace source values through this table first (slot recolouring); 0 in the table skips the pixel. */
  map?: ArrayLike<number>;
}

/** Draw `src` onto `dst` at (x, y); transparent source pixels are skipped. */
export function blit(dst: Indexed, src: Indexed, x: number, y: number, o: BlitOptions = {}) {
  for (let sy = 0; sy < src.h; sy++) {
    const dy = y + sy;
    if (dy < 0 || dy >= dst.h) continue;
    for (let sx = 0; sx < src.w; sx++) {
      let v = src.px[sy * src.w + (o.mirror ? src.w - 1 - sx : sx)] as number;
      if (v === 0) continue;
      if (o.map) v = (o.map[v] as number | undefined) ?? 0;
      if (v === 0) continue;
      const dx = x + sx;
      if (dx < 0 || dx >= dst.w) continue;
      const i = dy * dst.w + dx;
      if (o.clip && dst.px[i] === 0) continue;
      if (o.under && dst.px[i] !== 0) continue;
      dst.px[i] = v;
    }
  }
}

/** Fill a rectangle with one value. */
export function fill(dst: Indexed, x: number, y: number, w: number, h: number, v: number) {
  for (let yy = Math.max(0, y); yy < Math.min(dst.h, y + h); yy++) for (let xx = Math.max(0, x); xx < Math.min(dst.w, x + w); xx++) dst.px[yy * dst.w + xx] = v;
}

/** Tile `src` over a rectangle of `dst` (a wall or roof texture), starting the pattern at (ox, oy). */
export function tileFill(dst: Indexed, src: Indexed, x: number, y: number, w: number, h: number, ox = 0, oy = 0, o: BlitOptions = {}) {
  for (let yy = Math.max(0, y); yy < Math.min(dst.h, y + h); yy++) {
    for (let xx = Math.max(0, x); xx < Math.min(dst.w, x + w); xx++) {
      const sx = (((xx - x + ox) % src.w) + src.w) % src.w;
      const sy = (((yy - y + oy) % src.h) + src.h) % src.h;
      let v = src.px[sy * src.w + sx] as number;
      if (v === 0) continue;
      if (o.map) v = (o.map[v] as number | undefined) ?? 0;
      if (v === 0) continue;
      const i = yy * dst.w + xx;
      if (o.clip && dst.px[i] === 0) continue;
      dst.px[i] = v;
    }
  }
}

/** Every pixel through a table (slots to palette indices, or one palette to another). */
export function remap(img: Indexed, table: ArrayLike<number>): Indexed {
  const out = blank(img.w, img.h);
  for (let i = 0; i < img.px.length; i++) out.px[i] = (table[img.px[i] as number] as number | undefined) ?? 0;
  return out;
}

/** Distinct non-transparent values in an image. */
export function colours(img: Indexed): number[] {
  const seen = new Set<number>();
  for (const v of img.px) if (v !== 0) seen.add(v);
  return [...seen].sort((a, b) => a - b);
}

/** The bounding box of the opaque pixels, or null for an empty image. */
export function bounds(img: Indexed): { x: number; y: number; w: number; h: number } | null {
  let x0 = img.w;
  let y0 = img.h;
  let x1 = -1;
  let y1 = -1;
  for (let y = 0; y < img.h; y++) {
    for (let x = 0; x < img.w; x++) {
      if (img.px[y * img.w + x] === 0) continue;
      if (x < x0) x0 = x;
      if (x > x1) x1 = x;
      if (y < y0) y0 = y;
      if (y > y1) y1 = y;
    }
  }
  return x1 < 0 ? null : { x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1 };
}

/**
 * The automatic exterior outline (decision D2): every transparent pixel that
 * touches an opaque one on a side becomes the outline colour of what it
 * touches (`lineOf`); where it touches several, the darkest line wins
 * (`lightness`). Diagonal-only neighbours stay empty, which rounds corners.
 */
export function outline(img: Indexed, lineOf: ArrayLike<number>, lightness: ArrayLike<number>): Indexed {
  const out = clone(img);
  const { w, h, px } = img;
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      if (px[y * w + x] !== 0) continue;
      let best = 0;
      let bestL = Infinity;
      for (const [dx, dy] of NEIGHBOURS) {
        const nx = x + dx;
        const ny = y + dy;
        if (nx < 0 || ny < 0 || nx >= w || ny >= h) continue;
        const v = px[ny * w + nx] as number;
        if (v === 0) continue;
        const line = lineOf[v] as number;
        const l = lightness[line] as number;
        if (l < bestL) {
          bestL = l;
          best = line;
        }
      }
      if (best) out.px[y * w + x] = best;
    }
  }
  return out;
}

const NEIGHBOURS = [
  [0, 1],
  [0, -1],
  [-1, 0],
  [1, 0],
] as const;

// ---------------------------------------------------------------------------
// Text grids: how parts are authored (one character per pixel, '.' empty).

/**
 * Parse a block of text into frames. Leading and trailing blank lines and the
 * common indentation are removed; frames sit side by side, separated by
 * spaces, so a walk cycle reads left to right in review. Every frame of a
 * block has the same size.
 */
export function gridFrames(text: string): string[][] {
  const lines = text.split("\n");
  while (lines.length && (lines[0] as string).trim() === "") lines.shift();
  while (lines.length && (lines[lines.length - 1] as string).trim() === "") lines.pop();
  if (lines.length === 0) throw new Error("empty grid");
  const rows = lines.map((l) => l.trim().split(/ +/));
  const n = (rows[0] as string[]).length;
  const frames: string[][] = Array.from({ length: n }, () => []);
  rows.forEach((segs, y) => {
    if (segs.length !== n) throw new Error(`grid row ${y} has ${segs.length} frames, expected ${n}:\n${text}`);
    segs.forEach((s, i) => (frames[i] as string[]).push(s));
  });
  for (const f of frames) {
    const w = (f[0] as string).length;
    if (f.some((r) => r.length !== w)) throw new Error(`ragged grid:\n${f.join("\n")}`);
  }
  return frames;
}

/** One frame's rows to an image of codes through a legend ('.' is always 0). */
export function gridToIndexed(rows: readonly string[], legend: Readonly<Record<string, number>>): Indexed {
  const h = rows.length;
  const w = (rows[0] ?? "").length;
  const img = blank(w, h);
  rows.forEach((r, y) => {
    for (let x = 0; x < w; x++) {
      const ch = r[x] as string;
      if (ch === ".") continue;
      const v = legend[ch];
      if (v === undefined) throw new Error(`unknown grid character '${ch}' at (${x}, ${y}):\n${rows.join("\n")}`);
      img.px[y * w + x] = v;
    }
  });
  return img;
}

/** Parse a block into images (one per frame). */
export const grids = (text: string, legend: Readonly<Record<string, number>>): Indexed[] => gridFrames(text).map((f) => gridToIndexed(f, legend));

/** Parse a block that holds exactly one frame. */
export function grid(text: string, legend: Readonly<Record<string, number>>): Indexed {
  const f = grids(text, legend);
  if (f.length !== 1) throw new Error(`expected one frame, got ${f.length}`);
  return f[0] as Indexed;
}
