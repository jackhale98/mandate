// Round 6 lints for street ends (PIXEL_ART_TOOL.md "Round 6: street ends";
// DEC-090 D5 amended): the test town shows every kind in every facing it
// should, at widths 2 to 4, and draws every cell; the kerb runs unbroken round
// every bulb and head (every pixel pair where carriageway meets pavement is
// drawn as the kerb); repainted cells join their neighbours (no pixel changes
// where neither its material nor any within reach of its edge did); nothing
// away from the ends changes; no new colours; the new props stay small; and
// what meets at the new edges keeps its contrast by day and at night.
import { hexToRgb, rgbToLab, type Rgb8 } from "../core/color";
import { colourCount, offPalette } from "../core/lint";
import { diagScene } from "../../art/mandate/diagscene";
import { END_KINDS } from "../../art/mandate/ends";
import { endScene, type EndScene } from "../../art/mandate/endscene";
import type { ArtPack } from "../../art/mandate/pack";
import { TERRAIN as K, TerrainPainter, V1_NAMES, type TerrainView } from "../../art/mandate/terrain";
import { v2Scene } from "../../art/mandate/v2scene";
import type { Finding } from "./lint";
import { groundImage } from "./scene";

export interface EndStats {
  /** Ends laid out of those recorded. */
  laidOut: number;
  recorded: number;
  /** Kerb pixel pairs (carriageway beside pavement) round bulbs and heads, and those not drawn as a kerb. */
  kerbs: number;
  kerbBad: number;
  worstKerb: string;
  /** Pixels of repainted cells that changed surface (ramp) though nothing within an edge's reach changed material (a seam). */
  seams: number;
  worstSeam: string;
  /** Repainted pixels that show neither their material's surfaces nor a detail (a cell drawn as another material than its map pixel). */
  mismatches: number;
  worstMismatch: string;
  /** Cells away from every end that differ from the town drawn without `streetEnds`. */
  awayChanged: number;
  /** Palette indices the end cells use that no ground drew before. */
  newColours: number[];
  /** Most colours in one end cell. */
  maxColours: number;
}

