# The budget round and the turn

**Date:** 2026-11-24
**Area:** narrative (the director's pacing), arcs (the standing arcs), finance (a budget voted for next year)

## Parameter and change

Staged arcs A2 (DEC-109): a chapter's climax is its lead arc's turn, the climax fallback is gone, and every session has the budget round. The rules are in the progress entry of the same day; the numbers that were set or moved:

| Parameter | Before | After |
|---|---|---|
| `director.cycle.climax_force_days` | 14 | removed (no forced climax peak) |
| `director.climax_memory_chapters` | 2 | removed (variety is the arc's: `arc_last_turn`) |
| `director.storylines_active` | 2 | `arcs_active` 2 (standing arcs aside) |
| `director.relevance.storyline` | 1.0 | `relevance.arc` 1.0 (the old key reads) |
| pacing test: chapters with a climax | ≥ 80% | ≥ 95% |
| budget round: raised | (new) | 250 to 110 days before the treasury's year turns; cooldown 200 days |
| budget round: the vote (a clock the director places) | (new) | 190 to 40 days before the year turns and 56 to 250 days after the round opens; the turn is entered 28 days before it; the turn's decision waits 21 days |
| turn *a hole to fill* | (new) | the last year closed ≥ 1% of revenue short |
| turn *a deal across parties* | (new) | no party holds 60% of the posts that approve the budget |
| turn *a surplus to spend* | (new) | the last year closed ≥ 5% of revenue over |
| a change carries | (new) | half of the other members back the player (`body_support`) |
| a carried change, next year's budget | (new) | × 0.97 (cuts), × 1.03 (spend), × 1.01 (one amendment); a late budget × 0.98 |
| heat rise per week: budget line / its turn | (new) | 0.01 / 0.03 (start 0.2) |
| heat rise per week: favour debt reminder, plant organising | (new) | 0.005, 0.03 (heat feeds live tension; higher values held the tension curve up all year) |
| record of visible work at the round's exits | (first build: +0.04 carried, +0.02 a bill moved, +0.01 defeated, −0.01 nothing done) | none |
| a bill on the rates moved at the turn (hole, surplus) | (first build: a mark) | heat +0.05, no mark |

Thresholds were set on forty generated towns as play begins: every town has a one-party majority (so "no majority" never fires; a majority under 60% is 11 of 40), and most treasuries close 2% to 13% of revenue over.

## Evidence

Release build, two lab threads. "Before" is the branch head (8edba26, play note 20, which moved no pin and no bot rate against 2ac45b9); "after" is this branch merged with it. Run again after merging 12bfe17 (the seed cache, `repin.sh`, one ranking of worries, none of which moved a pin or a bot rate there): the same hashes and the same bot rates, `scripts/check.sh full` and `scripts/repin.sh --check` pass.

| Measure | Before | After |
|---|---|---|
| chapters with a climax, seeds 1 to 12 (6 × 30 turns, every decision answered) | 71 of 84 (with the fallback) | 84 of 84 |
| the same, seeds 13 to 24 (`PACING_FIRST_SEED=13`) | 69 of 84 (with the fallback) | 84 of 84 |
| the round's turn put to the player as the climax decision, each seed set (the test's player answers decisions and does nothing else, loses the first election in 11 seeds of 12 and sits out the years after) | n/a | in office at the turn: 16 sessions of 72, asked in 16. Out of office: 56, a climax decision in 4 (a plant closure led) |
| budget round raised once a budget year, 8 seeds × 4 years, idle | n/a | 32 of 32; no two consecutive years on one turn; surplus 14, row 11, deal 7 (a hole only in played towns) |
| passive against engaged script, first round, 8 seeds | n/a | exits differ in 8 of 8; the change carried in 7 (next year's budget +3.00% in six, +1.00% in one), defeated in 1 (a deal the room would not back) |
| `bots --seeds 16`: enforced gates | all pass | all pass |
| `bots --seeds 32`: enforced gates | all pass | all pass |
| best informed first-election wins (16 / 32 seeds) | 0.875 / 0.844 | 0.875 / 0.844 |
| canvass-only wins (16 / 32) | 0.312 / 0.344 | 0.312 / 0.344 |
| idle wins (16 / 32) | 0.062 / 0.062 | 0.062 / 0.062 |
| `ladder_learned`: explorer minus mixed share (16 / 32; margin 0.010) | +0.0158 / +0.0105 | +0.0134 / +0.0112 |
| `good_styles_vary` clauses a, b, c (16 seeds) | true, true, false | true, true, false |
| `good_styles_vary` clauses a, b, c (32 seeds) | true, true, true | true, true, true |
| every strategy's win rate | | 16 seeds: the same, but for explorer_coalition 0.812 → 0.875 (one race). 32 seeds: the same for all fourteen |
| `knowledge_matters`, `skill_gap` (reported, not enforced) | fail, fail | fail, fail |
| determinism (seed 2026, 80 turns): twice, and through save and load | one hash | one hash |
| native against WASM (seeds 777 and 31, medium, 60 turns) | equal | equal |
| `content doctor --seeds 0 --strict-prose` | | 0 errors, 0 warnings, no prose finding (33 unused-key notes, none in `arcs.ftl`) |
| a turn's cost, eight seeds × 300 turns (minimum of four runs each, world generation subtracted) | 4.36 ms | 4.19 ms (−3.8%; single seeds −15% to +8%, which is the machine's noise) |
| save size, seed 777, medium, 120 turns | 1,555,966 bytes | 1,558,881 bytes (+2,915, +0.19%) |

**The record at the exits.** The first build gave the round's exits a small record of visible work. On 32 seeds that alone took the canvass-only bot from 0.344 to 0.438 of first elections and the proxy bot from 0.406 to 0.531 (every bot but the idle one answers decisions, and the thin-record attack is steep near zero: audit fix 1 found the same at a canvass gain of 0.01). With the record effects out, and nothing else changed, every strategy's win rate at 32 seeds equals the branch head's and no mean share differs by more than 0.2 points. So no exit moves the record: it is built by acts, and a bill the turn drafts earns it when introduced.

**Pins** (`scripts/repin.sh`). The spine proof: all eight games moved, with the pack's arcs (`PINS`: the round each year, the rival's arc in each campaign, the other arcs where their causes hold; 89 to 185 events more a game) and with none (`BEFORE_ARCS`: storylines no longer run and the director opens no climax of its own; against 8edba26, kind by kind over the eight games, 38 storyline stages and 10 decisions fewer, 21 news items more). The proof's note says the rest. The state-shape fingerprint (save v28: `1cabea2fbd9fc802`), the v18 continuation golden (the rival storyline in it loads as a run of `arc.rival`) and the web asks fixture (ask ids one higher) were re-recorded; the geography hash and `tests/seeds.json` did not move.

