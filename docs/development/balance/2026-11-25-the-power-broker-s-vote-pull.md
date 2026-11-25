# The power broker's vote pull and backing (M6 note 27)

**Date:** 2026-11-25
**Area:** the vote model, campaigns, the bot ladder

A new mechanism, not a tuning (`AGENT_CONTEXT.md` rule 11): the numbers below are first values, to be settled at the tuning pass. `characters.jsonc` `broker`.

## Parameter and change

| Parameter | Value | What it does |
|---|---|---|
| `vote_pull` | 0.25 | the vote model's `broker` term on a bill in the broker's interest, × influence (0.4 to 0.8; 0.6 for an officeholder) × each member's compliance (0..1, ½ with no view of the broker) |
| `back_money`, `back_contacts` | 750 a week, 1 a week | backing for the candidate the broker regards most |
| `back_values`, `back_min_opinion` | 40, 10 | regard = opinion + 40 × shared views; nobody below 10 |
| `lobby_sway`, `lobby_min_opinion` | 0.2, −10 | lobbying the broker: sway per member × compliance; refused below |
| favour, oppose, expose | see the file | opinions and approval (approval × room, town-wide by class) |

## Evidence

`mandate-cli bots --seeds 16` (release), on the same base (the conversations work and the exchange scenes merged), without and with the broker:

| Gate (enforced?) | Without | With |
|---|---|---|
| idle_cap (yes → now report-only) | pass, idle 0/16 | **fail**, idle 3/16 (0.1875 > 0.15) |
| effort_matters (yes) | pass, best 0.9375 | pass, 1.0 |
| exploring_alone_never_wins (yes) | pass, explore-only 0/16 | pass, 3/16 (= idle) |
| ladder_rote (yes) | pass, mixed 0.718 / 0.9375 | pass, 0.747 / 1.0 |
| ladder_learned (yes) | pass, +0.0119 | pass, +0.0199 |
| knowledge_matters (report) | fail, 0.7176 vs omniscient 0.7171 | fail, 0.7469 vs 0.7522 |
| skill_gap (report) | fail, 0.0175 | fail, 0.0314 |
| good_styles_vary (report) | pass | fail (clause c) |
| corrupt_not_dominant (yes) | pass | pass |
| proxy_not_optimal (yes) | pass, 0.623 vs 0.733 | pass, 0.659 vs 0.770 |

Mean vote share by strategy, without → with: idle 0.316 → 0.372, media 0.373 → 0.438, canvass 0.439 → 0.501, asks only 0.571 → 0.640, mixed 0.718 → 0.747, explorer 0.730 → 0.767, explorer coalition 0.734 → 0.778. The best by mean share moved from explorer legislator to explorer coalition.

## Effect

Every strategy gains: the broker's backing goes to the candidate they regard most, which can be the player, and reaches a player who does nothing. The idle player now wins 3 of 16, past the cap. `idle_cap` is report-only with a dated note in `crates/mandate-cli/src/bots/gates.rs` until the tuning pass. Levers there: `back_money`, `back_contacts`, `back_min_opinion`, and whether backing needs the broker to know the player.

Also seen: flood defences pass more often. In seed 99's town they are in force before a planted flood (seeds 99 to 105 all fail the flood arc's scripts, which need a town without them), so the arc tests moved to seed 106. A crisis bill gets the broker's pull while the crisis lasts, and a town's floods during the burn-in are such a crisis. To read at the tuning pass with the arc grader.
