# Exchange scenes X3: the turn measured (figures only)

**Date:** 2026-11-25
**Area:** the stand-ins (`mandate-sim/src/standin.rs`), the grader (`arcs grade`), `lab rate-start`, `lab repetition`.

## Parameter and change

A mechanism: the stand-ins play exchange scenes through their commands, by a play style (absent, cautious, bold), and the grader measures the turn. **No content or simulation number was tuned.** One number is the stand-in's own: `PROMISE_MARGIN`, a point of lean by weight. Only stand-in play moves; the game, the skip's proxy and every pin are as they were.

## Evidence

`mandate-cli arcs grade --seeds 16`, `MANDATE_LAB_THREADS=2`. Before: ad6c492. After: this task. The same output twice (`cmp`). 56 s before, 88 s after (four more stand-ins on the three arcs with a scene).

| Arc | Median agency | Exits differ (passive, engaged) | Engaged exits | Contrarian exits |
|---|---|---|---|---|
| rival | 0.104 → 0.402 | 7 → 8 of 16 | won 0.625 → 0.688 | won 0.688 → 0.750 |
| plant closure | 0.005 → 0.005 | 3 → 4 of 14 | closed 0.071 → 0 | closed 0.071 → 0 |
| flood | 0.605 → 0.606 | 11 of 16, same | same | defended 0.688 → 0.813 |
| budget round, favour debt | unchanged (no scene) | same | same | same |

The turn, after (each stand-in's scenes; result: the weighted change in lean in thousandths):

| Arc | Stand-in | Scenes held | won / split / lost / absent | Mean result | Arc exits |
|---|---|---|---|---|---|
| rival (debate) | passive (absent) | 16 | 0 / 0 / 0 / 1.0 | −337 | lost 0.81 |
| | passive/cautious (turns up, no preparation) | 16 | 0.25 / 0.75 / 0 / 0 | +61 | lost 0.81 |
| | engaged (cautious) | 16 | 0.63 / 0.38 / 0 / 0 | +132 | won 0.69 |
| | engaged/bold | 16 | 0.63 / 0.38 / 0 / 0 | +124 | won 0.69 |
| | engaged/absent | 16 | absent 1.0 | −289 | won 0.63 |
| | contrarian (bold) | 16 | 0.75 / 0.25 / 0 / 0 | +138 | won 0.75 |
| | attends | 16 | 0.56 / 0.44 / 0 / 0 | +107 | won 0.69 |
| plant (hall) | passive | 8 | absent 1.0 | −189 | answered 0.43, closed 0.14, scare 0.43 |
| | passive/cautious | 8 | 0 / 0.25 / 0.50 / 0.25 | −109 | same as passive |
| | engaged | 7 | 0 / 0.29 / 0.57 / 0.14 | −135 | answered 0.43, reprieved 0.14, scare 0.43 |
| | engaged/bold | 7 | 0 / 0.29 / 0.57 / 0.14 | −203 | same as engaged |
| | engaged/absent | 7 | absent 1.0 | −320 | adds closed 0.07 |
| | attends | 7 | 0 / 0.29 / 0.57 / 0.14 | −118 | adds closed 0.07 |
| flood (meeting) | passive | 5 | absent 1.0 | −208 | hollowed 0.69 |
| | passive/cautious | 5 | split 1.0 | −26 | hollowed 0.69 |
| | every engaged style, attends | 0 | the meeting is never reached | | defended 0.69 |

Exits differ, after: engaged played or absent: rival 1 of 16, plant 1 of 14, flood 0 of 16; cautious or bold: 0 everywhere; passive absent or turned up: 0 everywhere; passive and engaged where the turn was played: rival 8 of 16, plant 3 of 6.

Report-only bars of the turn: `scene_outcome_lopsided` (≤ 0.7): rival 0.625 pass, plant 0.571 pass. `scene_kind_dominant` (≤ 0.6): rival `promise` 0.875 **fail**, plant `offer` 0.333 pass. `scene_prep_pays` (engaged above attends): rival +24 pass, plant −17 **fail**. `turn_changes_the_exit` (≥ 0.6): rival 0.063, plant 0.071, flood 0 **fail**. Rows in `TUNING_QUEUE.md`.

**`lab rate-start --seed 5 --seeds-per-start 8`**: the engaged stand-in wins 7 → 8 of 8 (share 0.635 → 0.653; it now plays the debate); the informed style 8 of 8 (0.738) as before; the rating 15 → 16 of 16.

**`bots --seeds 16`**: every strategy, every gate and the ladder are as before to the last digit (no bot plays a scene; only the `skips` section moves). The stand-ins there, first election: engaged every turn 0.94, share 0.697 → 1.0, 0.718; attending the stops 0.75, 0.675 → 1.0, 0.719. The gap between them, 2.2 points of share, is now −0.1.

**`lab repetition --careers 5 --seeds 4`**: career 3 seen before 0.768 → 0.828 (target under 0.4; report-only already); career 1 shows 38.5 → 44.75 items (arc stages 14.0 → 15.75, dilemmas 9.0 → 12.0).

## Effect

- The debate is played in every world that holds it, and a played debate is won or split, never lost, by every style; the empty chair loses about 0.3 of lean by weight. The race's exit moves in 1 world of 16: the room's change is real but rarely the margin.
- The cautious line in the debate is nearly always a promise (the menu's three are the best lines a round), so cautious and bold say much the same where every line is certain and exact, as in most worlds.
- The plant's hall is lost or split by everyone who turns up, and the steward's answer does not hold off a closure the economy decides; preparation helps the rival's room and not the plant's.
- The flood's engaged stand-ins defend the town before the meeting comes, so only the passive ones hold it.
- The plant's hall was never offered to any player in 1 of 7 worlds: its clock (7 to 14 days) ran out inside one 14-day governing turn. A content or step fault, not a number; the doctor's `exchange-not-offered` reports it.
