# Arcs X4: the four turns played (figures only)

**Date:** 2026-11-25
**Area:** arcs (`content/base/arcs/{rival,favour_debt,plant_closure,flood}.jsonc`); the lab's planted causes (`arcs_grade::plant`).

## Parameter and change

Content structure and the smallest engine changes it needed (the progress entry of the same day lists them). **No simulation number was tuned.** New content numbers are placeholders for the tuning pass: the plant's relief (0.08 for the steward's short weeks, 0.08 for the manager's case, each for 12 months; −0.04 for two months when the line stops), the rival's lead-up from 30 days before the vote and the showdown from 21 (the debate 7 to 14 days on; it was 14 to 21 days on from 30), the favour's call held 14 to 21 days. The lab's planted plant review is a fall in demand for the largest plant's industry of 0.85, 0.72 or 0.6 by seed, for 30 months (the lab's numbers).

No stand-in or bot plays a scene yet (task X3): every graded meeting and debate is an empty chair. The grader therefore measures the levers around the turns, not the turns.

## Evidence

`mandate-cli arcs grade --seeds 16`. Before: this branch with X1 merged (d2d44b5 + the exchange core). After: aa5e308 with `claude/bold-brown-shyeug` merged.

| Arc | Worlds | Median agency | Exits differ (passive, engaged) | Coverage | Engaged exits |
|---|---|---|---|---|---|
| rival | 16 → 16 | 0.601 → 0.601 | 0.75 → 0.75 | `cooled` listed → same | won 0.75, lost 0.25 (passive: lost 1.0) |
| favour debt | 0 (cast fails: no debt) → 16 (debt planted) | none → 0.004 | none → 0.31 | none → complete but `deferred` (listed) | repaid 0.25, burned 0.06, settled 0.69 (passive: settled 1.0); the call came in 1 of 16 |
| plant closure | 13 → 14 | 0.007 → 0.005 | 0.23 → 0.14 | complete → complete | reprieved 0.14, answered 0.21, closed 0.21, scare 0.43 (passive: answered 0.29, closed 0.29, scare 0.43) |
| flood | 16 → 16 | 0.606 → 0.606 | 0.75 → 0.75 | complete → complete | defended 0.81, rebuilt 0.19 (passive: hollowed 0.75) |
| budget round | unchanged | 0.804 | 0.94 | `late` listed | |

## Effect

- The favour debt starts in every planted world; its turn needs a bill a yes would cost, which came once in 16 (the bind is read from live state, so a cost-free bill is paid by an ordinary vote and the run ends at *repaid* with no turn).
- The plant's levers reach the plant: in the middling worlds the steward's and the manager's relief lifts some notices the passive player meets closed; the grave ones close whatever is done, as the design intends; the mild ones end in *scare* for everyone.
- The flood's and the rival's figures are where they were: their engaged stand-ins already took the levers that decide their exits, and the turn itself is an empty chair for every stand-in until X3.
