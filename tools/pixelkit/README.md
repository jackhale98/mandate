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
- `art/mandate`: Mandate's pack (all rights reserved): palette, lighting, characters, buildings, tiles, props, terrain, interiors, works. `pack.ts` builds it (`PACK_VERSION`).

## Interiors

Art round 7 (DEC-088): the council chamber, the town hall's front office and
the diner as small single-screen rooms, in the street's scale and view (16 px
floor tiles, 16×24 people, three-quarter top-down: the floor from above and
the back wall's face, no side-wall faces).

| File | What |
|---|---|
| `interiorsParts.ts` | 12 surfaces (floors: planks, parquet, checker, carpet, stone tiles, terrazzo, lino; walls: plaster, panelling, brick, subway tile, stripes) and 63 parts: windows, doors, wall fixtures, furniture, things that stand on furniture, a doormat, a rug |
| `interiorsRecipes.ts` | The recipe type and the three recipes |
| `interiors.ts` | The composer, the recipe validator, placeholder people, the reference renderer |
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

**A recipe** (`InteriorRecipe`) is plain data:

- `id`, `version` (DEC-085: a change that moves anything for the same seed bumps it; old versions stay callable), `place` (the `places.jsonc` key);
- `cols` × `rows` floor tiles, the back wall's height `wallH` (px), the entrance column `door` (front wall, bottom row);
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

## What a web integration needs

- **Renderer.** Compose the room once on entering (it takes a few ms) and cache it by place, recipe version and seed. Draw `base` graded as in `renderInterior` (indoor ambient on non-`outdoor` pixels, the moods palette on `outdoor` pixels), then `sprites` and people merged by `key`, then glows, pools and daylight. The canvas version is the street renderer's: multiply by an ambient colour, add pools with `lighter`, blit emissives over the top. Time of day comes from the turn's slots (`turnClock.ts`), as on the street.
- **People.** The engine's place view (DEC-079) says who is present; the UI assigns them to spots by role (occupation → role: `occupation.diner_owner` → `owner`, councillors → `councillor`, the player's own seat → `player`), in the recipe's order, and draws sitters cropped at the hips (`sitCut`) on their seat. Extra people stand on free floor tiles near the door. Walking uses `walk` and `reachable`.
- **Hotspots.** Tapping a hotspot walks to its `stand` tile and offers its `actions` and `services`, filtered by what the engine's place view allows now (`late` ones only late at night). A place kind without a recipe keeps the place panel (DEC-088).
- **Content.** Recipes are JSON-shaped on purpose. They move to `content/base/interiors.jsonc` (one entry per recipe and version), validated by a schema generated from a Rust `InteriorRecipe` type (DEC-053), with a `content validate` check that every hotspot's actions exist in `places.jsonc` for that place and every part id exists in the art pack. Places name their recipe (`"interior": "interior.diner"`) in `places.jsonc`. The parts stay in the art pack; the recipe validator's tile checks run in both the Rust content check and these tests.

## Works

Art round 8 (DEC-099, "works you can see"): funded projects change the town,
stalled ones sit half-built and decay, and finished ones carry the sponsor's
plaque. The kit draws eight kinds in every phase:

- `bridge`: a deck replaced;
- `flood_wall` and `levee`: runs along a bank;
- `civic`: a clinic, library or community centre in a free slot;
- `housing`: infill;
- `road`: resurfacing, orthogonal or 45°;
- `mine` and `factory`: also how an existing works shows idle, working, busy and closed.

No palette colour was added (still 114). `PACK_VERSION` is `mandate-p0.9`.

```text
node --import ./bin/hooks.mjs art/mandate/worksSheets.ts --out target/pixelkit/works
```

