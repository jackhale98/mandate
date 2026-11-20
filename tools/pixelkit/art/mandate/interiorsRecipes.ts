// Interior recipes (DEC-088, DEC-098 §4): data that says what a place's room
// is like — its size, its furniture kit and how the kit may be arranged,
// where the people present stand or sit, and which actions sit at which
// hotspot. The composer (./interiors) builds a room from a recipe and a keyed
// seed; the seed only chooses among what the recipe allows (materials,
// arrangements, small decorations), so spots and hotspots are always valid.
// A recipe is versioned like the town layout (DEC-085): a change that moves
// anything for the same seed bumps `version`.
//
// The recipes are content: `content/base/interiors.jsonc` is their single
// source of truth, typed and validated in Rust (`mandate-content`, the
// `InteriorRecipe` type, with a generated schema, DEC-053) and read here as
// they are (`parseInteriorRecipes`): by the Node tools and tests from the
// file (./interiorsContent), by the game as the engine sends it in the place
// view (web/src/interior). The types below mirror the Rust ones field for field.
//
// Coordinates are floor tiles (16 px): (0, 0) is the floor tile under the
// back wall's left end, x to the right, y toward the viewer. The entrance is
// in the front wall at (door, rows - 1). Things hung on the back wall take a
// tile column and the height of their centre on the wall face (`wy`, px from
// its top), or stand on the floor line (`floor`).

/** Where a person faces. */
export type Facing = "down" | "up" | "left" | "right";

/** A ramp reference: a palette ramp name, optionally shifted ("grass+1"), or a room role ("@wood"). */
export type RampRef = string;
/** A choice among values, each with a weight (default 1). */
export type Weighted<T> = T | { value: T; weight: number };

/** How the engine's place view says someone is here (`PlacePanelView.here_as`). */
export type HereAs = "presiding" | "member" | "works" | "visitor";
export const HERE_AS: readonly HereAs[] = ["presiding", "member", "works", "visitor"];

/** Where a person present stands or sits. The engine's place view says who is there; the UI puts them on spots by role. */
export interface SpotDef {
  id: string;
  /** Who goes here: "chair", "clerk", "councillor", "official", "player", "staff", "owner", "cook", "waiter", "patron", "public". */
  role: string;
  x: number;
  y: number;
  facing: Facing;
  /** Seated people are drawn from the hips up on their seat (default "stand"). */
  pose?: "stand" | "sit";
}

/** A place action's hotspot: the thing (a rectangle of tiles) and the tile the player stands on to use it. */
export interface HotspotDef {
  id: string;
  /** Engine action keys (places.jsonc `actions`) offered here. */
  actions?: readonly string[];
  /** Engine services shown here (places.jsonc `services`). */
  services?: readonly string[];
  x: number;
  y: number;
  w: number;
  h: number;
  stand: { x: number; y: number };
  facing: Facing;
  /** Only offered late at night (places.jsonc `late_night`). */
  late?: boolean;
}

/** One placed part. Floor parts take tiles; wall parts take a column and a height on the wall. */
export interface PieceDef {
  /** A part id, or alternatives the seed picks between. */
  part: string | readonly Weighted<string>[];
  x: number;
  y?: number;
  /** Strips: length in tiles. */
  len?: number;
  mirror?: boolean;
  /** Pixel nudges. */
  dx?: number;
  dy?: number;
  /** Wall parts: the centre's height on the wall face (px from its top), or standing on the floor line. */
  wy?: number;
  floor?: boolean;
  /** Wall parts on the front wall's top, at this column (the bell over the entrance). */
  front?: boolean;
  /** Present with this chance (keyed by the seed and the piece's position). */
  chance?: number;
}

/** A patch of floor: another surface (the staff side of a counter), a rug, a dais, a mat. */
export interface PatchDef {
  kind: "surface" | "rug" | "dais";
  x: number;
  y: number;
  w: number;
  h: number;
  surface?: string;
  ramps?: readonly RampRef[];
}

/** One arrangement of a part of the room. */
export interface OptionDef {
  id: string;
  weight?: number;
  pieces: readonly PieceDef[];
  patches?: readonly PatchDef[];
  spots?: readonly SpotDef[];
  hotspots?: readonly HotspotDef[];
}

export interface ChoiceDef {
  id: string;
  options: readonly OptionDef[];
}

/** A surface and its ramps (second ramp for two-colour surfaces). */
export interface SurfaceRef {
  surface: string;
  ramps: readonly RampRef[];
}

