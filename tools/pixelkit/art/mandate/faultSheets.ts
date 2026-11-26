// Review sheets for the fault and repair cues (task B4, DEC-114). Run from tools/pixelkit:
//   node --import ./bin/hooks.mjs art/mandate/faultSheets.ts [--out DIR]
// Writes:
//   faults.png     each fault kind of content/base/faults.jsonc on a patch of
//                  its street (pavement over road, or grass): as the plan has
//                  it, open, being repaired and new, by day and at night, at 3×
//   faults-1x.png  the same at 1× (the street's real size; the phone draws it
//                  at 2–3×)
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { hexToRgb, type Rgb8 } from "../../src/core/color";
import { blank, blit, type Indexed } from "../../src/core/image";
import { encodePng, type Rgba } from "../../src/node/png";
import { Sheet } from "../../src/node/sheet";
import { REPO_ROOT } from "./interiorsContent";
import { jsoncToJson } from "./interiorsRecipes";
import { buildPack } from "./pack";
import { groundTile } from "./tiles";

const args = process.argv.slice(2);
const outIdx = args.indexOf("--out");
const out = path.resolve(outIdx >= 0 ? (args[outIdx + 1] as string) : "target/pixelkit/faults");
mkdirSync(out, { recursive: true });

const pack = buildPack();
const rgb = pack.palette.colors.map(hexToRgb);
const LABEL: Rgb8 = [154, 160, 180];
const faults = (JSON.parse(jsoncToJson(readFileSync(path.join(REPO_ROOT, "content/base/faults.jsonc"), "utf8"))) as { value: { faults: { id: string; spot: { kind: string }; cue: { open: string; repairing: string; new: string } }[] } }).value.faults;

const png = (name: string, img: Rgba) => {
  writeFileSync(path.join(out, name), encodePng(img));
  console.log(`wrote ${path.join(out, name)} (${img.w}×${img.h})`);
};

const COLS = 5;
const ROWS = 4;
const T = 16;
/** Rows of each scene, top down: what lies behind the spot, the spot's row, and the rows in front. */
function rowsFor(kind: string): string[] {
  if (kind === "green") return ["grass", "grass", "grass", "pavement"];
  if (kind === "road" || kind === "kerb") return ["pavement", "pavement", "road", "road"];
  return ["pavement", "pavement", "road", "road"];
}
/** The spot's cell in the scene. */
const spotRow = (kind: string) => (kind === "road" || kind === "kerb" ? 2 : 1);

interface Scene {
  ground: Indexed;
  props: { prop: string; variant: number; cx: number; cy: number }[];
}

function scene(kind: string, prop: string | null): Scene {
  const ground = blank(COLS * T, ROWS * T);
  rowsFor(kind).forEach((t, y) => {
    for (let x = 0; x < COLS; x++) blit(ground, groundTile(pack.tiles, t, 255, (x * 3 + y) & 7), x * T, y * T);
  });
  const props: Scene["props"] = [];
  if (prop) props.push({ prop, variant: 0, cx: 1, cy: spotRow(kind) });
  else if (kind === "pavement") props.push({ prop: "lamp", variant: 0, cx: 1, cy: 1 });
  return { ground, props };
}

/** The scene as an image: the ground, then each prop with its anchor on its cell; and its lit parts. */
function compose(s: Scene, top: number): { img: Indexed; lit: Indexed } {
  const img = blank(s.ground.w, s.ground.h + top);
  const lit = blank(img.w, img.h);
  blit(img, s.ground, 0, top);
  for (const p of s.props) {
    const b = pack.props[p.prop]?.[p.variant];
    if (!b) throw new Error(`no prop ${p.prop}`);
    blit(img, b.albedo, p.cx * T - b.ax, top + p.cy * T - b.ay);
    if (b.emissive) blit(lit, b.emissive, p.cx * T - b.ax, top + p.cy * T - b.ay);
  }
  return { img, lit };
}

const night = pack.lighting.entries[Math.floor(1290 / pack.lighting.stepMinutes) % pack.lighting.entries.length];

function paint(sheet: Sheet, c: { img: Indexed; lit: Indexed }, x: number, y: number, scale: number, dark: boolean) {
  const m = dark && night ? night.mul : ([255, 255, 255] as const);
  const table = rgb.map((v) => [Math.round((v[0] * m[0]) / 255), Math.round((v[1] * m[1]) / 255), Math.round((v[2] * m[2]) / 255)] as Rgb8);
  sheet.paste(c.img, table, x, y, scale);
  if (dark) {
    // The lit parts at full strength, and the lamp's pool as a warm wash round them.
    for (let iy = 0; iy < c.lit.h; iy++) {
      for (let ix = 0; ix < c.lit.w; ix++) {
        if (!c.lit.px[iy * c.lit.w + ix]) continue;
        sheet.rect(x + (ix - 12) * scale, y + (iy + 14) * scale, 24 * scale, 6 * scale, [232, 180, 76], 0.006);
      }
    }
    sheet.paste(c.lit, rgb, x, y, scale);
  }
}

function sheetAt(scale: number): Sheet {
  const top = 24;
  const cw = COLS * T * scale;
  const ch = (ROWS * T + top) * scale;
  const pad = 8;
  const labelW = 140;
  const columns = ["as planned", "open", "being repaired", "new"];
  const s = new Sheet(labelW + (cw + pad) * columns.length * 2 + pad, 40 + (ch + 20) * faults.length);
  s.text("FAULT CUES: AS PLANNED, OPEN, BEING REPAIRED, NEW. BY DAY (LEFT), AT NIGHT (RIGHT)", pad, 8, [233, 234, 240], 1);
  faults.forEach((f, row) => {
    const y = 30 + row * (ch + 20);
    s.text(`${f.id} (${f.spot.kind})`, pad, y + ch / 2, LABEL, 1);
    const props = [null, `cue_${f.cue.open}`, `cue_${f.cue.repairing}`, `cue_${f.cue.new}`];
    for (const dark of [false, true]) {
      props.forEach((p, i) => {
        const x = labelW + ((dark ? columns.length : 0) + i) * (cw + pad);
        paint(s, compose(scene(f.spot.kind, p), top), x, y, scale, dark);
        if (row === 0) s.text(columns[i] as string, x, 22, LABEL, 1);
      });
    }
  });
  return s;
}

png("faults.png", sheetAt(3));
png("faults-1x.png", sheetAt(1));
