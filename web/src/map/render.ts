// Bakes the static map layers into an ImageData at one pixel per cell.
import type { MapView } from "../engine/protocol/MapView";
import { BIOME_RGB, decodeB64 } from "./palette";

/**
 * A map layer's fill per town (municipality id to RGB), from the engine's
 * classes (DEC-089, ./layers); `null` draws terrain only.
 */
export type LayerPaint = ReadonlyMap<number, readonly [number, number, number]> | null;

/** The selected town's outline over a layer (bright, in both themes). */
const PICK_RING = [255, 210, 63] as const;

export interface MapRaster {
  width: number;
  height: number;
  owner: Uint16Array;
  image: ImageData;
}

export function bakeRaster(view: MapView, selected: number | null, paint: LayerPaint = null): MapRaster {
  const { width: w, height: h } = view;
  const biome = decodeB64(view.biome_b64);
  const cover = decodeB64(view.cover_b64);
  const shade = decodeB64(view.shade_b64);
  const river = decodeB64(view.river_b64);
  const ownerBytes = decodeB64(view.owner_b64);
  const owner = new Uint16Array(w * h);
  for (let i = 0; i < w * h; i++) owner[i] = (ownerBytes[2 * i] ?? 255) | ((ownerBytes[2 * i + 1] ?? 255) << 8);
  const regionOf = new Map<number, number>(view.municipalities.map((m) => [m.id, m.region]));

  const img = new ImageData(w, h);
  const px = img.data;
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = y * w + x;
      const base = BIOME_RGB[biome[i] ?? 0] ?? [0, 0, 0];
      let [r, g, b] = base;
      const s = ((shade[i] ?? 128) - 128) / 128;
      const k = 1 + s * 0.7;
      r *= k;
      g *= k;
      b *= k;
      const c = cover[i] ?? 0;
      if (c === 1) {
        // Foreign land: desaturated and lighter.
        const avg = (r + g + b) / 3;
        r = avg * 0.55 + 70;
        g = avg * 0.55 + 70;
        b = avg * 0.55 + 74;
      }
      const rv = river[i] ?? 0;
      if (rv >= 2 && c !== 0) {
        [r, g, b] = rv === 3 ? [52, 96, 170] : [84, 132, 196];
      }
      const o = owner[i] ?? 65535;
      const fill = o !== 65535 && (cover[i] ?? 0) === 2 ? paint?.get(o) : undefined;
      if (fill) {
        // A whisper of relief, so the land still reads without shifting a class's colour.
        const relief = 1 + s * 0.12;
        r = fill[0] * relief;
        g = fill[1] * relief;
        b = fill[2] * relief;
      }
      if (o !== 65535) {
        const right = x + 1 < w ? owner[i + 1] : o;
        const down = y + 1 < h ? owner[i + w] : o;
        const regionBorder = [right, down].some(
          (n) => n !== undefined && n !== o && (n === 65535 || regionOf.get(n) !== regionOf.get(o)),
        );
        if (regionBorder) {
          [r, g, b] = [36, 28, 30];
        } else if (right !== o || down !== o) {
          r *= 0.82;
          g *= 0.82;
          b *= 0.82;
        }
        if (selected !== null && o === selected) {
          if (paint) {
            // Over a layer the town keeps its class colour: a bright ring
            // round its edge marks it instead of a tint.
            const left = x > 0 ? owner[i - 1] : o;
            const up = y > 0 ? owner[i - w] : o;
            if ([right, down, left, up].some((n) => n !== o)) [r, g, b] = PICK_RING;
          } else {
            r = r * 0.6 + 255 * 0.4;
            g = g * 0.6 + 214 * 0.4;
            b = b * 0.6 + 90 * 0.4;
          }
        }
      }
      const p = i * 4;
      px[p] = r;
      px[p + 1] = g;
      px[p + 2] = b;
      px[p + 3] = 255;
    }
  }
  return { width: w, height: h, owner, image: img };
}
