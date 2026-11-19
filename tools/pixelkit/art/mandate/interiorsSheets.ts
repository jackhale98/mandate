// Review sheets for the interiors (art round 7). Run from tools/pixelkit:
//   node --import ./bin/hooks.mjs art/mandate/interiorsSheets.ts [--out DIR]
// Writes:
//   interiors-<room>.png   one room at four seeds, by day (11:00) and in the
//                          evening with the lamps on (20:00), with placeholder
//                          people on their spots, at 2×
//   interiors-1x.png       the same twelve rooms by day and evening at 1×
//   interiors-phone.png    one seed of each room at 3× (a phone's zoom)
//   interiors-times.png    one seed of each room through the day (08:00,
//                          12:30, 17:45, 19:30, 22:30)
//   interiors-debug.png    the walk grid, spots (by role, with facing) and
//                          hotspots (with the tile to stand on)
//   interiors-parts.png    every part and surface, recoloured from one room's
//                          materials, at 4×
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { hexToRgb, type Rgb8 } from "../../src/core/color";
import { blank, blit, outline, type Indexed } from "../../src/core/image";
import { encodePng } from "../../src/node/png";
import { Sheet } from "../../src/node/sheet";
import { composeInterior, placeholderPeople, renderInterior, type InteriorRoom, type RgbaImage } from "./interiors";
import { FIXED_RAMPS, ICHARS, ILEGEND, PARTS, partSlots, SCODES, SLEGEND, SURFACES, surfaceSlots } from "./interiorsParts";
import { RECIPES, type InteriorRecipe } from "./interiorsRecipes";
import { buildPack } from "./pack";
import { INTERIOR_SEEDS, INTERIOR_TIMES } from "./samples";

const args = process.argv.slice(2);
const outIdx = args.indexOf("--out");
const out = path.resolve(outIdx >= 0 ? (args[outIdx + 1] as string) : "target/pixelkit/interiors");
mkdirSync(out, { recursive: true });

const pack = buildPack();
const pal = pack.palette;
const rgb = pal.colors.map(hexToRgb);
const SEEDS = INTERIOR_SEEDS;
const DAY = INTERIOR_TIMES.day;
const EVENING = INTERIOR_TIMES.evening;
const BG: Rgb8 = [24, 26, 34];
const LABEL: Rgb8 = [154, 160, 180];
const NAMES: Record<string, string> = { council_chamber: "chamber", town_hall: "town-hall", diner: "diner" };

const png = (name: string, s: Sheet) => {
  writeFileSync(path.join(out, name), encodePng(s));
  console.log(`wrote ${path.join(out, name)} (${s.w}×${s.h})`);
};

function pasteRgba(s: Sheet, img: RgbaImage, x0: number, y0: number, scale: number) {
  for (let y = 0; y < img.h; y++) {
    for (let x = 0; x < img.w; x++) {
      const i = (y * img.w + x) * 4;
      s.rect(x0 + x * scale, y0 + y * scale, scale, scale, [img.data[i] as number, img.data[i + 1] as number, img.data[i + 2] as number]);
    }
  }
}

const room = (r: InteriorRecipe, seed: number) => composeInterior(r, seed, pal, pack.lightness);
const people = (rm: InteriorRoom, seed: number) => placeholderPeople(rm, pal, pack.lightness, seed, 0.75);
const render = (rm: InteriorRoom, seed: number, minutes: number) => renderInterior(rm, pal, pack.lighting, minutes, people(rm, seed));
const clock = (m: number) => `${String(Math.floor(m / 60)).padStart(2, "0")}:${String(m % 60).padStart(2, "0")}`;
const picks = (rm: InteriorRoom) => `${rm.picks.wall} / ${rm.picks.floor} / ${Object.entries(rm.picks).filter(([k]) => k.startsWith("choice:")).map(([, v]) => v).join(" ")}`;

