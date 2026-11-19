// What the town layout needs from the game: the data the engine already
// exposes for a town, nothing more (DEC-079: places come from the simulation;
// DEC-009: walking is UI state). Adapters read the existing protocol views;
// the protocol and the engine are unchanged.
import type { LocationDto } from "../engine/protocol/LocationDto";
import type { MunicipalityView } from "../engine/protocol/MunicipalityView";
import type { TownPlacesView } from "../engine/protocol/TownPlacesView";

/** Neighbourhood kinds the engine generates (`hood-<kind>` in Fluent). Unknown kinds lay out like `inner`. */
export type HoodKind = "core" | "inner" | "waterfront" | "industrial" | "suburb" | "rural";
export const HOOD_KINDS: readonly HoodKind[] = ["core", "inner", "waterfront", "industrial", "suburb", "rural"];

export interface TownHood {
  id: number;
  name: string;
  /** A `HoodKind`, as the engine sends it. */
  kind: string;
}

export interface TownPlace {
  id: number;
  /** Place-registry key (`content/base/places.jsonc`), e.g. `town_hall`. */
  kind: string;
  /** A generated proper name, or empty. */
  name: string;
  /** The neighbourhood id it stands in. */
  neighborhood: number;
}

export type TownLocation = LocationDto;

export interface TownInput {
  municipality: string;
  neighborhoods: TownHood[];
  places: TownPlace[];
  /** The player's home neighbourhood (their house stands there), if any. */
  home: number | null;
  /** Where the player is now. */
  location: TownLocation;
}

/** The player's town, from the `town` query (`TownPlacesView`). */
export function townInputFromView(view: TownPlacesView): TownInput {
  const places: TownPlace[] = [];
  for (const h of view.neighborhoods) {
    for (const s of h.spots) {
      const l = s.location.location;
      if (l.type !== "place") continue;
      places.push({ id: l.place, kind: s.location.kind_key, name: s.location.name, neighborhood: h.id });
    }
  }
  return {
    municipality: view.municipality,
    neighborhoods: view.neighborhoods.map((h) => ({ id: h.id, name: h.name, kind: h.kind })),
    places,
    home: view.home_neighborhood,
    location: view.location,
  };
}

/** Any town on the map, from the `municipality` query (a visit: no home; the player starts on the central streets). */
export function townInputFromMunicipality(view: MunicipalityView): TownInput {
  const centre = view.neighborhoods.find((h) => h.kind === "core") ?? view.neighborhoods[0];
  return {
    municipality: view.name,
    neighborhoods: view.neighborhoods.map((h) => ({ id: h.id, name: h.name, kind: h.kind })),
    places: view.places.map((p) => ({ id: p.id, kind: p.kind, name: p.name, neighborhood: p.neighborhood })),
    home: null,
    location: centre ? { type: "street", neighborhood: centre.id } : { type: "home" },
  };
}
