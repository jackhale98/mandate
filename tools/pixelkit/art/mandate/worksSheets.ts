// Review sheets for the works kit (art round 8, DEC-099). Run from tools/pixelkit:
//   node --import ./bin/hooks.mjs art/mandate/worksSheets.ts [--out DIR]
// Writes:
//   works-<kind>.png        each sample site across its phases (building 0, 50
//                           and 100 %, open, stalled, decay 1–3, cleared; mines
//                           and works also idle, working and busy), by day and
//                           in the evening, at 2×
//   works-<kind>-1x.png     the same at 1× (the street's real size)
//   works-before-after.png  the bridges (both axes) and the flood wall before
//                           the works and once open, day and evening, at 2×
//   works-parts.png         every part and frame of the kit, at 3×
//   works-lines.png         fences, barriers, railings, walls and levees in
//                           straight, corner and 45° pieces, standing,
//                           sagging and down, at 3×
//   works-plaque.png        the plaque and the opening ribbon at street zoom
//                           (1×, 2×, 3×), day and evening
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { hexToRgb, type Rgb8 } from "../../src/core/color";
import { encodePng, type Rgba } from "../../src/node/png";
import { Sheet } from "../../src/node/sheet";
import { buildPack } from "./pack";
import { worksLook, type WorksOptions, type WorksPhase } from "./works";
import { composeWorks, worksKit } from "./worksCompose";
import { drawRun, LINE_STYLES, rectPieces, runCentres, type LinePiece } from "./worksLines";
import { renderWorks, worksSamples, type Moment, type WorksSample } from "./worksScenes";

const args = process.argv.slice(2);
const outIdx = args.indexOf("--out");
const out = path.resolve(outIdx >= 0 ? (args[outIdx + 1] as string) : "target/pixelkit/works");
mkdirSync(out, { recursive: true });

const pack = buildPack();
const kit = worksKit(pack);
const rgb = pack.palette.colors.map(hexToRgb);
const BG: Rgb8 = [24, 26, 34];
const LABEL: Rgb8 = [154, 160, 180];

const png = (name: string, img: Rgba) => {
  writeFileSync(path.join(out, name), encodePng(img));
  console.log(`wrote ${path.join(out, name)} (${img.w}×${img.h})`);
};

interface Column {
  label: string;
  phase: WorksPhase | null;
  progress: number;
  months: number;
  opts?: WorksOptions;
}

/** The strip of a kind: building 0/50/100 %, open, stalled, decay 1–3, cleared (mines and works: idle, working, busy, then closed). */
function columns(kind: string): Column[] {
  const industry = kind === "mine" || kind === "factory";
  const stallAt = industry ? 100 : 50;
  const cols: Column[] = [
    { label: "building 0%", phase: "building", progress: 0, months: 0 },
    { label: "building 50%", phase: "building", progress: 50, months: 0 },
    { label: "building 100%", phase: "building", progress: 100, months: 0 },
  ];
  if (industry) {
    cols.push({ label: "open: idle", phase: "open", progress: 100, months: 0, opts: { activity: 10 } }, { label: "open: working", phase: "open", progress: 100, months: 0, opts: { activity: 50 } }, { label: "open: busy", phase: "open", progress: 100, months: 0, opts: { activity: 90, extraction: 90 } });
  } else cols.push({ label: "open", phase: "open", progress: 100, months: 0 });
  cols.push(
    { label: industry ? "closed" : "stalled at 50%", phase: "stalled", progress: stallAt, months: 1 },
    { label: "decay 1 (3 months)", phase: "stalled", progress: stallAt, months: 3 },
    { label: "decay 2 (9 months)", phase: "stalled", progress: stallAt, months: 9 },
    { label: "decay 3 (18 months)", phase: "stalled", progress: stallAt, months: 18 },
    { label: "cleared", phase: "cleared", progress: stallAt, months: 24 },
  );
  return cols;
}

function render(sample: WorksSample, col: Column | null, moment: Moment, scale: number, frame = 0) {
  if (!col || !col.phase) return renderWorks(pack, sample, null, moment, scale, frame);
  const look = worksLook(sample.kind, col.phase, col.progress, col.months, sample.seed, col.opts);
  const works = composeWorks(look, sample.site, kit, sample.seed);
  return renderWorks(pack, sample, works, moment, scale, frame);
}

