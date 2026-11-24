# Audit fix 2: rents bounded, private building restarted

**Date:** 2026-11-23
**Status:** done

- `economy::housing`: households double up as the rent burden rises, builders answer vacancy and price over cost, and the start is an equilibrium. Every housing constant is content (`EconomyParams::housing`). No saved field was added; the save format is unchanged.
- `population::apportion`: movers, arrivals over a town's neighbourhoods and the displaced are split by the largest-remainder method, so no town or neighbourhood gains from its position in a list (the last town used to grow up to fifteenfold over the burn-in).
- `audit stuck` reports real rent growth, rent burdens and town growth over the burn-in; `audit gate` gains the rent-sanity check (2 seeds × six worlds × 30 years: no runaway town, median real rent growth within ±50%). It passes.
- Re-seeded because the world changed: `talk::voices_choose_the_closers` 35 → 36, `knowledge::people_by_level` 13 → 16, `asks::laws_in_force_never_fall_unexplained` and its e2e twin 5 → 14, `characters::a_plant_closure_crisis_comes_and_goes` 11 → 12; e2e `asks` 3 → 10 and 2 → 6, `offices` 2 → 4, `talk` 7311 → 7312.
- Results and evidence: `docs/design/research/SIMULATION_AUDIT.md` (topic 6, "After fix 2") and the balance entry of the same date.
