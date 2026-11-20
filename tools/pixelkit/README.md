# pixelkit

Pixel art from hand-drawn parts composed by code (working name; DEC-084). The
plan, the style guide and each art round's results are in
`docs/design/research/PIXEL_ART_TOOL.md`.

```text
npm run check                                   # typecheck and tests
node bin/pixelkit.mjs all --out target/pixelkit # palette, sheets, lints, the built pack
node --import ./bin/hooks.mjs art/mandate/interiorsSheets.ts --out target/pixelkit/interiors
```

- `src/core`: the tool (compose, recolour, outline, palette, autotile, lints). Imports nothing from `art/` or the game.
- `src/cli`, `src/node`: the CLI and contact sheets.
- `art/mandate`: Mandate's pack (all rights reserved): palette, lighting, characters, buildings, tiles, props, terrain, interiors. `pack.ts` builds it (`PACK_VERSION`).

## Interiors

Art round 7 (DEC-088): the council chamber, the town hall's front office and
the diner as small single-screen rooms, in the street's scale and view (16 px
floor tiles, 16×24 people, three-quarter top-down: the floor from above and
the back wall's face, no side-wall faces).

| File | What |
|---|---|
| `interiorsParts.ts` | 12 surfaces (floors: planks, parquet, checker, carpet, stone tiles, terrazzo, lino; walls: plaster, panelling, brick, subway tile, stripes) and 63 parts: windows, doors, wall fixtures, furniture, things that stand on furniture, a doormat, a rug |
| `interiorsRecipes.ts` | The recipe type (mirroring the Rust `InteriorRecipe`) and the JSONC parser; the recipes themselves are content, `content/base/interiors.jsonc` |
| `interiorsContent.ts` | The recipes as the Node tools see them: read from `content/base/interiors.jsonc` (sheets, tests) |
| `interiorsCatalog.ts` | The part catalogue the Rust content check validates recipes against (`crates/mandate-content/src/interior_parts.json`, generated with `--write`, kept equal by the tests) |
| `interiors.ts` | The composer, the recipe validator, placeholder people, the grade (`interiorGrade`, `gradeRoom`, `gradeImage`: shared by the reference renderer and the game) and the reference renderer |
| `interiorsSheets.ts` | Review sheets (each room at four seeds by day and evening, 1×, 3×, through the day, debug, parts) |

**Parts** are drawn in *roles*, not colours: `wood`, `uph` (upholstery),
`acc`, `acc2`, `wall` and `trim` vary by room and seed; steel, brass, paper,
stone, leaves and the rest are fixed ramps. The legend is at the top of
`interiorsParts.ts`. Furniture gets the automatic outline (D2); floors, walls
and things hung on the wall don't. Strips (`desk`, `counter`, `pew`, `rail`,
…) are drawn as a start, a middle and an end and run any number of tiles.
Seats say where a sitter's hips go (`sit`) and which rows are drawn over the
sitter (`front`: a seat's front, a pew's backrest). Lit parts (`lit`) glow
when the lamps are on, or always (`glow: "always"`: the TV, the jukebox, the
neon sign).

