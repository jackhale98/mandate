// `pixelkit lint`: the P0 lints (PIXEL_ART_TOOL.md §7.1) over Mandate's pack,
// with the starting thresholds from §7.1. Errors fail the command; warnings
// and measurements are reported.
import { hexToRgb, lightness, rgbToLab, type Rgb8 } from "../core/color";
import { Prng } from "../core/hash";
import { crop, remap, type Indexed } from "../core/image";
import { colourCount, edgePixels, nearClonePairs, offPalette, openOutline, orphans, silhouetteIoU } from "../core/lint";
import { composeCharacter, composeSlots, FACINGS, FRAMES, HAIR_SKIN_MIN, hairSteps, lookFromSeed, lookLines, partList, PLAYER_LOOK, slotMaterials, slotTable, visibleFeatures, type Look } from "../../art/mandate/characters";
import { HAIR_NAMES } from "../../art/mandate/palette";
import { LEGEND, SLOT } from "../../art/mandate/characterParts";
import { buildFacade, COPY_SIMILARITY, designOf, kitPartList, signatureOf, similarity } from "../../art/mandate/buildings";
import { sampleRows } from "../../art/mandate/samples";
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
  // Round 3: the outline is a darker colour of each material, so "is a line" is per look.
  let lineSum = 0;
  let linePx = 0;
  let softPx = 0;
  for (const [, look] of looks) {
    const c = composeCharacter(look, pal, pack.lightness);
    const lines = lookLines(look, pal);
    for (const row of c.frames) {
      for (const img of row) {
        off += offPalette(img, n);
        open += openOutline(img, (v) => lines.has(v));
        worstColours = Math.max(worstColours, colourCount(img));
      }
    }
    for (const v of edgePixels(c.frames[0]?.[0] as Indexed)) {
      const L = (pack.lightness[v] as number) / 1000;
      lineSum += L;
      linePx++;
      if (L > 0.3) softPx++;
    }
  }
  f.push({ lint: "palette conformance", level: off ? "error" : "info", subject: "21 characters × 18 frames", value: off, limit: "0 off-palette pixels" });
  f.push({ lint: "outline consistency", level: open ? "error" : "info", subject: "21 characters × 18 frames", value: open, limit: "0 edge pixels without outline" });
  f.push({ lint: "colours per character", level: worstColours <= 14 ? "info" : "warn", subject: "worst frame", value: worstColours, limit: "≤ 14 including the outline" });
  f.push({ lint: "outline softness", level: "info", subject: "outline pixels of the 21 people (idle down): mean Oklab L / share lighter than L 0.30 (near-black lines are about 0.19–0.23)", value: `${(lineSum / linePx).toFixed(2)} / ${Math.round((100 * softPx) / linePx)}%` });

  // ---- flatness (round 3): one flat tone per material and at most one shade, in every frame of every look
  {
    let worst = 0;
    let worstWhere = "";
    let over = 0;
    for (const [name, look] of [...looks, ...Array.from({ length: 180 }, (_, i) => [`seed ${i + 21}`, lookFromSeed(i + 21)] as [string, Look])]) {
      const table = slotTable(look, pal);
      const mat = slotMaterials(look);
      FACINGS.forEach((facing) => {
        FRAMES.forEach((_, k) => {
          const tones = new Map<string, Set<number>>();
          for (const sl of composeSlots(look, facing, k).px) {
            const m = mat[sl];
            if (!sl || !m) continue;
            let set = tones.get(m);
            if (!set) tones.set(m, (set = new Set()));
            set.add(table[sl] as number);
          }
          for (const [m, set] of tones) {
            if (set.size > 2) over++;
            if (set.size > worst) {
              worst = set.size;
              worstWhere = `${name}, ${m}, ${facing} ${FRAMES[k]}`;
            }
          }
        });
      });
    }
    f.push({ lint: "flatness", level: over ? "error" : "info", subject: `tones per material (skin, hair, top, bottoms, shoes, accent, hat, bag) in every frame of 200 looks; worst: ${worstWhere}`, value: worst, limit: "≤ 2 (a base and one shade)" });
  }

  // ---- heads (round 3): hair parts from skin on every skin tone; eyes show on every skin
  {
    const lab = pal.colors.map((c) => rgbToLab(hexToRgb(c)));
    let worstHair = Infinity;
    let worstHairAt = "";
    let worstEye = Infinity;
    let worstEyeAt = "";
    for (let sk = 0; sk < 6; sk++) {
      const t = slotTable({ ...PLAYER_LOOK, skin: sk }, pal);
      const skin = lab[t[SLOT.skin] as number];
      for (const hc of HAIR_NAMES) {
        const [base] = hairSteps(pal, sk, hc);
        const h = lab[base];
        if (!skin || !h) continue;
        const d = Math.hypot(skin.L - h.L, skin.a - h.a, skin.b - h.b);
        if (d < worstHair) {
          worstHair = d;
          worstHairAt = `skin ${sk + 1}, ${hc}`;
        }
      }
      // The eye is two pixels (its top and the dark below); the one further from the skin is what shows.
      const eyes = [lab[t[SLOT.eye] as number], lab[t[SLOT.ink] as number]].filter((e) => e !== undefined);
      const eyeL = skin ? Math.max(...eyes.map((e) => Math.abs(skin.L - e.L))) : 0;
      if (eyeL < worstEye) {
        worstEye = eyeL;
        worstEyeAt = `skin ${sk + 1}`;
      }
    }
    f.push({ lint: "head contrast", level: worstHair >= HAIR_SKIN_MIN * 0.7 ? (worstHair >= HAIR_SKIN_MIN ? "info" : "warn") : "error", subject: `hair against skin, every skin × hair colour, Oklab ΔE (worst: ${worstHairAt})`, value: worstHair.toFixed(3), limit: `≥ ${HAIR_SKIN_MIN}` });
    f.push({ lint: "head contrast", level: worstEye >= 0.2 ? "info" : "warn", subject: `the eye against the skin, Oklab ΔL (worst: ${worstEyeAt})`, value: worstEye.toFixed(3), limit: "≥ 0.2" });
  }

  // ---- orphan pixels in parts (highlights and eyes allowed)
  const allowedChar = new Set<number>([SLOT.ink, SLOT.eye, SLOT.white, SLOT.skinL, SLOT.hairL, SLOT.topL, SLOT.accL, LEGEND.I ?? -1]);
  // Measured on the parts recoloured for an everyday look (trousers, long sleeves), where slots that share a colour merge.
  const table = slotTable({ ...lookFromSeed(1), top: "suit", bottom: "trousers", skin: 2 }, pal);
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
  const base: Look = { ...lookFromSeed(1), hat: "none", carry: "none", old: false, glasses: false, top: "tee", bottom: "trousers" };
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
        const lines = lookLines(look, pal);
        const body = [...img.px].filter((v) => v !== 0 && !lines.has(v));
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

  // ---- buildings: neighbours never look like copies (round 2). 40 rows of 8, as a town lays them out.
  {
    let pairs = 0;
    let copies = 0;
    let worst = 0;
    let sum = 0;
    let pixelSame = 0;
    for (const row of sampleRows()) {
      for (let i = 1; i < row.length; i++) {
        const a = row[i - 1];
        const b = row[i];
        if (!a || !b) continue;
        const sim = similarity(signatureOf(designOf(a)), signatureOf(designOf(b)));
        pairs++;
        sum += sim;
        worst = Math.max(worst, sim);
        if (sim >= COPY_SIMILARITY) copies++;
        // And by pixels: the share of equal pixels where both buildings' frames overlap, aligned at the bottom left.
        const fa = buildFacade(a, pal).albedo;
        const fb = buildFacade(b, pal).albedo;
        let same = 0;
        let n = 0;
        const w = Math.min(fa.w, fb.w);
        const h = Math.min(fa.h, fb.h);
        for (let y = 0; y < h; y++) {
          for (let x = 0; x < w; x++) {
            const va = fa.px[(fa.h - h + y) * fa.w + x] as number;
            const vb = fb.px[(fb.h - h + y) * fb.w + x] as number;
            if (va === 0 && vb === 0) continue;
            n++;
            if (va === vb) same++;
          }
        }
        if (n && same / n > 0.8) pixelSame++;
      }
    }
    f.push({ lint: "neighbour similarity", level: copies ? "error" : "info", subject: `${pairs} neighbouring pairs in 40 rows: pairs sharing ≥ ${Math.round(COPY_SIMILARITY * 12)} of 12 roof and facade features`, value: copies, limit: "0" });
    f.push({ lint: "neighbour similarity", level: worst >= 0.67 ? "warn" : "info", subject: "mean / worst share of shared features between neighbours", value: `${(sum / Math.max(1, pairs)).toFixed(2)} / ${worst.toFixed(2)}`, limit: "worst < 0.67" });
    f.push({ lint: "neighbour similarity", level: pixelSame ? "error" : "info", subject: "neighbouring pairs with more than 80% of their pixels equal", value: pixelSame, limit: "0" });
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
