// `pixelkit lint`: the P0 lints (PIXEL_ART_TOOL.md §7.1) over Mandate's pack,
// with the starting thresholds from §7.1. Errors fail the command; warnings
// and measurements are reported.
import { hexToRgb, lightness, type Rgb8 } from "../core/color";
import { Prng } from "../core/hash";
import { crop, remap, type Indexed } from "../core/image";
import { colourCount, edgePixels, nearClonePairs, offPalette, openOutline, orphans, silhouetteIoU } from "../core/lint";
import { composeCharacter, lookFromSeed, partList, PLAYER_LOOK, slotTable, visibleFeatures, type Look } from "../../art/mandate/characters";
import { LEGEND, SLOT } from "../../art/mandate/characterParts";
import { kitPartList } from "../../art/mandate/buildings";
import { BSLOT } from "../../art/mandate/buildingParts";
import type { ArtPack } from "../../art/mandate/pack";
import type { LightEntry } from "../../art/mandate/lighting";
import { groundTile } from "../../art/mandate/tiles";

export type Level = "error" | "warn" | "info";
export interface Finding {
  lint: string;
  level: Level;
  subject: string;
  value: number | string;
  limit?: string;
}

/** A colour after the multiply look's grade, as the renderer composites it (saturation, multiply, soft light). */
export function gradeColour(c: Rgb8, e: LightEntry): Rgb8 {
  const l = 0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2];
  const k = 1 - e.sat;
  const out: number[] = [];
  for (let i = 0; i < 3; i++) {
    const d = (c[i] as number) + (l - (c[i] as number)) * k;
    const b = (d * (e.mul[i] as number)) / 255 / 255;
    const s = (e.glow[i] as number) / 255;
    const dd = b <= 0.25 ? ((16 * b - 12) * b + 4) * b : Math.sqrt(b);
    const sl = s <= 0.5 ? b - (1 - 2 * s) * b * (1 - b) : b + (2 * s - 1) * (dd - b);
    out.push(255 * (b + (sl - b) * e.glowAlpha));
  }
  return [out[0] as number, out[1] as number, out[2] as number];
}

const mulColour = (c: Rgb8, m: Rgb8): Rgb8 => [(c[0] * m[0]) / 255, (c[1] * m[1]) / 255, (c[2] * m[2]) / 255];

