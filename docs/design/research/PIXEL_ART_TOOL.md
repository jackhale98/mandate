# Research and plan: a pixel-art generator for Mandate

**Purpose:** A decision-ready plan for a tool that produces attractive, consistent, tunable pixel art for Mandate (characters, buildings, tiles, props, icons), robust enough to become its own project. Nothing here is built yet. The owner asked for this plan before any tool is built ("plan that before actually building the tool to ensure it's robust, provides what we need and can actually be designed to output attractive and nice pixel art").
**Date:** 2026-11-19. One research pass over web sources (listed at the end, cited as [ID]). Most are practitioner guides, modding wikis and developer interviews, not measured studies.
**Status:** Proposal. Nothing here changes an accepted decision. Section 9 lists what the owner must decide.

## Executive summary

1. A generator cannot invent good pixel art. It can multiply a small library of good hand-drawn parts into thousands of consistent sprites. Shipped games of this kind work this way: Stardew Valley's farmer, RimWorld's pawns, Caves of Qud's recoloured tiles [SDV-FS, RW, CoQ].
2. Recommendation: **hand-drawn part libraries, composed by code, coloured from designed palette ramps.** Code handles selection, layering, recolouring, outlines, autotiling and facade layout. People draw the parts.
3. Characters are composed on the device from a part atlas and an appearance "genome" that the engine stores (DEC-031). Tiles, autotile sets and icons are built ahead of time. Buildings are assembled on the device from modular kits by a small facade grammar.
4. Lighting follows DEC-080: no per-pixel lighting. Every asset ships with an emissive layer (lit windows, lamps), shadow metadata (footprint and height, so the renderer can draw shadows that follow the sun), and the palette ships with day-cycle grading keyframes.
5. Tool form: a TypeScript library with a CLI, a browser "studio" for tuning (sliders, seed browsing, contact sheets, favourites), and a small runtime the game imports. One compositor serves the tool, the tests and the game, so a preview and the game always agree.
6. Quality control: a written style guide, automated lints (palette, orphan pixels, outlines, contrast, colour-blind checks, clone rate in crowds), golden-image tests, and contact sheets the owner reviews.
7. AI image generation: not for shipped pixels. At 16 px it gives off-grid, off-palette output that needs a redraw anyway, and purely generated images cannot be copyrighted in the US [AI-CLEAN, USCO].
8. Start with a **validation spike** (about 6 agent tasks): one palette, one building kit, one tile set, 20 townsfolk, rendered as one street at day and dusk and judged against a rubric. Then an MVP for M6 (about 14 tasks) and an expansion for M7–M8 (about 12 tasks).
9. Keep it in this repo (`tools/pixelkit/`, a working name) with a hard boundary, and split it out after M6 once its interface has held steady.

## 1. What makes pixel art look good, and what makes it look like programmer art

The sources agree on a short list. Each item becomes a rule in the style guide (§6) and, where a machine can check it, a lint (§7).

