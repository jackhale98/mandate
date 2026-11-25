# Stand-ins, skips and start ratings measured

**Date:** 2026-11-25
**Area:** arcs (the scorer and the grader), the act flow (a skip's limits), the lab (start ratings, repetition)

Figures only. Staged arcs A4 built the measuring and tuned nothing (`AGENT_CONTEXT.md` rule 11): every bar below is printed by its command beside the figure and enforces nothing. Small worlds, the dev build, seeds 1 to N.

## Parameter and change

No simulation number changed. Three switches of a skip are new in `content/base/act_flow.jsonc`:

| Parameter | Before | After |
|---|---|---|
| `proxy.learns` | (the proxy's acts filled the notebook) | false |
| `interrupts[request_due].offers` | (an offer never stopped a skip, and lapsed) | true |
| `interrupts[arc_stage]` | (none) | material: when raised, and each stage once a beat is the player's to take; standing: its turn, once the player can act; a stage with nothing to do and an exit: never |

## Evidence

### The arc grader (`arcs grade --seeds 16`; 87 s; the same output twice)

Median of each term; E, V and P per stand-in on its own trace; coverage is the share of the template's stages and exits visited.

| Arc | Worlds | A | E | V | P | N | Stand-in | E | V | P | Coverage | Exits reached |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `arc.flood` | 16 planted | 0.605 | 0.633 | 0.750 | 0.500 | 0.000 | passive | 0.633 | 0.750 | 0.500 | 1.000 | hollowed 62%, defended 31%, rebuilt 6% |
| | | | | | | | engaged | 0.267 | 0.500 | 0.500 | 0.714 | defended 69%, rebuilt 31% |
| | | | | | | | contrarian | 0.262 | 0.500 | 0.500 | 0.714 | defended 62%, rebuilt 38% |
| `arc.budget_round` | 16 raised | 0.804 | 0.420 | 0.750 | 0.750 | 0.000 | passive | 0.420 | 0.750 | 0.750 | 0.600 | passed 100% |
| | | | | | | | engaged | 0.395 | 0.750 | 0.750 | 0.700 | carried 81%, defeated 19% |
| | | | | | | | contrarian | 0.380 | 0.750 | 0.750 | 0.600 | passed 100% |
| `arc.plant_closure` | 13 planted of 16 | 0.007 | 0.400 | 0.667 | 0.250 | 0.000 | passive | 0.400 | 0.667 | 0.250 | 0.833 | gone 62%, resolution 31%, answered 8% |
| | | | | | | | engaged | 0.300 | 0.667 | 0.250 | 0.833 | gone 38%, answered 31%, resolution 31% |
| | | | | | | | contrarian | 0.790 | 0.500 | 0.250 | 1.000 | gone 46%, answered 38%, resolution 15% |
| `arc.rival` | 16 raised | 0.005 | 0.590 | 0.500 | 0.250 | 0.000 | passive | 0.590 | 0.500 | 0.500 | 0.571 | lost 100% |
| | | | | | | | engaged | 0.510 | 0.400 | 0.250 | 0.857 | lost 56%, won 44% |
| | | | | | | | contrarian | 0.790 | 0.500 | 0.250 | 0.714 | lost 56%, won 44% |
| `arc.favour_debt` | 0 of 16 | | | | | | | | | | 0.000 | (its cast fails: the player owes nobody) |

Coverage by all stand-ins together: flood 1.000, plant closure 1.000, budget round 0.800 (`turn_hole`, `late` unvisited), rival 0.857 (`cooled`), favour debt 0. Each unvisited one is listed with its reason in `arcs_grade::UNVISITED`.

The design's bars (§6.9), report-only:

| Bar | Target | Flood | Budget round | Plant closure | Rival |
|---|---|---|---|---|---|
| passive and engaged exits differ | ≥ 60% of worlds | 62% (10 of 16) | 100% | 23% (3 of 13) | 44% (7 of 16) |
| median A | ≥ 0.4 | 0.605 | 0.804 | 0.007 | 0.005 |
| largest exit's share of engaged runs | ≤ 70% | 69% | 81% | 38% | 56% |
| engaged median turns to the turn | ≤ a chapter (26 turns) | never reached (acting keeps it from the meeting) | 5 | 3 | 3 |

### Skip-only play (`bots --seeds 16`)

| Figure | Target (DEC-102 clause 7) | Measured |
|---|---|---|
| skip-only first-election win rate | no higher than the best single-action routine's | 56.2% (share 0.549), against `asks_only` 43.8% and `canvass` 18.8% |
| skip-only re-election (of those who won the first) | about 40 to 45% | 78% (7 of 9) |
| the engaged newcomer's re-election (`mixed`) | about 60 to 70% | 85% (11 of 13) |
| the best informed style's re-election (`explorer_coalition`) | about 85% for a skilled player | 100% (14 of 14) |
| held the seat through two elections | | skip-only 44%, newcomer 69%, best informed 88% |

`proxy_not_optimal` is as it was and passes (the skip-only share is 0.110 under the informed median).

### Attending the stops against playing every turn (first election, 16 seeds)

| Player | Every turn: wins, share | Attends: wins, share | Gap: wins, share | Turns played | Skips, median length |
|---|---|---|---|---|---|
| best informed (`explorer_coalition`) | 87.5%, 0.669 | 75.0%, 0.661 | 12.5 points, 0.8 points | 20.3 of 26 | 3.1 a career, 2 turns |
| engaged stand-in | 62.5%, 0.563 | 56.2%, 0.558 | 6.2 points, 0.5 points | 14.3 of 26 | 6.9 a career, 1 turn |

Target: within a few points (the decision gives no number). The share gap is under a point; the win-rate gap is two careers and one career of sixteen.

### How long a skip runs (`act-length --seeds 4 --years 8`: 16 careers, the informed styles, skipping every stretch out of a campaign)

| Rule | Skips | Turns a skip (mean, median) | Weeks a skip (mean) | Interrupts a skipped year | Act 1 with skips, median minutes |
|---|---|---|---|---|---|
| before this task | 574 | 4.68, not recorded | 9.4 | about 5.6 (skips over skipped years; the old build did not record why each stopped) | 150.5 |
| as shipped (an offer stops; arcs by the narrowed rule) | 963 | 2.26, 2 | 4.5 | 11.6 | 224.5 |
| the broad arc rule (every stage and exit of every arc) | 1,047 | 1.99, 1 | 4.0 | 13.1 | 241.0 |
| no arc stops (an offer stops) | 931 | 2.34, 2 | 4.7 | 11.1 | 222.0 |
| arcs by the narrowed rule, an offer does not stop | 601 | 4.41, 3 | 8.8 | 5.9 | 155.0 |

Stops as shipped, by the interrupt that named them: `request_due` 543, `sponsored_vote` 264, `decision` 106, `arc_stage` 27, `election` 23. Under the broad rule `arc_stage` names 179. Without skips Act 1 is 434 minutes at the same assumptions (2 minutes a played turn, 1 a skip). Target: a floor on a typical skip (the decision gives no number); REQ-008 asks 90 to 150 minutes for Act 1 with skips.

### What each limit of a skip changed

| Limit | Bot figures (`directed`, 16 seeds) | Skip lengths |
|---|---|---|
| it learns nothing new | none: 56.2% and share 0.549 before and after, to the last digit (the proxy works its own ward, which the start profile already knows). In a 12-turn skip the proxy's acts would have taught 40 ledger entries; now 0 | none |
| an ask on offer stops it | none (a skip-only player skips on, and the offer lapses as before) | stops double: 5.9 to 11.6 a skipped year; Act 1 with skips 155 to 224 minutes |
| an arc stops it (narrowed rule) | none | + 0.4 stops a skipped year (11.1 to 11.6); the broad rule adds 2.0 |
| no promises, no filing | none (already so) | none |

The strategies of the tournament other than `directed` are unchanged to the last digit, and so is every gate of the ladder.

### A start's rating (`lab rate-start --seed S --seeds-per-start 8`; the same rating twice)

Sixteen plays a race: eight by the engaged stand-in, eight by `explorer_coalition`. A factor's figure is its mean share of the margin between the player and the other of the first two, in share points; above zero it worked for the player.

| World | Race | Rating | Engaged stand-in | Informed style | Facts |
|---|---|---|---|---|---|
| seed 3 | the opening (their seat) | 87.5% (14 of 16) | 75.0% | 100% | `factor-party` −0.131, `factor-record` +0.122, `factor-campaign` +0.115 |
| seed 5 | the opening | 81.3% (13 of 16) | 62.5% | 100% | `factor-record` +0.377, `factor-issues` −0.319, `factor-valence` +0.062 |
| seed 11 | the opening | 50.0% (8 of 16) | 12.5% | 87.5% | `factor-campaign` +0.098, `factor-party` −0.065, `factor-issues` +0.063 |
| seed 3 | the town's executive, 11 months on | 6.3% (1 of 16) | 0% | 12.5% | on the ballot in 1 play of 8 (the party passed them over in the rest) |
| seed 5 | the town's executive, 11 months on | 43.8% (7 of 16) | 25.0% | 62.5% | `factor-issues` −0.243, `factor-party` +0.134, `factor-record` +0.105 |
| seed 11 | the town's executive, 11 months on | 12.5% (2 of 16) | 12.5% | 12.5% | on the ballot in 1 play of 8 |

### Repetition across careers (`lab repetition --careers 5 --seeds 4`; six years a career, the engaged stand-in)

The share of a career's content an earlier career of the same player showed (mean of four players).

| Career | All | Dilemmas | Arc stages | Asks | Conversation lines | Items shown |
|---|---|---|---|---|---|---|
| 1 | 0% | 0% | 0% | 0% | 0% | 27 |
| 2 | 61% | 52% | 65% | 70% | 64% | 40 |
| 3 | 77% | 69% | 81% | 78% | 88% | 43 |
| 4 | 90% | 89% | 93% | 85% | 100% | 39 |
| 5 | 94% | 91% | 93% | 98% | 100% | 36 |

Target (DEC-100 item 1): under 40% in career 3. Measured: 77%. The stand-in talks only where an arc or an ask sends it, so a career shows two to four conversation lines; that column is thin.

## Effect

Nothing is acted on before the tuning pass at the end of M6. For the queue (`TUNING_QUEUE.md` is the lead's): skip-only wins 56% of first elections against a target of the best single routine's 44%; its re-election is 78% against 40 to 45%; the engaged newcomer's is 85% against 60 to 70%; attending the stops costs under a point of share and 6 to 12 points of win rate; a skip runs 2 turns and is stopped 11.6 times a skipped year, most often by an ask on offer, and Act 1 with skips is 224 minutes against REQ-008's 90 to 150; career 3 repeats 77% against under 40%; plant closure and the rival arc are under the grader's agency bars; the budget round's `carried` takes 81% of engaged runs against at most 70%.
