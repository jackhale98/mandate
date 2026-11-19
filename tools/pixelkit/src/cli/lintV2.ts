// Round 4 lints for layout v2 (TOWN_GENERATION.md §7.2; PIXEL_ART_TOOL.md
// "Round 4"): palette headroom (D11), coverage of every v2 terrain code
// (through the painter and through the plain `ground` path), connector kits
// in both axes, every plot state in every building family, seams of the
// creek and rail sets, colours per tile, and the contrast the new edges keep
// by day and at night (palette moods).
import { BLOB_MASKS } from "../core/autotile";
import { hexToRgb, rgbToLab, type Rgb8 } from "../core/color";
import type { Indexed } from "../core/image";
import { colourCount, offPalette } from "../core/lint";
import { buildFacade, PLOT_STATES } from "../../art/mandate/buildings";
import type { ArtPack } from "../../art/mandate/pack";
import { FAMILY_SPECS } from "../../art/mandate/samples";
import { TERRAIN as K, TerrainPainter, tileOf, V2_CODES, V2_NAMES, type TerrainView } from "../../art/mandate/terrain";
import { allPieces, bridgePieces, creekPiece, railPiece } from "../../art/mandate/v2pieces";
import { v2Scene } from "../../art/mandate/v2scene";
import type { Finding } from "./lint";
import { groundImage } from "./scene";

/** The 16×16 cell (cx, cy) of a whole-map ground image. */
function cellOf(img: Indexed, cx: number, cy: number): Indexed {
  const out = { w: 16, h: 16, px: new Uint8Array(256) };
  for (let y = 0; y < 16; y++) for (let x = 0; x < 16; x++) out.px[y * 16 + x] = img.px[(cy * 16 + y) * img.w + cx * 16 + x] as number;
  return out;
}

const share = (a: Indexed, b: Indexed) => {
  let d = 0;
  for (let i = 0; i < a.px.length; i++) if (a.px[i] !== b.px[i]) d++;
  return d / a.px.length;
};

