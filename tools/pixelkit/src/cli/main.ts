// pixelkit CLI. Run from the repository root (Node ≥ 22.18):
//   node tools/pixelkit/bin/pixelkit.mjs <command> [--out target/pixelkit]
// Commands:
//   palette   the palette as JSON and a swatch sheet
//   sheets    the contact sheets: palette, parts, townsfolk, buildings, tiles, props
//   lint      the P0 lints; writes lint.json and lint.txt; exits 1 on an error
//   build     the built pack as the game would load it: pack.json and a tile atlas
//   street    lays the spike's street screenshots (web/spikes/hub/pixelkit-capture.mjs) side by side
//   all       palette, sheets, build and lint
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { hexToRgb, type Rgb8 } from "../core/color";
import { BLOB_MASKS } from "../core/autotile";
import { blank, blit, remap, type Indexed } from "../core/image";
import { packToJson } from "../core/serial";
import { decodePng, encodePng, type Rgba } from "../node/png";
import { Sheet } from "../node/sheet";
import { lintPack, lintReport } from "./lint";
import { paletteSheet } from "./palette";
import { gridSheet, type Cell } from "./sheets";
import { buildFacade, kitPartList, kitTable, type FacadeSpec } from "../../art/mandate/buildings";
import { composeCharacter, faceLeft, lookFromSeed, partList, PLAYER_LOOK, slotTable } from "../../art/mandate/characters";
import { buildPack, type ArtPack } from "../../art/mandate/pack";
import { groundTile, TERRAINS } from "../../art/mandate/tiles";
import { sampleBuildings } from "../../art/mandate/samples";

const args = process.argv.slice(2);
const cmd = args[0] ?? "all";
const outIdx = args.indexOf("--out");
const out = path.resolve(outIdx >= 0 ? (args[outIdx + 1] as string) : "target/pixelkit");
mkdirSync(out, { recursive: true });

const png = (name: string, img: Rgba) => {
  writeFileSync(path.join(out, name), encodePng(img));
  console.log(`wrote ${path.join(out, name)} (${img.w}×${img.h})`);
};

const stepColour = (pack: ArtPack, ramp: string, k: number): Rgb8 => {
  const r = pack.palette.ramps.find((x) => x.name === ramp);
  if (!r) throw new Error(ramp);
  return hexToRgb(pack.palette.colors[r.steps[Math.max(0, Math.min(r.steps.length - 1, r.anchorStep + k))] as number] as string);
};

/** A building at night for the kit sheet: the albedo under the night multiply, its lit openings over it. */
function litPreview(pack: ArtPack, spec: FacadeSpec): { img: Indexed; rgb: Rgb8[] } {
  const f = buildFacade(spec, pack.palette);
  const n = pack.palette.colors.length;
  const night = pack.lighting.entries[90];
  const rgb: Rgb8[] = pack.palette.colors.map(hexToRgb);
  const m = night?.mul ?? [90, 100, 170];
  // Night colours go in the upper half of the table (index + 128).
  for (let i = 0; i < n; i++) {
    const c = rgb[i] as Rgb8;
    rgb[i + 128] = [(c[0] * m[0]) / 255, (c[1] * m[1]) / 255, (c[2] * m[2]) / 255];
  }
  const dark = remap(f.albedo, Array.from({ length: 256 }, (_, i) => (i === 0 ? 0 : i + 128)));
  for (const o of [...f.windows, ...f.doors]) if (o.emissive) blit(dark, o.emissive, o.x, o.y);
  return { img: dark, rgb };
}

