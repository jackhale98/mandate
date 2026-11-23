# Daily routines (M6 S1, DEC-031): People move through the day by `content/base/routines.jsonc` (posts by

**Date:** 2026-11-20
**Area:** Daily routines (M6 S1, DEC-031)

## Parameter and change
People move through the day by `content/base/routines.jsonc` (posts by role word, occupations, the broker, the rival, politicians out of office, anyone); placement derived from state and the turn's representative day, keyed (`[ROUTINE, person, day, part]`). Movement, `routines --seed 4242 --days 7`: councillors on their ward's streets in the morning, the town hall or the diner (Tuesdays and Thursdays) in the afternoon, the chamber on session days (the regular sitting weekday or a law-making stage this turn: 1 turn in 7 for an idle player), home streets at night; the mayor at the town hall by day, the diner some Friday lunchtimes, the chamber on some session afternoons; the broker (a business leader in that seed) at the market or their association by day and in the diner late at night every night; a shopkeeper at the market by day, the park or home on Sundays, the community centre on meeting nights. Bots find people through the engine's answer (`characters::routines::whereabouts`, as "Go to them" does)

## Evidence
Bots 16, before → after: every win rate and mean share identical to full precision (canvass 87.5%, explorer 87.5%, corrupt 81.25%, legislator 81.25%, mixed 81.25%, proxy 81.25%, media 6.25%, idle 6.25%, explore_only 6.25%; explorer − mixed 1.144 points; sole best canvass 8, explorer 6). Determinism 2026 × 80 identical twice and through save/load; native = WASM (777 × 60)

## Effect
No balance change: moving is free and a conversation's outcome does not depend on where it is held, so the bots that talk only walk further. Where people are is now something to learn (the notebook's last seen and "usually at" leads)
