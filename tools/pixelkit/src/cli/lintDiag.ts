// Round 5 lints for diagonal streets (PIXEL_ART_TOOL.md "Round 5: diagonal
// streets"; DEC-090 D5 amended): the test town draws every cell; it shows
// both axes, the widths, zebras, a level crossing, junctions and wedges; no
// staircase survives across the cell edges of a diagonal street (seams); the
// kerbs are drawn the same all along every 45° edge; nothing outside the
// diagonal streets changes; no new colours; and what meets at the new edges
// keeps its contrast by day and at night (palette moods).
import { hexToRgb, rgbToLab, type Rgb8 } from "../core/color";
import type { Indexed } from "../core/image";
import { colourCount, offPalette } from "../core/lint";
import { diagScene } from "../../art/mandate/diagscene";
import type { ArtPack } from "../../art/mandate/pack";
import { STREET_AXIS } from "../../art/mandate/streets";
import { TERRAIN as K, TerrainPainter, type TerrainView } from "../../art/mandate/terrain";
import { v2Scene } from "../../art/mandate/v2scene";
import type { Finding } from "./lint";
import { groundImage } from "./scene";

export interface DiagStats {
  /** Edges between two cells of the same diagonal street axis, and those whose drawn surfaces step across the edge (a staircase). */
  edges: number;
  stairs: number;
  worstStair: string;
  /** 45° kerb pixels checked, and those not drawn as the kerb's profile. */
  kerbs: number;
  kerbBad: number;
  /** Palette indices the diagonal cells use that no v1 or round 4 ground uses. */
  newColours: number[];
  /** Most colours in one drawn diagonal cell. */
  maxColours: number;
}

/** Ramp of a palette index ("" for chalk, which both markings and flowers use). */
function rampName(pack: ArtPack, i: number): string {
  const r = pack.palette.ramps[pack.palette.rampOf[i] as number];
  return !r || r.name === "chalk" ? "" : r.name;
}

/** Measures a map with diagonal streets (the test town by default): seams, kerbs, colours. */
export function diagStats(pack: ArtPack, view: TerrainView = diagScene()): DiagStats {
  const img = groundImage(pack, view);
  const painter = new TerrainPainter(pack.tiles, pack.terrain, view);
  const s = painter.streets;
  const W = view.width;
  const axis = (x: number, y: number) => (view.streetAxis?.[y * W + x] as number | undefined) ?? 0;
  const px = (X: number, Y: number) => img.px[Y * img.w + X] as number;
  const out: DiagStats = { edges: 0, stairs: 0, worstStair: "", kerbs: 0, kerbBad: 0, newColours: [], maxColours: 0 };
  if (!s) return out;
  // ---- seams: across every edge between two cells along one diagonal street (away from its ends, junctions and
  // crossings), the surfaces (ramps) change only where a 45° edge crosses it: a few rows, never a staircase's riser.
  let worst = 0;
  const t = (x: number, y: number) => view.terrain[Math.max(0, Math.min(view.height - 1, y)) * W + Math.max(0, Math.min(W - 1, x))] as number;
  /** A cell inside a diagonal street: the cells one step along the street both ways are the same terrain on the same axis (or off the map). */
  const along = (x: number, y: number) => {
    const a = axis(x, y);
    if (a !== STREET_AXIS.DiagonalNE && a !== STREET_AXIS.DiagonalNW) return false;
    const s2 = a === STREET_AXIS.DiagonalNW ? 1 : -1;
    return [-1, 1].every((k) => {
      const nx = x + s2 * k;
      const ny = y + k;
      if (nx < 0 || ny < 0 || nx >= W || ny >= view.height) return true;
      return axis(nx, ny) === a && t(nx, ny) === t(x, y);
    });
  };
  for (let y = 0; y < view.height; y++) {
    for (let x = 0; x < W; x++) {
      const a = axis(x, y);
      if (!along(x, y)) continue;
      for (const [dx, dy] of [
        [1, 0],
        [0, 1],
      ] as const) {
        if (x + dx >= W || y + dy >= view.height || axis(x + dx, y + dy) !== a || !along(x + dx, y + dy)) continue;
        out.edges++;
        let steps = 0;
        for (let k = 0; k < 16; k++) {
          const [X, Y] = dx ? [x * 16 + 15, y * 16 + k] : [x * 16 + k, y * 16 + 15];
          const ra = rampName(pack, px(X, Y));
          const rb = rampName(pack, px(X + dx, Y + dy));
          if (ra && rb && ra !== rb) steps++;
        }
        if (steps > 3) out.stairs++;
        if (steps > worst) {
          worst = steps;
          out.worstStair = `${x},${y} ${dx ? "east" : "south"} edge: ${steps} rows change surface`;
        }
      }
    }
  }
  // ---- kerbs: wherever a 45° edge runs between pavement and carriageway, the drawn pixels on each side are the kerb's profile.
  const L = pack.terrain.legend;
  for (let Y = 0; Y + 1 < view.height * 16; Y++) {
    for (let X = 0; X < W * 16; X++) {
      const ca = (Y >> 4) * W + (X >> 4);
      const cb = ((Y + 1) >> 4) * W + (X >> 4);
      // Not on crossings (stop lines, rails, posts) or near the map's edge.
      if (view.terrain[ca] === K.Crossing || view.terrain[cb] === K.Crossing) continue;
      if (!s.cutQuarter(X, Y) && !s.cutQuarter(X, Y + 1)) continue;
      const up = s.mat(X, Y);
      const down = s.mat(X, Y + 1);
      if (up === down) continue;
      // Where two 45° edges meet in a sharp point, both profiles fall on the same few pixels: not a kerb to check.
      if ([2, 3, 4].some((k) => s.mat(X, Y + 1 - k) !== up || s.mat(X, Y + k) !== down)) continue;
      let want: [number, number] | null = null;
      if (up === K.Pavement && down === K.Road) want = [L.Q as number, L.A as number];
      else if (up === K.Road && down === K.Pavement) want = [L.A as number, L.P as number];
      if (!want) continue;
      out.kerbs++;
      if (px(X, Y) !== want[0] || px(X, Y + 1) !== want[1]) out.kerbBad++;
    }
  }
  // ---- colours: what the diagonal cells use, against what any ground drew before (v1 tiles and the round 4 scene).
  const before = new Set<number>();
  for (const t of Object.values(pack.tiles.terrains)) {
    for (const v of t.template.px) before.add(v);
    for (const f of t.fills) for (const v of f.px) before.add(v);
  }
  for (const d of Object.values(pack.tiles.decals)) for (const v of d.px) before.add(v);
  for (const v of groundImage(pack, v2Scene()).px) before.add(v);
  const used = new Set<number>();
  for (let y = 0; y < view.height; y++) {
    for (let x = 0; x < W; x++) {
      if (!s.touched(x, y)) continue;
      const cell: Indexed = { w: 16, h: 16, px: new Uint8Array(256) };
      for (let yy = 0; yy < 16; yy++) for (let xx = 0; xx < 16; xx++) cell.px[yy * 16 + xx] = px(x * 16 + xx, y * 16 + yy);
      for (const v of cell.px) used.add(v);
      out.maxColours = Math.max(out.maxColours, colourCount(cell));
    }
  }
  out.newColours = [...used].filter((v) => v !== 0 && !before.has(v)).sort((a, b) => a - b);
  return out;
}

