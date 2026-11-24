# Audit fix 1: the record decides votes

**Date:** 2026-11-23 (final pass 2026-11-24)
**Status:** done; one band edge missed (below).

What was done:
- Vote (`politics/elections.rs`): the record term's weight lives in content (no hidden `× 0.1`); approval is judged against the nation's mean approval; the benchmarked economy reads the town's rate now above the nation's beside the year's change; optional diminishing returns on awareness (`known_share`, off in the base pack).
- Opinion (`population/mod.rs`, `unemployment_felt`): approval's experienced term adds the benchmarked level, so a closure costs until the town recovers; each neighbourhood feels unemployment through its own factor (`concerns::hood_unemployment`, the factor the jobs worry reads), so the estate by the plant blames the town hall most (audit fix 6). No new saved state.
- Career: canvassing adds nothing to the record of visible work (`career.record_gain.canvass` 0), so door-knocking alone does not build a governing record; the existing thin-record attack does the rest.
- Content: new `politics.vote.{retro_level_weight, awareness_known, awareness_beyond}`, `opinion.experience.unemployment_gap`, bounds in `validate/tuning.rs`, schema regenerated; values in the balance entry.
- Gates: `audit gate`'s believe core requires retrospective voting to work (gap ≥ 10 points, ≥ +0.5 points of re-election per approval point); `good_styles_vary_b` is enforced again.
- Election night: the breakdown has one `shock` factor (race noise and national swing together), so it was not split; the protocol is unchanged.
- Dead mechanisms (β, the partisan filter, trust) were left alone; nothing here depends on them.
- Tests: `talk.rs::every_topic_works` checks the late-night rule in the council chamber itself; `characters.rs::opinion_and_favours…` finds a colleague with no edge by search and checks the trade opens one.

What is left:
- The best informed style wins its first election 0.88 of the time on 16 seeds (band top 0.85; 0.84 on 32 seeds). On these worlds every setting that lowered it broke `ladder_learned` at 32 seeds.
- `knowledge_matters` and `skill_gap` still fail (numbers in the balance entry); not enforced.
- `audit gate`'s policy part fails on the final tree: `policy.emergency_rent` and `policy.fiscal_bridge` lower approval (transfers; audit fix 3's ground).
- The executive party never changes in 30 years in 38.5% of towns (fix 4).
- The pacing test (`narrative::pacing_stays_within_budgets_and_cooldowns_hold`, a climax in at least 85% of chapters) fails: 71 of 84 on seeds 1–12 (the branch had 72), 69 of 84 on seeds 13–24. Every chapter without one is a quiet `local_life` session: at each of its climax peaks the director had no grounded candidate, nothing pending and spacing met, and the fallback (cooldowns waived, at most two earlier uses, climax memory, no repeat in a chapter) found none either. The owner's decision (2026-11-24): the bar is 80% for now; the director-on-arcs task (A2, DEC-109) raises it to 95% and removes the climax fallback.
- The once-failing `media.rs::the_director_shifts_when_never_whether` (seed 5: `Fate.who` 1962 against 1961, in one workspace run) did not reproduce in 100 runs alone or 15 whole-file runs at 1, 2 and default threads; no process-global state, unordered map, clock or thread use was found in the engine or content loading.
