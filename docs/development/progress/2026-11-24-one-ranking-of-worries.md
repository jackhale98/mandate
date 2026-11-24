# One ranking of worries (concerns::ranked)

**Date:** 2026-11-24
**Status:** done (no pin moved; one view changes what it shows)

Why: two parts of the engine could name different top worries for the same town. `the_concern_condition_reads_the_top_concern` (talk_lines.rs) failed for seed 7 when `race_shock_sd` went 0.12 → 0.132 as an experiment: the talk context said the environment, `concerns::top_concerns` said housing.

## Why they disagreed

Not a tie-break and not a stale cache: two readings of the same levels. `top_concerns` ranked a town's worries by level alone. The talk context, the `concern` condition's default and every notebook view ranked them as felt: level × (1 + ½ × how far residents lean the way worried voters do). In seed 7 after three turns, with the changed number: by level housing 0.4666, environment 0.4493; as felt environment 0.4828, housing 0.4739. The lean (×1.075 against ×1.016) is larger than the gap between first and second. The test compared one reading with the other, so it held only where the leader's margin was wider than the lean.

It was already wrong on the branch, with no balance change: in seeds 35, 53 and 59 (small worlds, first turns) the town's top worry by level is not the one most felt, so the town card on the map (by level) named another top worry than the notebook, the pocket and talk (felt). The top three differed in 9 of seeds 1–60 at turn 0–3. TEXT_AUDIT_1 item 2 fixed the same split for talk only.

## What ranked worries, and what each read

| Reader | Scope | Read before |
|---|---|---|
| `concerns::top_concerns`: a town's priorities (`characters/town.rs`, kept monthly), `Knowing::town_concerns`, the map's town card (`office::town_concerns`), the lab corpus | town | level (the town's stored levels: neighbourhoods weighted by residents); ties in registry order |
| `concerns::felt_concerns`: talk (`talk/context.rs`), the `concern` condition `by: felt`, overheard lines, the canvass match, `hood_reading`, `Knowing::concerns_of` / `top_worry` (the bots), `office::concerns_for` (notebook cards, pocket, office, town character, town panel, canvass preview) | town (all its neighbourhoods re-averaged) or one neighbourhood | level × lean; ties in registry order |
| the `concern` condition `by: measured` (dilemmas, asks, promises, goals, the director's themes) | town; neighbourhood | the town's cached priorities (three); `levels_in` with its own sort |
| `hood_concern` condition and a policy family's choice of neighbourhood (`policy/families.rs`) | neighbourhood | stored level; place = how many are strictly stronger (ties shared a place, a worry at level 0 could be "among the top three") |
| the promise menu (`asks/commands.rs::top_worries`) | a race's constituency | unweighted mean of neighbourhood levels, level 0 included; ties by concern id |
| the vote model and NPC agendas (`salience_in`, `political_levels`) | constituency | measured town-wide levels shifted by neighbourhoods: weights, not a ranking (unchanged) |

The press and the director have no ranking of their own (they read the condition); the bill's ward view reads support, not worries.

## The one function

`concerns::ranked(pop, content, world, scope, by) -> Vec<(index, weight)>`, strongest first.

- `Scope`: `Jurisdiction` (a town, a region, the nation), `District` (some neighbourhoods, in one town or several), `Neighbourhood`. All of a town's neighbourhoods are the town; a district of one neighbourhood is that neighbourhood.
- `by` is the content's `Worry`: `Felt` (what people say and the player is told) or `Measured` (by level alone: priorities, dilemma, ask and promise conditions, a family's neighbourhood).
- Levels (`levels_of`): a town's stored levels, a neighbourhood's own, a district's weighted by residents, towns weighted by their residents in the scope. No level changes.
- Tie-break (`order`, the only sort): the stronger first by `f64::total_cmp`; equal weights keep registry order (the order of `concerns.jsonc`).
- `top(ranking, n)`: the first n of those anyone worries about (weight above 0). `place(ranking, concern)`: its place among them.
- What the player knows is a cut of the same list (`gate_concerns`, `Knowing::ranked`), never another ranking.

`top_concerns` and `felt_concerns` are gone; every reader in the table calls `ranked`. `office::concerns_for` takes a `Scope`.

## What changed for the player and for outcomes

- The map's town card now names worries as felt, like every other view (it was by level).
- No pin moved: `scripts/repin.sh --check` reports every pin current (the spine proof's `PINS` and `BEFORE_ARCS`, the shape fingerprint, the fixtures, the geography, the seeds).
- Where an outcome could differ in a world not pinned: a `concern` condition `by: measured` on a town reads the levels now, not the priorities kept at the month's character step (the same list whenever that step has run); a `hood_concern` or a family's neighbourhood no longer counts a worry at level 0 among the top, and two worries at exactly the same level take successive places in registry order instead of sharing one; the promise menu weighs a constituency's neighbourhoods by residents and leaves out worries at level 0.

## Tests

- `concerns.rs`: the tie-break, and `top` / `place`.
- `crates/mandate-wasm/tests/worries.rs`:
  - `every_reader_names_the_same_top_worry`: seeds 1–12 at turns 0, 2, 5 and 9; conditions (felt and measured, top 1–3), priorities, readings, overheard lines, the canvass match, the engine's knowledge view and every protocol view against `ranked`; a region and the nation rank too.
  - `the_player_is_told_one_ranking_of_the_towns_worries`: seeds 7, 53, 59 at heard, known and detailed; the notebook card, the canvass preview, the pocket, the office, the town's character and the town card are cuts of one list, and a reply about the whole town names its first.
  - Both fail if a view is switched to the other reading (seed 53: the reply says taxes, the view housing).
- `the_concern_condition_reads_the_top_concern` asks `ranked` for the town's felt worries (the reading the condition's default takes) in place of `top_concerns`, which no longer exists; its assertions are unchanged. It passes on the branch and with `race_shock_sd` 0.132.

## Checks

- `scripts/check.sh full`: 16 passed, 0 failed (e2e not run).
- `scripts/repin.sh --check`: no pin moved.
- `determinism --seed 2026 --turns 80`: `896728a6…` twice and through save and load, the same as the branch head's build.
- Native and WASM agree on seed 777 medium 60 turns (`2e58908e…`), the branch head's hash.
- `bots --seeds 16`: the seven enforced gates pass; the output is byte-identical to the branch head's, so no win rate moved.
- `content doctor --seeds 0 --strict-prose`: 0 errors, 0 warnings, no prose finding (33 notes of unused text, as before).

## Left

- The town card shows three worries of any town to anyone; the notebook shows one at `heard`. They agree on order, but the card is not gated by knowledge.
- Promises and dilemmas ask for `measured` and the notebook shows `felt`, so a promise can be offered on a worry the notebook's top three does not list. That is content's choice of reading, not a second ranking.