function sheets(pack: ArtPack) {
  const pal = pack.palette;
  const rgb = pal.colors.map(hexToRgb);
  const pavement = stepColour(pack, "paving", 0);

  // ---- every part
  const charTable = slotTable(lookFromSeed(7), pal);
  const charCells: Cell[] = partList().map((p) => ({ img: remap(p.img, charTable), label: p.name }));
  png("parts-characters.png", gridSheet("CHARACTER PARTS (RECOLOURED FROM ONE LOOK; SIDE VIEWS FACE RIGHT)", charCells, rgb, { cols: 10, scale: 4, checker: true }));
  const kitSpec: FacadeSpec = { kind: "shop", hood: "inner", w: 3, h: 3, doorX: 1, storeys: 2, visualSeed: 202, named: true, home: false, landmark: false, conditions: [] };
  const kt = kitTable(kitSpec, pal);
  const board = kitPartList().find((p) => p.name === "sign board")?.img;
  const kitCells: Cell[] = kitPartList().map((p) => {
    // Pictograms are shown on their sign board, as they appear on a building.
    let img = p.img;
    if (p.name.startsWith("picto") && board) {
      img = blank(board.w, board.h);
      blit(img, board, 0, 0);
      blit(img, p.img, 2, 2);
    }
    return { img: remap(img, kt), label: p.name.replace("business_association", "business").replace("_", " ") };
  });
  png("parts-kit.png", gridSheet("BUILDING KIT PARTS (RECOLOURED FROM THE MATERIALS OF ONE BUILDING)", kitCells, rgb, { cols: 10, scale: 4, checker: true }));

  // ---- townsfolk: 20 and the player, idle in four directions, then the walk down, up and right
  const folk: Cell[] = [];
  for (let i = 0; i <= 20; i++) {
    const look = i === 0 ? PLAYER_LOOK : lookFromSeed(i);
    const c = composeCharacter(look, pal, pack.lightness);
    const [down, up, side] = c.frames as [Indexed[], Indexed[], Indexed[]];
    folk.push({ img: down[0] as Indexed, label: i === 0 ? "player" : `seed ${i}` }, { img: faceLeft(side[0] as Indexed) }, { img: up[0] as Indexed }, { img: side[0] as Indexed });
    for (const row of [down, up, side]) for (let k = 2; k < 6; k++) folk.push({ img: row[k] as Indexed });
  }
  png("townsfolk.png", gridSheet("TOWNSFOLK: IDLE DOWN LEFT UP RIGHT / WALK DOWN / WALK UP / WALK RIGHT (4 FRAMES)", folk, rgb, { cols: 16, scale: 4, ground: pavement }));

  // The same crowd at 1× and 3× on the street's grounds (readability at real size).
  const crowd = Array.from({ length: 20 }, (_, i) => composeCharacter(lookFromSeed(i + 1), pal, pack.lightness).frames[0]?.[0] as Indexed);
  const grounds: [string, Rgb8][] = [
    ["pavement", pavement],
    ["road", stepColour(pack, "asphalt", 0)],
    ["plaza", stepColour(pack, "stone", 0)],
    ["grass", stepColour(pack, "grass", 0)],
  ];
  const rs = new Sheet(20 * 18 * 3 + 20 * 18 + 48, 30 + grounds.length * 82);
  rs.text("THE 20 TOWNSFOLK AT 1× AND 3× ON THE GROUNDS OF THE STREET", 8, 8);
  grounds.forEach(([name, c], gi) => {
    const y = 30 + gi * 82;
    rs.rect(8, y, 20 * 18 + 8, 30, c);
    rs.rect(20 * 18 + 24, y, 20 * 18 * 3 + 8, 78, c);
    crowd.forEach((img, i) => {
      rs.paste(img, rgb, 12 + i * 18, y + 3, 1);
      rs.paste(img, rgb, 20 * 18 + 28 + i * 18 * 3, y + 3, 3);
    });
    rs.text(name, 8, y + 34, [154, 160, 180], 1);
  });
  png("townsfolk-scale.png", rs);

  // ---- the building kit assembled several ways, by day and lit at night
  const specs = sampleBuildings();
  const label = (s: FacadeSpec) => `${s.kind.replace("_", " ")} ${s.hood} ${s.w}x${s.h}${s.conditions.length ? " boarded" : ""}`;
  png("buildings.png", gridSheet("BUILDING KIT: THE TOWN-CENTRE FAMILY ASSEMBLED BY THE FACADE GRAMMAR", specs.map((s) => ({ img: buildFacade(s, pal).albedo, label: label(s) })), rgb, { cols: 6, scale: 3, ground: pavement }));
  const lit = specs.map((s) => litPreview(pack, s));
  png("buildings-night.png", gridSheet("THE SAME BUILDINGS AT NIGHT (MULTIPLY LOOK): LIT WINDOWS AND DOORS FROM THEIR EMISSIVE LAYERS", lit.map((l, i) => ({ img: l.img, label: label(specs[i] as FacadeSpec) })), lit[0]?.rgb ?? rgb, { cols: 6, scale: 3, ground: [40, 44, 70] }));

  // ---- tiles: per terrain, the 47 blob tiles and the fill variants
  const tcells: Cell[] = [];
  for (const t of [...TERRAINS, "crossing"]) {
    const terr = pack.tiles.terrains[t === "crossing" ? "road" : t];
    if (!terr) continue;
    tcells.push({ img: blank(16, 16), label: t });
    for (const m of BLOB_MASKS) tcells.push({ img: groundTile(pack.tiles, t, m, t === "road" ? 0 : 0) });
    if (t !== "crossing") terr.fills.forEach((f) => tcells.push({ img: f }));
    for (let k = t === "crossing" ? 0 : terr.fills.length; k < 3; k++) tcells.push({ img: blank(16, 16) });
  }
  png("tiles.png", gridSheet("GROUND TILES: THE 47 AUTOTILES OF EACH TERRAIN, THEN ITS FILL VARIANTS", tcells, rgb, { cols: 51, scale: 3, pad: 3 }));
  const templates: Cell[] = TERRAINS.map((t) => ({ img: pack.tiles.terrains[t]?.template ?? blank(32, 48), label: t }));
  png("tile-templates.png", gridSheet("AUTOTILE TEMPLATES (32×48, BUILT FROM A HAND-DRAWN FILL AND FOUR EDGE PROFILES)", templates, rgb, { cols: 7, scale: 4 }));

  // ---- props
  const pcells: Cell[] = [];
  for (const [kind, list] of Object.entries(pack.props)) {
    list.forEach((p, i) => {
      pcells.push({ img: p.albedo, label: `${kind} ${i}` });
      if (p.emissive) pcells.push({ img: p.emissive, label: `${kind} lit` });
    });
  }
  png("props.png", gridSheet("PROPS (WITH THE AUTOMATIC OUTLINE) AND THEIR LIT PARTS", pcells, rgb, { cols: 8, scale: 4, ground: pavement }));
}

