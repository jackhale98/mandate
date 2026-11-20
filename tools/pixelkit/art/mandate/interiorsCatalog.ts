// What the content check needs to know about the interior kit (DEC-098 §4):
// each part's kind and footprint (to build the walk grid), a wall part's
// height (to keep it on the wall face), which surfaces are floors and which
// walls, and the palette's ramp names. The parts themselves stay in the art
// pack; `mandate-content` validates recipes against this catalogue, a
// generated file kept equal to the pack by the tests:
//   node --import ./bin/hooks.mjs art/mandate/interiorsCatalog.ts --write
import { writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { PARTS, partSlots, SURFACES } from "./interiorsParts";
import { paletteData } from "./pack";

export const CATALOG_FILE = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../../crates/mandate-content/src/interior_parts.json");

export interface CatalogPart {
  kind: string;
  /** Footprint in tiles across a strip, or of a single drawing ([w, h]). */
  foot: [number, number];
  strip: "h" | "v" | null;
  /** Height in px at length 1. */
  h: number;
}

export function interiorCatalog() {
  const parts: Record<string, CatalogPart> = {};
  for (const p of [...PARTS].sort((a, b) => a.id.localeCompare(b.id))) {
    parts[p.id] = { kind: p.kind, foot: [p.foot?.[0] ?? 1, p.foot?.[1] ?? 1], strip: p.strip?.dir ?? null, h: partSlots(p.id, 1).h };
  }
  const surfaces: Record<string, string> = {};
  for (const s of [...SURFACES].sort((a, b) => a.id.localeCompare(b.id))) surfaces[s.id] = s.on;
  const ramps = paletteData()
    .palette.ramps.map((r) => r.name)
    .sort();
  return { generated: "tools/pixelkit art/mandate/interiorsCatalog.ts --write; do not edit", parts, surfaces, ramps };
}

export const catalogText = () => `${JSON.stringify(interiorCatalog(), null, 1)}\n`;

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url) && process.argv.includes("--write")) {
  writeFileSync(CATALOG_FILE, catalogText());
  console.log(`wrote ${CATALOG_FILE}`);
}