function strip(sample: WorksSample, scale: number): Sheet {
  const cols = columns(sample.kind);
  const cw = sample.window.w * 16 * scale;
  const ch = sample.window.h * 16 * scale;
  const gap = 6 * scale;
  const ls = scale >= 2 ? 2 : 1;
  const top = 22 + 12 * ls;
  const s = new Sheet(gap + cols.length * (cw + gap), top + 2 * (ch + gap + 12) + gap, BG);
  s.text(`WORKS: ${sample.kind.replace("_", " ")} (${sample.label}) ACROSS ITS PHASES, BY DAY AND IN THE EVENING (20:00), ${scale}x`, gap, 6, [233, 234, 240], ls);
  (["day", "evening"] as const).forEach((m, r) => {
    cols.forEach((c, i) => {
      const x = gap + i * (cw + gap);
      const y = top + r * (ch + gap + 12);
      s.text(`${c.label}${r ? " (evening)" : ""}`, x, y - 9 * ls + 2, LABEL, 1);
      s.pasteRgba(render(sample, c, m, scale), x, y + 4);
    });
  });
  return s;
}

const samples = worksSamples();
for (const sample of samples) {
  const name = sample.kind + (samples.filter((x) => x.kind === sample.kind).indexOf(sample) ? "-diagonal" : "");
  png(`works-${name}.png`, strip(sample, 2));
  png(`works-${name}-1x.png`, strip(sample, 1));
}

// ---- before and after: both bridge axes and the flood wall
{
  const bridgeNs: WorksSample = { ...(samples[0] as WorksSample), label: "the New Bridge", site: { connector: { kind: "bridge", x: 14, y: 18, w: 2, h: 3, axis: "ns" }, yard: { x: 16, y: 16, w: 3, h: 1 }, front: [13, 17], entrance: [13, 17] }, window: { x: 9, y: 14, w: 11, h: 9 }, seed: 0x0b1e };
  const pairs: WorksSample[] = [samples[0] as WorksSample, bridgeNs, samples.find((x) => x.kind === "flood_wall") as WorksSample];
  const scale = 2;
  const ws = pairs.map((p) => p.window.w * 16 * scale);
  const hs = pairs.map((p) => p.window.h * 16 * scale);
  const W = 12 + 4 * (Math.max(...ws) + 12);
  const sheet = new Sheet(W, 30 + pairs.reduce((a, _p, i) => a + (hs[i] as number) + 30, 0), BG);
  sheet.text("BEFORE AND AFTER: THE WORKS OPEN (BY DAY, THEN IN THE EVENING), 2x", 12, 8);
  let y = 34;
  pairs.forEach((p, i) => {
    const open: Column = { label: "after", phase: "open", progress: 100, months: 0 };
    const cells: [string, Column | null, Moment][] = [
      ["before", null, "day"],
      ["after", open, "day"],
      ["before (evening)", null, "evening"],
      ["after (evening)", open, "evening"],
    ];
    cells.forEach(([label, c, m], k) => {
      const x = 12 + k * ((ws[i] as number) + 12);
      sheet.text(`${p.label}: ${label}`, x, y - 2, LABEL, 1);
      sheet.pasteRgba(render(p, c, m, scale), x, y + 8);
    });
    y += (hs[i] as number) + 30;
  });
  png("works-before-after.png", sheet);
}

// ---- the parts
{
  const parts = Object.values(kit.parts);
  const scale = 3;
  const maxW = 1900;
  let x = 10;
  let y = 34;
  let rowH = 0;
  const placed: { img: (typeof parts)[number]["frames"][number]; x: number; y: number; label: string }[] = [];
  for (const p of parts) {
    p.frames.forEach((f, k) => {
      const w = f.w * scale;
      if (x + w > maxW) {
        x = 10;
        y += rowH + 22;
        rowH = 0;
      }
      placed.push({ img: f, x, y, label: p.frames.length > 1 ? `${p.id} ${k + 1}` : p.id });
      x += Math.max(w, p.id.length * 4 + 8) + 10;
      rowH = Math.max(rowH, f.h * scale);
    });
  }
  const sheet = new Sheet(maxW + 10, y + rowH + 30, BG);
  sheet.text("WORKS KIT PARTS (EXTERIOR OUTLINE: LATTICES AND MESH STAY OPEN), 3x, ON PAVING", 10, 8);
  const paving = rgb[pack.terrain.legend.p as number] ?? [170, 162, 147];
  for (const p of placed) {
    sheet.rect(p.x - 2, p.y - 2, p.img.w * scale + 4, p.img.h * scale + 4, paving);
    sheet.paste(p.img, rgb, p.x, p.y, scale);
    sheet.text(p.label, p.x, p.y + p.img.h * scale + 4, LABEL, 1);
  }
  png("works-parts.png", sheet);
}

