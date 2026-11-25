# Town map layers: wards, people and worries

**Date:** 2026-11-25
**Status:** done (the registry, the engine's view, three layers, the legend, the card and the list; the town card's worries follow the ledger); not done is listed at the end

Checklist notes 11, 18 (the wards layer) and 22 (first follow-up). Decisions: DEC-086, DEC-089 (the pattern), DEC-091, DEC-098, DEC-051.

## What a player gets

The town map has a picker: Places (the plain map), then the registry's layers. The choice is remembered per device, as the country map's is.

- **Wards** (the constitution's word for the town's districts): each district outlined, named as the "where you are" chip names it, the player's own in gold with "Your seat" (or "You live here" when out of office), and each seat's holder as a portrait in a ring of their party's colour.
- **People**: everyone in town the player has at least heard of, at their workplace or where the player last or usually saw them. People at one place share a mark with a count; the mark opens their list. Filters: everyone, the legislature's members (named by the constitution), each party, people the player knows.
- **Worries**: each neighbourhood filled and labelled with its top worry, or with how high a chosen worry stands there (top, in the top three, lesser), from what the ledger holds of that neighbourhood. A neighbourhood the player knows nothing of is striped and says "Not known yet", with the lead toward learning it.

A portrait opens the person's card: name as far as known, role, district and party in words, where the player knows to find them, "Open their card" (the character sheet) and "Go to them" (the engine's talk route, walked in the street). The legend is always shown. "List" gives the same data as text for every layer.

## Built

- **Content** (`content/base/town_map_layers.jsonc`, `mandate_content::town_map_layers`): `id`, `label_key`, `unit_key`, `note_key`, `kind` (boundaries, markers, shaded), `source` (a tagged object: `districts`, `people`, `worries { top }`), `scale` (party, sequential), `filters`, `legend.categories`. A source fixes its kind and scale and names its parameter. Validation refuses an unknown source, a source with a setting it does not have, a kind or scale that is not the source's, and misplaced filters or categories. The schema is regenerated; the doctor reads the keys from the bundle as it does the country layers'.
- **Engine** (`mandate_protocol::town_layers`, query `town_map_layers`, about 18 KB): `TownMapLayersView` for the player's home town (`player::home`, never a field of theirs). Districts are those of the home jurisdiction's legislature (the body of its `ordinance` procedure) from `Jurisdiction::constituencies`; a district's label is `ui-post-seat` with its number and tier. People come from `routines::townsfolk`, at `heard` or above; a place is their workplace (`routines::workplace`, new and read-only), else the ledger's sightings. Worries are `concerns::ranked` as felt, cut by `gate_concerns` to what the ledger holds of each neighbourhood; a worry the cut cannot place has no class. Legends, filters and the worries' choices are the engine's. The query writes nothing (tested on the ledger and the state hash).
- **The town card** (`municipality_view.top_concerns`): now cut by the ledger as the notebook is (none, one at `heard`, three from `known`).
- **Web** (`web/src/components/townmap/`): `layerModel.ts` (pure: outlines along cell edges, anchors, the fill a tile to a pixel, clustering, colours), `TownLayerOverlay.tsx` (the fill, the outlines and the marks, following the camera by `TownMapView`'s new `onFrame`), `TownLayerPanel.tsx` (legend, readout, person card, a mark's list), `TownLayerList.tsx`, `townLayers.css`; `TownMap.tsx` holds the picker and the state.
- **Text**: `content/base/locale/en/map.ftl` (36 lines).

## How a later layer plugs in

A bill's impact by neighbourhood is a new `TownLayerSource` variant (`bill_impact`, kind shaded, parameter: the bill), one function in `town_layers.rs` returning a `TownShadeOptionView` per bill before the body, its Fluent lines and one registry entry. The registry's shape, the views and the client do not change: the client draws any shaded layer from its options, bins and cells. Asks, works and upkeep are markers sources in the same way. `docs/development/EXTENDING.md`, "Adding a town map layer".

## Tests

- Rust: the registry (4, and 2 unit), the views (5 in `mandate-wasm/tests/town_map_layers.rs`, 1 unit): every district once and every neighbourhood in one; no one below `heard`; placed only where known; unknown is unknown; nothing written. `worries.rs` and `knowledge.rs` now hold the town card to the ledger.
- Web unit: 16 (geometry, fills, clustering, lists, legend, card).
- Playwright (`e2e/town-map-layers.spec.ts`): the three layers, a member's portrait to their card and their sheet, the list, Go to them, and fit at 320, 360 and 390 px with wide fonts and at 1280 px; no raw key or id anywhere in the overlay.

## Not done

- The map's town card says nothing where a town's worries are unknown (the line is left out); a "not known yet" line needs `MunicipalityPanel`.
- Later layers (a bill's impact, asks, works, upkeep): the shape is ready, none is built.
- The layers are the home town's only (the map is). Regions and the nation come with M7's maps.
- A neighbourhood's label can hide behind another at the widest zoom on a narrow phone; zooming in or the list shows it.
- Note 22's second follow-up (what residents ask for follows what they feel) is untouched.
