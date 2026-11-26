# Exchange scenes X3: stand-ins play the turn

**Date:** 2026-11-25
**Status:** done for the stand-ins, the grader and the doctor's dynamic scene lints; no save change, no content change, nothing tuned. Figures: the balance entry of the same day.

Design: `docs/design/EXCHANGE_SCENES.md` §8.2 and §9.4 (DEC-112); DEC-102 clauses 6 and 7; DEC-091.

## What was built

- **A stand-in plays a scene by its commands** (`mandate-sim/src/standin.rs`). `Turn::next` sends, before any other lever: `Go { location, then: Exchange { Begin } }` at the cast place, one `Exchange { Say { option } }` a round, and `Leave` only when nothing it would say is listed (after a refused command, then nothing more). Each line is chosen from `arcs::exchange::view`, the view the player has (redacted by the ledger); the scene's draws are its own keyed ones, so a stand-in rolls the chance it was shown and touches no other stream.
- **Play styles** (`standin::Play`): `absent` (never begins), `cautious` (the best expected change by weight: each range at its middle, a chance at its expected value; never a line that airs a secret or denies; a promise only when it beats the best other line by `PROMISE_MARGIN`, one point), `bold` (the widest swing by weight, stood or not; ties to the larger best case; never `own`, `pass` only when nothing else is listed). Each kind has its own: passive absent, engaged and attends cautious, contrarian bold. `Turn::styled`, `Career::styled`, `play_turn_styled`; a lab driver names one as `standin:engaged/bold` (`standin::parse_styled`, `styled_key`).
- **When it begins.** The engaged and contrarian stand-ins begin on the last turn that can (the next turn would start after the scene's last day), so the weeks before are for preparing. The one that attends the stops, and a passive one given a style, begin at once.
- **The skip (DEC-102).** The skip's proxy still never plays a scene, and a skip with no attending stand-in still leaves the chair empty (`skip.rs` unchanged; `exchange.rs`'s skip test holds). The stand-in that attends the stops now plays the scene where its skip stopped, and does not skip on while a scene at its seat is on offer or under way (`standin::scene_at`): a skip begun with a scene already on offer would not stop for it again.
- **The scorer's trace** (`arcs/score.rs`): `Trace::scenes`, each scene's lines (option id, stood) and outcome and result, read from `ArcExchange` and `ArcExchangeEnded`.
- **The grader** (`arcs grade`, `arcs_grade.rs`): for a template with a scene, four more stand-ins play each fork (`turn_players`: `engaged/absent`, `engaged/bold`, `passive/cautious`, `attends`). A `turn` section per arc: each player's scenes (held, outcome shares, won share, mean result, kinds of line said), exits and paths; how often exits differ (passive and engaged where the turn was played, engaged played or absent, cautious or bold, passive absent or turned up, engaged or attends); the outcomes nobody reached; the scenes the engaged stand-in was never offered; four report-only bars. Each row lists the scenes each player held, line by line.
- **The doctor's dynamic lints** (`doctor/arcs.rs`, `turn_findings`, with `--seeds`): `exchange-outcome-unvisited`, `exchange-outcome-lopsided` (over 70% of engaged scenes), `exchange-kind-dominant` (over 60% of engaged rounds), `exchange-prep-idle` (engaged no better in the room than attends), and `exchange-not-offered` (an empty chair for the engaged stand-in, which begins every scene it is offered). One fixture has every defect, one none.

## Tests

`mandate-save/tests/standin_scenes.rs` (5, seeds 1, 3, 31, 99): a stand-in begins, says a line every round and the scene ends, each line one the view listed, at the chance the view and the true view both show, the change applied inside what was shown; the styles part, and absent sends no scene command and ends `absent`; it chooses from the redacted view (with a blank ledger) and the same commands sent by hand give the same hash; the engaged one begins on the last turn it can; the one that attends the stops plays the debate where it stopped. `doctor::arcs::tests::each_dynamic_scene_lint_has_a_failing_fixture`. The lab's stand-in games (command log replays, blind to unlearned truths, the grader twice) and `worldgen/tests/arcs_x4.rs` pass unchanged.

## Departures from the design, each with its reason

1. **`promise_margin` is the stand-in's constant, not content** (§8.2 names it as if a parameter): it is a policy of the measurer's, not of the game.
2. **Cautious avoids a denial as well as an airing**: a denial "can unravel" (§9.1), which is the bold style's risk to take.
3. **The engaged stand-in begins on the last turn it can** (§8.2 says only "begins"): begun at once, it never used the preparation the stage still offers.
4. **The bots of `mandate-cli/src/bots` do not play scenes yet** (§8.3: `pass_only`, `first_line`, the ladder by mean result, the calibration table). They are outside this task; `standin::line` is the policy they would call.

## Not done, and open

- **A scene a player is never offered.** The plant's hall and the flood's meeting have a clock of 7 to 14 days and a governing turn is 14: when the stage opens late in a turn, the clock runs out before the player's next turn, and every player, engaged or not, gets the empty chair (the plant's hall in 1 of 7 worlds that reached it, seed 5). The fix is the content's clock or the step's rule, not this task's; `exchange-not-offered` reports it.
- **On the scene's own turn, preparation comes after.** The engaged stand-in begins first so the turn has the time; a preparation beat still on offer that turn is taken after the scene, too late.
- The repetition measure counts no scene lines or prompts.
- `EXTENDING.md` and `EXCHANGE_SCENES.md` §15/§16 are not updated for X3 (outside this task's files).
