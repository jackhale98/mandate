# Places with purpose (M6 PL1, DEC-079 note): Visit verbs (`actions.jsonc` `visit`): every amount small (awareness ≤

**Date:** 2026-11-20
**Area:** Places with purpose (M6 PL1, DEC-079 note)

## Parameter and change
Visit verbs (`actions.jsonc` `visit`): every amount small (awareness ≤ 0.08 of the gap among the works' neighbourhood's low and middle classes, approval ≤ 0.01 × room, opinion +1 to +3); photo op risk 0.04 + 0.25 (plant closure within 6 months) + 0.15 (plant-closure crisis) + 0.1 (job loss ≥ 3% a year) + 0.3 × shedding, within [0.02, 0.6], heckled: approval −0.04 × room (works' neighbourhood, low and middle) and −0.005 town-wide; the photo op has no approval gain (awareness only, so repeating it does not pile up support); outlook bands from jobs ÷ capacity (growing ≥ 1.02, shedding < 0.95, full at 0.85). Eleven occupations seeded in round 1 (after the existing cast). Bots: informed styles no longer chase the round-1 people one by one while exploring (they are met at their places by visits); the coalition builder makes learning visits; the campaigner takes a photo op at a ward works between elections at shown risk ≤ 0.15

## Evidence
`bots --seeds 16`, before → after: explorer 0.6184 → 0.6187, campaigner 0.6198 → 0.6199, coalition 0.6155 → 0.6168, legislator 0.6156 → 0.6151, mixed 0.6074 → 0.6080 (win rates unchanged, every style 81%); with the new people chased in exploration (first try) `explorer` was among the best in 11–12 of 16 worlds and `good_styles_vary_b` failed

## Effect
Every enforced gate passes; ladder_learned +1.06 points (was +1.10); best-in-world counts explorer 4, campaigner 7, coalition 6, legislator 6 (top share 0.44). Reported, not enforced: `good_styles_vary` clause (a) now fails (near-best styles that are sole best often enough: coalition only, was coalition and campaigner; the campaigner is sole best in 3 worlds, was 5)
