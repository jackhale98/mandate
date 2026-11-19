// A small town compositor for review sheets (round 4): the ground through the
// v2 terrain painter (the same calls the game's art provider makes), the
// buildings and props in base-row order, a few townsfolk, day shadows, and
// the palette-moods night (static layers in the moment's mood palette, the
// people under their gentler multiply, lit windows and lamp heads unmoved,
// banded lamp pools added). It approximates the hub renderer for contact
// sheets; the game draws with web/src/hub/render.
import { hexToRgb, type Rgb8 } from "../core/color";
import { blank, blit, type Indexed } from "../core/image";
import { Sheet } from "../node/sheet";
import { buildFacade } from "../../art/mandate/buildings";
import { composeCharacter, lookFromSeed } from "../../art/mandate/characters";
import type { ArtPack } from "../../art/mandate/pack";
import { TerrainPainter, V1_NAMES, type ConnectorView } from "../../art/mandate/terrain";
import { groundTile } from "../../art/mandate/tiles";
import type { SceneBuilding, SceneProp } from "../../art/mandate/v2scene";

export interface SceneInput {
  width: number;
  height: number;
  terrain: ArrayLike<number>;
  elevation?: ArrayLike<number>;
  connectors?: readonly ConnectorView[];
  buildings?: readonly SceneBuilding[];
  props?: readonly SceneProp[];
  people?: readonly { x: number; y: number; seed: number }[];
}

export type Moment = "day" | "dusk" | "night";

/** The renderer's variant per cell (a stable hash; the low bit a checkerboard). */
const variantOf = (x: number, y: number) => ((Math.imul(x * 73856093 ^ y * 19349663, 0x45d9f3b) >>> 7) & 6) | ((x + y) & 1);

/** The ground of a whole map as one indexed image (what the hub bakes into its chunks). */
export function groundImage(pack: ArtPack, s: SceneInput): Indexed {
  const painter = new TerrainPainter(pack.tiles, pack.terrain, s);
  const img = blank(s.width * 16, s.height * 16);
  for (let y = 0; y < s.height; y++) {
    for (let x = 0; x < s.width; x++) {
      const t = s.terrain[y * s.width + x] as number;
      const v = variantOf(x, y);
      const tile = painter.simple(x, y) ? groundTile(pack.tiles, V1_NAMES[t] ?? "grass", painter.v1Mask(x, y), v) : painter.draw(x, y, v);
      blit(img, tile, x * 16, y * 16);
    }
  }
  return img;
}

interface Item {
  img: Indexed;
  x: number;
  y: number;
  base: number;
  person?: boolean;
  lit?: { img: Indexed; x: number; y: number }[];
  pool?: [number, number];
  shadows?: { footprint: readonly (readonly [number, number])[]; height: number; ox: number; oy: number }[];
}

