// Mandate's art pack for the P0 spike. `buildPack()` runs ahead of time (the
// CLI, or the spike's Vite build) and gives plain data: the frozen palette and
// its lightness, the tiles (autotile templates, fills, decals), the props, the
// lamp's light pool and the day's lighting. Characters and buildings are
// composed on the device (D10) from the parts and the kit, by the functions
// re-exported at the bottom; they read only the pack's palette.
import { hexToRgb, lightness } from "../../src/core/color";
import { paletteLightness } from "../../src/core/compose";
import type { Palette } from "../../src/core/palette";
import { buildLighting, type Lighting } from "./lighting";
import { buildMandatePalette } from "./palette";
import { buildProps, lampPool, windowSpill, type BuiltProp, type Pool, type Spill } from "./props";
import { buildTerrain, type TerrainSet } from "./terrain";
import { buildTiles, type TileSet } from "./tiles";

/** Bump when any drawing or rule changes what an existing look or building looks like (§3.5 rule 8). */
export const PACK_VERSION = "mandate-p0.7";

export interface ArtPack {
  version: string;
  palette: Palette;
  /** Oklab lightness ×1000 per colour, frozen (the runtime does no colour maths). */
  lightness: number[];
  tiles: TileSet;
  /** Layout v2 terrain (round 4; diagonal streets, round 5; street ends, round 6): shore templates, v2 grounds, the legend and step tables the painter draws with. */
  terrain: TerrainSet;
  props: Record<string, BuiltProp[]>;
  pool: Pool;
  /** Light from lit shop windows onto the pavement (round 2). */
  spill: Spill;
  lighting: Lighting;
}

export interface PaletteData {
  palette: Palette;
  lightness: number[];
}

let cached: PaletteData | null = null;
export function paletteData(): PaletteData {
  if (cached) return cached;
  const palette = buildMandatePalette();
  cached = { palette, lightness: paletteLightness(palette, (h) => lightness(hexToRgb(h))) };
  return cached;
}

export function buildPack(): ArtPack {
  const { palette, lightness: L } = paletteData();
  return {
    version: PACK_VERSION,
    palette,
    lightness: L,
    tiles: buildTiles(palette),
    terrain: buildTerrain(palette),
    props: buildProps(palette, L),
    pool: lampPool(palette),
    spill: windowSpill(palette),
    lighting: buildLighting(palette),
  };
}

export { composeCharacter, faceLeft, lookFromSeed, PLAYER_LOOK, FRAMES, FACINGS, ANCHOR, type Look } from "./characters";
export { buildFacade, type FacadeSpec, type Facade } from "./buildings";
export { groundTile, TERRAINS } from "./tiles";
export { TerrainPainter, tileOf, TERRAIN, V1_NAMES, V2_CODES, V2_NAMES, CONNECTOR_KINDS, type TerrainView, type ConnectorView } from "./terrain";
export { END_KINDS, END_FACINGS, endPropVariant, type StreetEndView } from "./ends";
export type { Pool } from "./props";
