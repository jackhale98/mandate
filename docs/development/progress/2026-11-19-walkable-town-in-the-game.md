# Walkable town in the game (DEC-086, DEC-088, DEC-090)

**Date:** 2026-11-19
**Status:**

**Done**, 2026-11-19: the town screen draws layout v2 from the engine's town views (planned in a town worker of its own, cached in memory and IndexedDB, loader meanwhile); layout v1 retired (generator, fixtures, pins, per-run version and recipe snapshot storage); "Walk the streets" from the map opens the walkable street (hub renderer, pixel-art pack built in the worker): tap a place, walk there, its panel opens (the List's command); townsfolk thin out at night; the light follows the turn's slots and eases forward; ending a turn passes the night (date card, skippable, quick with reduced motion; election night waits for it). No interiors or talking yet.