export function lintDiag(pack: ArtPack): Finding[] {
  const f: Finding[] = [];
  const pal = pack.palette;
  const n = pal.colors.length - 1;
  const scene = diagScene();
  const W = scene.width;

  // ---- coverage: the test town draws; it shows what the street vocabulary can say
  {
    const img = groundImage(pack, scene);
    const bad = (offPalette(img, n) ? 1 : 0) + (img.px.some((p) => p === 0) ? 1 : 0);
    const widths: Record<number, Set<number>> = { 1: new Set(), 2: new Set() };
    let junctions = 0;
    for (let y = 0; y < scene.height; y++) {
      let run = 0;
      let runAxis = 0;
      for (let x = 0; x <= W; x++) {
        const i = y * W + x;
        const a = x < W ? (scene.streetAxis[i] as number) : 0;
        const road = x < W && (scene.terrain[i] === K.Road || scene.terrain[i] === K.Crossing) && (a === 1 || a === 2);
        if (x < W && a === STREET_AXIS.Junction) junctions++;
        if (road && (run === 0 || a === runAxis)) {
          run++;
          runAxis = a;
          continue;
        }
        if (run) widths[runAxis]?.add(run);
        run = road ? 1 : 0;
        runAxis = road ? a : 0;
      }
    }
    const painter = new TerrainPainter(pack.tiles, pack.terrain, scene);
    const missing: string[] = [];
    for (const a of [1, 2]) for (const w of [2, 3, 4]) if (!widths[a]?.has(w)) missing.push(`${a === 1 ? "/" : "\\"} ${w} cells`);
    let zebraNE = 0;
    let zebraNW = 0;
    let level = 0;
    for (let y = 0; y < scene.height; y++) {
      for (let x = 0; x < W; x++) {
        const i = y * W + x;
        if (scene.terrain[i] !== K.Crossing) continue;
        if (painter.isRail(x, y)) level++;
        else if (scene.streetAxis[i] === 1) zebraNE++;
        else if (scene.streetAxis[i] === 2) zebraNW++;
      }
    }
    if (!zebraNE || !zebraNW) missing.push("a zebra on each axis");
    if (!level) missing.push("a level crossing");
    if (!junctions) missing.push("junctions");
    if (!scene.connectors.some((c) => c.kind === "bridge")) missing.push("a bridge");
    const wedge = (t: number) => {
      for (let y = 0; y < scene.height; y++) for (let x = 0; x < W; x++) if (scene.terrain[y * W + x] === t && painter.streets?.touched(x, y)) return true;
      return false;
    };
    for (const [t, name] of [
      [K.Park, "a green wedge"],
      [K.Yard, "a yard wedge"],
      [K.Plaza, "a square's edge"],
    ] as const)
      if (!wedge(t)) missing.push(name);
    const ws = [1, 2].map((a) => `${a === 1 ? "/" : "\\"} ${[...(widths[a] ?? [])].sort().join(", ")}`).join("; ");
    f.push({ lint: "diagonal coverage", level: bad || missing.length ? "error" : "info", subject: `the diagonal test town (${W} × ${scene.height}): every cell drawn, opaque, palette colours; widths per row (${ws}), zebras (/ ${zebraNE}, \\ ${zebraNW} cells), level crossing (${level}), junction cells (${junctions}), bridge, green, yard and square wedges${missing.length ? `; missing: ${missing.join(", ")}` : ""}`, value: bad + missing.length, limit: "0" });
  }

  const st = diagStats(pack, scene);
  // ---- seams: no staircase across the cell edges of a diagonal street; kerbs drawn alike all along every 45° edge
  f.push({ lint: "diagonal seams", level: st.stairs ? "error" : "info", subject: `cell edges along diagonal streets, away from their ends, junctions and crossings (${st.edges}) whose surfaces step across the edge (more than 3 of 16 rows change; worst: ${st.worstStair || "none"})`, value: st.stairs, limit: "0" });
  f.push({ lint: "diagonal seams", level: st.kerbBad ? "error" : "info", subject: `45° kerb pixels (${st.kerbs}) not drawn as the kerb (upper pavement: face over gutter; lower pavement: gutter over kerb edge)`, value: st.kerbBad, limit: "0" });

  // ---- unchanged elsewhere: an all-orthogonal street axis draws the round 4 scene exactly; untouched cells of the test town match the town drawn without an axis
  {
    const v2 = v2Scene();
    const a = groundImage(pack, v2);
    const b = groundImage(pack, { ...v2, streetAxis: new Uint8Array(v2.width * v2.height) });
    let diff = 0;
    for (let i = 0; i < a.px.length; i++) if (a.px[i] !== b.px[i]) diff++;
    const painter = new TerrainPainter(pack.tiles, pack.terrain, scene);
    const withAxis = groundImage(pack, scene);
    const without = groundImage(pack, { ...scene, streetAxis: undefined });
    let cellsDiff = 0;
    for (let y = 0; y < scene.height; y++) {
      for (let x = 0; x < W; x++) {
        if (painter.streets?.touched(x, y)) continue;
        let d = false;
        for (let yy = 0; yy < 16 && !d; yy++) for (let xx = 0; xx < 16; xx++) if (withAxis.px[(y * 16 + yy) * withAxis.w + x * 16 + xx] !== without.px[(y * 16 + yy) * without.w + x * 16 + xx]) d = true;
        if (d) cellsDiff++;
      }
    }
    f.push({ lint: "diagonal isolation", level: diff || cellsDiff ? "error" : "info", subject: `pixels of the round 4 scene that change with an all-orthogonal street axis (${diff}), and cells of the test town away from its diagonals that change (${cellsDiff})`, value: diff + cellsDiff, limit: "0" });
  }

  // ---- colours: no new colours; colours per drawn diagonal cell
  f.push({ lint: "diagonal palette", level: st.newColours.length ? "error" : "info", subject: `colours in diagonal cells that no ground drew before (${st.newColours.join(", ") || "none"}); the palette stays at ${n}`, value: st.newColours.length, limit: "0" });
  f.push({ lint: "colours per tile", level: st.maxColours <= 8 ? "info" : "warn", subject: "a drawn diagonal cell (the most colourful in the test town)", value: st.maxColours, limit: "≤ 8 (v1 tiles: ≤ 6)" });

  // ---- what meets at the new edges and the markings keep their contrast by day and at night (palette moods, 22:30)
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
      ["carriageway / pavement", "a", "p"],
      ["upper kerb: face / gutter", "Q", "A"],
      ["lower kerb: gutter / edge", "A", "P"],
      ["kerb top / carriageway", "q", "a"],
      ["pavement / grass", "p", "g"],
      ["pavement / green", "p", "G"],
      ["lane / grass", "a", "g"],
      ["square / pavement", "s", "p"],
      ["centre line / carriageway", "M", "a"],
      ["zebra end / carriageway", "m", "a"],
      ["barrier post / carriageway", "R", "a"],
    ];
    for (const [name, a, b] of pairs) {
      const ia = L[a] as number;
      const ib = L[b] as number;
      const day = dE(rgb[ia] as Rgb8, rgb[ib] as Rgb8);
      const nt = dE(atNight(ia), atNight(ib));
      f.push({ lint: "diagonal edge contrast", level: Math.min(day, nt) >= 0.05 ? "info" : "warn", subject: `${name}: Oklab ΔE by day / at night (moods)`, value: `${day.toFixed(3)} / ${nt.toFixed(3)}`, limit: "≥ 0.05" });
    }
  }
  return f;
}