/** The street screenshots side by side (run web/spikes/hub/pixelkit-capture.mjs first). */
function street() {
  const dir = path.join(out, "street");
  const load = (n: string) => {
    const f = path.join(dir, `${n}.png`);
    return existsSync(f) ? decodePng(readFileSync(f)) : null;
  };
  const board = (name: string, title: string, rows: string[][], down: number) => {
    const imgs = rows.map((cols) => cols.map(load));
    const first = imgs.flat().find((x) => x);
    if (!first) {
      console.log(`no screenshots in ${dir} for ${name}`);
      return;
    }
    const cw = Math.floor(first.w / down);
    const ch = Math.floor(first.h / down);
    const cols = Math.max(...rows.map((c) => c.length));
    const s = new Sheet(16 + cols * (cw + 8), 40 + rows.length * (ch + 26));
    s.text(title, 8, 10);
    rows.forEach((names, r) => {
      names.forEach((n, c) => {
        const img = imgs[r]?.[c];
        const x = 8 + c * (cw + 8);
        const y = 36 + r * (ch + 26);
        if (img) s.pasteRgba(img, x, y + 14, down);
        s.text(n.split("-").slice(1).join(" "), x, y, [233, 234, 240], 2);
      });
    });
    png(name, s);
  };
  for (const device of ["desktop", "phone"] as const) {
    const d = device === "phone" ? 2 : 1;
    board(
      `night-looks-${device}.png`,
      `D12, THE TWO NIGHT LOOKS (${device.toUpperCase()}): MULTIPLY ONLY (LEFT), PALETTE MOODS (RIGHT)`,
      ["golden", "dusk", "night"].map((t) => [`${device}-multiply-${t}`, `${device}-moods-${t}`]),
      d,
    );
    board(
      `street-${device}.png`,
      `THE SAMPLE STREET (${device.toUpperCase()}): CODE PLACEHOLDER (LEFT), PIXELKIT PACK (RIGHT)`,
      ["noon", "dusk"].map((t) => [`${device}-placeholder-${t}`, `${device}-multiply-${t}`]),
      d,
    );
  }
}

function build(pack: ArtPack) {
  const dir = path.join(out, "pack");
  mkdirSync(dir, { recursive: true });
  const json = packToJson(pack);
  writeFileSync(path.join(dir, "pack.json"), json);
  console.log(`wrote ${path.join(dir, "pack.json")} (${json.length} bytes)`);
  // The tile atlas a game would load (P1 ships atlases like this instead of pixel data in JSON).
  const rgb = pack.palette.colors.map(hexToRgb);
  const tiles: Indexed[] = [];
  for (const t of TERRAINS) for (const m of BLOB_MASKS) tiles.push(groundTile(pack.tiles, t, m, 0));
  const cols = 47;
  const atlas = new Sheet(cols * 16, Math.ceil(tiles.length / cols) * 16, [0, 0, 0]);
  tiles.forEach((t, i) => atlas.paste(t, rgb, (i % cols) * 16, Math.floor(i / cols) * 16));
  png("pack/tiles.png", atlas);
}

async function main() {
  const t0 = performance.now();
  const pack = buildPack();
  console.log(`pack ${pack.version}: ${pack.palette.colors.length - 1} colours, built in ${Math.round(performance.now() - t0)} ms`);
  if (cmd === "palette" || cmd === "all") {
    writeFileSync(path.join(out, "palette.json"), `${JSON.stringify({ colors: pack.palette.colors.slice(1), ramps: pack.palette.ramps.map((r) => ({ name: r.name, steps: r.steps.map((i) => pack.palette.colors[i]), line: pack.palette.colors[r.line] })) }, null, 1)}\n`);
    png("palette.png", paletteSheet(pack.palette));
  }
  if (cmd === "sheets" || cmd === "all") sheets(pack);
  if (cmd === "build" || cmd === "all") build(pack);
  if (cmd === "street") street();
  if (cmd === "lint" || cmd === "all") {
    const f = lintPack(pack);
    const report = lintReport(f);
    writeFileSync(path.join(out, "lint.txt"), report);
    writeFileSync(path.join(out, "lint.json"), `${JSON.stringify(f, null, 1)}\n`);
    console.log(report);
    if (f.some((x) => x.level === "error")) process.exitCode = 1;
  }
}

main().catch((e: unknown) => {
  console.error(e);
  process.exit(1);
});