export interface WallStyle {
  id: string;
  upper: SurfaceRef;
  /** A lower band (dado panelling, tiles) and its height in px. */
  lower?: SurfaceRef & { h: number };
  /** Cornice, dado rail, skirting and window frames. */
  trim: RampRef;
}

export interface InteriorRecipe {
  id: string;
  version: number;
  /** The place kind (places.jsonc `key`) this recipe furnishes. */
  place: string;
  cols: number;
  rows: number;
  /** Height of the back wall's face, px. */
  wall_h: number;
  /** The entrance's column in the front wall. */
  door: number;
  /** Light: lamp colour, how much of the night's grade reaches inside, and invisible ceiling lights (tile coordinates). */
  light: { lamp: "warm" | "white"; night: number; ceiling: readonly { x: number; y: number }[] };
  floors: readonly Weighted<SurfaceRef>[];
  walls: readonly Weighted<WallStyle>[];
  /** Variable roles' ramps (see interiorsParts.ts). */
  roles: { wood: readonly Weighted<RampRef>[]; uph: readonly Weighted<RampRef>[]; acc: readonly Weighted<RampRef>[]; acc2: readonly Weighted<RampRef>[] };
  pieces: readonly PieceDef[];
  patches: readonly PatchDef[];
  choices: readonly ChoiceDef[];
  spots: readonly SpotDef[];
  hotspots: readonly HotspotDef[];
  /** How someone is here (the place view's `here_as`) → the spot roles they take, in order of preference. */
  cast: Readonly<Partial<Record<HereAs, readonly string[]>>>;
  /** Staff roles drawn as scenery where nobody present takes them (no name, no conversation). */
  extras: readonly string[];
}

/**
 * Strip JSONC (DEC-053 content is JSONC: `//` and `/* *\/` comments,
 * trailing commas) to JSON, leaving string literals alone.
 */
export function jsoncToJson(text: string): string {
  let out = "";
  let i = 0;
  const n = text.length;
  while (i < n) {
    const c = text[i] as string;
    if (c === '"') {
      let j = i + 1;
      while (j < n && text[j] !== '"') j += text[j] === "\\" ? 2 : 1;
      out += text.slice(i, j + 1);
      i = j + 1;
    } else if (c === "/" && text[i + 1] === "/") {
      while (i < n && text[i] !== "\n") i++;
    } else if (c === "/" && text[i + 1] === "*") {
      const end = text.indexOf("*/", i + 2);
      i = end < 0 ? n : end + 2;
    } else {
      out += c;
      i++;
    }
  }
  let clean = "";
  for (let k = 0; k < out.length; k++) {
    const c = out[k] as string;
    if (c === '"') {
      let j = k + 1;
      while (j < out.length && out[j] !== '"') j += out[j] === "\\" ? 2 : 1;
      clean += out.slice(k, j + 1);
      k = j;
      continue;
    }
    if (c === ",") {
      let j = k + 1;
      while (j < out.length && /\s/.test(out[j] as string)) j++;
      if (out[j] === "}" || out[j] === "]") continue;
    }
    clean += c;
  }
  return clean;
}

/**
 * The recipes of an `interiors` content file (its JSONC text). Optional lists
 * the Rust type defaults (`patches`, `choices`, `spots`, `hotspots`, `extras`)
 * are filled in; the file is otherwise taken as it is (`content validate`
 * checks it against the schema and the tile rules).
 */
export function parseInteriorRecipes(text: string): InteriorRecipe[] {
  const file = JSON.parse(jsoncToJson(text)) as { kind?: string; items?: InteriorRecipe[] };
  if (file.kind !== "interiors" || !Array.isArray(file.items)) throw new Error('not an "interiors" content file');
  return file.items.map(normalizeRecipe);
}

/** A recipe with the lists the Rust type defaults filled in (as the engine's place view sends it, or a file has it). */
export function normalizeRecipe(r: InteriorRecipe): InteriorRecipe {
  return { ...r, patches: r.patches ?? [], choices: r.choices ?? [], spots: r.spots ?? [], hotspots: r.hotspots ?? [], cast: r.cast ?? {}, extras: r.extras ?? [] };
}

/** A place kind's recipe: by id (the place view's `interior`) or place key; the highest version unless one is asked for. */
export function recipeFor(recipes: readonly InteriorRecipe[], key: string, version?: number): InteriorRecipe | undefined {
  const all = recipes.filter((r) => (r.id === key || r.place === key) && (version === undefined || r.version === version));
  return all.reduce<InteriorRecipe | undefined>((best, r) => (!best || r.version > best.version ? r : best), undefined);
}
