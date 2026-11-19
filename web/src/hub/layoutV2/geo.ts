// What layout v2 plans a town from (TOWN_GENERATION.md §4.1): the shape of
// the engine's planned `TownGeographyView` (D3, task E2), quantised to
// integers so tiny engine changes in floating point never move a town.
//
// Stage A: the engine does not send this view yet. `../townGeoAdapter`
// derives it on the client from the views the game already has (the
// municipality view, the national map's rasters and links), and two fields
// carry what stage B moves elsewhere: `places` (the baseline place list;
// stage B reserves slots for every kind and the dress pass assigns places)
// and the current populations standing in for baselines (E1).
import type { Compass16 } from "./imath";

export type TownClass = "metro" | "city" | "town" | "rural";

export interface GeoHood {
  id: number;
  name: string;
  /** A `HoodKind` as the engine sends it. */
  kind: string;
  /** At world generation (stage A: the population when the town was first shown). */
  baseline_population: number;
  /** Neighbourhood ids it borders (stage A: unknown, empty). */
  adjacent: number[];
}

export interface GeoPlace {
  id: number;
  kind: string;
  name: string;
  neighborhood: number;
}

export interface GeoLink {
  mode: "road" | "highway" | "rail";
  /** Where the link leaves the town. */
  bearing: Compass16;
  /** The municipality at the other end (-1 unknown). */
  to: number;
  to_name: string;
  foreign: boolean;
}

export interface TownGeo {
  municipality: number;
  name: string;
  class: TownClass;
  regional_capital: boolean;
  national_capital: boolean;
  baseline_population: number;
  neighborhoods: GeoHood[];
  /** Stage A only: the town's places (see the header). */
  places: GeoPlace[];
  /** Baseline jobs per sector (stage A: the town's defining industry when known, else empty). */
  baseline_sectors: { sector: string; driver: string; jobs: number }[];
  river: { class: 1 | 2 | 3; enters: Compass16; leaves: Compass16; through_centre: boolean } | null;
  creeks: { enters: Compass16; joins_river: boolean }[];
  coast: { sea: Compass16; shore: "beach" | "rocky" | "cliff"; bay: boolean } | null;
  lake: { bearing: Compass16; near: boolean } | null;
  relief: { ruggedness_pm: number; uphill: Compass16; high_ground: Compass16; relief_m: number };
  woods: { share_pm: number; bearing: Compass16 };
  minerals: { value_pm: number; bearing: Compass16 };
  /** Resource indexes, per mille (fertility, timber, minerals, fishing, energy, scenic). */
  resources_pm: Record<string, number>;
  links: GeoLink[];
  hazards_pm: { flood: number; storm: number; wildfire: number; seismic: number; drought: number };
  climate: { temperature_c: number; precipitation_mm: number; biome: string; snow: boolean };
  wind_from: Compass16;
  elevation_m: number;
  era: string | null;
}

/** Population bands (§9.1). */
export type Band = "village" | "small_town" | "town" | "city" | "metro";

export function bandOf(population: number): Band {
  if (population < 30_000) return "village";
  if (population < 100_000) return "small_town";
  if (population < 300_000) return "town";
  if (population < 1_000_000) return "city";
  return "metro";
}

/** Readability caps per band (§8, §9.1). */
export const BAND_CAPS: Record<Band, { arterials: [number, number]; bridges: number; landmarks: [number, number]; ring: boolean }> = {
  village: { arterials: [2, 4], bridges: 1, landmarks: [3, 5], ring: false },
  small_town: { arterials: [3, 5], bridges: 2, landmarks: [5, 8], ring: false },
  town: { arterials: [4, 6], bridges: 3, landmarks: [6, 10], ring: false },
  city: { arterials: [5, 7], bridges: 4, landmarks: [8, 12], ring: true },
  metro: { arterials: [6, 8], bridges: 5, landmarks: [10, 12], ring: true },
};