| Principle | What the sources say | What "programmer art" does instead |
|---|---|---|
| Limited, designed palette | Fewer colours give cohesion and control; the count is a guideline [PJ]. Build ramps from a vivid middle colour, peak saturation mid-ramp, never 0 or 100 [SLY-1]. | Picks colours one by one (the spike's `WALLS`/`ROOFS` arrays): no shared ramps, so nothing harmonises |
| Hue-shifted ramps | Shift hue as well as brightness: shadows cooler, highlights warmer. Slynyrd uses up to 20° per step and calls "straight ramps" a beginner mistake [SLY-1]. | Darkens with black overlays (`rgba(0,0,0,0.14)`), which greys colours and flattens them |
| Value contrast | Value carries depth and readability; check forms in greyscale [PP-COL]. | Similar values everywhere; sprites sink into the ground |
| One light direction | Shade every form from one consistent light source [PJ]. | Each asset lit differently, or not at all |
| Clusters, not pixels | Judge pixels by the shapes they form; avoid stray single pixels ("noise") except for highlights and tiny essential detail [PJ]. | Random speckle (the spike's grass: random 1×2 dots) |
| No pillow shading or banding | Pillow shading darkens around the edges and ignores the light. Banding lines up pixel runs and exposes the grid [PJ]. | Concentric outlines and edge darkening |
| Hand anti-aliasing, sparingly | Apply by hand; match each AA pixel's value to how much of it the line covers; longer segments need longer AA; keep AA inside the sprite, because outer AA breaks on unknown backgrounds; on small sprites AA reads as noise [PP-AA]. | Canvas path fills, which anti-alias automatically and add off-palette colours |
| Selective outlines | A full outline with light variation holds up better than sel-out; sel-out only works when the background is known [PJ]. | Black outlines everywhere, or none |
| Dithering rules | Dither to buffer between colours, not to fill areas; never random [PJ]. LPC: "sparingly if at all" [LPC-SG]. | Dither as texture; random noise |
| One pixel scale | Mixed pixel sizes look wrong; fractional scaling makes pixels uneven and shimmer when the camera moves [MDN, JJ]. DEC-080 already snaps zoom to whole steps. | Sprites scaled by 1.5× next to 1× tiles |
| Readable silhouettes at small sizes | At 16 px, realistic proportions read as a stick figure: enlarge the head (a third to a half of the figure), and use height to tell young, old, and body types apart, in one consistent system [SLY-22]. | Realistic proportions; every figure the same height |
| Tiny walk cycles | Top-down: draw down, up and side, mirror the side view; a 4-frame walk reuses a run cycle minus the full-stride frames, played slower; 8-direction movement with 4 facings is acceptable [SLY-22, SLY-55]. | 3 frames, legs only, arms static |
| Standardised modules | Townscaper's author: tiles "have to be standardized" to get the handcrafted look; big shapes predictable, small details varied [TS]. | Every building drawn with ad-hoc rectangles |

**For Mandate:** the current placeholder art (`web/spikes/hub/src/art.ts`) shows most of the right-hand column. That is expected of placeholders. The tool's job is to make the left-hand column the default, so that code can't easily produce the right-hand one.

## 2. Existing approaches and tools

### 2.1 Layered characters (paper doll)

- **Stardew Valley's farmer** is 16×32 per frame, stacked bottom to top as body, pants, shirt, accessories, hair, hat, arms. The game recolours skin, eyes, boots, pants and sleeves by matching specific palette colours in the base sprite. Left and right are mirrors [SDV-FS]. NPC sprites are 16×32 and portraits 64×64 [SDV-SZ]. One person drew all of it and had never done pixel art before; he got better through practice [SDV-PCG]. This is the closest precedent for Mandate's scale.
- **RimWorld** overlays head and body apparel 1:1 onto heads and bodies, needs three facings (south, north, east; west mirrors east), draws apparel per body type (five), and colours apparel through masks: red marks the primary-colour area, black stays uncoloured [RW]. Players report layering bugs (outer clothes over hair, back hair between layers) [RW-LAYER]. Lesson: the layer order and occlusion rules need to be explicit data, and every combination needs testing.
- **Caves of Qud** draws tiles in three "colours": black takes the foreground colour, white the detail colour, transparent the background. A fourth colour blends by its red channel. Tiles are 16×24 [CoQ]. Lesson: a recolour-by-slot format makes one drawing serve hundreds of creatures.
- **LPC Universal Spritesheet Character Generator** is the largest open paper-doll library. The generator code is GPL-3.0. Each image has its own licence (CC0, CC-BY, OGA-BY, CC-BY-SA, GPL), and CC-BY-SA requires credit, no DRM, and share-alike for derivatives [LPC-GEN]. The style is a 32×32 grid [LPC-SG], with larger characters than Mandate's 16 px tiles. **Assessment:** a useful reference for animation tables and layer naming, but a poor source of art. It does not match the 16 px scale (DEC-065), share-alike conflicts with the permissive-only policy (OVERVIEW §9), and crediting hundreds of authors per sprite is a real burden.
- **Kenney** packs are CC0 [KEN]. Tiny Town is 16×16 and is DEC-065's initial art. Kenney's modular Roguelike Characters (16×16) can be assembled from parts but come without animation [KEN-RC]. They are good placeholders and proportion references, and too small and too generic to be Mandate's final look.

### 2.2 Fully procedural sprites

- **Dave Bollinger's pixel spaceships** and their many ports (zfedoran's MIT JavaScript version is the best known) start from a 2D mask of cell types, randomise it, mirror it, then outline and colour the result [PSG]. This works for symmetric, abstract subjects (ships, bugs, emblems). It produces neither readable people nor architecture.
- **Assessment:** procedural *shapes* hit a low quality ceiling for people and buildings. Procedural *selection, placement and recolouring* of hand-made parts is where code adds value.

### 2.3 Palette swaps and colour ramps

All three shipped examples above recolour by slot (key colours, masks, or 3-colour tiles). The general form: parts are drawn in "slot colours" (for example skin shade 1 to 4, cloth A shade 1 to 3), and a lookup table maps each slot to a real palette colour at composition time. One drawing then covers every skin tone, hair colour and outfit colour. Palettes designed in a perceptual space (Oklab/OKLCH) give ramps whose steps look even [OK].

### 2.4 Buildings and towns

- **Shape grammars:** CGA shape [CGA] splits a facade into floors and bays, and repeats window modules, with rules. A tiny 2D version fits pixel facades: split a wall into storeys of 32 px, split storeys into 16 px bays, choose a door bay, fill the other bays with windows from the building family's kit.
- **Townscaper** places hand-made, standardised modules with wave function collapse and marching cubes [TS]. The lesson for us is the split between hand-made modules and procedural placement, not the 3D method.
- **Wave function collapse** (overlapping and simple tiled models) fills a grid with patterns consistent with a sample or with adjacency rules. Contradictions are rare but possible, and deciding whether a valid fill exists is NP-hard [WFC]. Mandate's layouts already come from the town generator, which places roads and lots. WFC is not needed for layout. It may be useful later for decorative fill (park paths, rooftop clutter). The reference repository's licence is not a standard open-source licence [WFC-LIC], so any implementation would be our own.

### 2.5 Autotiling

- Most systems are **marching squares (16 tiles, corner-based) or blob (47 tiles, 8-neighbour)**, and in practice nobody draws 47 unique tiles: most are copied or rotated [BTB-AT, BTB-RU].
- **Dual grid** puts the visible tiles on cell corners, so two terrains need 16 tiles (15 plus empty, or 6 with rotation) instead of 47 [DG].
- **Quarter tiles** (RPG Maker) need 5 source tiles if they can rotate, and assemble the 48 blob tiles from quarters ahead of time [BTB-QT].
- **For Mandate:** dual grid for terrain pairs (grass and pavement, grass and water, pavement and road kerbs), with the tool expanding 5–6 hand-drawn templates into each set at build time.

### 2.6 Shading from depth and normals

- **Dead Cells** renders 3D models "in a very small size and without antialiasing", with a normal map per frame so a toon shader adds volume. The team chose this to favour animation over hand-drawn detail [DC].
- **Eastward** (two people) hand-crafted every asset, hand-painted bump maps "one by one", and lights them with a custom engine [EW].
- **Sprite Lamp** builds normal and depth maps from a few hand-painted lighting directions [SL].
- **For Mandate:** these need per-pixel lighting, which DEC-080 rules out for Canvas 2D. Normal maps also look too smooth at 16 px unless hand-polished. One cheap idea survives: the tool can emit a *sun-facing rim mask* per asset (the pixels on the lit edge), which the renderer can add warmly at golden hour with one extra draw. This is optional and is tested in the spike.

### 2.7 Upscaling and downscaling

- Fractional scaling makes some pixels wider than others, and motion exposes it. Integer scaling with nearest-neighbour sampling avoids it [JJ, MDN]. DEC-080 already draws at CSS resolution or lower, with `image-rendering: pixelated` and whole-number zoom.
- Downscaling a large image to pixel art does not give pixel art. It gives blurred clusters and stray colours (see AI below).
- Portraits drawn at a larger size are a different asset, not an upscaled sprite.

### 2.8 AI-assisted pixel art

- **Output quality:** diffusion models paint images that resemble pixel art: block sizes vary, pixels sit off the grid, edges anti-alias, and colour counts explode (one sprite asked to use 12 colours came back with 44,725). Cleanup means detecting the grid, downscaling, quantising, then fixing by hand. One practitioner says at 32×32 and below conversion does not keep a character's detail, and the output is better used as a reference [AI-CLEAN]. Dedicated tools (Retro Diffusion, an Aseprite extension) add palette control and grid snapping [RD], but output sizes are fixed (32, 64) and each generation is a separate drawing, so parts generated one at a time don't line up on shared anchors.
- **Consistency:** a paper doll needs every hair style aligned to the same head in three facings and four walk frames. Generation does not guarantee alignment between images, so each part would be redrawn by hand anyway.
- **Copyright:** the US Copyright Office's 2025 report says outputs generated from a prompt alone are not copyrightable, and choosing one output from several is not authorship. Human selection, arrangement, and creative modification can be protected [USCO]. Shipped AI pixels could be copied freely.
- **Stores:** Steam requires disclosure of AI-generated content that players see (rules rewritten in 2026 to cover only player-facing content) [STEAM-AI]. Mandate ships on the web, but a store release later would need the disclosure.
- **Honest assessment for a small team:** useful for mood boards and quick exploration of a building's look before someone draws it. Not useful for shipped 16 px parts.

### 2.9 What similar games did, in one line each

| Game | Art approach | Source |
|---|---|---|
| Stardew Valley | One self-taught artist; 16 px tiles, 16×32 characters, 64×64 portraits; layered, palette-matched farmer | [SDV-PCG, SDV-SZ, SDV-FS] |
| Eastward | Two-person studio; everything hand-crafted in Aseprite; hand-painted bump maps; engine lighting | [EW] |
| RimWorld | Layered pawns: body types, heads, apparel overlays, mask-based colouring | [RW] |
| Caves of Qud | 16×24 tiles recoloured from 3–4 slot colours | [CoQ] |
| Townscaper | Hand-made standardised modules, placed procedurally | [TS] |
| Dead Cells | 3D models rendered small without AA, plus normal maps and toon shading | [DC] |

## 3. Requirements from the game

### 3.1 Requirements table

| ID | Requirement | From | How we check it |
|---|---|---|---|
| R1 | Top-down three-quarter view, 16 px tiles, one pixel scale in the scene | DEC-049, DEC-065, DEC-080 | Style guide; lint rejects off-grid sizes |
| R2 | Layered characters composed from appearance parameters, 4 facings (3 drawn, 1 mirrored) | DEC-009, DEC-031, DEC-049 | Every part has down, up and side views and all frames |
| R3 | Ambient crowds must not look like clones; ambient people are generated on demand and never stored | DEC-018, DEC-009 | Clone-rate lint on sampled crowds (§7) |
| R4 | Same parameters, same sprite, on every device and every load | DEC-041 spirit, DEC-018 | Golden images; build twice gives identical pixels |
| R5 | An art update must not change a known character's look unless intended | Owner brief | Stored keys never remapped; alias table; golden diffs reviewed |
| R6 | Buildings for every place kind (23 in `PlaceKind`) and neighbourhood kind (6), plus filler housing | DEC-079, `places.jsonc`, `world.rs` | Coverage lint against the registries |
| R7 | Visible conditions from the simulation: boarded storefronts, construction, flood damage, protests | DEC-009 | Condition overlays in every shop and house kit |
| R8 | Towns feel distinct | DEC-077 | Owner side-by-side test of two towns (§8 rubric) |
| R9 | Continuous day cycle with pre-rendered lighting: grading keyframes, lit windows and lamps at dusk, sun-following shadows | DEC-013, DEC-080 | Each asset ships emissive and shadow data; dusk contact sheet |
| R10 | Canvas 2D: no shaders; static layers pre-rendered; one atlas for sprites | DEC-080 | Runtime uses only `drawImage`, composite modes, and `ImageData` |
| R11 | Mobile PWA budgets: hub ≥ 30 fps on a mid-range phone; small download | DEC-048, OVERVIEW §8 | Art payload and decoded-memory budgets (§3.5) |
| R12 | Colour is never the only signal | DEC-051 | Colour-blind lint; every coloured cue has a shape cue |
| R13 | Diverse skin tones and bodies without stereotypes | Owner brief | Independence rules (§3.3); review checklist |
| R14 | No text in art; the engine emits keys, the UI draws text as HTML | DEC-054, DEC-080 | Signs are pictograms; lint for no glyph parts |
| R15 | Player appearance is editable at character creation | DEC-036 | The studio's sliders are reusable as a creator UI |
| R16 | Strategic map shares the palette in a cartographic style | DEC-049 | Map biome colours come from the palette's terrain ramps |
| R17 | Brand palette consistent with the logo and UI | DEC-081, `styles.css` | Palette anchors include the brand tokens |
| R18 | Permissive licences only for code dependencies | OVERVIEW §9 | npm licence check covers the tool |
| R19 | Engine crates stay free of art and browser code | AGENTS.md | The engine stores semantic keys only |

### 3.2 Asset inventory for v1 (M6–M8)

Counts are for the MVP (P1, M6) unless marked M7/M8. "Parts" are drawings; "variants" are what composition and recolouring produce from them.

**Tiles (16×16, built ahead of time)**

| Group | Contents | Approx. drawn tiles |
|---|---|---|
| Ground | Grass (3 base variants plus decals: flowers, worn patches), dirt path, sand, gravel, farmland rows | 12 |
| Paved | Pavement (2 patterns), plaza stone, car park, kerbs | 8 |
| Roads | Asphalt (2), lane markings (centre, edge), zebra crossing (both orientations), junction box, manholes | 10 |
| Water | River and harbour water, animated (3–4 frames), shore, quay edge | 8 |
| Parks | Lawn, flower beds, hedge, park path | 6 |
| Autotile templates | 5–6 templates per terrain pair, expanded to dual-grid sets: grass/pavement, grass/path, grass/water, pavement/road, grass/sand, plaza/lawn, hedge, fence | ~45 templates giving ~130 tiles |
| Biome swaps | Grassland, steppe/dry, boreal (palette swaps of the ground ramps, no new drawing) | 0 |
| M7/M8 | Snow, desert ground, rail track, tram lines | ~20 |

**Building kits (modular on the 16 px grid, assembled on the device)**

Buildings are wall faces and roofs in three-quarter view (as in the spike). One storey is 32 px of wall face. Kits are grouped by *family*, and each place kind gets a family plus a signature piece.

| Family | Place kinds | Signature pieces |
|---|---|---|
| Civic | town hall, council chamber; M7/M8: regional assembly, parliament, executive residence | Columns, pediment, steps, flagpole, clock, dome (M8) |
| Small commercial | diner, market, news office, party office, hotel, shops (filler) | Shopfronts, awnings (recolourable), pictogram signs, market stalls |
| Community | school, clinic, community centre, union hall, business association, university | Entrance canopies, noticeboards, playground fence, cross-free clinic sign |
| Industrial | factory, mill, mine, port, tech campus | Sawtooth roofs, chimneys, loading doors, cranes, headframe, glass curtain wall (tech) |
| Residential (filler) | houses, terraces, apartment blocks, farmhouses | Porches, gardens, balconies, garages |

Per family: wall materials (3–4, recolourable), storey trims, 4–6 window styles, 2–4 doors, roof types (flat with parapet, gable, hip, sawtooth, mansard), chimneys and roof clutter. Every window and door has three states: day, lit (emissive), and boarded. Condition overlays: boarded shopfront, "to let" board, scaffolding, flood tide-line, fresh paint, protest banners (party colours). Approx. 180–240 drawn parts for P1.

Neighbourhood kind sets the material and density mix:

| Neighbourhood | Materials | Typical forms |
|---|---|---|
| Core | Stone, dressed brick | 2–4 storeys, civic and commercial, plazas |
| Inner | Brick, render | Terraces with shops at street level |
| Waterfront | Painted timber, brick warehouses | Quays, warehouses, cafés |
| Industrial | Corrugated metal, concrete | Sheds, yards, chimneys |
| Suburb | Render, timber, tile roofs | Detached houses, gardens, driveways |
| Rural | Stone, timber | Farmhouses, barns, fields |

**Props (built ahead of time; 16×16 to 32×48)**

Street lamps (2 styles, emissive head plus light pool), benches, bins, bollards, bus stop, post box, planters, trees (3 species × 2 sizes, recoloured by biome), bushes, fountain, statue, flagpoles (party colours), campaign posters and placards (party colours plus party symbol), market stall goods, crates, a boat, a crane, parked cars (2 body shapes × 2 facings, recolourable). About 40 drawn props for P1. Moving traffic is not in v1 (decision D13). M7/M8 adds scene props: podium, stage, ballot box, TV camera, crowd barriers, desks.

**Characters (composed on the device)**

| Item | P1 count | Notes |
|---|---|---|
| Frame | 16×24 (decision D1) | Figures 18–23 px tall by height band |
| Facings | down, up, side (mirrored for the other side) | [SLY-22] |
| Animations | walk 4 frames, idle 2, talk 2, sit 1 (down and up), cheer 2 | Scenes need talk, sit and cheer (DEC-009 staged scenes) |
| Body builds | 4 (slim, average, broad, heavy) × 3 heights | Independent of class and occupation |
| Age forms | adult, older (posture, gait), elderly (shorter stride, optional cane) | From `age`, already in the engine |
| Skin ramps | 10, 4 steps each | Spans the range of the Monk Skin Tone scale [MST] |
| Hair styles | ~16 (short, long, curly, coily, braids, locs, buzz, bald, bun, tied back, …) | Textures as well as lengths |
| Hair colours | 8, plus greying driven by age | |
| Facial hair | 4 | |
| Outfits | ~22: one or two per occupation in `occupations.jsonc` (15), class variants, politician suit, campaign wear | Recoloured from cloth ramps |
| Accessories | ~8: glasses, hats, lanyard, hi-vis vest, apron, hard hat, party rosette, bag | Rosette in party colour plus symbol |
| Drawn parts | ~150–200 | Each part: 3 views × the frames its layer needs |

**Portraits:** in P1 the character sheet shows a 3× crop of the sprite's head and shoulders over a background, which costs no extra art and always matches the walking sprite. Dedicated 64×64 portraits (Stardew's size [SDV-SZ]) with expressions are a P2 option (decision D11): they double the hair and head library.