/** Measures a map with street ends (the test town by default). */
export function endStats(pack: ArtPack, view: TerrainView & { streetEnds?: EndScene["streetEnds"] } = endScene()): EndStats {
  const img = groundImage(pack, view);
  const bare = groundImage(pack, { ...view, streetEnds: undefined });
  const painter = new TerrainPainter(pack.tiles, pack.terrain, view);
  const ends = painter.ends;
  const W = view.width;
  const out: EndStats = { laidOut: ends?.ends.length ?? 0, recorded: view.streetEnds?.length ?? 0, kerbs: 0, kerbBad: 0, worstKerb: "", seams: 0, worstSeam: "", mismatches: 0, worstMismatch: "", awayChanged: 0, newColours: [], maxColours: 0 };
  if (!ends) return out;
  const L = pack.terrain.legend;
  const px = (X: number, Y: number) => img.px[Y * img.w + X] as number;
  /** A colour's ramp ("" for markings, flowers and drain covers: chalk, flag, lamp, iron). */
  const rampOf = (i: number) => {
    const r = pack.palette.ramps[pack.palette.rampOf[i] as number];
    return !r || r.name === "chalk" || r.name === "flag" || r.name === "lamp" || r.name === "iron" ? "" : r.name;
  };

  // ---- kerbs round bulbs and heads: carriageway beside pavement is gutter beside kerb.
  const kerbSide = new Set([L.Q, L.q, L.P] as number[]);
  for (const e of ends.ends) {
    if (e.kind !== "turning_circle" && e.kind !== "hammerhead") continue;
    let bad = 0;
    for (let Y = e.zy0 * 16; Y < e.zy1 * 16; Y++) {
      for (let X = e.zx0 * 16; X < e.zx1 * 16; X++) {
        if (ends.mat(X, Y) !== K.Road) continue;
        for (const [dx, dy] of [
          [1, 0],
          [-1, 0],
          [0, 1],
          [0, -1],
        ] as const) {
          const nx = X + dx;
          const ny = Y + dy;
          if (nx < 0 || ny < 0 || nx >= img.w || ny >= img.h || ends.mat(nx, ny) !== K.Pavement) continue;
          out.kerbs++;
          if (px(X, Y) !== L.A || !kerbSide.has(px(nx, ny))) bad++;
        }
      }
    }
    out.kerbBad += bad;
    if (bad && !out.worstKerb) out.worstKerb = `${e.kind} ${e.facing} at ${e.ox},${e.oy}: ${bad} pairs`;
  }
  // ---- seams and isolation: compare with the town drawn without street ends.
  const nearChange = (X: number, Y: number) => {
    for (let dy = -4; dy <= 4; dy++) {
      for (let dx = -4; dx <= 4; dx++) {
        const XX = X + dx;
        const YY = Y + dy;
        if (XX < 0 || YY < 0 || XX >= img.w || YY >= img.h) continue;
        if (ends.mat(XX, YY) !== ends.baseMat(XX, YY)) return true;
      }
    }
    return false;
  };
  /** The surfaces (ramps) a material's tiles show: its fills and its edges. */
  const surfaces = new Map<number, Set<string> | null>();
  const allowed = (M: number) => {
    if (!surfaces.has(M)) {
      const t = pack.tiles.terrains[V1_NAMES[M] ?? ""];
      const set = t ? new Set<string>() : null;
      if (t && set) for (const img2 of [t.template, ...t.fills]) for (const c of img2.px) if (rampOf(c)) set.add(rampOf(c));
      surfaces.set(M, set);
    }
    return surfaces.get(M) ?? null;
  };
  for (let y = 0; y < view.height; y++) {
    for (let x = 0; x < W; x++) {
      const touched = ends.touched(x, y);
      let diffCell = 0;
      for (let py = 0; py < 16; py++) {
        for (let pxl = 0; pxl < 16; pxl++) {
          const X = x * 16 + pxl;
          const Y = y * 16 + py;
          const i = Y * img.w + X;
          if (img.px[i] === bare.px[i]) continue;
          if (!touched) {
            diffCell++;
            continue;
          }
          // Within one surface (a fill's pattern, a kerb's tones) the repainted cell may differ: a seam is another surface.
          const ra = rampOf(img.px[i] as number);
          const rb = rampOf(bare.px[i] as number);
          if (!ra || !rb || ra === rb) continue;
          // Allowed where the end draws a detail, or a material changed within 4 px (an edge's reach), or a marking went.
          if (ends.detailAt(X, Y) > 0 || bare.px[i] === L.M || bare.px[i] === L.m) continue;
          if (nearChange(X, Y)) continue;
          out.seams++;
          if (!out.worstSeam) out.worstSeam = `pixel ${X},${Y} (cell ${x},${y})`;
        }
      }
      if (diffCell) out.awayChanged++;
      // Every pixel of an end's cell shows its material (a colour of that material's fill or edges) or the end's detail.
      if (!touched) continue;
      for (let py = 0; py < 16; py++) {
        for (let pxl = 0; pxl < 16; pxl++) {
          const X = x * 16 + pxl;
          const Y = y * 16 + py;
          const v = px(X, Y);
          if (ends.detailAt(X, Y) > 0 || !rampOf(v)) continue;
          const M = ends.mat(X, Y);
          if (M !== ends.baseMat(X, Y) || nearChange(X, Y)) {
            const ok = allowed(M);
            if (ok && !ok.has(rampOf(v))) {
              out.mismatches++;
              if (!out.worstMismatch) out.worstMismatch = `pixel ${X},${Y} (cell ${x},${y})`;
            }
          }
        }
      }
    }
  }
  // ---- colours: what the end cells use, against what any ground drew before (v1 tiles, the round 4 and 5 scenes).
  const before = new Set<number>();
  for (const t of Object.values(pack.tiles.terrains)) {
    for (const v of t.template.px) before.add(v);
    for (const f of t.fills) for (const v of f.px) before.add(v);
  }
  for (const d of Object.values(pack.tiles.decals)) for (const v of d.px) before.add(v);
  for (const v of groundImage(pack, v2Scene()).px) before.add(v);
  for (const v of groundImage(pack, diagScene()).px) before.add(v);
  const used = new Set<number>();
  for (let y = 0; y < view.height; y++) {
    for (let x = 0; x < W; x++) {
      if (!ends.touched(x, y)) continue;
      const cell = { w: 16, h: 16, px: new Uint8Array(256) };
      for (let yy = 0; yy < 16; yy++) for (let xx = 0; xx < 16; xx++) cell.px[yy * 16 + xx] = px(x * 16 + xx, y * 16 + yy);
      for (const v of cell.px) used.add(v);
      out.maxColours = Math.max(out.maxColours, colourCount(cell));
    }
  }
  out.newColours = [...used].filter((v) => v !== 0 && !before.has(v)).sort((a, b) => a - b);
  return out;
}


