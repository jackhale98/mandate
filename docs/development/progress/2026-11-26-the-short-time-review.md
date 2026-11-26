# The short-time review (DEC-110 amended)

**Date:** 2026-11-26
**Status:** done. Save v34 (an event kind added; renumber on merge). The three numbers are content and the tuning pass's (`TUNING_QUEUE.md`).

Design: DEC-110 (amended 2026-11-26, owner: "short time ends in a review"), `docs/design/INDUSTRIES.md` §3; the arcs-infrastructure entry's item 5 (`2026-11-26-arc-infrastructure-the-harness-and-scenes.md`), DEC-109 answer 2, DEC-041, DEC-054.

## The mechanism

- **The review** (`mandate-sim/src/economy/plants.rs`, `process`): a plant on short time `review_months` (12) is reviewed each month until it is decided. Its margin-adjusted demand `m` under `review_line` (0.88), in an industry that closes: the notice, by the existing path (`notice`: the one keyed draw for the date, the town's notice date, a reprieve possible). The town's notice gap holds: while the town had a notice in the last 12 months (outside a recession) the review waits on short time, as a second plant did before. Otherwise: full time again; with `m` still under `full_line` (0.94) the plant sheds `review_cut` (0.10) of its capacity and that share of its industry's jobs here (capacity = unnamed firms + open plants still holds bit for bit). An industry that never closes goes back to full time at its review.
- **Why a fixed share for the cut:** jobs already follow demand month by month (`target = capacity × effective`), so a cut fitted to the shortfall would count the fall twice. What the review adds is the part that stays lost when orders come back; one number says it and the event can state it.
- **The early paths stay:** full time after 2 months at or over 0.94 (no event), and a notice after 9 months under 0.78. A reprieve now starts a new spell of short time (`since` = the reprieve's day), so its review is 12 months on; before, the spell kept the notice's day.
- **The spine:** `EventKind::PlantReviewed { municipality, plant, industry, jobs, months, demand_pct, line_pct, outcome, jobs_cut, factor }` on `Thread::Plant` (advances it to a notice; closes it at full time), salience notable, caused by the weakest factor (`Pressure`) as the other steps are. The notice it gives records `Event(review, Caused)` beside its pressure.
- **Text** (DEC-054, `industries.ftl`): `event-plant-reviewed-notice` "The review at { $plant } in { $town }: short time had run { $months } months, and demand was still under { $line }% of normal, at { $demand }%."; `-full`; `-cut` (with `{ $cut }` jobs fewer); their `headline-` lines.
- **Content:** `economy.industries.review_months` (u16), `review_line`, `review_cut`; validation: `notice_line < review_line <= full_line`, `review_months > short_months`, `review_cut` in [0, 1]. Schema regenerated.
- **Lab:** `plant_reviewed` in the lab's tallies (`lab/metrics.rs`) and `audit stuck`'s review steps.

## Tests

- `worldgen/tests/plant_review.rs`: the twin (the same plant, twelve months of short time; demand held under the review line: `PlantReviewed { outcome: notice, months: 12, line_pct: 88 }` then the dated notice the same day, caused by it, a date in the notice range, the town's notice date set; held over it: `PlantReviewed { outcome: full_time }`, capacity × 0.9, jobs cut, the thread ended, no notice); no review before its months; the probe (`--ignored`).
- `save/tests/industries.rs`: a plant a month short of its review saved, loaded and played beside the unsaved game: one state hash month by month, one review each.
- `wasm/tests/why.rs`: the Why of a notice from a review: the notice's line, then `event-plant-reviewed-notice` with months 12 and line 88, linked `caused`, with the weakest factor under it.

## Measured

The probe: `cargo test --release -p mandate-worldgen --test plant_review -- --ignored --nocapture` (16 small worlds, every town, idle from the first day of play; before = the same build with `MANDATE_PROBE_REVIEW_MONTHS=9999`, the rule before the amendment, which draws nothing else and reproduces item 5's start: 258 plants, the longest 236 months). Worlds differ after: their twenty years of history ran with the review.

| | before, 39 weeks | after, 39 weeks | before, 5 years | after, 5 years |
|---|---|---|---|---|
| on short time at the start (a year or more; longest) | 258 (142; 236 months) | 131 (0; 11 months) | | |
| new spells of short time | 85 | 73 | 497 | 470 |
| reviews (to a notice / to full time) | 0 | 83 (32 / 51) | 0 | 402 (197 / 205) |
| notices | 1 | 32 | 80 | 204 |
| reprieves | 0 | 19 | 1 | 113 |
| closures (named plants) | 3 | 8 | 75 | 88 |
| of the start's short time: short / full / notice / closed | 184 / 73 / 1 / 0 | 46 / 68 / 15 / 2 | 75 / 131 / 5 / 47 | 10 / 81 / 1 / 39 |
| on short time at the end (a year or more; longest) | 251 (155; 245 months) | 107 (0; 11) | 187 (122; 296) | 58 (0; 11) |
| jobs lost: closures + review cuts | 4,011 + 0 | 18,717 + 5,976 | 149,698 + 0 | 110,305 + 23,305 |
| jobs lost per town per year (480 towns) | 11.2 | 68.8 | 62.4 | 55.7 |
| plants a review gave two notices or more (notice, reprieve, short time, review again) | | 0 of 32 | | 38 of 153 |

`arcs grade --seeds 16 --arc arc.plant_closure` (before: the base binary on the base content): found in 14 of 16 worlds both times (planted 14; seeds 2 and 16 never raise it), so the raise rate is unchanged; it builds more: the passive stand-in's run goes from short time to the notice in 8 of 14 (6 of 14 before; the scare 5, was 8; one reprieve), escalation median 0.95 (0.038), visibility median 0.40 (0.00), E×V×P first pass median 0.19 (0.00). Exits, engaged: answered 0.357 (0.286), closed 0.071 (0.071), reprieved 0.214 (0.071), scare 0.357 (0.571). The hall: split in 1–2 of 7 now (lost in all before).

`bots --seeds 16`: every gate as before (enforced: effort_matters, exploring_alone_never_wins, ladder_rote, ladder_learned, corrupt_not_dominant, proxy_not_optimal pass; report-only knowledge_matters and skill_gap fail as before). No gate made report-only.

The audition (`lab audition`) is not on this base (A7 is on `a8-draft`, not merged): not measured.

## Worked example

World 1, the Crown works (`industry.metals`, town 1): short time in December 2018 (foreign competition the weakest factor); the review in December 2019, "short time had run 12 months, and demand was still under 88% of normal, at 82%", gives a notice; demand came back over 0.84 and the notice was lifted in April 2020 (a new spell); the next review, April 2021, at 75% (world orders now the weakest), gives a second notice; the works closed in August 2021 with 1,854 jobs. The notice's Why: "Closure notice at the Crown works … because of: The review at the Crown works …: short time had run 12 months, and demand was still under 88% of normal, at 82%. ← What weighed most on the sector: the country's industry has lost ground to its competitors."

## Pins and tests outside the plants

`scripts/repin.sh`: the spine proof's pins, the state-shape fingerprint (v34: `cba106bf9975aa18`), the town geography pin, `tests/seeds.json` (two requirements found again: 48 to 49, 1 to 2), the asks fixture; the pocket and scene fixtures re-written (`MANDATE_WRITE_FIXTURES=1`). Two tests read worlds the review changed, scoped to what they mean: `save/tests/ground.rs` (a v20 payload made from today's state drops `arcs`, as it drops `ground`: seed 9 now holds a run) and `wasm/tests/visits.rs` (a works shut in the town's history has no day shift: seed 1's town now has a closed mine).

## Not done, and open

- **A conflict the review exposed, fixed (lead's request):** `worldgen/tests/narrative.rs` `the_spotlight_never_changes_endogenous_outcomes` failed at seed 7 (`.arcs.past[1].marks: lengths 0 vs 1`). The plant arc now reaches its notice in the player's town beside the zoning fight; with the spotlight on, the director's chapter lead (the plant, a crisis) holds the chapter's one scene (`exchange::staging`, DEC-112), so the zoning meeting's day passed with nothing; with the spotlight off the director returned before choosing a lead, every run could hold its scene, and the meeting was played as an empty chair. Fix (`narrative/director/mod.rs`): with the spotlight off the lead is chosen as with it on (`lead`, for an active player), so which run holds the scene is the same either way; its inputs (the runs, the chapter, the arcs' uses) are none of the spotlight's. Spotlight-on play is unchanged: no pin moved (`repin.sh --check`). The test passes at all four seeds.
- A notice from a review can be lifted at 0.84, under the review line (0.88), so a plant between the two goes notice, reprieve, short time, review, notice once a year: 38 of the 153 plants a review gave a notice in five years got two or more. Kept as the owner's "the existing notice path, reprieve possible"; a balance row (the reprieve line against the review line).
- Jobs lost per town per year rise in the first 39 weeks (11 to 69: the plants that had sat on short time now meet their reviews) and are a little lower over five years (62 to 56); closures 75 to 88. Balance rows.
- The audition's before and after: on the branch that holds A7.
