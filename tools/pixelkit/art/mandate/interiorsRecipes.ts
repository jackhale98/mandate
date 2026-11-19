// Interior recipes (DEC-088): data that says what a place's room is like —
// its size, its furniture kit and how the kit may be arranged, where the
// people present stand or sit, and which actions sit at which hotspot. The
// composer (./interiors) builds a room from a recipe and a keyed seed; the
// seed only chooses among what the recipe allows (materials, arrangements,
// small decorations), so spots and hotspots are always valid. A recipe is
// versioned like the town layout (DEC-085): a change that moves anything for
// the same seed bumps `version`, and the previous version stays callable.
//
// Coordinates are floor tiles (16 px): (0, 0) is the floor tile under the
// back wall's left end, x to the right, y toward the viewer. The entrance is
// in the front wall at (door, rows - 1). Things hung on the back wall take a
// tile column and the height of their centre on the wall face (`wy`, px from
// its top), or stand on the floor line (`floor`).
//
// Everything here is plain JSON-shaped data (no functions), so it can move
// to content/ as JSONC with a schema (see the README's interiors section).

/** Where a person faces. */
export type Facing = "down" | "up" | "left" | "right";

/** A ramp reference: a palette ramp name, optionally shifted ("grass+1"), or a room role ("@wood"). */
export type RampRef = string;
/** A choice among values, each with a weight (default 1). */
export type Weighted<T> = T | { value: T; weight: number };

/** Where a person present stands or sits. The engine's place view says who is there; the UI puts them on spots by role. */
export interface SpotDef {
  id: string;
  /** Who goes here: "chair", "clerk", "councillor", "player", "staff", "owner", "cook", "patron", "public". */
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
  /** Engine action keys (content/base/actions, places.jsonc) offered here. */
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
  wallH: number;
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
}

// ---------------------------------------------------------------------------
// The council chamber: formal. A dais with the chair's seat under the crest,
// the clerk's table before it, members' desks as a horseshoe or in rows, a
// lectern, a balustrade and two rows of pews for the public gallery.

const chamberCouncillors = (xs: readonly number[], y: number, facing: Facing, pose: "stand" | "sit", prefix: string): SpotDef[] => xs.map((x, i) => ({ id: `${prefix}${i + 1}`, role: "councillor", x, y, facing, pose }));

