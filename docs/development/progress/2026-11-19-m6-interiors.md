# M6 Interiors (DEC-088, DEC-098 §4)

**Date:** 2026-11-19
**Status:**

**Done**, 2026-11-19: the council chamber, the town hall and the diner are entered from the street (a tap or Enter at the door, a short fade; the Places tab and the town map keep the panel): `web/src/interior/` (room cache by place, recipe version and seed; canvas grade shared with the art tool; walking, hotspots, talk via `openTalk`, the dialogue hook `stage.ts`); recipes are content (`content/base/interiors.jsonc`, `InteriorRecipe` in `mandate-content` with the art pack's tile rules ported to `content validate`, `places.jsonc` `interior`); `PlacePanelView.interior` (the recipe) and `here_as`; art: the town hall's lower half furnished (version 2), floors that read as floors, `PACK_VERSION` mandate-p0.9; `e2e/interiors.spec.ts`