// ---- line pieces: every style in straight, corner and 45° pieces, then worn
{
  const run: LinePiece[] = [
    { x: 0, y: 2, a: "w", b: "e" },
    { x: 1, y: 2, a: "w", b: "n" },
    { x: 1, y: 1, a: "s", b: "e" },
    { x: 2, y: 1, a: "w", b: "e" },
    { x: 3, y: 1, a: "w", b: "s", sharp: true },
    { x: 3, y: 2, a: "n", b: "s" },
    { x: 3, y: 3, a: "n", b: "e" },
    { x: 4, y: 3, a: "w", b: "s" },
    { x: 4, y: 4, a: "n", b: "e" },
    { x: 5, y: 4, a: "w", b: "e" },
  ];
  const box = rectPieces(0, 0, 2, 1, [[1, 1]]).map((p) => ({ ...p, x: p.x + 7, y: p.y + 2 }));
  const scale = 3;
  const cw = 11 * 16 * scale;
  const chh = 6 * 16 * scale;
  const styles = LINE_STYLES;
  const sheet = new Sheet(12 + 3 * (cw + 12), 34 + styles.length * (chh + 24), BG);
  sheet.text("RUNS ON THE STREET GRID'S LINES (MID-SIDES): STRAIGHT, 45 DEGREE, SHARP CORNERS; STANDING, SAGGING, DOWN; 3x", 12, 8);
  const pave = rgb[pack.terrain.legend.p as number] ?? [170, 162, 147];
  const grass = rgb[pack.terrain.legend.g as number] ?? [94, 150, 70];
  styles.forEach((st, r) => {
    [0, 1, 2].forEach((wear, c) => {
      const x0 = 12 + c * (cw + 12);
      const y0 = 34 + r * (chh + 24);
      sheet.rect(x0, y0, cw, chh, r % 2 ? pave : grass);
      // The cell grid, faintly.
      for (let gx = 0; gx <= 11; gx++) sheet.rect(x0 + gx * 16 * scale, y0, 1, chh, [0, 0, 0], 0.15);
      for (let gy = 0; gy <= 6; gy++) sheet.rect(x0, y0 + gy * 16 * scale, cw, 1, [0, 0, 0], 0.15);
      for (const pieces of [run, box]) {
        const sprites = drawRun(pack.palette, pack.lightness, st, runCentres(pieces), 7, () => wear).sort((a, b) => a.base - b.base);
        for (const sp of sprites) sheet.paste(sp.img, rgb, x0 + sp.x * scale, y0 + sp.y * scale + 16 * scale, scale);
      }
      sheet.text(`${st} ${["standing", "sagging", "down"][wear]}`, x0, y0 + chh + 4, LABEL, 1);
    });
  });
  png("works-lines.png", sheet);
}

// ---- the plaque and the ribbon at street zoom
{
  const civic = samples.find((x) => x.kind === "civic") as WorksSample;
  const bridge = samples[0] as WorksSample;
  const zoom = (sm: WorksSample, win: WorksSample["window"]): WorksSample => ({ ...sm, window: win });
  const shots: [string, WorksSample, Column][] = [
    ["clinic: the plaque by the door", zoom(civic, { x: 28, y: 21, w: 6, h: 5 }), { label: "open", phase: "open", progress: 100, months: 0 }],
    ["clinic: the ribbon at the opening", zoom(civic, { x: 28, y: 21, w: 6, h: 5 }), { label: "", phase: "building", progress: 100, months: 0 }],
    ["old bridge: the plaque at its end", zoom(bridge, { x: 23, y: 12, w: 6, h: 5 }), { label: "open", phase: "open", progress: 100, months: 0 }],
    ["old bridge: the ribbon", zoom(bridge, { x: 23, y: 12, w: 6, h: 5 }), { label: "", phase: "building", progress: 100, months: 0 }],
  ];
  const scales = [1, 2, 3];
  const colW = scales.reduce((a, s) => a + 6 * 16 * s + 12, 0);
  const sheet = new Sheet(12 + 2 * colW, 34 + shots.length * (5 * 16 * 3 + 30), BG);
  sheet.text("THE SPONSOR'S PLAQUE AND THE OPENING RIBBON AT STREET ZOOM: 1x, 2x, 3x; DAY, THEN EVENING", 12, 8);
  shots.forEach(([label, sm, col], r) => {
    let x = 12;
    const y = 34 + r * (5 * 16 * 3 + 30);
    sheet.text(label, x, y, LABEL, 1);
    for (const m of ["day", "evening"] as const) {
      for (const s of scales) {
        sheet.pasteRgba(render(sm, col, m, s), x, y + 10);
        x += 6 * 16 * s + 12;
      }
    }
  });
  png("works-plaque.png", sheet);
}