// ---- one sheet per room: four seeds, day and evening, 2×
for (const r of RECIPES) {
  const scale = 2;
  const sample = room(r, SEEDS[0] as number);
  const cw = sample.w * scale;
  const ch = sample.h * scale;
  const s = new Sheet(16 + SEEDS.length * (cw + 16), 40 + 2 * (ch + 40), BG);
  s.text(`${r.id.toUpperCase()} V${r.version}: FOUR SEEDS, DAY (${clock(DAY)}) AND EVENING WITH THE LAMPS ON (${clock(EVENING)}), 2×`, 8, 8);
  SEEDS.forEach((seed, i) => {
    const rm = room(r, seed);
    [DAY, EVENING].forEach((m, j) => {
      const x = 8 + i * (cw + 16);
      const y = 32 + j * (ch + 40);
      s.text(`seed ${seed} ${clock(m)}`, x, y, LABEL, 2);
      s.text(picks(rm), x, y + 14, LABEL, 1);
      pasteRgba(s, render(rm, seed, m), x, y + 24, scale);
    });
  });
  png(`interiors-${NAMES[r.place] ?? r.place}.png`, s);
}

// ---- all twelve at 1×, day and evening
{
  const rows = RECIPES.map((r) => SEEDS.map((seed) => room(r, seed)));
  const cw = Math.max(...rows.flat().map((rm) => rm.w));
  const ch = Math.max(...rows.flat().map((rm) => rm.h));
  const s = new Sheet(16 + SEEDS.length * 2 * (cw + 8), 30 + RECIPES.length * (ch + 18), BG);
  s.text("THE THREE ROOMS AT FOUR SEEDS, 1× (REAL SIZE): DAY AND EVENING SIDE BY SIDE", 8, 8);
  rows.forEach((list, ri) => {
    list.forEach((rm, i) => {
      [DAY, EVENING].forEach((m, j) => {
        const x = 8 + (i * 2 + j) * (cw + 8);
        const y = 26 + ri * (ch + 18);
        s.text(`${rm.recipe.replace("interior.", "")} ${SEEDS[i]} ${clock(m)}`, x, y, LABEL, 1);
        pasteRgba(s, render(rm, SEEDS[i] as number, m), x, y + 8, 1);
      });
    });
  });
  png("interiors-1x.png", s);
}

// ---- a phone's zoom: 3×
{
  const scale = 3;
  const list = RECIPES.map((r, i) => room(r, SEEDS[(i + 1) % SEEDS.length] as number));
  const cw = Math.max(...list.map((rm) => rm.w)) * scale;
  const ch = Math.max(...list.map((rm) => rm.h)) * scale;
  const s = new Sheet(16 + 2 * (cw + 16), 30 + list.length * (ch + 30), BG);
  s.text("PHONE ZOOM (3×): ONE SEED OF EACH ROOM, DAY AND EVENING", 8, 8);
  list.forEach((rm, i) => {
    [DAY, EVENING].forEach((m, j) => {
      const x = 8 + j * (cw + 16);
      const y = 28 + i * (ch + 30);
      s.text(`${rm.recipe} seed ${rm.seed} ${clock(m)}`, x, y, LABEL, 2);
      pasteRgba(s, render(rm, rm.seed, m), x, y + 14, scale);
    });
  });
  png("interiors-phone.png", s);
}

// ---- through the day
{
  const times = [8 * 60, 12 * 60 + 30, 17 * 60 + 45, 19 * 60 + 30, 22 * 60 + 30];
  const scale = 2;
  const list = RECIPES.map((r, i) => room(r, SEEDS[(i + 2) % SEEDS.length] as number));
  const cw = Math.max(...list.map((rm) => rm.w)) * scale;
  const ch = Math.max(...list.map((rm) => rm.h)) * scale;
  const s = new Sheet(16 + times.length * (cw + 12), 30 + list.length * (ch + 24), BG);
  s.text("THROUGH THE DAY: MORNING, NOON, GOLDEN HOUR, DUSK, NIGHT (THE STREET'S LIGHTING TABLE)", 8, 8);
  list.forEach((rm, i) => {
    times.forEach((m, j) => {
      const x = 8 + j * (cw + 12);
      const y = 26 + i * (ch + 24);
      s.text(`${rm.recipe.replace("interior.", "")} ${clock(m)}`, x, y, LABEL, 2);
      pasteRgba(s, render(rm, rm.seed, m), x, y + 14, scale);
    });
  });
  png("interiors-times.png", s);
}