export function lintEnds(pack: ArtPack): Finding[] {
  const f: Finding[] = [];
  const pal = pack.palette;
  const n = pal.colors.length - 1;
  const scene = endScene();

  // ---- coverage: every kind in every facing the round asks for, widths 2–4, every cell drawn
  {
    const img = groundImage(pack, scene);
    const bad = (offPalette(img, n) ? 1 : 0) + (img.px.some((p) => p === 0) ? 1 : 0);
    const missing: string[] = [];
    for (const kind of END_KINDS) {
      for (const facing of ["n", "e", "s", "w", "ne", "se"]) if (!scene.streetEnds.some((e) => e.kind === kind && e.facing === facing)) missing.push(`${kind} ${facing}`);
    }
    const painter = new TerrainPainter(pack.tiles, pack.terrain, scene);
    const laid = painter.ends?.ends ?? [];
    const widths = new Set(laid.map((e) => Math.round((e.hw * 2) / (e.diag ? 11.31 : 16))));
    for (const w of [2, 3, 4]) if (!widths.has(w)) missing.push(`width ${w}`);
    if (laid.length !== scene.streetEnds.length) missing.push(`${scene.streetEnds.length - laid.length} ends not laid out`);
    for (const k of ["bollards", "field_gate", "railing"]) if ((pack.props[k]?.length ?? 0) < 8) missing.push(`${k} props per facing`);
    f.push({ lint: "street-end coverage", level: bad || missing.length ? "error" : "info", subject: `the street-end test town (${scene.width} × ${scene.height}): every cell drawn, opaque, palette colours; ${scene.streetEnds.length} ends, every kind facing n, e, s, w, ne and se, widths ${[...widths].sort().join(", ")}; props per facing${missing.length ? `; missing: ${missing.join(", ")}` : ""}`, value: bad + missing.length, limit: "0" });
  }

  const st = endStats(pack, scene);
  f.push({ lint: "street-end kerbs", level: st.kerbBad ? "error" : "info", subject: `pixel pairs where carriageway meets pavement round bulbs and heads (${st.kerbs}) not drawn as gutter and kerb (first: ${st.worstKerb || "none"})`, value: st.kerbBad, limit: "0" });
  f.push({ lint: "street-end seams", level: st.seams ? "error" : "info", subject: `pixels of repainted cells drawn as another surface (ramp) than without street ends, where no material within an edge's reach changed and no detail is drawn (first: ${st.worstSeam || "none"})`, value: st.seams, limit: "0" });
  f.push({ lint: "street-end seams", level: st.mismatches ? "error" : "info", subject: `repainted pixels that show neither their material's surfaces (its fill and edges) nor an end's detail (first: ${st.worstMismatch || "none"})`, value: st.mismatches, limit: "0" });
  f.push({ lint: "street-end isolation", level: st.awayChanged ? "error" : "info", subject: "cells away from every street end that differ from the town drawn without streetEnds", value: st.awayChanged, limit: "0" });
  f.push({ lint: "street-end palette", level: st.newColours.length ? "error" : "info", subject: `colours in street-end cells that no ground drew before (${st.newColours.join(", ") || "none"}); the palette stays at ${n}`, value: st.newColours.length, limit: "0" });
  f.push({ lint: "colours per tile", level: st.maxColours <= 8 ? "info" : "warn", subject: "a street-end cell (the most colourful in the test town)", value: st.maxColours, limit: "≤ 8 (v1 tiles: ≤ 6)" });
  for (const k of ["bollards", "field_gate", "railing"]) {
    const most = Math.max(0, ...(pack.props[k] ?? []).map((p) => colourCount(p.albedo)));
    f.push({ lint: "colours per prop", level: most <= 8 ? "info" : "warn", subject: `${k} (the most colourful variant)`, value: most, limit: "≤ 8" });
  }

  // ---- what meets at the new edges keeps its contrast by day and at night (palette moods, 22:30)
  {
    const L = pack.terrain.legend;
    const night = pack.lighting.entries[90];
    const rgb = pal.colors.map(hexToRgb);
    const moodRgb = night ? night.moodPalette.map(hexToRgb) : rgb;
    const mul = night?.spriteMul ?? [255, 255, 255];
    const dE = (a: Rgb8, b: Rgb8) => {
      const x = rgbToLab(a);
      const y = rgbToLab(b);
      return Math.hypot(x.L - y.L, x.a - y.a, x.b - y.b);
    };
    const atNight = (i: number): Rgb8 => {
      const c = moodRgb[i] as Rgb8;
      return [(c[0] * mul[0]) / 255, (c[1] * mul[1]) / 255, (c[2] * mul[2]) / 255];
    };
    const pairs: [string, string, string][] = [
      ["dropped kerb stone / carriageway", "t", "a"],
      ["dropped kerb stone / pavement", "t", "p"],
      ["setts / carriageway", "q", "a"],
      ["setts / square", "P", "t"],
      ["hedge body / grass", "H", "g"],
      ["hedge top / body", "h", "H"],
      ["track / grass", "e", "g"],
      ["track / carriageway", "e", "a"],
      ["island kerb / carriageway", "q", "a"],
      ["island / carriageway", "g", "a"],
    ];
    for (const [name, a, b] of pairs) {
      const ia = L[a] as number;
      const ib = L[b] as number;
      const day = dE(rgb[ia] as Rgb8, rgb[ib] as Rgb8);
      const nt = dE(atNight(ia), atNight(ib));
      f.push({ lint: "street-end edge contrast", level: Math.min(day, nt) >= 0.05 ? "info" : "warn", subject: `${name}: Oklab ΔE by day / at night (moods)`, value: `${day.toFixed(3)} / ${nt.toFixed(3)}`, limit: "≥ 0.05" });
    }
  }
  return f;
}