## On the base as it stands (2026-11-25)

The table above compares this branch with the base it was built on. The base then took audit fixes 3, 5 and 8 (c55bd17: laws pay off in approval, honest ledgers), which move every game, and the "Why?" panels (356c8a9), which moved no pin. Measured again on the merged tree; the base after those fixes was not measured on its own here, so the rows below are this branch's figures, not a before and after.

| Measure | Merged tree |
|---|---|
| chapters where a turn is reached, seeds 1 to 12 / 13 to 24 | 84 of 84 / 84 of 84 (the base's own note: 84.5% and 82.1% with the fallback, bar held at 80%) |
| the round's turn put to the player, of 72 sessions each | seeds 1 to 12: in office 12, asked in 12; out of office 60, a climax decision in 2. Seeds 13 to 24: in office 16, asked in 16; out of office 56, a climax decision in 3 |
| budget round, 8 seeds × 4 years, idle | 32 of 32 raised, each to its turn; all four turns occur (a hole in seed 1's second year) |
| passive against engaged script, first round, 8 seeds | exits differ in 8 of 8; carried in 6, next year's budget changed in 5, a favour owed in 3 (7, 7 and 6 before the merge) |
| `bots --seeds 16`, enforced gates | pass, but for `good_styles_vary_b`: `explorer_coalition` is among the best in 10 of 16 worlds (0.625 against a limit of 0.6; 8 of 16 on the old base, 7 with this branch on it) |
| `bots --seeds 32`, enforced gates | all pass (`explorer_coalition` among the best in 16 of 32) |
| best informed / canvass-only / idle first-election wins (16 / 32) | 0.875 / 0.875; 0.188 / 0.250; 0.000 / 0.062 (the base's entry: canvass-only 0.19 / 0.22) |
| `ladder_learned` (16 / 32; margin 0.010) | +0.0184 / +0.0251 |
| `knowledge_matters`, `skill_gap` | fail, fail (as before) |
| determinism (seed 2026, 80 turns) | one hash, twice and through save and load |
| `content doctor --seeds 0 --strict-prose` | 0 errors, 0 warnings (33 unused-key notes) |
| save size, seed 777, medium, 120 turns | 1,564,461 bytes |
| pins | re-recorded with `scripts/repin.sh` after c55bd17 (the proof's sixteen digests, the shape fingerprint `336f9ea0b265bcff`, the v18 golden, the asks fixture); nothing moved after 356c8a9 |

**Awaiting the tuning pass.** `good_styles_vary_b` is report-only from this entry (a dated note in `bots/gates.rs`): one world over at 16 seeds, passing at 32. No number was tuned for it.

## Effect

Every session now turns on something the town's own books and room decide, and no climax is borrowed to fill a chapter. The first election's balance is where audit fix 1 left it: the round changes what a councillor does in a year, and by measurement nothing about who wins the first race. What it changes later in a career (a budget 3% larger or smaller each year a change carries, the opinions and favours the round leaves) is not measured by the bots, which stop at the first election; the sweep's twenty-year health checks pass. Open, and the largest: a player out of office reads of the vote and is not asked, so in those years the chapter's turn is reached without a decision of theirs. The old build opened a dilemma as the chapter's climax for such a player too (its 71 and 69 of 84 are these same careers); this one gives them none unless another arc leads. The pacing test now checks the two halves apart (reached in ≥ 95% of chapters; put to the player in ≥ 95% of the sessions where they hold a post) and prints the out-of-office count. The round needs a beat for a player without a vote (the gallery, the paper, a word with a member) before "a turn in every chapter" is true of a career that loses an election. Also open: a player who holds the proposing power gets no round yet.
