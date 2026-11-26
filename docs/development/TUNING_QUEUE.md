# Tuning queue

Numbers are tuned once, at the end of M6, before the tester checkpoint (owner, 2026-11-25; `AGENT_CONTEXT.md` rule 11). Until then mechanisms land with their correctness tests, balance gates they break become report-only, and what is off is listed here with its measured figure. Nothing on this list is acted on before the pass, unless the game stops being playable.

## The pass, in order
1. Re-run the audit (`mandate-cli audit gate`, `stuck`, `policy`, `believe`) and the bots at 16 and 32 seeds; record the baseline.
2. Tune, one system at a time, from the vote model outward; each change with before and after in `docs/development/balance/`.
3. Re-fit everything fitted to the numbers, each by its own command: the impact estimate's coefficients, the start ratings, the arc scorer and audition.
4. Re-enforce every gate listed below at its full threshold, re-record the pins (`scripts/repin.sh`), run the full check.

## Waiting

| Item | Target | Measured | Since | Gate |
|---|---|---|---|---|
| `knowledge_matters`: knowing the town helps | omniscient play at least 3.0 points above blind | +0.5 (16 seeds), +0.1 (32) | 2026-11-20 | reported, not enforced | Read with DEC-091's principle (owner, 2026-11-25): pass it by making knowledge change which choice is best, never by making it a requirement; and add the measure that learning everything first does not beat learning what a plan needs. |
| `skill_gap` on wins: informed play out-wins rote play | a clear margin in win rate | +0.00 (16), +0.06 (32); passes on share | 2026-11-20 | reported, not enforced |
| One-party towns | few towns never change party | 41.1% never change (38.5% before the laws fix) | 2026-11-24 | none yet |
| Pacing: a chapter's turn is put to the player | asked at climax weight in 95% of sessions, in office or out | reached in 84 of 84 chapters; in office asked 12 of 12 and 16 of 16; out of office 2 of 60 and 3 of 56 | 2026-11-25 | test asserts reached ≥ 95% and asked-in-office ≥ 95%; out of office is printed. Needs a beat for a player without a vote (content, A6), not numbers |
| Skip-only play (DEC-102 clause 7) | first-election wins no higher than canvass-only; re-election about 40–45% | 56% first-election wins, share 0.550 | 2026-11-24 | `proxy_not_optimal` passes at its old, loose bar |
| Skipping the quiet weeks (DEC-102 clause 7) | within a few points of playing every turn | `bots --seeds 16`, first election: the stand-in that attends the stops wins 1.0, share 0.719, against the engaged one every turn 1.0, 0.718 (before the stand-ins played the debate: 0.75, 0.675 against 0.94, 0.697); the informed style attending 1.0, 0.773 against 1.0, 0.781 | 2026-11-24; measured 2026-11-25 (exchange X3) | the bots' `skips` section, reported |
| The asks-only routine | a single-action routine should not win half its races | 50% wins, share 0.496 | 2026-11-24 | none; `ladder_rote` passes |
| Services laws after the laws fix | worth passing | +1.2 to +1.5 approval (+2.9 to +3.1 before) | 2026-11-24 | none |
| `audit gate`, policy part | stable | passes at services weight 1.2; 1.25–1.4 each failed one check at 3 seeds | 2026-11-24 | enforced; noisy |
| Templates that lower approval at five years | none without a stated cost | 4 remain: property tax, fiscal bridge, tech incubator, flood defences | 2026-11-24 | reported |
| `good_styles_vary_b`: no style among the best in more than 0.6 of worlds | ≤ 0.6 | `explorer_coalition` 10 of 16 (0.625); 16 of 32 passes | 2026-11-25 (arcs A2) | was enforced; report-only, dated note in `bots/gates.rs` |
| The budget round's reach (`budget_round.rs` band) | carried ≥ 5, spent ≥ 5, favour owed ≥ 4 of 8 seeds | 6, 5, 3 (7, 7, 6 before the laws fix) | 2026-11-25 (arcs A2) | bars lowered to 4, 3, 2 |
| Closures with no recovery in ten years (industries) | few | 5.9% of closures (2.0% before industries) | 2026-11-25 (industries I1) | `audit stuck`, reported |
| Time towns spend in crisis | no worse than before industries | 4,166 crisis spans (3,520 before): +18% | 2026-11-25 (industries I1) | reported |
| Bots win more with industries | idle ≤ 15%; skip-only as DEC-102 clause 7 | at 32 seeds: idle 12% (6%), mixed 91% (81%), skip stand-in 72% (50%) | 2026-11-25 (industries I1) | `idle_cap` still passes |
| Textiles' long decline | ends 30 years above 0.35 of its jobs | 0.33 | 2026-11-25 (industries I1) | the audit's band, reported |
| Plants opening against plants closing | openings about 40% of closures | 27% | 2026-11-25 (industries I1) | design band, reported |
| What is behind a closure | the industry's index behind about 60% | 69% | 2026-11-25 (industries I1) | design band, reported |
| Plants that start a game on short time | few | 29% | 2026-11-25 (industries I1) | reported |
| `idle_cap` with the power broker | idle ≤ 15% | idle wins 3 of 16 (19%) once the broker backs candidates and leans on votes | 2026-11-25 (broker, note 27) | made report-only on the broker merge |
| The flood arc's paths across worlds | every stage and exit reachable in most worlds; passive and engaged end differently in ≥ 60% | the scripted tests hold on 2 of 6 worlds before industries and 1 of 11 after (seed 99) | 2026-11-25 | the grader (`arcs grade`) is to measure it; the scripts are pinned to one world |
| The impact estimate's fit (DEC-111) | the card says something a player can act on; every honesty rule enforced | fitted on 8 seeds, month 1, the home town; ranges hold their twin 79.5% (band 70–90%); the finer rules (silent effect 102 cells, level 33, size 19) are report-only; a housing bill reads "150 homes started. No clear change in the number of homes." | 2026-11-25 (impact estimate) | `audit estimate`: direct counts, cost, sign and coverage enforced; the rest in `REPORT_ONLY`. Refit at full size with `mandate-cli lab impact calibrate` (`scripts/repin.sh --impact`) after tuning |
| `economy.attribution.homes_net` and `jobs_net` | ledger "added" within 0.67–1.5 of the twin | affordable housing 1.55 at five years, upzoning 1.50, tech incubator 0.63, closure response 3.1 / 5.8 / 0.16 | 2026-11-25 (impact estimate) | reported |
| The estimate's silent-effect rule | decide what it judges | it tests the mean of 24 twins while the range shown covers one game; both cannot hold at this scatter. Lead's view: judge whether a single game lands in the range | 2026-11-25 | owner's question, open |
| A scene's lines vary (`exchange_kinds_vary`, EXCHANGE_SCENES §9.1) | no kind of line is the engaged stand-in's pick in over 60% of rounds | the debate: `promise` 87.5% of rounds (16 worlds); the plant's hall: `offer` 33% (passes) | 2026-11-25 (exchange X3) | `arcs grade` bar `scene_kind_dominant` and the doctor's `exchange-kind-dominant`, reported |
| The turn changes how an arc ends | the engaged stand-in's exit differs, turn played or chair empty, in ≥ 60% of worlds (§6.9's bar, on the turn alone) | the debate 1 of 16; the plant's hall 1 of 14; the flood's meeting 0 of 16 (the engaged stand-in never reaches it) | 2026-11-25 (exchange X3) | `arcs grade` bar `turn_changes_the_exit`, reported |
| Preparation shows in the room (`exchange-prep-idle`) | the engaged stand-in's mean result above the one that only attends the stops | the debate +24 thousandths (passes); the plant's hall −17 | 2026-11-25 (exchange X3) | `arcs grade` bar `scene_prep_pays` and the doctor's lint, reported |
| Every outcome of a scene reached (`exchange-outcome-unvisited`) | each of won, split, lost, absent by some stand-in | the debate never `lost`; the plant's hall never `won`; the flood's meeting never `won` or `lost` | 2026-11-25 (exchange X3) | the doctor's lint, reported |