export const COUNCIL_CHAMBER: InteriorRecipe = {
  id: "interior.council_chamber",
  version: 1,
  place: "council_chamber",
  cols: 13,
  rows: 13,
  wallH: 48,
  door: 6,
  light: { lamp: "white", night: 0.72, ceiling: [{ x: 6, y: 5 }, { x: 3, y: 10 }, { x: 9, y: 10 }] },
  floors: [
    { surface: "parquet", ramps: ["hair_brown"] },
    { surface: "parquet", ramps: ["hair_blonde"] },
    { surface: "stone_tiles", ramps: ["stone"] },
  ],
  walls: [
    { id: "plaster_over_panels", upper: { surface: "plaster", ramps: ["render"] }, lower: { surface: "panelling", ramps: ["@wood"], h: 22 }, trim: "stone" },
    { id: "stripes_over_panels", upper: { surface: "stripes", ramps: ["stone", "@acc"] }, lower: { surface: "panelling", ramps: ["@wood"], h: 20 }, trim: "stone" },
    { id: "panelled", upper: { surface: "panelling", ramps: ["@wood"] }, trim: "render" },
  ],
  roles: {
    wood: ["hair_dark_brown", "hair_auburn", "hair_brown"],
    uph: ["cloth_green", "brick", "cloth_navy"],
    acc: ["brick", "cloth_navy", "cloth_plum", "cloth_teal"],
    acc2: ["cloth_mustard", "chalk", "cloth_sky"],
  },
  pieces: [
    // The back wall: four tall windows, the crest over the chair, lights either side.
    { part: "window_arch", x: 1, wy: 22 },
    { part: "window_arch", x: 3, wy: 22 },
    { part: "window_arch", x: 9, wy: 22 },
    { part: "window_arch", x: 11, wy: 22 },
    { part: "crest", x: 6, wy: 18 },
    { part: "sconce", x: 4, wy: 22 },
    { part: "sconce", x: 8, wy: 22 },
    { part: ["portrait", "sconce", "poster"], x: 2, wy: 22, chance: 0.8 },
    { part: ["portrait", "sconce"], x: 10, wy: 22, chance: 0.8 },
    // The dais: the chair's seat between flags, the chair's bench, the clerk's table.
    { part: "flag_stand", x: 4, y: 1 },
    { part: "flag_stand", x: 8, y: 1, mirror: true },
    { part: "chair_high", x: 6, y: 0 },
    { part: "desk", x: 5, y: 1, len: 3 },
    { part: "banker_lamp", x: 5, y: 1, dx: 3 },
    { part: "nameplate", x: 6, y: 1, dx: 5 },
    { part: "mic", x: 6, y: 1, dx: 10 },
    { part: "papers", x: 7, y: 1, dx: 4, chance: 0.7 },
    { part: "chair_front", x: 6, y: 2 },
    { part: "clerk_table", x: 5, y: 3, len: 3 },
    { part: "papers", x: 5, y: 3, dx: 4 },
    { part: "water_jug", x: 7, y: 3, dx: 6 },
    { part: "papers", x: 6, y: 3, dx: 8, chance: 0.5 },
    // The public gallery: a balustrade with a gate, two rows of pews.
    { part: "rail", x: 0, y: 9, len: 6 },
    { part: "rail", x: 7, y: 9, len: 6 },
    { part: "pew", x: 1, y: 10, len: 4 },
    { part: "pew", x: 8, y: 10, len: 4 },
    { part: "pew", x: 1, y: 11, len: 4 },
    { part: "pew", x: 8, y: 11, len: 4 },
    { part: "doormat", x: 6, y: 12 },
    // Corners.
    { part: ["plant_tall", "plant_small"], x: 0, y: 0, chance: 0.8 },
    { part: ["plant_tall", "plant_small"], x: 12, y: 0, chance: 0.8 },
    { part: ["coat_stand", "plant_small", "bin"], x: 0, y: 12, chance: 0.7 },
    { part: ["plant_small", "bin"], x: 12, y: 12, chance: 0.6 },
  ],
  patches: [{ kind: "dais", x: 4, y: 0, w: 5, h: 2, surface: "carpet", ramps: ["@uph"] }],
  choices: [
    {
      id: "benches",
      options: [
        {
          id: "horseshoe",
          pieces: [
            { part: "desk_side", x: 2, y: 3, len: 4 },
            { part: "desk_side", x: 10, y: 3, len: 4, mirror: true },
            { part: "desk", x: 2, y: 7, len: 4 },
            { part: "desk", x: 7, y: 7, len: 4 },
            { part: "lectern", x: 6, y: 5 },
            ...[3, 5].map((y) => ({ part: "mic", x: 2, y, dx: 6, dy: -6 })),
            ...[3, 5].map((y) => ({ part: "mic", x: 10, y, dx: 6, dy: -6 })),
            ...[2, 4, 7, 9].map((x) => ({ part: "mic", x, y: 7, dx: 6 })),
            { part: "papers", x: 3, y: 7, dx: 5, chance: 0.6 },
            { part: "papers", x: 8, y: 7, dx: 3, chance: 0.6 },
            ...[2, 3, 4, 5, 7, 8, 9, 10].map((x) => ({ part: "chair_back", x, y: 8 })),
          ],
          patches: [{ kind: "surface", x: 3, y: 3, w: 7, h: 4, surface: "carpet", ramps: ["@uph"] }],
          spots: [
            ...chamberCouncillors([3, 4, 5, 7, 8, 9], 8, "up", "sit", "bottom"),
            ...chamberCouncillors([3, 4, 5, 6], 1, "right", "stand", "left").map((s) => ({ ...s, x: 1, y: s.x })),
            ...chamberCouncillors([3, 4, 5, 6], 11, "left", "stand", "right").map((s) => ({ ...s, x: 11, y: s.x })),
            { id: "player", role: "player", x: 2, y: 8, facing: "up", pose: "sit" },
          ],
          hotspots: [
            { id: "speak", actions: ["introduce", "amend"], x: 6, y: 5, w: 1, h: 1, stand: { x: 6, y: 6 }, facing: "up" },
            { id: "vote", actions: ["vote"], x: 2, y: 7, w: 1, h: 1, stand: { x: 2, y: 8 }, facing: "up" },
          ],
        },
        {
          id: "rows",
          pieces: [
            { part: "desk", x: 1, y: 4, len: 4 },
            { part: "desk", x: 8, y: 4, len: 4 },
            { part: "desk", x: 1, y: 6, len: 4 },
            { part: "desk", x: 8, y: 6, len: 4 },
            { part: "lectern", x: 6, y: 6 },
            ...[1, 3, 8, 10].flatMap((x) => [4, 6].map((y) => ({ part: "mic", x, y, dx: 9 }))),
            { part: "papers", x: 2, y: 4, dx: 4, chance: 0.6 },
            { part: "papers", x: 9, y: 6, dx: 4, chance: 0.6 },
            ...[1, 2, 3, 4, 8, 9, 10, 11].flatMap((x) => [5, 7].map((y) => ({ part: "chair_back", x, y }))),
          ],
          patches: [{ kind: "surface", x: 5, y: 2, w: 3, h: 7, surface: "carpet", ramps: ["@uph"] }],
          spots: [
            ...chamberCouncillors([2, 3, 4, 8, 9, 10], 5, "up", "sit", "front"),
            ...chamberCouncillors([2, 3, 4, 8, 9, 10, 11], 7, "up", "sit", "back"),
            { id: "player", role: "player", x: 1, y: 7, facing: "up", pose: "sit" },
            { id: "front_end", role: "councillor", x: 1, y: 5, facing: "up", pose: "sit" },
            { id: "front_end_r", role: "councillor", x: 11, y: 5, facing: "up", pose: "sit" },
          ],
          hotspots: [
            { id: "speak", actions: ["introduce", "amend"], x: 6, y: 6, w: 1, h: 1, stand: { x: 6, y: 7 }, facing: "up" },
            { id: "vote", actions: ["vote"], x: 1, y: 6, w: 1, h: 1, stand: { x: 1, y: 7 }, facing: "up" },
          ],
        },
      ],
    },
  ],
  spots: [
    { id: "chair", role: "chair", x: 6, y: 0, facing: "down", pose: "sit" },
    { id: "clerk", role: "clerk", x: 6, y: 2, facing: "down", pose: "sit" },
    { id: "gallery1", role: "public", x: 2, y: 10, facing: "up", pose: "sit" },
    { id: "gallery2", role: "public", x: 10, y: 10, facing: "up", pose: "sit" },
    { id: "gallery3", role: "public", x: 3, y: 11, facing: "up", pose: "sit" },
    { id: "gallery4", role: "public", x: 9, y: 11, facing: "up", pose: "sit" },
    { id: "gallery5", role: "public", x: 4, y: 10, facing: "up", pose: "sit" },
    { id: "usher", role: "staff", x: 7, y: 10, facing: "up" },
  ],
  hotspots: [
    { id: "clerk", actions: ["withdraw"], x: 5, y: 3, w: 3, h: 1, stand: { x: 6, y: 4 }, facing: "up" },
  ],
};