// ---- debug: walk grid, spots, hotspots
{
  const ROLE: Record<string, Rgb8> = { chair: [255, 210, 80], clerk: [120, 200, 255], councillor: [120, 230, 140], player: [255, 120, 60], staff: [160, 170, 255], owner: [255, 110, 160], cook: [255, 160, 110], waiter: [200, 140, 255], patron: [90, 230, 220], public: [230, 230, 230] };
  const scale = 3;
  const combos = RECIPES.flatMap((r) => SEEDS.slice(0, 2).map((seed) => room(r, seed)));
  const cw = Math.max(...combos.map((rm) => rm.w)) * scale;
  const ch = Math.max(...combos.map((rm) => rm.h)) * scale;
  const cols = 2;
  const s = new Sheet(16 + cols * (cw + 16), 60 + Math.ceil(combos.length / cols) * (ch + 30), BG);
  s.text("DEBUG: BLOCKED TILES (RED), SEATS (BLUE), DOOR (GREEN); SPOTS BY ROLE WITH A FACING TICK; HOTSPOTS (YELLOW) AND THEIR STAND TILE (X)", 8, 8);
  let lx = 8;
  for (const [role, c] of Object.entries(ROLE)) {
    s.rect(lx, 24, 10, 10, c);
    s.text(role, lx + 14, 26, LABEL, 1);
    lx += 14 + role.length * 4 + 16;
  }
  combos.forEach((rm, k) => {
    const x0 = 8 + (k % cols) * (cw + 16);
    const y0 = 50 + Math.floor(k / cols) * (ch + 30);
    s.text(`${rm.recipe} seed ${rm.seed}: ${picks(rm)}`, x0, y0, LABEL, 1);
    const img = renderInterior(rm, pal, pack.lighting, DAY, []);
    // Dim the room so the markers read.
    for (let i = 0; i < img.w * img.h; i++) for (let c = 0; c < 3; c++) img.data[i * 4 + c] = Math.round((img.data[i * 4 + c] as number) * 0.55);
    pasteRgba(s, img, x0, y0 + 10, scale);
    const tile = (tx: number, ty: number) => ({ x: x0 + (rm.floor.x + tx * 16) * scale, y: y0 + 10 + (rm.floor.y + ty * 16) * scale });
    for (let ty = 0; ty < rm.rows; ty++) {
      for (let tx = 0; tx < rm.cols; tx++) {
        const v = rm.walk[ty * rm.cols + tx];
        const p = tile(tx, ty);
        if (v === 0) s.rect(p.x + 2, p.y + 2, 16 * scale - 4, 16 * scale - 4, [230, 60, 60], 0.35);
        else if (v === 2) s.rect(p.x + 2, p.y + 2, 16 * scale - 4, 16 * scale - 4, [70, 120, 255], 0.35);
        if (tx === rm.door.x && ty === rm.door.y) s.rect(p.x + 2, p.y + 2, 16 * scale - 4, 16 * scale - 4, [60, 220, 90], 0.5);
      }
    }
    for (const h of rm.hotspots) {
      const p = tile(h.x, h.y);
      const w = h.w * 16 * scale;
      const hh = h.h * 16 * scale;
      const c: Rgb8 = h.late ? [190, 120, 255] : [255, 220, 60];
      s.rect(p.x, p.y, w, 2, c);
      s.rect(p.x, p.y + hh - 2, w, 2, c);
      s.rect(p.x, p.y, 2, hh, c);
      s.rect(p.x + w - 2, p.y, 2, hh, c);
      const st = tile(h.stand.x, h.stand.y);
      for (let d = 0; d < 16 * scale - 12; d++) {
        s.rect(st.x + 6 + d, st.y + 6 + d, 3, 3, c);
        s.rect(st.x + 16 * scale - 9 - d, st.y + 6 + d, 3, 3, c);
      }
      s.text([...(h.actions ?? []), ...(h.services ?? [])].join(" ").slice(0, 24), p.x + 4, p.y + 4, c, 1);
    }
    for (const sp of rm.spots) {
      const p = tile(sp.x, sp.y);
      const c = ROLE[sp.role] ?? [255, 255, 255];
      const cx = p.x + 8 * scale;
      const cy = p.y + 8 * scale;
      s.rect(cx - 8, cy - 8, 16, 16, c);
      s.rect(cx - 5, cy - 5, 10, 10, sp.pose === "sit" ? [40, 60, 140] : c);
      const [dx, dy] = sp.facing === "up" ? [0, -1] : sp.facing === "down" ? [0, 1] : sp.facing === "left" ? [-1, 0] : [1, 0];
      for (let k = 8; k < 18; k++) s.rect(cx + dx * k - 2, cy + dy * k - 2, 4, 4, c);
    }
  });
  png("interiors-debug.png", s);
}

