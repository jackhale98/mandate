// Map palette (presentation only). Index = Biome discriminant from the engine.
export const BIOME_RGB: ReadonlyArray<readonly [number, number, number]> = [
  [38, 70, 112], // ocean
  [64, 110, 160], // lake
  [222, 208, 160], // beach
  [92, 128, 104], // wetland
  [150, 180, 96], // grassland
  [74, 128, 70], // temperate forest
  [44, 104, 64], // rainforest
  [70, 104, 84], // boreal
  [168, 160, 98], // shrubland
  [190, 178, 120], // steppe
  [214, 192, 138], // desert
  [170, 170, 150], // tundra
  [168, 164, 160], // alpine
  [130, 124, 120], // rock
];

export const BIOME_KEYS = [
  "ocean", "lake", "beach", "wetland", "grassland", "temperate_forest", "rainforest",
  "boreal", "shrubland", "steppe", "desert", "tundra", "alpine", "rock",
] as const;

export function decodeB64(s: string): Uint8Array {
  const bin = atob(s);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}