// ---------------------------------------------------------------------------
// The town hall's front office: civic, busy. A counter with two windows, the
// staff side behind it (a desk, files, a clock between the windows), and the
// public side: notice board, leaflets, the door to the offices, somewhere to
// wait.

export const TOWN_HALL: InteriorRecipe = {
  id: "interior.town_hall",
  version: 1,
  place: "town_hall",
  cols: 12,
  rows: 9,
  wallH: 40,
  door: 3,
  light: { lamp: "white", night: 0.7, ceiling: [{ x: 2, y: 4 }, { x: 8, y: 2 }, { x: 8, y: 6 }] },
  floors: [
    { surface: "stone_tiles", ramps: ["stone"] },
    { surface: "terrazzo", ramps: ["paving+1", "stone"] },
    { surface: "stone_tiles", ramps: ["paving+1"] },
  ],
  walls: [
    { id: "cream_over_panels", upper: { surface: "plaster", ramps: ["render"] }, lower: { surface: "panelling", ramps: ["@wood"], h: 16 }, trim: "@wood" },
    { id: "green_two_tone", upper: { surface: "plaster", ramps: ["grass+1"] }, lower: { surface: "plaster", ramps: ["foliage"], h: 14 }, trim: "render" },
    { id: "stripes_over_panels", upper: { surface: "stripes", ramps: ["stone", "@acc2"] }, lower: { surface: "panelling", ramps: ["@wood"], h: 16 }, trim: "@wood" },
  ],
  roles: {
    wood: ["wood", "hair_brown", "hair_blonde"],
    uph: ["cloth_teal", "cloth_navy", "cloth_green", "brick"],
    acc: ["cloth_navy", "brick", "cloth_teal"],
    acc2: ["cloth_mustard", "chalk", "cloth_red"],
  },
  pieces: [
    // Public wall: the notice board, leaflets, the door to the offices.
    { part: "notice_board", x: 0, dx: 8, wy: 16 },
    { part: "leaflet_rack", x: 2, wy: 20 },
    { part: "door_office", x: 3, floor: true },
    { part: ["poster", "sconce"], x: 4, wy: 18, chance: 0.9 },
    // Staff wall: windows either side of the clock.
    { part: "window_sash", x: 7, wy: 17 },
    { part: "clock", x: 8, dx: 8, wy: 12 },
    { part: "window_sash", x: 10, wy: 17 },
    // The counter and its return, with the files at the corner.
    { part: "filing_cabinet", x: 5, y: 0 },
    { part: "counter_side", x: 5, y: 1, len: 2 },
    { part: "counter", x: 5, y: 3, len: 5 },
    { part: "counter_screen", x: 6, y: 3 },
    { part: "counter_screen", x: 8, y: 3 },
    { part: "counter_bell", x: 7, y: 3, dx: 6 },
    { part: "papers", x: 9, y: 3, dx: 4, chance: 0.7 },
    // Staff side.
    { part: "office_desk", x: 6, y: 0 },
    { part: "monitor", x: 6, y: 0, dx: 9, dy: 2 },
    { part: "papers", x: 7, y: 0, dx: 6, dy: 2, chance: 0.7 },
    { part: "chair_back", x: 7, y: 1 },
    { part: "filing_cabinet", x: 10, y: 0 },
    { part: ["filing_cabinet", "plant_tall"], x: 11, y: 0 },
    { part: "doormat", x: 3, y: 8 },
  ],
  patches: [
    { kind: "surface", x: 6, y: 0, w: 6, h: 3, surface: "lino", ramps: ["paving"] },
    { kind: "rug", x: 5, y: 4, w: 5, h: 2 },
  ],
  choices: [
    {
      id: "waiting",
      options: [
        {
          id: "bench_by_board",
          pieces: [
            { part: "waiting_bench", x: 0, y: 0, len: 2 },
            { part: "stanchion", x: 6, y: 5, len: 3 },
            { part: "water_cooler", x: 0, y: 5 },
            { part: ["plant_tall", "coat_stand"], x: 11, y: 8 },
            { part: "plant_small", x: 4, y: 0, chance: 0.7 },
          ],
          spots: [
            { id: "waiting1", role: "public", x: 0, y: 0, facing: "down", pose: "sit" },
            { id: "waiting2", role: "public", x: 1, y: 0, facing: "down", pose: "sit" },
            { id: "queue1", role: "public", x: 7, y: 4, facing: "up" },
          ],
        },
        {
          id: "chairs_facing_counter",
          pieces: [
            ...[0, 1, 2].map((x) => ({ part: "chair_back", x, y: 6 })),
            { part: "plant_small", x: 0, y: 0 },
            { part: "water_cooler", x: 11, y: 6 },
            { part: ["plant_tall", "coat_stand"], x: 0, y: 8 },
            { part: "bin", x: 4, y: 0, chance: 0.6 },
          ],
          spots: [
            { id: "waiting1", role: "public", x: 1, y: 6, facing: "up", pose: "sit" },
            { id: "waiting2", role: "public", x: 2, y: 6, facing: "up", pose: "sit" },
            { id: "queue1", role: "public", x: 8, y: 5, facing: "up" },
          ],
        },
      ],
    },
  ],
  spots: [
    { id: "clerk1", role: "clerk", x: 6, y: 2, facing: "down" },
    { id: "clerk2", role: "clerk", x: 8, y: 2, facing: "down" },
    { id: "staff", role: "staff", x: 7, y: 1, facing: "up", pose: "sit" },
    { id: "files", role: "staff", x: 10, y: 1, facing: "up" },
    { id: "reader", role: "public", x: 0, y: 1, facing: "up" },
  ],
  hotspots: [
    { id: "window1", actions: ["file", "withdraw"], x: 6, y: 3, w: 1, h: 1, stand: { x: 6, y: 4 }, facing: "up" },
    { id: "window2", actions: ["draft"], x: 8, y: 3, w: 1, h: 1, stand: { x: 8, y: 4 }, facing: "up" },
    { id: "notices", services: ["town_data_detailed"], x: 0, y: 0, w: 2, h: 1, stand: { x: 1, y: 1 }, facing: "up" },
    { id: "leaflets", services: ["budget_papers"], x: 2, y: 0, w: 1, h: 1, stand: { x: 2, y: 1 }, facing: "up" },
    { id: "offices", actions: ["draft"], x: 3, y: 0, w: 1, h: 1, stand: { x: 3, y: 1 }, facing: "up" },
  ],
};

