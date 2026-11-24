# Notes 18: notebook wording, phone bar, ward chip, walk-in people, update prompt

**Date:** 2026-11-24
**Status:** done (play note 18 and the "people pop into a room" bullet of note 19).

- Seen lines and the usual-place lead: an unnamed place passes its in-sentence kind as a key (`place_key`, `to_place_key`, as talk does), so "Last seen at the union hall, in the morning."; a sighting at home reads "home"; the lead has `lead-usually-at-kind` for unnamed places. Test over generated worlds in `crates/mandate-wasm/tests/routines.rs`.
- Eye emoji removed. `ui-notebook-lead-gives` is a select on the level: "Would raise this to ●●○ Known", "You would hear of this".
- Bottom bar: badges wrap and shrink first, the date shortens to day and short month on a phone; `web/e2e/gamebar.spec.ts` at 360 and 390.
- Where-you-are chip adds the district ("Old Green · Ward 2 · Home") from the town view's `ward_number` through `districtLabel`.
- People walk in and out: rooms (`web/src/interior/crowd.ts`) and the street (`TownScene.setStanding` with `arrive`, newcomers from the nearest doorway or edge). Reduced motion and sheets: they are simply there; loops sleep when nobody moves.
- Update prompt: the test `web/e2e/update-prompt.spec.ts` serves build A, deploys B and checks the bar, the kept build and Reload. Findings: a page that installed the first worker itself never reloaded on Reload (the plugin reloads only when `isUpdate`), so the page kept the old code after the click; the bar (z-index 5) sat below the pocket and decision sheets (6), the bottom bar (7) and the map (8). Fixed: reload on `controllerchange` in `UpdatePrompt`, the bar at z-index 9 above the bottom bar. The build identifier shows on the title screen and in "Report a problem".
- Title screen (item 8): saved runs first with name, post and home (kept in the run index at each save; older runs read once from the save through the new read-only `readSaveSummary`), the new career below; Continue on the latest run is the primary action.
