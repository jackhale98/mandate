# The record decides votes (audit fix 1)

**Date:** 2026-11-23 (final pass 2026-11-24, on the tree with fix 2 and the causal spine)
**Area:** politics (vote model), opinion (approval), career (record of visible work)

## Parameter and change

Rule changes (`politics/elections.rs`, `population/mod.rs`, `concerns.rs`):
- The record term lost its hidden `× 0.1`; `retrospective_weight` is now the weight that applies. Approval is judged against the nation's mean approval (residents' mean), not against ½.
- The vote's benchmarked economy adds the town's unemployment rate now above the nation's (`retro_level_weight`) beside the year's change.
- Approval's experienced term adds the benchmarked level (`opinion.experience.unemployment_gap`), and each neighbourhood feels the change and the level through its own unemployment factor (the jobs worry's factor: income classes and sectors' losses since play began).
- Door-knocking alone builds no record of visible work (`career.record_gain.canvass` 0): the record comes from bills, votes, lobbying, town halls, asks kept and laws, so a councillor who only canvasses is attacked for a thin record (`record_penalty`, unchanged).
- Awareness can saturate (`awareness_known`, `awareness_beyond`); off (1.0, 1.0): every active style is near-fully known by election day, and the cap compressed informed play as much as canvassing.

| Parameter | Before | After |
|---|---|---|
| `politics.vote.retrospective_weight` | 6.0 (× 0.1 in code) | 5.0 (no hidden factor) |
| `politics.vote.race_shock_sd` | 0.25 | 0.12 |
| `politics.vote.national_swing_sd` | 0.2 | 0.15 |
| `politics.vote.retro_economy_weight` | 0.5 | 2.0 |
| `politics.vote.retro_level_weight` | (new) | 0.5 |
| `politics.vote.incumbent_awareness` | 0.55 | 0.45 |
| `politics.vote.record_bonus` | 0.15 | 0.0 |
| `politics.vote.player_approval_weight` | 1.2 | 2.5 |
| `politics.vote.awareness_known`, `awareness_beyond` | (new) | 1.0, 1.0 (off) |
| `politics.campaign.persuasion_per_canvass` | 0.08 | 0.03 |
| `career.record_gain.canvass` | 0.03 | 0.0 |
| `actions.canvass.approval` | 0.012 | 0.012 (unchanged: 0.004 made no difference to any rate) |
| `opinion.experience.unemployment` | 8.0 | 12.0 |
| `opinion.experience.unemployment_gap` | (new) | 6.0 |

## Evidence

Release build, two lab threads. "Before" is the tree before fix 1 (after the party-line change, before fix 2); "after" is the final tree (fixes 1 and 2 and the causal spine).

| Measure | Before | After |
|---|---|---|
| `audit believe --seeds 8`: re-election gap, worst minus best tercile of relative unemployment change | 3.0 pts | 22.8 pts (88.4, 82.7, 65.6%) |
| re-election per approval point | +0.1 pts | +2.1 pts |
| re-election when running (council / executive) | 80.8% (81.8 / 72.5) | 78.0% (78.7 / 71.8) |
| planted closure: incumbents' wins in the home town afterwards | 229 → 232 of 378 | 220 → 94; 170 races flipped |
| closure: approval at +12 / +48 months (unemployment +3.7 pts at +48) | −11.9 / +3.0 | −18.0 / −6.8 |
| closure: approval by neighbourhood at +12 | −11.9 … −11.8 | −28.2 … −13.2 |
| `audit readability --seeds 6`: incumbent losses where the shock / the record is the largest factor | 55.9% / 1.2% | 16.1% / 47.7% |
| `audit gate`: retrospective (gap ≥ 10, ≥ +0.5 per approval point) | fails (0.5 pts) | passes (24.6 pts, +2.1) |
| canvass / best informed first-election wins: `bots --seeds 16`; `--seeds 32`; `audit leverage --seeds 16` | 0.69 / 0.81; 0.72 / 0.81; 0.69 / 0.81 | 0.31 / 0.88; 0.34 / 0.84; 0.31 / 0.88 |
| `good_styles_vary_b` (16 / 32 seeds) | fails / fails | passes / passes |

## Effect

Governing well now matters to every incumbent, and to the player: a bad year or a lasting gap against the nation costs seats, a closure keeps costing until the town recovers, election night names the record far more often than luck, and a councillor who only knocks on doors is attacked for doing nothing else. Swept on the final worlds (16 and 32 seeds each): the canvass record gain (0.03, 0.01, 0), the canvass approval nudge (0.012, 0.004), the record bonus (0.4, 0.15, 0), the weight of approval of the player (0.6–2.5), persuasion (0.02–0.08), the race shock (0.12–0.18), the home-ward bonus and saturating awareness. A canvass record gain of 0.01 already keeps canvassing at 62–66%; only 0 separates it. Approval of the player at 2.5 is what keeps `ladder_learned` (learning the town pays) above its margin at 32 seeds (+0.0105). The cost: the best informed style wins 0.88 on 16 seeds (14 of 16; the band's top is 0.85) and 0.84 on 32; every lower point found on the same worlds broke `ladder_learned` at 32 seeds. `knowledge_matters` (+0.5 pt on 16 seeds, −0.2 on 32; needs +3) and `skill_gap` (share +3.0 / +1.9 pts, needs +3; wins +6 / +3 pts, needs +10) still fail and stay reported.