export function lintV2(pack: ArtPack): Finding[] {
  const f: Finding[] = [];
  const pal = pack.palette;
  const n = pal.colors.length - 1;
  const L = pack.terrain.legend;

  // ---- palette headroom (D11): about 10 colours free for new looks
  f.push({ lint: "palette headroom", level: 128 - n >= 10 ? "info" : "warn", subject: `colours free in the 128 budget (${n} used)`, value: 128 - n, limit: "≥ 10" });

  // ---- coverage: every v2 code draws, opaque and on the palette, through the plain tile path and the painter
  {
    let bad = 0;
    let worst = 0;
    let worstAt = "";
    for (const code of V2_CODES) {
      let codeWorst = 0;
      for (const m of BLOB_MASKS) {
        for (const v of [0, 6]) {
          const t = tileOf(pack.tiles, pack.terrain, code, m, v);
          if (t.w !== 16 || t.h !== 16 || t.px.some((p) => p === 0) || offPalette(t, n)) bad++;
          codeWorst = Math.max(codeWorst, colourCount(t));
        }
      }
      if (codeWorst > worst) {
        worst = codeWorst;
        worstAt = V2_NAMES[code] as string;
      }
      f.push({ lint: "colours per tile", level: codeWorst <= 8 ? "info" : "warn", subject: `v2 tile ${V2_NAMES[code]} (47 masks × 2 variants, drawn alone)`, value: codeWorst, limit: "≤ 8 (v1 tiles: ≤ 6)" });
    }
    f.push({ lint: "v2 coverage", level: bad ? "error" : "info", subject: `every v2 terrain code (${V2_CODES.length}) through ground(kind, mask, variant): 47 masks × 2 variants, 16×16, opaque, palette colours (most colours: ${worstAt}, ${worst})`, value: bad, limit: "0 bad tiles" });
    // Through the painter, in context: every cell of the pieces and the test scene; every code must appear.
    const seen = new Set<number>();
    let badCells = 0;
    let cells = 0;
    for (const view of [...allPieces(), v2Scene()] as TerrainView[]) {
      const img = groundImage(pack, view);
      if (offPalette(img, n) || img.px.some((p) => p === 0)) badCells++;
      for (let i = 0; i < view.width * view.height; i++) seen.add(view.terrain[i] as number);
      cells += view.width * view.height;
    }
    const missing = V2_CODES.filter((c) => !seen.has(c)).map((c) => V2_NAMES[c]);
    f.push({ lint: "v2 coverage", level: badCells || missing.length ? "error" : "info", subject: `${cells} cells of the review pieces and the v2 scene, in context (codes not shown: ${missing.join(", ") || "none"})`, value: badCells, limit: "0 maps with gaps or off-palette pixels" });
    // An unknown code (a newer layout than this pack) draws as grass, never throws.
    let threw = 0;
    try {
      const t = new Uint8Array(9).fill(31);
      new TerrainPainter(pack.tiles, pack.terrain, { width: 3, height: 3, terrain: t, elevation: new Uint8Array(9).fill(7) }).draw(1, 1, 0);
      tileOf(pack.tiles, pack.terrain, 31, 255, 0);
    } catch {
      threw = 1;
    }
    f.push({ lint: "v2 coverage", level: threw ? "error" : "info", subject: "an unknown terrain code and an out-of-range level draw a fallback", value: threw, limit: "0 throws" });
  }

  // ---- connector kits: every bridge kind in both axes draws its deck and what it shows beyond it
  {
    const problems: string[] = [];
    const decks = new Map<string, Indexed>();
    for (const p of bridgePieces()) {
      const c = p.connectors[0];
      if (!c) continue;
      const with_ = groundImage(pack, p);
      const plain = groundImage(pack, { ...p, connectors: [], terrain: p.terrain.map((t) => (t === K.Bridge ? K.Water : t)) });
      const deck = cellOf(with_, c.x + (c.axis === "ew" ? 1 : 0), c.y + (c.axis === "ns" ? 1 : 0));
      decks.set(`${c.kind} ${c.axis}`, deck);
      if (share(deck, cellOf(plain, c.x + (c.axis === "ew" ? 1 : 0), c.y + (c.axis === "ns" ? 1 : 0))) < 0.5) problems.push(`${p.label}: no deck`);
      const beyond = c.axis === "ew" ? [c.x + 1, c.y + c.h] : [c.x + c.w, c.y + 1];
      const d = share(cellOf(with_, beyond[0] as number, beyond[1] as number), cellOf(plain, beyond[0] as number, beyond[1] as number));
      if (d < (c.axis === "ew" ? 0.2 : 0.1)) problems.push(`${p.label}: no ${c.axis === "ew" ? "south face" : "shadow"} (${d.toFixed(2)})`);
    }
    for (const axis of ["ew", "ns"]) {
      const kinds = ["bridge", "footbridge", "viaduct"];
      for (let i = 0; i < kinds.length; i++) for (let j = i + 1; j < kinds.length; j++) {
        const a = decks.get(`${kinds[i]} ${axis}`);
        const b = decks.get(`${kinds[j]} ${axis}`);
        if (a && b && share(a, b) < 0.2) problems.push(`${kinds[i]} and ${kinds[j]} (${axis}) look alike`);
      }
    }
    // The ford and the level crossing: in the creek and rail pieces.
    const cp = creekPiece();
    const ford = cp.connectors.find((c) => c.kind === "ford");
    if (ford) {
      const a = groundImage(pack, cp);
      const b = groundImage(pack, { ...cp, terrain: cp.terrain.map((t) => (t === K.Ford ? K.Creek : t)) });
      if (share(cellOf(a, ford.x, ford.y), cellOf(b, ford.x, ford.y)) < 0.1) problems.push("ford: looks like the creek");
    }
    const rp = railPiece();
    const lc = rp.connectors.find((c) => c.kind === "level_crossing");
    if (lc) {
      const cell = cellOf(groundImage(pack, rp), lc.x, lc.y);
      if (!cell.px.includes(L.m as number)) problems.push("level crossing: no rails");
    }
    f.push({ lint: "connector kits", level: problems.length ? "error" : "info", subject: `bridge, footbridge and viaduct in both axes (deck; south face or shadow; distinct kits), ford, level crossing${problems.length ? `: ${problems.join("; ")}` : ""}`, value: problems.length, limit: "0" });
  }

  // ---- plot states: every state in every family, the building's frame, no lights when vacant, visibly different
  {
    const problems: string[] = [];
    for (const [fam, spec] of FAMILY_SPECS) {
      const built = buildFacade(spec, pal);
      for (const state of PLOT_STATES) {
        if (state === "built") continue;
        const b = buildFacade({ ...spec, conditions: [state] }, pal);
        if (b.albedo.w !== built.albedo.w || b.albedo.h !== built.albedo.h || b.ay !== built.ay) problems.push(`${fam} ${state}: frame`);
        if (b.doors.length < 1) problems.push(`${fam} ${state}: no door`);
        if (!b.shadows.every((s) => s.height > 0) || b.shadows.length < 1) problems.push(`${fam} ${state}: shadow`);
        if (b.receivers.length !== b.albedo.w * b.albedo.h) problems.push(`${fam} ${state}: receivers`);
        if ([...b.windows, ...b.doors].some((o) => o.emissive)) problems.push(`${fam} ${state}: lit`);
        // A boarded building is the building with its openings boarded (and dark at night); the others change more.
        const changed = share(b.albedo, built.albedo);
        if (changed < (state === "boarded" ? 0.015 : 0.05)) problems.push(`${fam} ${state}: looks built (${(changed * 100).toFixed(1)}% of pixels differ)`);
        if (offPalette(b.albedo, n)) problems.push(`${fam} ${state}: off palette`);
      }
    }
    f.push({ lint: "plot states", level: problems.length ? "error" : "info", subject: `reserved, construction, boarded, derelict × ${FAMILY_SPECS.map(([x]) => x).join(", ")}${problems.length ? `: ${problems.join("; ")}` : ""}`, value: problems.length, limit: "0" });
  }

  // ---- seams: the creek's water and the rails continue across cell edges
  {
    const water = new Set([L.w, L.W, L.v, L.z].map((x) => x as number));
    let creekBad = 0;
    let railBad = 0;
    let pairs = 0;
    for (const view of [...allPieces(), v2Scene()] as TerrainView[]) {
      const img = groundImage(pack, view);
      const W = view.width;
      const at = (x: number, y: number) => view.terrain[y * W + x] as number;
      const el = (x: number, y: number) => (view.elevation?.[y * W + x] as number | undefined) ?? 0;
      const creek = (t: number) => t === K.Creek || t === K.Ford;
      for (let y = 0; y < view.height; y++) {
        for (let x = 0; x < W; x++) {
          if (!creek(at(x, y))) continue;
          for (const [dx, dy] of [
            [1, 0],
            [0, 1],
          ] as const) {
            // A creek that drops a level runs over a weir: no seam to check there.
            if (x + dx >= W || y + dy >= view.height || !creek(at(x + dx, y + dy)) || el(x, y) !== el(x + dx, y + dy)) continue;
            pairs++;
            for (let k = 0; k < 16; k++) {
              const a = dx ? img.px[(y * 16 + k) * img.w + x * 16 + 15] : img.px[(y * 16 + 15) * img.w + x * 16 + k];
              const b = dx ? img.px[(y * 16 + k) * img.w + x * 16 + 16] : img.px[(y * 16 + 16) * img.w + x * 16 + k];
              if (water.has(a as number) !== water.has(b as number)) creekBad++;
            }
          }
        }
      }
      // Rail heads: every head pixel on a rail cell touches another (no breaks at cell edges or bends).
      const head = L.m as number;
      for (let y = 0; y < img.h; y++) {
        for (let x = 0; x < img.w; x++) {
          if (img.px[y * img.w + x] !== head || at(x >> 4, y >> 4) !== K.Rail) continue;
          let nb = 0;
          for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) if ((dx || dy) && img.px[(y + dy) * img.w + x + dx] === head) nb++;
          if (nb === 0) railBad++;
        }
      }
    }
    f.push({ lint: "v2 seams", level: creekBad ? "error" : "info", subject: `creek water across ${pairs} joined cell edges (rows or columns that are water on one side only)`, value: creekBad, limit: "0" });
    f.push({ lint: "v2 seams", level: railBad ? "error" : "info", subject: "isolated rail-head pixels on rail cells (a break at a cell edge or bend)", value: railBad, limit: "0" });
  }

  // ---- edges keep their contrast by day and at night (palette moods, 22:30): Oklab ΔE between what meets at the new edges
  {
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
      ["rail head / ballast", "m", "P"],
      ["sleeper / ballast", "D", "P"],
      ["ballast edge / grass", "Q", "g"],
      ["wall coping / water", "q", "w"],
      ["bank earth / grass", "e", "g"],
      ["bank foot / grass", "H", "g"],
      ["rock lit edge / rock", "q", "P"],
      ["parapet / carriageway", "t", "a"],
      ["creek / grass", "w", "g"],
      ["arch shade / spandrel", "A", "s"],
    ];
    for (const [name, a, b] of pairs) {
      const ia = L[a] as number;
      const ib = L[b] as number;
      const day = dE(rgb[ia] as Rgb8, rgb[ib] as Rgb8);
      const nt = dE(atNight(ia), atNight(ib));
      f.push({ lint: "v2 edge contrast", level: Math.min(day, nt) >= 0.05 ? "info" : "warn", subject: `${name}: Oklab ΔE by day / at night (moods)`, value: `${day.toFixed(3)} / ${nt.toFixed(3)}`, limit: "≥ 0.05" });
    }
  }
  return f;
}
