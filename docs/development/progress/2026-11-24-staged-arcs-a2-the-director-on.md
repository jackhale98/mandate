# Staged arcs A2: the director on arcs

**Date:** 2026-11-24
**Status:** done (the director on arcs, storylines folded, the budget round and the rival as standing arcs, save v28; no arc view, no stand-ins or grader, no fuses: tasks A3 to A5)

Design: `docs/design/STAGED_ARCS.md` (DEC-109); what this build corrected in it is marked **(A2)** there and listed in its §13. Architecture: OVERVIEW §4.3, DATA_MODEL §6.1. Authoring: EXTENDING, "Adding an arc".

## The director times arcs and nothing else

- **The lead** (`narrative/director/arcs.rs`). Each chapter has one lead arc, picked by the existing pick among the runs that can still turn in it: a material arc (theme `crisis`) first, then the standing arc of the chapter's kind, then the rest. The lead's turn, the first stage marked `climax` the run enters, is the chapter's climax, counted the day it is reached. If the lead never turns, the chapter takes the first turn reached inside it as it closes.
- **A director-owned clock** (`clock.placed`). The run's own keyed draw, taken inside the chapter's climax window where the content's range allows. It reads the chapter's dates and the storyteller profile, which are the same with the spotlight on or off.
- **When a decision beat is presented** (`narrative/offered.rs`). A run lists the beat in `open`; the director reads `ArcsState` and opens the decision within the chapter's budgets and `max_pending`. The lead's decisions and any under a clock are significant, in any phase; a background arc's is a minor beat. The request `OpenDecision` is gone.
- **What went.** The climax fallback, the forced climax peak, climax memory by template, and every dilemma as a climax on its own. The pacing cycle's climax peak begins when the lead's turn is presented.
- **What it cannot do.** Raise a run, take an edge, pick an exit, change an effect. With the spotlight off every run ends as it does when the player lets everything pass.

## Storylines are arcs

`storylines.jsonc`, `StorylineDef`, `StorylineRun`, the director's `runs.rs` and the storyline parts of the narrative state, the explain workbench, the doctor, the corpus and the story metrics are gone. The three templates are `arcs/rival.jsonc`, `arcs/favour_debt.jsonc` and `arcs/plant_closure.jsonc`, with their stage ids kept and three exits each (a storyline had none). `storyline_only` is `arc_only` (the old spelling reads). The story sheet lists arcs where it listed storylines, from the same view fields (`StoryView.storylines`, `DecisionView.storyline_key`), so the web client needed no change beyond the designer's kind list.