| File | What |
|---|---|
| `works.ts` | The dress rules: `worksLook(kind, phase, progress, monthsStalled, seed, opts?)`; the tables (`BUILD_FROM`, `DECAY_FROM`, `ACTIVITY_FROM`, `WORKS_RULES`, `WEAR`, `MARKS`); `wearLevel`, `workerLook` |
| `worksParts.ts` | 42 drawn parts: cabin, tower crane, excavator, piles, skip, signs, cones, boards (fresh, faded, peeling), headframe (turning, rusted, half-built), spoil tip, shaft cap, chimney, smoke, lorries, goods, yard gates, plaque, ribbon, plastic sheet, buddleia |
| `worksLines.ts` | Runs on the street grid's lines: Heras fencing, hoarding, barriers, railings, flood walls, levees, sheet piles |
| `worksCompose.ts` | `composeWorks(look, site, kit, seed)`: places a look on a site and draws what depends on the site's shape |
| `worksScenes.ts`, `worksSheets.ts` | Sample sites in the round 4 and 5 towns, a compositor with the works layers, and the review sheets |

**The rules** (`worksLook`) are pure, keyed and monotonic, like the dress pass
(TOWN_GENERATION.md §9.2):

- `progress` (whole percent) picks a build stage at 0, 20, 45, 80 and 100 %: the site set up, the ground work, the structure rising, finishing, and complete (the ribbon waits for the opening). A stalled or cleared project keeps the stage it reached.
- `monthsStalled` picks a decay stage at 0, 3, 9 and 18 months. Stalling stops the site at once: no workers, lights off, the crane's load set down, machines parked, gates shut. Decay then adds marks.
- **Marks** (weeds, scrub, moss, litter, puddles, potholes, gullies, graffiti, rust, a flapping sheet, buddleia) have per-kind counts per stage that never fall. Mark `n` always lands where the seed and `n` put it, so a later stage only adds.
- **Wear chains** move one way:
  - Heras and hoarding panels stand, sag, then fall, each panel at its own seeded stage;
  - boards fade, then peel;
  - cabins are broken into;
  - the crane, excavator, headframe and gates rust;
  - a frame weathers, then is overgrown;
  - scaffolding loses its boards;
  - a boarded shell becomes derelict.
- Open mines and works take `opts.activity` (0–100: idle, working, busy). A mine also takes `opts.extraction`: round 4's three heaps, then a flat-topped tip. A works that closes is `stalled` at 100 %. `opts.sponsor: false` leaves the plaque off.
- The look is plain data. Each part has a `role` (structure, ground, perimeter, closure, yard, front, entrance, crane, marks, people), an instance `n`, and optionally `frames` (idle loops), `lit`, `mark`, `level`, `wear` and `worn`. `WORKS_RULES_VERSION` versions the rules (DEC-085).

**Sites** (`WorksSite`) come from the plan:

- a connector (bridge);
- a run of `LinePiece`s (walls, levees);
- a slot and its finished `FacadeSpec` (civic, housing, factory);
- road cells, with the closed lane as a half-plane (a row, or one side of a 45° street's centre line);
- the mine's headframe and heap cells.

Each site also names a yard, the board's cell and the entrance. `composeWorks` returns:

- `tints`: ground remaps (planed or fresh asphalt);
- `decals`: deck states, foundations, slabs, marks on the ground;
- `sprites`: with a depth row, frames and lit layers;
- `people`: workers' looks, composed on the device;
- `replaces`: the slot whose plan building the works stands in for.

**Lines.** A piece joins two mid-sides of a cell, a side and the centre, or
two sides round a sharp corner through the centre. These are the lines round
5's 45° streets use for their edges, so a fence or wall can follow a diagonal
street or bank and meets the next piece at the same point. A run is drawn as
one heightfield (each footprint pixel stands a column up the screen) and cut
into one sprite per cell for depth sorting. Thin runs keep their mesh open,
because the kit's outline goes round the outside only.

**Budgets** (the round 7 precedent): a part at most 14 colours, a line piece
10, a composed site 56 (facades excepted). No colour is added.

**What the dress pass needs from the engine** (TownStateView, per project):

- the kind;
- its place in the plan (connector id, slot id, bank run or road strip);
- the phase, progress, and months since the stall;
- for an open mine or works, the activity and extraction;
- the sponsor, for the plaque's label and hotspot (the art draws no letters).
