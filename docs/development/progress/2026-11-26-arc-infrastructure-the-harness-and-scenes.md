# Arc infrastructure: the test harness, and played scenes that matter

**Date:** 2026-11-26
**Status:** done for the six items, on the pilot arc's branch (885e035). Save v33 (B2 takes 33 on main: renumber on merge; the step `worry_roots` names no version). One DEC proposal (item 5), not decided here.

Design: DEC-109 (places for arcs, §4.2), DEC-112 (scenes), DEC-110 (the plant review), DEC-098, DEC-041. The pilot's ledger (`2026-11-25-pilot-arc-the-zoning-fight-and.md`) and X3's findings.

## 1. The shared arc-test harness (`crates/mandate-testkit`)

A dev-only crate (no engine crate depends on it; no new external dependency): `pack`, `keep(pack, arcs)`, `only(&[arcs])` (the base pack with the arcs named and no other), `no_arcs`, `world(c, seed)`, `raised(e, arc)` (idle until live), `play(e, arc, policy, turns)` and `meet(e, arc, scene)`, `until_stage`, `exits(c, seed, arc, prepare, policies, turns)`, `plant_flood`, `shock_plant`, and the readers (`home`, `town`, `me`, `seat`, `run_of`, `exit_of`, `summary_of`, `occupation`). The policies: `PASSIVE`, `AWAY` (every lever, the chair empty), `COLD` (no lever, the best line shown), `BEST` (every lever, the best line shown), or `Scene::Lines(kinds)`. The levers are X3's stand-in (`standin::play_turn_styled`); the best line shown is the cautious stand-in's `standin::line`, read from the player's view; a player that prepares begins the scene on the last turn that can, as X3's stand-ins do.

Moved onto it: the flood (`worldgen/tests/arcs.rs`), the plant, the favour and the rival (`arcs_x4.rs`), the zoning fight (`arcs_zoning.rs`: 403 lines to 263, its 140 lines of helpers gone), the budget round (`budget_round.rs`, the round alone now), the debate's tests (`save/tests/exchange.rs`, `standin_scenes.rs`: the rival alone), the flood's views (`wasm/tests/pocket_views.rs`: `keep(embedded, flood)`), the registry-order assertion (`content/tests/arcs.rs`: whatever follows the flood's items is an arc file's, so a new arc file changes nothing), the debate's skip test (no longer skips past another arc's scene), and the lab's planted-flood test (`lab/arcs_games.rs`: the whole pack again, 8 of 8, now that a crisis outranks a matter).

What arcs do to each other is one file: `worldgen/tests/director_arcs.rs` (places by rank; a crisis beside the budget round leads the chapter, moved from `budget_round.rs`; one scene and one climax a chapter). The pacing bar in `narrative.rs` and the wire tests (`scene_views.rs`, which look for their own scene) keep the whole pack: they are about the whole game.

## 2. Exits read the scene played

- **Mechanism:** a condition `arc_scene` (`beat`, `outcomes`, `min_pm`, `max_pm`) reads the run's finished scenes (`ArcRun::played`, the `ArcExchangeEnded` outcome and result): an edge where the ending is decided can read how the room went. Validation refuses one that names no scene of its arc. A lint, `arc-scene-unread` (in `content validate`'s lints), names a scene whose outcome no edge reads and no outcome acts on the matter (repairs, a plant's margin, people leaving, a budget; a room of the race's voters moves the race): today only the zoning meeting.
- **Content:** the flood's hall carried (won or split) with a fifth repaired goes on to rebuilding; lost or an empty chair, hollowed after 60 days (120 before); work started early (paid, a crew) no longer skips the hall (only a law does), so a prepared player faces it in a cooler room. The plant's hall carried puts relief on the plant's margin (+0.06, 12 months); an empty chair, the members down tools (−0.04, 3 months); lost, nothing. The numbers are the tuning pass's (`TUNING_QUEUE.md`).

X3's measurement, `arcs grade --arc <arc> --seeds 16`, the turn section (exits differ, the engaged stand-in played against the same one absent; and the passive one turned up cold against absent):

| | best shown vs away, before (885e035) | after | cold vs away, before | after | reached the scene (engaged), before / after |
|---|---|---|---|---|---|
| the rival (debate) | 4 of 16 | 4 of 16 | 0 of 16 | 0 of 16 | 16 / 16 |
| the plant (hall) | 2 of 14 | 3 of 14 | 0 of 14 | 0 of 14 | 6 / 6 |
| the flood (meeting) | 0 of 14 | 1 of 16 | 0 of 14 | 5 of 16 | 0 / 5 |
| the zoning fight (meeting) | 0 of 11 | 0 of 11 | 0 of 11 | 0 of 11 | 7 / 7 |