/** Render a scene at a moment and an integer scale. */
export function renderScene(pack: ArtPack, s: SceneInput, moment: Moment, scale: number): Sheet {
  const pal = pack.palette;
  const W = s.width * 16;
  const H = s.height * 16;
  const ground = groundImage(pack, s);
  const items: Item[] = [];
  for (const b of s.buildings ?? []) {
    const f = buildFacade(b.spec, pal);
    const x = b.x * 16;
    const y = b.y * 16 - f.ay;
    items.push({
      img: f.albedo,
      x,
      y,
      base: (b.y + b.spec.h) * 16,
      lit: [...f.windows, ...f.doors].filter((o) => o.emissive).map((o) => ({ img: o.emissive as Indexed, x: x + o.x, y: y + o.y })),
      shadows: f.shadows.map((c) => ({ ...c, ox: b.x * 16, oy: b.y * 16 })),
    });
  }
  for (const p of s.props ?? []) {
    const list = pack.props[p.kind];
    const sp = list?.[p.variant % (list?.length || 1)];
    if (!sp) continue;
    const x = p.x * 16 - sp.ax;
    const y = p.y * 16 - sp.ay;
    const it: Item = { img: sp.albedo, x, y, base: p.y * 16 + Math.max(16, sp.albedo.h - sp.ay) };
    if (sp.emissive) {
      it.lit = [{ img: sp.emissive, x, y }];
      it.pool = [p.x * 16 + 8, p.y * 16 + 14];
    }
    if (sp.shadow) it.shadows = [{ ...sp.shadow, ox: x, oy: y }];
    items.push(it);
  }
  for (const p of s.people ?? []) {
    const c = composeCharacter(lookFromSeed(p.seed), pal, pack.lightness);
    items.push({ img: c.frames[0]?.[0] as Indexed, x: p.x * 16 + 1, y: p.y * 16 + 13 - 23, base: p.y * 16 + 13, person: true });
  }
  items.sort((a, b) => a.base - b.base || a.x - b.x);

  const base = pal.colors.map(hexToRgb);
  const entry = pack.lighting.entries[moment === "night" ? 90 : moment === "dusk" ? 78 : 56];
  const night = moment !== "day" && entry;
  const mul = night ? entry.spriteMul : ([255, 255, 255] as const);
  const mood = night ? entry.moodPalette.map(hexToRgb) : base;
  const statics: Rgb8[] = mood.map((c) => [(c[0] * mul[0]) / 255, (c[1] * mul[1]) / 255, (c[2] * mul[2]) / 255]);
  const people: Rgb8[] = base.map((c) => [(c[0] * mul[0]) / 255, (c[1] * mul[1]) / 255, (c[2] * mul[2]) / 255]);

  // Composite at 1×: RGB per pixel.
  const rgb = new Float32Array(W * H * 3);
  const put = (img: Indexed, x0: number, y0: number, table: readonly Rgb8[]) => {
    for (let y = 0; y < img.h; y++) {
      const Y = y0 + y;
      if (Y < 0 || Y >= H) continue;
      for (let x = 0; x < img.w; x++) {
        const X = x0 + x;
        const v = img.px[y * img.w + x] as number;
        if (!v || X < 0 || X >= W) continue;
        const c = table[v] as Rgb8;
        const i = (Y * W + X) * 3;
        rgb[i] = c[0];
        rgb[i + 1] = c[1];
        rgb[i + 2] = c[2];
      }
    }
  };
  put(ground, 0, 0, statics);
  // Day shadows on the ground: each caster's footprint swept along the sun's vector, one shade.
  const sun = moment === "night" ? null : moment === "dusk" ? ([0.9, 0.35] as const) : ([0.42, 0.24] as const);
  if (sun) {
    const mask = new Uint8Array(W * H);
    for (const it of items) {
      for (const c of it.shadows ?? []) {
        const xs = c.footprint.map((p) => p[0] + c.ox);
        const ys = c.footprint.map((p) => p[1] + c.oy);
        const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
        const steps = Math.ceil(c.height);
        for (let k = 0; k <= steps; k += 2) {
          const ox = Math.round((sun[0] * c.height * k) / Math.max(1, steps) / 2) * 2;
          const oy = Math.round((sun[1] * c.height * k) / Math.max(1, steps) / 2) * 2;
          for (let y = y0 + oy; y < y1 + oy; y++) for (let x = x0 + ox; x < x1 + ox; x++) if (x >= 0 && y >= 0 && x < W && y < H) mask[y * W + x] = 1;
        }
      }
    }
    const sc = hexToRgb(pack.lighting.shadowColor);
    const a = moment === "dusk" ? 0.3 : 0.34;
    for (let i = 0; i < W * H; i++) {
      if (!mask[i]) continue;
      for (let k = 0; k < 3; k++) rgb[i * 3 + k] = (rgb[i * 3 + k] as number) * (1 - a) + (sc[k] as number) * a;
    }
  }
  for (const it of items) put(it.img, it.x, it.y, it.person ? people : statics);
  if (night) {
    for (const it of items) for (const l of it.lit ?? []) put(l.img, l.x, l.y, base);
    // Lamp pools: stepped bands, added.
    const pool = pack.pool;
    for (const it of items) {
      if (!it.pool) continue;
      const [cx, cy] = it.pool;
      for (let y = 0; y < pool.h; y++) {
        for (let x = 0; x < pool.w; x++) {
          const band = pool.bands[y * pool.w + x] as number;
          const c = pool.colours[band];
          if (!band || !c) continue;
          // The outer band dithered, as the style guide allows for pools.
          if (band === 1 && (x + y) % 2) continue;
          const X = cx - pool.w / 2 + x;
          const Y = cy - pool.h / 2 + y;
          if (X < 0 || Y < 0 || X >= W || Y >= H) continue;
          const i = (Y * W + X) * 3;
          for (let k = 0; k < 3; k++) rgb[i + k] = Math.min(255, (rgb[i + k] as number) + (c[k] as number) * c[3] * entry.lights);
        }
      }
    }
  }
  const sheet = new Sheet(W * scale, H * scale);
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const i = (y * W + x) * 3;
      sheet.rect(x * scale, y * scale, scale, scale, [Math.round(rgb[i] as number), Math.round(rgb[i + 1] as number), Math.round(rgb[i + 2] as number)]);
    }
  }
  return sheet;
}