**A recipe** (`InteriorRecipe`) is content (DEC-098 §4): `content/base/interiors.jsonc` is the single source of truth, typed in Rust (`mandate-content/src/interiors.rs`, schema generated, DEC-053) and checked by `content validate`. This tool reads that file (`interiorsContent.ts`); the game receives the recipe of the place it is entering in the engine's place view (`PlacePanelView.interior`, the content's JSON). Round 8 moved the TypeScript recipes there (the chamber's and diner's unchanged; the town hall's lower half furnished, version 2). A recipe holds:

- `id`, `version` (DEC-085: a change that moves anything for the same seed bumps it; old versions stay callable), `place` (the `places.jsonc` key);
- `cols` × `rows` floor tiles, the back wall's height `wall_h` (px), the entrance column `door` (front wall, bottom row);
- `light`: lamp colour (`warm` or `white`), how much of the night's grade reaches inside, and unseen ceiling lights;
- materials: weighted floor styles, wall styles (upper surface, optional lower band, trim), and the ramps of the variable roles. A ramp reference is a palette ramp, optionally shifted (`grass+1`), or a role (`@wood`);
- `pieces`: parts on floor tiles (`x`, `y`, `len`, `mirror`, px nudges) or on the back wall (a column and `wy`, the centre's height on the wall, or `floor: true`), or over the entrance (`front: true`). A piece can name alternatives the seed picks from, and a `chance`;
- `patches`: another floor surface, a rug or a dais (which lifts what stands on it);
- `choices`: arrangements the seed picks between (the chamber's horseshoe or rows of desks, the town hall's waiting area, the diner's tables). Each option brings its own pieces, patches, spots and hotspots;
- `spots`: where people present stand or sit, by `role` (`chair`, `clerk`, `councillor`, `player`, `staff`, `public`, `owner`, `cook`, `waiter`, `patron`), with a facing and a pose;
- `hotspots`: a rectangle of tiles, the engine `actions` and `services` offered there (`vote`, `introduce`, `amend`, `withdraw`; `draft`, `file`; `lobby`, `trade_favor`, `take_bribe` late at night; `town_data_detailed`, `budget_papers`, `overheard`), and the tile the player stands on to use it.

`validateRecipe` checks every option combination and every alternative:
known parts, footprints inside the room and not overlapping, wall parts on the
wall face, spots and hotspot stands on walkable tiles reachable from the door,
seated spots on seats, hotspot stands next to their hotspot. The tests run it,
and also check 60 seeds of each recipe against the composed walk grid.

**Composing** (`composeInterior(recipe, seed, palette, lightness)`) is pure
and keyed: rendezvous picks from `hash32(seed, recipe.id, recipe.version)`, no
`Math.random`, no clock. `interiorSeed(townSeed, placeId)` gives the seed. The
result:

- `base`: walls, floor, patches, wall parts, the wall tops around the room and the entrance, with furniture shadows baked in;
- `sprites`: furniture and what stands on it, each with a depth `key`; people use `feetY * 4`, so they slot in between. Seats are split into the part behind a sitter and the part over them;
- `outdoor` (the view through windows), `baseGlow`, `farLit`, `lights` (lamp and ceiling pools), `windows` (where daylight falls);
- `walk` (blocked, floor, seat), `lift` (dais), `door`, and the chosen `spots` and `hotspots`;
- `picks`: what the seed chose (for review and debugging).

**Light.** `renderInterior(room, palette, lighting, minutes, actors)` grades a
room for a time of day from the same lighting table as the street: by day a
third of the street's grade and banded daylight under the windows; once the
lamps come on (`lampLevel(lights)`) the room is lit by them, not the sky (the
night grade's brightness in the lamps' colour, plus banded pools around each
lamp); the view through the glass takes the street's palette-moods colour for
the hour, with far windows lit at night. Glowing parts are drawn after grading.

## The web integration (DEC-088, M6)

`web/src/interior/` (see `docs/design/UI.md`, "Interiors"):

- **Renderer.** The room is composed once on entering and cached by place, recipe version and seed (`roomCache.ts`, with the pack version). The static layers are graded per 5-minute light step with `interiorGrade` + `gradeRoom` (the reference renderer's maths, checked equal pixel for pixel by a test) and blitted; people are graded where they stand (`gradeImage`) and merged with the sprites by `key`; blob shadows as in the sheets. Time of day comes from the turn's slots (`turnClock.ts`), as on the street. Nothing in a room moves by itself, so the loop stops when the player stops (DEC-080).
- **People.** The engine's place view says who is present and how (`here_as`: presiding, member, works, visitor; never an office or a name, DEC-098); the recipe's `cast` maps that to spot roles, taken in the recipe's order (`cast.ts`); sitters are cropped at `sitCut`; extra people stand on free floor nearest the door. Staff roles nobody present takes (`extras`: the clerks, the cook, the waiter, the usher) are drawn as scenery, like the street's townsfolk: no name, no conversation (fewer in the evening).
- **Hotspots.** A tap walks to the `stand` tile and opens the hotspot's sheet, with its `actions` and `services` filtered by what the place view offers now (`late` ones only once the day's slots are spent), through the place panel's forms.