// ---------------------------------------------------------------------------
// The diner: warm. Booths under the windows, a jukebox, the counter with its
// stools, the back counter and the kitchen pass, the menu board, a TV in the
// corner, the door with its bell.

export const DINER: InteriorRecipe = {
  id: "interior.diner",
  version: 1,
  place: "diner",
  cols: 12,
  rows: 9,
  wallH: 40,
  door: 4,
  light: { lamp: "warm", night: 0.68, ceiling: [{ x: 1, y: 1 }, { x: 1, y: 4 }, { x: 1, y: 7 }, { x: 7, y: 3 }, { x: 8, y: 6 }] },
  floors: [
    { value: { surface: "checker", ramps: ["chalk", "asphalt"] }, weight: 2 },
    { surface: "checker", ramps: ["chalk", "cloth_red"] },
    { surface: "checker", ramps: ["chalk", "cloth_teal"] },
    { surface: "planks", ramps: ["@wood"] },
  ],
  walls: [
    { id: "paint_over_tiles", upper: { surface: "plaster", ramps: ["cloth_teal+1"] }, lower: { surface: "subway", ramps: ["chalk"], h: 18 }, trim: "@wood" },
    { id: "brick_over_tiles", upper: { surface: "brick", ramps: ["brick", "render"] }, lower: { surface: "subway", ramps: ["chalk"], h: 18 }, trim: "@wood" },
    { id: "cream_over_panels", upper: { surface: "plaster", ramps: ["stone"] }, lower: { surface: "panelling", ramps: ["@wood"], h: 16 }, trim: "@wood" },
    { id: "mustard_over_tiles", upper: { surface: "plaster", ramps: ["cloth_mustard+1"] }, lower: { surface: "subway", ramps: ["chalk"], h: 18 }, trim: "@wood" },
  ],
  roles: {
    wood: ["hair_blonde", "wood", "hair_auburn"],
    uph: [{ value: "cloth_red", weight: 3 }, "cloth_teal", "cloth_mustard", "brick"],
    acc: ["flag", "cloth_red", "cloth_teal"],
    acc2: ["chalk", "cloth_teal+1", "cloth_mustard+1", "cloth_red+1"],
  },
  pieces: [
    // Back wall.
    { part: "window_wide", x: 1, wy: 15 },
    { part: "neon", x: 3, wy: 10 },
    { part: "menu_board", x: 5, dx: 12, wy: 10 },
    { part: "kitchen_pass", x: 9, wy: 24, dx: -4 },
    { part: "tv", x: 10, dx: 10, wy: 7 },
    { part: "door_swing", x: 11, floor: true },
    { part: "sconce", x: 4, wy: 22 },
    // Booths: the back one by the window, two more down the left side.
    ...[0, 3, 6].flatMap((y) => [
      { part: "booth_bench", x: 0, y },
      { part: "booth_table", x: 1, y },
      { part: "booth_bench", x: 2, y, mirror: true },
    ]),
    { part: "table_lamp", x: 1, y: 0, dx: 5, dy: 2 },
    { part: "condiments", x: 1, y: 3, dx: 5, dy: 2 },
    { part: ["plates", "mug", "condiments"], x: 1, y: 1, dx: 1, dy: 4, chance: 0.8 },
    { part: ["plates", "mug"], x: 1, y: 4, dx: 1, dy: 4, chance: 0.7 },
    { part: ["table_lamp", "condiments"], x: 1, y: 6, dx: 5, dy: 2 },
    { part: ["plates", "mug"], x: 1, y: 7, dx: 1, dy: 4, chance: 0.6 },
    { part: "jukebox", x: 3, y: 0 },
    // The back counter, the counter and its stools.
    { part: "back_counter", x: 5, y: 0, len: 6, dy: -4 },
    { part: "coffee_machine", x: 5, y: 0, dx: 1, dy: -4 },
    { part: "pie_case", x: 6, y: 0, dx: 2, dy: -4 },
    { part: "register", x: 10, y: 0, dx: 3, dy: -4 },
    { part: "diner_counter", x: 5, y: 3, len: 5 },
    ...[5, 6, 7, 8, 9].map((x) => ({ part: "stool", x, y: 4 })),
    { part: ["mug", "plates", "condiments"], x: 5, y: 3, dx: 5, chance: 0.7 },
    { part: ["mug", "plates"], x: 6, y: 3, dx: 6, chance: 0.5 },
    { part: ["mug", "condiments"], x: 8, y: 3, dx: 5, chance: 0.6 },
    { part: "pie_case", x: 9, y: 3, dx: 2 },
    { part: "doormat", x: 4, y: 8 },
    { part: "door_bell", x: 4, front: true, dx: -7 },
  ],
  patches: [],
  choices: [
    {
      id: "floor",
      options: [
        {
          id: "two_tables",
          pieces: [
            { part: "cafe_table", x: 7, y: 6 },
            { part: "chair_front", x: 7, y: 5, dy: -2 },
            { part: "chair_back", x: 7, y: 7 },
            { part: "cafe_table", x: 10, y: 6 },
            { part: "chair_back", x: 10, y: 7 },
            { part: ["mug", "plates"], x: 7, y: 6, dx: 4, dy: -2 },
            { part: "mug", x: 10, y: 6, dx: 6, dy: -2, chance: 0.7 },
            { part: "news_rack", x: 11, y: 8 },
            { part: ["coat_stand", "plant_small"], x: 3, y: 8, chance: 0.8 },
          ],
          spots: [
            { id: "table1", role: "patron", x: 7, y: 7, facing: "up", pose: "sit" },
            { id: "table2", role: "patron", x: 7, y: 5, facing: "down", pose: "sit" },
            { id: "table3", role: "patron", x: 10, y: 7, facing: "up", pose: "sit" },
          ],
        },
        {
          id: "one_table_and_rack",
          pieces: [
            { part: "cafe_table", x: 8, y: 6 },
            { part: "chair_back", x: 8, y: 7 },
            { part: "chair_front", x: 8, y: 5, dy: -2 },
            { part: "mug", x: 8, y: 6, dx: 5, dy: -2 },
            { part: "news_rack", x: 6, y: 8 },
            { part: "plant_tall", x: 11, y: 8 },
            { part: ["coat_stand", "plant_small"], x: 3, y: 8, chance: 0.8 },
          ],
          spots: [
            { id: "table1", role: "patron", x: 8, y: 7, facing: "up", pose: "sit" },
            { id: "table2", role: "patron", x: 8, y: 5, facing: "down", pose: "sit" },
            { id: "regular", role: "patron", x: 10, y: 6, facing: "left" },
          ],
        },
      ],
    },
  ],
  spots: [
    { id: "owner", role: "owner", x: 6, y: 2, facing: "down" },
    { id: "cook", role: "cook", x: 9, y: 1, facing: "down" },
    { id: "waiter", role: "waiter", x: 4, y: 5, facing: "left" },
    { id: "booth_back_l", role: "patron", x: 0, y: 0, facing: "right", pose: "sit" },
    { id: "booth_back_r", role: "patron", x: 2, y: 1, facing: "left", pose: "sit" },
    { id: "booth_mid_l", role: "patron", x: 0, y: 4, facing: "right", pose: "sit" },
    { id: "booth_mid_r", role: "patron", x: 2, y: 3, facing: "left", pose: "sit" },
    { id: "booth_front_l", role: "patron", x: 0, y: 6, facing: "right", pose: "sit" },
    { id: "stool1", role: "patron", x: 5, y: 4, facing: "up", pose: "sit" },
    { id: "stool2", role: "patron", x: 8, y: 4, facing: "up", pose: "sit" },
    { id: "stool3", role: "patron", x: 9, y: 4, facing: "up", pose: "sit" },
  ],
  hotspots: [
    { id: "booth_mid", actions: ["lobby", "trade_favor"], x: 0, y: 3, w: 3, h: 2, stand: { x: 3, y: 4 }, facing: "left" },
    { id: "booth_front", actions: ["lobby", "trade_favor"], x: 0, y: 6, w: 3, h: 2, stand: { x: 3, y: 7 }, facing: "left" },
    { id: "booth_back", actions: ["take_bribe"], late: true, x: 0, y: 0, w: 3, h: 2, stand: { x: 3, y: 1 }, facing: "left" },
    { id: "counter", services: ["overheard"], x: 5, y: 3, w: 5, h: 1, stand: { x: 7, y: 4 }, facing: "up" },
  ],
};

/** Every recipe, by id. The composer keeps older versions callable here (`id@version`). */
export const RECIPES: readonly InteriorRecipe[] = [COUNCIL_CHAMBER, TOWN_HALL, DINER];

export function recipeFor(place: string, version?: number): InteriorRecipe | undefined {
  return RECIPES.find((r) => r.place === place && (version === undefined || r.version === version));
}