The debate reaches the race through the room's approval (`exchange.approve` 0.08), unchanged: a number. The plant's hall is lost by every stand-in that holds it (6 of 6, engaged and cold), so its won relief is never seen: a balance row. The flood's engaged stand-in mostly passes a law first (defended 11 of 16), which answers the matter without a hall.

## 3. No scene window shorter than a turn

The step holds the clock of a stage with a scene to at least one turn's length from its entry (`routines::turn_days_in(phase)`, `tuning.turn_days`; no 14 in code), so its last day is never before the player's next turn. `content validate` warns `exchange-window-short` when a scene's clock range starts under the longest turn its tiers are played in. The flood's and the plant's meetings had `[7, 14]`; they are `[14, 21]` now, as the zoning meeting and the debate. Test: the flood's meeting on a one-day clock is held to a turn and offered (`arcs_scenes.rs`). The grader's `engaged_never_offered`: the plant's hall 1 before (seed 5), 0 after.

## 4. Crises outrank matters for places

`ArcDef::rank` (`matter`, the default, or `crisis`; schema regenerated): a run holds a place against a template only if it ranks as high, so a crisis is never kept out by matters raised before it; at most `arcs_active` runs of a rank or above. The lead's tier reads the same field (it read the theme `crisis` before; the flood and the plant are the two crises, so no lead changed). A standing arc with a rank is refused. Test: two matters (the zoning fight and a favour) hold both places, a flood comes and is raised; ranked a matter, it waits. The graded flood is found in 16 of 16 worlds (14 of 16 before, at 885e035; 13 of 16 in the pilot's own run).

## 5. Why plants rarely build (the audition's 2 of 31)

16 small worlds, every town, the plants on short time on the first day of play, 20 idle turns (39 weeks):

| | |
|---|---|
| plants on short time at the start | 258 |
| after 39 weeks: on short time / back to full time / under notice | 192 (74%) / 66 / 0 |
| a month under the notice line (0.78) in those weeks | 55 of 258 (203 never) |
| reached the 9 months under it a notice needs | 1 |
| on short time a year or more at the start | 143 (55%); the longest 19 years |
| in industries that never close (ports, farms) | 10 |

**Cause:** short time has no end for demand between the notice line (0.78) and the full-time line (0.94). A plant there neither gets a notice nor its hours back, for years; the plant arc's hidden first stage waits for one of the two (or 540 days), so it does not move inside the audition's horizon. It is a mechanism of the economy's review (DEC-110: "the review belongs to the economy; arcs read it"), not of the arc. **Not changed here:** an end to short time (after a set time the review decides: a notice under the short-time line where the industry can close, else full hours) changes closures in every town and the industries' bands, which is an accepted decision's owner's. **DEC proposal:** short time ends in a review after a set number of months. A row is in `TUNING_QUEUE.md`.

## 6. The worry as a root

A raise that reads a standing worry (`concern`) and no event now records `Pressure::Worry { concern }` (the concern's index in the pack) beside the arcs' rule on `ArcRaised`; the Why panel says "People in the town worried most about the rent." (`why-pressure-worry-named`, the concern's talk key; `CauseView::Pressure.concern_key`; `whyModel` passes it). Test: the zoning fight's raise names housing (`arcs_zoning.rs`). Save v33 (a cause can name it; nothing older changes).

## What adding an arc costs now

With the harness: an arc's tests are its own file, about 150–250 lines of tests and no helpers (the zoning fight's are 263 with comments); `only(&[arc])` keeps every other arc's tests out of its way and it out of theirs, so the pilot's "an hour or more for the other arcs' tests it meets" (twelve tests in seven files) goes to none, bar the whole-game pins (`repin.sh`) and `director_arcs.rs` when the arc changes how arcs meet. `play(.., Policy::BEST)` and `exits(.., [AWAY, BEST])` replace the hand-written scene probe, and `arcs grade` now measures the turn the arc's exits read. Estimate for the next arc on existing systems: about an hour for the arc and its tests, 20–30 minutes for checks and pins, 250–300 lines of JSONC, about 100 of Fluent, 150–250 of tests, 0–50 of engine.

## Not done, and open

- The zoning meeting's exits read no outcome (`arc-scene-unread`): the pilot's design; a content change for its owner.
- The plant's hall is never won and the debate barely moves a race: balance rows.
- Item 5's mechanism waits on the DEC proposal.
