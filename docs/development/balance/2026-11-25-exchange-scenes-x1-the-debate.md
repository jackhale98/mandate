# Exchange scenes X1: the debate (figures only)

**Date:** 2026-11-25
**Area:** arcs (`content/base/exchange.jsonc`, `arcs/rival.jsonc`; `mandate-sim/src/arcs/exchange`); DEC-112.

## Parameter and change

A mechanism change. **Nothing was tuned.** Every constant of `exchange.jsonc` is a placeholder: the design's first value where it gives one (`EXCHANGE_SCENES.md` §5.6), a first guess where it does not (marked in the file). They are set at the tuning pass at the end of M6.

- **Before:** the rival's showdown was a decision (`dilemma.debate_challenge`): accept (recognition +0.04 in the town, a slot) or decline (approval −0.01, the default). A time skip could answer it.
- **After:** a debate of three rounds at a hall, on a day 14 to 21 days after the showdown opens. A player who never comes meets an empty chair: every prompt bites in full, the first one cold. No bot and no stand-in plays a scene yet (task X3), so every lab player now meets the empty chair.

## Evidence

Before: 31d7bc2. After: this work merged onto `claude/bold-brown-shyeug` (366088e). `MANDATE_LAB_THREADS=2`.

**`mandate-cli bots --seeds 16`** (first election; win rate, mean share):

| Strategy | Win before | Win after | Share before | Share after |
|---|---|---|---|---|
| idle | 0.0625 | 0.0000 | 0.320 | 0.315 |
| explore_only | 0.0625 | 0.0000 | 0.320 | 0.315 |
| media | 0.1875 | 0.1875 | 0.383 | 0.373 |
| canvass | 0.5000 | 0.5000 | 0.453 | 0.439 |
| asks_only | 0.8125 | 0.7500 | 0.576 | 0.571 |
| proxy | 0.7500 | 0.7500 | 0.579 | 0.571 |
| directed | 0.8750 | 0.8750 | 0.633 | 0.623 |
| corrupt | 0.9375 | 0.9375 | 0.706 | 0.696 |
| legislator | 0.9375 | 0.9375 | 0.715 | 0.706 |
| mixed | 0.9375 | 0.9375 | 0.725 | 0.718 |
| explorer | 0.9375 | 0.9375 | 0.740 | 0.732 |
| explorer_campaigner | 0.9375 | 0.9375 | 0.737 | 0.729 |
| explorer_coalition | 0.9375 | 0.9375 | 0.735 | 0.728 |
| explorer_legislator | 0.9375 | 0.9375 | 0.742 | 0.734 |

| Gate | Before | After |
|---|---|---|
| `idle_cap` (enforced) | pass (idle 0.0625) | pass (0.0) |
| `effort_matters` (enforced) | pass (best 0.9375) | pass (0.9375) |
| `exploring_alone_never_wins` (enforced) | pass (0.0625; rote median 0.8125) | pass (0.0; 0.75) |
| `ladder_rote` (enforced) | pass (mixed share 0.7252) | pass (0.7176) |
| `ladder_learned` (enforced) | pass (explorer − mixed 0.0148; margin 0.01) | pass (0.0145) |
| `corrupt_not_dominant` (enforced) | pass (0.0) | pass (0.0) |
| `proxy_not_optimal` (enforced) | pass (directed 0.633; informed median 0.739) | pass (0.623; 0.730) |
| `knowledge_matters` (report-only) | false | false |
| `skill_gap` (report-only) | false (share gap 0.0169) | false (0.0168) |

No enforced gate changed its answer, so none was made report-only.

**The empty chair, eight seeds (1, 2, 3, 7, 31, 99, 777, 2026), a newcomer's first campaign:** the weighted change in lean is −0.39 in each (a thin record put cold, then two questions), which is −0.031 approval in the seat at `approve` 0.08, before the room each cohort has left. The decline it replaces cost −0.01.

**A debate played by the engaged script (seed 7):** split, +0.009.

## Effect

Every lab player loses about one point of first-election share to the empty chair, and the idle player's one win in sixteen is gone. The size of a scene in the world (`approve`, `aware`, `story_weight`), the thresholds (`win`, `lose`) and the strength scales (`worries[].full`, `heard_floor`, `act_base`) are the first things the tuning pass sets: an empty chair should cost about what declining cost, and a typical good law should give a `record` line near 0.6.
