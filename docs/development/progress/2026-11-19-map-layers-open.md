# Map layers, open (DEC-089)

**Date:** 2026-11-19
**Status:**

**Done**, 2026-11-19: the country map's picker is a registry in content (`content/base/map_layers.jsonc`, `mandate_content::map_layers`: key, Fluent label and units, a `source` naming a served town figure (an unknown one fails to load), sequential/diverging/categorical scale, midpoint, format, legend classes and breaks); seven open layers: population, density, unemployment (diverging around the national rate), median income, rent, main industry and main resource (3 named categories + Other). The engine serves `MapLayersView` (query `map_layers`: each town's public figures, the player's town rounded as its panel is; per layer the class breaks, legend bins with counts and every town's class), about 28 KB for 84 towns beside the 0.5 MB raster, fetched again each turn. Web: dropdown, validated light/dark palettes (`web/src/map/layerPalette.ts`), legend with units and breaks, tap/hover readout with Details, a sortable List of towns, the layer remembered per device; `e2e/map-layers.spec.ts`. Gated layers (unknown per town, an as-of date) are M7
