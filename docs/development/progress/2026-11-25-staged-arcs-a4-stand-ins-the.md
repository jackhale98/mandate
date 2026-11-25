# Staged arcs A4: stand-ins, the scorer and the grader

**Date:** 2026-11-25
**Status:** done (stand-in players, the arc scorer, `arcs grade`, the doctor's arc lints, the skip's limits and its measurements, a start's rating, repetition across careers; no save change, no view, nothing tuned)

Design: `docs/design/STAGED_ARCS.md` §6 and §4.5 (DEC-109); DEC-102 clauses 6 and 7; DEC-100 item 1. What this build corrected in the design is marked **(A4)** there and listed in its §14. Figures: the balance entry of the same day.

The rule for this task (owner, 2026-11-25; `AGENT_CONTEXT.md` rule 11): build the measuring, not the judging. Every figure below is a command that gives the same output for the same seeds. No weight, threshold or simulation number was tuned. Every bar is printed beside its figure as data and fails nothing. The enforced checks are structural: coverage, determinism, and that a stand-in sends only commands.

## Stand-in players (`mandate-sim/src/standin.rs`)

- Four kinds, each a `Turn` that reads the state it is shown and returns `PlayerCommand`s: *passive* (nothing), *engaged* (every beat the arcs offer, for the arc's ally; then the skip proxy at the fullest directives), *contrarian* (the same for the opponent; declines the ally's asks), *attends* (skips the quiet weeks with the player's own directives and plays each stop as *engaged*).
- A stand-in plays for a `Seat`: a jurisdiction and the post held or run for there. Nothing names an office or reads `player.municipality`; `post_named` resolves a post from its constitution id for any tier.
- `step` plays a career turn by turn through `Engine::apply` alone (`Player`, `EndTurn`, `Skip`). The lab drives it with `Driver::StandIn` (`standin:engaged`).
- Tests (`lab/standin_games.rs`): a game's command log replays to the same state for all four kinds; perturbing truths the player has not learned changes no command; the same commands under an omniscient ledger leave the same world.

## The scorer (`mandate-sim/src/arcs/score.rs`)

- A `Watch` follows one template at one jurisdiction through a played game and leaves a `Trace` (stages entered, what was seen and offered, heat, the exit).
- Five terms, each a type with its parts and its value by the design's own formula: `Agency`, `Escalation`, `Visibility`, `Pacing`, `variety`. `Coverage` says who visited each stage and exit. `evp` is the design's product for ordering candidates. No weighted total.
- `probe` says whether a template's `raise` holds and its cast can be bound (the doctor's `arc-cast-fails`).

## The grader (`mandate-cli arcs grade --seeds N [--arc id] [--bots] [--text]`)

- Per template and seed: plant the cause where the lab can (`Intervention::Disaster`, `PlantClosure`), else wait up to 14 months for the world to raise it; fork the turn it is raised; play the three arc stand-ins up to 130 weeks.
- Prints the five terms, each stand-in's own E, V and P, exits by stand-in, coverage with who visited what, how often the passive and engaged exits differ, the worlds under the design's floors, and the bars of §6.9 beside the figures.
- Coverage is enforced from 16 seeds: an unvisited stage or exit fails the command unless `arcs_grade::UNVISITED` lists it with its reason.

## The doctor's arc lints (`content doctor`, `doctor/arcs.rs`)

- Static, each a warning with a failing fixture: `arc-no-passive-exit`, `arc-no-lever`, `arc-nothing-to-do`, `arc-heat-unreachable`, `arc-exit-flavour`, `arc-names-office`, `arc-role-uncast`. On the shipped pack: six stages with no lever, five with nothing to do, two exits that only change opinions.
- Dynamic, with `--seeds`: `arc-never-raised`, `arc-cast-fails`, `arc-stage-unvisited`, `arc-exit-unvisited`, `arc-too-long`, read from the grader's worlds.
- `arc-no-drift` and `arc-tier` are not here: content validation already refuses both.

## A skip's limits (DEC-102 clause 7)

Each is a mechanism with its switch in `content/base/act_flow.jsonc`; outcomes change for skipped play only.

- **It learns nothing new** (`proxy.learns`: false). What the proxy's acts would put in the notebook during a skipped turn is not kept. What the end of a turn makes public still reaches the player.
- **An ask on offer waits** (`request_due.offers`: true). It stops the skip as its last day to answer nears; the proxy never answers it.
- **An arc stops a skip when it is the player's to meet** (a new interrupt, `arc_stage`, with a rule for material arcs and one for standing arcs). As shipped: a material arc when it is raised and at each stage once a beat of it is the player's to take; a standing arc only at its turn, once the player can act in it; a stage with nothing to do, and an exit, never. The broad rule the decision first named (every stage and exit of every arc) is the same mechanism with other content, and is measured beside it.
- **It makes no promises and files for no post.** Already so; now a test (`the_proxy_makes_no_promises_and_files_for_nothing`).
- The `directed` probe of the bots report now plays every turn as a turn of a real skip, so each limit is the probe's too.

## Measurements

- **Bots report** (`bots --seeds N`): a `skips` section. Skip-only against the single-action routines at the first election; re-election rates (the election after the first, for those who won it) for skip-only, the engaged newcomer (`mixed`) and the best informed style; a player who attends the stops against the same player every turn (the best informed style, and the engaged stand-in). `proxy_not_optimal` is as it was.
- **`act-length`**: in the mode with skips, how long a skip runs (turns and weeks), how often an interrupt stops one per skipped year, and the stops by interrupt.
- **`lab rate-start --seed S [--post id] [--seeds-per-start N] [--after-months M] [--style s]`** (DEC-102 clause 6): the race played N times by the engaged stand-in and N times by an informed style, each on a fork under its own luck (`standin::reluck` re-keys a fork's draws; luck 0 is the game's own). Prints the win rate with the two or three factors of the vote model's decomposition that drive it, as factor ids and Fluent keys. With `--post` the same character is first played on (by default until their first election is over) and then rated for that post.
- **`lab repetition --careers 5 --seeds N [--years Y]`** (DEC-100 item 1): a returning player's consecutive careers, each in a new world, and the share of each career's dilemmas, arc stages, asks and conversation lines an earlier one showed.

## Not done, and open

- The stand-ins read the engine's list of beats, not the redacted arc view (A3 is a protocol type); they agree while no run is hidden.
- The proxy still answers minor and significant decisions by the directives, an arc's among them (`proxy.answers`, data). DEC-102 clause 7 says a skip never makes one of the player's choices; whether that reserves an arc's decisions is the owner's call.
- DEC-102 clause 7 still reads "every stage of a staged arc stops it"; the narrowed rule shipped here needs the decision's text amended.
- The engaged stand-in talks only where an arc or an ask sends it, so a career shows two to four conversation lines: the repetition figure for lines is thin until a stand-in that explores exists.
- `arc.favour_debt` is never raised under the stand-ins (the debt comes from an act of the player's, and the lab has no intervention that plants one).

## Evidence

- `arcs grade --seeds 16`: 87 s in the dev build; the same output twice; coverage complete for the flood and the plant closure, and every unvisited stage and exit of the other three listed with its reason.
- `lab rate-start --seed 5 --seeds-per-start 8`: the same rating twice; works for the opening and for the town's executive post eleven months on (`--post`, an id read from the constitution).
- `lab repetition --careers 5 --seeds 4`: 77% of career 3 seen before (target under 40%).
- `bots --seeds 16`: every strategy and every gate as before this task, to the last digit; the `skips` section printed.
- `act-length --seeds 4 --years 8`: a skip runs 2.26 turns as shipped (4.68 before), and is stopped 11.6 times a skipped year; under the broad arc rule 13.1.
- `determinism --seed 2026 --turns 80` and `skip-determinism`: one hash twice and through save and load. On the content of before this task the new build gives the old hash for a game without a skip: only skipped play moved.
- Pins: `scripts/repin.sh` re-recorded two test seeds whose first skip no longer runs as far (`law_lapses_in_first_skip` 15 to 30, `promises_reachable_by_skipping` 8 to 15). The spine proof, the state-shape fingerprint, the fixtures and the geography hold.

## For whoever reads the figures

- An ask on offer is what stops most skips (543 of 963 stops), and it takes Act 1 with skips from 155 to 224 minutes at the lab's assumptions (REQ-008: 90 to 150). The arc rule adds 0.4 stops a skipped year as shipped and 2.0 under the broad rule.
- The `directed` probe's action mix is no longer counted: its commands are inside a skip, which logs itself and not them.
- `docs/development/EXTENDING.md` ("An interrupt") does not yet list `arc_stage` or `request_due.offers`.