export function lintPack(pack: ArtPack): Finding[] {
  const f: Finding[] = [];
  const pal = pack.palette;
  const rgb = pal.colors.map(hexToRgb);
  const n = pal.colors.length - 1;
  const isLine = new Set(pal.ramps.map((r) => r.line));

  // ---- palette
  f.push({ lint: "palette size", level: n <= 128 ? "info" : "error", subject: "master palette", value: n, limit: "≤ 128" });
  const extremes = pal.colors.slice(1).filter((c) => c === "#000000" || c === "#ffffff");
  f.push({ lint: "no pure black or white", level: extremes.length ? "error" : "info", subject: "master palette", value: extremes.length });

  // ---- characters: 20 townsfolk and the player
  const looks: [string, Look][] = [["player", PLAYER_LOOK], ...Array.from({ length: 20 }, (_, i) => [`seed ${i + 1}`, lookFromSeed(i + 1)] as [string, Look])];
  let worstColours = 0;
  let open = 0;
  let off = 0;
  for (const [, look] of looks) {
    const c = composeCharacter(look, pal, pack.lightness);
    for (const row of c.frames) {
      for (const img of row) {
        off += offPalette(img, n);
        open += openOutline(img, (v) => isLine.has(v));
        worstColours = Math.max(worstColours, colourCount(img));
      }
    }
  }
  f.push({ lint: "palette conformance", level: off ? "error" : "info", subject: "21 characters × 18 frames", value: off, limit: "0 off-palette pixels" });
  f.push({ lint: "outline consistency", level: open ? "error" : "info", subject: "21 characters × 18 frames", value: open, limit: "0 edge pixels without outline" });
  f.push({ lint: "colours per character", level: worstColours <= 14 ? "info" : "warn", subject: "worst frame", value: worstColours, limit: "≤ 14 including the outline" });

  // ---- orphan pixels in parts (highlights and eyes allowed)
  const allowedChar = new Set<number>([SLOT.ink, SLOT.white, SLOT.skinL, SLOT.hairL, SLOT.topL, SLOT.accL, LEGEND.I ?? -1]);
  // Measured on the parts recoloured for an everyday look (trousers, long sleeves), where slots that share a colour merge.
  const table = slotTable({ ...lookFromSeed(1), top: "suit", bottom: "trousers" }, pal);
  const allowedColours = new Set([...allowedChar].map((sl) => table[sl] as number));
  for (const p of partList()) {
    const o = orphans(remap(p.img, table), allowedColours);
    if (o > 2) f.push({ lint: "orphan pixels", level: "warn", subject: `character part: ${p.name}`, value: o, limit: "≤ 2" });
  }
  const allowedKit = new Set<number>([BSLOT.glassY, BSLOT.glassB, BSLOT.ironL, BSLOT.doorL, BSLOT.wallL, BSLOT.trimL, BSLOT.picto]);
  for (const p of kitPartList()) {
    if (p.name.startsWith("picto") || p.name.startsWith("tex")) continue;
    const o = orphans(p.img, allowedKit);
    if (o > 2) f.push({ lint: "orphan pixels", level: "warn", subject: `kit part: ${p.name}`, value: o, limit: "≤ 2" });
  }
  for (const [name, t] of Object.entries(pack.tiles.terrains)) {
    t.fills.forEach((fill, i) => {
      const o = orphans(fill);
      if (o > 2) f.push({ lint: "orphan pixels", level: "warn", subject: `tile fill: ${name} ${i}`, value: o, limit: "≤ 2" });
    });
  }

  // ---- props
  for (const [kind, list] of Object.entries(pack.props)) {
    list.forEach((p, i) => {
      const o = openOutline(p.albedo, (v) => isLine.has(v));
      if (o) f.push({ lint: "outline consistency", level: "error", subject: `prop ${kind} ${i}`, value: o });
      const cc = colourCount(p.albedo);
      if (cc > 8) f.push({ lint: "colours per prop", level: "warn", subject: `prop ${kind} ${i}`, value: cc, limit: "≤ 8" });
    });
  }

  // ---- tiles: colours per tile
  for (const name of Object.keys(pack.tiles.terrains)) {
    let worst = 0;
    for (const m of [0, 15, 255, 1 | 4, 2 | 8]) for (let v = 0; v < 8; v++) worst = Math.max(worst, colourCount(groundTile(pack.tiles, name, m, v)));
    if (worst > 6) f.push({ lint: "colours per tile", level: "warn", subject: `tile ${name}`, value: worst, limit: "≤ 6" });
  }

  // ---- silhouettes: builds must read apart at 1×, heights must step
  const base = lookFromSeed(1);
  const sil = (l: Look) => composeCharacter(l, pal, pack.lightness).frames[0]?.[0] as Indexed;
  // Builds differ in shoulders, hips and arms, never in head size (§6), so the silhouettes are compared below the chin.
  const builds = (["slim", "average", "broad"] as const).map((b) => crop(sil({ ...base, build: b, height: 1, hair: "crop", top: "tee" }), 0, 12, 16, 12));
  for (const [i, j] of [
    [0, 1],
    [1, 2],
    [0, 2],
  ] as const) {
    const iou = silhouetteIoU(builds[i] as Indexed, builds[j] as Indexed);
    f.push({ lint: "silhouette readability", level: iou <= 0.85 ? "info" : "error", subject: `builds ${["slim", "average", "broad"][i]} / ${["slim", "average", "broad"][j]}, body below the chin`, value: iou.toFixed(2), limit: "IoU ≤ 0.85" });
  }
  const heights = ([0, 1, 2] as const).map((h) => {
    const img = sil({ ...base, height: h, hair: "buzz" });
    let top = img.h;
    for (let y = 0; y < img.h && top === img.h; y++) for (let x = 0; x < img.w; x++) if (img.px[y * img.w + x]) top = y;
    return 23 - top - 1;
  });
  f.push({ lint: "height bands", level: heights[0]! < heights[1]! && heights[1]! < heights[2]! ? "info" : "error", subject: "short / middle / tall figure (px, without outline)", value: heights.join(" / ") });

  // ---- contrast against the ground, by day and at night, in both night looks
  const groundMean = (img: Indexed, colour: (c: Rgb8) => Rgb8) => {
    let s = 0;
    for (const v of img.px) s += lightness(colour(rgb[v] as Rgb8));
    return s / img.px.length;
  };
  const day = pack.lighting.entries[48] as LightEntry;
  const nightE = pack.lighting.entries[90] as LightEntry;
  const grounds = ["pavement", "road", "plaza", "grass"].map((t) => [t, groundTile(pack.tiles, t, 255, 0)] as const);
  const moments: [string, (c: Rgb8) => Rgb8, (v: number) => Rgb8, number][] = [
    ["noon", (c) => gradeColour(c, day), (v) => gradeColour(rgb[v] as Rgb8, day), 0.15],
    ["night (multiply)", (c) => gradeColour(c, nightE), (v) => gradeColour(rgb[v] as Rgb8, nightE), 0.1],
    ["night (moods)", (c) => c, (v) => mulColour(rgb[v] as Rgb8, nightE.spriteMul), 0.1],
  ];
  for (const [moment, groundColour, figColour, limit] of moments) {
    for (const [tname, tile] of grounds) {
      const gL =
        moment === "night (moods)"
          ? (() => {
              const mp = nightE.moodPalette.map(hexToRgb);
              let s = 0;
              for (const v of tile.px) s += lightness(mulColour(mp[v] as Rgb8, nightE.spriteMul));
              return s / tile.px.length;
            })()
          : groundMean(tile, groundColour);
      let worst = Infinity;
      let worstName = "";
      for (const [name, look] of looks) {
        const img = composeCharacter(look, pal, pack.lightness).frames[0]?.[0] as Indexed;
        const edge = edgePixels(img);
        // The figure's edge (outline) and its body's mean lightness: the larger difference is what the eye separates.
        const eL = edge.reduce((s, v) => s + lightness(figColour(v)), 0) / edge.length;
        const body = [...img.px].filter((v) => v !== 0 && !isLine.has(v));
        const bL = body.reduce((s, v) => s + lightness(figColour(v)), 0) / body.length;
        const d = Math.max(Math.abs(eL - gL), Math.abs(bL - gL));
        if (d < worst) {
          worst = d;
          worstName = name;
        }
      }
      f.push({ lint: "contrast against the ground", level: worst >= limit ? "info" : worst >= limit * 0.7 ? "warn" : "error", subject: `${moment}, on ${tname} (worst: ${worstName})`, value: worst.toFixed(3), limit: `ΔL ≥ ${limit}` });
    }
  }

  // ---- crowd clones: 1,000 crowds of 20
  const r = new Prng(2026);
  let pairs = 0;
  let worstCrowd = 0;
  for (let k = 0; k < 1000; k++) {
    const feats = Array.from({ length: 20 }, () => visibleFeatures(lookFromSeed(r.int(0, 2 ** 31))));
    const p = nearClonePairs(feats, 5);
    pairs += p;
    worstCrowd = Math.max(worstCrowd, p);
  }
  const avg = pairs / 1000;
  f.push({ lint: "crowd clones", level: avg <= 0.5 ? "info" : "error", subject: "pairs sharing 5 of 6 features, mean per crowd of 20 (worst crowd)", value: `${avg.toFixed(2)} (${worstCrowd})`, limit: "≤ 0.5" });
  const twenty = looks.slice(1).map(([, l]) => visibleFeatures(l));
  f.push({ lint: "crowd clones", level: "info", subject: "the 20-person sheet", value: nearClonePairs(twenty, 5), limit: "pairs sharing 5 of 6" });
  return f;
}

export function lintReport(fs: readonly Finding[]): string {
  const lines = fs.map((x) => `${x.level.toUpperCase().padEnd(5)} ${x.lint.padEnd(28)} ${String(x.value).padEnd(14)} ${x.limit ?? ""}  ${x.subject}`);
  const e = fs.filter((x) => x.level === "error").length;
  const w = fs.filter((x) => x.level === "warn").length;
  return `pixelkit lint: ${e} error(s), ${w} warning(s)\n\n${lines.join("\n")}\n`;
}