**Interiors (M6 needs one or two; M7/M8 more):** council chamber and a town hall office first: floor and wall tiles, desks, chairs, benches, a dais, windows. About 30 parts. Parliament and the executive residence at M8.

**UI icons:** about 50 icons on a 16×16 grid in one colour, exported as SVG so they take the theme's `--text` or `--accent`, as the logo does (DEC-081): place kinds (23 plus home and streets), actions (~15), information services (5), pressure tracks (5), resources. Shapes must differ, because colour is never the only signal (DEC-051).

**Strategic map:** the map keeps its own data-visualisation ramps (they must stay perceptually ordered and colour-blind safe), but its biome, water and land colours come from the palette's terrain ramps, and its town markers come from the icon set.

### 3.3 Variety, inclusivity and accessibility

- **Independence rule:** body build, height, skin tone and hair texture are drawn independently of class, occupation and community. Only clothing and accessories depend on occupation, class, age and context. This keeps the generator from encoding stereotypes such as "poor means heavy" or "this job means this skin tone".
- **Community:** the base pack has one community (`communities.jsonc`). Should an expansion want a community-specific *distribution* of skin tones, it is a broad distribution only, never features or clothing, and it needs owner review (decision D14). Default: none.
- **Gender:** the engine has no gender and the name pools are not gendered. Appearance stays gender-neutral data: hair, facial hair, clothing cut and accessories are separate parameters with independent weights. If pronouns are added later, appearance weights can be correlated with them, by explicit decision.
- **Faith:** the clergy occupation needs a neutral "faith leader" outfit (no specific religion's symbols) unless the owner decides otherwise.
- **Skin ramps:** anchored on the ten Monk Skin Tone values (CC BY 4.0, from #f6ede4 to #292420) [MST]. Each ramp is hue-shifted so that dark skin keeps visible highlights and light skin keeps visible shadows. Each ramp gets its own contrast check against every ground tile at day and at night (§7), because dark sprites on dark ground at night is a known failure.
- **Crowd variety:** the clone-rate lint (§7) samples crowds of 20 and 40 from the engine's ambient generator and counts pairs that share 5 of 6 visible features (build, skin bucket, hair style, hair colour, top colour, outfit silhouette). Idle animations start at random phases, and walking speed varies by ±10%, so crowds don't move in step.
- **Town variety:** each town draws its material ramps (brick hue, roof colour family, trim colours), tree species and street-lamp style from its region, biome and defining industry (DEC-077), so two towns differ before any building is placed. Industry landmarks (chimneys, headframes, cranes, a glass campus) appear on the skyline.
- **Colour-blind safety:** party colours always come with the party's symbol shape (posters, rosettes, flags); lit windows differ in value as well as colour; warning conditions (boarded, flooded) have a shape cue (planks, tide line). The lint simulates protanopia and deuteranopia with Machado et al.'s model [MACH], applied in linear RGB [CVD-LIN], and tritanopia with a cruder approximation.

### 3.4 Lighting and time of day

The other agent's M6 renderer has keyframed grading, sun-following shadows, and windows and lamps that light at dusk. The tool supplies the data each of those needs.

| Renderer feature | What the tool ships | How the renderer uses it |
|---|---|---|
| Grading keyframes | `lighting.json`: keyframes (dawn, morning, noon, golden hour, dusk, night) with multiply colour, ambient add, shadow colour and alpha, sun azimuth and elevation, emissive strength; plus a lookup table every 15 minutes, interpolated in Oklab by the tool | Looks up two neighbouring entries and blends them; no colour maths at run time |
| Palette moods (optional, tested in the spike) | For each keyframe, a remapped palette (every palette colour mapped to a designed night or dusk colour, as classic pixel-art games do) | Re-colours the static layer once per keyframe at town load and cross-fades between two layers |
| Lit windows and doors | Per window: rectangle, emissive sprite, and a use tag (`home`, `office`, `shop`, `civic`) | Draws emissive sprites after grading with `lighter` or plain source-over, so lights stay bright at night; the use tag lets a schedule decide which windows are lit when (homes at dusk, offices dark after hours) |
| Lamps and light pools | Emissive lamp heads, plus pre-drawn light-pool sprites in 3–4 stepped alpha bands (banded, as pixel-art light usually is, not smooth gradients) | Draws pools onto the ground layer at night |
| Sun-following shadows | Per object: footprint polygon (px), height (px); per character: blob shadow sprite | Projects the footprint along the sun vector into a shadow layer at art resolution, then composites it once with the keyframe's shadow alpha (so overlaps don't double-darken). Re-bakes only when the shadow offset changes by a whole pixel |
| Golden-hour rim (optional) | Per asset: a 1-bit mask of the sun-facing edge pixels | Adds it in a warm colour at low alpha near sunrise and sunset |

Notes:
- Canvas 2D anti-aliases path fills, which would put soft, off-palette edges on shadows. The tool's runtime includes a small integer polygon rasteriser that writes shadow pixels into an `ImageData`, so shadow edges stay crisp.
- Baked shading in the art uses a fixed key light from the upper left, while the shadows move with the sun. The two can disagree in the late afternoon. Keeping baked shading soft (two steps on walls) and letting the grading carry the time of day keeps the disagreement small. The spike checks it at dusk.
- Day-part lighting is a stylisation of the "representative day" (DEC-013). The tool only supplies data; when lights come on is the renderer's and the schedule's business.

### 3.5 Determinism, versioning and integration

**Where each piece lives**

| Piece | Owner | Contents |
|---|---|---|
| Appearance registry | Engine content (`content/base/appearance.jsonc`, new; schema from Rust types, DEC-053) | Semantic options with weights: builds, heights, skin tones, hair styles and colours, facial hair, accessory preferences, outfit rules by occupation, class and age. Stable string ids such as `hair.coily_short` |
| Appearance genome | Engine state (`Character`, DEC-031) and protocol | The choices drawn for one character, as content ids, plus a 32-bit art seed |
| Art pack | `art/mandate/` (sources) built to `web/public/art/` | Parts, kits, tiles, palette, lighting, icons, and a map from every appearance id to parts |
| Compositor | `tools/pixelkit` core, imported by the game | Pure functions: genome plus view-model context plus art pack gives pixels |

**Rules**

1. The engine draws appearance from its own keyed stream (`Appearance`, keyed by seed and character id, DEC-041 rule 5), so adding appearance never shifts any other stream or changes a simulation result.
2. Old saves without appearance get one generated on load from the same keyed stream, so every load gives the same look.
3. Ambient people get their genome from the existing keyed ambient generation (seed, place, week, slot), as a pure function; promotion to Named stores it.
4. **Stored choices are never remapped.** A genome stores ids, not indices. Ids are never deleted: a retired id gets `replaced_by` in the registry.
5. Choices that are derived rather than stored (outfit from current occupation, greying from age, campaign rosette from party and campaign state, art-side micro-variation from the art seed) change when the underlying fact changes. That is intended: a mayor dresses differently from a shopkeeper.
6. Derived picks use **rendezvous (highest-random-weight) hashing**: each candidate gets a score from a fixed hash of (seed, candidate id), weighted, and the highest wins. Adding a new candidate only takes over the picks where it now scores highest; every other pick stays the same. The same method is used in the engine's weighted appearance draws for unstored ambient people.
7. Redrawing a part changes everyone who uses it. That is an art release, reviewed through golden-image diffs (§7) before merge.
8. The art pack has a version and a content hash. Saves don't record it, because looks are not simulation state.
9. The hash used by the compositor is a fixed, specified function with shared test vectors in Rust and TypeScript, so engine and UI agree where they need to.

**What the renderer consumes**

```text
web/public/art/<pack-version>/
  manifest.json        pack id and version; frames and anchors; animation tables; part map
                       (appearance id -> parts, layer order, occlusion tags); building kits and
                       facade rules; autotile sets; props with shadow and emissive data; icons index
  palette.json         named ramps -> hex; slot tables; grading keyframes and 15-minute lookup table
  tiles.png            RGBA atlas, autotile sets already expanded
  props.png            RGBA atlas: albedo, emissive and light-pool sprites
  kits.png             RGBA atlas: building parts (albedo, emissive, boarded states)
  parts.png            character parts drawn in slot colours (recoloured on the device)
  icons.svg            symbol sprite; currentColor
```

A provider interface (to be reconciled with the M6 renderer's placeholder provider, which should implement the same interface):

```ts
interface ArtProvider {
  ground(kind: TerrainKind, cornerMask: number, variant: number): FrameRef;
  building(spec: BuildingSpec): BuiltBuilding;          // albedo frames, windows[], doors[], shadow footprint
  prop(key: string, variant: number): PropSprite;      // albedo, emissive?, pool?, shadow
  character(look: LookView, anim: AnimKey, facing: Facing, frame: number): FrameRef;
  grade(minuteOfDay: number): Grade;                    // multiply, add, shadow vector and alpha, emissive and pool alpha
  icon(key: string): string;                            // SVG symbol id
}
```

`BuildingSpec` is the view model the engine already has or can add: place kind, neighbourhood kind, footprint in tiles, door side, storeys, condition flags (boarded, construction, flooded), and a `visual_seed` the engine derives from (world seed, place id). The UI never invents seeds.

**Build time or on the device**

| Asset | Where | Why |
|---|---|---|
| Tiles, autotile sets, props, icons | Build time | Fixed set; pre-expanded sets are cheap to draw |
| Buildings | On the device, once per town load, into the static layer | Footprints vary per town; pre-rendering every combination would be thousands of images; the static layer is pre-rendered anyway (DEC-080) |
| Characters | On the device, per character, cached | The combination space is far too large to pre-render; composing one character (4 facings × ~11 frames at 16×24) is a few thousand pixel operations |
| Palette moods | On the device, at town load | Remapping a 1024×1024 static layer through a palette lookup takes a few milliseconds |

**Budgets (proposed)**

| Item | Budget |
|---|---|
| Act 1 art download | ≤ 1.0 MB (PNG is already compressed), precached by the service worker |
| Each later act | ≤ 0.5 MB, loaded on entering the act |
| Atlas size | ≤ 2048×2048 each |
| Decoded art in memory | ≤ 40 MB including palette-mood layers |
| Composed-character cache | ≤ 16 MB, least-recently-used (about 150 characters) |
| Runtime code added to the app | ≤ 10 KB gzipped |
| Compose one character | ≤ 1 ms on a mid-range phone; never more than 4 per frame |

## 4. Design options

### 4.1 How the art is made

| Option | Cost | Quality ceiling | Tunability | Licensing | Verdict |
|---|---|---|---|---|---|
| A. Hand-drawn part libraries, composed by code, coloured from palette ramps (paper doll) | Medium: ~400 parts for P1, then mostly free variety | High: as good as the parts | High: weights, ramps, rules, seeds | Clean: we own the parts | **Recommended core** |
| B. Fully procedural shapes (masks, primitives) | Low drawing cost, high tuning cost | Low for people and buildings | High, but tuning cannot fix a low ceiling | Clean | Use only for placement, layout and decoration |
| C. AI drafts cleaned by hand | Low to start, high to clean at 16 px | Medium; consistency between parts is the problem | Low: no parameters, only prompts | Unprotectable raw output [USCO]; disclosure on stores [STEAM-AI] | Not for shipped pixels; mood boards at most |
| D. Licensed packs plus recolouring | Lowest | Medium; generic look | Low to medium | Kenney: CC0, fine [KEN]. LPC: share-alike and per-asset credit [LPC-GEN]. Commercial packs often forbid redistribution of source files, which a public repository does | Kenney as placeholder and reference only |
| E. 3D models rendered to pixels (Dead Cells) | High: modelling and rigging skills | Medium at 16 px; smooth, not crisp | Medium | Clean | Not for v1; possibly for shadow masks later |

**Who draws the parts** is the real question behind option A. DEC-049 notes that art is where AI agents are weakest. Agents can write small parts as text grids (as the 16×16 logo was, DEC-081), and the spike tests whether that reaches the rubric. If it doesn't, the plan changes in one place: a commissioned pixel artist draws the part library (about 400 parts for P1, not thousands of sprites), and the tool does the rest. The tool is worth building either way.

### 4.2 Tool form

- **Core library:** pure functions over indexed pixel buffers, no DOM. Palette builder (OKLCH ramps, frozen to hex), part model, compositor (layer stacking, slot recolouring, automatic outline), facade grammar, autotile expander, integer polygon rasteriser, lints, fixed hash and PRNG, manifest types and JSON schema.
- **CLI:** `build` (sources to atlases and manifest), `lint`, `sheet` (contact sheets), `golden` (render fixtures and compare), `palette` (generate and preview ramps), `grid` (text grid to PNG).
- **Studio:** a small Vite web app. Browse parts; sliders for every genome parameter; a seed browser ("next 40 townsfolk at the diner"); crowd and building contact sheets; a sample street at any minute of the day with the real grading; 1×, 2×, 3× zoom; background swatches; colour-blind simulation toggle; pin favourites and export them as fixtures; side-by-side diff of two art-pack versions. It can be published unlinked for the owner to try on a phone, like the hub spike (DEC-080).
- **Runtime:** the subset of core the game imports: manifest loader, character compositor with cache, facade builder, palette remap, shadow rasteriser.

**Source formats:** parts are indexed PNGs drawn in slot colours (Aseprite, LibreSprite or any editor), with a JSONC sidecar for anchors, layer and occlusion tags. Agents can write the same parts as text grids, which compile to identical PNGs and diff well in review. Aseprite's own files are optional masters; its file format is documented [ASE-SPEC], but Aseprite is not open source (its EULA forbids redistribution [ASE-LIC]), so CI never depends on it.

### 4.3 Language and runtime

| Option | For | Against | Verdict |
|---|---|---|---|
| TypeScript | Runs in the studio, in Node for the CLI, and in the game, so one compositor serves all three; the web app is already TypeScript; no new toolchain | Integer discipline is by convention (use typed arrays and integer maths only) | **Recommended** |
| Rust (compiled to WASM for the browser) | Shares types and hash with the engine; strong determinism habits | A second WASM module on the main thread or a worker round trip per character; the renderer is TypeScript; slower iteration on art tooling | Only the appearance registry and genome live in Rust |
| Python | Best image libraries | Cannot run in the game, so the compositor would exist twice and drift | Only for one-off analysis scripts |

### 4.4 Where it lives

- **Now:** `tools/pixelkit/` in this repository (working name), as an npm workspace package. Mandate-specific data lives in `art/mandate/` (palette, parts, kits, rules); the core contains nothing Mandate-specific. The game imports only `pixelkit/runtime` and the manifest types.
- **Boundary checks from day one:** a lint fails if core imports anything from `web/` or `art/`; the core's tests use a small sample pack (CC0, drawn for the purpose), not Mandate's art.
- **Split-out criteria:** the runtime interface has not changed for one milestone; a second use exists or the owner wants it public; the docs explain the tool without Mandate. Then the core, CLI and studio move to their own repository with the sample pack, Mandate pins a version, and `art/mandate/` stays here.

## 5. Recommended architecture

```mermaid
flowchart LR
  subgraph Engine["Engine (Rust, no art)"]
    REG["appearance.jsonc\nsemantic options + weights"]
    GEN["Appearance stream\n(keyed: seed, character id)"]
    VM["View models\nLookView, BuildingSpec,\nvisual_seed, conditions"]
    REG --> GEN --> VM
  end
  subgraph Sources["art/mandate (hand-made)"]
    PAL["palette.jsonc\nramps, slots, grading"]
    PARTS["parts: PNG or text grid\n+ JSONC sidecars"]
    KITS["building kits\n+ facade rules"]
    TPL["autotile templates,\nprops, icons"]
  end
  subgraph Tool["tools/pixelkit (TypeScript)"]
    CORE["core: compose, recolour,\noutline, facade grammar,\nautotile, rasterise, lint"]
    CLI["CLI: build, lint,\nsheet, golden"]
    STUDIO["Studio (browser):\nsliders, seeds, contact\nsheets, favourites"]
    RT["runtime (≤10 KB)"]
    CORE --- CLI
    CORE --- STUDIO
    CORE --- RT
  end
  Sources --> CLI
  CLI --> PACK["web/public/art/<version>\nmanifest.json, palette.json,\natlases, icons.svg"]
  CLI --> QA["contact sheets,\ngolden diffs, lint report"]
  QA --> OWNER["Owner review"]
  PACK --> RENDER["M6 hub renderer\n(Canvas 2D)"]
  VM --> RENDER
  RT --> RENDER
  REG -. "coverage check" .- CLI
```

Data flow for one townsperson at dusk: the engine sends a `LookView` (genome ids, age, occupation, class, campaign state). The runtime picks parts (stored ids, plus derived outfit and greying by rendezvous hashing), stacks them in manifest layer order, recolours slots from the palette, adds the exterior outline, and caches the frames. The renderer draws the cached frame, multiplies by the dusk grade, and draws a blob shadow offset by the sun vector.

## 6. Style guide draft

These are starting values for the spike to confirm or change.

**Scale and view**
- One art pixel everywhere in the scene. Nothing is drawn scaled. Zoom is whole-number (DEC-080).
- Tiles 16×16. Character frames 16×24 (decision D1). One storey is 32 px of wall face; doors 22–24 px tall.
- Three-quarter top-down: the ground is seen from above, buildings show their south face and roof only. No side walls.

**Palette**
- One master palette of at most 128 colours, designed in OKLCH and frozen to hex: about 48 for the world (terrain, materials, water, foliage, asphalt, paving), 40 for skin (10 ramps × 4), 16 for hair, the rest for cloth, signage and civic colours.
- Ramps have 3–5 steps. Hue shifts 10–20° per step: shadows toward blue or purple, highlights toward yellow, except skin shadows, which shift toward red. Saturation peaks mid-ramp [SLY-1].
- Anchors: the darkest neutral is near `--bg` (#11131a); warm lights (lit windows, lamps) come from the `--accent` gold family (#e8b44c); the logo flag (#eb6834) is a signage accent; the three party colours (#2a78d6, #eb6834, #1baf7a, plus dark-theme variants) are ramps for posters, rosettes and flags.
- Colours per character after composition: at most 14 including the outline. Props: at most 8. A single tile: at most 6.
- Never pure black or pure white in the scene.

**Light**
- Baked key light from the upper left, slightly in front. Soft: two shade steps on walls, three on people.
- Time of day comes from grading, emissives and moving shadows (§3.4), not from redrawn art.
- Night emissives: homes warm (gold), offices and shops cooler white, street lamps by town (sodium orange in older and industrial towns, white in newer cores) as a town-identity lever.

**Outlines**
- People, vehicles and props: a 1 px exterior outline added automatically after layering, in the darkest step of the touching pixel's ramp shifted cooler. Parts never contain their own outer outline, so layers never double up.
- Tiles and building faces: no outline. Edges come from value: a dark eave line under roofs, a dark base line where walls meet the ground.
- No sel-out on anything that moves, because its background is unknown [PJ].

**Shading and texture**
- No pillow shading. No banding: avoid parallel runs of equal length along an edge [PJ, PP-AA].
- Texture from clusters of at least 2 px. No random speckle.
- No dithering on people or buildings. A 2×2 ordered dither is allowed only in water highlights and the outer band of light pools.
- Anti-aliasing by hand, inside the silhouette only; none on the outer edge of anything that moves.

**People**
- Head about 7×7 px, a third of the figure [SLY-22]. Eyes 1 px, with a highlight only when the facing allows.
- Heights: 18, 20, 22 px figures (short, mid, tall); elderly 1 px shorter with a forward lean.
- Builds differ in shoulder and hip width (6–10 px torso), never in head size.
- Clothing reads by silhouette and value first, colour second: an apron, a suit, scrubs, a hi-vis vest, overalls must be tellable apart in greyscale.

**Animation**
- Walk: 4 frames (contact, passing, contact, passing), head and body 1 px up on passing frames, arms opposite legs. Frame advances by distance walked (one frame per 4 px) so feet don't slide.
- Idle: 2 frames, 1 px breathing, 600 ms each, random start phase per person.
- Talk: 2 frames of head or hand movement. Sit: 1 frame. Cheer: 2 frames, arms up.
- Side view is drawn facing right and mirrored for left; asymmetric details (a bag on one shoulder) use the mirror as is.

**Text and symbols**
- No letters or numbers in art. Signs are pictograms (a cup for the diner, a folded paper for the news office, a ballot for the town hall). Names are HTML labels (DEC-054, DEC-080).

## 7. Quality control

### 7.1 Automated lints (CLI and CI)

| Lint | Rule | Level |
|---|---|---|
| Palette conformance | Every opaque source pixel is a palette colour or a slot colour; every built pixel (before grading) is a palette colour | Error |
| Binary alpha | Alpha is 0 or 255, except light pools and shadows | Error |
| Orphan pixels | A pixel with no 4-neighbour of the same colour, outside allowed highlight colours (eyes, specular) | Warn above 2 per part |
| Banding heuristic | Three or more parallel equal-length runs along an edge | Warn |
| Outline consistency | Composed actors have a closed 1 px exterior outline; parts contain no outline colour on their outer boundary | Error |
| Frames and anchors | Every part has every view and frame its layer needs; feet on the baseline; nothing outside the frame | Error |
| Layer clashes | For every pair of parts that can co-occur (exhaustive for small sets, sampled otherwise), no pixel from a lower layer pokes through where an occlusion tag forbids it (hair through hats) | Error |
| Silhouette readability | Figure silhouettes per build differ (overlap below a threshold); the figure's bounding box matches its height band | Error |
| Contrast against backgrounds | Mean Oklab lightness difference between each skin and outfit combination and each ground tile, at day and at night grading, above a threshold | Error below the floor, warn near it |
| Colour-blind safety | Listed cue pairs (party colours, lit and unlit windows, boarded and normal) stay apart under protan and deutan simulation [MACH, CVD-LIN]; each has a declared shape cue | Error |
| Coverage | Every appearance id has parts; every `PlaceKind` and `NeighborhoodKind` reaches a kit or a fallback; every terrain pair has a full autotile set | Error |
| Seams | Autotile edges match their neighbours | Error |
| Clone rate | In 1,000 sampled crowds of 20, the share of pairs that match on 5 of 6 visible features stays below a threshold | Error above, report always |
| Budgets | Atlas dimensions, file sizes, decoded memory, colours per sprite | Error |
| Determinism | Building twice gives identical decoded pixels and an identical manifest | Error |
| Boundary | Core imports nothing from `web/` or `art/` | Error |

### 7.2 Golden images

- A fixture set: 40 characters (pinned genomes covering every build, skin ramp and hair style), 12 buildings (one per family and condition), each autotile set, and one sample street at 06:00, 12:00, 18:30 and 22:00.
- Tests compare decoded pixels exactly. A failure writes a diff contact sheet (old, new, changed pixels in magenta) to the CI artifacts.
- Updating goldens needs an explicit flag and a line in the PR saying what changed and why. This is how "an art update must not silently change a known character" is enforced.

### 7.3 Human review

- The tool renders contact sheets for each review: a crowd of 40 per place kind, every building family per neighbourhood, every tile set, and the sample street at four times of day.
- They are published as an unlinked page (like the hub spike) so the owner can look on a phone at real size.
- The owner scores each sheet against the rubric (§8) and leaves comments; comments become tasks. Favourites pinned in the studio become fixtures.

## 8. Phased plan

Effort is in agent tasks (one brief per `WORKFLOW.md`, roughly one focused session each). Each phase needs owner approval to start (DEC-003, DEC-050 practice).

### P0 — Validation spike (about 6 tasks)

**Only purpose:** prove the look before investing. Code may be thrown away; the palette and style guide survive.

| Task | Output |
|---|---|
| P0-1 Palette and style numbers | Master palette (world ramps, 6 skin ramps, hair, cloth, civic colours) built in OKLCH; ramp preview sheet; style guide values confirmed or changed |
| P0-2 Core compositor and sheet CLI | Slot recolouring, layer stacking, automatic outline, contact-sheet PNG output, golden test harness |
| P0-3 Tiles and props | Grass, pavement, road, crossing with dual-grid sets from templates; lamp (with emissive and pool), bench, tree, bin, planter, parked car |
| P0-4 One building kit | Inner-neighbourhood terrace and shop family: 2 wall materials, 3 window styles (day, lit, boarded), 2 doors, 1 shopfront with awning, gable and flat roofs; facade grammar for 3–6-tile-wide footprints |
| P0-5 Twenty townsfolk | 3 builds × 2 heights, 6 skin ramps, 6 hair styles, 4 outfits (shopkeeper apron, nurse, suit, casual), walk 4 and idle 2 in 4 facings |
| P0-6 Sample street and review page | A 24×16-tile street rendered by a minimal Canvas preview at noon and dusk (grading, lit windows, lamps, sun shadows); both lighting variants (multiply only, and palette moods); contact sheets; rubric form |

**Rubric (owner, 1–5 unless stated):**
1. Reads as deliberate pixel art, not programmer art. Pass: 4 or more.
2. Cohesion: one palette, one light, nothing looks pasted in. Pass: 4 or more.
3. People readable at default phone zoom: the owner can name 4 of 5 outfits without labels, and facing is always clear.
4. Crowd variety: in the 20-person sheet, at most one pair "look like the same person".
5. Dusk is more attractive than noon, not muddier; people stay readable at night.
6. Preferred over the Kenney placeholder and the code-drawn placeholder in a side-by-side.
7. All lints pass; the street holds 30 fps on the owner's phone.

**Exit:** a pass on 1, 2 and 5 at least, and the owner's go. The spike also answers: is 16×24 enough room (otherwise 16×32)? Are agent-drawn parts good enough, or is a commissioned artist needed? Multiply-only grading or palette moods?

### P1 — MVP for M6 (about 14 tasks)

- Full palette (10 skin ramps) and final style guide in `docs/design/`.
- Kits for all five families with the P1 place kinds, all six neighbourhood material sets, condition overlays, filler housing.
- All terrain pairs as autotile sets; ~40 props; council chamber and town hall interiors.
- Character library (§3.2) with walk, idle, talk, sit and cheer; sprite-bust portraits.
- `appearance.jsonc` and the genome in the engine and protocol (save format bump with a load-time fill for old saves); coverage check in `content validate`.
- Runtime integrated behind the M6 renderer's provider interface, replacing the placeholder provider; lighting data wired to the day cycle.
- ~50 UI icons as SVG.
- Studio app; all lints and goldens in CI; budgets checked.
- **Exit:** the M6 game-quality gate (owner plus testers) judges the town's art, the clone-rate and contrast lints pass, budgets hold on a real phone.

### P2 — Expansion for M7–M8 (about 12 tasks)

Regional and national capital kits (denser cores, grander civic buildings, regional assembly, parliament, executive residence); more interiors; scene props (podium, stage, ballot boxes, cameras); biome kits (dry, boreal, snow); weather overlays (rain, snow); optional 64×64 portraits with expressions (decision D11); optional moving traffic (decision D13).
**Exit:** every hub in Acts 2–4 renders with no placeholder art.

### P3 — Spin-out (about 4 tasks, when the criteria in §4.4 hold)

Generic documentation, a CC0 sample pack, a public repository, a published package; Mandate pins a version.

## 9. Decisions for the owner

| ID | Decision | Options | Recommendation |
|---|---|---|---|
| D1 | Character frame size | 16×24 (as the spike; more town visible on a phone; less to draw) or 16×32 (Stardew; more room for detail and height differences; storeys must grow to about 40 px) | **16×24**, confirmed or overturned by the spike's readability item |
| D2 | Outline policy | Black outlines; automatic dark-tone exterior outline on things that move and props, none on the world; no outlines | **Automatic dark-tone outline on people, vehicles and props; none on tiles and buildings** |
| D3 | Palette | Our own, designed from the brand tokens; an existing Lospec palette | **Our own**, at most 128 colours, built in OKLCH, anchored on the brand colours |
| D4 | AI image generation | None; reference and mood boards only; drafts cleaned by hand | **None in shipped art.** Mood boards only if the owner wants them |
| D5 | Who draws the parts | Agents (text grids and PNGs); the owner; a commissioned pixel artist | **Agents for the spike**; commission an artist for P1 if the spike's rubric item 1 or 2 fails |
| D6 | Own repository | Now; after M6 when the criteria in §4.4 hold; never | **Later (P3)**, with the boundary enforced from day one |
| D7 | Tool licence | MIT, Apache-2.0, MIT OR Apache-2.0, or closed | **MIT OR Apache-2.0** (matches the dependency policy and keeps the spin-out simple) |
| D8 | Licence of Mandate's own art | All rights reserved; CC BY-NC; CC BY | **All rights reserved** for Mandate's pack; a separate CC0 sample pack ships with the tool |
| D9 | Language | TypeScript; Rust; Python | **TypeScript**; the appearance registry and genome in Rust content |
| D10 | Where composition happens | All at build time; characters and buildings on the device, the rest at build time | **Characters and buildings on the device**, tiles, props and icons at build time |
| D11 | Portraits in v1 | Sprite-bust crops (no extra art); dedicated 64×64 portraits with expressions | **Sprite-bust crops in M6**; decide on 64×64 after seeing the town |
| D12 | Night look | Multiply grading only; palette moods for static layers plus multiply for sprites | **Decide from the spike**, which renders both |
| D13 | Moving traffic in v1 | None; parked cars only; simple traffic | **Parked cars only** in M6 |
| D14 | Community-specific appearance | Never; broad skin-tone distributions only, by owner review; full | **None by default**; broad distributions only, with owner review, if an expansion asks |
| D15 | Timing | Post-v1 (current roadmap note); spike now, MVP with M6 art integration | **Spike before M6 art integration**, MVP as part of M6, because M6 already needs final-quality art for its game-quality gate and the "source of final art" decision is due then (ROADMAP open items) |
| D16 | Name | `pixelkit` (working name) or another | Owner's choice before spin-out |

## 10. Risks

| Risk | Likelihood | Effect | Mitigation |
|---|---|---|---|
| Parts drawn by agents look amateur, and the tool multiplies the amateurishness | High | High | The spike tests exactly this; fallback is a commissioned artist for ~400 parts |
| Layering bugs (hair through hats, sleeves over bags), as RimWorld players report [RW-LAYER] | High | Medium | Explicit layer order and occlusion tags; exhaustive or sampled clash lint |
| Baked light direction disagrees with moving shadows | Medium | Low–medium | Soft baked shading; the spike reviews dusk |
| Canvas anti-aliasing softens shadow edges | Certain without care | Low | Integer rasteriser into `ImageData` |
| Memory on low-end phones (palette-mood layers, character cache) | Medium | Medium | Budgets in §3.5; drop palette moods on low memory; least-recently-used cache |
| Stereotyped or narrow depiction of people | Medium | High | Independence rule; neutral faith outfit; skin-ramp contrast checks; owner and tester review of crowd sheets |
| An art update changes known characters unnoticed | Medium | Medium | Stored ids; rendezvous hashing for derived picks; goldens with diff sheets |
| Hash or RNG differs between Rust and TypeScript | Low | Medium | One specified hash with shared test vectors |
| Licence contamination (share-alike art, commercial packs in a public repository) | Low | High | No third-party art except CC0; licence notes in `ASSETS.md`; npm licence check |
| The tool grows into a product before the game's needs are met | Medium | Medium | Spin-out only in P3; P0–P2 scope is set by Mandate's inventory |
| Fractional zoom during pinch gestures shows uneven pixels | Medium | Low | DEC-080 whole-number zoom steps; the spike checks gestures |

## Sources

- [PJ] PixelJoint forum, "Pixel Art Tutorial" (classic thread, things to avoid and core advice): https://pixeljoint.com/forum/forum_posts.asp?TID=11299
- [PP-AA] Pixel Parmesan, "Anti-Aliasing Fundamentals for Pixel Artists": https://pixelparmesan.com/anti-aliasing-fundamentals-for-pixel-artists
- [PP-COL] Pixel Parmesan, "Color Theory for Pixel Artists: It's All Relative": https://pixelparmesan.com/color-theory-for-pixel-artists-its-all-relative/
- [SLY-1] Raymond Schlitter (Slynyrd), "Pixelblog 1 – Color Palettes": https://www.slynyrd.com/blog/2018/1/10/pixelblog-1-color-palettes
- [SLY-22] Slynyrd, "Pixelblog 22 – Top Down Character Sprites": https://www.slynyrd.com/blog/2019/10/21/pixelblog-22-top-down-character-sprites
- [SLY-55] Slynyrd, "Pixelblog 55 – Top Down Character Animation": https://slynyrd.com/blog/2025/3/24/pixelblog-55-top-down-character-animation
- [MDN] MDN, "Crisp pixel art look with image-rendering": https://developer.mozilla.org/en-US/docs/Games/Techniques/Crisp_pixel_art_look
- [JJ] Gabriel Sassone, "Pixel art filtering": https://jorenjoestar.github.io/post/pixel_art_filtering
- [LPC-GEN] Universal LPC Spritesheet Character Generator (code and per-asset licences): https://github.com/LiberatedPixelCup/Universal-LPC-Spritesheet-Character-Generator
- [LPC-SG] LPC Style Guide: https://lpc.opengameart.org/static/LPC-Style-Guide/build/styleguide.html
- [KEN] Kenney, "Tiny Town" (16×16, CC0): https://kenney.nl/assets/tiny-town
- [KEN-RC] Kenney, "Roguelike Character Pack" (OpenGameArt mirror): https://opengameart.org/content/roguelike-character-pack
- [PSG] zfedoran, pixel-sprite-generator (after Dave Bollinger): https://github.com/zfedoran/pixel-sprite-generator
- [SDV-FS] Stardew Valley Wiki, "Modding:Farmer sprite": https://stardewvalleywiki.com/Modding:Farmer_sprite
- [SDV-SZ] Stardew Valley forums, "Sprite size (First time modding SV)": https://forums.stardewvalley.net/threads/sprite-size-first-time-modding-sv.7128/latest
- [SDV-PCG] PC Gamer, Stardew Valley interview with Eric Barone: https://pcgamer.com/stardew-valley-interview
- [RW] RimWorld Wiki, "Modding Tutorials/Textures": https://rimworldwiki.com/wiki/Modding_Tutorials/Textures
- [RW-LAYER] Nexus Mods, RimWorld back-hair layer mod (layering notes): https://www.nexusmods.com/rimworld/mods/339
- [CoQ] Caves of Qud Wiki, "Modding:Tiles": https://wiki.cavesofqud.com/wiki/Modding:Tiles
- [TS] Game Developer, "How Townscaper works: a story four games in the making": https://www.gamedeveloper.com/game-platforms/how-townscaper-works-a-story-four-games-in-the-making
- [EW] Game Developer, "Eastward's creators share insights on making pixel art adventures": https://www.gamedeveloper.com/art/eastward-s-creators-share-insights-on-making-pixel-art-adventures
- [DC] Game Developer, "Art Design Deep Dive: Using a 3D pipeline for 2D animation in Dead Cells": https://www.gamedeveloper.com/production/art-design-deep-dive-using-a-3d-pipeline-for-2d-animation-in-i-dead-cells-i-
- [SL] Sprite Lamp (Steam): https://store.steampowered.com/app/316830/Sprite_Lamp/
- [CGA] Müller, Wonka, Haegler, Ulmer, Van Gool, "Procedural Modeling of Buildings", SIGGRAPH 2006: https://course.ccs.neu.edu/cs5150f13/readings/muller_buildings.pdf
- [WFC] Maxim Gumin, WaveFunctionCollapse (README): https://github.com/mxgmn/WaveFunctionCollapse
- [WFC-LIC] ecosyste.ms listing of the repository (licence shown as "other"): https://awesome.ecosyste.ms/projects/github.com%2Fmxgmn%2FWaveFunctionCollapse
- [BTB-AT] Boris the Brave, "Beyond Basic Autotiling": https://www.boristhebrave.com/2021/09/12/beyond-basic-autotiling/
- [BTB-RU] Boris the Brave, "Tileset Roundup": https://www.boristhebrave.com/2013/07/14/tileset-roundup
- [BTB-QT] Boris the Brave, "Quarter-Tile Autotiling": https://www.boristhebrave.com/2023/05/31/quarter-tile-autotiling/
- [DG] Dual-grid tilemap system (credits jess::codes and Oskar Stålberg): https://github.com/jamesplease/dual-grid-tilemap-system-godot-gdscript
- [OK] Björn Ottosson, "A perceptual color space for image processing" (Oklab): https://bottosson.github.io/posts/oklab/
- [MACH] Machado, Oliveira, Fernandes, "A Physiologically-based Model for Simulation of Color Vision Deficiency", IEEE TVCG 2009: https://www.inf.ufrgs.br/~oliveira/gmmachado.html
- [CVD-LIN] colorspace R package, `simulate_cvd` (notes the model assumes linear RGB): https://www.rdocumentation.org/packages/colorspace/versions/2.1-0/topics/simulate_cvd
- [MST] Google, "Improving skin tone representation" (Monk Skin Tone scale): https://blog.google/products/search/monk-skin-tone-scale/ ; values and licence: https://en.wikipedia.org/wiki/Monk_Skin_Tone_Scale
- [AI-CLEAN] "How to tame your AI pixel art" (DEV Community): https://dev.to/jenissimo/how-to-tame-your-ai-pixel-art-3pk5 ; Sprite Fusion, Pixel Snapper: https://www.spritefusion.com/pixel-snapper
- [RD] Astropulse, Retro Diffusion: https://astropulse.itch.io/retrodiffusion
- [USCO] Jones Day, "Copyrightability of AI Outputs: U.S. Copyright Office Analyzes Human Authorship Requirement" (2025): https://www.jonesday.com/en/insights/2025/02/copyrightability-of-ai-outputs-us-copyright-office-analyzes-human-authorship-requirement ; the report: https://www.copyright.gov/ai/
- [STEAM-AI] Video Games Chronicle, "Valve has 'significantly' rewritten Steam's rules for how developers must disclose AI use": https://www.videogameschronicle.com/news/valve-has-significantly-rewritten-steams-rules-for-how-developers-much-disclose-ai-use
- [ASE-SPEC] Aseprite file format specification: https://github.com/aseprite/aseprite/blob/main/docs/ase-file-specs.md
- [ASE-LIC] Wikipedia, "Aseprite" (EULA since v1.1.8; LibreSprite fork): https://en.wikipedia.org/wiki/Aseprite