// ---- parts: every drawing, recoloured from one room's materials
{
  const scale = 4;
  // Recolour through a fixed set of roles (oak, red vinyl, navy and gold, cream wall, stone trim).
  const roleRamp: Record<string, [string, number]> = { wood: ["wood", 0], uph: ["cloth_red", 0], acc: ["cloth_navy", 0], acc2: ["cloth_mustard", 0], wall: ["render", 0], trim: ["stone", 0] };
  const rampStep = (name: string, k: number) => {
    const rr = pal.ramps.find((x) => x.name === name);
    if (!rr) throw new Error(name);
    return rr.steps[Math.max(0, Math.min(rr.steps.length - 1, rr.anchorStep + k))] as number;
  };
  const table = new Uint8Array(256);
  ICHARS.forEach((chr, code) => {
    if (!chr) return;
    const [role, k] = ILEGEND[chr] as readonly [string, number];
    const rr = roleRamp[role];
    table[code] = rr ? rampStep(rr[0], k) : rampStep(FIXED_RAMPS[role] as string, k);
  });
  const cells: { img: Indexed; label: string; outlined: boolean }[] = [];
  for (const p of PARTS) {
    const len = p.strip ? (p.strip.dir === "h" ? 3 : 2) : 1;
    const slots = partSlots(p.id, len);
    const img = blank(slots.w + 2, slots.h + 2);
    blit(img, slots, 1, 1, { map: table });
    const outlined = p.kind !== "wall" && p.kind !== "decal";
    cells.push({ img: outlined ? outline(img, pal.lineOf, pack.lightness) : img, label: `${p.id}${p.strip ? ` ×${len}` : ""}`, outlined });
  }
  const surfaceCells = SURFACES.map((sf) => {
    const t = surfaceSlots(sf.id);
    const sTable = new Uint8Array(256);
    const ramps = sf.id === "checker" ? ["chalk", "asphalt"] : sf.id === "brick" ? ["brick", "render"] : sf.id === "terrazzo" ? ["paving", "slate"] : sf.id === "stripes" ? ["stone", "cloth_navy"] : sf.id === "carpet" ? ["brick"] : sf.id === "subway" ? ["chalk"] : sf.id === "plaster" ? ["render"] : sf.id === "stone_tiles" ? ["stone"] : sf.id === "lino" ? ["paving"] : ["wood"];
    for (const [chr, code] of Object.entries(SCODES)) {
      const [ri, k] = SLEGEND[chr] as readonly [number, number];
      sTable[code] = rampStep((ramps[ri] ?? ramps[0]) as string, k);
    }
    const img = blank(48, 48);
    for (let y = 0; y < 48; y++) for (let x = 0; x < 48; x++) img.px[y * 48 + x] = sTable[t.px[(y % 16) * 16 + (x % 16)] as number] as number;
    return { img, label: `${sf.id} (${sf.on})`, outlined: false };
  });
  const all = [...surfaceCells, ...cells];
  // Row-packed layout.
  const maxW = 1800;
  let x = 8;
  let y = 30;
  let rowH = 0;
  const pos: { x: number; y: number }[] = [];
  for (const c of all) {
    const w = c.img.w * scale;
    const h = c.img.h * scale + 14;
    if (x + w > maxW) {
      x = 8;
      y += rowH + 12;
      rowH = 0;
    }
    pos.push({ x, y });
    x += Math.max(w, c.label.length * 4) + 16;
    rowH = Math.max(rowH, h);
  }
  const s = new Sheet(maxW + 16, y + rowH + 16, BG);
  s.text(`INTERIOR PARTS: ${SURFACES.length} SURFACES (3×3 TILES) AND ${PARTS.length} PARTS, RECOLOURED FROM ONE SET OF MATERIALS, 4× (FURNITURE OUTLINED; WALL PARTS AND DECALS NOT)`, 8, 8);
  all.forEach((c, i) => {
    const p = pos[i] as { x: number; y: number };
    s.text(c.label, p.x, p.y, LABEL, 1);
    s.rect(p.x, p.y + 10, c.img.w * scale, c.img.h * scale, [52, 56, 70]);
    s.paste(c.img, rgb, p.x, p.y + 10, scale);
  });
  png("interiors-parts.png", s);
}
