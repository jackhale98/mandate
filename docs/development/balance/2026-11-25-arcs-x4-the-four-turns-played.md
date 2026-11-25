# Arcs X4: the four turns played (figures only)

**Date:** 2026-11-25
**Area:** arcs (`content/base/arcs/{rival,favour_debt,plant_closure,flood}.jsonc`); the lab's planted causes (`arcs_grade::plant`).

## Parameter and change

Content structure and the smallest engine changes it needed (the progress entry of the same day lists them). **No simulation number was tuned.** New content numbers are placeholders for the tuning pass: the plant's relief (0.08 for the steward's short weeks, 0.08 for the manager's case, each for 12 months; −0.04 for two months when the line stops), the rival's lead-up from 40 days before the vote (the showdown stays at 30 and the debate 14 to 21 days on, as X1 built it; the design's 45 and 21 left the truce under a week of an eight-week campaign, and the stand-ins never took it), the favour's call held 14 to 21 days.

One engine rule moved with it: a run in a hidden stage leads no chapter. Without it a plant on short time (common at the start of play) outranked the rival and the budget round for the lead and silenced the debate. The lab's planted plant review is a fall in demand for the largest plant's industry of 0.85, 0.72 or 0.6 by seed, for 30 months (the lab's numbers).

No stand-in or bot plays a scene yet (task X3): every graded meeting and debate is an empty chair. The grader therefore measures the levers around the turns, not the turns.

## Evidence

`mandate-cli arcs grade --seeds 16`. Before: this branch with X1 merged (d2d44b5 + the exchange core). After: ed96b16 (`claude/bold-brown-shyeug` merged).

| Arc | Worlds | Median agency | Exits differ (passive, engaged) | Coverage | Engaged exits |
|---|---|---|---|---|---|
| rival | 16 → 16 | 0.601 → 0.601 | 0.75 → 0.69 | `cooled` listed → same (`pact` visited) | won 0.69, lost 0.31 (passive: lost 1.0) |
| favour debt | 0 (cast fails: no debt) → 16 (debt planted) | none → 0.004 | none → 0.31 | none → complete but `deferred` (listed) | repaid 0.25, burned 0.06, settled 0.69 (passive: settled 1.0); the call came in 1 of 16 |
| plant closure | 13 → 14 | 0.007 → 0.005 | 0.23 → 0.14 | complete → complete | reprieved 0.14, answered 0.21, closed 0.21, scare 0.43 (passive: answered 0.29, closed 0.29, scare 0.43) |
| flood | 16 → 16 | 0.606 → 0.606 | 0.75 → 0.75 | complete → complete | defended 0.81, rebuilt 0.19 (passive: hollowed 0.75) |
| budget round | unchanged | 0.804 | 0.94 | `late` listed | |

## Effect

- The favour debt starts in every planted world; its turn needs a bill a yes would cost, which came once in 16 (the bind is read from live state, so a cost-free bill is paid by an ordinary vote and the run ends at *repaid* with no turn).
- The plant's levers reach the plant: in the middling worlds the steward's and the manager's relief lifts some notices the passive player meets closed; the grave ones close whatever is done, as the design intends; the mild ones end in *scare* for everyone.
- The flood's and the rival's figures are where they were: their engaged stand-ins already took the levers that decide their exits, and the turn itself is an empty chair for every stand-in until X3.

**`mandate-cli bots --seeds 16`** (first election; win rate, mean share; before is X1's "after" in its balance entry): idle 0.00/0.315, media 0.19/0.373, canvass 0.50/0.439, asks_only 0.75 → 0.69 /0.571, proxy 0.75/0.571, directed 0.88/0.624, corrupt 0.94/0.696, legislator 0.94/0.706, mixed 0.94/0.714, explorer 0.94/0.733. Every enforced gate passes (`idle_cap`, `effort_matters`, `exploring_alone_never_wins`, `ladder_rote`, `ladder_learned`, `corrupt_not_dominant`, `proxy_not_optimal`); none was made report-only.