**Migration (v27 → v28).** A running storyline becomes a run of its arc at the stage of the same id (the last it reached), in the player's town, with its cast, the marks its answered decisions set, and a decision still waiting on the player as the run's open beat. Tested on a real save (`v27_storylines.mndt`, written by 2ac45b9: the favour debt at its reminder, the rival's offer pending): both load at their stages, the offer is answered or let pass, the rival's arc ends at the exit the race decides, and the game is deterministic from there. The v25 and v26 fixtures load through the new version.

## Standing arcs

- **The budget round** (`arcs/budget_round.jsonc`, `standing: "session"`). Raised by the budget calendar once a budget year (`budget_year`: 250 to 110 days before the treasury's year turns). Estimates, the line, one of four turns, four exits. Roles bind through powers: whoever proposes the budget, another holder of the power to approve it, the place the proposer's post sits, the neighbourhood that thinks least of the player.
- **The four turns and the signals that pick each:** a hole to fill (the treasury closed its last year at least 1% short of its revenue), a deal across parties (no party holds 60% of the posts that approve the budget), a surplus to spend (it closed at least 5% over), a row over one line (otherwise). Never the turn of the last round (`arc_last_turn`).
- **Agency.** A change the player moves at the turn carries if half of the others with a vote would back them (`body_support`: their party, members who think well of them, members who owe them a favour). A carried change is voted into the next budget and applied when the year turns (`Effect::Budget`, `finance::vote_budget`).
- **The rival** (`arcs/rival.jsonc`, `standing: "campaign"`): the campaign's arc. A campaign's climax stays the election.

## Evidence

- **Pacing** (`narrative::pacing_stays_within_budgets_and_cooldowns_hold`, the bar now 95%): a turn is reached in 84 of 84 chapters on seeds 1 to 12 (71 of 84 before, with the fallback) and 84 of 84 on seeds 13 to 24 (`PACING_FIRST_SEED=13`; 69 before). Every budget and cooldown in the test holds. **Reached is not the same as put to the player.** The test's player loses the first election in 11 seeds of 12; on each seed set the round turned in 72 sessions, the player held a post in 16 and was asked at climax weight in all 16, and was out of office in 56, with a climax decision in 4 (a plant closure led). The test asserts both halves (reached ≥ 95% of chapters; asked ≥ 95% of sessions in office).
- **Spotlight on against off** (`narrative::the_spotlight_never_changes_endogenous_outcomes`): four seeds, three years; the arcs' runs (stage, heat, marks, clock, turn), their exits, the asks, and every domain compared before are identical.
- **The round** (`mandate-worldgen/tests/budget_round.rs`): raised once a budget year in the player's town in 8 seeds of 8 over four years, each round reaching its turn, no two consecutive years on the same turn; three variants in the 32 idle rounds (surplus 14, row 11, deal 7; the hole appears in played towns). Two scripts on the first round of each of the eight seeds: the exits differ in 8 of 8; the passive player's budget *passed* in all; the engaged player's change *carried* in 7 (next year's budget +3.00% in six, +1.00% in one) and was *defeated* in 1. In seed 1 the engaged player read the estimates, asked the proposer, kept the colleague's ask, pressed, moved the spending, and it carried: next year's budget is 3.00% larger, the colleague's opinion is higher and they owe a favour, and the ask kept is on the record of visible work (no exit moves the record: see the balance entry). Every beat is reached in quick mode and through Places, and the two reach the same beats.
- **Beside a material arc:** a planted flood leads the chapter (its meeting is the climax) while the round runs and turns in the same chapter.
- **The flood's A1 tests** pass with their assertions unchanged, on a pack with the flood arc alone (their helpers assumed one run in a town).

(The proof's pins, the bots, the doctor, determinism, WASM parity and the timings are in the balance entry of the same day.)

**On the base as it stands (2026-11-25).** The figures above were measured before the base took audit fixes 3, 5 and 8. On the merged tree: a turn is reached in 84 of 84 chapters on both seed sets; the round's turn is put to the player in every session where they hold a post (12 of 12 and 16 of 16) and a climax decision reaches a player out of office in 2 of 60 and 3 of 56; the two scripts end apart in 8 seeds of 8 (carried in 6, a budget changed in 5). `scripts/check.sh affected` passes. One bot gate is report-only and waits for the tuning pass: `good_styles_vary_b` (10 of 16 worlds against a limit of 9.6; passes at 32 seeds). The balance entry has the table.

## What moved outside the task's list, and why

- `finance/mod.rs`, `scheduler.rs`, `systems.rs`: `Effect::Budget` and its applier (`finance.effects`, `vote_budget`, `Treasury::voted`), `finance::treasury` and `budget_year_turns`.
- `condition.rs` (content and sim): seven conditions (`budget_year`, `budget_balance`, `body_majority`, `body_support`, `arc_last_turn`, `last_race`, `sponsors_bill`).
- `media.rs`: `audit_story` accepts an arc's story as town news (A1 added the coverage, not the audit; the round's press beat runs in every game).
- `mandate-cli/src/lab/seed_reqs.rs`: the flood requirement counts the flood's own run. `proof.rs`: pins, and its notes on what moved them.
- `explain.rs`, `spine/prune.rs`, `skip.rs`, `mandate-protocol` (`narrative.rs`, `views.rs`, `dev.rs`), `mandate-cli` (`story.rs`, `corpus.rs`, `sweep.rs`, the doctor), `web/src/dev` (the designer's kinds, the corpus order): storylines removed.
- Tests: `characters.rs::betrayal_is_remembered…` runs without arc content (six idle years of the round move a colleague's opinion on their own account; the test is about memories alone). The save crate's synthetic older payloads drop `arcs` (an older save holds no run; every played game has one now). `lab/arcs_games.rs` counts the flood's runs, not every arc's.

## What is left

- **A player out of office has no turn of their own.** The round runs and turns in their town, they read of it, and nothing is put to them: its decisions need the power to approve the budget. The old build gave them a dilemma as a climax. The round needs a beat for a player without a vote.
- A player who holds the proposing power gets no round: the proposer role cannot be the player (the executive's side of the round is not written).
- The round is `tiers: ["local"]`: the bills its turn drafts are a town's.
- Still inert: `opportunity` (no opportunity arc exists), `goals`, `answers`, `follow_up`, `legacy_key`, `hidden`, `teaches`, `sides`.
- A3 (the arc view), A4 (stand-ins, the grader, the doctor's dynamic lints), A5 (the fuses), A6 (the plant's review replaces `arc.plant_closure`).
